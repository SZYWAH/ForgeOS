import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { contextPackApi } from "../api/context-pack";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { CodexTaskProposal } from "../types/codex-task-proposal";
import type { ContextExport } from "../types/context-pack";
import type { Project } from "../types/project";
import { useTranslation } from "react-i18next";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";
import { displayLabel, formatUpdatedAt } from "../utils/format";

const lines = (value: string) => value.split("\n").map((item) => item.trim()).filter(Boolean);

export function CodexTasksPage() {
  const { t } = useTranslation();
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
      setItems((current) => [created, ...current]); setSelectedId(created.id); setNotice(t("codex.draftSaved"));
    } catch (value) { setError(getErrorMessage(value)); } finally { setBusy(false); }
  }

  async function transition(action: "submit" | "approve" | "reject" | "archive") {
    if (!projectId || !selected) return;
    const actionLabel = { submit: t("codex.submit"), approve: t("codex.approve"), reject: t("codex.reject"), archive: t("codex.archive") }[action];
    const message = action === "approve" ? t("codex.approveConfirm") : t("codex.transitionConfirm", { action: actionLabel });
    if (!window.confirm(message)) return;
    setBusy(true); setError(undefined); setNotice(undefined);
    try {
      const updated = await codexTaskProposalApi[action](projectId, selected.id, selected.version);
      setItems((current) => current.map((item) => item.id === updated.id ? updated : item)); setNotice(t("codex.statusRecorded", { status: displayLabel(updated.status) }));
    } catch (value) { setError(getErrorMessage(value)); await load(); } finally { setBusy(false); }
  }

  if (!project || !projectId) return <p className="loading-copy">{t("common.loading")}…</p>;
  return <div className="page-stack codex-task-page">
    <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("codex.tasksTitle")}</span></div>
    <PageIntro title={t("codex.tasksTitle")} technicalTerm={t("terms.taskProposal")} description={t("codex.tasksDescription")} secondaryActions={<Link className="secondary-button button-link" to={`/projects/${project.id}/codex-executions`}>{t("codex.executionsTitle")}</Link>} />
    <div className="ai-boundary-banner"><strong>{t("codex.proposalOnly")}</strong></div>
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
    <form className="panel proposal-form" onSubmit={create}>
      <div className="panel-header"><div><h2>{t("codex.newDraft")}</h2><p>{t("codex.lineHelp")}</p></div></div>
      <label>{t("context.title")}<select required value={form.contextExportId} onChange={(e) => setForm({ ...form, contextExportId: e.target.value })}><option value="">{t("common.none")}</option>{exports.map((item) => <option key={item.id} value={item.id}>{displayLabel(item.templateType)} · {formatUpdatedAt(item.createdAt)}</option>)}</select></label>
      <div className="form-grid"><label>{t("common.title")}<input required value={form.title} onChange={(e) => setForm({ ...form, title: e.target.value })} /></label><label>{t("codex.goal")}<input required value={form.goal} onChange={(e) => setForm({ ...form, goal: e.target.value })} /></label></div>
      <label>{t("codex.taskDescription")}<textarea value={form.description} onChange={(e) => setForm({ ...form, description: e.target.value })} /></label>
      <div className="form-grid"><label>{t("codex.allowedScope")}<textarea value={form.allowedScope} onChange={(e) => setForm({ ...form, allowedScope: e.target.value })} /></label><label>{t("codex.forbiddenScope")}<textarea value={form.forbiddenScope} onChange={(e) => setForm({ ...form, forbiddenScope: e.target.value })} /></label><label>{t("codex.nonGoals")}<textarea value={form.nonGoals} onChange={(e) => setForm({ ...form, nonGoals: e.target.value })} /></label><label>{t("codex.acceptance")}<textarea required value={form.acceptanceCriteria} onChange={(e) => setForm({ ...form, acceptanceCriteria: e.target.value })} /></label></div>
      <label>{t("codex.commands")}<textarea value={form.validationCommands} onChange={(e) => setForm({ ...form, validationCommands: e.target.value })} /></label>
      <button className="primary-button" disabled={busy || exports.length === 0} type="submit">{t("codex.saveDraft")}</button>
    </form>
    <div className="candidate-workbench"><section className="panel candidate-list-panel"><h2>{t("codex.taskList")}</h2><div className="candidate-list record-list-pane">{items.map((item) => <button className={item.id === selectedId ? "selected" : ""} key={item.id} onClick={() => setSelectedId(item.id)} type="button"><span className={`candidate-status candidate-status--${item.status}`}>{displayLabel(item.status)}</span><strong>{item.title}</strong><small>{t("codex.recordVersion", { version: item.version })}</small></button>)}</div></section>
      {selected ? <section className="panel candidate-detail-panel"><div className="panel-header"><div><span className={`candidate-status candidate-status--${selected.status}`}>{displayLabel(selected.status)}</span><h2>{selected.title}</h2><TechnicalDetails label={t("common.technicalDetails")}><code>{selected.contextExportId}</code></TechnicalDetails></div></div><h3>{t("codex.goal")}</h3><p>{selected.goal}</p><h3>{t("codex.allowedScope")}</h3><pre className="candidate-content document-scroll-region">{selected.allowedScope.join("\n") || t("common.none")}</pre><h3>{t("codex.acceptance")}</h3><pre className="candidate-content document-scroll-region">{selected.acceptanceCriteria.join("\n")}</pre><div className="candidate-review-actions"><span>{t("codex.proposalOnly")}</span><div>{selected.status === "draft" ? <button onClick={() => void transition("submit")} type="button">{t("codex.submit")}</button> : null}{selected.status === "proposed" ? <><button className="primary-button" onClick={() => void transition("approve")} type="button">{t("codex.approve")}</button><button className="danger-button" onClick={() => void transition("reject")} type="button">{t("codex.reject")}</button></> : null}{selected.status === "approved" ? <button onClick={() => void transition("archive")} type="button">{t("codex.archive")}</button> : null}</div></div></section> : null}
    </div>
  </div>;
}
