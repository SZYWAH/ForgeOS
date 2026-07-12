import { useCallback, useEffect, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { evidenceKernelApi } from "../api/evidence-kernel";
import { getErrorMessage } from "../api/project-kernel";
import type { Evidence } from "../types/evidence";
import { formatUpdatedAt } from "../utils/format";

type EvidenceSummaryCardProps = {
  projectId: string;
  projectName: string;
};

export function EvidenceSummaryCard({
  projectId,
  projectName,
}: EvidenceSummaryCardProps) {
  const [items, setItems] = useState<Evidence[]>();
  const [error, setError] = useState<string>();
  const loadGeneration = useRef(0);

  const load = useCallback(async () => {
    const generation = ++loadGeneration.current;
    setItems(undefined);
    setError(undefined);
    try {
      const value = await evidenceKernelApi.listEvidences(projectId);
      if (generation === loadGeneration.current) {
        setItems(value);
      }
    } catch (loadError) {
      if (generation === loadGeneration.current) {
        setError(getErrorMessage(loadError));
      }
    }
  }, [projectId]);

  useEffect(() => {
    void load();
    return () => {
      loadGeneration.current += 1;
    };
  }, [load]);

  const verified = items?.filter(
    (item) => item.verificationStatus === "verified",
  ).length;

  return (
    <section className="panel evidence-summary-card">
      <div className="panel-header evidence-summary-header">
        <div>
          <p className="evidence-kicker">Evidence</p>
          <h2>Engineering Evidence</h2>
          <p>保存支持 {projectName} 研发事实的文档、测试结果和人工记录。</p>
        </div>
        {items ? (
          <Link
            className="secondary-button button-link"
            to={`/projects/${projectId}/evidence`}
          >
            Manage Evidence
          </Link>
        ) : null}
      </div>

      {error ? (
        <div className="load-error-state">
          <div className="error-banner">{error}</div>
          <button className="secondary-button" onClick={() => void load()} type="button">
            Retry
          </button>
        </div>
      ) : items === undefined ? (
        <p className="loading-copy">Loading evidence…</p>
      ) : items.length === 0 ? (
        <div className="evidence-empty-state">
          <div>
            <strong>还没有研发证据</strong>
            <p>添加文档、截图路径、链接、测试结果或人工记录。</p>
          </div>
          <Link className="primary-button button-link" to={`/projects/${projectId}/evidence`}>
            Add Evidence
          </Link>
        </div>
      ) : (
        <div className="evidence-summary-body">
          <dl className="evidence-metrics">
            <div>
              <dt>Total</dt>
              <dd>{items.length}</dd>
            </div>
            <div>
              <dt>Verified</dt>
              <dd>{verified}</dd>
            </div>
            <div>
              <dt>Needs review</dt>
              <dd>{items.length - (verified ?? 0)}</dd>
            </div>
          </dl>
          <div className="recent-evidence-list">
            {items.slice(0, 3).map((item) => (
              <Link key={item.id} to={`/projects/${projectId}/evidence?selected=${item.id}`}>
                <span className={`verification verification--${item.verificationStatus}`}>
                  {item.verificationStatus}
                </span>
                <strong>{item.title}</strong>
                <small>{formatUpdatedAt(item.updatedAt)}</small>
              </Link>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}
