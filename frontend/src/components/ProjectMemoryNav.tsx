import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";

export function ProjectMemoryNav({ projectId }: { projectId: string }) {
  const { t } = useTranslation();
  const items = [
    { to: `/projects/${projectId}`, label: t("navigation.overview"), end: true },
    { to: `/projects/${projectId}/sessions`, label: t("memory.historyTitle") },
    { to: `/projects/${projectId}/evidence`, label: t("memory.achievementsAndMaterials") },
    { to: `/projects/${projectId}/context`, label: t("memory.aiContext") },
  ];

  return (
    <nav className="project-tabs project-memory-nav" aria-label={t("memory.projectPages")}>
      <div className="project-tabs__scroll">
        <div className="project-tabs__track">
          {items.map((item) => (
            <NavLink
              className={({ isActive }) => (isActive ? "active" : undefined)}
              end={item.end}
              key={item.to}
              to={item.to}
            >
              {item.label}
            </NavLink>
          ))}
        </div>
      </div>
    </nav>
  );
}
