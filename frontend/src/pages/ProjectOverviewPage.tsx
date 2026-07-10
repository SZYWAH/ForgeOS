import {
  type FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { Link, useParams } from "react-router-dom";
import { CurrentSnapshotCard } from "../components/CurrentSnapshotCard";
import {
  getErrorCode,
  getErrorMessage,
  projectKernelApi,
} from "../api/project-kernel";
import {
  LIFECYCLE_STAGES,
  PROJECT_STATUSES,
  type Project,
  type UpdateProjectInput,
} from "../types/project";
import {
  formatUpdatedAt,
  lifecycleLabel,
  statusLabel,
} from "../utils/format";

export function ProjectOverviewPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [draft, setDraft] = useState<Project>();
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [hasVersionConflict, setHasVersionConflict] = useState(false);
  const loadGeneration = useRef(0);

  const loadProject = useCallback(async () => {
    const generation = ++loadGeneration.current;
    if (!projectId) {
      setProject(undefined);
      setDraft(undefined);
      setSaving(false);
      setError("Project ID 缺失。");
      setLoading(false);
      return;
    }

    setLoading(true);
    setProject(undefined);
    setDraft(undefined);
    setSaving(false);
    setError(undefined);
    setNotice(undefined);
    setHasVersionConflict(false);
    try {
      const value = await projectKernelApi.getProject(projectId);
      if (generation !== loadGeneration.current) {
        return;
      }
      setProject(value);
      setDraft(value);
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
    void loadProject();
    return () => {
      loadGeneration.current += 1;
    };
  }, [loadProject]);

  async function handleSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !draft) {
      return;
    }

    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setHasVersionConflict(false);
    const saveGeneration = loadGeneration.current;

    const input: UpdateProjectInput = {
      slug: draft.slug.trim(),
      name: draft.name.trim(),
      description: draft.description.trim(),
      vision: draft.vision.trim(),
      targetUser: draft.targetUser.trim(),
      scope: draft.scope.trim(),
      nonGoals: draft.nonGoals.trim(),
      techStack: draft.techStack.trim(),
      status: draft.status,
      lifecycleStage: draft.lifecycleStage,
      priority: draft.priority,
      expectedVersion: draft.version,
    };

    try {
      const updated = await projectKernelApi.updateProject(projectId, input);
      if (saveGeneration !== loadGeneration.current) {
        return;
      }
      setProject(updated);
      setDraft(updated);
      setNotice(`已保存。当前版本 v${updated.version}`);
    } catch (saveError) {
      if (saveGeneration === loadGeneration.current) {
        setError(getErrorMessage(saveError));
        setHasVersionConflict(getErrorCode(saveError) === "version_conflict");
      }
    } finally {
      if (saveGeneration === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  async function handleArchive() {
    if (!projectId || !draft || draft.status === "archived") {
      return;
    }

    if (!window.confirm(`确认归档 ${draft.name}？项目记录会保留。`)) {
      return;
    }

    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    setHasVersionConflict(false);
    const archiveGeneration = loadGeneration.current;
    try {
      const archived = await projectKernelApi.archiveProject(
        projectId,
        draft.version,
      );
      if (archiveGeneration !== loadGeneration.current) {
        return;
      }
      setProject(archived);
      setDraft(archived);
      setNotice(`项目已归档。当前版本 v${archived.version}`);
    } catch (archiveError) {
      if (archiveGeneration === loadGeneration.current) {
        setError(getErrorMessage(archiveError));
        setHasVersionConflict(getErrorCode(archiveError) === "version_conflict");
      }
    } finally {
      if (archiveGeneration === loadGeneration.current) {
        setSaving(false);
      }
    }
  }

  if (loading) {
    return <p className="loading-copy">Loading project…</p>;
  }

  if (!project || !draft) {
    return (
      <section className="panel not-found">
        <h1>Project unavailable</h1>
        <p>{error ?? "找不到该项目。"}</p>
        <Link to="/projects">返回 Projects</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack">
      <div className="breadcrumb">
        <Link to="/projects">Projects</Link>
        <span>/</span>
        <span>{project.name}</span>
      </div>

      <header className="page-header project-title-row">
        <div>
          <div className="title-with-status">
            <h1>{project.name}</h1>
            <span className={`status status--${project.status}`}>
              {statusLabel(project.status)}
            </span>
          </div>
          <p className="project-identity">
            {project.slug} · v{project.version} · Updated {formatUpdatedAt(project.updatedAt)}
          </p>
        </div>
        <button
          className="danger-button"
          disabled={saving || draft.status === "archived"}
          onClick={handleArchive}
          type="button"
        >
          {draft.status === "archived" ? "Archived" : "Archive Project"}
        </button>
      </header>

      <CurrentSnapshotCard
        archived={isArchived}
        projectId={project.id}
        projectName={project.name}
      />


      <form className="project-form overview-form" onSubmit={handleSave}>
        {isArchived ? (
          <div className="archive-banner">
            该项目已归档。项目身份与上下文保留为只读记录。
          </div>
        ) : null}
        <fieldset className="editor-fields" disabled={isArchived}>
        <section className="panel">
          <div className="panel-header">
            <div>
              <h2>Identity</h2>
              <p>项目的稳定名称与边界标识。</p>
            </div>
          </div>
          <div className="form-grid">
            <label>
              Name <span className="required">*</span>
              <input
                required
                value={draft.name}
                onChange={(event) =>
                  setDraft({ ...draft, name: event.target.value })
                }
              />
            </label>
            <label>
              Slug <span className="required">*</span>
              <input
                required
                minLength={2}
                maxLength={64}
                pattern="[a-z0-9]+(?:-[a-z0-9]+)*"
                value={draft.slug}
                onChange={(event) =>
                  setDraft({ ...draft, slug: event.target.value })
                }
              />
            </label>
            <label className="span-2">
              Description
              <textarea
                rows={3}
                value={draft.description}
                onChange={(event) =>
                  setDraft({ ...draft, description: event.target.value })
                }
              />
            </label>
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <div>
              <h2>Product Context</h2>
              <p>说明项目为何存在、服务谁，以及明确的范围边界。</p>
            </div>
          </div>
          <div className="form-grid">
            <label className="span-2">
              Vision
              <textarea
                rows={3}
                value={draft.vision}
                onChange={(event) =>
                  setDraft({ ...draft, vision: event.target.value })
                }
              />
            </label>
            <label className="span-2">
              Target User
              <textarea
                rows={2}
                value={draft.targetUser}
                onChange={(event) =>
                  setDraft({ ...draft, targetUser: event.target.value })
                }
              />
            </label>
            <label>
              Scope
              <textarea
                rows={5}
                value={draft.scope}
                onChange={(event) =>
                  setDraft({ ...draft, scope: event.target.value })
                }
              />
            </label>
            <label>
              Non-goals
              <textarea
                rows={5}
                value={draft.nonGoals}
                onChange={(event) =>
                  setDraft({ ...draft, nonGoals: event.target.value })
                }
              />
            </label>
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <div>
              <h2>Engineering & Lifecycle</h2>
              <p>维护技术栈、当前状态和研发生命周期阶段。</p>
            </div>
          </div>
          <div className="form-grid">
            <label className="span-2">
              Tech Stack
              <textarea
                rows={3}
                value={draft.techStack}
                onChange={(event) =>
                  setDraft({ ...draft, techStack: event.target.value })
                }
                placeholder="React, TypeScript, Tauri 2, Rust, SQLite"
              />
            </label>
            <label>
              Status
              <select
                value={draft.status}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    status: event.target.value as Project["status"],
                  })
                }
              >
                {PROJECT_STATUSES.filter(
                  (status) => status !== "archived" || isArchived,
                ).map((status) => (
                  <option key={status} value={status}>
                    {statusLabel(status)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Lifecycle
              <select
                value={draft.lifecycleStage}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    lifecycleStage: event.target.value as Project["lifecycleStage"],
                  })
                }
              >
                {LIFECYCLE_STAGES.map((stage) => (
                  <option key={stage} value={stage}>
                    {lifecycleLabel(stage)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Priority
              <input
                max={100}
                min={0}
                type="number"
                value={draft.priority}
                onChange={(event) =>
                  setDraft({ ...draft, priority: Number(event.target.value) })
                }
              />
            </label>
          </div>
        </section>
        </fieldset>

        {error ? (
          <div className="error-banner conflict-banner">
            <span>{error}</span>
            {hasVersionConflict ? (
              <button
                className="secondary-button"
                onClick={() => {
                  if (window.confirm("重新载入会放弃当前未保存的修改，是否继续？")) {
                    void loadProject();
                  }
                }}
                type="button"
              >
                Reload Latest
              </button>
            ) : null}
          </div>
        ) : null}
        {notice ? <div className="success-banner">{notice}</div> : null}

        <div className="sticky-actions">
          <span>Changes use version {draft.version} for conflict protection.</span>
          <div>
            <button
              className="secondary-button"
              disabled={saving || isArchived}
              onClick={() => {
                setDraft(project);
                setError(undefined);
                setNotice(undefined);
              }}
              type="button"
            >
              Reset
            </button>
            <button
              className="primary-button"
              disabled={saving || isArchived}
              type="submit"
            >
              {saving ? "Saving…" : "Save Changes"}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
