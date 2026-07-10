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
import { sessionKernelApi } from "../api/session-kernel";
import type { Project } from "../types/project";
import type { DevelopmentSession } from "../types/session";
import { formatUpdatedAt } from "../utils/format";

type OpenSessionDraft = {
  goal: string;
  notes: string;
};

type FinishSessionDraft = {
  outcome: string;
  validation: string;
  handoff: string;
};

const EMPTY_OPEN_DRAFT: OpenSessionDraft = { goal: "", notes: "" };
const EMPTY_FINISH_DRAFT: FinishSessionDraft = {
  outcome: "",
  validation: "",
  handoff: "",
};

function toOpenDraft(session: DevelopmentSession): OpenSessionDraft {
  return { goal: session.goal, notes: session.notes };
}

function compareSessions(
  left: DevelopmentSession,
  right: DevelopmentSession,
): number {
  const byStart =
    new Date(right.startedAt).getTime() - new Date(left.startedAt).getTime();
  return Number.isNaN(byStart) || byStart === 0
    ? right.createdAt.localeCompare(left.createdAt)
    : byStart;
}

function resolveSessionState(
  current: DevelopmentSession | null,
  sessions: DevelopmentSession[],
): { current: DevelopmentSession | null; sessions: DevelopmentSession[] } {
  const byId = new Map(sessions.map((session) => [session.id, session]));
  if (current) {
    const listed = byId.get(current.id);
    if (!listed || current.version >= listed.version) {
      byId.set(current.id, current);
    }
  }

  const ordered = [...byId.values()].sort(compareSessions);
  const canonicalCurrent = current ? byId.get(current.id) : undefined;
  const open =
    canonicalCurrent?.status === "open"
      ? canonicalCurrent
      : (ordered.find((session) => session.status === "open") ?? null);
  return { current: open, sessions: ordered };
}

function statusLabel(status: DevelopmentSession["status"]): string {
  switch (status) {
    case "open":
      return "Open";
    case "completed":
      return "Completed";
    case "abandoned":
      return "Abandoned";
  }
}

function isReloadableSessionError(error: unknown): boolean {
  const code = getErrorCode(error);
  return (
    code === "session_already_open" ||
    code === "session_not_found" ||
    code === "session_version_conflict" ||
    code === "session_terminal_read_only"
  );
}

