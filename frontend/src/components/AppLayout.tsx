import { NavLink, Outlet } from "react-router-dom";
import type { Workspace } from "../types/project";
import { useTranslation } from "react-i18next";

export function AppLayout({ workspace }: { workspace: Workspace }) {
  const { t } = useTranslation();
  return <div className="app-shell">
    <aside className="sidebar">
      <div className="brand-block"><span className="brand-mark">F</span><div><strong>ForgeOS</strong><span>Personal Engineering OS</span></div></div>
      <nav className="main-nav" aria-label={t("navigation.workspace")}>
        <span className="main-nav__group">{t("navigation.workspace")}</span>
        <NavLink end to="/">{t("navigation.dashboard")}</NavLink>
        <NavLink to="/projects">{t("navigation.projectList")}</NavLink>
        <NavLink to="/github-import">{t("navigation.importGithub")}</NavLink>
        <span className="main-nav__group">{t("navigation.ai")}</span>
        <NavLink to="/settings/ai-providers">{t("navigation.aiServices")}</NavLink>
        <span className="main-nav__group">{t("navigation.settings")}</span>
        <NavLink to="/settings">{t("navigation.appSettings")}</NavLink>
      </nav>
      <div className="workspace-card"><span>{t("navigation.workspace")}</span><strong>{workspace.name}</strong><small>{workspace.timezone}</small></div>
    </aside>
    <div className="app-main">
      <header className="topbar"><span>ForgeOS</span><span className="kernel-state">{t("navigation.topbar")}</span></header>
      <div className="content-scroll"><main className="content"><Outlet /></main></div>
    </div>
  </div>;
}
