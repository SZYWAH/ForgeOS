import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { getErrorMessage } from "../api/project-kernel";
import { snapshotKernelApi } from "../api/snapshot-kernel";
import type { ProjectSnapshot } from "../types/snapshot";
import { formatUpdatedAt } from "../utils/format";
import {
  BoundedList,
  CollapsibleText,
  ExclusiveExpansionGroup,
} from "./content/ContentGovernance";

type CurrentSnapshotCardProps = {
  projectId: string;
  projectName: string;
  archived: boolean;
  variant?: "default" | "reference";
};

export function CurrentSnapshotCard({
  projectId,
  projectName,
  archived,
  variant = "default",
}: CurrentSnapshotCardProps) {
  const { t } = useTranslation();
  const [snapshot, setSnapshot] = useState<ProjectSnapshot | null>();
  const [error, setError] = useState<string>();
  const loadGeneration = useRef(0);

  const loadSnapshot = useCallback(async () => {
    const generation = ++loadGeneration.current;
    // Route params can change without remounting this card. Clear the previous
    // project value first so one project's context is never shown for another.
    setSnapshot(undefined);
    setError(undefined);
    try {
      const value = await snapshotKernelApi.getCurrentSnapshot(projectId);
      if (generation === loadGeneration.current) {
        setSnapshot(value);
      }
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    }
  }, [projectId]);

  useEffect(() => {
    void loadSnapshot();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadSnapshot]);

  if (variant === "reference") {
    return (
      <section className="workbench-baseline" aria-labelledby="confirmed-baseline-title">
        <header className="workbench-baseline__header">
          <div>
            <span>{t("workbench.confirmedFacts")}</span>
            <h2 id="confirmed-baseline-title">{t("workbench.confirmedBaseline")}</h2>
          </div>
          {snapshot !== undefined ? (
            <Link
              aria-disabled={!snapshot && archived}
              className={!snapshot && archived ? "disabled-link" : undefined}
              onClick={(event) => {
                if (!snapshot && archived) event.preventDefault();
              }}
              to={`/projects/${projectId}/snapshot`}
            >
              {snapshot ? t("workbench.viewFullState") : t("workbench.recordState")}
            </Link>
          ) : null}
        </header>

        {error ? (
          <div className="workbench-baseline__message">
            <span>{t("workbench.stateLoadFailed")}：{error}</span>
            <button className="text-button" onClick={() => void loadSnapshot()} type="button">{t("common.retry")}</button>
          </div>
        ) : snapshot === undefined ? (
          <div className="workbench-baseline__skeleton" aria-label={t("workbench.loadingState")}><span /><span /></div>
        ) : snapshot === null ? (
          <div className="workbench-baseline__empty">
            <strong>{t("workbench.noState")}</strong>
            <p>{archived ? t("workbench.archivedNoState") : t("workbench.stateHint")}</p>
          </div>
        ) : (
          <ExclusiveExpansionGroup>
            <div className="workbench-baseline__body">
              <div className="workbench-baseline__meta">
                <span>{t("workbench.lastConfirmed", { time: formatUpdatedAt(snapshot.confirmedAt) })}</span>
                <span>{t("snapshot.stateVersion")} v{snapshot.version}</span>
              </div>
              <div className="workbench-baseline__lead">
                <article>
                  <h3>{t("workbench.currentGoal")}</h3>
                  <CollapsibleText collapsedLines={2} expandedMaxHeight={320} previewMode="fact" value={snapshot.currentGoal} />
                </article>
                <article>
                  <h3>{t("workbench.currentSituation")}</h3>
                  <CollapsibleText collapsedLines={3} expandedMaxHeight={320} value={snapshot.statusSummary} />
                </article>
              </div>
              <details className="workbench-baseline__details">
                <summary>{t("workbench.expandFacts")}</summary>
                <div className="workbench-baseline__facts">
                  <section><h3>{t("workbench.achievements")}</h3><BoundedList items={snapshot.achievements} emptyLabel={t("workbench.noAchievements")} /></section>
                  <section><h3>{t("workbench.blockers")}</h3><BoundedList items={snapshot.blockers} emptyLabel={t("workbench.noBlockers")} /></section>
                  <section><h3>{t("workbench.nextSteps")}</h3><BoundedList items={snapshot.nextSteps} ordered emptyLabel={t("workbench.noNextSteps")} /></section>
                </div>
              </details>
            </div>
          </ExclusiveExpansionGroup>
        )}
      </section>
    );
  }

  return (
    <section className="panel snapshot-card" aria-labelledby="current-snapshot-title">
      <div className="panel-header snapshot-card-header">
        <div>
          <div className="context-card-kicker-row">
            <p className="snapshot-kicker">{t("snapshot.title")}</p>
            <span className="context-source-label context-source-label--confirmed">{t("workbench.confirmed")}</span>
          </div>
          <h2 id="current-snapshot-title">{projectName} · {t("snapshot.title")}</h2>
          <p>{t("snapshot.description")}</p>
        </div>
        {snapshot !== undefined ? (
          <Link
            className={snapshot || !archived ? "secondary-button button-link" : "secondary-button button-link disabled-link"}
            aria-disabled={!snapshot && archived}
            onClick={(event) => {
              if (!snapshot && archived) {
                event.preventDefault();
              }
            }}
            to={`/projects/${projectId}/snapshot`}
          >
            {snapshot ? t("workbench.viewFullState") : t("snapshot.create")}
          </Link>
        ) : null}
      </div>

      {error ? (
        <div className="load-error-state">
          <div className="error-banner">{error}</div>
          <button className="secondary-button" onClick={() => void loadSnapshot()} type="button">
            {t("common.retry")}
          </button>
        </div>
      ) : snapshot === undefined ? (
        <p className="loading-copy">{t("workbench.loadingState")}…</p>
      ) : snapshot === null ? (
        <div className="snapshot-empty-state">
          <strong>{t("workbench.noState")}</strong>
          <p>
            {archived
              ? t("workbench.archivedNoState")
              : t("snapshot.emptyDescription")}
          </p>
          {!archived ? (
            <Link className="primary-button button-link" to={`/projects/${projectId}/snapshot`}>
              {t("snapshot.create")}
            </Link>
          ) : null}
        </div>
      ) : (
        <ExclusiveExpansionGroup>
          <div className="snapshot-card-body">
            <div className="snapshot-primary-context">
              <article>
                <span>{t("workbench.currentGoal")}</span>
                <CollapsibleText collapsedLines={2} expandedMaxHeight={280} previewMode="fact" value={snapshot.currentGoal} />
              </article>
              <article>
                <span>{t("workbench.currentSituation")}</span>
                <CollapsibleText collapsedLines={3} expandedMaxHeight={280} value={snapshot.statusSummary} />
              </article>
            </div>

            <div className="snapshot-context-grid">
              <article>
                <h3>{t("workbench.achievements")}</h3>
                <BoundedList exclusiveExpansion expandedMaxHeight={280} items={snapshot.achievements} emptyLabel={t("workbench.noAchievements")} />
              </article>
              <article>
                <h3>{t("workbench.blockers")}</h3>
                <BoundedList exclusiveExpansion expandedMaxHeight={280} items={snapshot.blockers} emptyLabel={t("workbench.noBlockers")} />
              </article>
              <article>
                <h3>{t("workbench.nextSteps")}</h3>
                <BoundedList
                  exclusiveExpansion
                  expandedMaxHeight={280}
                  items={snapshot.nextSteps}
                  ordered
                  emptyLabel={t("workbench.noNextSteps")}
                />
              </article>
            </div>

            <footer className="snapshot-meta">
              <span>{t("workbench.lastConfirmed", { time: formatUpdatedAt(snapshot.confirmedAt) })}</span>
              <span>{t("snapshot.stateVersion")} v{snapshot.version}</span>
            </footer>
          </div>
        </ExclusiveExpansionGroup>
      )}
    </section>
  );
}
