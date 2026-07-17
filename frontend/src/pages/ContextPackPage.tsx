import { type FormEvent, type ReactNode, useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { projectMemoryApi } from "../api/project-memory";
import { contextPackApi } from "../api/context-pack";
import { aiProviderApi } from "../api/ai-provider";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { ProjectMemoryNav } from "../components/ProjectMemoryNav";
import { BoundedList, CollapsibleText } from "../components/content/ContentGovernance";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";
import { Notice, StatusBadge } from "../components/ui/VisualSystem";
import type { AiProvider, AiInvocationResult } from "../types/ai-provider";
import { CONTEXT_TEMPLATES, type ContextExport, type ContextTemplateType } from "../types/context-pack";
import type { MemoryQualityIssue, ProjectAiContext } from "../types/project-memory";
import type { Project } from "../types/project";
import { displayLabel, formatUpdatedAt } from "../utils/format";

export function ContextPackPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [context, setContext] = useState<ProjectAiContext>();
  const [exports, setExports] = useState<ContextExport[]>([]);
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [template, setTemplate] = useState<ContextTemplateType>("coding");
  const [selectedExport, setSelectedExport] = useState<ContextExport>();
  const [providerId, setProviderId] = useState("");
  const [prompt, setPrompt] = useState("");
  const [response, setResponse] = useState<AiInvocationResult>();
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();

  const load = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, contextValue, exportValues, providerValues] = await Promise.all([
        projectKernelApi.getProject(projectId),
        projectMemoryApi.buildAiContext(projectId),
        contextPackApi.list(projectId),
        aiProviderApi.list(),
      ]);
      setProject(projectValue);
      setContext(contextValue);
      setExports(exportValues);
      setProviders(providerValues.filter((value) => value.enabled));
      setProviderId((current) => current || providerValues.find((value) => value.enabled)?.id || "");
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => { void load(); }, [load]);

  async function copyContext() {
    if (!context || context.readiness === "blocked") return;
    try {
      await navigator.clipboard.writeText(context.contentMarkdown);
      setNotice(t("memory.contextCopied"));
    } catch (reason) {
      setError(getErrorMessage(reason));
    }
  }

  async function generate() {
    if (!projectId || context?.readiness === "blocked") return;
    setWorking(true);
    setError(undefined);
    try {
      const value = await contextPackApi.generate(projectId, template);
      setSelectedExport(value);
      setExports((items) => [value, ...items.filter((item) => item.id !== value.id)]);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setWorking(false);
    }
  }

  async function askAi(event: FormEvent) {
    event.preventDefault();
    if (!projectId || !providerId || !prompt.trim() || context?.readiness === "blocked") return;
    setWorking(true);
    setError(undefined);
    try {
      const exportValue = selectedExport ?? await contextPackApi.generate(projectId, template);
      setSelectedExport(exportValue);
      const value = await contextPackApi.invokeWithAi(projectId, exportValue.id, providerId, prompt.trim());
      setResponse(value.ai);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setWorking(false);
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) return <div className="error-banner">{error ?? t("projects.noProject")}</div>;

  return (
    <div className="page-stack ai-context-page">
      <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("memory.aiContext")}</span></div>
      <PageIntro
        title={t("memory.aiContext")}
        description={t("memory.aiContextDescription")}
        primaryAction={<button className="primary-button" disabled={!context || context.readiness === "blocked"} onClick={() => void copyContext()} type="button">{t("memory.copyContext")}</button>}
      />
      <ProjectMemoryNav projectId={project.id} />
      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}
      {context ? <ContextReadiness context={context} /> : null}
      {context ? <ProjectContextDocument context={context} /> : null}

      <TechnicalDetails label={t("contextTrust.technicalInformation")}>
        <div className="ai-context-technical">
          <code>SHA-256 {context?.contentHash ?? "-"}</code>
          <p>{t("contextTrust.rawDescription")}</p>
          <pre className="context-markdown-preview document-scroll-region">{context?.contentMarkdown ?? t("common.none")}</pre>
        </div>
      </TechnicalDetails>

      <details className="mcp-guidance"><summary>{t("memory.mcpGuide")}</summary><div><p>{t("memory.mcpGuideDescription")}</p><p>{t("memory.mcpReadOnly")}</p></div></details>
      <details className="advanced-ai-context"><summary>{t("memory.advancedAiTools")}</summary><div className="advanced-ai-context__body">
        <section><h2>{t("context.prepare")}</h2><div className="inline-controls"><label>{t("context.purpose")}<select value={template} onChange={(event) => setTemplate(event.target.value as ContextTemplateType)}>{CONTEXT_TEMPLATES.map((value) => <option key={value} value={value}>{displayLabel(value)}</option>)}</select></label><button className="secondary-button" disabled={working || context?.readiness === "blocked"} onClick={() => void generate()} type="button">{t("context.generate")}</button></div>{selectedExport ? <pre className="document-scroll-region">{selectedExport.contentMarkdown}</pre> : null}</section>
        <form onSubmit={askAi}><h2>{t("context.ask")}</h2><div className="form-grid"><label>{t("context.provider")}<select value={providerId} onChange={(event) => setProviderId(event.target.value)}><option value="">{t("context.chooseProvider")}</option>{providers.map((value) => <option key={value.id} value={value.id}>{value.name} · {value.model}</option>)}</select></label><label className="span-2">{t("context.question")}<textarea rows={4} value={prompt} onChange={(event) => setPrompt(event.target.value)} /></label></div><button className="secondary-button" disabled={working || !providerId || !prompt.trim() || context?.readiness === "blocked"} type="submit">{t("context.send")}</button>{response ? <pre className="document-scroll-region">{response.content}</pre> : null}</form>
        <section><h2>{t("context.previousExports")}</h2>{exports.length === 0 ? <p>{t("context.emptyExports")}</p> : <div className="context-export-list">{exports.map((value) => <button key={value.id} onClick={() => setSelectedExport(value)} type="button"><strong>{displayLabel(value.templateType)}</strong><span>{formatUpdatedAt(value.createdAt)}</span></button>)}</div>}</section>
      </div></details>
    </div>
  );
}

