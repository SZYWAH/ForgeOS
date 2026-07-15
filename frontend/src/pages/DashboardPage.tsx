import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { projectPulseApi } from "../api/project-pulse";
import { repositoryApi } from "../api/repository";
import type { ProjectSummary } from "../types/project";
import type { ProjectPulse } from "../types/project-pulse";
import type { RepositoryRecord } from "../types/repository";
import { formatUpdatedAt, statusLabel } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

type ProjectAttention = {
  project: ProjectSummary;
  pulse: ProjectPulse | null;
  repositories: RepositoryRecord[];
};

export function DashboardPage() {
  const { t } = useTranslation();
  const [items, setItems] = useState<ProjectAttention[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string>();

  useEffect(() => {
    projectKernelApi.listProjects()
      .then(async (projects) => Promise.all(projects.map(async (project) => {
        const [pulse, repositories] = await Promise.all([
          projectPulseApi.latest(project.id).catch(() => null),
          repositoryApi.list(project.id).catch(() => []),
        ]);
        return { project, pulse, repositories };
      })))
      .then(setItems)
      .catch((value) => setError(getErrorMessage(value)))
      .finally(() => setLoading(false));
  }, []);

  const ordered = useMemo(
    () => [...items].sort((left, right) => attentionScore(right) - attentionScore(left)
      || right.project.updatedAt.localeCompare(left.project.updatedAt)),
    [items],
  );
  const pending = items.filter((item) => item.pulse?.status === "pending_review").length;
  const dirty = items.filter((item) => item.repositories.some((repository) => repository.workingCopy.isDirty)).length;
  const stale = items.filter((item) => item.pulse?.freshness === "stale" || !item.pulse).length;

  return <div className="page-stack dashboard-v2">
    <PageIntro title={t("projects.attentionTitle")} description={t("projects.attentionDescription")} primaryAction={<Link className="primary-button button-link" to="/projects">{t("projects.manage")}</Link>} />
    <section className="metrics" aria-label={t("projects.attentionAria")}>
      <article><span>{t("projects.pendingReview")}</span><strong>{loading ? "…" : pending}</strong></article>
      <article><span>{t("projects.gitUncommitted")}</span><strong>{loading ? "…" : dirty}</strong></article>
      <article><span>{t("projects.staleState")}</span><strong>{loading ? "…" : stale}</strong></article>
    </section>
    <section className="panel attention-list">
      <div className="panel-header"><div><h2>{t("projects.queueTitle")}</h2><p>{t("projects.queueDescription")}</p></div></div>
      {loading ? <p className="loading-copy">{t("common.loading")}…</p> : null}
      {error ? <div className="error-banner">{error}</div> : null}
      {!loading && !ordered.length ? <p className="empty-copy">{t("projects.createProject")}</p> : <div>{ordered.map(({ project, pulse, repositories }) => {
        const changed = repositories.reduce((total, repository) => total + (repository.workingCopy.isDirty ? repository.workingCopy.diffSummary.changedFiles : 0), 0);
        const ahead = repositories.reduce((total, repository) => total + (repository.workingCopy.aheadCount ?? 0), 0);
        return <Link className="attention-row" key={project.id} to={`/projects/${project.id}`}>
          <div><span className={`status status--${project.status}`}>{statusLabel(project.status)}</span><strong>{project.name}</strong><p>{pulse?.currentGoal ?? t("projects.forgeosObservation")}</p></div>
          <div className="attention-signals">
            {pulse?.status === "pending_review" ? <span className="signal signal--review">{t("pulse.pending")}</span> : null}
            {pulse?.freshness === "stale" || !pulse ? <span className="signal signal--stale">{t("projects.stale")}</span> : null}
            {changed > 0 ? <span className="signal signal--dirty">{t("workbench.uncommittedFiles", { count: changed })}</span> : null}
            {ahead > 0 ? <span className="signal signal--ahead">{t("projects.aheadCount", { count: ahead })}</span> : null}
          </div>
          <time>{pulse ? t("projects.syncedAt", { time: formatUpdatedAt(pulse.lastSuccessfulSyncAt ?? pulse.generatedAt) }) : t("projects.projectUpdatedAt", { time: formatUpdatedAt(project.updatedAt) })}</time>
        </Link>;
      })}</div>}
    </section>
  </div>;
}

function attentionScore(item: ProjectAttention) {
  let score = 0;
  if (item.pulse?.status === "pending_review") score += 100;
  score += (item.pulse?.blockers.length ?? 0) * 10;
  if (item.repositories.some((repository) => repository.workingCopy.isDirty)) score += 30;
  if (item.repositories.some((repository) => (repository.workingCopy.aheadCount ?? 0) > 0)) score += 20;
  if (!item.pulse || item.pulse.freshness === "stale") score += 15;
  return score;
}
