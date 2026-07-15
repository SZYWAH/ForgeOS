import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { getErrorMessage } from "../api/project-kernel";
import { sessionKernelApi } from "../api/session-kernel";
import type { DevelopmentSession } from "../types/session";
import { formatUpdatedAt } from "../utils/format";
import { CollapsibleText } from "./content/ContentGovernance";

type CurrentSessionCardProps = {
  projectId: string;
  projectName: string;
  archived: boolean;
  variant?: "default" | "compact" | "inline";
};

export function CurrentSessionCard({
  projectId,
  archived,
  variant = "default",
}: CurrentSessionCardProps) {
  const { t } = useTranslation();
  const [session, setSession] = useState<DevelopmentSession | null>();
  const [error, setError] = useState<string>();
  const loadGeneration = useRef(0);

  const loadSession = useCallback(async () => {
    const generation = ++loadGeneration.current;
    setSession(undefined);
    setError(undefined);
    try {
      const value = await sessionKernelApi.getCurrentSession(projectId);
      if (generation === loadGeneration.current) {
        setSession(value);
      }
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    }
  }, [projectId]);

  useEffect(() => {
    void loadSession();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadSession]);

  if (variant === "inline") {
    return (
      <section className="workbench-signal workbench-signal--session" aria-labelledby="workbench-session-title">
        <div className="workbench-signal__heading">
          <h2 id="workbench-session-title">{t("workbench.session")}</h2>
          <Link to={`/projects/${projectId}/sessions`}>{session ? t("workbench.viewSession") : archived ? t("workbench.viewHistory") : t("workbench.startSession")}</Link>
        </div>
        {error ? (
          <div className="workbench-signal__error">{t("workbench.sessionLoadFailed")}：{error}</div>
        ) : session === undefined ? (
          <div className="workbench-signal__skeleton" aria-label={t("workbench.loadingSession")} />
        ) : session === null ? (
          <>
            <strong className="workbench-signal__value">{t("workbench.noOpenSession")}</strong>
            <span className="workbench-signal__meta">{archived ? t("workbench.projectArchived") : t("workbench.sessionHint")}</span>
          </>
        ) : (
          <>
            <CollapsibleText className="workbench-signal__value" collapsedLines={2} expandedMaxHeight={220} previewMode="fact" value={session.goal} />
            <span className="workbench-signal__meta">{t("workbench.inProgressSince", { time: formatUpdatedAt(session.startedAt) })}</span>
          </>
        )}
      </section>
    );
  }

  return (
    <section
      className={`panel session-card session-card--${variant}`}
      aria-labelledby="current-session-title"
    >
      <div className="panel-header session-card-header">
        <div>
          <div className="context-card-kicker-row">
            <p className="session-kicker">{t("workbench.session")}</p>
            <span className="context-source-label context-source-label--confirmed">{t("workbench.confirmed")}</span>
          </div>
          <h2 id="current-session-title">{t("workbench.currentActivity")}</h2>
          <p>{t("session.description")}</p>
        </div>
        {session !== undefined ? (
          <Link
            aria-disabled={archived && !session}
            className={
              archived && !session
                ? "secondary-button button-link disabled-link"
                : "secondary-button button-link"
            }
            onClick={(event) => {
              if (archived && !session) {
                event.preventDefault();
              }
            }}
            to={`/projects/${projectId}/sessions`}
          >
            {session ? t("workbench.viewSession") : t("workbench.startSession")}
          </Link>
        ) : null}
      </div>

      {error ? (
        <div className="load-error-state">
          <div className="error-banner">{error}</div>
          <button
            className="secondary-button"
            onClick={() => void loadSession()}
            type="button"
          >
            {t("common.retry")}
          </button>
        </div>
      ) : session === undefined ? (
        <p className="loading-copy">{t("workbench.loadingSession")}…</p>
      ) : session === null ? (
        <div className="session-empty-state">
          <div>
            <strong>{t("workbench.noOpenSession")}</strong>
            <p>
              {archived
                ? t("workbench.projectArchived")
                : t("session.emptyDescription")}
            </p>
          </div>
          {!archived ? (
            <Link
              className="primary-button button-link"
              to={`/projects/${projectId}/sessions`}
            >
              {t("session.start")}
            </Link>
          ) : (
            <Link
              className="secondary-button button-link"
              to={`/projects/${projectId}/sessions`}
            >
              {t("workbench.viewHistory")}
            </Link>
          )}
        </div>
      ) : (
        <div className="session-card-body">
          <div className="session-card-goal">
            <span>{t("session.goal")}</span>
            <CollapsibleText collapsedLines={2} expandedMaxHeight={variant === "compact" ? 220 : 480} previewMode="fact" value={session.goal} />
          </div>
          <dl className="session-card-meta">
            <div>
              <dt>{t("common.createdAt")}</dt>
              <dd>{formatUpdatedAt(session.startedAt)}</dd>
            </div>
            <div>
              <dt>{t("common.status")}</dt>
              <dd>
                <span className="session-status session-status--open">{t("session.open")}</span>
              </dd>
            </div>
            <div>
              <dt>{t("common.version")}</dt>
              <dd>v{session.version}</dd>
            </div>
          </dl>
        </div>
      )}
    </section>
  );
}
