import { type FormEvent, useCallback, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { projectMemoryApi } from "../api/project-memory";
import { projectPulseApi } from "../api/project-pulse";
import { projectKernelApi, getErrorMessage } from "../api/project-kernel";
import { sessionKernelApi } from "../api/session-kernel";
import { achievementApi } from "../api/achievements";
import { ProjectMemoryNav } from "../components/ProjectMemoryNav";
import { CollapsibleText } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { Project } from "../types/project";
import type { DevelopmentSession } from "../types/session";
import type { DevelopmentHistoryItem } from "../types/project-memory";
import { formatUpdatedAt } from "../utils/format";

export function SessionsPage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [current, setCurrent] = useState<DevelopmentSession | null>(null);
  const [history, setHistory] = useState<DevelopmentHistoryItem[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();

  const load = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, currentValue, historyPage] = await Promise.all([
        projectKernelApi.getProject(projectId),
        sessionKernelApi.getCurrentSession(projectId),
        projectMemoryApi.history(projectId),
      ]);
      setProject(projectValue);
      setCurrent(currentValue);
      setHistory(historyPage.items);
      setNextCursor(historyPage.nextCursor);
    } catch (reason) {
      setError(getErrorMessage(reason));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => { void load(); }, [load]);

  async function loadMore() {
    if (!projectId || !nextCursor) return;
    setWorking(true);
    try {
      const page = await projectMemoryApi.history(projectId, nextCursor);
      setHistory((values) => [...values, ...page.items]);
      setNextCursor(page.nextCursor);
    } catch (reason) { setError(getErrorMessage(reason)); }
    finally { setWorking(false); }
  }

  async function sync() {
    if (!projectId) return;
    setWorking(true);
    setNotice(undefined);
    try {
      await projectPulseApi.sync(projectId, "manual");
      setNotice(t("memory.syncStarted"));
    } catch (reason) { setError(getErrorMessage(reason)); }
    finally { setWorking(false); }
  }

  async function saveAsAchievement(item: DevelopmentHistoryItem) {
    if (!projectId) return;
    setWorking(true);
    setNotice(undefined);
    try {
      await achievementApi.fromHistory(projectId, item.id);
      setNotice(t("memory.achievementDraftCreated"));
    } catch (reason) { setError(getErrorMessage(reason)); }
    finally { setWorking(false); }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) return <div className="error-banner">{error ?? t("projects.noProject")}</div>;

  return (
    <div className="page-stack memory-history-page">
      <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("memory.historyTitle")}</span></div>
      <PageIntro title={t("memory.historyTitle")} description={t("memory.historyDescription")} primaryAction={<button className="primary-button" disabled={working || project.status === "archived"} onClick={() => void sync()} type="button">{working ? t("projects.syncing") : t("memory.syncLatest")}</button>} />
      <ProjectMemoryNav projectId={project.id} />
      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}
      <CurrentSessionDisclosure current={current} projectId={project.id} onChanged={load} />

      <section className="development-timeline" aria-label={t("memory.historyTitle")}>
        {history.length === 0 ? <p className="memory-empty-copy">{t("memory.noHistory")}</p> : history.map((item) => {
          const isExpanded = expanded === item.id;
          return <article className="development-activity" key={item.id}>
            <button aria-expanded={isExpanded} className="development-activity__summary" onClick={() => setExpanded(isExpanded ? null : item.id)} type="button">
              <span className={`memory-trust memory-trust--${item.trustStatus}`}>{t(`memory.${item.trustStatus}`)}</span>
              <span><strong>{item.title}</strong><small>{formatUpdatedAt(item.startedAt)} · {t("memory.turnCount", { count: item.codexTurnCount })} · {t("memory.commitCount", { count: item.gitCommits.length })}</small></span>
              <span>{isExpanded ? t("common.collapse") : t("common.details")}</span>
            </button>
            {isExpanded ? <div className="development-activity__detail">
              <HistoryField label={t("memory.activityGoal")} value={item.goal} />
              <HistoryField label={t("memory.rationale")} value={item.rationale} />
              <HistoryField label={t("memory.activityDone")} value={item.activity} />
              <HistoryField label={t("memory.outcome")} value={item.outcome} />
              <HistoryField label={t("memory.validation")} value={item.validation} />
              <HistoryField label={t("memory.resumeFrom")} value={item.handoff} />
              {item.gitCommits.length > 0 ? <details><summary>{t("memory.commitCount", { count: item.gitCommits.length })}</summary><ul>{item.gitCommits.map((commit) => <li key={commit.sha}><code>{commit.sha.slice(0, 10)}</code> {commit.message}</li>)}</ul></details> : null}
              <div className="development-activity__actions"><button className="secondary-button" disabled={working} onClick={() => void saveAsAchievement(item)} type="button">{t("memory.saveAsAchievementDraft")}</button></div>
            </div> : null}
          </article>;
        })}
      </section>
      {nextCursor ? <button className="secondary-button memory-load-more" disabled={working} onClick={() => void loadMore()} type="button">{t("memory.loadMore")}</button> : null}
    </div>
  );
}