function ContextReadiness({ context }: { context: ProjectAiContext }) {
  const { t } = useTranslation();
  const tone = context.readiness === "blocked" ? "danger" : context.readiness === "warning" ? "warning" : "success";
  return (
    <Notice tone={tone} title={t(`contextTrust.readiness_${context.readiness}`)}>
      {context.qualityIssues.length === 0 ? <p>{t("contextTrust.noIssues")}</p> : (
        <ul className="quality-issue-list">
          {context.qualityIssues.map((issue, index) => <li key={`${issue.code}-${issue.section}-${index}`}>{qualityIssueLabel(issue, t)}</li>)}
        </ul>
      )}
    </Notice>
  );
}

function ProjectContextDocument({ context }: { context: ProjectAiContext }) {
  const { t } = useTranslation();
  return (
    <article className="ai-context-document">
      <ContextSection title={t("contextTrust.identity")}>
        <dl className="context-fact-grid">
          <Fact label={t("projects.name")} value={context.identity.name} />
          <Fact label={t("projects.projectDescription")} value={context.identity.description} />
          <Fact label={t("memory.vision")} value={context.identity.vision} />
          <Fact label={t("memory.targetUser")} value={context.identity.targetUser} />
          <Fact label={t("memory.scope")} value={context.identity.scopeSummary} />
          <Fact label={t("projects.nonGoals")} value={context.identity.nonGoals} />
          <Fact label={t("memory.techStack")} value={context.identity.techStack} />
        </dl>
      </ContextSection>

      <ContextSection title={t("contextTrust.confirmedState")} badge={<StatusBadge tone="success">{t("memory.confirmed")}</StatusBadge>}>
        {context.confirmedState ? <>
          {context.confirmedState.supersededByNewerActivity ? <p className="context-freshness-warning">{t("contextTrust.confirmedSuperseded")}</p> : null}
          <FactBlock label={t("projects.currentGoal")} value={context.confirmedState.currentGoal} />
          <FactBlock label={t("projects.currentSituation")} value={context.confirmedState.statusSummary} />
          <ContextLists achievements={context.confirmedState.achievements} blockers={context.confirmedState.blockers} nextSteps={context.confirmedState.nextSteps} />
        </> : <p>{t("contextTrust.noConfirmedState")}</p>}
      </ContextSection>

      <ContextSection title={t("contextTrust.observedState")} badge={<StatusBadge tone="warning">{t("memory.observed")}</StatusBadge>}>
        {context.observedState ? <>
          <FactBlock label={t("projects.currentGoal")} value={context.observedState.currentGoal} />
          <FactBlock label={t("projects.currentSituation")} value={context.observedState.statusSummary} />
          <ContextLists achievements={context.observedState.achievements} blockers={context.observedState.blockers} nextSteps={context.observedState.nextSteps} />
        </> : <p>{t("contextTrust.noObservedState")}</p>}
      </ContextSection>

      <ContextSection title={t("memory.recentHistory")}>
        {context.recentHistory.length === 0 ? <p>{t("memory.noHistory")}</p> : <ol className="context-history-list">{context.recentHistory.map((item) => <li key={item.id}><strong>{item.title}</strong><span>{formatUpdatedAt(item.startedAt)}</span><CollapsibleText collapsedLines={3} value={item.outcome || item.activity || item.goal} /></li>)}</ol>}
      </ContextSection>

      <ContextSection title={t("memory.confirmedAchievements")}>
        {context.confirmedAchievements.length === 0 ? <p>{t("memory.noAchievements")}</p> : <ul className="context-achievement-list">{context.confirmedAchievements.map((item) => <li key={item.id}><strong>{item.title}</strong><span>{item.result}</span></li>)}</ul>}
      </ContextSection>

      <ContextSection title={t("contextTrust.gitState")}>
        {context.gitState.length === 0 ? <p>{t("contextTrust.noGitState")}</p> : <div className="context-git-grid">{context.gitState.map((item, index) => <div key={`${item.branch}-${index}`}><code>{item.branch}</code><span>{t("contextTrust.gitSummary", { dirty: item.dirtyFiles, ahead: item.aheadCount ?? t("common.unknown"), behind: item.behindCount ?? t("common.unknown") })}</span></div>)}</div>}
      </ContextSection>

      <ContextSection title={t("projects.nextSteps")}>
        <BoundedList initialCount={3} items={context.nextSteps} ordered emptyLabel={t("memory.noNextStep")} />
      </ContextSection>
    </article>
  );
}

