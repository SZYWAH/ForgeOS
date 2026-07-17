import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { CurrentSessionCard } from "../components/CurrentSessionCard";
import { CurrentSnapshotCard } from "../components/CurrentSnapshotCard";
import { ProjectPulseCard } from "../components/ProjectPulseCard";
import { ProjectRuntimeStatusCard } from "../components/ProjectRuntimeStatusCard";
import { ProjectMemoryNav } from "../components/ProjectMemoryNav";
import { CollapsibleText } from "../components/content/ContentGovernance";
import { useProjectOverviewWorkbench } from "../hooks/useProjectOverviewWorkbench";
import { achievementApi } from "../api/achievements";
import { projectMemoryApi } from "../api/project-memory";
import type { Project } from "../types/project";
import type { ProjectAchievement } from "../types/achievement";
import type { DevelopmentHistoryItem } from "../types/project-memory";
import type { ProjectAiContext } from "../types/project-memory";
import { displayLabel, formatUpdatedAt, lifecycleLabel, statusLabel } from "../utils/format";
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
  const [history, setHistory] = useState<DevelopmentHistoryItem[]>([]);
  const [achievements, setAchievements] = useState<ProjectAchievement[]>([]);
  const [safeIdentity, setSafeIdentity] = useState<ProjectAiContext["identity"]>();
  const [baselineSuperseded, setBaselineSuperseded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      projectMemoryApi.overview(project.id),
      achievementApi.list(project.id),
      projectMemoryApi.buildAiContext(project.id),
    ]).then(([memory, achievementValues, aiContext]) => {
      if (!cancelled) {
        setHistory(memory.recentHistory);
        setAchievements(achievementValues.filter((value) => value.status === "confirmed").slice(0, 3));
        setSafeIdentity(aiContext.identity);
        setBaselineSuperseded(aiContext.confirmedState?.supersededByNewerActivity ?? false);
      }
    }).catch(() => undefined);
    return () => { cancelled = true; };
  }, [project.id]);

  const identityFields = [
    safeIdentity?.vision ?? project.vision,
    safeIdentity?.targetUser ?? project.targetUser,
    safeIdentity?.scopeSummary ?? project.scope,
    safeIdentity?.techStack ?? project.techStack,
  ];
  const missingIdentityCount = identityFields.filter((value) => !value.trim()).length;

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

      <ProjectMemoryNav projectId={project.id} />

      <section className="project-identity-brief" aria-labelledby="project-identity-heading">
        <header>
          <div><h2 id="project-identity-heading">{t("memory.projectIdentity")}</h2><p>{t("memory.projectIdentityDescription")}</p></div>
        </header>
        <div className="project-identity-brief__grid">
          <article><span>{t("memory.vision")}</span><CollapsibleText collapsedLines={2} value={identityFields[0] || safeIdentity?.description || project.description || t("common.notProvided")} /></article>
          <article><span>{t("memory.targetUser")}</span><CollapsibleText collapsedLines={2} value={identityFields[1] || t("common.notProvided")} /></article>
          <article><span>{t("memory.scope")}</span><CollapsibleText collapsedLines={2} value={identityFields[2] || t("common.notProvided")} /></article>
          <article><span>{t("memory.techStack")}</span><CollapsibleText collapsedLines={2} value={identityFields[3] || t("common.notProvided")} /></article>
        </div>
        {missingIdentityCount > 0 ? <p className="project-identity-brief__missing">{t("contextTrust.identityMissing", { count: missingIdentityCount })} <Link to={`/projects/${project.id}/settings`}>{t("contextTrust.completeIdentity")}</Link></p> : null}
      </section>

      <ProjectPulseCard
        activityState={activity}
        archived={archived}
        onSync={() => void activity.startSync("manual")}
        projectId={project.id}
        variant="document"
      />

      <CurrentSnapshotCard archived={archived} projectId={project.id} projectName={project.name} superseded={baselineSuperseded} variant="reference" />

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

      <section className="memory-overview-section">
        <header><div><h2>{t("memory.recentHistory")}</h2><p>{t("memory.historyDescription")}</p></div><Link to={`/projects/${project.id}/sessions`}>{t("memory.viewAllHistory")}</Link></header>
        {history.length === 0 ? <p className="memory-empty-copy">{t("memory.noHistory")}</p> : <div className="memory-overview-list">{history.map((item) => <article key={item.id}><div><span className={`memory-trust memory-trust--${item.trustStatus}`}>{t(`memory.${item.trustStatus}`)}</span><time>{formatUpdatedAt(item.startedAt)}</time></div><strong>{item.title}</strong><p>{item.outcome || item.activity || item.goal}</p><small>{t("memory.turnCount", { count: item.codexTurnCount })} · {t("memory.commitCount", { count: item.gitCommits.length })}</small></article>)}</div>}
      </section>

      <section className="memory-overview-section">
        <header><div><h2>{t("memory.recentAchievements")}</h2><p>{t("memory.achievementsDescription")}</p></div><Link to={`/projects/${project.id}/evidence`}>{t("memory.viewAllAchievements")}</Link></header>
        {achievements.length === 0 ? <p className="memory-empty-copy">{t("memory.noAchievements")}</p> : <div className="memory-overview-list">{achievements.map((item) => <article key={item.id}><div><span>{displayLabel(item.category)}</span>{item.featured ? <span>{t("memory.featured")}</span> : null}</div><strong>{item.title}</strong><p>{item.result}</p></article>)}</div>}
      </section>
    </div>
  );
}

function syncStageLabel(value: string, t: (key: string) => string) {
  return ({ connecting: t("projects.syncing"), reading_codex: t("pulse.readingCodex"), reading_git: t("pulse.readingGit"), analyzing: t("pulse.analyzing"), persisting: t("pulse.persisting") } as Record<string, string>)[value] ?? t("projects.syncing");
}
