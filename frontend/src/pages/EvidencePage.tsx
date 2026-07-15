import { type FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { achievementApi } from "../api/achievements";
import { evidenceKernelApi } from "../api/evidence-kernel";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { ProjectMemoryNav } from "../components/ProjectMemoryNav";
import { CollapsibleText, CompactIdentifier } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { AchievementCategory, AchievementInput, ProjectAchievement } from "../types/achievement";
import type { Evidence, EvidenceContentInput, EvidenceSourceType, EvidenceType } from "../types/evidence";
import type { Project } from "../types/project";
import { displayLabel, formatUpdatedAt } from "../utils/format";

const EMPTY_ACHIEVEMENT: AchievementInput = { category: "feature", title: "", problem: "", approach: "", result: "", validation: "", impact: "", technologies: [], resumeBullet: "", featured: false };
const EMPTY_MATERIAL: EvidenceContentInput = { title: "", description: "", evidenceType: "document", sourceType: "manual_note", sourceUri: "" };

export function EvidencePage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [project, setProject] = useState<Project>();
  const [achievements, setAchievements] = useState<ProjectAchievement[]>([]);
  const [materials, setMaterials] = useState<Evidence[]>([]);
  const [editing, setEditing] = useState<ProjectAchievement | "new" | null>(null);
  const [materialOpen, setMaterialOpen] = useState(false);
  const [loading, setLoading] = useState(true);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();

  const load = useCallback(async () => {
    if (!projectId) return;
    setLoading(true); setError(undefined);
    try {
      const [projectValue, achievementValues, materialValues] = await Promise.all([
        projectKernelApi.getProject(projectId), achievementApi.list(projectId), evidenceKernelApi.listEvidences(projectId),
      ]);
      setProject(projectValue); setAchievements(achievementValues); setMaterials(materialValues);
    } catch (reason) { setError(getErrorMessage(reason)); }
    finally { setLoading(false); }
  }, [projectId]);
  useEffect(() => { void load(); }, [load]);

  const drafts = useMemo(() => achievements.filter((value) => value.status === "draft"), [achievements]);
  const confirmed = useMemo(() => achievements.filter((value) => value.status === "confirmed").sort((a, b) => Number(b.featured) - Number(a.featured)), [achievements]);
  const archived = useMemo(() => achievements.filter((value) => value.status === "archived"), [achievements]);

  async function confirm(value: ProjectAchievement) {
    if (!projectId) return; setWorking(true); setError(undefined);
    try { await achievementApi.confirm(projectId, value.id, value.version); setNotice(t("memory.achievementConfirmed")); await load(); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }
  async function archive(value: ProjectAchievement) {
    if (!projectId) return; setWorking(true); setError(undefined);
    try { await achievementApi.archive(projectId, value.id, value.version); await load(); }
    catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); }
  }

  if (loading) return <p className="loading-copy">{t("common.loading")}…</p>;
  if (!project || !projectId) return <div className="error-banner">{error ?? t("projects.noProject")}</div>;
  const mainAction = drafts.length > 0 ? () => setEditing(drafts[0]) : () => setEditing("new");

  return <div className="page-stack achievement-page">
    <div className="breadcrumb"><Link to="/projects">{t("navigation.projects")}</Link><span>/</span><Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span><span>{t("memory.achievementsAndMaterials")}</span></div>
    <PageIntro title={t("memory.achievementsAndMaterials")} description={t("memory.achievementsDescription")} primaryAction={<button className="primary-button" disabled={working || project.status === "archived"} onClick={mainAction} type="button">{drafts.length > 0 ? t("memory.reviewDrafts") : t("memory.recordAchievement")}</button>} />
    <ProjectMemoryNav projectId={project.id} />
    {error ? <div className="error-banner">{error}</div> : null}{notice ? <div className="success-banner">{notice}</div> : null}
    {editing ? <AchievementEditor achievement={editing === "new" ? undefined : editing} onClose={() => setEditing(null)} onSaved={async () => { setEditing(null); await load(); }} projectId={project.id} /> : null}
    <AchievementGroup title={t("memory.draftAchievements")} values={drafts} empty={t("memory.noDraftAchievements")} onEdit={setEditing} onConfirm={confirm} onArchive={archive} working={working} />
    <AchievementGroup title={t("memory.confirmedAchievements")} values={confirmed} empty={t("memory.noAchievements")} onEdit={setEditing} onConfirm={confirm} onArchive={archive} working={working} />
    {archived.length > 0 ? <details className="archived-achievements"><summary>{t("memory.archivedAchievements")}（{archived.length}）</summary><AchievementGroup values={archived} onEdit={setEditing} onConfirm={confirm} onArchive={archive} working={working} /></details> : null}

    <section className="project-materials-section">
      <header><div><h2>{t("memory.materials")}</h2><p>{t("memory.importedMaterials")}</p></div><button className="secondary-button" disabled={project.status === "archived"} onClick={() => setMaterialOpen((value) => !value)} type="button">{t("evidence.add")}</button></header>
      {materialOpen ? <MaterialCreateForm projectId={project.id} onCreated={async () => { setMaterialOpen(false); await load(); }} /> : null}
      {materials.length === 0 ? <p className="memory-empty-copy">{t("evidence.emptyDescription")}</p> : <div className="materials-list">{materials.map((material) => <article key={material.id}><div><span>{displayLabel(material.evidenceType)}</span><span>{displayLabel(material.verificationStatus)}</span></div><strong>{material.title}</strong><CollapsibleText collapsedLines={2} value={material.description} />{material.sourceUri ? <CompactIdentifier copyable value={material.sourceUri} /> : null}<small>{formatUpdatedAt(material.updatedAt)}</small></article>)}</div>}
    </section>
  </div>;
}

