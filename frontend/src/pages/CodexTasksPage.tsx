import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { contextPackApi } from "../api/context-pack";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { CodexTaskProposal, CodexTaskProposalStatus } from "../types/codex-task-proposal";
import type { ContextExport } from "../types/context-pack";
import type { Project } from "../types/project";

const STATUS: Record<CodexTaskProposalStatus, string> = { draft: "草稿", proposed: "待审批", approved: "已批准", rejected: "已拒绝", archived: "已归档" };
const TEMPLATE: Record<ContextExport["templateType"], string> = { planning: "规划", coding: "编码", debugging: "调试", review: "审查", handoff: "交接" };
const lines = (value: string) => value.split("\n").map((item) => item.trim()).filter(Boolean);

export function CodexTasksPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [exports, setExports] = useState<ContextExport[]>([]);
  const [items, setItems] = useState<CodexTaskProposal[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [form, setForm] = useState({ contextExportId: "", title: "", description: "", goal: "", nonGoals: "", allowedScope: "", forbiddenScope: "", acceptanceCriteria: "", validationCommands: "" });
  const selected = useMemo(() => items.find((item) => item.id === selectedId), [items, selectedId]);

  const load = useCallback(async () => {
    if (!projectId) return;
    try {
      const [p, contexts, proposals] = await Promise.all([projectKernelApi.getProject(projectId), contextPackApi.list(projectId), codexTaskProposalApi.list(projectId)]);
      setProject(p); setExports(contexts); setItems(proposals);
      setSelectedId((current) => proposals.some((item) => item.id === current) ? current : (proposals[0]?.id ?? ""));
      setForm((current) => ({ ...current, contextExportId: current.contextExportId || contexts[0]?.id || "" }));
    } catch (value) { setError(getErrorMessage(value)); }
  }, [projectId]);
  useEffect(() => { void load(); }, [load]);

  async function create(event: FormEvent) {
    event.preventDefault(); if (!projectId) return;
    setBusy(true); setError(undefined); setNotice(undefined);
    try {
      const created = await codexTaskProposalApi.create(projectId, { contextExportId: form.contextExportId, title: form.title, description: form.description, goal: form.goal, nonGoals: lines(form.nonGoals), allowedScope: lines(form.allowedScope), forbiddenScope: lines(form.forbiddenScope), acceptanceCriteria: lines(form.acceptanceCriteria), validationCommands: lines(form.validationCommands) });
      setItems((current) => [created, ...current]); setSelectedId(created.id); setNotice("任务提案草稿已保存；没有执行代码。");
    } catch (value) { setError(getErrorMessage(value)); } finally { setBusy(false); }
  }

  async function transition(action: "submit" | "approve" | "reject" | "archive") {
    if (!projectId || !selected) return;
    const actionLabel = { submit: "提交审批", approve: "批准", reject: "拒绝", archive: "归档" }[action];
    const message = action === "approve" ? "批准只代表授权未来可能的执行，不会启动 Codex 或修改代码。确认？" : `${actionLabel}会记录状态历史，不会执行代码。确认？`;
    if (!window.confirm(message)) return;
    setBusy(true); setError(undefined); setNotice(undefined);
    try {
      const updated = await codexTaskProposalApi[action](projectId, selected.id, selected.version);
      setItems((current) => current.map((item) => item.id === updated.id ? updated : item)); setNotice(`${STATUS[updated.status]}已记录；当前仅为任务提案。`);
    } catch (value) { setError(getErrorMessage(value)); await load(); } finally { setBusy(false); }
  }

  if (!project || !projectId) return <p className="loading-copy">正在加载 Codex 任务提案…</p>;
  return <div className="page-stack codex-task-page">
    <div className="breadcrumb"><Link to="/projects">项目</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>Codex 任务提案</span></div>
    <header className="page-header"><div><p className="eyebrow">D4 · 提案治理</p><h1>Codex 任务提案</h1><p>定义受上下文包（Context Pack）约束的编码请求。当前仅为提案，不会执行代码。</p></div><Link className="secondary-button button-link" to={`/projects/${project.id}/codex-executions`}>查看执行记录</Link></header>
    <div className="ai-boundary-banner"><strong>不会执行 Codex</strong><span>批准只记录人工授权；不会创建分支、修改文件、提交、推送或调用智能体（Agent）。</span></div>
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
    <form className="panel proposal-form" onSubmit={create}>
      <div className="panel-header"><div><h2>新建提案草稿</h2><p>范围、标准和命令均为每行一项。</p></div></div>
      <label>上下文包（Context Pack）<select required value={form.contextExportId} onChange={(e) => setForm({ ...form, contextExportId: e.target.value })}><option value="">请选择已确认的上下文</option>{exports.map((item) => <option key={item.id} value={item.id}>{TEMPLATE[item.templateType]} · {item.contentHash.slice(0, 10)} · {new Date(item.createdAt).toLocaleString()}</option>)}</select></label>
      <div className="form-grid"><label>标题<input required value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></label><label>任务目标<input required value={form.goal} onChange={(e) => setForm({ ...form, goal: e.target.value })} /></label></div>
      <label>任务说明<textarea value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} /></label>
      <div className="form-grid"><label>允许修改范围<textarea value={form.allowedScope} onChange={(e) => setForm({ ...form, allowedScope: e.target.value })} /></label><label>禁止修改范围<textarea value={form.forbiddenScope} onChange={(e) => setForm({ ...form, forbiddenScope: e.target.value })} /></label><label>非目标（明确不做什么）<textarea value={form.nonGoals} onChange={(e) => setForm({ ...form, nonGoals: e.target.value })} /></label><label>验收标准<textarea required value={form.acceptanceCriteria} onChange={(e) => setForm({ ...form, acceptanceCriteria: e.target.value })} /></label></div>
      <label>验证命令<textarea value={form.validationCommands} onChange={(e) => setForm({ ...form, validationCommands: e.target.value })} /></label>
      <button className="primary-button" disabled={busy || exports.length === 0} type="submit">保存提案草稿</button>
    </form>
    <div className="candidate-workbench"><section className="panel candidate-list-panel"><h2>任务提案</h2><div className="candidate-list">{items.map((item) => <button className={item.id === selectedId ? "selected" : ""} key={item.id} onClick={() => setSelectedId(item.id)} type="button"><span className={`candidate-status candidate-status--${item.status}`}>{STATUS[item.status]}</span><strong>{item.title}</strong><small>版本 v{item.version}</small></button>)}</div></section>
      {selected ? <section className="panel candidate-detail-panel"><div className="panel-header"><div><span className={`candidate-status candidate-status--${selected.status}`}>{STATUS[selected.status]}</span><h2>{selected.title}</h2><p>来源上下文 · {selected.contextExportId}</p></div></div><h3>任务目标</h3><p>{selected.goal}</p><h3>允许修改范围</h3><pre className="candidate-content">{selected.allowedScope.join("\n") || "尚未记录"}</pre><h3>验收标准</h3><pre className="candidate-content">{selected.acceptanceCriteria.join("\n")}</pre><div className="candidate-review-actions"><span>状态变化只写入任务提案与历史记录。</span><div>{selected.status === "draft" ? <button onClick={() => void transition("submit")} type="button">提交审批</button> : null}{selected.status === "proposed" ? <><button className="primary-button" onClick={() => void transition("approve")} type="button">批准</button><button className="danger-button" onClick={() => void transition("reject")} type="button">拒绝</button></> : null}{selected.status === "approved" ? <button onClick={() => void transition("archive")} type="button">归档</button> : null}</div></div></section> : null}
    </div>
  </div>;
}
