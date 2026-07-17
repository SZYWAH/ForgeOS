import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { resolveLocale, useAppPreferences } from "../contexts/AppPreferencesProvider";
import type { LanguageMode } from "../types/app-preferences";
import { PageIntro, TechnicalDetails } from "../components/ui/PlainLanguage";

export function AppSettingsPage() {
  const { t } = useTranslation();
  const { preferences, loadWarning, saving, saveError, updatePreferences, reload } = useAppPreferences();
  const [languageMode, setLanguageMode] = useState<LanguageMode>(preferences.languageMode);
  const [showTerms, setShowTerms] = useState(preferences.showTechnicalTerms);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setLanguageMode(preferences.languageMode);
    setShowTerms(preferences.showTechnicalTerms);
  }, [preferences]);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSaved(false);
    setSaved(await updatePreferences(languageMode, showTerms));
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
  </div>;
}
