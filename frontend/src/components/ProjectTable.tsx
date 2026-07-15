import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  if (projects.length === 0) {
    return <div className="empty-state">{emptyMessage}</div>;
  }

  return (
    <div className="table-frame">
      <table className="project-table">
        <thead>
          <tr>
            <th>{t("navigation.projects")}</th>
            <th>{t("common.status")}</th>
            <th>{t("projects.lifecycle")}</th>
            <th>{t("projects.priority")}</th>
            <th>{t("common.updatedAt")}</th>
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
