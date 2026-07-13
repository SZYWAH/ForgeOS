import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { githubApi } from "../api/github";
import type { GitHubImportDraft } from "../types/github";
import { formatUpdatedAt } from "../utils/format";

export function GitHubRepositoryCard({ projectId }: { projectId: string }) {
  const [draft, setDraft] = useState<GitHubImportDraft | null>();

  useEffect(() => {
    let active = true;
    void githubApi.getProjectImport(projectId).then((value) => {
      if (active) setDraft(value);
    }).catch(() => { if (active) setDraft(null); });
    return () => { active = false; };
  }, [projectId]);

  return (
    <section className="panel github-project-card">
      <div className="panel-header">
        <div>
          <p className="snapshot-kicker">GitHub 外部事实</p>
          <h2>GitHub 仓库</h2>
          <p>{draft ? "该项目由人工审核后的 GitHub 导入草稿创建。" : "尚未关联 GitHub 导入来源。"}</p>
        </div>
        <Link className="secondary-button button-link" to="/github-import">GitHub 导入</Link>
      </div>
      {draft ? (
        <div className="github-card-grid">
          <div><span>仓库</span><a href={draft.url} target="_blank" rel="noreferrer">{draft.owner}/{draft.name}</a></div>
          <div><span>默认分支</span><strong>{draft.defaultBranch}</strong></div>
          <div><span>主要语言</span><strong>{draft.language ?? "未识别"}</strong></div>
          <div><span>最后导入</span><strong>{formatUpdatedAt(draft.updatedAt)}</strong></div>
        </div>
      ) : <p className="empty-state">项目仍可独立使用；GitHub 关联不是必需项。</p>}
    </section>
  );
}
