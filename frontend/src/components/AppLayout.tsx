import { NavLink, Outlet } from "react-router-dom";
import type { Workspace } from "../types/project";

type AppLayoutProps = {
  workspace: Workspace;
};

export function AppLayout({ workspace }: AppLayoutProps) {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-block">
          <span className="brand-mark">F</span>
          <div>
            <strong>ForgeOS</strong>
            <span>D1.1 Project Kernel</span>
          </div>
        </div>

        <nav className="main-nav" aria-label="Main navigation">
          <NavLink to="/" end>
            Dashboard
          </NavLink>
          <NavLink to="/projects">Projects</NavLink>
        </nav>

        <div className="workspace-card">
          <span>Workspace</span>
          <strong>{workspace.name}</strong>
          <small>{workspace.timezone}</small>
        </div>
      </aside>

      <div className="app-main">
        <header className="topbar">
          <span>Personal Engineering Workspace</span>
          <span className="kernel-state">Project Kernel · Local</span>
        </header>
        <main className="content">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
