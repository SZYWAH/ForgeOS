import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link, useParams, useSearchParams } from "react-router-dom";
import { evidenceKernelApi } from "../api/evidence-kernel";
import { getErrorCode, getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { sessionKernelApi } from "../api/session-kernel";
import { snapshotKernelApi } from "../api/snapshot-kernel";
import {
  EVIDENCE_RELATION_TYPES,
  EVIDENCE_SOURCE_TYPES,
  EVIDENCE_TARGET_TYPES,
  MANUAL_EVIDENCE_TYPES,
  type Evidence,
  type EvidenceContentInput,
  type EvidenceRecord,
  type EvidenceRelationType,
  type EvidenceSourceType,
  type EvidenceTargetType,
  type EvidenceType,
} from "../types/evidence";
import type { Project } from "../types/project";
import type { DevelopmentSession } from "../types/session";
import type { ProjectSnapshot } from "../types/snapshot";
import { displayLabel, formatUpdatedAt } from "../utils/format";
import { useTranslation } from "react-i18next";
import { PageIntro } from "../components/ui/PlainLanguage";

const EMPTY_EVIDENCE: EvidenceContentInput = {
  title: "",
  description: "",
  evidenceType: "document",
  sourceType: "local_file",
  sourceUri: "",
};

export function EvidencePage() {
  const { t } = useTranslation();
  const { projectId } = useParams();
  const [searchParams, setSearchParams] = useSearchParams();
  const [project, setProject] = useState<Project>();
  const [items, setItems] = useState<Evidence[]>([]);
  const [snapshots, setSnapshots] = useState<ProjectSnapshot[]>([]);
  const [sessions, setSessions] = useState<DevelopmentSession[]>([]);
  const [record, setRecord] = useState<EvidenceRecord>();
  const [draft, setDraft] = useState<EvidenceContentInput>(EMPTY_EVIDENCE);
  const [createDraft, setCreateDraft] = useState<EvidenceContentInput>(EMPTY_EVIDENCE);
  const [showCreate, setShowCreate] = useState(false);
  const [targetType, setTargetType] = useState<EvidenceTargetType>("project");
  const [targetId, setTargetId] = useState("");
  const [relationType, setRelationType] = useState<EvidenceRelationType>("supports");
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string>();
  const [notice, setNotice] = useState<string>();
  const loadGeneration = useRef(0);

  const selectedId = searchParams.get("selected") ?? "";

  const loadIndex = useCallback(async () => {
    if (!projectId) {
      setError(t("projects.noProject"));
      setLoading(false);
      return;
    }
    const generation = ++loadGeneration.current;
    setLoading(true);
    setError(undefined);
    try {
      const [projectValue, evidenceValue, snapshotValue, sessionValue] =
        await Promise.all([
          projectKernelApi.getProject(projectId),
          evidenceKernelApi.listEvidences(projectId),
          snapshotKernelApi.getSnapshotHistory(projectId),
          sessionKernelApi.listSessions(projectId),
        ]);
      if (generation !== loadGeneration.current) {
        return;
      }
      setProject(projectValue);
      setItems(evidenceValue);
      setSnapshots(snapshotValue);
      setSessions(sessionValue);
      const requested = searchParams.get("selected");
      if (!requested && evidenceValue.length > 0) {
        setSearchParams({ selected: evidenceValue[0].id }, { replace: true });
      }
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setLoading(false);
      }
    }
  }, [projectId, searchParams, setSearchParams]);

  const loadDetail = useCallback(async () => {
    if (!projectId || !selectedId) {
      setRecord(undefined);
      return;
    }
    const generation = loadGeneration.current;
    setDetailLoading(true);
    setError(undefined);
    try {
      const value = await evidenceKernelApi.getEvidence(projectId, selectedId);
      if (generation === loadGeneration.current) {
        setRecord(value);
        setDraft(contentFrom(value.evidence));
      }
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    } finally {
      if (generation === loadGeneration.current) {
        setDetailLoading(false);
      }
    }
  }, [projectId, selectedId]);

  useEffect(() => {
    void loadIndex();
    return () => {
      loadGeneration.current += 1;
    };
    // The query selection is handled by loadDetail; the index should reload only per Project.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  useEffect(() => {
    void loadDetail();
  }, [loadDetail]);

  const targetOptions = useMemo(() => {
    if (!project) {
      return [];
    }
    if (targetType === "project") {
      return [{ id: project.id, label: project.name }];
    }
    if (targetType === "snapshot") {
      return snapshots.map((snapshot) => ({
        id: snapshot.id,
        label: `${t("snapshot.title")} v${snapshot.version} · ${snapshot.currentGoal}`,
      }));
    }
    return sessions.map((session) => ({
      id: session.id,
      label: `${displayLabel(session.status)} · ${session.goal}`,
    }));
  }, [project, sessions, snapshots, targetType]);

  useEffect(() => {
    setTargetId(targetOptions[0]?.id ?? "");
  }, [targetOptions]);

  async function refreshAfterChange(evidenceId: string) {
    if (!projectId) {
      return;
    }
    const evidenceValue = await evidenceKernelApi.listEvidences(projectId);
    setItems(evidenceValue);
    setSearchParams({ selected: evidenceId }, { replace: true });
    const value = await evidenceKernelApi.getEvidence(projectId, evidenceId);
    setRecord(value);
    setDraft(contentFrom(value.evidence));
  }

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const created = await evidenceKernelApi.createEvidence(projectId, normalized(createDraft));
      setCreateDraft(EMPTY_EVIDENCE);
      setShowCreate(false);
      await refreshAfterChange(created.id);
      setNotice(t("evidence.createdNotice"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleUpdate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !record) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const updated = await evidenceKernelApi.updateEvidence(
        projectId,
        record.evidence.id,
        { ...normalized(draft), expectedVersion: record.evidence.version },
      );
      await refreshAfterChange(updated.id);
      setNotice(t("evidence.updatedNotice"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
      if (getErrorCode(saveError) === "evidence_version_conflict") {
        await loadDetail();
      }
    } finally {
      setSaving(false);
    }
  }

  async function handleStatus(action: "verify" | "invalidate") {
    if (!projectId || !record) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const input = { expectedVersion: record.evidence.version };
      const updated =
        action === "verify"
          ? await evidenceKernelApi.verifyEvidence(projectId, record.evidence.id, input)
          : await evidenceKernelApi.invalidateEvidence(projectId, record.evidence.id, input);
      await refreshAfterChange(updated.id);
      setNotice(action === "verify" ? t("evidence.verifiedNotice") : t("evidence.invalidNotice"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  async function handleLink(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!projectId || !record || !targetId) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      await evidenceKernelApi.linkEvidence(projectId, record.evidence.id, {
        targetType,
        targetId,
        relationType,
      });
      await refreshAfterChange(record.evidence.id);
      setNotice(t("evidence.linkSaved"));
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <p className="loading-copy">{t("common.loading")}…</p>;
  }

  if (!project || !projectId) {
    return (
      <section className="panel not-found">
        <h1>{t("evidence.title")}</h1>
        <p>{error ?? t("projects.noProject")}</p>
        <Link to="/projects">{t("navigation.projectList")}</Link>
      </section>
    );
  }

  const archived = project.status === "archived";
  const gitEvidence = record
    ? ["git_commit", "git_status", "github_repository", "github_commit", "github_release"].includes(record.evidence.evidenceType)
    : false;

  return (
    <div className="page-stack evidence-page">
      <div className="breadcrumb">
        <Link to="/projects">{t("navigation.projects")}</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>{t("evidence.title")}</span>
      </div>

      <header className="page-header evidence-page-header">
        <div>
          <PageIntro title={t("evidence.title")} technicalTerm={t("terms.evidence")} description={t("evidence.description")} />
        </div>
        <button
          className="primary-button"
          disabled={archived}
          onClick={() => setShowCreate((value) => !value)}
          type="button"
        >
          {showCreate ? t("common.cancel") : t("evidence.add")}
        </button>
      </header>

      {archived ? <div className="archive-banner">{t("evidence.archivedNotice")}</div> : null}
      {error ? <div className="error-banner evidence-message">{error}</div> : null}
      {notice ? <div className="success-banner evidence-message">{notice}</div> : null}

      {showCreate ? (
        <form className="panel evidence-create-form" onSubmit={handleCreate}>
          <div className="panel-header">
            <div><h2>{t("evidence.add")}</h2><p>{t("evidence.description")}</p></div>
          </div>
          <EvidenceFields value={createDraft} onChange={setCreateDraft} />
          <div className="form-actions evidence-form-actions">
            <button className="primary-button" disabled={saving} type="submit">
              {saving ? t("common.saving") : t("evidence.add")}
            </button>
          </div>
        </form>
      ) : null}

      <div className="evidence-workbench">
        <section className="panel evidence-list-panel">
          <div className="panel-header">
            <div><h2>{t("evidence.title")}</h2><p>{t("evidence.recordCount", { count: items.length })}</p></div>
          </div>
          {items.length === 0 ? (
            <p className="empty-state">{t("evidence.emptyDescription")}</p>
          ) : (
            <div className="evidence-list record-list-pane">
              {items.map((item) => (
                <button
                  className={item.id === selectedId ? "selected" : ""}
                  key={item.id}
                  onClick={() => setSearchParams({ selected: item.id })}
                  type="button"
                >
                  <span className={`verification verification--${item.verificationStatus}`}>
                    {t(`evidence.${item.verificationStatus}`)}
                  </span>
                  <strong>{item.title}</strong>
                  <small>{displayLabel(item.evidenceType)} · v{item.version}</small>
                  <time>{formatUpdatedAt(item.updatedAt)}</time>
                </button>
              ))}
            </div>
          )}
        </section>

        <div className="evidence-detail-stack">
          {detailLoading ? (
            <section className="panel"><p className="loading-copy">{t("common.loading")}…</p></section>
          ) : record ? (
            <>
              <form className="panel evidence-editor" onSubmit={handleUpdate}>
                <div className="panel-header evidence-editor-header">
                  <div>
                    <span className={`verification verification--${record.evidence.verificationStatus}`}>
                      {t(`evidence.${record.evidence.verificationStatus}`)}
                    </span>
                    <h2>{record.evidence.title}</h2>
                    <p>v{record.evidence.version} · {t("common.updatedAt")} {formatUpdatedAt(record.evidence.updatedAt)}</p>
                  </div>
                  <div className="evidence-status-actions">
                    <button
                      className="secondary-button"
                      disabled={saving || archived}
                      onClick={() => void handleStatus("verify")}
                      type="button"
                    >{t("evidence.verify")}</button>
                    <button
                      className="danger-button"
                      disabled={saving || archived}
                      onClick={() => void handleStatus("invalidate")}
                      type="button"
                    >{t("evidence.invalidate")}</button>
                  </div>
                </div>
                <fieldset className="editor-fields" disabled={archived || gitEvidence}>
                  <EvidenceFields value={draft} onChange={setDraft} />
                </fieldset>
                <div className="form-actions evidence-form-actions">
                  <span>{gitEvidence ? t("evidence.readOnlySource") : t("evidence.saveReverify")}</span>
                  <button className="primary-button" disabled={saving || archived || gitEvidence} type="submit">
                    {saving ? t("common.saving") : t("evidence.saveChanges")}
                  </button>
                </div>
              </form>

              <section className="panel evidence-links-panel">
                <div className="panel-header">
                  <div><h2>{t("evidence.link")}</h2><p>{t("evidence.linkQuestion")}</p></div>
                  <span className="history-count">{t("evidence.linkCount", { count: record.links.length })}</span>
                </div>
                {record.links.length === 0 ? (
                  <p className="empty-state">{t("evidence.noLinks")}</p>
                ) : (
                  <div className="evidence-links-list">
                    {record.links.map((link) => (
                      <div key={link.id}>
                        <span>{displayLabel(link.relationType)}</span>
                        <strong>{link.targetType === "project" ? t("navigation.projects") : link.targetType === "snapshot" ? t("snapshot.title") : t("session.title")}</strong>
                        <code>{link.targetId}</code>
                      </div>
                    ))}
                  </div>
                )}
                {!archived ? (
                  <form className="evidence-link-form" onSubmit={handleLink}>
                    <label>{t("evidence.targetType")}
                      <select value={targetType} onChange={(event) => setTargetType(event.target.value as EvidenceTargetType)}>
                        {EVIDENCE_TARGET_TYPES.map((value) => <option key={value} value={value}>{value === "project" ? t("navigation.projects") : value === "snapshot" ? t("snapshot.title") : t("session.title")}</option>)}
                      </select>
                    </label>
                    <label>{t("evidence.target")}
                      <select required value={targetId} onChange={(event) => setTargetId(event.target.value)}>
                        {targetOptions.length === 0 ? <option value="">{t("evidence.noTarget")}</option> : null}
                        {targetOptions.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
                      </select>
                    </label>
                    <label>{t("evidence.relation")}
                      <select value={relationType} onChange={(event) => setRelationType(event.target.value as EvidenceRelationType)}>
                        {EVIDENCE_RELATION_TYPES.map((value) => <option key={value} value={value}>{value === "supports" ? t("evidence.supports") : t("evidence.relatesTo")}</option>)}
                      </select>
                    </label>
                    <button className="secondary-button" disabled={saving || !targetId} type="submit">{t("evidence.addLink")}</button>
                  </form>
                ) : null}
              </section>
            </>
          ) : (
            <section className="panel evidence-detail-empty">
              <strong>{t("evidence.selectTitle")}</strong>
              <p>{t("evidence.selectDescription")}</p>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}

type EvidenceFieldsProps = {
  value: EvidenceContentInput;
  onChange: (value: EvidenceContentInput) => void;
};

function EvidenceFields({ value, onChange }: EvidenceFieldsProps) {
  const { t } = useTranslation();
  const sourceRequired = value.sourceType !== "manual_note";
  return (
    <div className="form-grid evidence-fields">
      <label className="span-2">{t("common.title")} <span className="required">*</span>
        <input required maxLength={200} value={value.title} onChange={(event) => onChange({ ...value, title: event.target.value })} />
      </label>
      <label>{t("evidence.type")}
        <select value={value.evidenceType} onChange={(event) => onChange({ ...value, evidenceType: event.target.value as EvidenceType })}>
          {MANUAL_EVIDENCE_TYPES.map((type) => <option key={type} value={type}>{displayLabel(type)}</option>)}
        </select>
      </label>
      <label>{t("evidence.source")}
        <select value={value.sourceType} onChange={(event) => onChange({ ...value, sourceType: event.target.value as EvidenceSourceType, sourceUri: event.target.value === "manual_note" ? "" : value.sourceUri })}>
          {EVIDENCE_SOURCE_TYPES.filter((type) => type !== "github_api").map((type) => <option key={type} value={type}>{displayLabel(type)}</option>)}
        </select>
      </label>
      <label className="span-2">{t("common.description")}
        <textarea rows={5} value={value.description} onChange={(event) => onChange({ ...value, description: event.target.value })} />
      </label>
      <label className="span-2">{t("evidence.source")} {sourceRequired ? <span className="required">*</span> : null}
        <input
          disabled={!sourceRequired}
          required={sourceRequired}
          value={value.sourceUri}
          onChange={(event) => onChange({ ...value, sourceUri: event.target.value })}
          placeholder={value.sourceType === "external_url" ? "https://…" : value.sourceType === "local_file" ? "D:\\project\\artifact.png" : t("evidence.noExternalLocation")}
        />
        <small>{t("evidence.sourceStorageHelp")}</small>
      </label>
    </div>
  );
}

function contentFrom(evidence: Evidence): EvidenceContentInput {
  return {
    title: evidence.title,
    description: evidence.description,
    evidenceType: evidence.evidenceType,
    sourceType: evidence.sourceType,
    sourceUri: evidence.sourceUri,
  };
}

function normalized(input: EvidenceContentInput): EvidenceContentInput {
  return {
    ...input,
    title: input.title.trim(),
    description: input.description.trim(),
    sourceUri: input.sourceUri.trim(),
  };
}
