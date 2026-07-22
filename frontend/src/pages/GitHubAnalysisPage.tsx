import { type FormEvent, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { githubAnalysisApi } from "../api/github";
import { projectKernelApi } from "../api/project-kernel";
import { EmptyState, PageIntro } from "../components/ui/PlainLanguage";
import type { GitHubAnalysisFocus, GitHubAnalysisStatusView } from "../types/github";
import { formatUpdatedAt } from "../utils/format";
import { localizeAppError } from "../utils/appError";

const FOCUS_VALUES: GitHubAnalysisFocus[] = ["overview", "architecture", "learning", "reproduction", "secondary_development", "custom"];

export function GitHubAnalysisPage() {
  const { t } = useTranslation();
  const errorText = (reason: unknown) => { const value = localizeAppError(reason, t); return value.guidance ? `${value.title}：${value.guidance}` : value.title; };
  const navigate = useNavigate();
  const [workspaceId, setWorkspaceId] = useState<string>();
  const [repositoryUrl, setRepositoryUrl] = useState("");
  const [focusType, setFocusType] = useState<GitHubAnalysisFocus>("overview");
  const [customPrompt, setCustomPrompt] = useState("");
  const [runs, setRuns] = useState<GitHubAnalysisStatusView[]>([]);
  const [cursor, setCursor] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => {
    void loadInitial();
    let disposed = false;
    const unlisten = listen<GitHubAnalysisStatusView>("github-analysis://completed", (event) => {
      if (!disposed) setRuns((items) => items.map((item) => item.analysisRunId === event.payload.analysisRunId ? event.payload : item));
    });
    return () => { disposed = true; void unlisten.then((dispose) => dispose()); };
  }, []);

  async function loadInitial() {
    setError(undefined);
    try {
      const workspace = await projectKernelApi.getCurrentWorkspace();
      if (!workspace) {
        setError(t("githubAnalysis.workspaceRequired"));
        return;
      }
      setWorkspaceId(workspace.id);
      const page = await githubAnalysisApi.list(workspace.id);
      setRuns(page.items); setCursor(page.nextCursor);
    } catch (reason) { setError(errorText(reason)); }
  }

  async function start(event: FormEvent) {
    event.preventDefault();
    if (!workspaceId) return;
    setBusy(true); setError(undefined);
    try {
      const result = await githubAnalysisApi.start({ workspaceId, repositoryUrl: repositoryUrl.trim(), focusType, customPrompt: focusType === "custom" ? customPrompt.trim() : undefined, analysisMode: "rules" });
      navigate(`/github-import/analyses/${result.analysisRunId}`);
    } catch (reason) { setError(errorText(reason)); }
    finally { setBusy(false); }
  }

  async function loadMore() {
    if (!workspaceId || !cursor) return;
    const page = await githubAnalysisApi.list(workspaceId, cursor);
    setRuns((items) => [...items, ...page.items]); setCursor(page.nextCursor);
  }

  return <div className="page-stack github-analysis-index">
    <PageIntro title={t("githubAnalysis.title")} description={t("githubAnalysis.description")} />
    <div className="github-mode-tabs" aria-label={t("githubAnalysis.modeLabel")}><span aria-current="page">{t("githubAnalysis.analysisMode")}</span><button type="button" onClick={() => navigate("/github-import?mode=import")}>{t("githubAnalysis.importMode")}</button></div>
    <section className="surface github-analysis-start">
      <div><h2>{t("githubAnalysis.startTitle")}</h2><p>{t("githubAnalysis.rulesOnly")}</p></div>
      <form onSubmit={start}>
        <label>{t("githubAnalysis.repositoryUrl")}<input required type="url" value={repositoryUrl} onChange={(event) => setRepositoryUrl(event.target.value)} placeholder="https://github.com/owner/repository" /></label>
        <label>{t("githubAnalysis.focus")}<select value={focusType} onChange={(event) => setFocusType(event.target.value as GitHubAnalysisFocus)}>{FOCUS_VALUES.map((value) => <option value={value} key={value}>{t(`githubAnalysis.focus_${value}`)}</option>)}</select></label>
        {focusType === "custom" ? <label className="span-2">{t("githubAnalysis.customQuestion")}<textarea required maxLength={2000} rows={3} value={customPrompt} onChange={(event) => setCustomPrompt(event.target.value)} /></label> : null}
        <div className="span-2 form-actions"><button className="primary-button" disabled={busy || !workspaceId} type="submit">{busy ? t("githubAnalysis.resolving") : t("githubAnalysis.start")}</button></div>
      </form>
    </section>
    {error ? <div className="error-banner">{error}</div> : null}
    <section className="github-analysis-history">
      <div className="section-heading"><div><h2>{t("githubAnalysis.history")}</h2><p>{t("githubAnalysis.historyDescription")}</p></div></div>
      {runs.length ? <div className="analysis-run-list">{runs.map((run) => <button key={run.analysisRunId} type="button" onClick={() => navigate(`/github-import/analyses/${run.analysisRunId}`)}><span><strong>{run.owner}/{run.name}</strong><small>{t(`githubAnalysis.focus_${run.focusType}`)} · {formatUpdatedAt(run.createdAt)}</small></span><span className={`status-badge ${run.status === "ready" ? "success" : run.status === "failed" ? "danger" : run.status === "partial" ? "warning" : "info"}`}>{t(`githubAnalysis.status_${run.status}`)}</span></button>)}</div> : <EmptyState title={t("githubAnalysis.noHistory")} description={t("githubAnalysis.noHistoryDescription")} />}
      {cursor ? <button className="secondary-button" type="button" onClick={() => void loadMore()}>{t("common.loadMore")}</button> : null}
    </section>
  </div>;
}
