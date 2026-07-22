import { useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { githubAnalysisApi } from "../api/github";
import { projectKernelApi } from "../api/project-kernel";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { GitHubAnalysisReportView, GitHubAnalysisSource, SourcedConclusion } from "../types/github";
import { localizeAppError } from "../utils/appError";

const RUNNING = new Set(["queued", "collecting", "analyzing"]);

export function GitHubAnalysisReportPage() {
  const { t } = useTranslation(); const navigate = useNavigate(); const { analysisId = "" } = useParams();
  const errorText = (reason: unknown) => { const value = localizeAppError(reason, t); return value.guidance ? `${value.title}：${value.guidance}` : value.title; };
  const [workspaceId, setWorkspaceId] = useState<string>(); const [view, setView] = useState<GitHubAnalysisReportView>();
  const [error, setError] = useState<string>(); const [busy, setBusy] = useState(false); const [copied, setCopied] = useState(false);
  useEffect(() => { void initialize(); let disposed = false; const unlisten = listen<{ analysisRunId: string }>("github-analysis://completed", (event) => { if (!disposed && event.payload.analysisRunId === analysisId) void refresh(); }); return () => { disposed = true; void unlisten.then((dispose) => dispose()); }; }, [analysisId]);
  useEffect(() => { if (!view || !RUNNING.has(view.status.status)) return; const timer = window.setInterval(() => void refresh(), 1000); return () => window.clearInterval(timer); }, [view?.status.status, workspaceId, analysisId]);
  async function initialize() { try { const workspace = await projectKernelApi.getCurrentWorkspace(); if (!workspace) { setError(t("githubAnalysis.workspaceRequired")); return; } setWorkspaceId(workspace.id); setView(await githubAnalysisApi.getReport(workspace.id, analysisId)); } catch (reason) { setError(errorText(reason)); } }
  async function refresh() { if (!workspaceId) return; try { setView(await githubAnalysisApi.getReport(workspaceId, analysisId)); } catch (reason) { setError(errorText(reason)); } }
  async function reanalyze() { if (!workspaceId) return; setBusy(true); try { const result = await githubAnalysisApi.reanalyze(workspaceId, analysisId); navigate(`/github-import/analyses/${result.analysisRunId}`); if (result.reusedExistingRun) await refresh(); } catch (reason) { setError(errorText(reason)); } finally { setBusy(false); } }
  async function archive() { if (!workspaceId || !view) return; setBusy(true); try { const status = await githubAnalysisApi.archive(workspaceId, analysisId, view.status.version); setView((current) => current ? { ...current, status } : current); } catch (reason) { setError(errorText(reason)); } finally { setBusy(false); } }
  async function copy() { if (!view?.contentMarkdown || view.status.readiness === "blocked") return; await navigator.clipboard.writeText(view.contentMarkdown); setCopied(true); }
  if (!view) return <div className="page-stack"><PageIntro title={t("githubAnalysis.reportTitle")} description={t("common.loading")} />{error ? <div className="error-banner">{error}</div> : null}</div>;
  const status = view.status; const running = RUNNING.has(status.status);
  return <div className="page-stack github-analysis-report-page">
    <PageIntro title={`${status.owner}/${status.name}`} description={t("githubAnalysis.reportDescription")} primaryAction={!running && view.contentMarkdown ? <button className="primary-button" disabled={status.readiness === "blocked"} onClick={() => void copy()} type="button">{copied ? t("common.copied") : t("githubAnalysis.copyReport")}</button> : undefined} secondaryActions={<button className="secondary-button" type="button" onClick={() => navigate("/github-import?mode=analysis")}>{t("githubAnalysis.backToAnalyses")}</button>} />
    {error ? <div className="error-banner">{error}</div> : null}
    <AnalysisStatus view={view} />
    {view.report ? <ReportDocument view={view} /> : null}
    {!running && status.status !== "archived" ? <div className="github-analysis-actions"><button className="secondary-button" disabled={busy} onClick={() => void reanalyze()} type="button">{t("githubAnalysis.reanalyze")}</button><button className="text-button" disabled={busy} onClick={() => void archive()} type="button">{t("githubAnalysis.archive")}</button></div> : null}
    <details className="technical-details"><summary>{t("common.technicalDetails")}</summary><dl><dt>HEAD</dt><dd><CompactIdentifier value={status.sourceCommitSha} copyable /></dd><dt>Analyzer</dt><dd>github-rules-v1</dd><dt>SHA-256</dt><dd><CompactIdentifier value={view.contentHash ?? "-"} copyable={Boolean(view.contentHash)} /></dd></dl></details>
  </div>;
}

function AnalysisStatus({ view }: { view: GitHubAnalysisReportView }) { const { t } = useTranslation(); const s = view.status; return <section className={`notice ${s.status === "failed" ? "notice--danger" : s.status === "partial" || s.readiness === "blocked" ? "notice--warning" : "notice--info"}`}><div><strong>{t(`githubAnalysis.status_${s.status}`)}</strong><p>{s.stage ? t(`githubAnalysis.stage_${s.stage}`) : t(`githubAnalysis.readiness_${s.readiness}`)}</p></div><div className="analysis-progress-metrics"><span>{t("githubAnalysis.treeCount", { count: s.treeEntriesSeen })}</span><span>{t("githubAnalysis.readCount", { count: s.filesRead })}</span><span>{t("githubAnalysis.skippedCount", { count: s.filesSkipped })}</span><span>{t("githubAnalysis.rateRemaining", { count: s.apiRequestsRemaining ?? t("common.unknown") })}</span></div>{s.errorSummary ? <p>{s.errorSummary}</p> : null}{s.stale ? <p>{t("githubAnalysis.stale")}</p> : null}</section>; }

function ReportDocument({ view }: { view: GitHubAnalysisReportView }) { const { t } = useTranslation(); const report = view.report!; const sourceMap = useMemo(() => new Map(view.sources.map((source) => [source.id, source])), [view.sources]); return <article className="github-analysis-document">
  <ReportSection title={t("githubAnalysis.sectionOverview")} items={report.overview ? [report.overview] : []} sourceMap={sourceMap} />
  <ReportSection title={t("githubAnalysis.sectionCapabilities")} items={report.capabilities} sourceMap={sourceMap} />
  <ReportSection title={t("githubAnalysis.sectionTechStack")} items={report.techStack} sourceMap={sourceMap} />
  <section><h2>{t("githubAnalysis.sectionArchitecture")}</h2>{report.architecture.map((item) => <SourcedItem key={`${item.value.name}-${item.sourceIds.join()}`} item={{ ...item, value: `${item.value.name} — ${item.value.responsibility}\n${item.value.keyPaths.join(", ")}` }} sourceMap={sourceMap} />)}</section>
  <section><h2>{t("githubAnalysis.sectionEntrypoints")}</h2>{report.entrypoints.map((item) => <SourcedItem key={item.value.path} item={{ ...item, value: `${item.value.path} — ${item.value.purpose}` }} sourceMap={sourceMap} />)}</section>
  <ReportSection title={t("githubAnalysis.sectionBuild")} items={report.buildAndRun} sourceMap={sourceMap} />
  <ReportSection title={t("githubAnalysis.sectionTests")} items={report.testStrategy} sourceMap={sourceMap} />
  <section><h2>{t("githubAnalysis.sectionDependencies")}</h2>{report.dependencies ? <SourcedItem item={{ ...report.dependencies, value: `${report.dependencies.value.summary}\n${t("githubAnalysis.packageManagers")}: ${report.dependencies.value.packageManagers.join(", ") || "—"}\n${t("githubAnalysis.sbomAvailable")}: ${report.dependencies.value.sbomAvailable ? t("common.enabled") : t("common.disabled")}` }} sourceMap={sourceMap} /> : <p className="muted">—</p>}</section>
  <section><h2>{t("githubAnalysis.sectionLicense")}</h2>{report.license ? <SourcedItem item={{ ...report.license, value: `${report.license.value.identifier || t("common.unknown")}\n${report.license.value.reuseGuidance}` }} sourceMap={sourceMap} /> : <p className="muted">—</p>}</section>
  <ReportSection title={t("githubAnalysis.sectionRisks")} items={report.risks} sourceMap={sourceMap} />
  <ReportSection title={t("githubAnalysis.sectionReading")} items={report.recommendedReadingOrder} sourceMap={sourceMap} />
  <ReportSection title={t("githubAnalysis.sectionExtension")} items={report.extensionPoints} sourceMap={sourceMap} />
  {report.userQuestionAnswer ? <ReportSection title={t("githubAnalysis.sectionQuestion")} items={[report.userQuestionAnswer]} sourceMap={sourceMap} /> : null}
  <section><h2>{t("githubAnalysis.sectionReproduction")}</h2><SourcedItem item={{ ...report.reproductionAssessment, value: `${t(`githubAnalysis.difficulty_${report.reproductionAssessment.value.difficulty}`)}\n${report.reproductionAssessment.value.caveats.join("\n")}` }} sourceMap={sourceMap} /></section>
  {report.qualityWarnings.length ? <section><h2>{t("githubAnalysis.qualityWarnings")}</h2><ul>{report.qualityWarnings.map((warning) => <li key={warning}>{t(`githubAnalysis.warning_${warning}`)}</li>)}</ul></section> : null}
  <p className="analysis-boundary">{t("githubAnalysis.noProjectCreated")}</p>
 </article>; }

function ReportSection({ title, items, sourceMap }: { title: string; items: SourcedConclusion<string>[]; sourceMap: Map<string, GitHubAnalysisSource> }) { return <section><h2>{title}</h2>{items.length ? items.map((item, index) => <SourcedItem key={`${title}-${index}`} item={item} sourceMap={sourceMap} />) : <p className="muted">—</p>}</section>; }
function SourcedItem({ item, sourceMap }: { item: SourcedConclusion<string>; sourceMap: Map<string, GitHubAnalysisSource> }) { const { t } = useTranslation(); const sources = item.sourceIds.map((id) => sourceMap.get(id)).filter(Boolean) as GitHubAnalysisSource[]; return <div className="sourced-conclusion"><p>{item.value}</p><details><summary>{t("githubAnalysis.viewSources", { count: sources.length })} · {t(`githubAnalysis.confidence_${item.confidence}`)}</summary>{sources.map((source) => <div className="analysis-source" key={source.id}><strong>{source.sourcePath || source.sourceType}</strong><pre>{source.excerpt}</pre></div>)}</details></div>; }
