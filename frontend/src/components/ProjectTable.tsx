import { Link } from "react-router-dom";
import type { ProjectSummary } from "../types/project";
import {
  formatUpdatedAt,
  lifecycleLabel,
  statusLabel,
} from "../utils/format";

type ProjectTableProps = {
  projects: ProjectSummary[];
  emptyMessage: string;
};

export function ProjectTable({ projects, emptyMessage }: ProjectTableProps) {
  if (projects.length === 0) {
    return <div className="empty-state">{emptyMessage}</div>;
  }

  return (
    <div className="table-frame">
      <table className="project-table">
        <thead>
          <tr>
            <th>项目</th>
            <th>状态</th>
            <th>生命周期</th>
            <th>优先级</th>
            <th>更新时间</th>
          </tr>
        </thead>
        <tbody>
          {projects.map((project) => (
            <tr key={project.id}>
              <td>
                <Link className="project-link" to={`/projects/${project.id}`}>
                  {project.name}
                </Link>
                <span className="project-slug">{project.slug}</span>
              </td>
              <td>
                <span className={`status status--${project.status}`}>
                  {statusLabel(project.status)}
                </span>
              </td>
              <td>{lifecycleLabel(project.lifecycleStage)}</td>
              <td>{project.priority}</td>
              <td>{formatUpdatedAt(project.updatedAt)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
