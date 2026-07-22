import { type FormEvent, useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { githubApi } from "../api/github";
import { getErrorMessage } from "../api/project-kernel";
import type { GitHubConnector, GitHubImportDraft } from "../types/github";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";
import { GitHubAnalysisPage } from "./GitHubAnalysisPage";

export function GitHubImportPage() {
  const [searchParams] = useSearchParams();
  return searchParams.get("mode") === "import" ? <GitHubImportLegacyPage /> : <GitHubAnalysisPage />;
}

function GitHubImportLegacyPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [connector, setConnector] = useState<GitHubConnector>();
  const [drafts, setDrafts] = useState<GitHubImportDraft[]>([]);
  const [selectedId, setSelectedId] = useState<string>();
  const [repositoryUrl, setRepositoryUrl] = useState("");
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const selected = useMemo(() => drafts.find((item) => item.id === selectedId) ?? drafts[0], [drafts, selectedId]);

  useEffect(() => { void load(); }, []);

  async function load() {
    setError(undefined);
    try {
      const [connectorValue, draftValues] = await Promise.all([githubApi.getConnector(), githubApi.listDrafts()]);
      setConnector(connectorValue);
      setDrafts(draftValues);
      setSelectedId((current) => current ?? draftValues[0]?.id);
    } catch (loadError) { setError(getErrorMessage(loadError)); }
  }

  async function run(operation: () => Promise<void>) {
    setBusy(true); setError(undefined); setNotice(undefined);
    try { await operation(); } catch (operationError) { setError(getErrorMessage(operationError)); }
    finally { setBusy(false); }
  }

  function replaceDraft(updated: GitHubImportDraft) {
    setDrafts((items) => items.map((item) => item.id === updated.id ? updated : item));
    setSelectedId(updated.id);
  }

  async function handlePreview(event: FormEvent) {
    event.preventDefault();
    await run(async () => {
      const draft = await githubApi.preview(repositoryUrl.trim());
      setDrafts((items) => [draft, ...items]); setSelectedId(draft.id); setRepositoryUrl("");
      setNotice(t("github.previewCreated"));
    });
  }

  return (
    <div className="page-stack">
      <PageIntro title={t("github.title")} description={t("github.description")} secondaryActions={<button className="secondary-button" type="button" onClick={() => navigate("/github-import?mode=analysis")}>{t("githubAnalysis.analysisMode")}</button>} />

      <section className="panel github-boundary">
        <strong>{t("github.readOnlyTitle")}</strong><span>{t("github.readOnlyDescription")}</span>
      </section>

      <section className="panel github-auth-panel">
        <div className="panel-header"><div><h2>{t("github.connector")}</h2><p>{connector?.tokenConfigured ? t("github.tokenStoredDescription") : t("github.publicOnlyDescription")}</p></div><span className={`provider-state ${connector?.tokenConfigured ? "provider-state--enabled" : "provider-state--disabled"}`}>{connector?.tokenConfigured ? t("github.tokenConfigured") : t("github.publicOnly")}</span></div>
        <div className="github-token-row">
          <input type="password" autoComplete="off" value={token} onChange={(event) => setToken(event.target.value)} placeholder={t("github.tokenPlaceholder")} />
          <button className="secondary-button" disabled={busy || !token.trim()} type="button" onClick={() => void run(async () => { setConnector(await githubApi.setToken(token)); setToken(""); setNotice(t("github.tokenSaved")); })}>{t("github.saveToken")}</button>
          <button className="secondary-button" disabled={busy || !connector?.tokenConfigured} type="button" onClick={() => void run(async () => { setConnector(await githubApi.clearToken()); setNotice(t("github.tokenCleared")); })}>{t("github.clearToken")}</button>
        </div>
      </section>

      <section className="panel">
        <div className="panel-header"><div><h2>{t("github.stepUrl")}</h2><p>{t("github.description")}</p></div></div>
        <form className="github-url-form" onSubmit={handlePreview}>
          <input required type="url" value={repositoryUrl} onChange={(event) => setRepositoryUrl(event.target.value)} placeholder="https://github.com/owner/repository" />
          <button className="primary-button" disabled={busy} type="submit">{busy ? t("common.loading") : t("github.preview")}</button>
        </form>
      </section>

      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}

      <div className="github-import-workbench">
        <section className="panel github-draft-list record-list-pane">
          <div className="panel-header"><div><h2>{t("github.preview")}</h2><p>{t("github.recordCount", { count: drafts.length })}</p></div></div>
          {drafts.map((draft) => <button className={draft.id === selected?.id ? "selected" : ""} key={draft.id} onClick={() => setSelectedId(draft.id)} type="button"><strong>{draft.owner}/{draft.name}</strong><span className={`candidate-status candidate-status--${draft.status === "reviewed" ? "accepted" : draft.status === "imported" ? "accepted" : draft.status === "rejected" ? "rejected" : "pending"}`}>{displayLabel(draft.status)}</span><small>{formatUpdatedAt(draft.updatedAt)}</small></button>)}
          {!drafts.length ? <p className="empty-state">{t("github.noDrafts")}</p> : null}
        </section>

        <section className="panel github-preview-panel">
          {selected ? <>
            <div className="panel-header"><div><p className="snapshot-kicker">{t("github.preview")}</p><h2>{selected.owner}/{selected.name}</h2><p>{selected.description || t("common.notProvided")}</p></div><a className="secondary-button button-link" href={selected.url} target="_blank" rel="noreferrer">GitHub</a></div>
            <div className="github-preview-meta"><div><span>{t("github.defaultBranch")}</span><strong>{selected.defaultBranch}</strong></div><div><span>{t("github.primaryLanguage")}</span><strong>{selected.language ?? t("common.unknown")}</strong></div><div><span>{t("github.topics")}</span><strong>{selected.topics.join(", ") || t("common.none")}</strong></div><div><span>{t("github.commitsReleases")}</span><strong>{selected.recentCommits.length} / {selected.latestRelease ? "1" : "0"}</strong></div></div>
            <article className="github-readme-preview"><h3>{t("github.readmeSummary")}</h3><pre className="document-scroll-region">{selected.readmeSummary || t("github.noReadme")}</pre></article>
            <div className="github-review-actions">
              <span>{t("common.status")}：{displayLabel(selected.status)} · v{selected.version}</span>
              <div>
                {selected.status === "draft" ? <button className="secondary-button" disabled={busy} onClick={() => void run(async () => { replaceDraft(await githubApi.review(selected.id, selected.version)); setNotice(t("github.previewReviewed")); })} type="button">{t("github.confirmPreview")}</button> : null}
                {selected.status === "reviewed" ? <button className="primary-button" disabled={busy} onClick={() => void run(async () => { const result = await githubApi.importProject(selected.id, selected.version); replaceDraft(result.draft); navigate(`/projects/${result.project.id}`); })} type="button">{t("github.importProject")}</button> : null}
                {(selected.status === "draft" || selected.status === "reviewed") ? <button className="danger-button" disabled={busy} onClick={() => void run(async () => { replaceDraft(await githubApi.reject(selected.id, selected.version)); setNotice(t("github.draftRejected")); })} type="button">{t("github.ignore")}</button> : null}
                {selected.importedProjectId ? <button className="secondary-button" onClick={() => navigate(`/projects/${selected.importedProjectId}`)} type="button">{t("github.openProject")}</button> : null}
              </div>
            </div>
          </> : <div className="provider-detail-empty"><strong>{t("github.selectDraft")}</strong><p>{t("github.selectDraftDescription")}</p></div>}
        </section>
      </div>
    </div>
  );
}
