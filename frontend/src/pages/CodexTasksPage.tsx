import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { codexTaskProposalApi } from "../api/codex-task-proposal";
import { getErrorMessage } from "../api/project-kernel";
import { BoundedList, CollapsibleText } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { CodexTaskProposal } from "../types/codex-task-proposal";
import { displayLabel, formatUpdatedAt } from "../utils/format";

export function CodexTasksPage() { const { t } = useTranslation(); const { projectId } = useParams(); const [items, setItems] = useState<CodexTaskProposal[]>([]); const [error, setError] = useState<string>(); useEffect(() => { if (projectId) void codexTaskProposalApi.list(projectId).then(setItems).catch((reason) => setError(getErrorMessage(reason))); }, [projectId]); return <div className="page-stack frozen-page"><Link to={projectId ? `/projects/${projectId}` : "/projects"}>← {t("navigation.overview")}</Link><PageIntro title={t("navigation.codexTasks")} description={t("memory.frozenFeatureDescription")} /><div className="archive-banner"><strong>{t("memory.frozenFeature")}</strong><span>{t("memory.frozenFeatureDescription")}</span></div>{error ? <div className="error-banner">{error}</div> : null}<div className="frozen-record-list">{items.map((item) => <article key={item.id}><div><span>{displayLabel(item.status)}</span><time>{formatUpdatedAt(item.createdAt)}</time></div><h2>{item.title}</h2><CollapsibleText collapsedLines={3} value={item.goal} /><BoundedList initialCount={3} items={item.acceptanceCriteria} /></article>)}</div></div>; }
