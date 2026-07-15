use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

use crate::application::{
    DevelopmentHistoryItem, DevelopmentHistoryPage, GitCommitSummary, ProjectAiContext,
    ProjectMemoryOverview, ProjectMemoryQueryKernel, ProjectMemorySummary,
};

use super::{
    dto::{DevelopmentSessionDto, ProjectDto, ProjectSnapshotDto},
    IpcError,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemorySummaryDto {
    project_id: String,
    name: String,
    description: String,
    status: String,
    lifecycle_stage: String,
    current_goal: String,
    next_steps: Vec<String>,
    blocker_count: usize,
    recent_activity_at: Option<String>,
    pulse_status: Option<String>,
    pulse_freshness: Option<String>,
    has_repository: bool,
    dirty_files: i64,
    ahead_count: Option<i64>,
    behind_count: Option<i64>,
    pending_achievement_count: i64,
}

impl From<ProjectMemorySummary> for ProjectMemorySummaryDto {
    fn from(value: ProjectMemorySummary) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
            description: value.description,
            status: value.status,
            lifecycle_stage: value.lifecycle_stage,
            current_goal: value.current_goal,
            next_steps: value.next_steps,
            blocker_count: value.blocker_count,
            recent_activity_at: value.recent_activity_at.map(timestamp),
            pulse_status: value.pulse_status,
            pulse_freshness: value.pulse_freshness,
            has_repository: value.has_repository,
            dirty_files: value.dirty_files,
            ahead_count: value.ahead_count,
            behind_count: value.behind_count,
            pending_achievement_count: value.pending_achievement_count,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryPulseDto {
    id: String,
    status: String,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemoryOverviewDto {
    project: ProjectDto,
    pulse: Option<MemoryPulseDto>,
    current_snapshot: Option<ProjectSnapshotDto>,
    current_session: Option<DevelopmentSessionDto>,
    recent_history: Vec<DevelopmentHistoryItemDto>,
}

impl From<ProjectMemoryOverview> for ProjectMemoryOverviewDto {
    fn from(value: ProjectMemoryOverview) -> Self {
        Self {
            project: value.project.into(),
            pulse: value.pulse.map(|pulse| MemoryPulseDto {
                id: pulse.id,
                status: pulse.status.as_str().into(),
                current_goal: pulse.details.current_goal,
                status_summary: pulse.details.status_summary,
                achievements: pulse.details.achievements,
                blockers: pulse.details.blockers,
                next_steps: pulse.details.next_steps,
                generated_at: timestamp(pulse.generated_at),
            }),
            current_snapshot: value.current_snapshot.map(Into::into),
            current_session: value.current_session.map(Into::into),
            recent_history: value.recent_history.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitSummaryDto {
    sha: String,
    message: String,
    author_name: String,
    authored_at: String,
}

impl From<GitCommitSummary> for GitCommitSummaryDto {
    fn from(value: GitCommitSummary) -> Self {
        Self {
            sha: value.sha,
            message: value.message,
            author_name: value.author_name,
            authored_at: timestamp(value.authored_at),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentHistoryItemDto {
    id: String,
    kind: String,
    trust_status: String,
    title: String,
    goal: String,
    rationale: String,
    activity: String,
    outcome: String,
    validation: String,
    handoff: String,
    started_at: String,
    ended_at: Option<String>,
    codex_turn_count: usize,
    git_commits: Vec<GitCommitSummaryDto>,
    session_id: Option<String>,
    pulse_id: Option<String>,
}

impl From<DevelopmentHistoryItem> for DevelopmentHistoryItemDto {
    fn from(value: DevelopmentHistoryItem) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            trust_status: value.trust_status,
            title: value.title,
            goal: value.goal,
            rationale: value.rationale,
            activity: value.activity,
            outcome: value.outcome,
            validation: value.validation,
            handoff: value.handoff,
            started_at: timestamp(value.started_at),
            ended_at: value.ended_at.map(timestamp),
            codex_turn_count: value.codex_turn_count,
            git_commits: value.git_commits.into_iter().map(Into::into).collect(),
            session_id: value.session_id,
            pulse_id: value.pulse_id,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentHistoryPageDto {
    items: Vec<DevelopmentHistoryItemDto>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAiContextDto {
    project_id: String,
    content_markdown: String,
    content_hash: String,
}

impl From<ProjectAiContext> for ProjectAiContextDto {
    fn from(value: ProjectAiContext) -> Self {
        Self {
            project_id: value.project_id,
            content_markdown: value.content_markdown,
            content_hash: value.content_hash,
        }
    }
}

impl From<DevelopmentHistoryPage> for DevelopmentHistoryPageDto {
    fn from(value: DevelopmentHistoryPage) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor,
        }
    }
}

#[tauri::command]
pub async fn list_project_memory_summaries(
    kernel: State<'_, ProjectMemoryQueryKernel>,
) -> Result<Vec<ProjectMemorySummaryDto>, IpcError> {
    kernel
        .list_project_memory_summaries()
        .await
        .map(|values| values.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_project_memory_overview(
    kernel: State<'_, ProjectMemoryQueryKernel>,
    project_id: String,
) -> Result<ProjectMemoryOverviewDto, IpcError> {
    kernel
        .get_project_memory_overview(&project_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_development_history(
    kernel: State<'_, ProjectMemoryQueryKernel>,
    project_id: String,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<DevelopmentHistoryPageDto, IpcError> {
    kernel
        .list_development_history(&project_id, cursor.as_deref(), limit.unwrap_or(20))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn build_project_ai_context(
    kernel: State<'_, ProjectMemoryQueryKernel>,
    project_id: String,
) -> Result<ProjectAiContextDto, IpcError> {
    kernel
        .build_project_ai_context(&project_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
