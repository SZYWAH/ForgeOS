import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { contextPackApi } from "../api/context-pack";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { CodexTaskProposal, CodexTaskProposalStatus } from "../types/codex-task-proposal";
import type { ContextExport } from "../types/context-pack";
import type { Project } from "../types/project";

const STATUS: Record<CodexTaskProposalStatus, string> = { draft: "Draft", proposed: "Proposed", approved: "Approved", rejected: "Rejected", archived: "Archived" };
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
      setItems((current) => [created, ...current]); setSelectedId(created.id); setNotice("Draft Proposal 已保存；没有执行代码。");
    } catch (value) { setError(getErrorMessage(value)); } finally { setBusy(false); }
  }

  async function transition(action: "submit" | "approve" | "reject" | "archive") {
    if (!projectId || !selected) return;
    const message = action === "approve" ? "Approve 只授权未来可能的执行，不会启动 Codex 或修改代码。确认？" : `${action} 会记录状态历史，不会执行代码。确认？`;
    if (!window.confirm(message)) return;
    setBusy(true); setError(undefined); setNotice(undefined);
    try {
      const updated = await codexTaskProposalApi[action](projectId, selected.id, selected.version);
      setItems((current) => current.map((item) => item.id === updated.id ? updated : item)); setNotice(`${STATUS[updated.status]} 已记录；Proposal only。`);
    } catch (value) { setError(getErrorMessage(value)); await load(); } finally { setBusy(false); }
  }

  if (!project || !projectId) return <p className="loading-copy">Loading Codex Task Proposals…</p>;
  return <div className="page-stack codex-task-page">
    <div className="breadcrumb"><Link to="/projects">Projects</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>Codex Tasks</span></div>
    <header className="page-header"><div><p className="eyebrow">D4 · Proposal Governance</p><h1>Codex Task Proposals</h1><p>定义受 Context Pack 约束的编码请求。Proposal only，不会执行代码。</p></div><Link className="secondary-button button-link" to={`/projects/${project.id}/codex-executions`}>查看执行记录</Link></header>
    <div className="ai-boundary-banner"><strong>不会执行 Codex</strong><span>Approve 只记录人工授权；不会创建分支、修改文件、提交、推送或调用 Agent。</span></div>
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
    <form className="panel proposal-form" onSubmit={create}>
      <div className="panel-header"><div><h2>Create Draft</h2><p>每行表示一项范围、标准或命令。</p></div></div>
      <label>Context Pack<select required value={form.contextExportId} onChange={(e) => setForm({ ...form, contextExportId: e.target.value })}><option value="">Select confirmed context</option>{exports.map((item) => <option key={item.id} value={item.id}>{item.templateType} · {item.contentHash.slice(0, 10)} · {new Date(item.createdAt).toLocaleString()}</option>)}</select></label>
      <div className="form-grid"><label>Title<input required value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></label><label>Goal<input required value={form.goal} onChange={(e) => setForm({ ...form, goal: e.target.value })} /></label></div>
      <label>Description<textarea value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} /></label>
      <div className="form-grid"><label>Allowed Scope<textarea value={form.allowedScope} onChange={(e) => setForm({ ...form, allowedScope: e.target.value })} /></label><label>Forbidden Scope<textarea value={form.forbiddenScope} onChange={(e) => setForm({ ...form, forbiddenScope: e.target.value })} /></label><label>Non Goals<textarea value={form.nonGoals} onChange={(e) => setForm({ ...form, nonGoals: e.target.value })} /></label><label>Acceptance Criteria<textarea required value={form.acceptanceCriteria} onChange={(e) => setForm({ ...form, acceptanceCriteria: e.target.value })} /></label></div>
      <label>Validation Commands<textarea value={form.validationCommands} onChange={(e) => setForm({ ...form, validationCommands: e.target.value })} /></label>
      <button className="primary-button" disabled={busy || exports.length === 0} type="submit">Save Draft Proposal</button>
    </form>
    <div className="candidate-workbench"><section className="panel candidate-list-panel"><h2>Proposals</h2><div className="candidate-list">{items.map((item) => <button className={item.id === selectedId ? "selected" : ""} key={item.id} onClick={() => setSelectedId(item.id)} type="button"><span className={`candidate-status candidate-status--${item.status}`}>{STATUS[item.status]}</span><strong>{item.title}</strong><small>v{item.version}</small></button>)}</div></section>
      {selected ? <section className="panel candidate-detail-panel"><div className="panel-header"><div><span className={`candidate-status candidate-status--${selected.status}`}>{STATUS[selected.status]}</span><h2>{selected.title}</h2><p>Context Export · {selected.contextExportId}</p></div></div><h3>Goal</h3><p>{selected.goal}</p><h3>Allowed Scope</h3><pre className="candidate-content">{selected.allowedScope.join("\n") || "None recorded"}</pre><h3>Acceptance Criteria</h3><pre className="candidate-content">{selected.acceptanceCriteria.join("\n")}</pre><div className="candidate-review-actions"><span>状态变化只写 Proposal 与历史记录。</span><div>{selected.status === "draft" ? <button onClick={() => void transition("submit")} type="button">Submit</button> : null}{selected.status === "proposed" ? <><button className="primary-button" onClick={() => void transition("approve")} type="button">Approve</button><button className="danger-button" onClick={() => void transition("reject")} type="button">Reject</button></> : null}{selected.status === "approved" ? <button onClick={() => void transition("archive")} type="button">Archive</button> : null}</div></div></section> : null}
    </div>
  </div>;
}
