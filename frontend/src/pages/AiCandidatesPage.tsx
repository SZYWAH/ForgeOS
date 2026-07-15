import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { aiCandidateApi } from "../api/ai-candidate";
import { getErrorMessage } from "../api/project-kernel";
import { CollapsibleText } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { AiCandidate } from "../types/ai-candidate";
import { displayLabel, formatUpdatedAt } from "../utils/format";

export function AiCandidatesPage() {
  const { t } = useTranslation(); const { projectId } = useParams(); const [items, setItems] = useState<AiCandidate[]>([]); const [error, setError] = useState<string>();
  useEffect(() => { if (projectId) void aiCandidateApi.list(projectId).then(setItems).catch((reason) => setError(getErrorMessage(reason))); }, [projectId]);
  return <div className="page-stack frozen-page"><Link to={projectId ? `/projects/${projectId}/context` : "/projects"}>← {t("memory.aiContext")}</Link><PageIntro title={t("navigation.aiSuggestions")} description={t("memory.frozenFeatureDescription")} /><div className="archive-banner"><strong>{t("memory.frozenFeature")}</strong><span>{t("memory.frozenFeatureDescription")}</span></div>{error ? <div className="error-banner">{error}</div> : null}<div className="frozen-record-list">{items.map((item) => <article key={item.id}><div><span>{displayLabel(item.status)}</span><time>{formatUpdatedAt(item.createdAt)}</time></div><h2>{item.title}</h2><CollapsibleText collapsedLines={5} value={item.contentMarkdown} /></article>)}</div></div>;
}
