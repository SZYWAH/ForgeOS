import {
  type FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Link, useParams } from "react-router-dom";
import {
  getErrorCode,
  getErrorMessage,
  projectKernelApi,
} from "../api/project-kernel";
import { snapshotKernelApi } from "../api/snapshot-kernel";
import type { Project } from "../types/project";
import type {
  ProjectSnapshot,
  SnapshotContentInput,
} from "../types/snapshot";
import { formatUpdatedAt } from "../utils/format";

type SnapshotDraft = {
  currentGoal: string;
  statusSummary: string;
  achievements: string;
  blockers: string;
  nextSteps: string;
};

const EMPTY_DRAFT: SnapshotDraft = {
  currentGoal: "",
  statusSummary: "",
  achievements: "",
  blockers: "",
  nextSteps: "",
};

function toDraft(snapshot: ProjectSnapshot): SnapshotDraft {
  return {
    currentGoal: snapshot.currentGoal,
    statusSummary: snapshot.statusSummary,
    achievements: snapshot.achievements.join("\n"),
    blockers: snapshot.blockers.join("\n"),
    nextSteps: snapshot.nextSteps.join("\n"),
  };
}

function parseList(value: string): string[] {
  return value
    .split(/\r?\n/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function resolveSnapshotState(
  current: ProjectSnapshot | null,
  history: ProjectSnapshot[],
): { current: ProjectSnapshot | null; history: ProjectSnapshot[] } {
  const byId = new Map(history.map((snapshot) => [snapshot.id, snapshot]));
  if (current) {
    byId.set(current.id, current);
  }

  const ordered = [...byId.values()].sort(
    (left, right) => right.version - left.version,
  );
  return { current: ordered[0] ?? null, history: ordered };
}

function HistoryList({ items, ordered = false }: { items: string[]; ordered?: boolean }) {
  if (items.length === 0) {
    return <p className="snapshot-empty-value">无</p>;
  }

  const List = ordered ? "ol" : "ul";
  return (
    <List className="snapshot-list snapshot-history-list">
      {items.map((item, index) => (
        <li key={`${index}-${item}`}>{item}</li>
      ))}
    </List>
  );
}

export function SnapshotPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [current, setCurrent] = useState<ProjectSnapshot | null>();
  const [history, setHistory] = useState<ProjectSnapshot[]>([]);
  const [draft, setDraft] = useState<SnapshotDraft>(EMPTY_DRAFT);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [hasVersionConflict, setHasVersionConflict] = useState(false);
  const loadGeneration = useRef(0);

  const loadPage = useCallback(async () => {
    const generation = ++loadGeneration.current;
    if (!projectId) {
      setProject(undefined);
      setCurrent(undefined);
      setHistory([]);
      setDraft(EMPTY_DRAFT);
      setError("项目 ID 缺失。");
      setLoading(false);
      return;
    }

    setLoading(true);
    // React can reuse this route component for a new projectId. Clear every
    // project-scoped value before loading to prevent cross-project context bleed.
    setProject(undefined);
    setCurrent(undefined);
    setHistory([]);
    setDraft(EMPTY_DRAFT);
    setSaving(false);
    setError(undefined);
    setNotice(undefined);
    setHasVersionConflict(false);
    try {
      const [projectValue, currentValue, historyValue] = await Promise.all([
        projectKernelApi.getProject(projectId),
        snapshotKernelApi.getCurrentSnapshot(projectId),
        snapshotKernelApi.getSnapshotHistory(projectId),
      ]);
      if (generation !== loadGeneration.current) {
        return;
      }
      const resolved = resolveSnapshotState(currentValue, historyValue);
      setProject(projectValue);
      setCurrent(resolved.current);
      setHistory(resolved.history);
      setDraft(resolved.current ? toDraft(resolved.current) : EMPTY_DRAFT);
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setLoading(false);
      }
    }
  }, [projectId]);

  useEffect(() => {
    void loadPage();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadPage]);

  const parsedNextSteps = useMemo(() => parseList(draft.nextSteps), [draft.nextSteps]);
  const nextStepsExceeded = parsedNextSteps.length > 3;

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !project || project.status === "archived") {
      return;
    }

    if (nextStepsExceeded) {
      setError("下一步最多允许 3 项，请精简后再保存。");
      return;
    }

    const input: SnapshotContentInput = {
      currentGoal: draft.currentGoal.trim(),
      statusSummary: draft.statusSummary.trim(),
      achievements: parseList(draft.achievements),
      blockers: parseList(draft.blockers),
      nextSteps: parsedNextSteps,
    };

    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setHasVersionConflict(false);
    const saveGeneration = loadGeneration.current;

    try {
      const wasUpdate = Boolean(current);
      const saved = current
        ? await snapshotKernelApi.updateSnapshot(projectId, {
            ...input,
            expectedVersion: current.version,
          })
        : await snapshotKernelApi.createSnapshot(projectId, input);
      if (saveGeneration !== loadGeneration.current) {
        return;
      }

      const optimistic = resolveSnapshotState(saved, [saved, ...history]);
      setCurrent(optimistic.current);
      setDraft(optimistic.current ? toDraft(optimistic.current) : EMPTY_DRAFT);
      setHistory(optimistic.history);
      setNotice(
        wasUpdate
          ? `已确认新版本 Snapshot v${saved.version}，历史版本保持不变。`
          : `已创建首个 Snapshot v${saved.version}。`,
      );

      try {
        const historyValue = await snapshotKernelApi.getSnapshotHistory(projectId);
        if (saveGeneration !== loadGeneration.current) {
          return;
        }
        const resolved = resolveSnapshotState(saved, historyValue);
        setCurrent(resolved.current);
        setDraft(resolved.current ? toDraft(resolved.current) : EMPTY_DRAFT);
        setHistory(resolved.history);
        if (resolved.current && resolved.current.version > saved.version) {
          setNotice(
            `已保存快照 v${saved.version}；同时检测到更新的当前版本 v${resolved.current.version}，已载入最新版本。`,
          );
        }
      } catch (historyError) {
        if (saveGeneration === loadGeneration.current) {
          setError(
            `Snapshot v${saved.version} 已保存，但历史刷新失败：${getErrorMessage(historyError)}`,
          );
        }
      }
    } catch (saveError) {
      if (saveGeneration === loadGeneration.current) {
        setError(getErrorMessage(saveError));
        const errorCode = getErrorCode(saveError);
        setHasVersionConflict(
          errorCode === "snapshot_version_conflict" ||
            errorCode === "snapshot_already_exists",
        );
      }
    } finally {
      if (saveGeneration === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  if (loading) {
    return <p className="loading-copy">正在加载快照上下文…</p>;
  }

  if (!project) {
    return (
      <section className="panel not-found">
        <h1>快照不可用</h1>
        <p>{error ?? "找不到该项目。"}</p>
        <Link to="/projects">返回项目列表</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack snapshot-page">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link>
        <span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link>
        <span>/</span>
        <span>当前快照</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">当前快照 · Current Snapshot</p>
          <h1>{project.name}</h1>
          <p>
            人工确认项目当前真实状态。每次保存都会创建一个不可变的历史版本。
          </p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${project.id}`}>
          返回项目概览
        </Link>
      </header>

      {isArchived ? (
        <div className="archive-banner">
          该项目已归档。Snapshot 及其历史版本保留为只读记录，不能创建或更新。
        </div>
      ) : null}

      {current || !isArchived ? (
        <form className="project-form snapshot-editor" onSubmit={handleSave}>
          <section className="panel">
            <div className="panel-header snapshot-editor-header">
              <div>
                <h2>{current ? `确认快照 v${current.version + 1}` : "创建快照 v1"}</h2>
                <p>
                  {current
                    ? `基于当前 v${current.version} 确认新状态；保存不会覆盖已有版本。`
                    : "记录项目第一份当前状态。"}
                </p>
              </div>
              {current ? (
                <span className="version-badge">当前 v{current.version}</span>
              ) : null}
            </div>

            <fieldset className="editor-fields" disabled={isArchived || saving}>
              <div className="form-grid snapshot-form-grid">
                <label className="span-2">
                  当前目标 <span className="required">*</span>
                  <textarea
                    required
                    rows={3}
                    value={draft.currentGoal}
                    onChange={(event) =>
                      setDraft({ ...draft, currentGoal: event.target.value })
                    }
                    placeholder="当前项目最重要、最需要保持聚焦的目标"
                  />
                </label>
                <label className="span-2">
                  状态总结 <span className="required">*</span>
                  <textarea
                    required
                    rows={4}
                    value={draft.statusSummary}
                    onChange={(event) =>
                      setDraft({ ...draft, statusSummary: event.target.value })
                    }
                    placeholder="当前整体状态、已验证到哪里，以及仍然不确定的部分"
                  />
                </label>
                <label>
                  最近成果
                  <textarea
                    rows={6}
                    value={draft.achievements}
                    onChange={(event) =>
                      setDraft({ ...draft, achievements: event.target.value })
                    }
                    placeholder={"每行一项\n完成可运行原型\n验证核心数据流"}
                  />
                  <small>每行会保存为一个结构化条目。</small>
                </label>
                <label>
                  当前阻塞
                  <textarea
                    rows={6}
                    value={draft.blockers}
                    onChange={(event) =>
                      setDraft({ ...draft, blockers: event.target.value })
                    }
                    placeholder={"每行一项\n大型仓库分析速度不足"}
                  />
                  <small>没有阻塞时可以留空。</small>
                </label>
                <label className="span-2">
                  下一步方向
                  <textarea
                    aria-invalid={nextStepsExceeded}
                    className={nextStepsExceeded ? "input-invalid" : undefined}
                    rows={4}
                    value={draft.nextSteps}
                    onChange={(event) =>
                      setDraft({ ...draft, nextSteps: event.target.value })
                    }
                    placeholder={"每行一项，最多三项\n优化索引\n增加缓存\n完善测试"}
                  />
                  <small className={nextStepsExceeded ? "field-error" : undefined}>
                    已填写 {parsedNextSteps.length}/3 项
                    {nextStepsExceeded ? "；超过上限，无法保存。" : "。"}
                  </small>
                </label>
              </div>
            </fieldset>
          </section>

          {error ? (
            <div className="error-banner conflict-banner">
              <span>{error}</span>
              {hasVersionConflict ? (
                <button
                  className="secondary-button"
                  onClick={() => {
                    if (window.confirm("重新载入会放弃当前未保存的修改，是否继续？")) {
                      setNotice(undefined);
                      void loadPage();
                    }
                  }}
                  type="button"
                >
                  重新加载最新版本
                </button>
              ) : null}
            </div>
          ) : null}
          {notice ? <div className="success-banner">{notice}</div> : null}

          {!isArchived ? (
            <div className="sticky-actions snapshot-actions">
              <span>
                {current
                  ? `保存后将确认 v${current.version + 1}；v${current.version} 会保留在历史中。`
                  : "保存后将创建第一份已确认快照。"}
              </span>
              <div>
                <button
                  className="secondary-button"
                  disabled={saving}
                  onClick={() => {
                    setDraft(current ? toDraft(current) : EMPTY_DRAFT);
                    setError(undefined);
                    setNotice(undefined);
                  }}
                  type="button"
                >
                  重置
                </button>
                <button
                  className="primary-button"
                  disabled={saving || nextStepsExceeded}
                  type="submit"
                >
                  {saving
                    ? "正在确认…"
                    : current
                      ? "确认新版本"
                      : "创建快照"}
                </button>
              </div>
            </div>
          ) : null}
        </form>
      ) : (
        <section className="panel snapshot-empty-state">
          <strong>没有可查看的 Snapshot</strong>
          <p>该项目在归档前没有确认过当前状态。</p>
        </section>
      )}

      <section className="panel snapshot-history-panel">
        <div className="panel-header">
          <div>
            <h2>快照历史</h2>
            <p>所有已确认版本，按最新版本优先展示。</p>
          </div>
          <span className="history-count">{history.length} 个版本</span>
        </div>

        {history.length === 0 ? (
          <p className="empty-state">创建首个快照后，版本历史会显示在这里。</p>
        ) : (
          <div className="snapshot-history">
            {history.map((snapshot) => (
              <article className="snapshot-history-item" key={snapshot.id}>
                <header>
                  <div>
                    <span className="version-badge">v{snapshot.version}</span>
                    {snapshot.id === current?.id ? (
                      <span className="current-badge">当前</span>
                    ) : null}
                  </div>
                  <time dateTime={snapshot.confirmedAt}>
                    确认于 {formatUpdatedAt(snapshot.confirmedAt)}
                  </time>
                </header>
                <div className="history-summary-grid">
                  <div>
                    <h3>当前目标</h3>
                    <p>{snapshot.currentGoal}</p>
                  </div>
                  <div>
                    <h3>状态总结</h3>
                    <p>{snapshot.statusSummary}</p>
                  </div>
                </div>
                <div className="history-list-grid">
                  <div>
                    <h3>成果</h3>
                    <HistoryList items={snapshot.achievements} />
                  </div>
                  <div>
                    <h3>阻塞</h3>
                    <HistoryList items={snapshot.blockers} />
                  </div>
                  <div>
                    <h3>下一步</h3>
                    <HistoryList items={snapshot.nextSteps} ordered />
                  </div>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
