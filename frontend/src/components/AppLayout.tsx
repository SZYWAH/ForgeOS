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
            <span>D4.5 · GitHub 只读导入</span>
          </div>
        </div>

        <nav className="main-nav" aria-label="主导航">
          <NavLink to="/" end>
            总览
          </NavLink>
          <NavLink to="/projects">项目</NavLink>
          <NavLink to="/github-import">GitHub 导入</NavLink>
          <NavLink to="/settings/ai-providers">AI 服务配置</NavLink>
        </nav>

        <div className="workspace-card">
          <span>工作空间</span>
          <strong>{workspace.name}</strong>
          <small>{workspace.timezone}</small>
        </div>
      </aside>

      <div className="app-main">
        <header className="topbar">
          <span>个人软件研发工作空间</span>
          <span className="kernel-state">研发记忆 + 只读 Git · 本地运行</span>
        </header>
        <main className="content">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
