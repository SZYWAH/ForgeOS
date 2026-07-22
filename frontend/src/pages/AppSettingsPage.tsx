import { useEffect, useLayoutEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { resolveLocale, useAppPreferences } from "../contexts/AppPreferencesProvider";
import type { LanguageMode } from "../types/app-preferences";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";
import { Notice, StatusBadge } from "../components/ui/VisualSystem";
import { codexMcpApi } from "../api/codex-mcp";
import type { CodexMcpRegistrationStatus } from "../types/codex-mcp";
import { localizeAppError, type LocalizedAppError } from "../utils/appError";

export function AppSettingsPage() {
  const { t } = useTranslation();
  const { preferences, loadWarning, saving, saveError, updatePreferences, reload } = useAppPreferences();
  const [languageMode, setLanguageMode] = useState<LanguageMode>(preferences.languageMode);
  const [showTerms, setShowTerms] = useState(preferences.showTechnicalTerms);
  const [saved, setSaved] = useState(false);
  const [mcpStatus, setMcpStatus] = useState<CodexMcpRegistrationStatus | null>(null);
  const [mcpLoading, setMcpLoading] = useState(true);
  const [mcpWorking, setMcpWorking] = useState(false);
  const [mcpNotice, setMcpNotice] = useState<string>();
  const [mcpError, setMcpError] = useState<LocalizedAppError | null>(null);

  useLayoutEffect(() => {
    setLanguageMode(preferences.languageMode);
    setShowTerms(preferences.showTechnicalTerms);
  }, [preferences]);

  const loadMcp = async () => {
    setMcpLoading(true);
    setMcpError(null);
    try {
      setMcpStatus(await codexMcpApi.get());
    } catch (error) {
      setMcpError(localizeAppError(error, t));
    } finally {
      setMcpLoading(false);
    }
  };

  useEffect(() => {
    void loadMcp();
  }, []);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSaved(false);
    setSaved(await updatePreferences(languageMode, showTerms));
  };

  const configureMcp = async (replaceExisting: boolean) => {
    if (replaceExisting && !window.confirm(t("settings.codexReplaceConfirm"))) return;
    setMcpWorking(true);
    setMcpError(null);
    setMcpNotice(undefined);
    try {
      const result = await codexMcpApi.configure(replaceExisting);
      setMcpNotice(result.restartRequired ? t("settings.codexRestart") : t("settings.codexAlreadyConnected"));
      await loadMcp();
    } catch (error) {
      setMcpError(localizeAppError(error, t));
    } finally {
      setMcpWorking(false);
    }
  };

  const removeMcp = async () => {
    if (!window.confirm(t("settings.codexDisconnectConfirm"))) return;
    setMcpWorking(true);
    setMcpError(null);
    setMcpNotice(undefined);
    try {
      const result = await codexMcpApi.remove();
      setMcpNotice(result.restartRequired ? t("settings.codexRestart") : t("settings.codexNotConnected"));
      await loadMcp();
    } catch (error) {
      setMcpError(localizeAppError(error, t));
    } finally {
      setMcpWorking(false);
    }
  };

  return <div className="page-stack settings-page">
    <PageIntro title={t("settings.title")} description={t("settings.description")} />
    {loadWarning && <div className="notice warning"><span>{t("settings.readFailed")}</span><button className="button secondary" onClick={() => void reload()}>{t("common.retry")}</button></div>}
    <form className="panel settings-form" onSubmit={submit}>
      <fieldset className="language-choice-group">
        <legend>{t("settings.language")}</legend>
        <p className="helper-text">{t("settings.languageHelp")}</p>
        <div className="language-choice-group__options">
          {(["system", "zh-CN", "en-US"] as LanguageMode[]).map((value) => (
            <label className="language-choice" key={value}>
              <input
                checked={languageMode === value}
                name="language-mode"
                onChange={() => setLanguageMode(value)}
                type="radio"
                value={value}
              />
              <span>{value === "system" ? t("settings.system") : value === "zh-CN" ? t("settings.simplifiedChinese") : t("settings.english")}</span>
            </label>
          ))}
        </div>
      </fieldset>
      <section>
        <label className="check-row"><input type="checkbox" checked={showTerms} disabled={resolveLocale(languageMode) !== "zh-CN"} onChange={(event) => setShowTerms(event.target.checked)} /><span>{t("settings.technicalTerms")}</span></label>
        <p className="helper-text">{t("settings.technicalTermsHelp")}</p>
      </section>
      <div className="form-actions"><button className="button primary" disabled={saving}>{saving ? t("common.saving") : t("settings.save")}</button></div>
      {saved && <p className="success-text">{t("settings.saved")}</p>}
      {saveError && <div className="notice error"><div><strong>{saveError.title}</strong>{saveError.guidance ? <p>{saveError.guidance}</p> : null}</div><TechnicalDetails label={t("common.technicalDetails")}><code>{saveError.technicalCode}{saveError.technicalMessage ? ` · ${saveError.technicalMessage}` : ""}</code></TechnicalDetails></div>}
    </form>
    <section className="panel settings-form codex-mcp-settings" aria-labelledby="codex-mcp-heading">
      <div className="settings-section-heading">
        <div>
          <h2 id="codex-mcp-heading">{t("settings.codexTitle")}</h2>
          <p className="helper-text">{t("settings.codexDescription")}</p>
        </div>
        {mcpStatus ? <StatusBadge tone={mcpTone(mcpStatus)}>{t(`settings.codexState_${mcpStatus.state}`)}</StatusBadge> : null}
      </div>
      <Notice tone="info" title={t("settings.codexReadOnlyTitle")}>{t("settings.codexReadOnlyDescription")}</Notice>
      {mcpLoading ? <p className="helper-text">{t("settings.codexChecking")}</p> : null}
      {mcpStatus ? <div className="codex-mcp-summary">
        <div><span>{t("settings.codexCli")}</span><strong>{mcpStatus.codexVersion || t("common.unknown")}</strong></div>
        <div><span>{t("settings.codexConnection")}</span><strong>{t(`settings.codexState_${mcpStatus.state}`)}</strong></div>
      </div> : null}
      {mcpStatus?.state === "sidecar_missing" ? <Notice tone="warning">{t("settings.codexReleaseOnly")}</Notice> : null}
      {mcpStatus?.state === "conflict" ? <Notice tone="warning" title={t("settings.codexConflictTitle")}>{t("settings.codexConflictDescription")}</Notice> : null}
      {mcpNotice ? <Notice tone="success">{mcpNotice}</Notice> : null}
      {mcpError ? <Notice tone="danger" title={mcpError.title}>
        {mcpError.guidance ? <p>{mcpError.guidance}</p> : null}
        <TechnicalDetails label={t("common.technicalDetails")}><code>{mcpError.technicalCode}{mcpError.technicalMessage ? ` · ${mcpError.technicalMessage}` : ""}</code></TechnicalDetails>
      </Notice> : null}
      <div className="form-actions codex-mcp-actions">
        {mcpStatus?.state === "configured" && mcpStatus.enabled ? <button className="button secondary" disabled={mcpWorking} onClick={() => void removeMcp()} type="button">{t("settings.codexDisconnect")}</button> : null}
        {mcpStatus?.state === "conflict" ? <button className="button secondary" disabled={mcpWorking} onClick={() => void configureMcp(true)} type="button">{t("settings.codexReplace")}</button> : null}
        {mcpStatus && ["not_configured", "invalid"].includes(mcpStatus.state) ? <button className="button secondary" disabled={mcpWorking || !mcpStatus.canConfigure} onClick={() => void configureMcp(false)} type="button">{t("settings.codexConnect")}</button> : null}
        {mcpStatus?.state === "configured" && !mcpStatus.enabled ? <button className="button secondary" disabled={mcpWorking} onClick={() => void configureMcp(false)} type="button">{t("settings.codexUpdate")}</button> : null}
        <button className="button quiet" disabled={mcpWorking || mcpLoading} onClick={() => void loadMcp()} type="button">{t("common.refresh")}</button>
      </div>
      {mcpStatus ? <TechnicalDetails label={t("common.technicalDetails")}>
        <dl className="technical-definition-list">
          <div><dt>{t("settings.codexExpectedCommand")}</dt><dd>{mcpStatus.expectedCommand || t("common.none")}</dd></div>
          <div><dt>{t("settings.codexConfiguredCommand")}</dt><dd>{mcpStatus.configuredCommand || t("common.none")}</dd></div>
        </dl>
      </TechnicalDetails> : null}
    </section>
  </div>;
}

function mcpTone(status: CodexMcpRegistrationStatus): "neutral" | "info" | "success" | "warning" | "danger" {
  if (status.state === "configured" && status.enabled) return "success";
  if (status.state === "conflict" || status.state === "sidecar_missing") return "warning";
  if (status.state === "codex_unavailable" || status.state === "invalid") return "danger";
  return "neutral";
}
