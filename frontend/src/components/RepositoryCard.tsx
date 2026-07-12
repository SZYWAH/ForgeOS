import { useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { repositoryApi } from "../api/repository";
import type { RepositoryRecord } from "../types/repository";
import { formatUpdatedAt } from "../utils/format";

type RepositoryCardProps = {
  projectId: string;
  archived: boolean;
};

export function RepositoryCard({ projectId, archived }: RepositoryCardProps) {
  const [record, setRecord] = useState<RepositoryRecord>();
  const [count, setCount] = useState(0);
  const [error, setError] = useState<string>();
  const generation = useRef(0);

  useEffect(() => {
    const current = ++generation.current;
    setRecord(undefined);
    setError(undefined);
    void repositoryApi.list(projectId).then((items) => {
      if (current !== generation.current) return;
      setCount(items.length);
      setRecord(items[0]);
    }).catch(() => {
      if (current === generation.current) setError("Repository 状态暂时不可用。");
    });
    return () => { generation.current += 1; };
  }, [projectId]);

  return (
    <section className="panel repository-card">
      <div className="panel-header repository-card-header">
        <div>
          <p className="snapshot-kicker">Read-only Git</p>
          <h2>Repository</h2>
          <p>{count > 0 ? `${count} local repositories registered` : "关联本地 Git 工作区并生成外部事实证据。"}</p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${projectId}/repositories`}>
          {record ? "Open Repository" : archived ? "View Repositories" : "Add Repository"}
        </Link>
      </div>
      {error ? <div className="error-banner repository-card-message">{error}</div> : record ? (
        <div className="repository-card-body">
          <div><span>Path</span><strong title={record.repository.path}>{record.repository.path}</strong></div>
          <div><span>Branch</span><strong>{record.workingCopy.branch}</strong></div>
          <div><span>HEAD</span><strong>{record.workingCopy.headCommit.slice(0, 12)}</strong></div>
          <div><span>Status</span><strong className={record.workingCopy.isDirty ? "git-dirty" : "git-clean"}>{record.workingCopy.isDirty ? "Dirty" : "Clean"}</strong></div>
          <div><span>Checked</span><strong>{formatUpdatedAt(record.workingCopy.checkedAt)}</strong></div>
        </div>
      ) : <p className="empty-state">尚未注册本地 Git Repository。</p>}
    </section>
  );
}
