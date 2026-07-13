import { type FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { codexExecutionApi } from "../api/codex-execution";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { CodexExecution, CodexExecutionStatus } from "../types/codex-execution";
import type { CodexTaskProposal } from "../types/codex-task-proposal";
import type { Project } from "../types/project";
import { formatUpdatedAt } from "../utils/format";

const STATUS_LABELS: Record<CodexExecutionStatus, string> = {
  pending: "等待开始",
  running: "运行中（仅记录）",
  success: "成功",
  failed: "失败",
  cancelled: "已取消",
};

export function CodexExecutionsPage() {
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [proposals, setProposals] = useState<CodexTaskProposal[]>([]);
  const [executions, setExecutions] = useState<CodexExecution[]>([]);
  const [proposalId, setProposalId] = useState("");
  const [selectedId, setSelectedId] = useState("");
  const [outputSummary, setOutputSummary] = useState("");
  const [validationResult, setValidationResult] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();

  const approvedProposals = useMemo(
    () => proposals.filter((proposal) => proposal.status === "approved"),
    [proposals],
  );
  const selected = useMemo(
    () => executions.find((execution) => execution.id === selectedId) ?? executions[0],
    [executions, selectedId],
  );
  const proposalMap = useMemo(
    () => new Map(proposals.map((proposal) => [proposal.id, proposal])),
    [proposals],
  );

  const load = useCallback(async () => {
    if (!projectId) return;
    setError(undefined);
    try {
      const [projectValue, proposalValues, executionValues] = await Promise.all([
        projectKernelApi.getProject(projectId),
        codexTaskProposalApi.list(projectId),
        codexExecutionApi.list(projectId),
      ]);
      setProject(projectValue);
      setProposals(proposalValues);
      setExecutions(executionValues);
      setProposalId((current) =>
        proposalValues.some((proposal) => proposal.id === current && proposal.status === "approved")
          ? current
          : (proposalValues.find((proposal) => proposal.status === "approved")?.id ?? ""),
      );
      setSelectedId((current) =>
        executionValues.some((execution) => execution.id === current)
          ? current
          : (executionValues[0]?.id ?? ""),
      );
    } catch (loadError) {
      setError(getErrorMessage(loadError));
    }
  }, [projectId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    setOutputSummary(selected?.outputSummary ?? "");
    setValidationResult(selected?.validationResult ?? "");
  }, [selected?.id, selected?.outputSummary, selected?.validationResult]);

  function replaceExecution(updated: CodexExecution) {
    setExecutions((items) =>
      items.some((item) => item.id === updated.id)
        ? items.map((item) => (item.id === updated.id ? updated : item))
        : [updated, ...items],
    );
    setSelectedId(updated.id);
  }

  async function run(operation: () => Promise<CodexExecution>, message: string) {
    setBusy(true);
    setError(undefined);
    setNotice(undefined);
    try {
      replaceExecution(await operation());
      setNotice(message);
    } catch (operationError) {
      const message = getErrorMessage(operationError);
      await load();
      setError(message);
    } finally {
      setBusy(false);
    }
  }

  async function createExecution(event: FormEvent) {
    event.preventDefault();
    if (!projectId || !proposalId) return;
    await run(
      () => codexExecutionApi.create(projectId, proposalId),
      "执行请求已记录为 pending；RecordOnlyCodexRunner 没有启动 Codex。",
    );
  }

  async function startExecution() {
    if (!projectId || !selected) return;
    if (!window.confirm("这只会记录 running 状态，不会启动 Codex 或执行命令。继续？")) return;
    await run(
      () => codexExecutionApi.start(projectId, selected.id, selected.version),
      "已记录为 running；当前仍未执行 Codex。",
    );
  }

  async function recordResult(status: "success" | "failed") {
    if (!projectId || !selected) return;
    if (!outputSummary.trim() || !validationResult.trim()) {
      setError("请填写输出摘要和验证结果，再记录终态。");
      return;
    }
    const input = {
      outputSummary,
      validationResult,
      expectedVersion: selected.version,
    };
    await run(
      () =>
        status === "success"
          ? codexExecutionApi.complete(projectId, selected.id, input)
          : codexExecutionApi.fail(projectId, selected.id, input),
      status === "success" ? "成功结果已保存。" : "失败结果已保存并保留历史。",
    );
  }

  async function cancelExecution() {
    if (!projectId || !selected) return;
    if (!window.confirm("取消会保留当前执行记录和状态历史。继续？")) return;
    await run(
      () =>
        codexExecutionApi.cancel(projectId, selected.id, {
          outputSummary: outputSummary.trim() || "用户取消了执行请求。",
          expectedVersion: selected.version,
        }),
      "执行请求已取消并保留历史。",
    );
  }

  if (!project || !projectId) {
    return <p className="loading-copy">正在加载 Codex 执行记录…</p>;
  }

  const selectedProposal = selected ? proposalMap.get(selected.proposalId) : undefined;

  return (
    <div className="page-stack codex-execution-page">
      <div className="breadcrumb">
        <Link to="/projects">项目</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <Link to={`/projects/${project.id}/codex-tasks`}>Codex 任务提案</Link><span>/</span>
        <span>执行记录</span>
      </div>

      <header className="page-header">
        <div>
          <p className="eyebrow">D5 · 执行治理基础</p>
          <h1>Codex 执行记录</h1>
          <p>将已批准 Proposal 转换为可审计的执行请求、状态记录和结果记录。</p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${project.id}/codex-tasks`}>
          返回任务提案
        </Link>
      </header>

      <div className="ai-boundary-banner execution-boundary">
        <strong>记录模式，不执行代码</strong>
        <span>当前 Runner 只接收请求元数据；不会调用 codex exec、修改文件、运行验证命令或执行 Git 操作。</span>
      </div>

      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}

      <form className="panel execution-request-panel" onSubmit={createExecution}>
        <div className="panel-header">
          <div><h2>创建执行请求</h2><p>只有已批准的任务提案可以创建 pending 记录。</p></div>
        </div>
        <div className="execution-request-form">
          <label>
            已批准 Proposal
            <select required value={proposalId} onChange={(event) => setProposalId(event.target.value)}>
              <option value="">请选择任务提案</option>
              {approvedProposals.map((proposal) => (
                <option key={proposal.id} value={proposal.id}>{proposal.title} · v{proposal.version}</option>
              ))}
            </select>
          </label>
          <button className="primary-button" disabled={busy || !proposalId} type="submit">
            {busy ? "正在记录…" : "创建执行请求"}
          </button>
        </div>
        {approvedProposals.length === 0 ? (
          <p className="execution-help">当前没有已批准 Proposal。请先在任务提案页面完成人工审批。</p>
        ) : null}
      </form>

      <div className="candidate-workbench execution-workbench">
        <section className="panel candidate-list-panel">
          <h2>执行历史（Execution History）</h2>
          <div className="candidate-list execution-list">
            {executions.map((execution) => (
              <button
                className={execution.id === selected?.id ? "selected" : ""}
                key={execution.id}
                onClick={() => setSelectedId(execution.id)}
                type="button"
              >
                <span className={`execution-status execution-status--${execution.status}`}>
                  {STATUS_LABELS[execution.status]}
                </span>
                <strong>{proposalMap.get(execution.proposalId)?.title ?? execution.proposalId}</strong>
                <small>{formatUpdatedAt(execution.createdAt)} · v{execution.version}</small>
              </button>
            ))}
          </div>
          {executions.length === 0 ? <p className="empty-state">尚无执行记录。</p> : null}
        </section>

        {selected ? (
          <section className="panel candidate-detail-panel execution-detail-panel">
            <div className="panel-header">
              <div>
                <span className={`execution-status execution-status--${selected.status}`}>
                  {STATUS_LABELS[selected.status]}
                </span>
                <h2>{selectedProposal?.title ?? "关联任务提案"}</h2>
                <p>Proposal · {selected.proposalId}</p>
              </div>
            </div>

            <dl className="execution-meta-grid">
              <div><dt>请求时间</dt><dd>{formatUpdatedAt(selected.createdAt)}</dd></div>
              <div><dt>开始时间</dt><dd>{selected.startTime ? formatUpdatedAt(selected.startTime) : "尚未开始"}</dd></div>
              <div><dt>结束时间</dt><dd>{selected.endTime ? formatUpdatedAt(selected.endTime) : "尚未结束"}</dd></div>
              <div><dt>记录版本</dt><dd>v{selected.version}</dd></div>
            </dl>

            {selected.status === "running" ? (
              <div className="execution-result-form">
                <label>输出摘要<textarea rows={5} value={outputSummary} onChange={(event) => setOutputSummary(event.target.value)} /></label>
                <label>验证结果<textarea rows={5} value={validationResult} onChange={(event) => setValidationResult(event.target.value)} /></label>
              </div>
            ) : (
              <div className="execution-result-view">
                <article><h3>输出摘要</h3><pre>{selected.outputSummary || "尚无结果"}</pre></article>
                <article><h3>验证结果</h3><pre>{selected.validationResult || "尚无验证结果"}</pre></article>
              </div>
            )}

            <div className="candidate-review-actions execution-actions">
              <span>这些按钮只记录状态，不触发 Codex 或验证命令。</span>
              <div>
                {selected.status === "pending" ? (
                  <button className="primary-button" disabled={busy} onClick={() => void startExecution()} type="button">记录为运行中</button>
                ) : null}
                {selected.status === "running" ? (
                  <>
                    <button className="primary-button" disabled={busy} onClick={() => void recordResult("success")} type="button">记录成功</button>
                    <button className="danger-button" disabled={busy} onClick={() => void recordResult("failed")} type="button">记录失败</button>
                  </>
                ) : null}
                {(selected.status === "pending" || selected.status === "running") ? (
                  <button className="secondary-button" disabled={busy} onClick={() => void cancelExecution()} type="button">取消请求</button>
                ) : null}
              </div>
            </div>
          </section>
        ) : (
          <section className="panel provider-detail-empty"><strong>选择执行记录</strong><p>状态、结果和关联 Proposal 会显示在这里。</p></section>
        )}
      </div>
    </div>
  );
}
