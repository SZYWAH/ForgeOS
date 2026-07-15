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
import { BoundedList, CollapsibleText, RecordDisclosure } from "../components/content/ContentGovernance";
import { formatFactPreview } from "../utils/contentPreview";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

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

export function SnapshotPage() {
  const { t } = useTranslation();
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
  const [expandedSnapshotId, setExpandedSnapshotId] = useState<string | null>(null);
  const [historyLimit, setHistoryLimit] = useState(10);
  const loadGeneration = useRef(0);

  const loadPage = useCallback(async () => {
    const generation = ++loadGeneration.current;
    if (!projectId) {
      setProject(undefined);
      setCurrent(undefined);
      setHistory([]);
      setDraft(EMPTY_DRAFT);
      setError(t("projects.noProject"));
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
    setExpandedSnapshotId(null);
    setHistoryLimit(10);
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
  }, [projectId, t]);

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
      setError(t("snapshot.nextStepsLimit"));
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
          ? t("snapshot.savedNewVersion", { version: saved.version })
          : t("snapshot.savedFirst", { version: saved.version }),
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
            t("snapshot.savedNewerLoaded", { version: saved.version, current: resolved.current.version }),
          );
        }
      } catch (historyError) {
        if (saveGeneration === loadGeneration.current) {
          setError(
            t("snapshot.savedHistoryFailed", { version: saved.version, error: getErrorMessage(historyError) }),
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
    return <p className="loading-copy">{t("common.loading")}…</p>;
  }

  if (!project) {
    return (
      <section className="panel not-found">
        <h1>{t("snapshot.title")}</h1>
        <p>{error ?? t("projects.noProject")}</p>
        <Link to="/projects">{t("navigation.projects")}</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack snapshot-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link>
        <span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link>
        <span>/</span>
        <span>{t("snapshot.title")}</span>
      </div>

      <PageIntro
        title={t("snapshot.title")}
        technicalTerm={t("terms.snapshot")}
        description={t("snapshot.description")}
        secondaryActions={<Link className="secondary-button button-link" to={`/projects/${project.id}`}>{t("common.backToProject")}</Link>}
      />

      {isArchived ? (
        <div className="archive-banner">
          {t("snapshot.archivedNotice")}
        </div>
      ) : null}

      {current || !isArchived ? (
        <form className="project-form snapshot-editor" onSubmit={handleSave}>
          <section className="panel">
            <div className="panel-header snapshot-editor-header">
              <div>
                <h2>{current ? t("snapshot.confirmVersion", { version: current.version + 1 }) : t("snapshot.createVersion")}</h2>
                <p>
                  {current
                    ? t("snapshot.versionHint", { version: current.version })
                    : t("snapshot.firstHint")}
                </p>
              </div>
              {current ? (
                <span className="version-badge">{t("snapshot.currentVersion", { version: current.version })}</span>
              ) : null}
            </div>

            <fieldset className="editor-fields" disabled={isArchived || saving}>
              <div className="form-grid snapshot-form-grid">
                <label className="span-2">
                  {t("projects.currentGoal")} <span className="required">*</span>
                  <textarea
                    maxLength={20_000}
                    required
                    rows={3}
                    value={draft.currentGoal}
                    onChange={(event) =>
                      setDraft({ ...draft, currentGoal: event.target.value })
                    }
                    placeholder={t("snapshot.goalPlaceholder")}
                  />
                </label>
                <label className="span-2">
                  {t("projects.currentSituation")} <span className="required">*</span>
                  <textarea
                    maxLength={20_000}
                    required
                    rows={4}
                    value={draft.statusSummary}
                    onChange={(event) =>
                      setDraft({ ...draft, statusSummary: event.target.value })
                    }
                    placeholder={t("snapshot.situationPlaceholder")}
                  />
                </label>
                <label>
                  {t("projects.achievements")}
                  <textarea
                    maxLength={20_000}
                    rows={6}
                    value={draft.achievements}
                    onChange={(event) =>
                      setDraft({ ...draft, achievements: event.target.value })
                    }
                    placeholder={t("snapshot.achievementsPlaceholder")}
                  />
                  <small>{t("snapshot.achievementsHelp")}</small>
                </label>
                <label>
                  {t("projects.blockers")}
                  <textarea
                    maxLength={20_000}
                    rows={6}
                    value={draft.blockers}
                    onChange={(event) =>
                      setDraft({ ...draft, blockers: event.target.value })
                    }
                    placeholder={t("snapshot.blockersPlaceholder")}
                  />
                  <small>{t("snapshot.blockersHelp")}</small>
                </label>
                <label className="span-2">
                  {t("projects.nextSteps")}
                  <textarea
                    aria-invalid={nextStepsExceeded}
                    className={nextStepsExceeded ? "input-invalid" : undefined}
                    maxLength={20_000}
                    rows={4}
                    value={draft.nextSteps}
                    onChange={(event) =>
                      setDraft({ ...draft, nextSteps: event.target.value })
                    }
                    placeholder={t("snapshot.nextStepsPlaceholder")}
                  />
                  <small className={nextStepsExceeded ? "field-error" : undefined}>
                    {t("snapshot.nextStepsCount", { count: parsedNextSteps.length })}
                    {nextStepsExceeded ? t("snapshot.nextStepsExceeded") : "。"}
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
                    if (window.confirm(t("snapshot.reloadConfirm"))) {
                      setNotice(undefined);
                      void loadPage();
                    }
                  }}
                  type="button"
                >
                  {t("snapshot.reloadLatest")}
                </button>
              ) : null}
            </div>
          ) : null}
          {notice ? <div className="success-banner">{notice}</div> : null}

          {!isArchived ? (
            <div className="sticky-actions snapshot-actions">
              <span>
                {current
                  ? t("snapshot.saveVersionHint", { next: current.version + 1, current: current.version })
                  : t("snapshot.saveFirstHint")}
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
                  {t("snapshot.reset")}
                </button>
                <button
                  className="primary-button"
                  disabled={saving || nextStepsExceeded}
                  type="submit"
                >
                  {saving
                    ? t("snapshot.confirming")
                    : current
                      ? t("snapshot.update")
                      : t("snapshot.create")}
                </button>
              </div>
            </div>
          ) : null}
        </form>
      ) : (
        <section className="panel snapshot-empty-state">
          <strong>{t("snapshot.emptyTitle")}</strong>
          <p>{t("snapshot.emptyDescription")}</p>
        </section>
      )}

      <section className="panel snapshot-history-panel">
        <div className="panel-header">
          <div>
            <h2>{t("snapshot.history")}</h2>
            <p>{t("snapshot.historyDescription")}</p>
          </div>
          <span className="history-count">{t("snapshot.versionCount", { count: history.length })}</span>
        </div>

        {history.length === 0 ? (
          <p className="empty-state">{t("snapshot.historyEmpty")}</p>
        ) : (
          <div className="snapshot-history governed-history">
            {history.slice(0, historyLimit).map((snapshot) => (
              <RecordDisclosure
                className="snapshot-history-item"
                expanded={expandedSnapshotId === snapshot.id}
                key={snapshot.id}
                onToggle={(id) => setExpandedSnapshotId((currentId) => currentId === id ? null : id)}
                recordId={snapshot.id}
                summary={<div className="history-record-summary"><div><span className="version-badge">v{snapshot.version}</span>{snapshot.id === current?.id ? <span className="current-badge">{t("snapshot.current")}</span> : null}<time dateTime={snapshot.confirmedAt}>{t("snapshot.confirmedTime", { time: formatUpdatedAt(snapshot.confirmedAt) })}</time></div><strong>{formatFactPreview(snapshot.currentGoal)}</strong><small>{snapshot.statusSummary}</small></div>}
              >
                <div className="history-summary-grid">
                  <div>
                    <h3>{t("projects.currentGoal")}</h3>
                    <CollapsibleText collapsedLines={4} previewMode="fact" value={snapshot.currentGoal} />
                  </div>
                  <div>
                    <h3>{t("projects.currentSituation")}</h3>
                    <CollapsibleText collapsedLines={4} value={snapshot.statusSummary} />
                  </div>
                </div>
                <div className="history-list-grid">
                  <div>
                    <h3>{t("projects.achievements")}</h3>
                    <BoundedList items={snapshot.achievements} emptyLabel={t("common.none")} />
                  </div>
                  <div>
                    <h3>{t("projects.blockers")}</h3>
                    <BoundedList items={snapshot.blockers} emptyLabel={t("common.none")} />
                  </div>
                  <div>
                    <h3>{t("projects.nextSteps")}</h3>
                    <BoundedList items={snapshot.nextSteps} emptyLabel={t("common.none")} ordered />
                  </div>
                </div>
              </RecordDisclosure>
            ))}
            {historyLimit < history.length ? <button className="secondary-button content-load-more" onClick={() => setHistoryLimit((value) => Math.min(value + 10, history.length))} type="button">{t("snapshot.showMore", { count: Math.min(10, history.length - historyLimit) })}</button> : null}
          </div>
        )}
      </section>
    </div>
  );
}
