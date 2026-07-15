import type { ReactNode } from "react";
import { useAppPreferences } from "../../contexts/AppPreferencesProvider";

export function TechnicalTerm({ label, term }: { label: string; term: string }) {
  const { preferences, locale } = useAppPreferences();
  if (locale !== "zh-CN" || !preferences.showTechnicalTerms) return <>{label}</>;
  return <>{label}<span className="technical-term">（{term}）</span></>;
}

export function PageIntro({ title, description, primaryAction, secondaryActions, technicalTerm }: {
  title: string;
  description: string;
  primaryAction?: ReactNode;
  secondaryActions?: ReactNode;
  technicalTerm?: string;
}) {
  return <header className="page-intro">
    <div className="page-intro__copy">
      <h1>{technicalTerm ? <TechnicalTerm label={title} term={technicalTerm} /> : title}</h1>
      <p>{description}</p>
    </div>
    {(primaryAction || secondaryActions) && <div className="page-intro__actions">{primaryAction}{secondaryActions}</div>}
  </header>;
}

export function EmptyState({ title, description, action }: { title: string; description: string; action?: ReactNode }) {
  return <div className="plain-empty-state"><strong>{title}</strong><p>{description}</p>{action}</div>;
}

export function TechnicalDetails({ label, children }: { label: string; children: ReactNode }) {
  return <details className="technical-details"><summary>{label}</summary><div>{children}</div></details>;
}