export function SessionsPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [current, setCurrent] = useState<DevelopmentSession | null>();
  const [sessions, setSessions] = useState<DevelopmentSession[]>([]);
  const [startGoal, setStartGoal] = useState("");
  const [openDraft, setOpenDraft] =
    useState<OpenSessionDraft>(EMPTY_OPEN_DRAFT);
  const [finishDraft, setFinishDraft] =
    useState<FinishSessionDraft>(EMPTY_FINISH_DRAFT);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [canReload, setCanReload] = useState(false);
  const loadGeneration = useRef(0);

  const loadPage = useCallback(async () => {
    const generation = ++loadGeneration.current;
    if (!projectId) {
      setProject(undefined);
      setCurrent(undefined);
      setSessions([]);
      setStartGoal("");
      setOpenDraft(EMPTY_OPEN_DRAFT);
      setFinishDraft(EMPTY_FINISH_DRAFT);
      setSaving(false);
      setError("Project ID 缺失。");
      setLoading(false);
      return;
    }

    setLoading(true);
    setProject(undefined);
    setCurrent(undefined);
    setSessions([]);
    setStartGoal("");
    setOpenDraft(EMPTY_OPEN_DRAFT);
    setFinishDraft(EMPTY_FINISH_DRAFT);
    setSaving(false);
    setError(undefined);
    setNotice(undefined);
    setCanReload(false);

    try {
      const [projectValue, currentValue, sessionsValue] = await Promise.all([
        projectKernelApi.getProject(projectId),
        sessionKernelApi.getCurrentSession(projectId),
        sessionKernelApi.listSessions(projectId),
      ]);
      if (generation !== loadGeneration.current) {
        return;
      }

      const resolved = resolveSessionState(currentValue, sessionsValue);
      setProject(projectValue);
      setCurrent(resolved.current);
      setSessions(resolved.sessions);
      setOpenDraft(
        resolved.current ? toOpenDraft(resolved.current) : EMPTY_OPEN_DRAFT,
      );
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

  const history = useMemo(
    () => sessions.filter((session) => session.status !== "open"),
    [sessions],
  );

  function applySavedSession(saved: DevelopmentSession) {
    const nextSessions = [
      saved,
      ...sessions.filter((session) => session.id !== saved.id),
    ];
    const resolved = resolveSessionState(
      saved.status === "open" ? saved : null,
      nextSessions,
    );
    setCurrent(resolved.current);
    setSessions(resolved.sessions);
    setOpenDraft(
      resolved.current ? toOpenDraft(resolved.current) : EMPTY_OPEN_DRAFT,
    );
  }

  async function refreshAfterMutation(
    generation: number,
    saved: DevelopmentSession,
  ) {
    if (!projectId) {
      return;
    }

    try {
      const [currentValue, sessionsValue] = await Promise.all([
        sessionKernelApi.getCurrentSession(projectId),
        sessionKernelApi.listSessions(projectId),
      ]);
      if (generation !== loadGeneration.current) {
        return;
      }
      const resolved = resolveSessionState(currentValue, sessionsValue);
      setCurrent(resolved.current);
      setSessions(resolved.sessions);
      setOpenDraft(
        resolved.current ? toOpenDraft(resolved.current) : EMPTY_OPEN_DRAFT,
      );
    } catch (refreshError) {
      if (generation === loadGeneration.current) {
        setError(
          `Session v${saved.version} 已保存，但会话列表刷新失败：${getErrorMessage(refreshError)}`,
        );
        setCanReload(true);
      }
    }
  }

  async function handleStart(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !project || project.status === "archived" || current) {
      return;
    }

    const goal = startGoal.trim();
    if (!goal) {
      setError("请填写本次研发目标。");
      return;
    }

    const generation = loadGeneration.current;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setCanReload(false);
    try {
      const started = await sessionKernelApi.startSession(projectId, { goal });
      if (generation !== loadGeneration.current) {
        return;
      }
      applySavedSession(started);
      setStartGoal("");
      setFinishDraft(EMPTY_FINISH_DRAFT);
      setNotice(`Development Session 已开始，当前版本 v${started.version}。`);
      await refreshAfterMutation(generation, started);
    } catch (startError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(startError));
        setCanReload(isReloadableSessionError(startError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  async function handleUpdate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (
      !projectId ||
      !project ||
      project.status === "archived" ||
      !current ||
      current.status !== "open"
    ) {
      return;
    }

    const goal = openDraft.goal.trim();
    if (!goal) {
      setError("开发会话目标不能为空。");
      return;
    }

    const generation = loadGeneration.current;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setCanReload(false);
    try {
      const updated = await sessionKernelApi.updateSession(
        projectId,
        current.id,
        {
          goal,
          notes: openDraft.notes.trim(),
          expectedVersion: current.version,
        },
      );
      if (generation !== loadGeneration.current) {
        return;
      }
      applySavedSession(updated);
      setNotice(`开发会话已保存，当前版本 v${updated.version}。`);
      await refreshAfterMutation(generation, updated);
    } catch (updateError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(updateError));
        setCanReload(isReloadableSessionError(updateError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  async function handleComplete(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (
      !projectId ||
      !project ||
      !current ||
      current.status !== "open"
    ) {
      return;
    }

    const outcome = finishDraft.outcome.trim();
    const validation = finishDraft.validation.trim();
    const handoff = finishDraft.handoff.trim();
    if (!outcome || !validation || !handoff) {
      setError("完成会话前，请填写 Outcome、Validation 和 Handoff。");
      return;
    }

    const generation = loadGeneration.current;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setCanReload(false);
    try {
      const completed = await sessionKernelApi.completeSession(
        projectId,
        current.id,
        {
          outcome,
          validation,
          handoff,
          expectedVersion: current.version,
        },
      );
      if (generation !== loadGeneration.current) {
        return;
      }
      applySavedSession(completed);
      setFinishDraft(EMPTY_FINISH_DRAFT);
      setNotice("Development Session 已完成并保留为只读历史。");
      await refreshAfterMutation(generation, completed);
    } catch (completeError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(completeError));
        setCanReload(isReloadableSessionError(completeError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  async function handleAbandon() {
    if (
      !projectId ||
      !project ||
      !current ||
      current.status !== "open"
    ) {
      return;
    }

    const handoff = finishDraft.handoff.trim();
    if (!handoff) {
      setError("放弃会话前，请填写 Handoff，说明下次需要知道的信息。");
      return;
    }
    if (!window.confirm("确认放弃当前开发会话？记录会保留为只读历史。")) {
      return;
    }

    const generation = loadGeneration.current;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setCanReload(false);
    try {
      const abandoned = await sessionKernelApi.abandonSession(
        projectId,
        current.id,
        { handoff, expectedVersion: current.version },
      );
      if (generation !== loadGeneration.current) {
        return;
      }
      applySavedSession(abandoned);
      setFinishDraft(EMPTY_FINISH_DRAFT);
      setNotice("Development Session 已放弃并保留为只读历史。");
      await refreshAfterMutation(generation, abandoned);
    } catch (abandonError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(abandonError));
        setCanReload(isReloadableSessionError(abandonError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  if (loading) {
    return <p className="loading-copy">Loading development sessions…</p>;
  }

  if (!project) {
    return (
      <section className="panel not-found">
        <h1>Sessions unavailable</h1>
        <p>{error ?? "找不到该项目。"}</p>
        <Link to="/projects">返回 Projects</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack sessions-page">
      <div className="breadcrumb">
        <Link to="/projects">Projects</Link>
        <span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link>
        <span>/</span>
        <span>Development Sessions</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">Development Session</p>
          <h1>{project.name}</h1>
          <p>记录一次研发活动为何开始、做了什么、如何验证，以及下次如何继续。</p>
        </div>
        <Link
          className="secondary-button button-link"
          to={`/projects/${project.id}`}
        >
          Back to Overview
        </Link>
      </header>

      {isArchived ? (
        <div className="archive-banner">
          {current
            ? "该项目已经归档。当前 Open Session 的目标和过程记录只读，但仍可填写收尾信息并完成或放弃会话。"
            : "该项目已经归档。Development Session 保留为只读研发记录。"}
        </div>
      ) : null}

      {error ? (
        <div className="error-banner conflict-banner session-page-message">
          <span>{error}</span>
          {canReload ? (
            <button
              className="secondary-button"
              onClick={() => {
                if (
                  window.confirm(
                    "重新载入会放弃当前未保存的会话内容，是否继续？",
                  )
                ) {
                  void loadPage();
                }
              }}
              type="button"
            >
              Reload Latest
            </button>
          ) : null}
        </div>
      ) : null}
      {notice ? (
        <div className="success-banner session-page-message">{notice}</div>
      ) : null}

      {current ? (
        <>
          <form className="project-form session-editor" onSubmit={handleUpdate}>
            <section className="panel">
              <div className="panel-header session-editor-header">
                <div>
                  <p className="session-kicker">Current Session</p>
                  <h2>{current.goal}</h2>
                  <p>
                    Started {formatUpdatedAt(current.startedAt)} · version {current.version}
                  </p>
                </div>
                <span className="session-status session-status--open">Open</span>
              </div>
              <fieldset className="editor-fields" disabled={saving || isArchived}>
                <div className="form-grid session-form-grid">
                  <label className="span-2">
                    Goal <span className="required">*</span>
                    <textarea
                      required
                      rows={3}
                      value={openDraft.goal}
                      onChange={(event) =>
                        setOpenDraft({ ...openDraft, goal: event.target.value })
                      }
                      placeholder="这次研发活动需要完成什么？"
                    />
                  </label>
                  <label className="span-2">
                    Notes <span className="markdown-hint">Markdown</span>
                    <textarea
                      className="session-notes-input"
                      rows={10}
                      value={openDraft.notes}
                      onChange={(event) =>
                        setOpenDraft({ ...openDraft, notes: event.target.value })
                      }
                      placeholder="记录过程中做了什么、发现了什么，以及重要的中间信息。"
                    />
                  </label>
                </div>
              </fieldset>
            </section>

            {!isArchived ? (
              <div className="form-actions session-inline-actions">
                <button
                  className="secondary-button"
                  disabled={saving}
                  onClick={() => {
                    setOpenDraft(toOpenDraft(current));
                    setError(undefined);
                    setNotice(undefined);
                  }}
                  type="button"
                >
                  Reset
                </button>
                <button className="primary-button" disabled={saving} type="submit">
                  {saving ? "Saving…" : "Save Session"}
                </button>
              </div>
            ) : null}
          </form>

          {isArchived ? (
            <div className="archive-banner session-close-banner">
              归档项目仅允许收尾：不能修改 Goal 或 Notes，也不能开始新会话。
            </div>
          ) : null}

          {current ? (
            <form className="project-form session-finish-form" onSubmit={handleComplete}>
              <section className="panel">
                <div className="panel-header session-editor-header">
                  <div>
                    <h2>Close Development Session</h2>
                    <p>确认结果、验证方式和下次接手信息后，结束这次研发活动。</p>
                  </div>
                </div>
                <fieldset className="editor-fields" disabled={saving}>
                  <div className="form-grid session-form-grid">
                    <label>
                      Outcome <span className="required">*</span>
                      <textarea
                        required
                        rows={6}
                        value={finishDraft.outcome}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            outcome: event.target.value,
                          })
                        }
                        placeholder="这次研发最终得到了什么结果？"
                      />
                    </label>
                    <label>
                      Validation <span className="required">*</span>
                      <textarea
                        required
                        rows={6}
                        value={finishDraft.validation}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            validation: event.target.value,
                          })
                        }
                        placeholder="如何确认结果有效？例如 cargo test 通过。"
                      />
                    </label>
                    <label className="span-2">
                      Handoff <span className="required">*</span>
                      <textarea
                        required
                        rows={5}
                        value={finishDraft.handoff}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            handoff: event.target.value,
                          })
                        }
                        placeholder="下一次继续开发时，最需要知道什么？"
                      />
                    </label>
                  </div>
                </fieldset>
              </section>

              <div className="sticky-actions session-close-actions">
                <span>Completed 或 Abandoned 后，核心结果将成为只读历史。</span>
                <div>
                  <button
                    className="danger-button"
                    disabled={saving}
                    onClick={() => void handleAbandon()}
                    type="button"
                  >
                    Abandon Session
                  </button>
                  <button
                    className="primary-button"
                    disabled={saving}
                    type="submit"
                  >
                    {saving ? "Closing…" : "Complete Session"}
                  </button>
                </div>
              </div>
            </form>
          ) : null}
        </>
      ) : isArchived ? (
        <section className="panel session-empty-state">
          <div>
            <strong>没有进行中的 Development Session</strong>
            <p>该项目已经归档，只能查看下方会话历史。</p>
          </div>
        </section>
      ) : (
        <form className="project-form" onSubmit={handleStart}>
          <section className="panel session-start-panel">
            <div className="panel-header">
              <div>
                <p className="session-kicker">New Session</p>
                <h2>开始新的开发会话</h2>
                <p>一个项目同一时间只能有一个 Open Session。</p>
              </div>
            </div>
            <div className="form-grid">
              <label className="span-2">
                Goal <span className="required">*</span>
                <textarea
                  autoFocus
                  required
                  rows={4}
                  value={startGoal}
                  onChange={(event) => setStartGoal(event.target.value)}
                  placeholder="例如：优化 Agent 流式响应"
                />
                <small>说明为什么开始这次研发活动，以及本次最重要的目标。</small>
              </label>
            </div>
            <div className="form-actions session-start-actions">
              <button className="primary-button" disabled={saving} type="submit">
                {saving ? "Starting…" : "Start Development Session"}
              </button>
            </div>
          </section>
        </form>
      )}

      <section className="panel session-history-panel">
        <div className="panel-header">
          <div>
            <h2>Session History</h2>
            <p>已完成和已放弃的研发活动，按开始时间倒序保留。</p>
          </div>
          <span className="history-count">{history.length} sessions</span>
        </div>

        {history.length === 0 ? (
          <p className="empty-state">结束第一个 Session 后，历史记录会显示在这里。</p>
        ) : (
          <div className="session-history">
            {history.map((session) => (
              <article className="session-history-item" key={session.id}>
                <header>
                  <div>
                    <span
                      className={`session-status session-status--${session.status}`}
                    >
                      {statusLabel(session.status)}
                    </span>
                    <span className="session-version">v{session.version}</span>
                  </div>
                  <div className="session-history-times">
                    <span>Started {formatUpdatedAt(session.startedAt)}</span>
                    {session.completedAt ? (
                      <span>Closed {formatUpdatedAt(session.completedAt)}</span>
                    ) : null}
                  </div>
                </header>
                <div className="session-history-goal">
                  <h3>Goal</h3>
                  <p>{session.goal}</p>
                </div>
                {session.notes ? (
                  <div className="session-history-field">
                    <h3>Notes</h3>
                    <p>{session.notes}</p>
                  </div>
                ) : null}
                <div className="session-result-grid">
                  <div>
                    <h3>Outcome</h3>
                    <p>{session.outcome || "未记录结果。"}</p>
                  </div>
                  <div>
                    <h3>Validation</h3>
                    <p>{session.validation || "未记录验证。"}</p>
                  </div>
                </div>
                <div className="session-history-field session-handoff-field">
                  <h3>Handoff</h3>
                  <p>{session.handoff || "未记录接手信息。"}</p>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
