import { type FormEvent, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { projectKernelApi, getErrorMessage } from "../api/project-kernel";
import { projectMemoryApi } from "../api/project-memory";
import { EmptyState, PageIntro } from "../components/ui/PlainLanguage";
import {
  LIFECYCLE_STAGES,
  PROJECT_STATUSES,
  type CreateProjectInput,
} from "../types/project";
import type { ProjectMemorySummary } from "../types/project-memory";
import { formatRelativeTime, lifecycleLabel, statusLabel, toProjectSlug } from "../utils/format";

const INITIAL_FORM: CreateProjectInput = {
  slug: "",
  name: "",
  description: "",
  vision: "",
  targetUser: "",
  scope: "",
  nonGoals: "",
  techStack: "",
  status: "idea",
  lifecycleStage: "discovery",
  priority: 50,
};

export function ProjectsPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [projects, setProjects] = useState<ProjectMemorySummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();
  const [formOpen, setFormOpen] = useState(false);
  const [form, setForm] = useState(INITIAL_FORM);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    void load();
  }, []);

  async function load() {
    setLoading(true);
    setError(undefined);
    try {
      setProjects(await projectMemoryApi.listSummaries());
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setLoading(false);
    }
  }

  async function createProject(event: FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    setError(undefined);
    try {
      const project = await projectKernelApi.createProject({
        ...form,
        name: form.name.trim(),
        slug: form.slug.trim(),
        description: form.description.trim(),
      });
      navigate(`/projects/${project.id}`);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setSubmitting(false);
    }
  }

  const active = useMemo(
    () => projects.filter((project) => project.status !== "archived"),
    [projects],
  );
  const archived = useMemo(
    () => projects.filter((project) => project.status === "archived"),
    [projects],
  );

  return (
    <div className="page-stack memory-projects-page">
      <PageIntro
        title={t("projects.title")}
        description={t("memory.projectListDescription")}
        primaryAction={
          <details className="add-project-menu">
            <summary className="primary-button">{t("memory.addProject")}</summary>
            <div className="add-project-menu__items">
              <button type="button" onClick={() => setFormOpen(true)}>{t("memory.newProject")}</button>
              <button type="button" onClick={() => navigate("/github-import")}>{t("memory.importFromGithub")}</button>
              <button type="button" onClick={() => navigate("/github-import?mode=analysis")}>{t("githubAnalysis.analyzeProject")}</button>
            </div>
          </details>
        }
      />

      {formOpen ? (
        <section className="memory-create-panel" aria-label={t("memory.newProject")}>
          <div className="memory-create-panel__heading">
            <div><h2>{t("memory.newProject")}</h2><p>{t("memory.createProjectHelp")}</p></div>
            <button className="text-button" type="button" onClick={() => setFormOpen(false)}>{t("common.close")}</button>
          </div>
          <form className="project-form compact-form" onSubmit={createProject}>
            <div className="form-grid">
              <label>{t("projects.name")}<input autoFocus required maxLength={200} value={form.name} onChange={(event) => setForm((current) => ({ ...current, name: event.target.value, slug: toProjectSlug(event.target.value) }))} /></label>
              <label>{t("projects.slug")}<input required minLength={2} maxLength={64} pattern="[a-z0-9]+(?:-[a-z0-9]+)*" value={form.slug} onChange={(event) => setForm((current) => ({ ...current, slug: event.target.value }))} /></label>
              <label className="span-2">{t("memory.shortDescription")}<textarea rows={2} maxLength={2000} value={form.description} onChange={(event) => setForm((current) => ({ ...current, description: event.target.value }))} /></label>
              <label>{t("projects.projectStatus")}<select value={form.status} onChange={(event) => setForm((current) => ({ ...current, status: event.target.value as CreateProjectInput["status"] }))}>{PROJECT_STATUSES.filter((value) => value !== "archived").map((value) => <option key={value} value={value}>{statusLabel(value)}</option>)}</select></label>
              <label>{t("projects.lifecycle")}<select value={form.lifecycleStage} onChange={(event) => setForm((current) => ({ ...current, lifecycleStage: event.target.value as CreateProjectInput["lifecycleStage"] }))}>{LIFECYCLE_STAGES.map((value) => <option key={value} value={value}>{lifecycleLabel(value)}</option>)}</select></label>
            </div>
            <div className="form-actions"><button type="button" className="secondary-button" onClick={() => setFormOpen(false)}>{t("common.cancel")}</button><button className="primary-button" disabled={submitting} type="submit">{submitting ? t("common.saving") : t("projects.createProject")}</button></div>
          </form>
        </section>
      ) : null}

      {error ? <div className="error-banner">{error}<button className="text-button" type="button" onClick={() => void load()}>{t("common.retry")}</button></div> : null}
      {loading ? <p className="loading-copy">{t("memory.loadingProjects")}</p> : null}
      {!loading && !error && active.length === 0 ? <EmptyState title={t("memory.noProjects")} description={t("memory.noProjectsHelp")} action={<button className="primary-button" type="button" onClick={() => setFormOpen(true)}>{t("memory.newProject")}</button>} /> : null}
      {!loading && active.length > 0 ? <div className="memory-project-list">{active.map((project) => <ProjectMemoryRow key={project.projectId} project={project} onOpen={() => navigate(`/projects/${project.projectId}`)} />)}</div> : null}
      {archived.length > 0 ? <details className="archived-projects"><summary>{t("memory.archivedProjects", { count: archived.length })}</summary><div className="memory-project-list">{archived.map((project) => <ProjectMemoryRow key={project.projectId} project={project} onOpen={() => navigate(`/projects/${project.projectId}`)} />)}</div></details> : null}
    </div>
  );
}

function ProjectMemoryRow({ project, onOpen }: { project: ProjectMemorySummary; onOpen: () => void }) {
  const { t } = useTranslation();
  const needsReview = project.pulseStatus === "pending_review";
  const warnings = [
    project.blockerCount > 0 ? t("memory.currentProblemsCount", { count: project.blockerCount }) : null,
    project.dirtyFiles > 0 ? t("memory.uncommittedFilesCount", { count: project.dirtyFiles }) : null,
    (project.aheadCount ?? 0) > 0 ? t("projects.aheadCount", { count: project.aheadCount }) : null,
    project.pulseFreshness === "stale" ? t("memory.staleProjectState") : null,
  ].filter(Boolean) as string[];
  return (
    <article className="memory-project-row">
      <button className="memory-project-row__main" type="button" onClick={onOpen}>
        <span className="memory-project-row__title"><strong>{project.name}</strong><span>{statusLabel(project.status)} · {lifecycleLabel(project.lifecycleStage)}</span></span>
        <span className="memory-project-row__goal"><small>{t("projects.currentGoal")}</small><strong>{project.currentGoal || t("memory.noCurrentGoal")}</strong></span>
        <span className="memory-project-row__next"><small>{t("projects.nextSteps")}</small><span>{project.nextSteps[0] || t("memory.noNextStep")}</span></span>
        <span className="memory-project-row__meta"><small>{t("memory.recentActivity")}</small><span>{project.recentActivityAt ? formatRelativeTime(project.recentActivityAt) : t("common.none")}</span></span>
      </button>
      <div className="memory-project-row__attention">
        {needsReview ? <span className="status-badge warning">{t("projects.pendingReview")}</span> : null}
        {project.pendingAchievementCount > 0 ? <span className="status-badge warning">{t("memory.draftAchievements")} {project.pendingAchievementCount}</span> : null}
        {warnings.map((value) => <span key={value}>{value}</span>)}
      </div>
    </article>
  );
}
