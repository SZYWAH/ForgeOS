import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  type ProjectActivityOverview,
  useProjectOverviewWorkbench,
} from "../hooks/useProjectOverviewWorkbench";
import type { ActivitySyncRun, ProjectPulse } from "../types/project-pulse";
import { formatUpdatedAt } from "../utils/format";
import {
  BoundedList,
  CollapsibleText,
  ExclusiveExpansionGroup,
} from "./content/ContentGovernance";

type Props = {
  projectId: string;
  archived: boolean;
  variant?: "card" | "document";
  activityState?: ProjectActivityOverview;
  onSync?: () => void;
};

const STAGES = ["reading_codex", "reading_git", "analyzing", "persisting"] as const;

export function ProjectPulseCard({
  projectId,
  archived,
  variant = "card",
  activityState,
  onSync,
}: Props) {
  const { t } = useTranslation();
  const localActivity = useProjectOverviewWorkbench({
    projectId,
    archived,
    enabled: activityState === undefined,
    autoSync: activityState === undefined,
  });
  const activity = activityState ?? localActivity;
  const pulse = activity.pulse;
  const run = activity.run;
  const review = pulse?.status === "pending_review";
  const error = activity.actionError ?? activity.pulseError;

  function startManualSync() {
    if (onSync) {
      onSync();
      return;
    }
    void activity.startSync("manual");
  }

  if (variant === "document") {
    return (
      <article className="project-brief" aria-busy={activity.running || activity.loading} aria-label={t("pulse.latestObservation")}>
        <header className="project-brief__header">
          <div className="project-brief__identity">
            <span>{t("pulse.latestObservation")}</span>
            {pulse ? <strong className={`pulse-state pulse-state--${pulse.status}`}>{factStatusLabel(pulse, t)}</strong> : null}
            {pulse?.freshness === "stale" ? <strong className="pulse-state pulse-state--stale">{t("pulse.stale")}</strong> : null}
          </div>
          {pulse ? (
            <div className="project-brief__source-meta">
              <span>{t("pulse.sourceCount", { count: pulse.sourceCount })}</span>
              <span>{t("pulse.syncedAt", { time: formatUpdatedAt(pulse.lastSuccessfulSyncAt ?? pulse.generatedAt) })}</span>
              <Link to={`/projects/${projectId}/pulse`}>{t("pulse.viewSources")}</Link>
            </div>
          ) : null}
        </header>

        <SyncFeedback run={run} running={activity.running} />
        {error ? <div className="error-banner project-brief__error">{error}</div> : null}

        {pulse === undefined ? (
          <ProjectBriefSkeleton />
        ) : pulse === null ? (
          <div className="project-brief__empty">
            <h2 id="project-brief-title">{t("pulse.waitingTitle")}</h2>
            <p>{t("pulse.waitingDescription")}</p>
            {!archived ? <button className="secondary-button" onClick={startManualSync} type="button">{t("pulse.syncNow")}</button> : null}
          </div>
        ) : (
          <ExclusiveExpansionGroup>
            <div className="project-brief__body">
              <section className="project-brief__lead">
                <span>{t("pulse.currentGoal")}</span>
                <CollapsibleText className="project-brief__goal" collapsedLines={2} expandedMaxHeight={360} previewMode="fact" value={pulse.currentGoal} />
                <span>{t("pulse.currentSituation")}</span>
                <CollapsibleText className="project-brief__summary" collapsedLines={3} expandedMaxHeight={360} value={pulse.statusSummary} />
              </section>

              <section className="project-brief__section" aria-labelledby="project-brief-achievements">
                <h2 id="project-brief-achievements">{t("pulse.achievements")}</h2>
                <BoundedList emptyLabel={t("pulse.noAchievements")} items={pulse.achievements} />
              </section>
              <section className="project-brief__section" aria-labelledby="project-brief-validation">
                <h2 id="project-brief-validation">{t("pulse.validation")}</h2>
                <BoundedList emptyLabel={t("pulse.noValidation")} items={[pulse.validationSummary].filter(Boolean)} />
              </section>
              <section className="project-brief__section project-brief__section--warning" aria-labelledby="project-brief-blockers">
                <h2 id="project-brief-blockers">{t("pulse.blockers")}</h2>
                <BoundedList emptyLabel={t("pulse.noBlockers")} items={pulse.blockers} />
              </section>
              <section className="project-brief__section" aria-labelledby="project-brief-next">
                <h2 id="project-brief-next">{t("pulse.nextSteps")}</h2>
                <BoundedList emptyLabel={t("pulse.noNextSteps")} items={pulse.nextSteps} ordered />
              </section>
            </div>
          </ExclusiveExpansionGroup>
        )}

        {pulse ? (
          <footer className="project-brief__footer">
            <span>{pulse.analysisMode === "ai" ? t("pulse.aiAssisted") : t("pulse.localRules")}</span>
            <span>{activityTypeLabel(pulse.activityType, t)}</span>
            <Link to={`/projects/${projectId}/pulse#activity-settings`}>{t("pulse.settings")}</Link>
          </footer>
        ) : null}
      </article>
    );
  }

  return (
    <section className="panel pulse-card" aria-busy={activity.running || activity.loading}>
      <div className="pulse-card__header">
        <div>
          <div className="pulse-heading-line">
            <p className="snapshot-kicker">{t("pulse.latestObservation")}</p>
            {pulse ? <span className={`pulse-state pulse-state--${pulse.status}`}>{factStatusLabel(pulse, t)}</span> : null}
            {pulse?.freshness === "stale" ? <span className="pulse-state pulse-state--stale">{t("pulse.stale")}</span> : null}
          </div>
          <CollapsibleText className="pulse-card__goal" collapsedLines={2} previewMode="fact" value={pulse?.currentGoal ?? t("pulse.waitingTitle")} />
          <CollapsibleText className="pulse-card__summary" collapsedLines={3} value={pulse?.statusSummary ?? t("pulse.waitingDescription")} />
        </div>
        <div className="pulse-actions">
          {review ? <Link className="primary-button button-link" to={`/projects/${projectId}/pulse`}>{t("projects.reviewState")}</Link> : <button className="primary-button" disabled={activity.running || archived} onClick={startManualSync} type="button">{activity.running ? t("projects.syncing") : t("projects.sync")}</button>}
          {pulse ? <Link className="secondary-button button-link" to={`/projects/${projectId}/pulse`}>{t("pulse.viewSources")}</Link> : <Link className="secondary-button button-link" to={`/projects/${projectId}/repositories`}>{t("repository.connect")}</Link>}
        </div>
      </div>
      <SyncFeedback run={run} running={activity.running} />
      {error ? <div className="error-banner pulse-error">{error}</div> : null}
      {pulse ? <div className="pulse-grid">
        <PulseSection title={t("pulse.achievements")} empty={t("pulse.noAchievements")} items={pulse.achievements} />
        <PulseSection title={t("pulse.validation")} empty={t("pulse.noValidation")} items={[pulse.validationSummary].filter(Boolean)} />
        <PulseSection title={t("pulse.blockers")} empty={t("pulse.noBlockers")} items={pulse.blockers} tone="warning" />
        <PulseSection title={t("pulse.nextSteps")} empty={t("pulse.noNextSteps")} items={pulse.nextSteps} ordered />
      </div> : null}
    </section>
  );
}

