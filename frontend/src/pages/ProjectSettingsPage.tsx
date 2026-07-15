import { type FormEvent, useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { LIFECYCLE_STAGES, PROJECT_STATUSES, type Project } from "../types/project";
import { lifecycleLabel, statusLabel } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";
import { repositoryApi } from "../api/repository";

export function ProjectSettingsPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [draft, setDraft] = useState<Project>();
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [repositoryCount, setRepositoryCount] = useState(0);

  const load = useCallback(async () => {
    if (!projectId) return;
    try { const [value, repositories] = await Promise.all([projectKernelApi.getProject(projectId), repositoryApi.list(projectId)]); setProject(value); setDraft(value); setRepositoryCount(repositories.length); setError(undefined); }
    catch (value) { setError(getErrorMessage(value)); }
  }, [projectId]);
  useEffect(() => { void load(); }, [load]);

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!projectId || !draft) return;
    setSaving(true); setError(undefined); setNotice(undefined);
    try {
      const value = await projectKernelApi.updateProject(projectId, { slug: draft.slug.trim(), name: draft.name.trim(), description: draft.description.trim(), vision: draft.vision.trim(), targetUser: draft.targetUser.trim(), scope: draft.scope.trim(), nonGoals: draft.nonGoals.trim(), techStack: draft.techStack.trim(), status: draft.status, lifecycleStage: draft.lifecycleStage, priority: draft.priority, expectedVersion: draft.version });
      setProject(value); setDraft(value); setNotice(t("projects.settingsSaved", { version: value.version }));
    } catch (value) { setError(getErrorCode(value) === "version_conflict" ? t("projects.settingsConflict") : getErrorMessage(value)); }
    finally { setSaving(false); }
  }

  async function archive() {
    if (!projectId || !draft || draft.status === "archived" || !window.confirm(t("projects.archiveConfirm", { name: draft.name }))) return;
    setSaving(true);
    try { const value = await projectKernelApi.archiveProject(projectId, draft.version); setProject(value); setDraft(value); setNotice(t("projects.archivedNotice")); }
    catch (value) { setError(getErrorMessage(value)); } finally { setSaving(false); }
  }

  if (!draft || !project) return <div className="page-stack"><p className="loading-copy">{t("common.loading")}…</p>{error ? <div className="error-banner">{error}</div> : null}</div>;
  const archived = project.status === "archived";
  return <div className="page-stack">
    <div className="breadcrumb"><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("navigation.projectSettings")}</span></div>
    <PageIntro title={t("navigation.projectSettings")} description={t("projects.description")} secondaryActions={<button className="danger-button" disabled={saving || archived} onClick={() => void archive()} type="button">{archived ? t("projects.archived") : t("projects.archive")}</button>} />
    <form className="project-form" onSubmit={save}><fieldset className="editor-fields" disabled={archived}>
      <section className="panel"><div className="panel-header"><div><h2>{t("projects.basicInfo")}</h2><p>{t("projects.projectDescription")}</p></div></div><div className="form-grid"><label>{t("projects.name")}<input required value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} /></label><label>{t("projects.slug")}<input required pattern="[a-z0-9]+(?:-[a-z0-9]+)*" value={draft.slug} onChange={(event) => setDraft({ ...draft, slug: event.target.value })} /></label><label className="span-2">{t("projects.projectDescription")}<textarea rows={3} value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} /></label></div></section>
      <section className="panel"><div className="panel-header"><div><h2>{t("projects.direction")}</h2><p>{t("projects.overviewDescription")}</p></div></div><div className="form-grid"><label className="span-2">{t("projects.vision")}<textarea rows={3} value={draft.vision} onChange={(event) => setDraft({ ...draft, vision: event.target.value })} /></label><label className="span-2">{t("projects.targetUser")}<textarea rows={2} value={draft.targetUser} onChange={(event) => setDraft({ ...draft, targetUser: event.target.value })} /></label><label>{t("projects.scope")}<textarea rows={5} value={draft.scope} onChange={(event) => setDraft({ ...draft, scope: event.target.value })} /></label><label>{t("projects.nonGoals")}<textarea rows={5} value={draft.nonGoals} onChange={(event) => setDraft({ ...draft, nonGoals: event.target.value })} /></label></div></section>
      <section className="panel"><div className="panel-header"><div><h2>{t("projects.techStack")}</h2><p>{t("projects.lifecycle")}</p></div></div><div className="form-grid"><label className="span-2">{t("projects.techStack")}<textarea rows={3} value={draft.techStack} onChange={(event) => setDraft({ ...draft, techStack: event.target.value })} /></label><label>{t("projects.projectStatus")}<select value={draft.status} onChange={(event) => setDraft({ ...draft, status: event.target.value as Project["status"] })}>{PROJECT_STATUSES.filter((value) => value !== "archived" || archived).map((value) => <option key={value} value={value}>{statusLabel(value)}</option>)}</select></label><label>{t("projects.lifecycle")}<select value={draft.lifecycleStage} onChange={(event) => setDraft({ ...draft, lifecycleStage: event.target.value as Project["lifecycleStage"] })}>{LIFECYCLE_STAGES.map((value) => <option key={value} value={value}>{lifecycleLabel(value)}</option>)}</select></label><label>{t("projects.priority")}<input min={0} max={100} type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) })} /></label></div></section>
      <section className="panel project-settings-connections"><div className="panel-header"><div><h2>{t("memory.projectConnections")}</h2><p>{t("memory.projectConnectionsDescription")}</p></div></div><div><span>{t("memory.connectedRepositories", { count: repositoryCount })}</span><Link className="secondary-button button-link" to={`/projects/${project.id}/repositories`}>{t("memory.manageRepositories")}</Link><Link className="secondary-button button-link" to={`/projects/${project.id}/pulse`}>{t("memory.activitySettings")}</Link></div></section>
    </fieldset>{error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}<div className="sticky-actions"><TechnicalDetails label={t("common.technicalDetails")}><span>v{project.version}</span></TechnicalDetails><div><button className="secondary-button" disabled={saving || archived} onClick={() => setDraft(project)} type="button">{t("common.cancel")}</button><button className="primary-button" disabled={saving || archived} type="submit">{saving ? t("common.saving") : t("common.save")}</button></div></div></form>
  </div>;
}
