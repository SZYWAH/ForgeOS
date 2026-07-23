import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { githubAnalysisApi } from "../api/github";
import { aiProviderApi } from "../api/ai-provider";
import { projectKernelApi } from "../api/project-kernel";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type {
  GitHubAnalysisReport,
  GitHubAnalysisReportView,
  GitHubAnalysisSource,
  GitHubSectionAvailability,
  SourcedConclusion,
} from "../types/github";
import type { AiProvider } from "../types/ai-provider";
import { localizeAppError } from "../utils/appError";

const RUNNING = new Set(["queued", "collecting", "analyzing"]);

export function GitHubAnalysisReportPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { analysisId = "" } = useParams();
  const [workspaceId, setWorkspaceId] = useState<string>();
  const [view, setView] = useState<GitHubAnalysisReportView>();
  const [error, setError] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [aiProviderId, setAiProviderId] = useState("");

  const errorText = (reason: unknown) => {
    const value = localizeAppError(reason, t);
    return value.guidance ? `${value.title}：${value.guidance}` : value.title;
  };

  useEffect(() => {
    void initialize();
    let disposed = false;
    const unlisten = listen<{ analysisRunId: string }>("github-analysis://completed", (event) => {
      if (!disposed && event.payload.analysisRunId === analysisId) void refresh();
    });
    return () => {
      disposed = true;
      void unlisten.then((dispose) => dispose());
    };
  }, [analysisId]);

  useEffect(() => {
    if (!view || !RUNNING.has(view.status.status)) return;
    const timer = window.setInterval(() => void refresh(), 1000);
    return () => window.clearInterval(timer);
  }, [view?.status.status, workspaceId, analysisId]);

  async function initialize() {
    try {
      const workspace = await projectKernelApi.getCurrentWorkspace();
      if (!workspace) {
        setError(t("githubAnalysis.workspaceRequired"));
        return;
      }
      setWorkspaceId(workspace.id);
      const [report, availableProviders] = await Promise.all([
        githubAnalysisApi.getReport(workspace.id, analysisId),
        aiProviderApi.list().catch(() => [] as AiProvider[]),
      ]);
      const enabledProviders = availableProviders.filter((provider) => provider.enabled);
      setProviders(enabledProviders);
      if (enabledProviders.length === 1) setAiProviderId(enabledProviders[0].id);
      setView(report);
    } catch (reason) {
      setError(errorText(reason));
    }
  }

  async function refresh() {
    if (!workspaceId) return;
    try {
      setView(await githubAnalysisApi.getReport(workspaceId, analysisId));
    } catch (reason) {
      setError(errorText(reason));
    }
  }

  async function reanalyze() {
    if (!workspaceId) return;
    setBusy(true);
    try {
      const result = await githubAnalysisApi.reanalyze(workspaceId, analysisId);
      navigate(`/github-import/analyses/${result.analysisRunId}`);
      if (result.reusedExistingRun) await refresh();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }

  async function archive() {
    if (!workspaceId || !view) return;
    setBusy(true);
    try {
      const status = await githubAnalysisApi.archive(workspaceId, analysisId, view.status.version);
      setView((current) => current ? { ...current, status } : current);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }

  async function enhanceWithAi() {
    if (!workspaceId || !view || !aiProviderId) return;
    setBusy(true); setError(undefined);
    try {
      const status = view.status;
      const result = await githubAnalysisApi.start({
        workspaceId,
        repositoryUrl: status.repositoryUrl,
        focusType: status.focusType,
        customPrompt: status.customPrompt,
        analysisMode: "ai",
        aiProviderId,
      });
      navigate(`/github-import/analyses/${result.analysisRunId}`);
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }

  async function copy() {
    if (!view?.contentMarkdown || view.status.readiness === "blocked") return;
    await navigator.clipboard.writeText(view.contentMarkdown);
    setCopied(true);
  }

  if (!view) {
    return <div className="page-stack"><PageIntro title={t("githubAnalysis.reportTitle")} description={t("common.loading")} />{error ? <div className="error-banner">{error}</div> : null}</div>;
  }

  const status = view.status;
  const running = RUNNING.has(status.status);
  const legacy = status.analyzerRevision === "github-rules-v1";

  return <div className="page-stack github-analysis-report-page">
    <PageIntro
      title={`${status.owner}/${status.name}`}
      description={t("githubAnalysis.reportDescription")}
      primaryAction={!running && view.contentMarkdown ? <button className="primary-button" disabled={status.readiness === "blocked"} onClick={() => void copy()} type="button">{copied ? t("common.copied") : t("githubAnalysis.copyReport")}</button> : undefined}
      secondaryActions={<button className="secondary-button" type="button" onClick={() => navigate("/github-import?mode=analysis")}>{t("githubAnalysis.backToAnalyses")}</button>}
    />
    {error ? <div className="error-banner">{error}</div> : null}
    {legacy ? <div className="notice notice--warning"><strong>{t("githubAnalysis.legacyReport")}</strong><p>{t("githubAnalysis.legacyReportDescription")}</p></div> : null}
    <AnalysisStatus view={view} />
    {!running && status.analysisMode === "rules" && status.readiness !== "blocked" ? <section className="surface github-ai-enhance"><div><h2>{t("githubAnalysis.enhanceReport")}</h2><p>{t("githubAnalysis.enhanceReportDescription")}</p></div>{providers.length ? <div className="github-ai-enhance__actions"><label>{t("githubAnalysis.aiProvider")}<select value={aiProviderId} onChange={(event) => setAiProviderId(event.target.value)}><option value="">{t("githubAnalysis.selectAiProvider")}</option>{providers.map((provider) => <option key={provider.id} value={provider.id}>{provider.name} · {provider.model}</option>)}</select></label><button className="secondary-button" disabled={busy || !aiProviderId} type="button" onClick={() => void enhanceWithAi()}>{t("githubAnalysis.enhanceAction")}</button></div> : <div className="notice notice--warning"><p>{t("githubAnalysis.noAiProviderDescription")}</p><button className="secondary-button" type="button" onClick={() => navigate("/settings/ai-providers")}>{t("githubAnalysis.configureAiProvider")}</button></div>}</section> : null}
    {view.report ? <ReportDocument view={view} /> : null}
    {!running && status.status !== "archived" ? <div className="github-analysis-actions"><button className="secondary-button" disabled={busy} onClick={() => void reanalyze()} type="button">{t("githubAnalysis.reanalyze")}</button><button className="text-button" disabled={busy} onClick={() => void archive()} type="button">{t("githubAnalysis.archive")}</button></div> : null}
    <details className="technical-details"><summary>{t("common.technicalDetails")}</summary><dl><dt>HEAD</dt><dd><CompactIdentifier value={status.sourceCommitSha} copyable /></dd><dt>Analyzer</dt><dd>{status.analyzerRevision}</dd><dt>SHA-256</dt><dd><CompactIdentifier value={view.contentHash ?? "-"} copyable={Boolean(view.contentHash)} /></dd></dl></details>
  </div>;
}

function AnalysisStatus({ view }: { view: GitHubAnalysisReportView }) {
  const { t } = useTranslation();
  const status = view.status;
  const warning = status.status === "partial" || status.readiness === "blocked" || status.readiness === "warning";
  return <section className={`notice ${status.status === "failed" ? "notice--danger" : warning ? "notice--warning" : "notice--info"}`}>
    <div><strong>{t(`githubAnalysis.status_${status.status}`)} · {t(`githubAnalysis.method_${status.analysisMethod}`)}</strong><p>{status.stage ? t(`githubAnalysis.stage_${status.stage}`) : t(`githubAnalysis.readiness_${status.readiness}`)}</p>{status.analysisMode === "ai" && status.aiModel ? <small>{t("githubAnalysis.aiModel", { model: status.aiModel })}</small> : null}</div>
    <div className="analysis-progress-metrics"><span>{t("githubAnalysis.treeCount", { count: status.treeEntriesSeen })}</span><span>{t("githubAnalysis.readCount", { count: status.filesRead })}</span><span>{t("githubAnalysis.skippedCount", { count: status.filesSkipped })}</span><span>{t("githubAnalysis.rateRemaining", { count: status.apiRequestsRemaining ?? t("common.unknown") })}</span></div>
    {status.errorSummary ? <p>{status.errorSummary}</p> : null}
    {status.stale ? <p>{t("githubAnalysis.stale")}</p> : null}
  </section>;
}

function ReportDocument({ view }: { view: GitHubAnalysisReportView }) {
  const { t } = useTranslation();
  const report = view.report!;
  const sourceMap = useMemo(() => new Map(view.sources.map((source) => [source.id, source])), [view.sources]);
  const productTech = report.schemaVersion === "github-report-v2" ? report.productTechStack ?? [] : report.techStack;

  return <article className="github-analysis-document">
    {report.repositoryKind ? <section className="github-analysis-boundary"><h2>{t("githubAnalysis.repositoryKindAndBoundary")}</h2><SourcedItem item={{ ...report.repositoryKind, value: t(`githubAnalysis.repositoryKind_${report.repositoryKind.value}`) }} sourceMap={sourceMap} />{report.repositoryKind.value === "release_registry" ? <p>{t("githubAnalysis.releaseRegistryBoundary")}</p> : null}</section> : null}
    <ReportSection title={t("githubAnalysis.sectionOverview")} items={report.overview ? [report.overview] : []} sourceMap={sourceMap} availability={availability(report, "overview")} />
    <ReportSection title={t("githubAnalysis.sectionCapabilities")} items={report.capabilities} sourceMap={sourceMap} availability={availability(report, "capabilities")} />
    <ReportSection title={t("githubAnalysis.sectionProductTechStack")} items={productTech} sourceMap={sourceMap} availability={availability(report, "product_tech_stack")} />
    {report.schemaVersion === "github-report-v2" ? <ReportSection title={t("githubAnalysis.sectionToolingStack")} items={report.repositoryToolingStack ?? []} sourceMap={sourceMap} availability={availability(report, "repository_tooling_stack")} /> : null}
    <section><h2>{t("githubAnalysis.sectionArchitecture")}</h2>{report.architecture.length ? report.architecture.map((item) => <SourcedItem key={`${item.value.name}-${item.sourceIds.join()}`} item={{ ...item, value: `${item.value.name} — ${item.value.responsibility}\n${item.value.keyPaths.join(", ")}` }} sourceMap={sourceMap} />) : <Unavailable reason={availability(report, "architecture")} />}</section>
    <section><h2>{t("githubAnalysis.sectionEntrypoints")}</h2>{report.entrypoints.length ? report.entrypoints.map((item) => <SourcedItem key={item.value.path} item={{ ...item, value: `${item.value.path} — ${item.value.purpose}` }} sourceMap={sourceMap} />) : <Unavailable reason={availability(report, "entrypoints")} />}</section>
    <ReportSection title={t("githubAnalysis.sectionBuild")} items={report.buildAndRun} sourceMap={sourceMap} availability={availability(report, "build_and_run")} />
    <ReportSection title={t("githubAnalysis.sectionTests")} items={report.testStrategy} sourceMap={sourceMap} availability={availability(report, "test_strategy")} />
    <section><h2>{t("githubAnalysis.sectionDependencies")}</h2>{report.dependencies ? <SourcedItem item={{ ...report.dependencies, value: `${report.dependencies.value.summary}\n${t("githubAnalysis.packageManagers")}: ${report.dependencies.value.packageManagers.join(", ") || t("common.unknown")}\n${t("githubAnalysis.sbomAvailable")}: ${report.dependencies.value.sbomAvailable ? t("common.enabled") : t("common.disabled")}` }} sourceMap={sourceMap} /> : <Unavailable reason={availability(report, "dependencies")} />}</section>
    <section><h2>{t("githubAnalysis.sectionLicense")}</h2>{report.license ? <SourcedItem item={{ ...report.license, value: `${report.license.value.identifier || t("common.unknown")}\n${report.license.value.reuseGuidance}` }} sourceMap={sourceMap} /> : <Unavailable reason={availability(report, "license")} />}</section>
    <section><h2>{t("githubAnalysis.sectionRecentActivity")}</h2>{report.recentActivity?.length ? <ul className="github-analysis-activity-list">{report.recentActivity.map((item) => {
      const sources = item.sourceIds.map((id) => sourceMap.get(id)).filter(Boolean) as GitHubAnalysisSource[];
      return <li key={`${item.kind}-${item.identifier}`}><strong>{t(`githubAnalysis.activity_${item.kind}`)}</strong><CompactIdentifier value={item.identifier} /><span>{item.title}</span>{item.occurredAt ? <time>{item.occurredAt}</time> : null}<details><summary>{t("githubAnalysis.viewSources", { count: sources.length })}</summary>{sources.map((source) => <div className="analysis-source" key={source.id}><strong>{source.sourcePath || source.sourceType}</strong><pre>{source.excerpt}</pre></div>)}</details></li>;
    })}</ul> : <Unavailable reason={availability(report, "recent_activity")} />}</section>
    <ReportSection title={t("githubAnalysis.sectionRisks")} items={report.risks} sourceMap={sourceMap} availability={availability(report, "risks")} />
    <ReportSection title={t("githubAnalysis.sectionReading")} items={report.recommendedReadingOrder} sourceMap={sourceMap} availability={availability(report, "recommended_reading_order")} />
    <section><h2>{t("githubAnalysis.sectionReproduction")}</h2><SourcedItem item={{ ...report.reproductionAssessment, value: `${t(`githubAnalysis.difficulty_${report.reproductionAssessment.value.difficulty}`)}\n${report.reproductionAssessment.value.caveats.join("\n")}` }} sourceMap={sourceMap} /></section>
    <ReportSection title={t("githubAnalysis.sectionExtension")} items={report.extensionPoints} sourceMap={sourceMap} availability={availability(report, "extension_points")} />
    {report.userQuestionAnswer ? <ReportSection title={t("githubAnalysis.sectionQuestion")} items={[report.userQuestionAnswer]} sourceMap={sourceMap} availability={availability(report, "user_question_answer")} /> : availability(report, "user_question_answer") !== "not_applicable" ? <ReportSection title={t("githubAnalysis.sectionQuestion")} items={[]} sourceMap={sourceMap} availability={availability(report, "user_question_answer")} /> : null}
    {report.qualityWarnings.length ? <section><h2>{t("githubAnalysis.qualityWarnings")}</h2><ul>{report.qualityWarnings.map((warning) => <li key={warning}>{t(`githubAnalysis.warning_${warning}`)}</li>)}</ul></section> : null}
    <p className="analysis-boundary">{t("githubAnalysis.noProjectCreated")}</p>
  </article>;
}

function availability(report: GitHubAnalysisReport, section: string): GitHubSectionAvailability {
  return report.sectionAvailability?.[section] ?? "source_not_present";
}

function Unavailable({ reason }: { reason: GitHubSectionAvailability }) {
  const { t } = useTranslation();
  return <p className="muted">{t(`githubAnalysis.availability_${reason}`)}</p>;
}

function ReportSection({ title, items, sourceMap, availability: reason = "source_not_present" }: { title: string; items: SourcedConclusion<string>[]; sourceMap: Map<string, GitHubAnalysisSource>; availability?: GitHubSectionAvailability }) {
  return <section><h2>{title}</h2>{items.length ? items.map((item, index) => <SourcedItem key={`${title}-${index}`} item={item} sourceMap={sourceMap} />) : <Unavailable reason={reason} />}</section>;
}

function SourcedItem({ item, sourceMap }: { item: SourcedConclusion<string>; sourceMap: Map<string, GitHubAnalysisSource> }) {
  const { t } = useTranslation();
  const sources = item.sourceIds.map((id) => sourceMap.get(id)).filter(Boolean) as GitHubAnalysisSource[];
  return <div className="sourced-conclusion"><p>{item.value}</p><details><summary>{t("githubAnalysis.viewSources", { count: sources.length })} · {t(`githubAnalysis.confidence_${item.confidence}`)}</summary>{sources.map((source) => <div className="analysis-source" key={source.id}><strong>{source.sourcePath || source.sourceType}</strong><pre>{source.excerpt}</pre></div>)}</details></div>;
}
