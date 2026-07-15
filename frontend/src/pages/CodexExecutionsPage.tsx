import { useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { codexExecutionApi } from "../api/codex-execution";
import { getErrorMessage } from "../api/project-kernel";
import { CollapsibleText } from "../components/content/ContentGovernance";
import { PageIntro } from "../components/ui/PlainLanguage";
import type { CodexExecution } from "../types/codex-execution";
import { displayLabel, formatUpdatedAt } from "../utils/format";

export function CodexExecutionsPage() { const { t } = useTranslation(); const { projectId } = useParams(); const [items, setItems] = useState<CodexExecution[]>([]); const [error, setError] = useState<string>(); useEffect(() => { if (projectId) void codexExecutionApi.list(projectId).then(setItems).catch((reason) => setError(getErrorMessage(reason))); }, [projectId]); return <div className="page-stack frozen-page"><Link to={projectId ? `/projects/${projectId}` : "/projects"}>← {t("navigation.overview")}</Link><PageIntro title={t("navigation.executionRecords")} description={t("memory.frozenFeatureDescription")} /><div className="archive-banner"><strong>{t("memory.frozenFeature")}</strong><span>{t("memory.frozenFeatureDescription")}</span></div>{error ? <div className="error-banner">{error}</div> : null}<div className="frozen-record-list">{items.map((item) => <article key={item.id}><div><span>{displayLabel(item.status)}</span><time>{formatUpdatedAt(item.createdAt)}</time></div><h2>{item.id}</h2><CollapsibleText collapsedLines={3} value={item.outputSummary || "暂无执行结果"} /><CollapsibleText collapsedLines={3} value={item.validationResult || "暂无验证结果"} /></article>)}</div></div>; }
