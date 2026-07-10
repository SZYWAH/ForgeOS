import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { getErrorMessage } from "../api/project-kernel";
import { snapshotKernelApi } from "../api/snapshot-kernel";
import type { ProjectSnapshot } from "../types/snapshot";
import { formatUpdatedAt } from "../utils/format";

type CurrentSnapshotCardProps = {
  projectId: string;
  projectName: string;
  archived: boolean;
};

function SnapshotList({
  items,
  ordered = false,
  emptyLabel,
}: {
  items: string[];
  ordered?: boolean;
  emptyLabel: string;
}) {
  if (items.length === 0) {
    return <p className="snapshot-empty-value">{emptyLabel}</p>;
  }

  const List = ordered ? "ol" : "ul";
  return (
    <List className="snapshot-list">
      {items.map((item, index) => (
        <li key={`${index}-${item}`}>{item}</li>
      ))}
    </List>
  );
}

export function CurrentSnapshotCard({
  projectId,
  projectName,
  archived,
}: CurrentSnapshotCardProps) {
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

  return (
    <section className="panel snapshot-card" aria-labelledby="current-snapshot-title">
      <div className="panel-header snapshot-card-header">
        <div>
          <p className="snapshot-kicker">Current Snapshot</p>
          <h2 id="current-snapshot-title">{projectName} 当前状态</h2>
          <p>经人工确认的项目状态，用于快速恢复研发上下文。</p>
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
            {snapshot ? (archived ? "View History" : "Edit & History") : "Create Snapshot"}
          </Link>
        ) : null}
      </div>

      {error ? (
        <div className="load-error-state">
          <div className="error-banner">{error}</div>
          <button className="secondary-button" onClick={() => void loadSnapshot()} type="button">
            Retry
          </button>
        </div>
      ) : snapshot === undefined ? (
        <p className="loading-copy">Loading current snapshot…</p>
      ) : snapshot === null ? (
        <div className="snapshot-empty-state">
          <strong>尚未确认当前状态</strong>
          <p>
            {archived
              ? "该归档项目没有 Snapshot，当前状态不可再创建。"
              : "创建第一份人工 Snapshot，记录当前目标、成果、阻塞和下一步。"}
          </p>
          {!archived ? (
            <Link className="primary-button button-link" to={`/projects/${projectId}/snapshot`}>
              Create Snapshot
            </Link>
          ) : null}
        </div>
      ) : (
        <div className="snapshot-card-body">
          <div className="snapshot-primary-context">
            <article>
              <span>当前目标</span>
              <p>{snapshot.currentGoal}</p>
            </article>
            <article>
              <span>状态总结</span>
              <p>{snapshot.statusSummary}</p>
            </article>
          </div>

          <div className="snapshot-context-grid">
            <article>
              <h3>最近成果</h3>
              <SnapshotList items={snapshot.achievements} emptyLabel="暂无已确认成果" />
            </article>
            <article>
              <h3>当前阻塞</h3>
              <SnapshotList items={snapshot.blockers} emptyLabel="当前无已知阻塞" />
            </article>
            <article>
              <h3>下一步方向</h3>
              <SnapshotList
                items={snapshot.nextSteps}
                ordered
                emptyLabel="暂无已确认下一步"
              />
            </article>
          </div>

          <footer className="snapshot-meta">
            <span>最后确认 {formatUpdatedAt(snapshot.confirmedAt)}</span>
            <span>Snapshot v{snapshot.version}</span>
          </footer>
        </div>
      )}
    </section>
  );
}