function AchievementGroup({ title, values, empty, onEdit, onConfirm, onArchive, working }: { title?: string; values: ProjectAchievement[]; empty?: string; onEdit: (value: ProjectAchievement) => void; onConfirm: (value: ProjectAchievement) => Promise<void>; onArchive: (value: ProjectAchievement) => Promise<void>; working: boolean }) {
  const { t } = useTranslation();
  return <section className="achievement-group">{title ? <header><h2>{title}</h2><span>{values.length}</span></header> : null}{values.length === 0 ? <p className="memory-empty-copy">{empty}</p> : <div className="achievement-list">{values.map((value) => <article key={value.id}><div className="achievement-card__meta"><span>{displayLabel(value.category)}</span>{value.featured ? <span>{t("memory.featured")}</span> : null}<time>{formatUpdatedAt(value.updatedAt)}</time></div><h3>{value.title}</h3><CollapsibleText collapsedLines={3} value={value.result} /><div className="achievement-card__actions"><button className="text-button" onClick={() => onEdit(value)} type="button">{t("common.edit")}</button>{value.status === "draft" ? <button className="secondary-button" disabled={working} onClick={() => void onConfirm(value)} type="button">{t("memory.confirmAchievement")}</button> : null}{value.resumeBullet ? <button className="text-button" onClick={() => void navigator.clipboard.writeText(value.resumeBullet)} type="button">{t("memory.copyResume")}</button> : null}{value.status !== "archived" ? <button className="text-button" disabled={working} onClick={() => void onArchive(value)} type="button">{t("memory.archiveAchievement")}</button> : null}</div></article>)}</div>}</section>;
}