function HistoryField({ label, value }: { label: string; value: string }) {
  if (!value.trim()) return null;
  return <section><h3>{label}</h3><CollapsibleText collapsedLines={4} expandedMaxHeight={480} value={value} /></section>;
}

function CurrentSessionDisclosure({ current, projectId, onChanged }: { current: DevelopmentSession | null; projectId: string; onChanged: () => Promise<void> }) {
  const { t } = useTranslation();
  const [goal, setGoal] = useState(current?.goal ?? "");
  const [notes, setNotes] = useState(current?.notes ?? "");
  const [outcome, setOutcome] = useState("");
  const [validation, setValidation] = useState("");
  const [handoff, setHandoff] = useState(current?.handoff ?? "");
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();

  useEffect(() => { setGoal(current?.goal ?? ""); setNotes(current?.notes ?? ""); setHandoff(current?.handoff ?? ""); }, [current]);

  async function start(event: FormEvent) {
    event.preventDefault(); setWorking(true); setError(undefined);
    try { await sessionKernelApi.startSession(projectId, { goal: goal.trim() }); await onChanged(); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }
  async function update(event: FormEvent) {
    event.preventDefault(); if (!current) return; setWorking(true); setError(undefined);
    try { await sessionKernelApi.updateSession(projectId, current.id, { goal: goal.trim(), notes: notes.trim(), expectedVersion: current.version }); await onChanged(); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }
  async function finish(mode: "complete" | "abandon") {
    if (!current) return; setWorking(true); setError(undefined);
    try {
      if (mode === "complete") await sessionKernelApi.completeSession(projectId, current.id, { outcome: outcome.trim(), validation: validation.trim(), handoff: handoff.trim(), expectedVersion: current.version });
      else await sessionKernelApi.abandonSession(projectId, current.id, { handoff: handoff.trim(), expectedVersion: current.version });
      await onChanged();
    } catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }

  return <details className="manual-session-disclosure"><summary>{current ? t("memory.currentDevelopment", { goal: current.goal }) : t("memory.manualSession")}</summary><div>{error ? <div className="error-banner">{error}</div> : null}{current ? <>
    <form className="compact-form" onSubmit={update}><label>{t("session.goal")}<input required maxLength={2000} value={goal} onChange={(event) => setGoal(event.target.value)} /></label><label>{t("session.notes")}<textarea maxLength={20000} rows={5} value={notes} onChange={(event) => setNotes(event.target.value)} /></label><button className="secondary-button" disabled={working} type="submit">{t("common.save")}</button></form>
    <div className="manual-session-finish"><label>{t("session.outcome")}<textarea rows={3} value={outcome} onChange={(event) => setOutcome(event.target.value)} /></label><label>{t("session.validation")}<textarea rows={3} value={validation} onChange={(event) => setValidation(event.target.value)} /></label><label>{t("session.handoff")}<textarea rows={3} value={handoff} onChange={(event) => setHandoff(event.target.value)} /></label><div className="form-actions"><button className="text-button danger" disabled={working || !handoff.trim()} onClick={() => void finish("abandon")} type="button">{t("session.abandon")}</button><button className="secondary-button" disabled={working || !outcome.trim() || !validation.trim() || !handoff.trim()} onClick={() => void finish("complete")} type="button">{t("session.complete")}</button></div></div>
  </> : <form className="compact-form" onSubmit={start}><label>{t("session.goal")}<input required maxLength={2000} value={goal} onChange={(event) => setGoal(event.target.value)} /></label><button className="secondary-button" disabled={working || !goal.trim()} type="submit">{t("session.start")}</button></form>}</div></details>;
}
