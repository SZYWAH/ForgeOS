import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { appPreferencesApi } from "../api/app-preferences";
import i18n from "../i18n";
import type { AppPreferences, LanguageMode, ResolvedLocale } from "../types/app-preferences";
import { localizeAppError, type LocalizedAppError } from "../utils/appError";

const FALLBACK: AppPreferences = {
  languageMode: "system",
  showTechnicalTerms: false,
  version: 1,
  createdAt: "",
  updatedAt: "",
};

export function resolveLocale(mode: LanguageMode, systemLanguage = navigator.language): ResolvedLocale {
  if (mode === "zh-CN" || mode === "en-US") return mode;
  return systemLanguage.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

interface AppPreferencesContextValue {
  preferences: AppPreferences;
  locale: ResolvedLocale;
  loadWarning: boolean;
  saving: boolean;
  saveError: LocalizedAppError | null;
  updatePreferences: (languageMode: LanguageMode, showTechnicalTerms: boolean) => Promise<boolean>;
  reload: () => Promise<void>;
}

const AppPreferencesContext = createContext<AppPreferencesContextValue | null>(null);

export function AppPreferencesProvider({ children }: { children: ReactNode }) {
  const [preferences, setPreferences] = useState(FALLBACK);
  const [ready, setReady] = useState(false);
  const [loadWarning, setLoadWarning] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<LocalizedAppError | null>(null);

  const apply = async (value: AppPreferences) => {
    await i18n.changeLanguage(resolveLocale(value.languageMode));
    document.documentElement.lang = resolveLocale(value.languageMode);
    setPreferences(value);
  };

  const reload = async () => {
    try {
      const value = await appPreferencesApi.get();
      await apply(value);
      setLoadWarning(false);
    } catch {
      await apply(FALLBACK);
      setLoadWarning(true);
    } finally {
      setReady(true);
    }
  };

  useEffect(() => {
    void reload();
  }, []);

  const updatePreferences = async (languageMode: LanguageMode, showTechnicalTerms: boolean) => {
    setSaving(true);
    setSaveError(null);
    try {
      const value = await appPreferencesApi.update({
        languageMode,
        showTechnicalTerms,
        expectedVersion: preferences.version,
      });
      await apply(value);
      return true;
    } catch (error) {
      if (error && typeof error === "object" && "code" in error && String(error.code) === "app_preferences_version_conflict") {
        await reload();
      }
      setSaveError(localizeAppError(error, i18n.t));
      return false;
    } finally {
      setSaving(false);
    }
  };

  const value = useMemo<AppPreferencesContextValue>(() => ({
    preferences,
    locale: resolveLocale(preferences.languageMode),
    loadWarning,
    saving,
    saveError,
    updatePreferences,
    reload,
  }), [preferences, loadWarning, saving, saveError]);

  if (!ready) {
    return <div className="app-boot" aria-label="ForgeOS"><strong>ForgeOS</strong><span /></div>;
  }

  return <AppPreferencesContext.Provider value={value}>{children}</AppPreferencesContext.Provider>;
}

export function useAppPreferences() {
  const value = useContext(AppPreferencesContext);
  if (!value) throw new Error("useAppPreferences must be used inside AppPreferencesProvider");
  return value;
}
