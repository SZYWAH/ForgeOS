import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { getErrorMessage, projectKernelApi } from "../api/project-kernel";
import { ProjectTable } from "../components/ProjectTable";
import type { ProjectSummary } from "../types/project";

type ProjectsState =
  | { kind: "loading" }
  | { kind: "ready"; projects: ProjectSummary[] }
  | { kind: "error"; message: string };

export function DashboardPage() {
  const [state, setState] = useState<ProjectsState>({ kind: "loading" });

  useEffect(() => {
    projectKernelApi
      .listProjects()
      .then((projects) => setState({ kind: "ready", projects }))
      .catch((error: unknown) =>
        setState({ kind: "error", message: getErrorMessage(error) }),
      );
  }, []);

  const projects = state.kind === "ready" ? state.projects : [];
  const activeCount = projects.filter((project) => project.status === "active").length;
  const archivedCount = projects.filter(
    (project) => project.status === "archived",
  ).length;

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">Workspace Overview</p>
          <h1>Dashboard</h1>
          <p>查看当前 Workspace 中的软件项目身份与生命周期状态。</p>
        </div>
        <Link className="primary-button button-link" to="/projects">
          Manage Projects
        </Link>
      </header>

      <section className="metrics" aria-label="Project summary">
        <article>
          <span>Total Projects</span>
          <strong>{state.kind === "loading" ? "—" : projects.length}</strong>
        </article>
        <article>
          <span>Active</span>
          <strong>{state.kind === "loading" ? "—" : activeCount}</strong>
        </article>
        <article>
          <span>Archived</span>
          <strong>{state.kind === "loading" ? "—" : archivedCount}</strong>
        </article>
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>Current Projects</h2>
            <p>按最近更新时间查看项目。</p>
          </div>
        </div>

        {state.kind === "loading" ? <p className="loading-copy">Loading projects…</p> : null}
        {state.kind === "error" ? (
          <div className="error-banner">{state.message}</div>
        ) : null}
        {state.kind === "ready" ? (
          <ProjectTable
            projects={state.projects}
            emptyMessage="尚未创建项目。前往 Projects 建立第一个项目身份。"
          />
        ) : null}
      </section>
    </div>
  );
}
