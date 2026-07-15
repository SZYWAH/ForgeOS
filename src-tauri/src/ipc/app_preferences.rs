use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    application::{app_preferences::UpdateAppPreferencesInput, AppPreferencesKernel},
    domain::AppPreferences,
};

use super::IpcError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferencesDto {
    pub language_mode: String,
    pub show_technical_terms: bool,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AppPreferences> for AppPreferencesDto {
    fn from(value: AppPreferences) -> Self {
        Self {
            language_mode: value.language_mode().as_str().to_owned(),
            show_technical_terms: value.show_technical_terms(),
            version: value.version(),
            created_at: value
                .created_at()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: value
                .updated_at()
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppPreferencesRequest {
    pub language_mode: String,
    pub show_technical_terms: bool,
    pub expected_version: i64,
}

#[tauri::command]
pub async fn get_app_preferences(
    kernel: State<'_, AppPreferencesKernel>,
) -> Result<AppPreferencesDto, IpcError> {
    kernel.get().await.map(Into::into).map_err(Into::into)
}

#[tauri::command]
pub async fn update_app_preferences(
    kernel: State<'_, AppPreferencesKernel>,
    input: UpdateAppPreferencesRequest,
) -> Result<AppPreferencesDto, IpcError> {
    kernel
        .update(UpdateAppPreferencesInput {
            language_mode: input.language_mode,
            show_technical_terms: input.show_technical_terms,
            expected_version: input.expected_version,
        })
        .await
        .map(Into::into)
        .map_err(Into::into)
}
