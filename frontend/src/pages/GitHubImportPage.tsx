import { type FormEvent, useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { githubApi } from "../api/github";
import { getErrorMessage } from "../api/project-kernel";
import type { GitHubConnector, GitHubImportDraft, GitHubImportStatus } from "../types/github";
import { formatUpdatedAt } from "../utils/format";

const statusLabels: Record<GitHubImportStatus, string> = {
  draft: "待审核",
  reviewed: "已审核",
  imported: "已导入",
  rejected: "已拒绝",
};

export function GitHubImportPage() {
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
      setNotice("已读取 GitHub 事实并生成导入草稿；尚未创建项目。\n");
    });
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div><p className="eyebrow">D4.5 · 只读导入</p><h1>GitHub 项目导入</h1><p>先读取、生成草稿、人工审核，再创建 ForgeOS 项目与研发证据。</p></div>
      </header>

      <section className="panel github-boundary">
        <strong>只读边界</strong><span>不会创建 Issue、修改 PR、提交代码、后台同步或向 GitHub 写入任何内容。</span>
      </section>

      <section className="panel github-auth-panel">
        <div className="panel-header"><div><h2>GitHub 连接器</h2><p>{connector?.tokenConfigured ? "Token 已保存在 Windows Credential Manager；不会进入 SQLite 或 IPC 返回。" : "未认证模式仅适用于公开仓库。"}</p></div><span className={`provider-state ${connector?.tokenConfigured ? "provider-state--enabled" : "provider-state--disabled"}`}>{connector?.tokenConfigured ? "Token 已配置" : "仅公开仓库"}</span></div>
        <div className="github-token-row">
          <input type="password" autoComplete="off" value={token} onChange={(event) => setToken(event.target.value)} placeholder="GitHub fine-grained token（可选）" />
          <button className="secondary-button" disabled={busy || !token.trim()} type="button" onClick={() => void run(async () => { setConnector(await githubApi.setToken(token)); setToken(""); setNotice("Token 已安全保存。"); })}>保存 Token</button>
          <button className="secondary-button" disabled={busy || !connector?.tokenConfigured} type="button" onClick={() => void run(async () => { setConnector(await githubApi.clearToken()); setNotice("Token 已从安全存储清除。"); })}>清除</button>
        </div>
      </section>

      <section className="panel">
        <div className="panel-header"><div><h2>读取 GitHub 仓库</h2><p>API 全部成功后才保存导入草稿；失败不会污染数据库。</p></div></div>
        <form className="github-url-form" onSubmit={handlePreview}>
          <input required type="url" value={repositoryUrl} onChange={(event) => setRepositoryUrl(event.target.value)} placeholder="https://github.com/owner/repository" />
          <button className="primary-button" disabled={busy} type="submit">{busy ? "读取中…" : "生成导入预览"}</button>
        </form>
      </section>

      {error ? <div className="error-banner">{error}</div> : null}
      {notice ? <div className="success-banner">{notice}</div> : null}

      <div className="github-import-workbench">
        <section className="panel github-draft-list">
          <div className="panel-header"><div><h2>导入草稿</h2><p>{drafts.length} 条历史记录</p></div></div>
          {drafts.map((draft) => <button className={draft.id === selected?.id ? "selected" : ""} key={draft.id} onClick={() => setSelectedId(draft.id)} type="button"><strong>{draft.owner}/{draft.name}</strong><span className={`candidate-status candidate-status--${draft.status === "reviewed" ? "accepted" : draft.status === "imported" ? "accepted" : draft.status === "rejected" ? "rejected" : "pending"}`}>{statusLabels[draft.status]}</span><small>{formatUpdatedAt(draft.updatedAt)}</small></button>)}
          {!drafts.length ? <p className="empty-state">尚未生成导入草稿。</p> : null}
        </section>

        <section className="panel github-preview-panel">
          {selected ? <>
            <div className="panel-header"><div><p className="snapshot-kicker">导入预览</p><h2>{selected.owner}/{selected.name}</h2><p>{selected.description || "仓库没有提供描述。"}</p></div><a className="secondary-button button-link" href={selected.url} target="_blank" rel="noreferrer">打开 GitHub</a></div>
            <div className="github-preview-meta"><div><span>默认分支</span><strong>{selected.defaultBranch}</strong></div><div><span>主要语言</span><strong>{selected.language ?? "未识别"}</strong></div><div><span>主题标签</span><strong>{selected.topics.join(", ") || "无"}</strong></div><div><span>提交 / 发布版本</span><strong>{selected.recentCommits.length} / {selected.latestRelease ? "1" : "0"}</strong></div></div>
            <article className="github-readme-preview"><h3>README 摘要</h3><pre>{selected.readmeSummary || "未找到 README。"}</pre></article>
            <div className="github-review-actions">
              <span>状态：{statusLabels[selected.status]} · v{selected.version}</span>
              <div>
                {selected.status === "draft" ? <button className="secondary-button" disabled={busy} onClick={() => void run(async () => { replaceDraft(await githubApi.review(selected.id, selected.version)); setNotice("已人工确认预览；现在可以导入项目。"); })} type="button">确认预览</button> : null}
                {selected.status === "reviewed" ? <button className="primary-button" disabled={busy} onClick={() => void run(async () => { const result = await githubApi.importProject(selected.id, selected.version); replaceDraft(result.draft); navigate(`/projects/${result.project.id}`); })} type="button">导入项目</button> : null}
                {(selected.status === "draft" || selected.status === "reviewed") ? <button className="danger-button" disabled={busy} onClick={() => void run(async () => { replaceDraft(await githubApi.reject(selected.id, selected.version)); setNotice("导入草稿已拒绝；没有创建项目。"); })} type="button">拒绝</button> : null}
                {selected.importedProjectId ? <button className="secondary-button" onClick={() => navigate(`/projects/${selected.importedProjectId}`)} type="button">打开项目</button> : null}
              </div>
            </div>
          </> : <div className="provider-detail-empty"><strong>选择或创建导入草稿</strong><p>GitHub 数据不会自动进入正式事实层。</p></div>}
        </section>
      </div>
    </div>
  );
}
