import { type FormEvent, useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { githubAnalysisImportApi, githubApi } from "../api/github";
import { projectKernelApi } from "../api/project-kernel";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";
import type {
  GitHubAnalysisImportProjectFields,
  GitHubAnalysisImportView,
} from "../types/github";
import { LIFECYCLE_STAGES, PROJECT_STATUSES } from "../types/project";
import { localizeAppError } from "../utils/appError";
import { displayLabel, formatUpdatedAt } from "../utils/format";

function fieldsFromView(view: GitHubAnalysisImportView): GitHubAnalysisImportProjectFields | undefined {
  const proposal = view.proposal;
  if (!proposal) return undefined;
  return {
    name: proposal.projectName,
    slug: proposal.projectSlug,
    description: proposal.description,
    vision: proposal.vision,
    targetUser: proposal.targetUser,
    scope: proposal.scope,
    nonGoals: proposal.nonGoals,
    techStack: proposal.techStack,
    status: proposal.projectStatus,
    lifecycleStage: proposal.lifecycleStage,
    priority: proposal.priority,
  };
}

export function GitHubAnalysisImportPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { analysisId } = useParams();
  const [workspaceId, setWorkspaceId] = useState("");
  const [view, setView] = useState<GitHubAnalysisImportView>();
  const [project, setProject] = useState<GitHubAnalysisImportProjectFields>();
  const [partialAcknowledged, setPartialAcknowledged] = useState(false);
  const [staleAcknowledged, setStaleAcknowledged] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();

  const errorText = (reason: unknown) => {
    const localized = localizeAppError(reason, t);
    return localized.guidance ? `${localized.title}：${localized.guidance}` : localized.title;
  };

  useEffect(() => {
    void load();
  }, [analysisId]);

  async function load() {
    if (!analysisId) return;
    setError(undefined);
    try {
      const workspace = await projectKernelApi.getCurrentWorkspace();
      if (!workspace) {
        setError(t("githubAnalysis.workspaceRequired"));
        return;
      }
      setWorkspaceId(workspace.id);
      applyView(await githubAnalysisImportApi.prepare(workspace.id, analysisId));
    } catch (reason) {
      setError(errorText(reason));
    }
  }

  function applyView(next: GitHubAnalysisImportView) {
    setView(next);
    setProject(fieldsFromView(next));
    setPartialAcknowledged(next.proposal?.partialAcknowledged ?? false);
    setStaleAcknowledged(next.proposal?.staleAcknowledged ?? false);
    setDirty(false);
  }

  function updateField<K extends keyof GitHubAnalysisImportProjectFields>(
    key: K,
    value: GitHubAnalysisImportProjectFields[K],
  ) {
    setProject((current) => current ? { ...current, [key]: value } : current);
    setDirty(true);
  }

  async function run(operation: () => Promise<void>) {
    setBusy(true);
    setError(undefined);
    setNotice(undefined);
    try {
      await operation();
    } catch (reason) {
      setError(errorText(reason));
    } finally {
      setBusy(false);
    }
  }

  async function save(event?: FormEvent) {
    event?.preventDefault();
    if (!view?.draft || !view.proposal || !project) return;
    await run(async () => {
      const next = await githubAnalysisImportApi.update({
        workspaceId,
        draftId: view.draft!.id,
        expectedDraftVersion: view.draft!.version,
        expectedProposalVersion: view.proposal!.version,
        project,
        partialAcknowledged,
        staleAcknowledged,
      });
      applyView(next);
      setNotice(t("githubAnalysisImport.saved"));
    });
  }

  async function review() {
    if (!view?.draft || !view.proposal || dirty) return;
    await run(async () => {
      const next = await githubAnalysisImportApi.review({
        workspaceId,
        draftId: view.draft!.id,
        expectedDraftVersion: view.draft!.version,
        expectedProposalVersion: view.proposal!.version,
        partialAcknowledged,
        staleAcknowledged,
      });
      applyView(next);
      setNotice(t("githubAnalysisImport.reviewed"));
    });
  }

  async function refresh() {
    if (!analysisId || !view?.draft || !view.proposal) return;
    if (!window.confirm(t("githubAnalysisImport.replaceConfirm"))) return;
    await run(async () => {
      applyView(await githubAnalysisImportApi.refresh(
        workspaceId,
        analysisId,
        view.draft!.version,
        view.proposal!.version,
      ));
      setNotice(t("githubAnalysisImport.regenerated"));
    });
  }

  async function createProject() {
    if (!view?.draft || !view.proposal) return;
    await run(async () => {
      const result = await githubApi.importProject(
        view.draft!.id,
        view.draft!.version,
        workspaceId,
        view.proposal!.version,
      );
      navigate(`/projects/${result.project.id}`, { replace: true });
    });
  }

  const isPartial = view?.analysis.status === "partial";
  const draftStatus = view?.draft?.status;
  const canReview = !dirty
    && draftStatus === "draft"
    && (!isPartial || partialAcknowledged)
    && (!view?.stale || staleAcknowledged);
  const primaryAction = useMemo(() => {
    if (view?.existingProjectId) {
      return <button className="primary-button" type="button" onClick={() => navigate(`/projects/${view.existingProjectId}`)}>{t("githubAnalysisImport.openProject")}</button>;
    }
    if (draftStatus === "reviewed" && !dirty) {
      return <button className="primary-button" disabled={busy} type="button" onClick={() => void createProject()}>{t("githubAnalysisImport.createProject")}</button>;
    }
    if (draftStatus === "rejected") {
      return <button className="primary-button" disabled={busy} type="button" onClick={() => void refresh()}>{t("githubAnalysisImport.reopenDraft")}</button>;
    }
    return <button className="primary-button" disabled={busy || !dirty || !project} form="github-analysis-import-form" type="submit">{t("githubAnalysisImport.saveDraft")}</button>;
  }, [busy, dirty, draftStatus, project, view?.existingProjectId]);

  if (view?.existingProjectId) {
    return <div className="page-stack"><PageIntro title={t("githubAnalysisImport.title")} description={t("githubAnalysisImport.alreadyImported")} primaryAction={<button className="primary-button" type="button" onClick={() => navigate(`/projects/${view.existingProjectId}`)}>{t("githubAnalysisImport.openProject")}</button>} secondaryActions={<button className="secondary-button" type="button" onClick={() => navigate(`/github-import/analyses/${analysisId}`)}>{t("githubAnalysisImport.backToReport")}</button>} /></div>;
  }

  if (!view || !project || !view.draft || !view.proposal) {
    return <div className="page-stack"><PageIntro title={t("githubAnalysisImport.title")} description={t("common.loading")} />{error ? <div className="error-banner">{error}</div> : null}</div>;
  }

  return (
    <div className="page-stack github-analysis-import-page">
      <PageIntro
        title={t("githubAnalysisImport.title")}
        description={t("githubAnalysisImport.description")}
        primaryAction={primaryAction}
        secondaryActions={<button className="secondary-button" type="button" onClick={() => navigate(`/github-import/analyses/${analysisId}`)}>{t("githubAnalysisImport.backToReport")}</button>}
      />

      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}
      {view.sourceChanged ? <div className="notice notice--warning"><strong>{t("githubAnalysisImport.sourceChanged")}</strong><p>{t("githubAnalysisImport.sourceChangedDescription")}</p><button className="secondary-button" disabled={busy} type="button" onClick={() => void refresh()}>{t("githubAnalysisImport.useCurrentReport")}</button></div> : null}
      {view.stale ? <div className="notice notice--warning"><strong>{t("githubAnalysisImport.staleTitle")}</strong><p>{t("githubAnalysisImport.staleDescription")}</p></div> : null}
      {isPartial ? <div className="notice notice--warning"><strong>{t("githubAnalysisImport.partialTitle")}</strong><p>{t("githubAnalysisImport.partialDescription")}</p></div> : null}

      <section className="surface github-analysis-import-source">
        <div>
          <span>{t("githubAnalysisImport.repository")}</span>
          <strong>{view.analysis.owner}/{view.analysis.name}</strong>
        </div>
        <div>
          <span>{t("githubAnalysisImport.branchAndHead")}</span>
          <strong>{view.analysis.defaultBranch} · {view.analysis.sourceCommitSha.slice(0, 12)}</strong>
        </div>
        <div>
          <span>{t("githubAnalysisImport.analysisMethod")}</span>
          <strong>{t(`githubAnalysis.method_${view.analysis.analysisMethod}`)}</strong>
        </div>
        <div>
          <span>{t("githubAnalysisImport.reportTime")}</span>
          <strong>{formatUpdatedAt(view.analysis.completedAt ?? view.analysis.createdAt)}</strong>
        </div>
      </section>

      <form id="github-analysis-import-form" className="surface github-analysis-import-form" onSubmit={(event) => void save(event)}>
        <div className="section-heading"><div><h2>{t("githubAnalysisImport.projectDraft")}</h2><p>{t("githubAnalysisImport.projectDraftDescription")}</p></div><span className="status-badge info">{displayLabel(draftStatus ?? "draft")}</span></div>
        <div className="form-grid">
          <label>{t("projects.name")}<input required maxLength={200} value={project.name} onChange={(event) => updateField("name", event.target.value)} /></label>
          <label>{t("projects.slug")}<input required maxLength={64} pattern="[a-z0-9]+(?:-[a-z0-9]+)*" value={project.slug} onChange={(event) => updateField("slug", event.target.value)} /></label>
          <label className="span-2">{t("projects.projectDescription")}<textarea maxLength={2000} rows={3} value={project.description} onChange={(event) => updateField("description", event.target.value)} /></label>
          <label className="span-2">{t("projects.vision")}<textarea maxLength={20000} rows={3} value={project.vision} onChange={(event) => updateField("vision", event.target.value)} /></label>
          <label className="span-2">{t("projects.targetUser")}<textarea maxLength={20000} rows={2} value={project.targetUser} onChange={(event) => updateField("targetUser", event.target.value)} /></label>
          <label className="span-2">{t("projects.scope")}<textarea maxLength={20000} rows={5} value={project.scope} onChange={(event) => updateField("scope", event.target.value)} /></label>
          <label className="span-2">{t("projects.nonGoals")}<textarea maxLength={20000} rows={3} value={project.nonGoals} onChange={(event) => updateField("nonGoals", event.target.value)} /></label>
          <label className="span-2">{t("projects.techStack")}<textarea maxLength={20000} rows={3} value={project.techStack} onChange={(event) => updateField("techStack", event.target.value)} /><small>{t("githubAnalysisImport.techStackNote")}</small></label>
          <label>{t("projects.projectStatus")}<select value={project.status} onChange={(event) => updateField("status", event.target.value as GitHubAnalysisImportProjectFields["status"])}>{PROJECT_STATUSES.map((status) => <option key={status} value={status}>{displayLabel(status)}</option>)}</select></label>
          <label>{t("projects.lifecycle")}<select value={project.lifecycleStage} onChange={(event) => updateField("lifecycleStage", event.target.value as GitHubAnalysisImportProjectFields["lifecycleStage"])}>{LIFECYCLE_STAGES.map((stage) => <option key={stage} value={stage}>{displayLabel(stage)}</option>)}</select></label>
          <label>{t("projects.priority")}<input min={0} max={100} type="number" value={project.priority} onChange={(event) => updateField("priority", Number(event.target.value))} /></label>
        </div>

        {(isPartial || view.stale) ? <fieldset className="github-analysis-import-acknowledgements"><legend>{t("githubAnalysisImport.acknowledgements")}</legend>
          {isPartial ? <label><input type="checkbox" checked={partialAcknowledged} onChange={(event) => { setPartialAcknowledged(event.target.checked); setDirty(true); }} />{t("githubAnalysisImport.ackPartial")}</label> : null}
          {view.stale ? <label><input type="checkbox" checked={staleAcknowledged} onChange={(event) => { setStaleAcknowledged(event.target.checked); setDirty(true); }} />{t("githubAnalysisImport.ackStale")}</label> : null}
        </fieldset> : null}
      </form>

      <section className="surface github-analysis-import-preview">
        <h2>{t("githubAnalysisImport.dataPreview")}</h2>
        <div className="github-analysis-import-preview__columns">
          <div><h3>{t("githubAnalysisImport.willCreate")}</h3><ul><li>{t("githubAnalysisImport.createProjectItem")}</li><li>{t("githubAnalysisImport.createRepositoryEvidence")}</li><li>{t("githubAnalysisImport.createReleaseEvidence")}</li></ul></div>
          <div><h3>{t("githubAnalysisImport.willNotCreate")}</h3><ul><li>{t("githubAnalysisImport.noSnapshot")}</li><li>{t("githubAnalysisImport.noSession")}</li><li>{t("githubAnalysisImport.noAchievement")}</li><li>{t("githubAnalysisImport.noLocalRepository")}</li><li>{t("githubAnalysisImport.noCommitEvidence")}</li></ul></div>
        </div>
      </section>

      <div className="github-analysis-import-actions">
        {draftStatus === "draft" ? <button className="secondary-button" disabled={busy || !canReview} type="button" onClick={() => void review()}>{dirty ? t("githubAnalysisImport.saveBeforeReview") : t("githubAnalysisImport.reviewContent")}</button> : null}
        {draftStatus === "reviewed" && dirty ? <span className="muted">{t("githubAnalysisImport.editReturnsToDraft")}</span> : null}
      </div>

      <TechnicalDetails label={t("common.technicalDetails")}>
        <dl><dt>Analysis Run</dt><dd>{view.analysis.analysisRunId}</dd><dt>Source HEAD</dt><dd>{view.proposal.sourceCommitSha}</dd><dt>Content Hash</dt><dd>{view.proposal.sourceContentHash}</dd></dl>
      </TechnicalDetails>
    </div>
  );
}
