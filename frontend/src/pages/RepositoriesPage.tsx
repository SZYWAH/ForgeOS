import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { repositoryApi } from "../api/repository";
import type { Evidence } from "../types/evidence";
import type { Project } from "../types/project";
import type { GitInspection, RepositoryRecord } from "../types/repository";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";

export function RepositoriesPage() {
  const { t } = useTranslation();
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
      setError(t("repository.projectMissing"));
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
      setNotice(t("repository.registered"));
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
      setNotice(t("repository.refreshed"));
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
      setNotice(t("repository.commitMaterialCreated"));
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
      setNotice(t("repository.statusMaterialCreated"));
    } catch (createError) {
      setError(getErrorMessage(createError));
    } finally {
      setWorking(false);
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>{t("repository.title")}</h1><p>{error ?? t("repository.projectMissing")}</p></section>;
  }
  const archived = project.status === "archived";

  return (
    <div className="page-stack repositories-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>{t("repository.title")}</span>
      </div>
      <PageIntro title={t("repository.title")} technicalTerm={t("terms.repository")} description={t("repository.description")} />
      <div className="git-readonly-banner"><strong>{t("repository.readOnlyTitle")}</strong><span>{t("repository.readOnlyDescription")}</span></div>
      {error ? <div className="error-banner repository-message">{error}</div> : null}
      {notice ? <div className="success-banner repository-message">{notice}</div> : null}

      {!archived ? (
        <form className="panel repository-register-panel" onSubmit={register}>
          <div className="panel-header"><div><h2>{t("repository.connect")}</h2><p>{t("repository.emptyDescription")}</p></div></div>
          <div className="repository-register-controls">
            <label>{t("repository.absolutePath")} <span className="required">*</span>
              <input required value={path} onChange={(event) => { setPath(event.target.value); setPreview(undefined); }} placeholder="D:\projects\CodeLens-Pro" />
            </label>
            <button className="secondary-button" disabled={working || !path.trim()} onClick={() => void inspectPath()} type="button">{t("repository.inspect")}</button>
            <button className="primary-button" disabled={working || !preview} type="submit">{t("repository.connect")}</button>
          </div>
          {preview ? <InspectionPreview inspection={preview} /> : null}
        </form>
      ) : <div className="archive-banner">{t("repository.archivedNotice")}</div>}

      <div className="repository-workbench">
        <section className="panel repository-list-panel">
          <div className="panel-header"><div><h2>{t("repository.title")}</h2><p>{t("repository.repositoryCount", { count: records.length })}</p></div></div>
          {records.length === 0 ? <p className="empty-state">{t("repository.emptyDescription")}</p> : (
            <div className="repository-list record-list-pane">
              {records.map((item) => <button className={item.repository.id === selectedId ? "selected" : ""} key={item.repository.id} onClick={() => { setSelectedId(item.repository.id); setError(undefined); setNotice(undefined); }} type="button"><strong>{tail(item.repository.path)}</strong><small>{item.workingCopy.branch} · {item.workingCopy.headCommit.slice(0, 12)}</small><span className={item.workingCopy.isDirty ? "git-dirty" : "git-clean"}>{item.workingCopy.isDirty ? t("repository.changed") : t("repository.clean")}</span></button>)}
            </div>
          )}
        </section>

        {selected ? (
          <div className="repository-detail-stack">
            <section className="panel repository-detail-panel">
              <div className="panel-header repository-detail-header"><div><h2>{tail(selected.repository.path)}</h2><CompactIdentifier copyable label={t("repository.location")} value={selected.repository.path} /></div><button className="secondary-button" disabled={working || archived} onClick={() => void refresh()} type="button">{t("repository.refresh")}</button></div>
              <WorkingCopyView record={selected} />
              <div className="repository-evidence-actions"><span>{t("repository.createMaterialDescription")}</span><button className="secondary-button" disabled={working || archived} onClick={() => void createStatusEvidence()} type="button">{t("repository.createStatusMaterial")}</button></div>
            </section>

            <section className="panel repository-commits-panel">
              <div className="panel-header"><div><h2>{t("repository.recentCommits")}</h2><p>{selected.recentCommits.length > 0 ? "10" : t("repository.refresh")}</p></div></div>
              {selected.recentCommits.length === 0 ? <p className="empty-state">{t("repository.noCommits")}</p> : <div className="repository-commit-list">{selected.recentCommits.map((commit) => <article key={commit.sha}><div><code>{commit.sha.slice(0, 12)}</code><strong>{commit.message}</strong><small>{commit.authorName} · {formatUpdatedAt(commit.authoredAt)}</small></div><button className="secondary-button" disabled={working || archived} onClick={() => void createCommitEvidence(commit.sha)} type="button">{t("repository.createCommitMaterial")}</button></article>)}</div>}
            </section>

            <section className="panel repository-evidence-panel">
              <div className="panel-header"><div><h2>{t("repository.gitActivity")}</h2><p>{t("repository.activityCount", { count: evidences.length })}</p></div><Link to={`/projects/${project.id}/evidence`}>{t("evidence.title")}</Link></div>
              {evidences.length === 0 ? <p className="empty-state">{t("repository.noActivity")}</p> : <div className="repository-evidence-list">{evidences.map((evidence) => <article key={evidence.id}><span>{displayLabel(evidence.evidenceType)}</span><strong>{evidence.title}</strong><small>{displayLabel(evidence.verificationStatus)} · {formatUpdatedAt(evidence.createdAt)}</small></article>)}</div>}
            </section>
          </div>
        ) : <section className="panel provider-detail-empty"><strong>{t("repository.chooseTitle")}</strong><p>{t("repository.chooseDescription")}</p></section>}
      </div>
    </div>
  );
}