function AchievementEditor({ achievement, projectId, onClose, onSaved }: { achievement?: ProjectAchievement; projectId: string; onClose: () => void; onSaved: () => Promise<void> }) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<AchievementInput>(achievement ? { category: achievement.category, title: achievement.title, problem: achievement.problem, approach: achievement.approach, result: achievement.result, validation: achievement.validation, impact: achievement.impact, technologies: achievement.technologies, resumeBullet: achievement.resumeBullet, featured: achievement.featured } : EMPTY_ACHIEVEMENT);
  const [technologyText, setTechnologyText] = useState(draft.technologies.join(", "));
  const [working, setWorking] = useState(false); const [error, setError] = useState<string>();
  async function save(event: FormEvent) { event.preventDefault(); setWorking(true); setError(undefined); const input = { ...draft, technologies: technologyText.split(/[,，\n]/).map((value) => value.trim()).filter(Boolean) }; try { if (achievement) await achievementApi.update(projectId, achievement.id, achievement.version, input); else await achievementApi.create(projectId, input); await onSaved(); } catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); } }
  return <form className="panel achievement-editor" onSubmit={save}><header><div><h2>{achievement ? t("common.edit") : t("memory.recordAchievement")}</h2><p>{t("memory.achievementDraftBoundary")}</p></div><button className="text-button" onClick={onClose} type="button">{t("common.close")}</button></header>{error ? <div className="error-banner">{error}</div> : null}<div className="form-grid"><label>{t("memory.achievementType")}<select value={draft.category} onChange={(event) => setDraft((value) => ({ ...value, category: event.target.value as AchievementCategory }))}>{["feature", "problem_solution", "release", "technical_highlight"].map((value) => <option key={value} value={value}>{displayLabel(value)}</option>)}</select></label><label className="check-row"><input checked={draft.featured} onChange={(event) => setDraft((value) => ({ ...value, featured: event.target.checked }))} type="checkbox" />{t("memory.featured")}</label><AchievementField label={t("memory.completedWhat")} required value={draft.title} onChange={(title) => setDraft((value) => ({ ...value, title }))} /><AchievementField label={t("memory.problem")} value={draft.problem} onChange={(problem) => setDraft((value) => ({ ...value, problem }))} /><AchievementField label={t("memory.approach")} value={draft.approach} onChange={(approach) => setDraft((value) => ({ ...value, approach }))} /><AchievementField label={t("memory.result")} required value={draft.result} onChange={(result) => setDraft((value) => ({ ...value, result }))} /><AchievementField label={t("memory.validation")} value={draft.validation} onChange={(validation) => setDraft((value) => ({ ...value, validation }))} /><AchievementField label={t("memory.impact")} value={draft.impact} onChange={(impact) => setDraft((value) => ({ ...value, impact }))} /><label className="span-2">{t("memory.technologies")}<input value={technologyText} onChange={(event) => setTechnologyText(event.target.value)} placeholder="React, Rust, SQLite" /></label><AchievementField label={t("memory.resumeBullet")} value={draft.resumeBullet} onChange={(resumeBullet) => setDraft((value) => ({ ...value, resumeBullet }))} /></div><div className="form-actions"><button className="secondary-button" onClick={onClose} type="button">{t("common.cancel")}</button><button className="primary-button" disabled={working} type="submit">{working ? t("common.saving") : t("common.save")}</button></div></form>;
}

function AchievementField({ label, value, onChange, required = false }: { label: string; value: string; onChange: (value: string) => void; required?: boolean }) { return <label className="span-2">{label}<textarea required={required} rows={3} maxLength={10000} value={value} onChange={(event) => onChange(event.target.value)} /></label>; }

function MaterialCreateForm({ projectId, onCreated }: { projectId: string; onCreated: () => Promise<void> }) {
  const { t } = useTranslation(); const [draft, setDraft] = useState(EMPTY_MATERIAL); const [working, setWorking] = useState(false); const [error, setError] = useState<string>();
  async function save(event: FormEvent) { event.preventDefault(); setWorking(true); try { await evidenceKernelApi.createEvidence(projectId, { ...draft, title: draft.title.trim(), description: draft.description.trim(), sourceUri: draft.sourceUri.trim() }); await onCreated(); } catch (reason) { setError(getErrorMessage(reason)); } finally { setWorking(false); } }
  return <form className="panel material-create-form" onSubmit={save}>{error ? <div className="error-banner">{error}</div> : null}<div className="form-grid"><label>{t("common.title")}<input required value={draft.title} onChange={(event) => setDraft((value) => ({ ...value, title: event.target.value }))} /></label><label>{t("evidence.type")}<select value={draft.evidenceType} onChange={(event) => setDraft((value) => ({ ...value, evidenceType: event.target.value as EvidenceType }))}>{["document", "screenshot", "link", "test_result", "note", "artifact"].map((value) => <option key={value} value={value}>{displayLabel(value)}</option>)}</select></label><label>{t("evidence.source")}<select value={draft.sourceType} onChange={(event) => setDraft((value) => ({ ...value, sourceType: event.target.value as EvidenceSourceType }))}>{["manual_note", "local_file", "external_url"].map((value) => <option key={value} value={value}>{displayLabel(value)}</option>)}</select></label><label>{t("memory.materialLocation")}<input value={draft.sourceUri} onChange={(event) => setDraft((value) => ({ ...value, sourceUri: event.target.value }))} /></label><label className="span-2">{t("common.description")}<textarea rows={3} value={draft.description} onChange={(event) => setDraft((value) => ({ ...value, description: event.target.value }))} /></label></div><div className="form-actions"><button className="primary-button" disabled={working} type="submit">{t("evidence.add")}</button></div></form>;
}
