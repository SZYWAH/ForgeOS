import {
  type FormEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { Link, useParams } from "react-router-dom";
import { CurrentSnapshotCard } from "../components/CurrentSnapshotCard";
import { CurrentSessionCard } from "../components/CurrentSessionCard";
import { EvidenceSummaryCard } from "../components/EvidenceSummaryCard";
import { RepositoryCard } from "../components/RepositoryCard";
import { GitHubRepositoryCard } from "../components/GitHubRepositoryCard";
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
    return <p className="loading-copy">正在加载项目…</p>;
  }

  if (!project || !draft) {
    return (
      <section className="panel not-found">
        <h1>项目不可用</h1>
        <p>{error ?? "找不到该项目。"}</p>
        <Link to="/projects">返回项目列表</Link>
      </section>
    );
  }

  const isArchived = project.status === "archived";

  return (
    <div className="page-stack">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link>
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
            {project.slug} · v{project.version} · 更新于 {formatUpdatedAt(project.updatedAt)}
          </p>
        </div>
        <button
          className="danger-button"
          disabled={saving || draft.status === "archived"}
          onClick={handleArchive}
          type="button"
        >
          {draft.status === "archived" ? "已归档" : "归档项目"}
        </button>
      </header>

      <CurrentSnapshotCard
        archived={isArchived}
        projectId={project.id}
        projectName={project.name}
      />

      <CurrentSessionCard
        archived={isArchived}
        projectId={project.id}
        projectName={project.name}
      />

      <EvidenceSummaryCard projectId={project.id} projectName={project.name} />

      <RepositoryCard archived={isArchived} projectId={project.id} />

      <GitHubRepositoryCard projectId={project.id} />

      <section className="panel context-overview-card">
        <div className="panel-header"><div><p className="snapshot-kicker">仅任务提案</p><h2>Codex 任务提案</h2><p>定义绑定上下文包的编码任务请求；不会执行 Codex 或修改代码。</p></div><Link className="secondary-button button-link" to={`/projects/${project.id}/codex-tasks`}>打开任务提案</Link></div>
      </section>

      <section className="panel context-overview-card">
        <div className="panel-header">
          <div>
            <p className="snapshot-kicker">派生上下文</p>
            <h2>上下文包（Context Pack）</h2>
            <p>从 Project、Current Snapshot、最近 Session 与 Evidence 生成稳定 Markdown。</p>
          </div>
          <Link className="secondary-button button-link" to={`/projects/${project.id}/context`}>
            打开上下文包
          </Link>
        </div>
      </section>

      <section className="panel candidate-overview-card">
        <div className="panel-header">
          <div>
            <p className="snapshot-kicker">AI 输出治理</p>
            <h2>AI 候选建议</h2>
            <p>审核 AI 输出；Accept 仅记录人工认可，不自动修改任何正式事实。</p>
          </div>
          <Link className="secondary-button button-link" to={`/projects/${project.id}/candidates`}>
            审核候选建议
          </Link>
        </div>
      </section>

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
              <h2>项目身份</h2>
              <p>项目的稳定名称与边界标识。</p>
            </div>
          </div>
          <div className="form-grid">
            <label>
              名称 <span className="required">*</span>
              <input
                required
                value={draft.name}
                onChange={(event) =>
                  setDraft({ ...draft, name: event.target.value })
                }
              />
            </label>
            <label>
              Slug 标识 <span className="required">*</span>
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
              描述
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
              <h2>产品上下文</h2>
              <p>说明项目为何存在、服务谁，以及明确的范围边界。</p>
            </div>
          </div>
          <div className="form-grid">
            <label className="span-2">
              产品愿景
              <textarea
                rows={3}
                value={draft.vision}
                onChange={(event) =>
                  setDraft({ ...draft, vision: event.target.value })
                }
              />
            </label>
            <label className="span-2">
              目标用户
              <textarea
                rows={2}
                value={draft.targetUser}
                onChange={(event) =>
                  setDraft({ ...draft, targetUser: event.target.value })
                }
              />
            </label>
            <label>
              范围
              <textarea
                rows={5}
                value={draft.scope}
                onChange={(event) =>
                  setDraft({ ...draft, scope: event.target.value })
                }
              />
            </label>
            <label>
              非目标
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
              <h2>工程与生命周期</h2>
              <p>维护技术栈、当前状态和研发生命周期阶段。</p>
            </div>
          </div>
          <div className="form-grid">
            <label className="span-2">
              技术栈
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
              状态
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
              生命周期
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
              优先级
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
                重新加载最新版本
              </button>
            ) : null}
          </div>
        ) : null}
        {notice ? <div className="success-banner">{notice}</div> : null}

        <div className="sticky-actions">
          <span>当前修改基于版本 {draft.version}，保存时会进行冲突保护。</span>
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
              重置
            </button>
            <button
              className="primary-button"
              disabled={saving || isArchived}
              type="submit"
            >
              {saving ? "正在保存…" : "保存修改"}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
