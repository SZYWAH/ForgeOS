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
          <p className="eyebrow">工作空间总览</p>
          <h1>仪表盘</h1>
          <p>查看当前 Workspace 中的软件项目身份与生命周期状态。</p>
        </div>
        <Link className="primary-button button-link" to="/projects">
          管理项目
        </Link>
      </header>

      <section className="metrics" aria-label="项目汇总">
        <article>
          <span>项目总数</span>
          <strong>{state.kind === "loading" ? "—" : projects.length}</strong>
        </article>
        <article>
          <span>进行中</span>
          <strong>{state.kind === "loading" ? "—" : activeCount}</strong>
        </article>
        <article>
          <span>已归档</span>
          <strong>{state.kind === "loading" ? "—" : archivedCount}</strong>
        </article>
      </section>

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>当前项目</h2>
            <p>按最近更新时间查看项目。</p>
          </div>
        </div>

        {state.kind === "loading" ? <p className="loading-copy">正在加载项目…</p> : null}
        {state.kind === "error" ? (
          <div className="error-banner">{state.message}</div>
        ) : null}
        {state.kind === "ready" ? (
          <ProjectTable
            projects={state.projects}
            emptyMessage="尚未创建项目。前往项目页面建立第一个项目身份。"
          />
        ) : null}
      </section>
    </div>
  );
}
