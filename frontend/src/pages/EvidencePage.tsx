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
import { formatUpdatedAt } from "../utils/format";

const EMPTY_EVIDENCE: EvidenceContentInput = {
  title: "",
  description: "",
  evidenceType: "document",
  sourceType: "local_file",
  sourceUri: "",
};

const TYPE_LABELS: Record<EvidenceType, string> = {
  document: "文档",
  screenshot: "截图",
  link: "链接",
  test_result: "测试结果",
  note: "人工记录",
  artifact: "构建产物",
  git_commit: "Git 提交",
  git_status: "Git 状态",
  github_repository: "GitHub 仓库",
  github_commit: "GitHub 提交",
  github_release: "GitHub 发布版本",
};

const SOURCE_LABELS: Record<EvidenceSourceType, string> = {
  local_file: "本地文件路径",
  external_url: "外部 URL",
  manual_note: "人工记录",
  github_api: "GitHub API",
};

export function EvidencePage() {
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
      setError("项目 ID 缺失。");
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
        label: `快照 v${snapshot.version} · ${snapshot.currentGoal}`,
      }));
    }
    return sessions.map((session) => ({
      id: session.id,
      label: `${session.status} · ${session.goal}`,
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
      setNotice("证据已创建，当前状态为未验证。");
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
      setNotice("证据内容已更新；验证状态已重置为未验证。");
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
      setNotice(action === "verify" ? "证据已人工确认。" : "证据已标记为无效。");
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
      setNotice("证据关联已保存。");
    } catch (saveError) {
      setError(getErrorMessage(saveError));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return <p className="loading-copy">正在加载证据模块…</p>;
  }

  if (!project || !projectId) {
    return (
      <section className="panel not-found">
        <h1>项目不可用</h1>
        <p>{error ?? "找不到该项目。"}</p>
        <Link to="/projects">返回项目列表</Link>
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
        <Link to="/projects">项目</Link><span>/</span>
        <Link to={`/projects/${project.id}`}>{project.name}</Link><span>/</span>
        <span>研发证据</span>
      </div>

      <header className="page-header evidence-page-header">
        <div>
          <p className="evidence-kicker">D1.4 证据核心</p>
          <h1>研发证据</h1>
          <p>记录支持项目状态、研发过程与成果判断的人工证据。</p>
        </div>
        <button
          className="primary-button"
          disabled={archived}
          onClick={() => setShowCreate((value) => !value)}
          type="button"
        >
          {showCreate ? "取消" : "添加证据"}
        </button>
      </header>

      {archived ? <div className="archive-banner">该项目已归档，研发证据仅可查看。</div> : null}
      {error ? <div className="error-banner evidence-message">{error}</div> : null}
      {notice ? <div className="success-banner evidence-message">{notice}</div> : null}

      {showCreate ? (
        <form className="panel evidence-create-form" onSubmit={handleCreate}>
          <div className="panel-header">
            <div><h2>新建证据</h2><p>来源由用户提供，ForgeOS 不自动采集或验证。</p></div>
          </div>
          <EvidenceFields value={createDraft} onChange={setCreateDraft} />
          <div className="form-actions evidence-form-actions">
            <button className="primary-button" disabled={saving} type="submit">
              {saving ? "正在保存…" : "创建证据"}
            </button>
          </div>
        </form>
      ) : null}

      <div className="evidence-workbench">
        <section className="panel evidence-list-panel">
          <div className="panel-header">
            <div><h2>项目证据</h2><p>{items.length} 条持久记录</p></div>
          </div>
          {items.length === 0 ? (
            <p className="empty-state">当前项目还没有研发证据。</p>
          ) : (
            <div className="evidence-list">
              {items.map((item) => (
                <button
                  className={item.id === selectedId ? "selected" : ""}
                  key={item.id}
                  onClick={() => setSearchParams({ selected: item.id })}
                  type="button"
                >
                  <span className={`verification verification--${item.verificationStatus}`}>
                    {item.verificationStatus === "verified" ? "已验证" : item.verificationStatus === "invalid" ? "已失效" : "未验证"}
                  </span>
                  <strong>{item.title}</strong>
                  <small>{TYPE_LABELS[item.evidenceType]} · v{item.version}</small>
                  <time>{formatUpdatedAt(item.updatedAt)}</time>
                </button>
              ))}
            </div>
          )}
        </section>

        <div className="evidence-detail-stack">
          {detailLoading ? (
            <section className="panel"><p className="loading-copy">正在加载证据…</p></section>
          ) : record ? (
            <>
              <form className="panel evidence-editor" onSubmit={handleUpdate}>
                <div className="panel-header evidence-editor-header">
                  <div>
                    <span className={`verification verification--${record.evidence.verificationStatus}`}>
                      {record.evidence.verificationStatus === "verified" ? "已验证" : record.evidence.verificationStatus === "invalid" ? "已失效" : "未验证"}
                    </span>
                    <h2>{record.evidence.title}</h2>
                    <p>v{record.evidence.version} · 更新于 {formatUpdatedAt(record.evidence.updatedAt)}</p>
                  </div>
                  <div className="evidence-status-actions">
                    <button
                      className="secondary-button"
                      disabled={saving || archived}
                      onClick={() => void handleStatus("verify")}
                      type="button"
                    >验证</button>
                    <button
                      className="danger-button"
                      disabled={saving || archived}
                      onClick={() => void handleStatus("invalidate")}
                      type="button"
                    >标记失效</button>
                  </div>
                </div>
                <fieldset className="editor-fields" disabled={archived || gitEvidence}>
                  <EvidenceFields value={draft} onChange={setDraft} />
                </fieldset>
                <div className="form-actions evidence-form-actions">
                  <span>{gitEvidence ? "Git/GitHub 证据内容来自只读采集，不能手动改写。" : "保存内容后必须重新人工验证。"}</span>
                  <button className="primary-button" disabled={saving || archived || gitEvidence} type="submit">
                    {saving ? "正在保存…" : "保存修改"}
                  </button>
                </div>
              </form>

              <section className="panel evidence-links-panel">
                <div className="panel-header">
                  <div><h2>证据关联</h2><p>统一关联项目、快照或开发会话。</p></div>
                  <span className="history-count">{record.links.length} 条关联</span>
                </div>
                {record.links.length === 0 ? (
                  <p className="empty-state">尚未建立显式关联；证据仍归属于当前项目。</p>
                ) : (
                  <div className="evidence-links-list">
                    {record.links.map((link) => (
                      <div key={link.id}>
                        <span>{link.relationType}</span>
                        <strong>{link.targetType}</strong>
                        <code>{link.targetId}</code>
                      </div>
                    ))}
                  </div>
                )}
                {!archived ? (
                  <form className="evidence-link-form" onSubmit={handleLink}>
                    <label>目标类型
                      <select value={targetType} onChange={(event) => setTargetType(event.target.value as EvidenceTargetType)}>
                        {EVIDENCE_TARGET_TYPES.map((value) => <option key={value} value={value}>{value === "project" ? "项目" : value === "snapshot" ? "快照" : "开发会话"}</option>)}
                      </select>
                    </label>
                    <label>目标
                      <select required value={targetId} onChange={(event) => setTargetId(event.target.value)}>
                        {targetOptions.length === 0 ? <option value="">没有可用目标</option> : null}
                        {targetOptions.map((option) => <option key={option.id} value={option.id}>{option.label}</option>)}
                      </select>
                    </label>
                    <label>关系
                      <select value={relationType} onChange={(event) => setRelationType(event.target.value as EvidenceRelationType)}>
                        {EVIDENCE_RELATION_TYPES.map((value) => <option key={value} value={value}>{value === "supports" ? "支持" : "相关"}</option>)}
                      </select>
                    </label>
                    <button className="secondary-button" disabled={saving || !targetId} type="submit">添加关联</button>
                  </form>
                ) : null}
              </section>
            </>
          ) : (
            <section className="panel evidence-detail-empty">
              <strong>选择一条证据</strong>
              <p>查看来源、验证状态和与研发事实的关联。</p>
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
  const sourceRequired = value.sourceType !== "manual_note";
  return (
    <div className="form-grid evidence-fields">
      <label className="span-2">标题 <span className="required">*</span>
        <input required maxLength={200} value={value.title} onChange={(event) => onChange({ ...value, title: event.target.value })} />
      </label>
      <label>类型
        <select value={value.evidenceType} onChange={(event) => onChange({ ...value, evidenceType: event.target.value as EvidenceType })}>
          {MANUAL_EVIDENCE_TYPES.map((type) => <option key={type} value={type}>{TYPE_LABELS[type]}</option>)}
        </select>
      </label>
      <label>来源类型
        <select value={value.sourceType} onChange={(event) => onChange({ ...value, sourceType: event.target.value as EvidenceSourceType, sourceUri: event.target.value === "manual_note" ? "" : value.sourceUri })}>
          {EVIDENCE_SOURCE_TYPES.filter((type) => type !== "github_api").map((type) => <option key={type} value={type}>{SOURCE_LABELS[type]}</option>)}
        </select>
      </label>
      <label className="span-2">描述
        <textarea rows={5} value={value.description} onChange={(event) => onChange({ ...value, description: event.target.value })} />
      </label>
      <label className="span-2">来源 {sourceRequired ? <span className="required">*</span> : null}
        <input
          disabled={!sourceRequired}
          required={sourceRequired}
          value={value.sourceUri}
          onChange={(event) => onChange({ ...value, sourceUri: event.target.value })}
          placeholder={value.sourceType === "external_url" ? "https://…" : value.sourceType === "local_file" ? "D:\\project\\artifact.png" : "人工记录没有外部位置"}
        />
        <small>ForgeOS 仅保存路径或 URL，不复制文件，也不自动验证内容。</small>
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
