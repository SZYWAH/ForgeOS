use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::{
    application::project_pulse::{
        ConfirmPulseInput, ProjectPulseError, ProjectPulseKernel, UpdateActivitySettingsInput,
        UpdateConnectorSettingsInput,
    },
    domain::{
        ActivitySyncRun, CodexActivityObservation, CodexConnectorSettings, ProjectActivitySettings,
        ProjectPulse, ProjectPulseDetails, PulseStatus, SyncMode, SyncStatus, SyncTrigger,
    },
};

use super::IpcError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConnectorRequest {
    pub executable_path: Option<String>,
    pub expected_version: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateActivitySettingsRequest {
    pub sync_mode: String,
    pub ai_enabled: bool,
    pub ai_provider_id: Option<String>,
    pub expected_version: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProjectActivityRequest {
    pub trigger: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PulseDraftRequest {
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub validation_summary: String,
    pub handoff: String,
    pub git_summary: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmPulseRequest {
    pub expected_version: i64,
    pub update_snapshot: bool,
    pub apply_session: bool,
    pub complete_session: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorSettingsDto {
    pub workspace_id: String,
    pub executable_path: Option<String>,
    pub last_detected_version: Option<String>,
    pub last_health_check_at: Option<String>,
    pub version: i64,
    pub available: bool,
}

impl From<CodexConnectorSettings> for ConnectorSettingsDto {
    fn from(value: CodexConnectorSettings) -> Self {
        Self::from_settings(value, true)
    }
}

impl ConnectorSettingsDto {
    fn from_settings(value: CodexConnectorSettings, available: bool) -> Self {
        Self {
            workspace_id: value.workspace_id,
            executable_path: value.executable_path,
            last_detected_version: value.last_detected_version.clone(),
            last_health_check_at: value.last_health_check_at.map(timestamp),
            version: value.version,
            available,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySettingsDto {
    pub project_id: String,
    pub sync_mode: String,
    pub ai_enabled: bool,
    pub ai_provider_id: Option<String>,
    pub initial_backfill_completed: bool,
    pub version: i64,
    pub updated_at: String,
}

impl From<ProjectActivitySettings> for ActivitySettingsDto {
    fn from(value: ProjectActivitySettings) -> Self {
        Self {
            project_id: value.project_id,
            sync_mode: value.sync_mode.as_str().into(),
            ai_enabled: value.ai_enabled,
            ai_provider_id: value.ai_provider_id,
            initial_backfill_completed: value.initial_backfill_completed,
            version: value.version,
            updated_at: timestamp(value.updated_at),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunDto {
    pub id: String,
    pub project_id: String,
    pub trigger: String,
    pub status: String,
    pub stage: String,
    pub threads_scanned: i64,
    pub turns_imported: i64,
    pub observations_created: i64,
    pub ai_invoked: bool,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
}

impl From<ActivitySyncRun> for SyncRunDto {
    fn from(value: ActivitySyncRun) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            trigger: value.trigger.as_str().into(),
            status: value.status.as_str().into(),
            stage: value.stage.as_str().into(),
            threads_scanned: value.threads_scanned,
            turns_imported: value.turns_imported,
            observations_created: value.observations_created,
            ai_invoked: value.ai_invoked,
            started_at: timestamp(value.started_at),
            completed_at: value.completed_at.map(timestamp),
            error_code: value.error_code,
            error_summary: value.error_summary,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPulseDto {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub fact_status: String,
    pub analysis_mode: String,
    pub activity_type: String,
    pub analysis_revision: String,
    pub ai_provider_id: Option<String>,
    pub ai_model: Option<String>,
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub validation_summary: String,
    pub handoff: String,
    pub git_summary: String,
    pub generated_at: String,
    pub updated_at: String,
    pub source_window_started_at: Option<String>,
    pub source_window_ended_at: Option<String>,
    pub selected_observation_count: i64,
    pub available_observation_count: i64,
    pub quality_warnings: Vec<String>,
    pub last_successful_sync_at: Option<String>,
    pub freshness: String,
    pub content_hash: String,
    pub source_count: usize,
    pub version: i64,
    pub confirmed_snapshot_id: Option<String>,
    pub confirmed_session_id: Option<String>,
}

fn pulse_dto(
    value: ProjectPulse,
    sync_mode: SyncMode,
    source_count: usize,
    last_successful_sync_at: Option<DateTime<Utc>>,
) -> ProjectPulseDto {
    let stale_after = if sync_mode == SyncMode::Interval30m {
        chrono::Duration::minutes(60)
    } else {
        chrono::Duration::hours(24)
    };
    let quality_warnings = value.quality_warnings();
    ProjectPulseDto {
        id: value.id,
        project_id: value.project_id,
        status: value.status.as_str().into(),
        fact_status: if value.status == PulseStatus::Confirmed {
            "confirmed"
        } else {
            "observed_unconfirmed"
        }
        .into(),
        analysis_mode: value.analysis_mode.as_str().into(),
        activity_type: value.quality.activity_type.as_str().into(),
        analysis_revision: value.quality.analysis_revision.as_str().into(),
        ai_provider_id: value.ai_provider_id,
        ai_model: value.ai_model,
        current_goal: value.details.current_goal,
        status_summary: value.details.status_summary,
        achievements: value.details.achievements,
        blockers: value.details.blockers,
        next_steps: value.details.next_steps,
        validation_summary: value.details.validation_summary,
        handoff: value.details.handoff,
        git_summary: value.details.git_summary,
        generated_at: timestamp(value.generated_at),
        updated_at: timestamp(value.updated_at),
        source_window_started_at: value.quality.source_window_started_at.map(timestamp),
        source_window_ended_at: value.quality.source_window_ended_at.map(timestamp),
        selected_observation_count: value.quality.selected_observation_count,
        available_observation_count: value.quality.available_observation_count,
        quality_warnings,
        last_successful_sync_at: last_successful_sync_at.map(timestamp),
        freshness: if Utc::now() - last_successful_sync_at.unwrap_or(value.generated_at)
            > stale_after
        {
            "stale"
        } else {
            "fresh"
        }
        .into(),
        content_hash: value.content_hash,
        source_count,
        version: value.version,
        confirmed_snapshot_id: value.confirmed_snapshot_id,
        confirmed_session_id: value.confirmed_session_id,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PulseSourceDto {
    pub id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub source_kind: String,
    pub turn_status: String,
    pub goal_hint: String,
    pub activity_summary: String,
    pub outcome_summary: String,
    pub validation_summary: String,
    pub handoff_summary: String,
    pub blockers: Vec<String>,
    pub source_updated_at: String,
}

impl From<CodexActivityObservation> for PulseSourceDto {
    fn from(value: CodexActivityObservation) -> Self {
        Self {
            id: value.id,
            thread_id: value.thread_id,
            turn_id: value.turn_id,
            source_kind: value.source_kind,
            turn_status: value.turn_status,
            goal_hint: value.goal_hint,
            activity_summary: value.activity_summary,
            outcome_summary: value.outcome_summary,
            validation_summary: value.validation_summary,
            handoff_summary: value.handoff_summary,
            blockers: value.blockers,
            source_updated_at: timestamp(value.source_updated_at),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncProgressEvent {
    project_id: String,
    sync_run_id: String,
    stage: String,
    status: String,
    message: String,
    threads_scanned: i64,
    turns_imported: i64,
    observations_created: i64,
}

#[tauri::command]
pub async fn get_codex_connector_status(
    kernel: State<'_, ProjectPulseKernel>,
) -> Result<ConnectorSettingsDto, IpcError> {
    match kernel.check_connector().await {
        Ok(settings) => Ok(settings.into()),
        Err(ProjectPulseError::CodexUnavailable(_)) => kernel
            .get_connector_settings()
            .await
            .map(|settings| ConnectorSettingsDto::from_settings(settings, false))
            .map_err(Into::into),
        Err(error) => Err(error.into()),
    }
}

#[tauri::command]
pub async fn update_codex_connector_settings(
    kernel: State<'_, ProjectPulseKernel>,
    input: UpdateConnectorRequest,
) -> Result<ConnectorSettingsDto, IpcError> {
    kernel
        .update_connector_settings(UpdateConnectorSettingsInput {
            executable_path: input.executable_path,
            expected_version: input.expected_version,
        })
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_project_activity_settings(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
) -> Result<ActivitySettingsDto, IpcError> {
    kernel
        .get_activity_settings(&project_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_project_activity_settings(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    input: UpdateActivitySettingsRequest,
) -> Result<ActivitySettingsDto, IpcError> {
    kernel
        .update_activity_settings(
            &project_id,
            UpdateActivitySettingsInput {
                sync_mode: input.sync_mode,
                ai_enabled: input.ai_enabled,
                ai_provider_id: input.ai_provider_id,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn sync_project_activity(
    app: AppHandle,
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    input: SyncProjectActivityRequest,
) -> Result<SyncRunDto, IpcError> {
    let trigger = SyncTrigger::from_str(&input.trigger)
        .map_err(crate::application::ProjectPulseError::from)?;
    let run = kernel.queue_sync(&project_id, trigger).await?;
    let dto = run.clone().into();
    if run.status == SyncStatus::Queued {
        let kernel = kernel.inner().clone();
        let run_id = run.id.clone();
        tauri::async_runtime::spawn(async move {
            let progress_app = app.clone();
            let result = kernel
                .execute_sync(&run_id, move |run| {
                    let _ = progress_app.emit("project-pulse://sync-progress", progress_event(run));
                })
                .await;
            let payload = match result {
                Ok(outcome) => progress_event(&outcome.run),
                Err(_) => kernel
                    .get_sync_run(&run_id)
                    .await
                    .map(|run| progress_event(&run))
                    .unwrap_or(SyncProgressEvent {
                        project_id: String::new(),
                        sync_run_id: run_id,
                        stage: "persisting".into(),
                        status: "failed".into(),
                        message: "同步失败".into(),
                        threads_scanned: 0,
                        turns_imported: 0,
                        observations_created: 0,
                    }),
            };
            let _ = app.emit("project-pulse://sync-completed", payload);
        });
    }
    Ok(dto)
}

#[tauri::command]
pub async fn get_activity_sync_run(
    kernel: State<'_, ProjectPulseKernel>,
    sync_run_id: String,
) -> Result<SyncRunDto, IpcError> {
    kernel
        .get_sync_run(&sync_run_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_activity_sync_runs(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
) -> Result<Vec<SyncRunDto>, IpcError> {
    kernel
        .list_sync_runs(&project_id)
        .await
        .map(|values| values.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_latest_project_pulse(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
) -> Result<Option<ProjectPulseDto>, IpcError> {
    let mode = kernel.get_activity_settings(&project_id).await?.sync_mode;
    let last_sync = kernel.latest_successful_sync_at(&project_id).await?;
    match kernel.get_latest_pulse(&project_id).await? {
        Some(pulse) => {
            let count = kernel
                .get_pulse_sources(&project_id, &pulse.id)
                .await?
                .len();
            Ok(Some(pulse_dto(pulse, mode, count, last_sync)))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn list_project_pulses(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
) -> Result<Vec<ProjectPulseDto>, IpcError> {
    let mode = kernel.get_activity_settings(&project_id).await?.sync_mode;
    let last_sync = kernel.latest_successful_sync_at(&project_id).await?;
    let values = kernel.list_pulses(&project_id).await?;
    let mut output = Vec::with_capacity(values.len());
    for pulse in values {
        let count = kernel
            .get_pulse_sources(&project_id, &pulse.id)
            .await?
            .len();
        output.push(pulse_dto(pulse, mode, count, last_sync));
    }
    Ok(output)
}

#[tauri::command]
pub async fn get_project_pulse_sources(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    pulse_id: String,
) -> Result<Vec<PulseSourceDto>, IpcError> {
    kernel
        .get_pulse_sources(&project_id, &pulse_id)
        .await
        .map(|values| values.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_project_pulse_draft(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    pulse_id: String,
    input: PulseDraftRequest,
) -> Result<ProjectPulseDto, IpcError> {
    let mode = kernel.get_activity_settings(&project_id).await?.sync_mode;
    let last_sync = kernel.latest_successful_sync_at(&project_id).await?;
    let pulse = kernel
        .update_pulse_draft(
            &project_id,
            &pulse_id,
            ProjectPulseDetails {
                current_goal: input.current_goal,
                status_summary: input.status_summary,
                achievements: input.achievements,
                blockers: input.blockers,
                next_steps: input.next_steps,
                validation_summary: input.validation_summary,
                handoff: input.handoff,
                git_summary: input.git_summary,
            },
            input.expected_version,
        )
        .await?;
    let count = kernel
        .get_pulse_sources(&project_id, &pulse.id)
        .await?
        .len();
    Ok(pulse_dto(pulse, mode, count, last_sync))
}

#[tauri::command]
pub async fn confirm_project_pulse(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    pulse_id: String,
    input: ConfirmPulseRequest,
) -> Result<ProjectPulseDto, IpcError> {
    let mode = kernel.get_activity_settings(&project_id).await?.sync_mode;
    let last_sync = kernel.latest_successful_sync_at(&project_id).await?;
    let pulse = kernel
        .confirm_pulse(
            &project_id,
            &pulse_id,
            ConfirmPulseInput {
                expected_version: input.expected_version,
                update_snapshot: input.update_snapshot,
                apply_session: input.apply_session,
                complete_session: input.complete_session,
            },
        )
        .await?;
    let count = kernel
        .get_pulse_sources(&project_id, &pulse.id)
        .await?
        .len();
    Ok(pulse_dto(pulse, mode, count, last_sync))
}

#[tauri::command]
pub async fn dismiss_project_pulse(
    kernel: State<'_, ProjectPulseKernel>,
    project_id: String,
    pulse_id: String,
    expected_version: i64,
) -> Result<ProjectPulseDto, IpcError> {
    let mode = kernel.get_activity_settings(&project_id).await?.sync_mode;
    let last_sync = kernel.latest_successful_sync_at(&project_id).await?;
    let pulse = kernel
        .dismiss_pulse(&project_id, &pulse_id, expected_version)
        .await?;
    let count = kernel
        .get_pulse_sources(&project_id, &pulse.id)
        .await?
        .len();
    Ok(pulse_dto(pulse, mode, count, last_sync))
}

fn progress_event(run: &ActivitySyncRun) -> SyncProgressEvent {
    let message = match run.stage.as_str() {
        "connecting" => "正在连接 Codex",
        "reading_codex" => "正在读取 Codex",
        "reading_git" => "正在检查 Git",
        "analyzing" => "正在分析项目状态",
        "persisting" if matches!(run.status, SyncStatus::Succeeded | SyncStatus::Partial) => {
            "Project Pulse 已生成"
        }
        "persisting" => "正在保存 Project Pulse",
        _ => "正在同步",
    };
    SyncProgressEvent {
        project_id: run.project_id.clone(),
        sync_run_id: run.id.clone(),
        stage: run.stage.as_str().into(),
        status: run.status.as_str().into(),
        message: message.into(),
        threads_scanned: run.threads_scanned,
        turns_imported: run.turns_imported,
        observations_created: run.observations_created,
    }
}
fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
