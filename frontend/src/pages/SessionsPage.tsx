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
import {
  CollapsibleText,
  RecordDisclosure,
} from "../components/content/ContentGovernance";
import { formatFactPreview } from "../utils/contentPreview";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";
import { displayLabel, formatUpdatedAt } from "../utils/format";

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
  const { t } = useTranslation();
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
  const [expandedSessionId, setExpandedSessionId] = useState<string | null>(null);
  const [historyLimit, setHistoryLimit] = useState(10);
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
      setExpandedSessionId(null);
      setHistoryLimit(10);
      setSaving(false);
      setError(t("session.projectMissing"));
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
    setExpandedSessionId(null);
    setHistoryLimit(10);
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
          t("session.refreshFailed", { version: saved.version, error: getErrorMessage(refreshError) }),
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
      setError(t("session.goalRequired"));
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
      setNotice(t("session.startedNotice", { version: started.version }));
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
      setError(t("session.goalEmpty"));
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
      setNotice(t("session.savedNotice", { version: updated.version }));
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
      setError(t("session.finishRequired"));
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
      setNotice(t("session.completedNotice"));
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
      setError(t("session.abandonRequired"));
      return;
    }
    if (!window.confirm(t("session.abandonConfirm"))) {
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
      setNotice(t("session.abandonedNotice"));
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
    return <p className="loading-copy">{t("common.loading")}…</p>;
  }

  if (!project) {
    return (
      <section className="panel not-found">
        <h1>{t("session.title")}</h1>
        <p>{error ?? t("projects.noProject")}</p>
        <Link to="/projects">{t("navigation.projects")}</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack sessions-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link>
        <span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link>
        <span>/</span>
        <span>{t("session.title")}</span>
      </div>

      <PageIntro title={t("session.title")} technicalTerm={t("terms.session")} description={t("session.description")} secondaryActions={<Link className="secondary-button button-link" to={`/projects/${project.id}`}>{t("common.backToProject")}</Link>} />

      {isArchived ? (
        <div className="archive-banner">
          {current
            ? t("session.archivedCurrentNotice")
            : t("session.archivedNotice")}
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
                  window.confirm(t("session.reloadConfirm"))
                ) {
                  void loadPage();
                }
              }}
              type="button"
            >
              {t("session.reloadLatest")}
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
                  <p className="session-kicker">{t("session.current")}</p>
                  <h2>{current.goal}</h2>
                  <p>
                    {t("session.startedAtVersion", { time: formatUpdatedAt(current.startedAt), version: current.version })}
                  </p>
                </div>
                <span className="session-status session-status--open">{t("session.open")}</span>
              </div>
              <fieldset className="editor-fields" disabled={saving || isArchived}>
                <div className="form-grid session-form-grid">
                  <label className="span-2">
                    {t("session.goal")} <span className="required">*</span>
                    <textarea
                      maxLength={2_000}
                      required
                      rows={3}
                      value={openDraft.goal}
                      onChange={(event) =>
                        setOpenDraft({ ...openDraft, goal: event.target.value })
                      }
                      placeholder={t("session.goalPlaceholder")}
                    />
                  </label>
                  <label className="span-2">
                    {t("session.notes")} <span className="markdown-hint">Markdown</span>
                    <textarea
                      className="session-notes-input"
                      maxLength={20_000}
                      rows={10}
                      value={openDraft.notes}
                      onChange={(event) =>
                        setOpenDraft({ ...openDraft, notes: event.target.value })
                      }
                      placeholder={t("session.notesPlaceholder")}
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
                  {t("session.reset")}
                </button>
                <button className="primary-button" disabled={saving} type="submit">
                  {saving ? t("common.saving") : t("session.continue")}
                </button>
              </div>
            ) : null}
          </form>

          {isArchived ? (
            <div className="archive-banner session-close-banner">
              {t("session.archivedFinishOnly")}
            </div>
          ) : null}

          {current ? (
            <form className="project-form session-finish-form" onSubmit={handleComplete}>
              <section className="panel">
                <div className="panel-header session-editor-header">
                  <div>
                    <h2>{t("session.complete")}</h2>
                    <p>{t("session.finishDescription")}</p>
                  </div>
                </div>
                <fieldset className="editor-fields" disabled={saving}>
                  <div className="form-grid session-form-grid">
                    <label>
                      {t("session.outcome")} <span className="required">*</span>
                      <textarea
                        maxLength={20_000}
                        required
                        rows={6}
                        value={finishDraft.outcome}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            outcome: event.target.value,
                          })
                        }
                        placeholder={t("session.outcomePlaceholder")}
                      />
                    </label>
                    <label>
                      {t("session.validation")} <span className="required">*</span>
                      <textarea
                        maxLength={20_000}
                        required
                        rows={6}
                        value={finishDraft.validation}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            validation: event.target.value,
                          })
                        }
                        placeholder={t("session.validationPlaceholder")}
                      />
                    </label>
                    <label className="span-2">
                      {t("session.handoff")} <span className="required">*</span>
                      <textarea
                        maxLength={20_000}
                        required
                        rows={5}
                        value={finishDraft.handoff}
                        onChange={(event) =>
                          setFinishDraft({
                            ...finishDraft,
                            handoff: event.target.value,
                          })
                        }
                        placeholder={t("session.handoffPlaceholder")}
                      />
                    </label>
                  </div>
                </fieldset>
              </section>

              <div className="sticky-actions session-close-actions">
                <span>{t("session.readonlyAfterFinish")}</span>
                <div>
                  <button
                    className="danger-button"
                    disabled={saving}
                    onClick={() => void handleAbandon()}
                    type="button"
                  >
                    {t("session.abandon")}
                  </button>
                  <button
                    className="primary-button"
                    disabled={saving}
                    type="submit"
                  >
                    {saving ? t("common.saving") : t("session.complete")}
                  </button>
                </div>
              </div>
            </form>
          ) : null}
        </>
      ) : isArchived ? (
        <section className="panel session-empty-state">
          <div>
          <strong>{t("session.emptyTitle")}</strong>
            <p>{t("session.archivedHistoryOnly")}</p>
          </div>
        </section>
      ) : (
        <form className="project-form" onSubmit={handleStart}>
          <section className="panel session-start-panel">
            <div className="panel-header">
              <div>
                <p className="session-kicker">{t("session.newRecord")}</p>
                <h2>{t("session.start")}</h2>
                <p>{t("session.oneOpenOnly")}</p>
              </div>
            </div>
            <div className="form-grid">
              <label className="span-2">
                {t("session.goal")} <span className="required">*</span>
                <textarea
                  autoFocus
                  maxLength={2_000}
                  required
                  rows={4}
                  value={startGoal}
                  onChange={(event) => setStartGoal(event.target.value)}
                  placeholder={t("session.goalPlaceholder")}
                />
                <small>{t("session.goalHelp")}</small>
              </label>
            </div>
            <div className="form-actions session-start-actions">
              <button className="primary-button" disabled={saving} type="submit">
                {saving ? t("common.saving") : t("session.start")}
              </button>
            </div>
          </section>
        </form>
      )}

      <section className="panel session-history-panel">
        <div className="panel-header">
          <div>
            <h2>{t("session.history")}</h2>
            <p>{t("session.historyDescription")}</p>
          </div>
          <span className="history-count">{t("session.historyCount", { count: history.length })}</span>
        </div>

        {history.length === 0 ? (
          <p className="empty-state">{t("session.historyEmpty")}</p>
        ) : (
          <div className="session-history governed-history">
            {history.slice(0, historyLimit).map((session) => (
              <RecordDisclosure
                className="session-history-item"
                expanded={expandedSessionId === session.id}
                key={session.id}
                onToggle={(id) =>
                  setExpandedSessionId((currentId) =>
                    currentId === id ? null : id,
                  )
                }
                recordId={session.id}
                summary={
                  <div className="history-record-summary">
                    <div>
                      <span className={`session-status session-status--${session.status}`}>
                        {displayLabel(session.status)}
                      </span>
                      <span className="session-version">v{session.version}</span>
                      <time dateTime={session.startedAt}>
                        {formatUpdatedAt(session.startedAt)}
                      </time>
                    </div>
                    <strong>{formatFactPreview(session.goal)}</strong>
                    <small>
                      {session.completedAt
                        ? t("session.endedAt", { time: formatUpdatedAt(session.completedAt) })
                        : t("session.noEndTime")}
                    </small>
                  </div>
                }
              >
                <div className="session-history-goal">
                  <h3>{t("session.goal")}</h3>
                  <CollapsibleText collapsedLines={4} previewMode="fact" value={session.goal} />
                </div>
                {session.notes ? (
                  <div className="session-history-field">
                    <h3>{t("session.notes")}</h3>
                    <CollapsibleText collapsedLines={5} expandedMaxHeight={480} value={session.notes} />
                  </div>
                ) : null}
                <div className="session-result-grid">
                  <div>
                    <h3>{t("session.outcome")}</h3>
                    <CollapsibleText collapsedLines={4} value={session.outcome || t("session.noOutcome")} />
                  </div>
                  <div>
                    <h3>{t("session.validation")}</h3>
                    <CollapsibleText collapsedLines={4} value={session.validation || t("session.noValidation")} />
                  </div>
                </div>
                <div className="session-history-field session-handoff-field">
                  <h3>{t("session.handoff")}</h3>
                  <CollapsibleText collapsedLines={4} value={session.handoff || t("session.noHandoff")} />
                </div>
              </RecordDisclosure>
            ))}
            {historyLimit < history.length ? (
              <button
                className="secondary-button content-load-more"
                onClick={() =>
                  setHistoryLimit((value) => Math.min(value + 10, history.length))
                }
                type="button"
              >
                {t("session.showMore", { count: Math.min(10, history.length - historyLimit) })}
              </button>
            ) : null}
          </div>
        )}
      </section>
    </div>
  );
}
