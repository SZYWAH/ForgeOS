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
  planning: "规划",
  coding: "编码",
  debugging: "调试",
  review: "审查",
  handoff: "交接",
};

const CANDIDATE_TYPE_LABELS: Record<AiCandidateType, string> = {
  snapshot_update: "快照更新建议",
  session_summary: "会话总结",
  evidence_suggestion: "证据建议",
  risk_finding: "风险发现",
  general_note: "一般记录",
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
      setError("项目 ID 缺失。");
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
      setNotice("上下文包已根据当前确认事实确定性生成。");
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
      setCandidateTitle(`${TEMPLATE_LABELS[selectedExport.templateType]} AI 分析`);
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
      setNotice("AI 响应已保存为待审核候选建议，尚未成为正式事实。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSavingCandidate(false);
    }
  }

  if (loading) return <p className="loading-copy">正在加载上下文包…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>项目不可用</h1><p>{error}</p></section>;
  }

  return (
    <div className="page-stack context-pack-page">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>上下文包</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">D1.5 · 派生上下文</p>
          <h1>上下文包（Context Pack）</h1>
          <p>把已确认研发事实组织为稳定 Markdown；AI 只分析，不修改事实层。</p>
        </div>
      </header>

      <div className="ai-boundary-banner">
        <strong>显式调用边界</strong>
        <span>只有点击“发送给 AI”才会发送。提示词与响应只保留在当前页面内存中。</span>
      </div>
      {error ? <div className="error-banner context-message">{error}</div> : null}
      {notice ? <div className="success-banner context-message">{notice}</div> : null}

      <section className="panel context-builder">
        <div className="panel-header">
          <div><h2>上下文构建器</h2><p>模板只改变分析目的；事实来源和章节顺序固定。</p></div>
        </div>
        <div className="context-builder-controls">
          <label>模板
            <select value={templateType} onChange={(event) => setTemplateType(event.target.value as ContextTemplateType)}>
              {CONTEXT_TEMPLATES.map((template) => <option key={template} value={template}>{TEMPLATE_LABELS[template]}</option>)}
            </select>
          </label>
          <button className="primary-button" disabled={generating} onClick={() => void handleGenerate()} type="button">
            {generating ? "正在生成…" : "生成上下文"}
          </button>
        </div>
      </section>

      <div className="context-workbench">
        <section className="panel context-export-list">
          <div className="panel-header"><div><h2>导出记录</h2><p>{exports.length} 条已生成记录</p></div></div>
          {exports.length === 0 ? <p className="empty-state">尚未生成上下文包。</p> : (
            <div className="context-export-items">
              {exports.map((item) => (
                <button
                  className={item.id === selectedExportId ? "selected" : ""}
                  key={item.id}
                  onClick={() => { setSelectedExportId(item.id); setResponse(undefined); setSavedCandidate(undefined); setError(undefined); }}
                  type="button"
                >
                  <strong>{TEMPLATE_LABELS[item.templateType]}</strong>
                  <span className={`context-status context-status--${item.status}`}>{item.status === "sent_to_ai" ? "已发送给 AI" : "已生成"}</span>
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
                  <div><h2>Markdown 预览</h2><p>SHA-256 · {selectedExport.contentHash}</p></div>
                  <button className="secondary-button" onClick={() => void handleCopy()} type="button">复制 Markdown</button>
                </div>
                <div className="context-source-meta">
                  <span>快照 v{selectedExport.snapshotVersion ?? "无"}</span>
                  <span>会话 {selectedExport.generatedFromSessionIds.length} 条</span>
                  <span>导出版本 v{selectedExport.version}</span>
                </div>
                <pre className="context-markdown-preview">{selectedExport.contentMarkdown}</pre>
              </section>

              <form className="panel context-ai-panel" onSubmit={handleInvoke}>
                <div className="panel-header"><div><h2>AI 分析</h2><p>单次、用户触发、无对话历史。</p></div></div>
                <div className="context-ai-fields">
                  <label>已启用的 AI 服务
                    <select required value={providerId} onChange={(event) => { setProviderId(event.target.value); setResponse(undefined); setSavedCandidate(undefined); }}>
                      <option value="">选择 AI 服务</option>
                      {enabledProviders.map((provider) => <option key={provider.id} value={provider.id}>{provider.name} · {provider.model}</option>)}
                    </select>
                  </label>
                  <label>问题 <span className="required">*</span>
                    <textarea required rows={5} value={userPrompt} onChange={(event) => setUserPrompt(event.target.value)} placeholder="例如：分析当前项目最大风险" />
                  </label>
                </div>
                <div className="form-actions context-ai-actions">
                  {enabledProviders.length === 0 ? <Link to="/settings/ai-providers">配置并启用 AI 服务</Link> : <span>响应不会写入数据库事实。</span>}
                  <button className="primary-button" disabled={invoking || !providerId || !userPrompt.trim()} type="submit">{invoking ? "正在分析…" : "发送给 AI"}</button>
                </div>
                {response ? (
                  <div className="context-response-stack">
                    <div className="provider-result context-ai-result">
                      <header><strong>临时响应</strong><span>{response.providerName} · {response.model}</span></header>
                      <pre>{response.content}</pre>
                      {response.usage ? <small>Token 数：{response.usage.totalTokens ?? "未知"}</small> : null}
                    </div>
                    <div className="candidate-save-panel">
                      <header>
                        <div><strong>候选建议治理</strong><span>可仅查看，也可由你明确保存为待审核候选建议。</span></div>
                        <Link to={`/projects/${project.id}/candidates`}>打开候选建议</Link>
                      </header>
                      <div className="candidate-save-fields">
                        <label>候选类型
                          <select disabled={Boolean(savedCandidate)} value={candidateType} onChange={(event) => setCandidateType(event.target.value as AiCandidateType)}>
                            {AI_CANDIDATE_TYPES.map((type) => <option key={type} value={type}>{CANDIDATE_TYPE_LABELS[type]}</option>)}
                          </select>
                        </label>
                        <label>标题 <span className="required">*</span>
                          <input disabled={Boolean(savedCandidate)} required maxLength={200} value={candidateTitle} onChange={(event) => setCandidateTitle(event.target.value)} />
                        </label>
                      </div>
                      <div className="candidate-save-actions">
                        <span>{savedCandidate ? `已保存为待审核 · v${savedCandidate.version}` : "不保存时，响应仍只存在于当前页面。"}</span>
                        <button className="secondary-button" disabled={savingCandidate || Boolean(savedCandidate) || !candidateTitle.trim()} onClick={() => void handleSaveCandidate()} type="button">
                          {savingCandidate ? "正在保存…" : savedCandidate ? "已保存为候选建议" : "保存为候选建议"}
                        </button>
                      </div>
                    </div>
                  </div>
                ) : null}
              </form>
            </>
          ) : <section className="panel provider-detail-empty"><strong>生成一个上下文包</strong><p>生成后可预览、复制或显式发送给 AI。</p></section>}
        </div>
      </div>
    </div>
  );
}