function ContextSection({ title, badge, children }: { title: string; badge?: ReactNode; children: ReactNode }) {
  return <section className="ai-context-document__section"><header><h2>{title}</h2>{badge}</header>{children}</section>;
}

function Fact({ label, value }: { label: string; value: string }) {
  const { t } = useTranslation();
  return <div><dt>{label}</dt><dd>{value || t("common.notProvided")}</dd></div>;
}

function FactBlock({ label, value }: { label: string; value: string }) {
  if (!value) return null;
  return <div className="context-fact-block"><h3>{label}</h3><CollapsibleText collapsedLines={4} value={value} /></div>;
}

function ContextLists({ achievements, blockers, nextSteps }: { achievements: string[]; blockers: string[]; nextSteps: string[] }) {
  const { t } = useTranslation();
  return <div className="context-list-grid"><section><h3>{t("projects.achievements")}</h3><BoundedList items={achievements} emptyLabel={t("common.none")} /></section><section><h3>{t("projects.blockers")}</h3><BoundedList items={blockers} emptyLabel={t("common.none")} /></section><section><h3>{t("projects.nextSteps")}</h3><BoundedList items={nextSteps} ordered emptyLabel={t("common.none")} /></section></div>;
}

function qualityIssueLabel(issue: MemoryQualityIssue, t: (key: string, options?: Record<string, unknown>) => string) {
  return t(`contextTrust.quality_${issue.code}`, { section: issue.section });
}