function SyncFeedback({ run, running }: { run: ActivitySyncRun | null | undefined; running: boolean }) {
  const { t } = useTranslation();
  if (!run) return null;
  if (running) {
    const currentIndex = STAGES.findIndex((stage) => stage === run.stage);
    return <div className="sync-progress project-brief__sync"><div className="sync-progress__steps">{STAGES.map((stage, index) => <span className={index === currentIndex ? "active" : index < currentIndex ? "done" : ""} key={stage}>{syncStageLabel(stage, t)}</span>)}</div><p>{syncStageLabel(run.stage, t)} · {t("pulse.syncCounters", { threads: run.threadsScanned, observations: run.observationsCreated })}</p></div>;
  }
  return <div className={`sync-result sync-result--${run.status} project-brief__sync-result`}><strong>{run.status === "succeeded" ? t("pulse.syncComplete") : run.status === "partial" ? t("pulse.syncPartial") : t("pulse.syncFailed")}</strong><span>{t("pulse.syncCounters", { threads: run.threadsScanned, observations: run.observationsCreated })} · {run.aiInvoked ? t("pulse.aiAssisted") : t("pulse.localRules")}</span>{run.errorSummary ? <p>{run.errorSummary}</p> : null}</div>;
}

function ProjectBriefSkeleton() {
  const { t } = useTranslation();
  return <div className="project-brief__skeleton" aria-label={t("common.loading")}><span /><span /><span /></div>;
}

function factStatusLabel(pulse: ProjectPulse, t: (key: string) => string) {
  if (pulse.factStatus === "confirmed" || pulse.status === "confirmed") return t("pulse.confirmed");
  if (pulse.status === "pending_review") return t("pulse.pending");
  return t("pulse.historyObservation");
}

function activityTypeLabel(value: ProjectPulse["activityType"], t: (key: string) => string) {
  return ({
    implementation: t("pulse.activityImplementation"),
    debugging: t("pulse.activityDebugging"),
    release: t("pulse.activityRelease"),
    review: t("pulse.activityReview"),
    support_question: t("pulse.activitySupport"),
    unknown: t("pulse.activityUnknown"),
  } as const)[value];
}

function syncStageLabel(value: string, t: (key: string) => string) {
  return ({
    queued: t("projects.syncing"),
    connecting: t("projects.syncing"),
    reading_codex: t("pulse.readingCodex"),
    reading_git: t("pulse.readingGit"),
    analyzing: t("pulse.analyzing"),
    persisting: t("pulse.persisting"),
  } as Record<string, string>)[value] ?? t("projects.syncing");
}

function PulseSection({ title, items, empty, tone, ordered }: { title: string; items: string[]; empty: string; tone?: "warning"; ordered?: boolean }) {
  return <div className={`pulse-section ${tone ? `pulse-section--${tone}` : ""}`}><h3>{title}</h3><BoundedList emptyLabel={empty} items={items} ordered={ordered} /></div>;
}
