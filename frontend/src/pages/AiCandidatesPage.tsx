import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { aiCandidateApi } from "../api/ai-candidate";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { AiCandidate, AiCandidateStatus, AiCandidateType } from "../types/ai-candidate";
import type { Project } from "../types/project";
import { formatUpdatedAt } from "../utils/format";

const TYPE_LABELS: Record<AiCandidateType, string> = {
  snapshot_update: "Snapshot Update",
  session_summary: "Session Summary",
  evidence_suggestion: "Evidence Suggestion",
  risk_finding: "Risk Finding",
  general_note: "General Note",
};

const STATUS_LABELS: Record<AiCandidateStatus, string> = {
  pending: "Pending",
  accepted: "Accepted",
  rejected: "Rejected",
  archived: "Archived",
};

type Filter = "all" | AiCandidateStatus;
type ReviewAction = "accept" | "reject" | "archive";

export function AiCandidatesPage() {
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
      setError("Project ID 缺失。");
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
      ? "Accept 只记录你认可这条建议，不会修改 Snapshot、Session 或 Evidence。确认继续？"
      : action === "reject"
        ? "Reject 会保留完整历史，不会删除 Candidate。确认继续？"
        : "Archive 会保留 Candidate，但从默认 Pending 审核流移出。确认继续？";
    if (!window.confirm(explanation)) return;
    setReviewing(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await aiCandidateApi[action](projectId, selected.id, selected.version);
      setCandidates((current) => current.map((item) => item.id === updated.id ? updated : item));
      setSelectedId(updated.id);
      setNotice(`${STATUS_LABELS[updated.status]} 已记录；正式研发事实未发生变化。`);
    } catch (reviewError) {
      setError(getErrorMessage(reviewError));
      if (getErrorCode(reviewError) === "ai_candidate_version_conflict") {
        await load(selected.id);
      }
    } finally {
      setReviewing(false);
    }
  }

  if (loading) return <p className="loading-copy">Loading AI Candidates…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>Project unavailable</h1><p>{error}</p></section>;
  }

  return (
    <div className="page-stack candidate-page">
      <div className="breadcrumb">
        <Link to="/projects">Projects</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>AI Candidates</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">D1.6 · AI Governance</p>
          <h1>AI Candidates</h1>
          <p>审核 AI 建议。Candidate 不是事实，Accept 也不会自动修改正式领域。</p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${project.id}/context`}>Generate from AI Response</Link>
      </header>

      <div className="ai-boundary-banner">
        <strong>候选层边界</strong>
        <span>所有状态操作只写 ai_candidates；Project、Snapshot、Session、Evidence 保持不变。</span>
      </div>
      {error ? <div className="error-banner candidate-message">{error}</div> : null}
      {notice ? <div className="success-banner candidate-message">{notice}</div> : null}

      <div className="candidate-filters" role="group" aria-label="Candidate status filter">
        {(["all", "pending", "accepted", "rejected", "archived"] as const).map((value) => (
          <button className={filter === value ? "selected" : ""} key={value} onClick={() => setFilter(value)} type="button">
            {value === "all" ? "All" : STATUS_LABELS[value]}
            <span>{value === "all" ? candidates.length : counts[value]}</span>
          </button>
        ))}
      </div>

      <div className="candidate-workbench">
        <section className="panel candidate-list-panel">
          <div className="panel-header"><div><h2>{filter === "all" ? "All Candidates" : STATUS_LABELS[filter]}</h2><p>{visible.length} records</p></div></div>
          {visible.length === 0 ? <p className="empty-state">该状态下没有 Candidate。</p> : (
            <div className="candidate-list">
              {visible.map((candidate) => (
                <button className={candidate.id === selectedId ? "selected" : ""} key={candidate.id} onClick={() => { setSelectedId(candidate.id); setError(undefined); setNotice(undefined); }} type="button">
                  <span className={`candidate-status candidate-status--${candidate.status}`}>{STATUS_LABELS[candidate.status]}</span>
                  <strong>{candidate.title}</strong>
                  <small>{TYPE_LABELS[candidate.candidateType]} · v{candidate.version}</small>
                </button>
              ))}
            </div>
          )}
        </section>

        {selected ? (
          <section className="panel candidate-detail-panel">
            <div className="panel-header candidate-detail-header">
              <div>
                <span className={`candidate-status candidate-status--${selected.status}`}>{STATUS_LABELS[selected.status]}</span>
                <h2>{selected.title}</h2>
                <p>{TYPE_LABELS[selected.candidateType]} · Created {formatUpdatedAt(selected.createdAt)}</p>
              </div>
              <Link to={`/projects/${project.id}/context`}>Source Context Pack</Link>
            </div>
            <div className="candidate-source-id">Source · {selected.sourceContextExportId}</div>
            <pre className="candidate-content">{selected.contentMarkdown}</pre>
            <div className="candidate-review-actions">
              {selected.status === "pending" ? (
                <>
                  <span>人工审核只改变 Candidate 状态。</span>
                  <div>
                    <button className="primary-button" disabled={reviewing} onClick={() => void review("accept")} type="button">Accept</button>
                    <button className="danger-button" disabled={reviewing} onClick={() => void review("reject")} type="button">Reject</button>
                    <button className="secondary-button" disabled={reviewing} onClick={() => void review("archive")} type="button">Archive</button>
                  </div>
                </>
              ) : <span>该 Candidate 已进入终态，内容与审核结果保持只读。</span>}
            </div>
          </section>
        ) : <section className="panel provider-detail-empty"><strong>Select a Candidate</strong><p>从左侧选择候选建议进行审核。</p></section>}
      </div>
    </div>
  );
}
