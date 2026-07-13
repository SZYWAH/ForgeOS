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
      if (current === generation.current) setError("本地仓库状态暂时不可用。");
    });
    return () => { generation.current += 1; };
  }, [projectId]);

  return (
    <section className="panel repository-card">
      <div className="panel-header repository-card-header">
        <div>
          <p className="snapshot-kicker">只读 Git</p>
          <h2>本地代码仓库</h2>
          <p>{count > 0 ? `已注册 ${count} 个本地仓库` : "关联本地 Git 工作区并生成外部事实证据。"}</p>
        </div>
        <Link className="secondary-button button-link" to={`/projects/${projectId}/repositories`}>
          {record ? "打开仓库" : archived ? "查看仓库" : "添加仓库"}
        </Link>
      </div>
      {error ? <div className="error-banner repository-card-message">{error}</div> : record ? (
        <div className="repository-card-body">
          <div><span>路径</span><strong title={record.repository.path}>{record.repository.path}</strong></div>
          <div><span>分支</span><strong>{record.workingCopy.branch}</strong></div>
          <div><span>HEAD</span><strong>{record.workingCopy.headCommit.slice(0, 12)}</strong></div>
          <div><span>状态</span><strong className={record.workingCopy.isDirty ? "git-dirty" : "git-clean"}>{record.workingCopy.isDirty ? "有改动" : "干净"}</strong></div>
          <div><span>检查时间</span><strong>{formatUpdatedAt(record.workingCopy.checkedAt)}</strong></div>
        </div>
      ) : <p className="empty-state">尚未注册本地 Git 仓库。</p>}
    </section>
  );
}
