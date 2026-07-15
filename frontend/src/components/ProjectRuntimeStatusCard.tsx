import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  type ProjectActivityOverview,
  useProjectOverviewWorkbench,
} from "../hooks/useProjectOverviewWorkbench";
import type { SyncStatus } from "../types/project-pulse";
import type { RepositoryRecord } from "../types/repository";
import { formatUpdatedAt } from "../utils/format";

type ProjectRuntimeStatusCardProps = {
  projectId: string;
  archived: boolean;
  variant?: "card" | "strip";
  activityState?: ProjectActivityOverview;
};

export type RepositoryRuntimeSummary = {
  repositoryCount: number;
  branch: string;
  changedFiles: number;
  isDirty: boolean;
  ahead: string;
  behind: string;
  checkedAt: string;
};

function aggregateDirection(records: RepositoryRecord[], read: (record: RepositoryRecord) => number | null) {
  const values = records.map(read);
  if (values.some((value) => value === null)) {
    return records.length > 1 ? "部分未知" : "未知";
  }
  return String(values.reduce<number>((total, value) => total + (value ?? 0), 0));
}

export function summarizeRepositoryRuntime(records: RepositoryRecord[]): RepositoryRuntimeSummary | null {
  if (!records.length) return null;
  const primary = records[0].workingCopy;
  const checkedAt = records.reduce(
    (latest, record) => record.workingCopy.checkedAt > latest ? record.workingCopy.checkedAt : latest,
    primary.checkedAt,
  );
  return {
    repositoryCount: records.length,
    branch: primary.branch,
    changedFiles: records.reduce((total, record) => total + record.workingCopy.diffSummary.changedFiles, 0),
    isDirty: records.some((record) => record.workingCopy.isDirty),
    ahead: aggregateDirection(records, (record) => record.workingCopy.aheadCount),
    behind: aggregateDirection(records, (record) => record.workingCopy.behindCount),
    checkedAt,
  };
}

