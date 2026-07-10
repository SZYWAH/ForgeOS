import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { getErrorMessage } from "../api/project-kernel";
import { sessionKernelApi } from "../api/session-kernel";
import type { DevelopmentSession } from "../types/session";
import { formatUpdatedAt } from "../utils/format";

type CurrentSessionCardProps = {
  projectId: string;
  projectName: string;
  archived: boolean;
};

export function CurrentSessionCard({
  projectId,
  projectName,
  archived,
}: CurrentSessionCardProps) {
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

  return (
    <section
      className="panel session-card"
      aria-labelledby="current-session-title"
    >
      <div className="panel-header session-card-header">
        <div>
          <p className="session-kicker">Development Session</p>
          <h2 id="current-session-title">Current Development Session</h2>
          <p>记录 {projectName} 当前这次研发活动为什么开始，以及如何继续。</p>
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
            {session
              ? archived
                ? "Close Session"
                : "Continue Session"
              : "Start Session"}
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
            Retry
          </button>
        </div>
      ) : session === undefined ? (
        <p className="loading-copy">Loading current session…</p>
      ) : session === null ? (
        <div className="session-empty-state">
          <div>
            <strong>当前没有进行中的开发会话</strong>
            <p>
              {archived
                ? "该项目已经归档，历史会话仍可查看。"
                : "开始新的开发会话，明确这次研发活动的目标。"}
            </p>
          </div>
          {!archived ? (
            <Link
              className="primary-button button-link"
              to={`/projects/${projectId}/sessions`}
            >
              开始新的开发会话
            </Link>
          ) : (
            <Link
              className="secondary-button button-link"
              to={`/projects/${projectId}/sessions`}
            >
              View History
            </Link>
          )}
        </div>
      ) : (
        <div className="session-card-body">
          <div className="session-card-goal">
            <span>目标</span>
            <p>{session.goal}</p>
          </div>
          <dl className="session-card-meta">
            <div>
              <dt>开始时间</dt>
              <dd>{formatUpdatedAt(session.startedAt)}</dd>
            </div>
            <div>
              <dt>状态</dt>
              <dd>
                <span className="session-status session-status--open">Open</span>
              </dd>
            </div>
            <div>
              <dt>版本</dt>
              <dd>v{session.version}</dd>
            </div>
          </dl>
        </div>
      )}
    </section>
  );
}
