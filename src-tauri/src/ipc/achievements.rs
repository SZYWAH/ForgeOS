use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    application::{AchievementInput, AchievementKernel},
    domain::ProjectAchievement,
};

use super::IpcError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementRequest {
    category: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    problem: String,
    #[serde(default)]
    approach: String,
    #[serde(default)]
    result: String,
    #[serde(default)]
    validation: String,
    #[serde(default)]
    impact: String,
    #[serde(default)]
    technologies: Vec<String>,
    #[serde(default)]
    resume_bullet: String,
    #[serde(default)]
    featured: bool,
}

impl From<AchievementRequest> for AchievementInput {
    fn from(value: AchievementRequest) -> Self {
        Self {
            category: value.category,
            title: value.title,
            problem: value.problem,
            approach: value.approach,
            result: value.result,
            validation: value.validation,
            impact: value.impact,
            technologies: value.technologies,
            resume_bullet: value.resume_bullet,
            featured: value.featured,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AchievementDto {
    id: String,
    project_id: String,
    category: String,
    title: String,
    problem: String,
    approach: String,
    result: String,
    validation: String,
    impact: String,
    technologies: Vec<String>,
    resume_bullet: String,
    status: String,
    source_mode: String,
    featured: bool,
    source_pulse_id: Option<String>,
    confirmed_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl From<ProjectAchievement> for AchievementDto {
    fn from(value: ProjectAchievement) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            category: value.details.category.as_str().into(),
            title: value.details.title,
            problem: value.details.problem,
            approach: value.details.approach,
            result: value.details.result,
            validation: value.details.validation,
            impact: value.details.impact,
            technologies: value.details.technologies,
            resume_bullet: value.details.resume_bullet,
            status: value.status.as_str().into(),
            source_mode: value.source_mode.as_str().into(),
            featured: value.details.featured,
            source_pulse_id: value.source_pulse_id,
            confirmed_at: value.confirmed_at.map(timestamp),
            created_at: timestamp(value.created_at),
            updated_at: timestamp(value.updated_at),
            version: value.version,
        }
    }
}

#[tauri::command]
pub async fn list_project_achievements(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
) -> Result<Vec<AchievementDto>, IpcError> {
    kernel
        .list(&project_id)
        .await
        .map(|values| values.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_project_achievement(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    achievement_id: String,
) -> Result<AchievementDto, IpcError> {
    kernel
        .get(&project_id, &achievement_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn create_project_achievement(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    input: AchievementRequest,
) -> Result<AchievementDto, IpcError> {
    kernel
        .create(&project_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn create_achievement_draft_from_history(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    history_id: String,
) -> Result<AchievementDto, IpcError> {
    kernel
        .create_from_history(&project_id, &history_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_project_achievement(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    achievement_id: String,
    expected_version: i64,
    input: AchievementRequest,
) -> Result<AchievementDto, IpcError> {
    kernel
        .update(&project_id, &achievement_id, expected_version, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn confirm_project_achievement(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    achievement_id: String,
    expected_version: i64,
) -> Result<AchievementDto, IpcError> {
    kernel
        .confirm(&project_id, &achievement_id, expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn archive_project_achievement(
    kernel: State<'_, AchievementKernel>,
    project_id: String,
    achievement_id: String,
    expected_version: i64,
) -> Result<AchievementDto, IpcError> {
    kernel
        .archive(&project_id, &achievement_id, expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
