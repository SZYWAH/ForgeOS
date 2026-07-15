export type LanguageMode = "system" | "zh-CN" | "en-US";
export type ResolvedLocale = "zh-CN" | "en-US";

export interface AppPreferences {
  languageMode: LanguageMode;
  showTechnicalTerms: boolean;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface UpdateAppPreferencesInput {
  languageMode: LanguageMode;
  showTechnicalTerms: boolean;
  expectedVersion: number;
}
