import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { repositoryApi } from "../api/repository";
import type { Evidence } from "../types/evidence";
import type { Project } from "../types/project";
import type { GitInspection, RepositoryRecord } from "../types/repository";
import { formatUpdatedAt } from "../utils/format";

export function RepositoriesPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [records, setRecords] = useState<RepositoryRecord[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [evidences, setEvidences] = useState<Evidence[]>([]);
  const [path, setPath] = useState("");
  const [preview, setPreview] = useState<GitInspection>();
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const generation = useRef(0);
  const evidenceGeneration = useRef(0);

  const selected = useMemo(
    () => records.find((item) => item.repository.id === selectedId),
    [records, selectedId],
  );

  const load = useCallback(async (preferId?: string) => {
    const requestGeneration = ++generation.current;
    if (!projectId) {
      setError("Project ID 缺失。");
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, repositoryValues] = await Promise.all([
        projectKernelApi.getProject(projectId),
        repositoryApi.list(projectId),
      ]);
      if (requestGeneration !== generation.current) return;
      setProject(projectValue);
      setRecords(repositoryValues);
      const nextId = preferId ?? selectedId;
      setSelectedId(
        repositoryValues.some((item) => item.repository.id === nextId)
          ? nextId
          : (repositoryValues[0]?.repository.id ?? ""),
      );
    } catch (loadError) {
      if (requestGeneration === generation.current) setError(getErrorMessage(loadError));
    } finally {
      if (requestGeneration === generation.current) setLoading(false);
    }
  }, [projectId, selectedId]);

  useEffect(() => {
    void load();
    return () => { generation.current += 1; };
    // Project scope controls loading; selection is local page state.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  useEffect(() => {
    const requestGeneration = ++evidenceGeneration.current;
    if (!projectId || !selectedId) {
      setEvidences([]);
      return;
    }
    void repositoryApi.listEvidence(projectId, selectedId)
      .then((items) => {
        if (requestGeneration === evidenceGeneration.current) setEvidences(items);
      })
      .catch((loadError) => {
        if (requestGeneration === evidenceGeneration.current) setError(getErrorMessage(loadError));
      });
    return () => { evidenceGeneration.current += 1; };
  }, [projectId, selectedId]);

  async function inspectPath() {
    if (!path.trim()) return;
    setWorking(true);
    setError(undefined);
    setNotice(undefined);
    setPreview(undefined);
    try {
      setPreview(await repositoryApi.inspect(path.trim()));
    } catch (inspectError) {
      setError(getErrorMessage(inspectError));
    } finally {
      setWorking(false);
    }
  }

  async function register(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !path.trim()) return;
    setWorking(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const created = await repositoryApi.register(projectId, path.trim());
      setRecords((current) => [created, ...current]);
      setSelectedId(created.repository.id);
      setPath("");
      setPreview(undefined);
      setNotice("Repository 已注册；ForgeOS 只保存只读观察结果。");
    } catch (registerError) {
      setError(getErrorMessage(registerError));
    } finally {
      setWorking(false);
    }
  }

  async function refresh() {
    if (!projectId || !selected) return;
    setWorking(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await repositoryApi.refresh(
        projectId,
        selected.repository.id,
        selected.repository.version,
      );
      setRecords((current) => current.map((item) => item.repository.id === updated.repository.id ? updated : item));
      setNotice("Git 状态已通过只读命令刷新，并保存新的 WorkingCopy 观察记录。");
    } catch (refreshError) {
      setError(getErrorMessage(refreshError));
      if (getErrorCode(refreshError) === "repository_version_conflict") {
        await load(selected.repository.id);
      }
    } finally {
      setWorking(false);
    }
  }

  async function createCommitEvidence(sha: string) {
    if (!projectId || !selected) return;
    setWorking(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const evidence = await repositoryApi.createCommitEvidence(projectId, selected.repository.id, sha);
      setEvidences((current) => [evidence, ...current]);
      setNotice("Commit 已转换为未验证 Git Evidence；没有执行任何 Git 写操作。");
    } catch (createError) {
      setError(getErrorMessage(createError));
    } finally {
      setWorking(false);
    }
  }

  async function createStatusEvidence() {
    if (!projectId || !selected) return;
    setWorking(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const evidence = await repositoryApi.createStatusEvidence(projectId, selected.repository.id);
      setEvidences((current) => [evidence, ...current]);
      setNotice("当前 WorkingCopy 观察已保存为未验证 Git Status Evidence。");
    } catch (createError) {
      setError(getErrorMessage(createError));
    } finally {
      setWorking(false);
    }
  }

  if (loading) return <p className="loading-copy">Loading Repositories…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>Project unavailable</h1><p>{error}</p></section>;
  }
  const archived = project.status === "archived";

  return (
    <div className="page-stack repositories-page">
      <div className="breadcrumb">
        <Link to="/projects">Projects</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>Repositories</span>
      </div>
      <header className="page-header">
        <div><p className="eyebrow">D2 · Read-only Git</p><h1>Repositories</h1><p>观察本地 Git 工作区，并把明确选择的外部事实转换为 Evidence。</p></div>
      </header>
      <div className="git-readonly-banner"><strong>Observer only</strong><span>ForgeOS 不提供 commit、push、pull、merge、checkout、reset 或 stash。</span></div>
      {error ? <div className="error-banner repository-message">{error}</div> : null}
      {notice ? <div className="success-banner repository-message">{notice}</div> : null}

      {!archived ? (
        <form className="panel repository-register-panel" onSubmit={register}>
          <div className="panel-header"><div><h2>Register Local Repository</h2><p>先检查路径，再由你明确注册。</p></div></div>
          <div className="repository-register-controls">
            <label>Absolute repository path <span className="required">*</span>
              <input required value={path} onChange={(event) => { setPath(event.target.value); setPreview(undefined); }} placeholder="D:\projects\CodeLens-Pro" />
            </label>
            <button className="secondary-button" disabled={working || !path.trim()} onClick={() => void inspectPath()} type="button">Inspect</button>
            <button className="primary-button" disabled={working || !preview} type="submit">Register</button>
          </div>
          {preview ? <InspectionPreview inspection={preview} /> : null}
        </form>
      ) : <div className="archive-banner">归档 Project 的 Repository 集成保持只读，不能新增、刷新或生成 Evidence。</div>}

      <div className="repository-workbench">
        <section className="panel repository-list-panel">
          <div className="panel-header"><div><h2>Registered</h2><p>{records.length} repositories</p></div></div>
          {records.length === 0 ? <p className="empty-state">尚未注册 Repository。</p> : (
            <div className="repository-list">
              {records.map((item) => <button className={item.repository.id === selectedId ? "selected" : ""} key={item.repository.id} onClick={() => { setSelectedId(item.repository.id); setError(undefined); setNotice(undefined); }} type="button"><strong>{tail(item.repository.path)}</strong><small>{item.workingCopy.branch} · {item.workingCopy.headCommit.slice(0, 12)}</small><span className={item.workingCopy.isDirty ? "git-dirty" : "git-clean"}>{item.workingCopy.isDirty ? "Dirty" : "Clean"}</span></button>)}
            </div>
          )}
        </section>

        {selected ? (
          <div className="repository-detail-stack">
            <section className="panel repository-detail-panel">
              <div className="panel-header repository-detail-header"><div><h2>{tail(selected.repository.path)}</h2><p title={selected.repository.path}>{selected.repository.path}</p></div><button className="secondary-button" disabled={working || archived} onClick={() => void refresh()} type="button">Refresh Status</button></div>
              <WorkingCopyView record={selected} />
              <div className="repository-evidence-actions"><span>Evidence 创建是显式操作，不会改变 Git。</span><button className="secondary-button" disabled={working || archived} onClick={() => void createStatusEvidence()} type="button">Create Status Evidence</button></div>
            </section>

            <section className="panel repository-commits-panel">
              <div className="panel-header"><div><h2>Recent Commits</h2><p>{selected.recentCommits.length > 0 ? "Latest 10 from explicit inspection/refresh" : "Click Refresh Status to read commit log"}</p></div></div>
              {selected.recentCommits.length === 0 ? <p className="empty-state">当前页面尚未读取 Commit Log。</p> : <div className="repository-commit-list">{selected.recentCommits.map((commit) => <article key={commit.sha}><div><code>{commit.sha.slice(0, 12)}</code><strong>{commit.message}</strong><small>{commit.authorName} · {formatUpdatedAt(commit.authoredAt)}</small></div><button className="secondary-button" disabled={working || archived} onClick={() => void createCommitEvidence(commit.sha)} type="button">Create Evidence</button></article>)}</div>}
            </section>

            <section className="panel repository-evidence-panel">
              <div className="panel-header"><div><h2>Git Evidence</h2><p>{evidences.length} records linked to this Repository</p></div><Link to={`/projects/${project.id}/evidence`}>Open all Evidence</Link></div>
              {evidences.length === 0 ? <p className="empty-state">尚未创建 Git Evidence。</p> : <div className="repository-evidence-list">{evidences.map((evidence) => <article key={evidence.id}><span>{evidence.evidenceType === "git_commit" ? "Commit" : "Status"}</span><strong>{evidence.title}</strong><small>{evidence.verificationStatus} · {formatUpdatedAt(evidence.createdAt)}</small></article>)}</div>}
            </section>
          </div>
        ) : <section className="panel provider-detail-empty"><strong>Select a Repository</strong><p>注册或选择本地 Git 工作区。</p></section>}
      </div>
    </div>
  );
}

