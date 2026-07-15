import { type FormEvent, useCallback, useEffect, useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { aiProviderApi } from "../api/ai-provider";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { projectPulseApi } from "../api/project-pulse";
import { sessionKernelApi } from "../api/session-kernel";
import { snapshotKernelApi } from "../api/snapshot-kernel";
import type { AiProvider } from "../types/ai-provider";
import type { Project } from "../types/project";
import type { CodexConnectorSettings, ProjectActivitySettings, ProjectPulse, ProjectPulseSource } from "../types/project-pulse";
import type { DevelopmentSession } from "../types/session";
import type { ProjectSnapshot } from "../types/snapshot";
import { formatUpdatedAt } from "../utils/format";
import { CollapsibleText } from "../components/content/ContentGovernance";
import { TechnicalDetails } from "../components/ui/PlainLanguage";

type Draft = { currentGoal: string; statusSummary: string; achievements: string; blockers: string; nextSteps: string; validationSummary: string; handoff: string; gitSummary: string };

export function ProjectPulsePage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const location = useLocation();
  const [project, setProject] = useState<Project>();
  const [pulse, setPulse] = useState<ProjectPulse | null>(null);
  const [history, setHistory] = useState<ProjectPulse[]>([]);
  const [sources, setSources] = useState<ProjectPulseSource[]>([]);
  const [snapshot, setSnapshot] = useState<ProjectSnapshot | null>(null);
  const [session, setSession] = useState<DevelopmentSession | null>(null);
  const [settings, setSettings] = useState<ProjectActivitySettings>();
  const [connector, setConnector] = useState<CodexConnectorSettings>();
  const [providers, setProviders] = useState<AiProvider[]>([]);
  const [draft, setDraft] = useState<Draft>();
  const [updateSnapshot, setUpdateSnapshot] = useState(true);
  const [applySession, setApplySession] = useState(false);
  const [completeSession, setCompleteSession] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [sourceLimit, setSourceLimit] = useState(5);
  const [historyLimit, setHistoryLimit] = useState(10);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);

  const load = useCallback(async () => {
    if (!projectId) return;
    setError(undefined);
    try {
      const [projectValue, pulseValue, pulseHistory, snapshotValue, sessionValue, settingsValue, providerValues] = await Promise.all([
        projectKernelApi.getProject(projectId), projectPulseApi.latest(projectId), projectPulseApi.list(projectId), snapshotKernelApi.getCurrentSnapshot(projectId), sessionKernelApi.getCurrentSession(projectId), projectPulseApi.settings(projectId), aiProviderApi.list(),
      ]);
      setProject(projectValue); setPulse(pulseValue); setHistory(pulseHistory); setSnapshot(snapshotValue); setSession(sessionValue); setSettings(settingsValue); setProviders(providerValues);
      setUpdateSnapshot(true);
      setApplySession(pulseValue ? ["implementation", "debugging"].includes(pulseValue.activityType) : false);
      setCompleteSession(false);
      setSourceLimit(5);
      setHistoryLimit(10);
      setDraft(pulseValue ? toDraft(pulseValue) : undefined);
      setSources(pulseValue ? await projectPulseApi.sources(projectId, pulseValue.id) : []);
      try { setConnector(await projectPulseApi.connectorStatus()); } catch { setConnector(undefined); }
    } catch (value) { setError(getErrorMessage(value)); }
  }, [projectId]);

  useEffect(() => { void load(); }, [load]);
  useEffect(() => { if (location.hash === "#activity-settings") setSettingsOpen(true); }, [location.hash]);

  async function saveDraft(event: FormEvent) {
    event.preventDefault();
    if (!projectId || !pulse || !draft) return;
    setSaving(true); setError(undefined); setNotice(undefined);
    try {
      const updated = await projectPulseApi.updateDraft(projectId, pulse.id, { currentGoal: draft.currentGoal, statusSummary: draft.statusSummary, achievements: lines(draft.achievements), blockers: lines(draft.blockers), nextSteps: lines(draft.nextSteps), validationSummary: draft.validationSummary, handoff: draft.handoff, gitSummary: draft.gitSummary, expectedVersion: pulse.version });
      setPulse(updated); setDraft(toDraft(updated)); setNotice(t("pulseReview.draftSaved"));
    } catch (value) { setError(getErrorMessage(value)); } finally { setSaving(false); }
  }

  async function confirm() {
    if (!projectId || !pulse) return;
    setSaving(true); setError(undefined); setNotice(undefined);
    try {
      await projectPulseApi.confirm(projectId, pulse.id, { expectedVersion: pulse.version, updateSnapshot, applySession, completeSession });
      setNotice(t("pulseReview.confirmedNotice"));
      await load();
    } catch (value) {
      setError(getErrorCode(value) === "project_pulse_basis_conflict" ? t("pulseReview.basisConflict") : getErrorMessage(value));
    } finally { setSaving(false); }
  }

  async function saveSettings() {
    if (!projectId || !settings) return;
    setSaving(true); setError(undefined);
    try {
      const updated = await projectPulseApi.updateSettings(projectId, { syncMode: settings.syncMode, aiEnabled: settings.aiEnabled, aiProviderId: settings.aiEnabled ? settings.aiProviderId : null, expectedVersion: settings.version });
      setSettings(updated); setNotice(t("pulseReview.settingsSaved"));
    } catch (value) { setError(getErrorMessage(value)); } finally { setSaving(false); }
  }

  async function saveConnector() {
    if (!connector) return;
    setSaving(true); setError(undefined);
    try { setConnector(await projectPulseApi.updateConnector(connector.executablePath, connector.version)); setNotice(t("pulseReview.connectorSaved")); }
    catch (value) { setError(getErrorMessage(value)); } finally { setSaving(false); }
  }

  if (!projectId) return <div className="error-banner">{t("projects.noProject")}</div>;
  if (!project) return <div className="page-stack"><p className="loading-copy">{t("pulseReview.loading")}…</p>{error ? <div className="error-banner">{error}</div> : null}</div>;
  const blockingWarnings = pulse?.qualityWarnings.filter((warning) => !warning.startsWith("旧版规则摘要")) ?? [];
  const legacyOpenSession = Boolean(session && history.some((item) => item.analysisRevision === "legacy-v1" && item.confirmedSessionId === session.id));

  return <div className="page-stack pulse-review-page">
    <div className="breadcrumb"><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("pulseReview.breadcrumb")}</span></div>
    <header className="page-header"><div><p className="eyebrow">{t("pulseReview.eyebrow")}</p><h1>{t("pulse.reviewTitle")}</h1><p>{t("pulseReview.description")}</p></div>{pulse ? <span className={`pulse-state pulse-state--${pulse.status}`}>{pulse.status === "pending_review" ? t("pulse.pending") : t("pulse.confirmed")}</span> : null}</header>

    {!pulse || !draft ? <section className="panel empty-pulse"><h2>{t("pulseReview.noDraft")}</h2><p>{t("pulseReview.noDraftDescription")}</p><Link className="primary-button button-link" to={`/projects/${project.id}`}>{t("pulseReview.back")}</Link></section> : <>
      <section className="panel pulse-quality-summary" aria-label={t("pulseReview.analysisMethod")}>
        <div><span>{t("pulseReview.activityType")}</span><strong>{activityTypeLabel(pulse.activityType, t)}</strong></div>
        <div><span>{t("pulseReview.analysisMethod")}</span><strong>{revisionLabel(pulse.analysisRevision, t)}</strong></div>
        <div><span>{t("pulseReview.sourceWindow")}</span><strong>{pulse.sourceWindowStartedAt && pulse.sourceWindowEndedAt ? `${formatUpdatedAt(pulse.sourceWindowStartedAt)} — ${formatUpdatedAt(pulse.sourceWindowEndedAt)}` : t("pulseReview.sourceWindowUnknown")}</strong></div>
        <div><span>{t("pulse.sources")}</span><strong>{t("pulseReview.activityCount", { selected: pulse.selectedObservationCount || pulse.sourceCount, available: pulse.availableObservationCount || pulse.sourceCount })}</strong></div>
      </section>
      {pulse.qualityWarnings.length ? <section className="pulse-quality-warnings" role="alert"><strong>{t("pulseReview.checkBeforeConfirm")}</strong><ul>{pulse.qualityWarnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></section> : null}
      <section className="pulse-review-layout">
        <form className="panel pulse-editor" onSubmit={saveDraft}>
          <div className="panel-header"><div><h2>{t("pulseReview.draft")}</h2><p>{pulse.analysisMode === "ai" ? t("pulse.aiAssisted") : t("pulse.localRules")} · {t("pulseReview.generatedAt", { time: formatUpdatedAt(pulse.generatedAt) })}</p></div></div>
          <div className="pulse-editor__fields">
            <label>{t("pulseReview.currentGoal")}<input maxLength={200} value={draft.currentGoal} onChange={(event) => setDraft({ ...draft, currentGoal: event.target.value })} /></label>
            <label>{t("pulseReview.currentSituation")}<textarea maxLength={400} rows={4} value={draft.statusSummary} onChange={(event) => setDraft({ ...draft, statusSummary: event.target.value })} /></label>
            <label>{t("pulseReview.achievements")}<textarea rows={5} value={draft.achievements} onChange={(event) => setDraft({ ...draft, achievements: event.target.value })} /></label>
            <label>{t("pulseReview.validation")}<textarea rows={3} value={draft.validationSummary} onChange={(event) => setDraft({ ...draft, validationSummary: event.target.value })} /></label>
            <label>{t("pulseReview.blockers")}<textarea rows={4} value={draft.blockers} onChange={(event) => setDraft({ ...draft, blockers: event.target.value })} /></label>
            <label>{t("pulseReview.nextSteps")}<textarea rows={4} value={draft.nextSteps} onChange={(event) => setDraft({ ...draft, nextSteps: event.target.value })} /></label>
            <label>{t("pulseReview.handoff")}<textarea rows={3} value={draft.handoff} onChange={(event) => setDraft({ ...draft, handoff: event.target.value })} /></label>
            <label>{t("pulseReview.gitSummary")}<textarea rows={2} value={draft.gitSummary} onChange={(event) => setDraft({ ...draft, gitSummary: event.target.value })} /></label>
          </div>
          <TechnicalDetails label={t("common.technicalDetails")}><code>{pulse.analysisRevision}</code><code>{pulse.contentHash}</code></TechnicalDetails>
          <div className="pulse-editor__actions"><span>{t("pulseReview.saveDraftHint")}</span><button className="secondary-button" disabled={saving || pulse.status !== "pending_review"} type="submit">{t("pulseReview.saveDraft")}</button></div>
        </form>

        <aside className="pulse-review-sidebar">
          <section className="panel fact-compare">
            <div className="panel-header"><div><h2>{t("pulseReview.confirmedComparison")}</h2><p>{t("pulseReview.comparisonDescription")}</p></div><div className="fact-compare__links"><Link to={`/projects/${project.id}/snapshot`}>{t("workbench.viewFullState")}</Link><Link to={`/projects/${project.id}/sessions`}>{t("workbench.viewSession")}</Link></div></div>
            <dl>
              <div><dt>{t("snapshot.title")} · {t("projects.currentGoal")}</dt><dd><CollapsibleText collapsedLines={3} previewMode="fact" value={snapshot?.currentGoal ?? t("pulseReview.noConfirmedState")} /></dd></div>
              <div><dt>{t("snapshot.title")} · {t("projects.currentSituation")}</dt><dd><CollapsibleText collapsedLines={3} value={snapshot?.statusSummary ?? t("pulseReview.noConfirmedState")} /></dd></div>
              <div><dt>{t("snapshot.title")} · {t("projects.blockers")} / {t("projects.nextSteps")}</dt><dd><CollapsibleText collapsedLines={3} value={snapshot ? [...snapshot.blockers, ...snapshot.nextSteps].join("；") || t("pulseReview.noConfirmedState") : t("pulseReview.noConfirmedState")} /></dd></div>
              <div><dt>{t("session.current")} · {t("session.goal")}</dt><dd><CollapsibleText collapsedLines={3} previewMode="fact" value={session?.goal ?? t("pulseReview.noOpenDevelopment")} /></dd></div>
              <div><dt>{t("session.current")} · {t("session.notes")}</dt><dd><CollapsibleText collapsedLines={3} value={session?.notes || t("pulseReview.noConfirmedState")} /></dd></div>
            </dl>
          </section>
          <section className="panel confirmation-box"><div className="panel-header"><div><h2>{t("pulseReview.applyTitle")}</h2><p>{t("pulseReview.applyDescription")}</p></div></div>{legacyOpenSession ? <div className="legacy-session-note"><strong>{t("pulseReview.legacySessionTitle")}</strong><span>{t("pulseReview.legacySessionDescription")}</span><Link to={`/projects/${project.id}/sessions`}>{t("workbench.viewSession")}</Link></div> : null}<label><input checked={updateSnapshot} onChange={(event) => setUpdateSnapshot(event.target.checked)} type="checkbox" />{t("pulse.updateSnapshot")}</label><label><input checked={applySession} onChange={(event) => setApplySession(event.target.checked)} type="checkbox" />{t("pulse.updateSession")}</label><label className="danger-choice"><input checked={completeSession} onChange={(event) => setCompleteSession(event.target.checked)} type="checkbox" />{t("pulse.completeSession")}</label><button className="primary-button" disabled={saving || pulse.status !== "pending_review" || blockingWarnings.length > 0} onClick={() => void confirm()} type="button">{t("pulseReview.confirm")}</button>{blockingWarnings.length ? <small className="confirmation-blocked">{t("pulseReview.qualityBlocked")}</small> : null}<button className="text-button" disabled={saving || pulse.status !== "pending_review"} onClick={() => void projectPulseApi.dismiss(project.id, pulse.id, pulse.version).then(load).catch((value) => setError(getErrorMessage(value)))} type="button">{t("pulse.dismiss")}</button></section>
        </aside>
      </section>

      <section className="panel pulse-sources"><div className="panel-header"><div><h2>{t("pulse.sources")}</h2><p>{t("pulseReview.sourceDescription")}</p></div><span>{t("pulseReview.sourceTurns", { count: sources.length })}</span></div>{sources.length ? <><div className="source-list">{sources.slice(0, sourceLimit).map((source) => <article key={source.id}><div className="source-meta"><span>{source.sourceKind} · {source.turnStatus}</span><time>{formatUpdatedAt(source.sourceUpdatedAt)}</time></div><CollapsibleText className="source-goal" collapsedLines={3} previewMode="fact" value={source.goalHint || t("common.notProvided")} /><CollapsibleText collapsedLines={3} value={source.outcomeSummary || source.activitySummary || t("common.notProvided")} />{source.validationSummary ? <small>{t("projects.validation")}：<CollapsibleText collapsedLines={2} value={source.validationSummary} /></small> : null}</article>)}</div>{sourceLimit < sources.length ? <button className="secondary-button content-load-more" onClick={() => setSourceLimit((value) => Math.min(value + 5, sources.length))} type="button">{t("pulseReview.showMoreSources", { count: Math.min(5, sources.length - sourceLimit) })}</button> : null}</> : <p className="empty-copy">{t("pulseReview.noSources")}</p>}</section>
    </>}

    <details className="secondary-disclosure" id="activity-settings" onToggle={(event) => setSettingsOpen(event.currentTarget.open)} open={settingsOpen}>
      <summary>{t("pulseReview.settingsSummary")}</summary>
      <div className="settings-grid">
        <section className="panel compact-settings"><div className="panel-header"><div><h2>{t("pulseReview.automaticSync")}</h2><p>{t("pulseReview.automaticSyncDescription")}</p></div></div>{settings ? <div className="settings-fields"><label>{t("pulseReview.syncMode")}<select value={settings.syncMode} onChange={(event) => setSettings({ ...settings, syncMode: event.target.value as ProjectActivitySettings["syncMode"] })}><option value="open_manual">{t("pulseReview.syncOpenManual")}</option><option value="interval_30m">{t("pulseReview.syncInterval")}</option></select></label><label className="inline-check"><input checked={settings.aiEnabled} onChange={(event) => setSettings({ ...settings, aiEnabled: event.target.checked })} type="checkbox" />{t("pulseReview.enableAi")}</label><label>{t("pulseReview.chooseAi")}<select disabled={!settings.aiEnabled} value={settings.aiProviderId ?? ""} onChange={(event) => setSettings({ ...settings, aiProviderId: event.target.value || null })}><option value="">{t("common.none")}</option>{providers.filter((provider) => provider.enabled).map((provider) => <option key={provider.id} value={provider.id}>{provider.name} · {provider.model}</option>)}</select></label><button className="secondary-button" disabled={saving} onClick={() => void saveSettings()} type="button">{t("pulseReview.saveSync")}</button></div> : null}</section>
        <section className="panel compact-settings"><div className="panel-header"><div><h2>{t("pulseReview.codexConnection")}</h2><p>{t("pulseReview.codexConnectionDescription")}</p></div>{connector?.available ? <span className="health-ok">{t("pulseReview.codexAvailable")}</span> : <span className="health-bad">{t("pulseReview.codexUnavailable")}</span>}</div>{connector ? <div className="settings-fields"><label>{t("pulseReview.codexPath")}<input value={connector.executablePath ?? ""} onChange={(event) => setConnector({ ...connector, executablePath: event.target.value || null })} /></label><small>{connector.lastDetectedVersion ?? t("pulseReview.codexUnavailable")}</small><button className="secondary-button" disabled={saving} onClick={() => void saveConnector()} type="button">{t("pulseReview.saveCodex")}</button></div> : <p className="empty-copy">{t("pulseReview.codexUnavailable")}</p>}</section>
      </div>
    </details>

    <details className="secondary-disclosure" onToggle={(event) => setHistoryOpen(event.currentTarget.open)} open={historyOpen}><summary>{t("pulseReview.history", { count: history.length })}</summary><section className="panel pulse-history"><div className="panel-header"><div><h2>{t("pulseReview.history", { count: history.length })}</h2><p>{t("pulseReview.historyDescription")}</p></div></div><div>{history.slice(0, historyLimit).map((item) => <article key={item.id}><span className={`pulse-state pulse-state--${item.status}`}>{t(`pulse.${item.status}`)}</span><strong title={item.currentGoal}>{item.currentGoal}</strong><time>{formatUpdatedAt(item.generatedAt)}</time></article>)}</div>{historyLimit < history.length ? <button className="secondary-button content-load-more" onClick={() => setHistoryLimit((value) => Math.min(value + 10, history.length))} type="button">{t("pulseReview.showMoreHistory", { count: Math.min(10, history.length - historyLimit) })}</button> : null}</section></details>
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
  </div>;
}

function lines(value: string) { return value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean); }
function toDraft(value: ProjectPulse): Draft { return { currentGoal: value.currentGoal, statusSummary: value.statusSummary, achievements: value.achievements.join("\n"), blockers: value.blockers.join("\n"), nextSteps: value.nextSteps.join("\n"), validationSummary: value.validationSummary, handoff: value.handoff, gitSummary: value.gitSummary }; }
function activityTypeLabel(value: ProjectPulse["activityType"], t: (key: string) => string) { return { implementation: t("pulseReview.activityImplementation"), debugging: t("pulseReview.activityDebugging"), release: t("pulseReview.activityRelease"), review: t("pulseReview.activityReview"), support_question: t("pulseReview.activitySupport"), unknown: t("pulseReview.activityUnknown") }[value]; }
function revisionLabel(value: ProjectPulse["analysisRevision"], t: (key: string) => string) { return { "legacy-v1": t("pulseReview.revisionLegacy"), "rules-v2": t("pulseReview.revisionRules"), "ai-v2": t("pulseReview.revisionAi") }[value]; }