function InspectionPreview({ inspection }: { inspection: GitInspection }) {
  const { t } = useTranslation();
  return <div className="repository-inspection-preview"><div><CompactIdentifier copyable label={t("repository.rootPath")} value={inspection.rootPath} /></div><div><span>{t("repository.branch")}</span><strong>{inspection.branch}</strong></div><div><CompactIdentifier copyable label={t("repository.currentCommit")} value={inspection.headCommit} /></div><div><span>{t("repository.state")}</span><strong className={inspection.isDirty ? "git-dirty" : "git-clean"}>{inspection.isDirty ? t("repository.modified") : t("repository.clean")} · {t("repository.ahead")} {inspection.aheadCount ?? t("repository.unknown")} · {t("repository.behind")} {inspection.behindCount ?? t("repository.unknown")}</strong></div></div>;
}

function WorkingCopyView({ record }: { record: RepositoryRecord }) {
  const { t } = useTranslation();
  const { repository, workingCopy } = record;
  const summary = workingCopy.diffSummary;
  return <div className="working-copy-view"><dl><div><dt>{t("repository.branch")}</dt><dd>{workingCopy.branch}</dd></div><div><dt>{t("repository.currentCommit")}</dt><dd><CompactIdentifier copyable value={workingCopy.headCommit} /></dd></div><div><dt>{t("repository.workingCopy")}</dt><dd className={workingCopy.isDirty ? "git-dirty" : "git-clean"}>{workingCopy.isDirty ? t("repository.dirty") : t("repository.clean")}</dd></div><div><dt>{t("repository.upstream")}</dt><dd>{workingCopy.upstreamBranch ?? t("repository.unknown")}</dd></div><div><dt>{t("repository.ahead")} / {t("repository.behind")}</dt><dd>{workingCopy.aheadCount ?? t("repository.unknown")} / {workingCopy.behindCount ?? t("repository.unknown")}</dd></div><div><dt>{t("common.updatedAt")}</dt><dd>{formatUpdatedAt(workingCopy.checkedAt)}</dd></div></dl><div className="diff-summary-grid"><div><span>{t("repository.changedFiles")}</span><strong>{summary.changedFiles}</strong></div><div><span>{t("repository.stagedFiles")}</span><strong>{summary.stagedFiles}</strong></div><div><span>{t("repository.unstagedFiles")}</span><strong>{summary.unstagedFiles}</strong></div><div><span>{t("repository.untrackedFiles")}</span><strong>{summary.untrackedFiles}</strong></div><div><span>{t("repository.insertions")}</span><strong>+{summary.insertions}</strong></div><div><span>{t("repository.deletions")}</span><strong>-{summary.deletions}</strong></div></div><p className="repository-upstream-note">{t("repository.localReferenceNote")}</p><TechnicalDetails label={t("common.technicalDetails")}><span>{repository.defaultBranch}</span><span>{repository.remoteUrl ?? t("common.none")}</span></TechnicalDetails></div>;
}

function tail(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;
}
