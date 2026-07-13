import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { aiCandidateApi } from "../api/ai-candidate";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { AiCandidate, AiCandidateStatus, AiCandidateType } from "../types/ai-candidate";
import type { Project } from "../types/project";
import { formatUpdatedAt } from "../utils/format";

const TYPE_LABELS: Record<AiCandidateType, string> = {
  snapshot_update: "快照更新建议",
  session_summary: "会话总结",
  evidence_suggestion: "证据建议",
  risk_finding: "风险发现",
  general_note: "一般记录",
};

const STATUS_LABELS: Record<AiCandidateStatus, string> = {
  pending: "待审核",
  accepted: "已接受",
  rejected: "已拒绝",
  archived: "已归档",
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
      setError("项目 ID 缺失。");
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
      ? "接受操作只记录你认可这条建议，不会修改快照、会话或证据。确认继续？"
      : action === "reject"
        ? "拒绝操作会保留完整历史，不会删除候选建议。确认继续？"
        : "归档操作会保留候选建议，但将其移出默认待审核流程。确认继续？";
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

  if (loading) return <p className="loading-copy">正在加载 AI 候选建议…</p>;
  if (!project || !projectId) {
    return <section className="panel not-found"><h1>项目不可用</h1><p>{error}</p></section>;
  }

  return (
    <div className="page-stack candidate-page">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>AI 候选建议</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">D1.6 · AI 治理</p>
          <h1>AI 候选建议</h1>
          <p>审核 AI 建议。候选建议不是事实，接受后也不会自动修改正式领域。</p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${project.id}/context`}>从 AI 响应生成</Link>
      </header>

      <div className="ai-boundary-banner">
        <strong>候选层边界</strong>
        <span>所有状态操作只写候选建议记录；项目、快照、会话和证据保持不变。</span>
      </div>
      {error ? <div className="error-banner candidate-message">{error}</div> : null}
      {notice ? <div className="success-banner candidate-message">{notice}</div> : null}

      <div className="candidate-filters" role="group" aria-label="候选建议状态筛选">
        {(["all", "pending", "accepted", "rejected", "archived"] as const).map((value) => (
          <button className={filter === value ? "selected" : ""} key={value} onClick={() => setFilter(value)} type="button">
            {value === "all" ? "全部" : STATUS_LABELS[value]}
            <span>{value === "all" ? candidates.length : counts[value]}</span>
          </button>
        ))}
      </div>

      <div className="candidate-workbench">
        <section className="panel candidate-list-panel">
          <div className="panel-header"><div><h2>{filter === "all" ? "全部候选建议" : STATUS_LABELS[filter]}</h2><p>{visible.length} 条记录</p></div></div>
          {visible.length === 0 ? <p className="empty-state">该状态下没有候选建议。</p> : (
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
                <p>{TYPE_LABELS[selected.candidateType]} · 创建于 {formatUpdatedAt(selected.createdAt)}</p>
              </div>
              <Link to={`/projects/${project.id}/context`}>来源上下文包</Link>
            </div>
            <div className="candidate-source-id">来源 · {selected.sourceContextExportId}</div>
            <pre className="candidate-content">{selected.contentMarkdown}</pre>
            <div className="candidate-review-actions">
              {selected.status === "pending" ? (
                <>
                  <span>人工审核只改变 Candidate 状态。</span>
                  <div>
                    <button className="primary-button" disabled={reviewing} onClick={() => void review("accept")} type="button">接受</button>
                    <button className="danger-button" disabled={reviewing} onClick={() => void review("reject")} type="button">拒绝</button>
                    <button className="secondary-button" disabled={reviewing} onClick={() => void review("archive")} type="button">归档</button>
                  </div>
                </>
              ) : <span>该候选建议已进入终态，内容与审核结果保持只读。</span>}
            </div>
          </section>
        ) : <section className="panel provider-detail-empty"><strong>选择一条候选建议</strong><p>从左侧选择候选建议进行审核。</p></section>}
      </div>
    </div>
  );
}