function InspectionPreview({ inspection }: { inspection: GitInspection }) {
  return <div className="repository-inspection-preview"><div><span>Root</span><strong>{inspection.rootPath}</strong></div><div><span>Branch</span><strong>{inspection.branch}</strong></div><div><span>HEAD</span><strong>{inspection.headCommit.slice(0, 12)}</strong></div><div><span>Status</span><strong className={inspection.isDirty ? "git-dirty" : "git-clean"}>{inspection.isDirty ? "Dirty" : "Clean"}</strong></div></div>;
}

function WorkingCopyView({ record }: { record: RepositoryRecord }) {
  const { repository, workingCopy } = record;
  const summary = workingCopy.diffSummary;
  return <div className="working-copy-view"><dl><div><dt>Branch</dt><dd>{workingCopy.branch}</dd></div><div><dt>HEAD</dt><dd>{workingCopy.headCommit}</dd></div><div><dt>Dirty</dt><dd className={workingCopy.isDirty ? "git-dirty" : "git-clean"}>{workingCopy.isDirty ? "Yes" : "No"}</dd></div><div><dt>Default Branch</dt><dd>{repository.defaultBranch}</dd></div><div><dt>Remote</dt><dd>{repository.remoteUrl ?? "Not configured"}</dd></div><div><dt>Last Checked</dt><dd>{formatUpdatedAt(workingCopy.checkedAt)}</dd></div></dl><div className="diff-summary-grid"><div><span>Changed</span><strong>{summary.changedFiles}</strong></div><div><span>Staged</span><strong>{summary.stagedFiles}</strong></div><div><span>Unstaged</span><strong>{summary.unstagedFiles}</strong></div><div><span>Untracked</span><strong>{summary.untrackedFiles}</strong></div><div><span>Insertions</span><strong>+{summary.insertions}</strong></div><div><span>Deletions</span><strong>-{summary.deletions}</strong></div></div></div>;
}

function tail(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}
