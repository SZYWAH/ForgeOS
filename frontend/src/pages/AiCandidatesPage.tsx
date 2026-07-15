import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { aiCandidateApi } from "../api/ai-candidate";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { AiCandidate, AiCandidateStatus } from "../types/ai-candidate";
import type { Project } from "../types/project";
import { CompactIdentifier } from "../components/content/ContentGovernance";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

type Filter = "all" | AiCandidateStatus;
type ReviewAction = "accept" | "reject" | "archive";

export function AiCandidatesPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [candidates, setCandidates] = useState<AiCandidate[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [filter, setFilter] = useState<Filter>("pending");
  const [loading, setLoading] = useState(true);
  const [reviewing, setReviewing] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const generation = useRef(0);

  const visible = useMemo(
    () => candidates.filter((candidate) => filter === "all" || candidate.status === filter),
    [candidates, filter],
  );
  const selected = useMemo(
    () => candidates.find((candidate) => candidate.id === selectedId),
    [candidates, selectedId],
  );
  const counts = useMemo(() => ({
    pending: candidates.filter((item) => item.status === "pending").length,
    accepted: candidates.filter((item) => item.status === "accepted").length,
    rejected: candidates.filter((item) => item.status === "rejected").length,
    archived: candidates.filter((item) => item.status === "archived").length,
  }), [candidates]);

  const load = useCallback(async (preferId?: string) => {
    const requestGeneration = ++generation.current;
    if (!projectId) {
      setError(t("projects.noProject"));
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, candidateValues] = await Promise.all([
        projectKernelApi.getProject(projectId),
        aiCandidateApi.list(projectId),
      ]);
      if (requestGeneration !== generation.current) return;
      setProject(projectValue);
      setCandidates(candidateValues);
      const nextId = preferId ?? selectedId;
      setSelectedId(
        candidateValues.some((item) => item.id === nextId)
          ? nextId
          : (candidateValues.find((item) => item.status === "pending")?.id ?? candidateValues[0]?.id ?? ""),
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
    // Selection changes do not reload the Project-scoped collection.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  async function review(action: ReviewAction) {
    if (!projectId || !selected || selected.status !== "pending") return;
    const explanation = action === "accept"
      ? t("ai.acceptConfirm")
      : action === "reject"
        ? t("ai.rejectConfirm")
        : t("ai.archiveConfirm");
    if (!window.confirm(explanation)) return;
    setReviewing(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiCandidateApi[action](projectId, selected.id, selected.version);
      setCandidates((current) => current.map((item) => item.id === updated.id ? updated : item));
      setSelectedId(updated.id);
      setNotice(t("ai.statusRecorded", { status: displayLabel(updated.status) }));
    } catch (reviewError) {
      setError(getErrorMessage(reviewError));
      if (getErrorCode(reviewError) === "ai_candidate_version_conflict") {
        await load(selected.id);
      }
    } finally {
      setReviewing(false);
    }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>{t("ai.suggestionsTitle")}</h1><p>{error ?? t("projects.noProject")}</p></section>;
  }

  return (
    <div className="page-stack candidate-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>{t("ai.suggestionsTitle")}</span>
      </div>

      <PageIntro title={t("ai.suggestionsTitle")} technicalTerm={t("terms.candidate")} description={t("ai.suggestionsDescription")} secondaryActions={<Link className="secondary-button button-link" to={`/projects/${project.id}/context`}>{t("navigation.aiCollaboration")}</Link>} />

      <div className="ai-boundary-banner">
        <strong>{t("ai.candidateBoundary")}</strong>
        <span>{t("ai.candidateBoundaryDescription")}</span>
      </div>
      {error ? <div className="error-banner candidate-message">{error}</div> : null}
      {notice ? <div className="success-banner candidate-message">{notice}</div> : null}

      <div className="candidate-filters" role="group" aria-label={t("ai.suggestionsTitle")}>
        {(["all", "pending", "accepted", "rejected", "archived"] as const).map((value) => (
          <button className={filter === value ? "selected" : ""} key={value} onClick={() => setFilter(value)} type="button">
            {value === "all" ? t("common.all") : t(`ai.${value}`)}
            <span>{value === "all" ? candidates.length : counts[value]}</span>
          </button>
        ))}
      </div>

      <div className="candidate-workbench">
        <section className="panel candidate-list-panel">
          <div className="panel-header"><div><h2>{filter === "all" ? t("ai.allSuggestions") : displayLabel(filter)}</h2><p>{t("ai.recordCount", { count: visible.length })}</p></div></div>
          {visible.length === 0 ? <p className="empty-state">{t("ai.noSuggestions")}</p> : (
            <div className="candidate-list record-list-pane">
              {visible.map((candidate) => (
                <button className={candidate.id === selectedId ? "selected" : ""} key={candidate.id} onClick={() => { setSelectedId(candidate.id); setError(undefined); setNotice(undefined); }} type="button">
                  <span className={`candidate-status candidate-status--${candidate.status}`}>{t(`ai.${candidate.status}`)}</span>
                  <strong>{candidate.title}</strong>
                  <small>{displayLabel(candidate.candidateType)} · v{candidate.version}</small>
                </button>
              ))}
            </div>
          )}
        </section>

        {selected ? (
          <section className="panel candidate-detail-panel">
            <div className="panel-header candidate-detail-header">
              <div>
                <span className={`candidate-status candidate-status--${selected.status}`}>{t(`ai.${selected.status}`)}</span>
                <h2>{selected.title}</h2>
                <p>{displayLabel(selected.candidateType)} · {t("common.createdAt")} {formatUpdatedAt(selected.createdAt)}</p>
              </div>
              <Link to={`/projects/${project.id}/context`}>{t("ai.sourceMaterial")}</Link>
            </div>
            <CompactIdentifier label={t("ai.sourceMaterial")} value={selected.sourceContextExportId} />
            <pre className="candidate-content document-scroll-region">{selected.contentMarkdown}</pre>
            <div className="candidate-review-actions">
              {selected.status === "pending" ? (
                <>
                  <span>{t("ai.manualReviewOnly")}</span>
                  <div>
                    <button className="primary-button" disabled={reviewing} onClick={() => void review("accept")} type="button">{t("ai.accept")}</button>
                    <button className="danger-button" disabled={reviewing} onClick={() => void review("reject")} type="button">{t("ai.reject")}</button>
                    <button className="secondary-button" disabled={reviewing} onClick={() => void review("archive")} type="button">{t("ai.archive")}</button>
                  </div>
                </>
              ) : <span>{t("ai.terminalReadOnly")}</span>}
            </div>
          </section>
        ) : <section className="panel provider-detail-empty"><strong>{t("ai.selectSuggestion")}</strong><p>{t("ai.selectSuggestionDescription")}</p></section>}
      </div>
    </div>
  );
}