export function ProjectRuntimeStatusCard({
  projectId,
  archived,
  variant = "card",
  activityState,
}: ProjectRuntimeStatusCardProps) {
  const { t } = useTranslation();
  const localActivity = useProjectOverviewWorkbench({
    projectId,
    archived,
    enabled: activityState === undefined,
    autoSync: false,
  });
  const activity = activityState ?? localActivity;
  const summary = activity.repositories ? summarizeRepositoryRuntime(activity.repositories) : undefined;
  const run = activity.run;

  if (variant === "strip") {
    return (
      <div className="workbench-runtime-strip" role="group" aria-label={`${t("repository.title")} · ${t("workbench.lastSync")}`}>
        <section className="workbench-signal">
          <div className="workbench-signal__heading"><h2>{t("workbench.branch")}</h2><Link to={`/projects/${projectId}/repositories`}>{t("workbench.openRepository")}</Link></div>
          {activity.repositoryError ? <div className="workbench-signal__error">{t("workbench.repositoryReadFailed")}：{activity.repositoryError}</div> : summary === undefined ? <div className="workbench-signal__skeleton" /> : summary ? <><strong className="workbench-signal__value workbench-signal__mono" title={summary.branch}>{summary.branch}</strong><span className="workbench-signal__meta">{summary.repositoryCount > 1 ? t("workbench.primaryRepository", { count: summary.repositoryCount - 1 }) : t("workbench.checkedAt", { time: formatUpdatedAt(summary.checkedAt) })}</span></> : <><strong className="workbench-signal__value">{t("workbench.notConnected")}</strong><Link className="workbench-signal__link" to={`/projects/${projectId}/repositories`}>{archived ? t("workbench.viewRepository") : t("workbench.connectRepository")}</Link></>}
        </section>

        <section className="workbench-signal">
          <div className="workbench-signal__heading"><h2>{t("workbench.codeChanges")}</h2></div>
          {activity.repositoryError ? <span className="workbench-signal__meta">{t("workbench.unavailable")}</span> : summary === undefined ? <div className="workbench-signal__skeleton" /> : summary ? <><strong className={`workbench-signal__value ${summary.isDirty ? "workbench-signal__value--warning" : "workbench-signal__value--success"}`}>{summary.isDirty ? t("workbench.uncommittedFiles", { count: summary.changedFiles }) : t("workbench.clean")}</strong><span className="workbench-signal__meta">{t("workbench.readOnlyGit")}</span></> : <span className="workbench-signal__meta">{t("workbench.waitingRepository")}</span>}
        </section>

        <section className="workbench-signal">
          <div className="workbench-signal__heading"><h2>{t("workbench.remote")}</h2></div>
          {summary === undefined ? <div className="workbench-signal__skeleton" /> : summary ? <><strong className="workbench-signal__value workbench-signal__mono">{t("workbench.notPushed", { ahead: summary.ahead, behind: summary.behind })}</strong><span className="workbench-signal__meta">{t("workbench.localRemoteReference")}</span></> : <span className="workbench-signal__meta">{t("workbench.remoteUnknown")}</span>}
        </section>

        <section className="workbench-signal">
          <div className="workbench-signal__heading"><h2>{t("workbench.lastSync")}</h2><Link to={`/projects/${projectId}/pulse`}>{t("workbench.viewSyncSources")}</Link></div>
          {activity.syncError ? <div className="workbench-signal__error">{t("workbench.syncReadFailed")}：{activity.syncError}</div> : run === undefined ? <div className="workbench-signal__skeleton" /> : run ? <><strong className={`workbench-signal__value workbench-signal__value--${run.status}`}>{syncStatusLabel(run.status, t)}</strong><span className="workbench-signal__meta">{run.status === "queued" || run.status === "running" ? syncStageLabel(run.stage, t) : `${formatUpdatedAt(run.completedAt ?? run.startedAt)} · ${run.aiInvoked ? t("pulse.aiAssisted") : t("pulse.localRules")}`}</span></> : <><strong className="workbench-signal__value">{t("workbench.noSync")}</strong><span className="workbench-signal__meta">{t("workbench.noSyncHint")}</span></>}
        </section>
      </div>
    );
  }

  const loading = activity.repositories === undefined || run === undefined;
  const running = run?.status === "queued" || run?.status === "running";
  return (
    <section className="panel runtime-status-card" aria-busy={loading || running} aria-labelledby="runtime-status-title">
      <div className="panel-header runtime-status-card__header">
        <div><div className="context-card-kicker-row"><p className="runtime-kicker">{t("repository.title")}</p><span className="context-source-label context-source-label--observed">{t("workbench.readOnlyGit")}</span></div><h2 id="runtime-status-title">{t("repository.title")} · {t("workbench.lastSync")}</h2><p>{t("repository.description")}</p></div>
        <div className="runtime-status-card__actions"><Link className="secondary-button button-link" to={`/projects/${projectId}/repositories`}>{summary ? t("workbench.openRepository") : archived ? t("workbench.viewRepository") : t("workbench.connectRepository")}</Link><Link className="text-button" to={`/projects/${projectId}/pulse`}>{t("workbench.viewSyncSources")}</Link></div>
      </div>
      {loading ? <div className="runtime-status-skeleton" aria-label={t("common.loading")}><span /><span /></div> : <div className="runtime-status-card__body">
        <section className="runtime-signal-section"><div className="runtime-signal-heading"><h3>{t("repository.workingCopy")}</h3>{summary ? <span className={summary.isDirty ? "git-dirty" : "git-clean"}>{summary.isDirty ? t("repository.dirty") : t("repository.clean")}</span> : null}</div>{activity.repositoryError ? <div className="runtime-inline-error">{t("workbench.repositoryReadFailed")}：{activity.repositoryError}</div> : summary ? <><div className="runtime-branch-row"><strong title={summary.branch}>{summary.branch}</strong>{summary.repositoryCount > 1 ? <span>{t("workbench.primaryRepository", { count: summary.repositoryCount - 1 })}</span> : null}</div><dl className="runtime-metrics"><div><dt>{t("workbench.codeChanges")}</dt><dd>{summary.changedFiles}</dd></div><div><dt>{t("repository.ahead")}</dt><dd>{summary.ahead}</dd></div><div><dt>{t("repository.behind")}</dt><dd>{summary.behind}</dd></div></dl><p className="runtime-timestamp">{t("workbench.checkedAt", { time: formatUpdatedAt(summary.checkedAt) })}</p></> : <p className="runtime-empty">{t("repository.emptyTitle")}</p>}</section>
        <section className="runtime-signal-section"><div className="runtime-signal-heading"><h3>{t("workbench.lastSync")}</h3>{run ? <span className={`runtime-state runtime-state--${run.status}`}>{syncStatusLabel(run.status, t)}</span> : null}</div>{activity.syncError ? <div className="runtime-inline-error">{t("workbench.syncReadFailed")}：{activity.syncError}</div> : run ? <><p className="runtime-current-stage">{running ? syncStageLabel(run.stage, t) : run.aiInvoked ? t("pulse.aiAssisted") : t("pulse.localRules")}</p><dl className="runtime-metrics runtime-metrics--sync"><div><dt>{t("pulse.sources")}</dt><dd>{run.threadsScanned}</dd></div><div><dt>{t("pulse.latestObservation")}</dt><dd>{run.observationsCreated}</dd></div><div><dt>{t("common.details")}</dt><dd>{run.aiInvoked ? t("pulse.aiAssisted") : t("pulse.localRules")}</dd></div></dl>{run.errorSummary ? <p className="runtime-inline-error">{run.errorSummary}</p> : null}</> : <p className="runtime-empty">{t("workbench.noSync")}</p>}</section>
      </div>}
    </section>
  );
}

function syncStatusLabel(value: SyncStatus, t: (key: string) => string) {
  return ({ queued: t("projects.syncing"), running: t("projects.syncing"), succeeded: t("pulse.syncComplete"), partial: t("pulse.syncPartial"), failed: t("pulse.syncFailed") } as Record<SyncStatus, string>)[value];
}

function syncStageLabel(value: string, t: (key: string) => string) {
  return ({ connecting: t("projects.syncing"), reading_codex: t("pulse.readingCodex"), reading_git: t("pulse.readingGit"), analyzing: t("pulse.analyzing"), persisting: t("pulse.persisting") } as Record<string, string>)[value] ?? t("projects.syncing");
}
