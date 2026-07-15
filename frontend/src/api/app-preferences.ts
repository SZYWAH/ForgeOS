import { invoke } from "@tauri-apps/api/core";
import type { AppPreferences, UpdateAppPreferencesInput } from "../types/app-preferences";

export const appPreferencesApi = {
  get: () => invoke<AppPreferences>("get_app_preferences"),
  update: (input: UpdateAppPreferencesInput) =>
    invoke<AppPreferences>("update_app_preferences", { input }),
};
