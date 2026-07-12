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
import { formatUpdatedAt } from "../utils/format";

const TEMPLATE_LABELS: Record<ContextTemplateType, string> = {
  planning: "Planning",
  coding: "Coding",
  debugging: "Debugging",
  review: "Review",
  handoff: "Handoff",
};

const CANDIDATE_TYPE_LABELS: Record<AiCandidateType, string> = {
  snapshot_update: "Snapshot Update",
  session_summary: "Session Summary",
  evidence_suggestion: "Evidence Suggestion",
  risk_finding: "Risk Finding",
  general_note: "General Note",
};

export function ContextPackPage() {
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
      setError("Project ID 缺失。");
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
      setNotice("Context Pack 已从当前确认事实确定性生成。");
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
      setNotice("Markdown 已复制到剪贴板。");
      setError(undefined);
    } catch (copyError) {
      setError(`复制失败：${getErrorMessage(copyError)}`);
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
      setCandidateTitle(`${TEMPLATE_LABELS[selectedExport.templateType]} AI analysis`);
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
      setNotice("AI Response 已保存为 Pending Candidate，尚未成为正式事实。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSavingCandidate(false);
    }
  }

  if (loading) return <p className="loading-copy">Loading Context Packs…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>Project unavailable</h1><p>{error}</p></section>;
  }

  return (
    <div className="page-stack context-pack-page">
      <div className="breadcrumb">
        <Link to="/projects">Projects</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>Context Pack</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">D1.5 · Derived Context</p>
          <h1>Context Pack</h1>
          <p>把已确认研发事实组织为稳定 Markdown；AI 只分析，不修改事实层。</p>
        </div>
      </header>

      <div className="ai-boundary-banner">
        <strong>显式调用边界</strong>
        <span>只有点击 Send to AI 才会发送。Prompt 与 Response 只保留在当前页面内存中。</span>
      </div>
      {error ? <div className="error-banner context-message">{error}</div> : null}
      {notice ? <div className="success-banner context-message">{notice}</div> : null}

      <section className="panel context-builder">
        <div className="panel-header">
          <div><h2>Context Builder</h2><p>模板只改变分析目的；事实来源和章节顺序固定。</p></div>
        </div>
        <div className="context-builder-controls">
          <label>Template
            <select value={templateType} onChange={(event) => setTemplateType(event.target.value as ContextTemplateType)}>
              {CONTEXT_TEMPLATES.map((template) => <option key={template} value={template}>{TEMPLATE_LABELS[template]}</option>)}
            </select>
          </label>
          <button className="primary-button" disabled={generating} onClick={() => void handleGenerate()} type="button">
            {generating ? "Generating…" : "Generate Context"}
          </button>
        </div>
      </section>

      <div className="context-workbench">
        <section className="panel context-export-list">
          <div className="panel-header"><div><h2>Exports</h2><p>{exports.length} generated records</p></div></div>
          {exports.length === 0 ? <p className="empty-state">尚未生成 Context Pack。</p> : (
            <div className="context-export-items">
              {exports.map((item) => (
                <button
                  className={item.id === selectedExportId ? "selected" : ""}
                  key={item.id}
                  onClick={() => { setSelectedExportId(item.id); setResponse(undefined); setSavedCandidate(undefined); setError(undefined); }}
                  type="button"
                >
                  <strong>{TEMPLATE_LABELS[item.templateType]}</strong>
                  <span className={`context-status context-status--${item.status}`}>{item.status === "sent_to_ai" ? "Sent to AI" : "Generated"}</span>
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
                  <div><h2>Markdown Preview</h2><p>SHA-256 · {selectedExport.contentHash}</p></div>
                  <button className="secondary-button" onClick={() => void handleCopy()} type="button">Copy Markdown</button>
                </div>
                <div className="context-source-meta">
                  <span>Snapshot v{selectedExport.snapshotVersion ?? "none"}</span>
                  <span>Sessions {selectedExport.generatedFromSessionIds.length}</span>
                  <span>Export v{selectedExport.version}</span>
                </div>
                <pre className="context-markdown-preview">{selectedExport.contentMarkdown}</pre>
              </section>

              <form className="panel context-ai-panel" onSubmit={handleInvoke}>
                <div className="panel-header"><div><h2>AI Analysis</h2><p>单次、用户触发、无对话历史。</p></div></div>
                <div className="context-ai-fields">
                  <label>Enabled Provider
                    <select required value={providerId} onChange={(event) => { setProviderId(event.target.value); setResponse(undefined); setSavedCandidate(undefined); }}>
                      <option value="">Select Provider</option>
                      {enabledProviders.map((provider) => <option key={provider.id} value={provider.id}>{provider.name} · {provider.model}</option>)}
                    </select>
                  </label>
                  <label>Question <span className="required">*</span>
                    <textarea required rows={5} value={userPrompt} onChange={(event) => setUserPrompt(event.target.value)} placeholder="例如：分析当前项目最大风险" />
                  </label>
                </div>
                <div className="form-actions context-ai-actions">
                  {enabledProviders.length === 0 ? <Link to="/settings/ai-providers">配置并启用 Provider</Link> : <span>响应不会写入数据库事实。</span>}
                  <button className="primary-button" disabled={invoking || !providerId || !userPrompt.trim()} type="submit">{invoking ? "Analyzing…" : "Send to AI"}</button>
                </div>
                {response ? (
                  <div className="context-response-stack">
                    <div className="provider-result context-ai-result">
                      <header><strong>Temporary response</strong><span>{response.providerName} · {response.model}</span></header>
                      <pre>{response.content}</pre>
                      {response.usage ? <small>Tokens: {response.usage.totalTokens ?? "unknown"}</small> : null}
                    </div>
                    <div className="candidate-save-panel">
                      <header>
                        <div><strong>Candidate governance</strong><span>可仅查看，也可由你明确保存为 Pending Candidate。</span></div>
                        <Link to={`/projects/${project.id}/candidates`}>Open Candidates</Link>
                      </header>
                      <div className="candidate-save-fields">
                        <label>Candidate Type
                          <select disabled={Boolean(savedCandidate)} value={candidateType} onChange={(event) => setCandidateType(event.target.value as AiCandidateType)}>
                            {AI_CANDIDATE_TYPES.map((type) => <option key={type} value={type}>{CANDIDATE_TYPE_LABELS[type]}</option>)}
                          </select>
                        </label>
                        <label>Title <span className="required">*</span>
                          <input disabled={Boolean(savedCandidate)} required maxLength={200} value={candidateTitle} onChange={(event) => setCandidateTitle(event.target.value)} />
                        </label>
                      </div>
                      <div className="candidate-save-actions">
                        <span>{savedCandidate ? `Saved as Pending · v${savedCandidate.version}` : "不保存时，Response 仍只存在于当前页面。"}</span>
                        <button className="secondary-button" disabled={savingCandidate || Boolean(savedCandidate) || !candidateTitle.trim()} onClick={() => void handleSaveCandidate()} type="button">
                          {savingCandidate ? "Saving…" : savedCandidate ? "Saved as Candidate" : "Save as Candidate"}
                        </button>
                      </div>
                    </div>
                  </div>
                ) : null}
              </form>
            </>
          ) : <section className="panel provider-detail-empty"><strong>Generate a Context Pack</strong><p>生成后可预览、复制或显式发送给 AI。</p></section>}
        </div>
      </div>
    </div>
  );
}
