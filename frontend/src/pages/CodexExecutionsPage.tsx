import { type FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { codexExecutionApi } from "../api/codex-execution";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import type { CodexExecution } from "../types/codex-execution";
import type { CodexTaskProposal } from "../types/codex-task-proposal";
import type { Project } from "../types/project";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

export function CodexExecutionsPage() {
  const { t } = useTranslation();
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
      t("codex.executionCreatedNotice"),
    );
  }

  async function startExecution() {
    if (!projectId || !selected) return;
    if (!window.confirm(t("codex.startConfirm"))) return;
    await run(
      () => codexExecutionApi.start(projectId, selected.id, selected.version),
      t("codex.runningNotice"),
    );
  }

  async function recordResult(status: "success" | "failed") {
    if (!projectId || !selected) return;
    if (!outputSummary.trim() || !validationResult.trim()) {
      setError(t("codex.resultRequired"));
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
      status === "success" ? t("codex.successNotice") : t("codex.failedNotice"),
    );
  }

  async function cancelExecution() {
    if (!projectId || !selected) return;
    if (!window.confirm(t("codex.cancelConfirm"))) return;
    await run(
      () =>
        codexExecutionApi.cancel(projectId, selected.id, {
          outputSummary: outputSummary.trim() || t("codex.userCancelled"),
          expectedVersion: selected.version,
        }),
      t("codex.cancelledNotice"),
    );
  }

  if (!project || !projectId) {
    return <p className="loading-copy">{t("common.loading")}…</p>;
  }

  const selectedProposal = selected ? proposalMap.get(selected.proposalId) : undefined;

  return (
    <div className="page-stack codex-execution-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <Link to={`/projects/${project.id}/codex-tasks`}>{t("codex.tasksTitle")}</Link><span>/</span>
        <span>{t("codex.executionsTitle")}</span>
      </div>

      <PageIntro title={t("codex.executionsTitle")} technicalTerm={t("terms.execution")} description={t("codex.executionsDescription")} secondaryActions={<Link className="secondary-button button-link" to={`/projects/${project.id}/codex-tasks`}>{t("codex.tasksTitle")}</Link>} />

      <div className="ai-boundary-banner execution-boundary">
        <strong>{t("codex.compatibility")}</strong>
        <span>{t("codex.executionsDescription")}</span>
      </div>

      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}

      <form className="panel execution-request-panel" onSubmit={createExecution}>
        <div className="panel-header">
          <div><h2>{t("codex.createExecution")}</h2><p>{t("codex.createExecutionDescription")}</p></div>
        </div>
        <div className="execution-request-form">
          <label>
            {t("codex.approvedTask")}
            <select required value={proposalId} onChange={(event) => setProposalId(event.target.value)}>
              <option value="">{t("codex.chooseTask")}</option>
              {approvedProposals.map((proposal) => (
                <option key={proposal.id} value={proposal.id}>{proposal.title} · v{proposal.version}</option>
              ))}
            </select>
          </label>
          <button className="primary-button" disabled={busy || !proposalId} type="submit">
            {busy ? t("codex.recording") : t("codex.createExecution")}
          </button>
        </div>
        {approvedProposals.length === 0 ? (
          <p className="execution-help">{t("codex.noApproved")}</p>
        ) : null}
      </form>

      <div className="candidate-workbench execution-workbench">
        <section className="panel candidate-list-panel">
          <h2>{t("codex.executionsTitle")}</h2>
          <div className="candidate-list execution-list record-list-pane">
            {executions.map((execution) => (
              <button
                className={execution.id === selected?.id ? "selected" : ""}
                key={execution.id}
                onClick={() => setSelectedId(execution.id)}
                type="button"
              >
                <span className={`execution-status execution-status--${execution.status}`}>
                  {displayLabel(execution.status)}
                </span>
                <strong>{proposalMap.get(execution.proposalId)?.title ?? execution.proposalId}</strong>
                <small>{formatUpdatedAt(execution.createdAt)} · v{execution.version}</small>
              </button>
            ))}
          </div>
          {executions.length === 0 ? <p className="empty-state">{t("codex.noExecutions")}</p> : null}
        </section>

        {selected ? (
          <section className="panel candidate-detail-panel execution-detail-panel">
            <div className="panel-header">
              <div>
                <span className={`execution-status execution-status--${selected.status}`}>
                  {displayLabel(selected.status)}
                </span>
                <h2>{selectedProposal?.title ?? t("codex.relatedTask")}</h2>
              </div>
            </div>

            <dl className="execution-meta-grid">
              <div><dt>{t("codex.requestedAt")}</dt><dd>{formatUpdatedAt(selected.createdAt)}</dd></div>
              <div><dt>{t("codex.startedAt")}</dt><dd>{selected.startTime ? formatUpdatedAt(selected.startTime) : t("codex.notStarted")}</dd></div>
              <div><dt>{t("codex.endedAt")}</dt><dd>{selected.endTime ? formatUpdatedAt(selected.endTime) : t("codex.notEnded")}</dd></div>
              <div><dt>{t("common.version")}</dt><dd>v{selected.version}</dd></div>
            </dl>

            {selected.status === "running" ? (
              <div className="execution-result-form">
                <label>{t("codex.outputSummary")}<textarea rows={5} value={outputSummary} onChange={(event) => setOutputSummary(event.target.value)} /></label>
                <label>{t("codex.validationResult")}<textarea rows={5} value={validationResult} onChange={(event) => setValidationResult(event.target.value)} /></label>
              </div>
            ) : (
              <div className="execution-result-view">
                <article><h3>{t("codex.outputSummary")}</h3><pre className="document-scroll-region">{selected.outputSummary || t("codex.noResult")}</pre></article>
                <article><h3>{t("codex.validationResult")}</h3><pre className="document-scroll-region">{selected.validationResult || t("codex.noValidation")}</pre></article>
              </div>
            )}

            <div className="candidate-review-actions execution-actions">
              <span>{t("codex.recordOnly")}</span>
              <div>
                {selected.status === "pending" ? (
                  <button className="primary-button" disabled={busy} onClick={() => void startExecution()} type="button">{t("codex.markRunning")}</button>
                ) : null}
                {selected.status === "running" ? (
                  <>
                    <button className="primary-button" disabled={busy} onClick={() => void recordResult("success")} type="button">{t("codex.markSuccess")}</button>
                    <button className="danger-button" disabled={busy} onClick={() => void recordResult("failed")} type="button">{t("codex.markFailed")}</button>
                  </>
                ) : null}
                {(selected.status === "pending" || selected.status === "running") ? (
                  <button className="secondary-button" disabled={busy} onClick={() => void cancelExecution()} type="button">{t("codex.cancelRequest")}</button>
                ) : null}
              </div>
            </div>
          </section>
        ) : (
          <section className="panel provider-detail-empty"><strong>{t("codex.selectExecution")}</strong><p>{t("codex.selectExecutionDescription")}</p></section>
        )}
      </div>
    </div>
  );
}
