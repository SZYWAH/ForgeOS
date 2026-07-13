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

  if (loading) return <p className="loading-copy">正在加载本地仓库…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>项目不可用</h1><p>{error}</p></section>;
  }
  const archived = project.status === "archived";

  return (
    <div className="page-stack repositories-page">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>本地仓库</span>
      </div>
      <header className="page-header">
        <div><p className="eyebrow">D2 · 只读 Git</p><h1>本地仓库</h1><p>观察本地 Git 工作区，并把明确选择的外部事实转换为研发证据。</p></div>
      </header>
      <div className="git-readonly-banner"><strong>仅观察</strong><span>ForgeOS 不提供 commit、push、pull、merge、checkout、reset 或 stash。</span></div>
      {error ? <div className="error-banner repository-message">{error}</div> : null}
      {notice ? <div className="success-banner repository-message">{notice}</div> : null}

      {!archived ? (
        <form className="panel repository-register-panel" onSubmit={register}>
          <div className="panel-header"><div><h2>注册本地仓库</h2><p>先检查路径，再由你明确注册。</p></div></div>
          <div className="repository-register-controls">
            <label>仓库绝对路径 <span className="required">*</span>
              <input required value={path} onChange={(event) => { setPath(event.target.value); setPreview(undefined); }} placeholder="D:\projects\CodeLens-Pro" />
            </label>
            <button className="secondary-button" disabled={working || !path.trim()} onClick={() => void inspectPath()} type="button">检查</button>
            <button className="primary-button" disabled={working || !preview} type="submit">注册</button>
          </div>
          {preview ? <InspectionPreview inspection={preview} /> : null}
        </form>
      ) : <div className="archive-banner">归档项目的仓库集成保持只读，不能新增、刷新或生成证据。</div>}

      <div className="repository-workbench">
        <section className="panel repository-list-panel">
          <div className="panel-header"><div><h2>已注册仓库</h2><p>{records.length} 个仓库</p></div></div>
          {records.length === 0 ? <p className="empty-state">尚未注册本地仓库。</p> : (
            <div className="repository-list">
              {records.map((item) => <button className={item.repository.id === selectedId ? "selected" : ""} key={item.repository.id} onClick={() => { setSelectedId(item.repository.id); setError(undefined); setNotice(undefined); }} type="button"><strong>{tail(item.repository.path)}</strong><small>{item.workingCopy.branch} · {item.workingCopy.headCommit.slice(0, 12)}</small><span className={item.workingCopy.isDirty ? "git-dirty" : "git-clean"}>{item.workingCopy.isDirty ? "有改动" : "干净"}</span></button>)}
            </div>
          )}
        </section>

        {selected ? (
          <div className="repository-detail-stack">
            <section className="panel repository-detail-panel">
              <div className="panel-header repository-detail-header"><div><h2>{tail(selected.repository.path)}</h2><p title={selected.repository.path}>{selected.repository.path}</p></div><button className="secondary-button" disabled={working || archived} onClick={() => void refresh()} type="button">刷新状态</button></div>
              <WorkingCopyView record={selected} />
              <div className="repository-evidence-actions"><span>证据创建是显式操作，不会改变 Git。</span><button className="secondary-button" disabled={working || archived} onClick={() => void createStatusEvidence()} type="button">创建状态证据</button></div>
            </section>

            <section className="panel repository-commits-panel">
              <div className="panel-header"><div><h2>最近提交</h2><p>{selected.recentCommits.length > 0 ? "显式检查或刷新读取的最近 10 条" : "点击“刷新状态”读取提交记录"}</p></div></div>
              {selected.recentCommits.length === 0 ? <p className="empty-state">当前页面尚未读取提交记录。</p> : <div className="repository-commit-list">{selected.recentCommits.map((commit) => <article key={commit.sha}><div><code>{commit.sha.slice(0, 12)}</code><strong>{commit.message}</strong><small>{commit.authorName} · {formatUpdatedAt(commit.authoredAt)}</small></div><button className="secondary-button" disabled={working || archived} onClick={() => void createCommitEvidence(commit.sha)} type="button">创建证据</button></article>)}</div>}
            </section>

            <section className="panel repository-evidence-panel">
              <div className="panel-header"><div><h2>Git 证据</h2><p>{evidences.length} 条记录关联到该仓库</p></div><Link to={`/projects/${project.id}/evidence`}>打开全部证据</Link></div>
              {evidences.length === 0 ? <p className="empty-state">尚未创建 Git 证据。</p> : <div className="repository-evidence-list">{evidences.map((evidence) => <article key={evidence.id}><span>{evidence.evidenceType === "git_commit" ? "提交" : "状态"}</span><strong>{evidence.title}</strong><small>{evidence.verificationStatus === "verified" ? "已验证" : evidence.verificationStatus === "invalid" ? "已失效" : "未验证"} · {formatUpdatedAt(evidence.createdAt)}</small></article>)}</div>}
            </section>
          </div>
        ) : <section className="panel provider-detail-empty"><strong>选择一个仓库</strong><p>注册或选择本地 Git 工作区。</p></section>}
      </div>
    </div>
  );
}

function InspectionPreview({ inspection }: { inspection: GitInspection }) {
  return <div className="repository-inspection-preview"><div><span>根路径</span><strong>{inspection.rootPath}</strong></div><div><span>分支</span><strong>{inspection.branch}</strong></div><div><span>HEAD</span><strong>{inspection.headCommit.slice(0, 12)}</strong></div><div><span>状态</span><strong className={inspection.isDirty ? "git-dirty" : "git-clean"}>{inspection.isDirty ? "有改动" : "干净"}</strong></div></div>;
}

function WorkingCopyView({ record }: { record: RepositoryRecord }) {
  const { repository, workingCopy } = record;
  const summary = workingCopy.diffSummary;
  return <div className="working-copy-view"><dl><div><dt>分支</dt><dd>{workingCopy.branch}</dd></div><div><dt>HEAD</dt><dd>{workingCopy.headCommit}</dd></div><div><dt>工作区改动</dt><dd className={workingCopy.isDirty ? "git-dirty" : "git-clean"}>{workingCopy.isDirty ? "有" : "无"}</dd></div><div><dt>默认分支</dt><dd>{repository.defaultBranch}</dd></div><div><dt>远程地址</dt><dd>{repository.remoteUrl ?? "未配置"}</dd></div><div><dt>最后检查</dt><dd>{formatUpdatedAt(workingCopy.checkedAt)}</dd></div></dl><div className="diff-summary-grid"><div><span>已变更</span><strong>{summary.changedFiles}</strong></div><div><span>已暂存</span><strong>{summary.stagedFiles}</strong></div><div><span>未暂存</span><strong>{summary.unstagedFiles}</strong></div><div><span>未跟踪</span><strong>{summary.untrackedFiles}</strong></div><div><span>新增行</span><strong>+{summary.insertions}</strong></div><div><span>删除行</span><strong>-{summary.deletions}</strong></div></div></div>;
}

function tail(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}
