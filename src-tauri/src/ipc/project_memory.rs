use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

use crate::application::{
    DevelopmentHistoryItem, DevelopmentHistoryPage, GitCommitSummary, MemoryQualityIssue,
    ProjectAiAchievement, ProjectAiConfirmedState, ProjectAiContext, ProjectAiContextIdentity,
    ProjectAiGitState, ProjectAiObservedState, ProjectMemoryOverview, ProjectMemoryQueryKernel,
    ProjectMemorySummary, SnapshotRepairDraft,
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
pub struct MemoryQualityIssueDto {
    code: String,
    severity: String,
    section: String,
}

impl From<MemoryQualityIssue> for MemoryQualityIssueDto {
    fn from(value: MemoryQualityIssue) -> Self {
        Self {
            code: value.code.as_str().into(),
            severity: value.severity.as_str().into(),
            section: value.section,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRepairDraftDto {
    project_id: String,
    basis_snapshot_version: i64,
    source_mode: String,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
    quality_issues: Vec<MemoryQualityIssueDto>,
}

impl From<SnapshotRepairDraft> for SnapshotRepairDraftDto {
    fn from(value: SnapshotRepairDraft) -> Self {
        Self {
            project_id: value.project_id,
            basis_snapshot_version: value.basis_snapshot_version,
            source_mode: value.source_mode,
            current_goal: value.current_goal,
            status_summary: value.status_summary,
            achievements: value.achievements,
            blockers: value.blockers,
            next_steps: value.next_steps,
            quality_issues: value.quality_issues.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAiContextDto {
    project_id: String,
    readiness: String,
    quality_issues: Vec<MemoryQualityIssueDto>,
    identity: ProjectAiContextIdentityDto,
    confirmed_state: Option<ProjectAiConfirmedStateDto>,
    observed_state: Option<ProjectAiObservedStateDto>,
    recent_history: Vec<DevelopmentHistoryItemDto>,
    confirmed_achievements: Vec<ProjectAiAchievementDto>,
    git_state: Vec<ProjectAiGitStateDto>,
    next_steps: Vec<String>,
    content_markdown: String,
    content_hash: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAiContextIdentityDto {
    name: String,
    description: String,
    vision: String,
    target_user: String,
    scope_summary: String,
    non_goals: String,
    tech_stack: String,
}

impl From<ProjectAiContextIdentity> for ProjectAiContextIdentityDto {
    fn from(value: ProjectAiContextIdentity) -> Self {
        Self {
            name: value.name,
            description: value.description,
            vision: value.vision,
            target_user: value.target_user,
            scope_summary: value.scope_summary,
            non_goals: value.non_goals,
            tech_stack: value.tech_stack,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAiConfirmedStateDto {
    version: i64,
    confirmed_at: DateTime<Utc>,
    superseded_by_newer_activity: bool,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
}

impl From<ProjectAiConfirmedState> for ProjectAiConfirmedStateDto {
    fn from(value: ProjectAiConfirmedState) -> Self {
        Self {
            version: value.version,
            confirmed_at: value.confirmed_at,
            superseded_by_newer_activity: value.superseded_by_newer_activity,
            current_goal: value.current_goal,
            status_summary: value.status_summary,
            achievements: value.achievements,
            blockers: value.blockers,
            next_steps: value.next_steps,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAiObservedStateDto {
    status: String,
    fact_status: String,
    generated_at: DateTime<Utc>,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
}

impl From<ProjectAiObservedState> for ProjectAiObservedStateDto {
    fn from(value: ProjectAiObservedState) -> Self {
        Self {
            status: value.status,
            fact_status: value.fact_status,
            generated_at: value.generated_at,
            current_goal: value.current_goal,
            status_summary: value.status_summary,
            achievements: value.achievements,
            blockers: value.blockers,
            next_steps: value.next_steps,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAiAchievementDto {
    id: String,
    title: String,
    result: String,
    featured: bool,
}

impl From<ProjectAiAchievement> for ProjectAiAchievementDto {
    fn from(value: ProjectAiAchievement) -> Self {
        Self {
            id: value.id,
            title: value.title,
            result: value.result,
            featured: value.featured,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAiGitStateDto {
    branch: String,
    dirty_files: i64,
    ahead_count: Option<i64>,
    behind_count: Option<i64>,
}

impl From<ProjectAiGitState> for ProjectAiGitStateDto {
    fn from(value: ProjectAiGitState) -> Self {
        Self {
            branch: value.branch,
            dirty_files: value.dirty_files,
            ahead_count: value.ahead_count,
            behind_count: value.behind_count,
        }
    }
}

impl From<ProjectAiContext> for ProjectAiContextDto {
    fn from(value: ProjectAiContext) -> Self {
        Self {
            project_id: value.project_id,
            readiness: value.readiness,
            quality_issues: value.quality_issues.into_iter().map(Into::into).collect(),
            identity: value.identity.into(),
            confirmed_state: value.confirmed_state.map(Into::into),
            observed_state: value.observed_state.map(Into::into),
            recent_history: value.recent_history.into_iter().map(Into::into).collect(),
            confirmed_achievements: value
                .confirmed_achievements
                .into_iter()
                .map(Into::into)
                .collect(),
            git_state: value.git_state.into_iter().map(Into::into).collect(),
            next_steps: value.next_steps,
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

#[tauri::command]
pub async fn build_snapshot_repair_draft(
    kernel: State<'_, ProjectMemoryQueryKernel>,
    project_id: String,
) -> Result<SnapshotRepairDraftDto, IpcError> {
    kernel
        .build_snapshot_repair_draft(&project_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
