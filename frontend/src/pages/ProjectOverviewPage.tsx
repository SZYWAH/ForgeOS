import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { CurrentSessionCard } from "../components/CurrentSessionCard";
import { CurrentSnapshotCard } from "../components/CurrentSnapshotCard";
import { ProjectPulseCard } from "../components/ProjectPulseCard";
import { ProjectRuntimeStatusCard } from "../components/ProjectRuntimeStatusCard";
import { useProjectOverviewWorkbench } from "../hooks/useProjectOverviewWorkbench";
import type { Project } from "../types/project";
import { formatUpdatedAt, lifecycleLabel, statusLabel } from "../utils/format";
import { useTranslation } from "react-i18next";

export function ProjectOverviewPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    let cancelled = false;
    setProject(undefined);
    setError(undefined);
    if (!projectId) {
      setError(t("projects.noProject"));
      return;
    }
    void projectKernelApi.getProject(projectId).then((value) => {
      if (!cancelled) setProject(value);
    }).catch((value) => {
      if (!cancelled) setError(getErrorMessage(value));
    });
    return () => { cancelled = true; };
  }, [projectId, t]);

  if (!project) {
    return <div className="page-stack"><p className="loading-copy">{t("common.loading")}…</p>{error ? <div className="error-banner">{error}</div> : null}</div>;
  }

  return <ProjectOverviewWorkbench project={project} />;
}

function ProjectOverviewWorkbench({ project }: { project: Project }) {
  const { t } = useTranslation();
  const archived = project.status === "archived";
  const activity = useProjectOverviewWorkbench({ projectId: project.id, archived });
  const pendingReview = activity.pulse?.status === "pending_review";

  return (
    <div className="page-stack project-overview-document">
      <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><span>{project.name}</span></div>

      <header className="project-masthead">
        <div className="project-masthead__identity">
          <div className="title-with-status">
            <h1>{project.name}</h1>
            <span className={`status status--${project.status}`}>{statusLabel(project.status)}</span>
          </div>
          <div className="project-masthead__meta">
            <span>{lifecycleLabel(project.lifecycleStage)}</span>
            <span>{t("common.updatedAt")} {formatUpdatedAt(project.updatedAt)}</span>
          </div>
        </div>
        <div className="project-masthead__actions">
          {pendingReview ? (
            <Link className="primary-button button-link" to={`/projects/${project.id}/pulse`}>{t("projects.reviewState")}</Link>
          ) : (
            <button className="primary-button" disabled={activity.loading || activity.running || archived} onClick={() => void activity.startSync("manual")} type="button">
              {activity.running || activity.loading ? t("projects.syncing") : t("projects.sync")}
            </button>
          )}
          <Link className="secondary-button button-link" to={`/projects/${project.id}/settings`}>{t("navigation.projectSettings")}</Link>
          {activity.running && activity.run ? <span className="project-masthead__progress">{syncStageLabel(activity.run.stage, t)}</span> : null}
        </div>
      </header>

      <ProjectNavigation projectId={project.id} />

      <ProjectPulseCard
        activityState={activity}
        archived={archived}
        onSync={() => void activity.startSync("manual")}
        projectId={project.id}
        variant="document"
      />

      <section className="project-engineering" aria-labelledby="project-engineering-title">
        <header>
          <h2 id="project-engineering-title">{t("repository.workingCopy")}</h2>
          <p>{t("repository.description")}</p>
        </header>
        <div className="project-engineering-strip">
          <CurrentSessionCard archived={archived} projectId={project.id} projectName={project.name} variant="inline" />
          <ProjectRuntimeStatusCard activityState={activity} archived={archived} projectId={project.id} variant="strip" />
        </div>
      </section>

      <CurrentSnapshotCard archived={archived} projectId={project.id} projectName={project.name} variant="reference" />
    </div>
  );
}

function syncStageLabel(value: string, t: (key: string) => string) {
  return ({ connecting: t("projects.syncing"), reading_codex: t("pulse.readingCodex"), reading_git: t("pulse.readingGit"), analyzing: t("pulse.analyzing"), persisting: t("pulse.persisting") } as Record<string, string>)[value] ?? t("projects.syncing");
}

function ProjectNavigation({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const projectNav = [
    ["snapshot", t("navigation.currentState")],
    ["sessions", t("navigation.developmentLog")],
    ["evidence", t("navigation.projectMaterials")],
    ["repositories", t("navigation.codeRepository")],
    ["context", t("navigation.aiCollaboration")],
  ] as const;
  const moreNav = [
    ["/github-import", t("navigation.importGithub")],
    ["candidates", t("navigation.aiSuggestions")],
    ["codex-tasks", t("navigation.codexTasks")],
    ["codex-executions", t("navigation.executionRecords")],
  ] as const;
  return (
    <nav className="project-tabs" aria-label={t("navigation.workspace")}>
      <div className="project-tabs__scroll">
        <div className="project-tabs__track">
          <Link aria-current="page" className="active" to={`/projects/${projectId}`}>{t("navigation.overview")}</Link>
          {projectNav.map(([path, label]) => <Link key={path} to={`/projects/${projectId}/${path}`}>{label}</Link>)}
        </div>
      </div>
      <details className="project-tabs__more">
        <summary>{t("navigation.more")}</summary>
        <div className="project-tabs__menu">
          {moreNav.map(([path, label]) => <Link key={path} to={path.startsWith("/") ? path : `/projects/${projectId}/${path}`}>{label}</Link>)}
        </div>
      </details>
    </nav>
  );
}
