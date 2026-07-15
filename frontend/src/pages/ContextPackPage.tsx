import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { aiProviderApi } from "../api/ai-provider";
import { aiCandidateApi } from "../api/ai-candidate";
import { contextPackApi } from "../api/context-pack";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { AiInvocationResult, AiProvider } from "../types/ai-provider";
import {
  AI_CANDIDATE_TYPES,
  type AiCandidate,
  type AiCandidateType,
} from "../types/ai-candidate";
import {
  CONTEXT_TEMPLATES,
  type ContextExport,
  type ContextTemplateType,
} from "../types/context-pack";
import type { Project } from "../types/project";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";

export function ContextPackPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [exports, setExports] = useState<ContextExport[]>([]);
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [selectedExportId, setSelectedExportId] = useState("");
  const [templateType, setTemplateType] = useState<ContextTemplateType>("planning");
  const [providerId, setProviderId] = useState("");
  const [userPrompt, setUserPrompt] = useState("");
  const [response, setResponse] = useState<AiInvocationResult>();
  const [candidateType, setCandidateType] = useState<AiCandidateType>("general_note");
  const [candidateTitle, setCandidateTitle] = useState("");
  const [savedCandidate, setSavedCandidate] = useState<AiCandidate>();
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [invoking, setInvoking] = useState(false);
  const [savingCandidate, setSavingCandidate] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const generation = useRef(0);

  const enabledProviders = useMemo(
    () => providers.filter((provider) => provider.enabled),
    [providers],
  );
  const selectedExport = useMemo(
    () => exports.find((item) => item.id === selectedExportId),
    [exports, selectedExportId],
  );

  const load = useCallback(async () => {
    const requestGeneration = ++generation.current;
    if (!projectId) {
      setError(t("context.projectMissing"));
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, exportValues, providerValues] = await Promise.all([
        projectKernelApi.getProject(projectId),
        contextPackApi.list(projectId),
        aiProviderApi.list(),
      ]);
      if (requestGeneration !== generation.current) return;
      setProject(projectValue);
      setExports(exportValues);
      setSelectedExportId((current) =>
        exportValues.some((item) => item.id === current)
          ? current
          : (exportValues[0]?.id ?? ""),
      );
      setProviders(providerValues);
      const enabled = providerValues.filter((provider) => provider.enabled);
      setProviderId((current) =>
        enabled.some((provider) => provider.id === current)
          ? current
          : (enabled[0]?.id ?? ""),
      );
    } catch (loadError) {
      if (requestGeneration === generation.current) {
        setError(getErrorMessage(loadError));
      }
    } finally {
      if (requestGeneration === generation.current) setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void load();
    return () => {
      generation.current += 1;
    };
  }, [load]);

  async function handleGenerate() {
    if (!projectId) return;
    setGenerating(true);
    setError(undefined);
    setNotice(undefined);
      setResponse(undefined);
      setSavedCandidate(undefined);
    try {
      const generated = await contextPackApi.generate(projectId, templateType);
      setExports((current) => [generated, ...current]);
      setSelectedExportId(generated.id);
      setNotice(t("context.generatedNotice"));
    } catch (generateError) {
      setError(getErrorMessage(generateError));
    } finally {
      setGenerating(false);
    }
  }

  async function handleCopy() {
    if (!selectedExport) return;
    try {
      await navigator.clipboard.writeText(selectedExport.contentMarkdown);
      setNotice(t("context.copiedNotice"));
      setError(undefined);
    } catch (copyError) {
      setError(t("context.copyFailed", { error: getErrorMessage(copyError) }));
    }
  }

  async function handleInvoke(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !selectedExport || !providerId || !userPrompt.trim()) return;
    setInvoking(true);
    setError(undefined);
    setNotice(undefined);
    setResponse(undefined);
    setSavedCandidate(undefined);
    try {
      const result = await contextPackApi.invokeWithAi(
        projectId,
        selectedExport.id,
        providerId,
        userPrompt.trim(),
      );
      setExports((current) =>
        current.map((item) =>
          item.id === result.contextExport.id ? result.contextExport : item,
        ),
      );
      setResponse(result.ai);
      setCandidateType("general_note");
      setCandidateTitle(t("context.suggestionTitle", { purpose: displayLabel(selectedExport.templateType) }));
      setSavedCandidate(undefined);
    } catch (invokeError) {
      setError(getErrorMessage(invokeError));
    } finally {
      setInvoking(false);
    }
  }

  async function handleSaveCandidate() {
    if (!projectId || !selectedExport || !response || !candidateTitle.trim()) return;
    setSavingCandidate(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const candidate = await aiCandidateApi.create(projectId, {
        candidateType,
        sourceContextExportId: selectedExport.id,
        title: candidateTitle.trim(),
        contentMarkdown: response.content,
      });
      setSavedCandidate(candidate);
      setNotice(t("context.suggestionSaved"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSavingCandidate(false);
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>{t("context.title")}</h1><p>{error ?? t("context.projectMissing")}</p></section>;
  }

  return (
    <div className="page-stack context-pack-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>{t("context.title")}</span>
      </div>

      <PageIntro title={t("context.title")} technicalTerm={t("terms.context")} description={t("context.description")} />

      <div className="ai-boundary-banner">
        <strong>{t("context.manualBoundary")}</strong>
        <span>{t("context.manualBoundaryDescription")}</span>
      </div>
      {error ? <div className="error-banner context-message">{error}</div> : null}
      {notice ? <div className="success-banner context-message">{notice}</div> : null}

      <section className="panel context-builder">
        <div className="panel-header">
          <div><h2>{t("context.prepare")}</h2><p>{t("context.rawNotice")}</p></div>
        </div>
        <div className="context-builder-controls">
          <label>{t("context.purpose")}
            <select value={templateType} onChange={(event) => setTemplateType(event.target.value as ContextTemplateType)}>
              {CONTEXT_TEMPLATES.map((template) => <option key={template} value={template}>{displayLabel(template)}</option>)}
            </select>
          </label>
          <button className="primary-button" disabled={generating} onClick={() => void handleGenerate()} type="button">
            {generating ? t("common.loading") : t("context.generate")}
          </button>
        </div>
      </section>

      <div className="context-workbench">
        <section className="panel context-export-list">
          <div className="panel-header"><div><h2>{t("context.previousExports")}</h2><p>{t("context.recordCount", { count: exports.length })}</p></div></div>
          {exports.length === 0 ? <p className="empty-state">{t("context.emptyExports")}</p> : (
            <div className="context-export-items record-list-pane">
              {exports.map((item) => (
                <button
                  className={item.id === selectedExportId ? "selected" : ""}
                  key={item.id}
                  onClick={() => { setSelectedExportId(item.id); setResponse(undefined); setSavedCandidate(undefined); setError(undefined); }}
                  type="button"
                >
                  <strong>{displayLabel(item.templateType)}</strong>
                  <span className={`context-status context-status--${item.status}`}>{displayLabel(item.status)}</span>
                  <small>{formatUpdatedAt(item.createdAt)}</small>
                </button>
              ))}
            </div>
          )}
        </section>

        <div className="context-detail-stack">
          {selectedExport ? (
            <>
              <section className="panel context-preview-panel">
                <div className="panel-header context-preview-header">
                  <div><h2>{t("context.rawPreview")}</h2><TechnicalDetails label={t("common.technicalDetails")}><CompactIdentifier label="SHA-256" value={selectedExport.contentHash} /></TechnicalDetails></div>
                  <button className="secondary-button" onClick={() => void handleCopy()} type="button">{t("context.copyRaw")}</button>
                </div>
                <div className="context-source-meta">
                  <span>{t("context.snapshotVersion", { version: selectedExport.snapshotVersion ?? t("common.none") })}</span>
                  <span>{t("context.sessionCount", { count: selectedExport.generatedFromSessionIds.length })}</span>
                  <span>{t("context.exportVersion", { version: selectedExport.version })}</span>
                </div>
                <p className="helper-text">{t("context.rawNotice")}</p><pre className="context-markdown-preview document-scroll-region">{selectedExport.contentMarkdown}</pre>
              </section>

              <form className="panel context-ai-panel" onSubmit={handleInvoke}>
                <div className="panel-header"><div><h2>{t("context.ask")}</h2><p>{t("context.description")}</p></div></div>
                <div className="context-ai-fields">
                  <label>{t("context.provider")}
                    <select required value={providerId} onChange={(event) => { setProviderId(event.target.value); setResponse(undefined); setSavedCandidate(undefined); }}>
                      <option value="">{t("context.chooseProvider")}</option>
                      {enabledProviders.map((provider) => <option key={provider.id} value={provider.id}>{provider.name} · {provider.model}</option>)}
                    </select>
                  </label>
                  <label>{t("context.question")} <span className="required">*</span>
                    <textarea required rows={5} value={userPrompt} onChange={(event) => setUserPrompt(event.target.value)} placeholder={t("context.questionPlaceholder")} />
                  </label>
                </div>
                <div className="form-actions context-ai-actions">
                  {enabledProviders.length === 0 ? <Link to="/settings/ai-providers">{t("context.noProvider")}</Link> : <span>{t("context.responseBoundary")}</span>}
                  <button className="primary-button" disabled={invoking || !providerId || !userPrompt.trim()} type="submit">{invoking ? t("context.analyzing") : t("context.send")}</button>
                </div>
                {response ? (
                  <div className="context-response-stack">
                    <div className="provider-result context-ai-result">
                      <header><strong>{t("context.response")}</strong><span>{response.providerName} · {response.model}</span></header>
                      <pre className="document-scroll-region">{response.content}</pre>
                      {response.usage ? <small>{t("context.tokenCount", { count: response.usage.totalTokens ?? t("common.unknown") })}</small> : null}
                    </div>
                    <div className="candidate-save-panel">
                      <header>
                        <div><strong>{t("context.saveSuggestion")}</strong><span>{t("ai.suggestionsDescription")}</span></div>
                        <Link to={`/projects/${project.id}/candidates`}>{t("navigation.aiSuggestions")}</Link>
                      </header>
                      <div className="candidate-save-fields">
                        <label>{t("context.suggestionType")}
                          <select disabled={Boolean(savedCandidate)} value={candidateType} onChange={(event) => setCandidateType(event.target.value as AiCandidateType)}>
                            {AI_CANDIDATE_TYPES.map((type) => <option key={type} value={type}>{displayLabel(type)}</option>)}
                          </select>
                        </label>
                        <label>{t("context.suggestionTitleField")} <span className="required">*</span>
                          <input disabled={Boolean(savedCandidate)} required maxLength={200} value={candidateTitle} onChange={(event) => setCandidateTitle(event.target.value)} />
                        </label>
                      </div>
                      <div className="candidate-save-actions">
                        <span>{savedCandidate ? t("context.savedSuggestion", { version: savedCandidate.version }) : t("context.memoryOnly")}</span>
                        <button className="secondary-button" disabled={savingCandidate || Boolean(savedCandidate) || !candidateTitle.trim()} onClick={() => void handleSaveCandidate()} type="button">
                          {savingCandidate ? t("common.saving") : savedCandidate ? t("common.saved") : t("context.saveSuggestion")}
                        </button>
                      </div>
                    </div>
                  </div>
                ) : null}
              </form>
            </>
          ) : <section className="panel provider-detail-empty"><strong>{t("context.generate")}</strong><p>{t("context.description")}</p></section>}
        </div>
      </div>
    </div>
  );
}
