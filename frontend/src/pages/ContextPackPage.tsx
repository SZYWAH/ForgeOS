import { type FormEvent, useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { projectMemoryApi } from "../api/project-memory";
import { contextPackApi } from "../api/context-pack";
import { aiProviderApi } from "../api/ai-provider";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { ProjectMemoryNav } from "../components/ProjectMemoryNav";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { AiProvider, AiInvocationResult } from "../types/ai-provider";
import { CONTEXT_TEMPLATES, type ContextExport, type ContextTemplateType } from "../types/context-pack";
import type { ProjectAiContext } from "../types/project-memory";
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
    setLoading(true); setError(undefined);
    try {
      const [projectValue, contextValue, exportValues, providerValues] = await Promise.all([
        projectKernelApi.getProject(projectId), projectMemoryApi.buildAiContext(projectId), contextPackApi.list(projectId), aiProviderApi.list(),
      ]);
      setProject(projectValue); setContext(contextValue); setExports(exportValues);
      setProviders(providerValues.filter((value) => value.enabled));
      setProviderId((current) => current || providerValues.find((value) => value.enabled)?.id || "");
    } catch (reason) { setError(getErrorMessage(reason)); }
    finally { setLoading(false); }
  }, [projectId]);
  useEffect(() => { void load(); }, [load]);

  async function copyContext() {
    if (!context) return;
    try { await navigator.clipboard.writeText(context.contentMarkdown); setNotice(t("memory.contextCopied")); }
    catch (reason) { setError(getErrorMessage(reason)); }
  }
  async function generate() {
    if (!projectId) return; setWorking(true); setError(undefined);
    try { const value = await contextPackApi.generate(projectId, template); setSelectedExport(value); setExports((items) => [value, ...items.filter((item) => item.id !== value.id)]); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }
  async function askAi(event: FormEvent) {
    event.preventDefault(); if (!projectId || !providerId || !prompt.trim()) return; setWorking(true); setError(undefined);
    try { const exportValue = selectedExport ?? await contextPackApi.generate(projectId, template); setSelectedExport(exportValue); const value = await contextPackApi.invokeWithAi(projectId, exportValue.id, providerId, prompt.trim()); setResponse(value.ai); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) return <div className="error-banner">{error ?? t("projects.noProject")}</div>;
  return <div className="page-stack ai-context-page">
    <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("memory.aiContext")}</span></div>
    <PageIntro title={t("memory.aiContext")} description={t("memory.aiContextDescription")} primaryAction={<button className="primary-button" disabled={!context} onClick={() => void copyContext()} type="button">{t("memory.copyContext")}</button>} />
    <ProjectMemoryNav projectId={project.id} />
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
    <section className="ai-context-primary">
      <header><div><h2>{t("memory.aiContext")}</h2><p>{t("memory.contextPrimaryHelp")}</p></div>{context ? <CompactIdentifier copyable label="SHA-256" value={context.contentHash} /> : null}</header>
      <pre className="context-markdown-preview document-scroll-region">{context?.contentMarkdown ?? t("common.noData")}</pre>
    </section>
    <details className="mcp-guidance"><summary>{t("memory.mcpGuide")}</summary><div><p>{t("memory.mcpGuideDescription")}</p><p>{t("memory.mcpReadOnly")}</p></div></details>
    <details className="advanced-ai-context"><summary>{t("memory.advancedAiTools")}</summary><div className="advanced-ai-context__body">
      <section><h2>{t("context.prepare")}</h2><div className="inline-controls"><label>{t("context.purpose")}<select value={template} onChange={(event) => setTemplate(event.target.value as ContextTemplateType)}>{CONTEXT_TEMPLATES.map((value) => <option key={value} value={value}>{displayLabel(value)}</option>)}</select></label><button className="secondary-button" disabled={working} onClick={() => void generate()} type="button">{t("context.generate")}</button></div>{selectedExport ? <pre className="document-scroll-region">{selectedExport.contentMarkdown}</pre> : null}</section>
      <form onSubmit={askAi}><h2>{t("context.ask")}</h2><div className="form-grid"><label>{t("context.provider")}<select value={providerId} onChange={(event) => setProviderId(event.target.value)}><option value="">{t("context.chooseProvider")}</option>{providers.map((value) => <option key={value.id} value={value.id}>{value.name} · {value.model}</option>)}</select></label><label className="span-2">{t("context.question")}<textarea rows={4} value={prompt} onChange={(event) => setPrompt(event.target.value)} /></label></div><button className="secondary-button" disabled={working || !providerId || !prompt.trim()} type="submit">{t("context.send")}</button>{response ? <pre className="document-scroll-region">{response.content}</pre> : null}</form>
      <section><h2>{t("context.previousExports")}</h2>{exports.length === 0 ? <p>{t("context.emptyExports")}</p> : <div className="context-export-list">{exports.map((value) => <button key={value.id} onClick={() => setSelectedExport(value)} type="button"><strong>{displayLabel(value.templateType)}</strong><span>{formatUpdatedAt(value.createdAt)}</span></button>)}</div>}</section>
    </div></details>
  </div>;
}
