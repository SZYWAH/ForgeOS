use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    application::{
        GitHubAnalysisImportError, GitHubAnalysisImportKernel, GitHubAnalysisImportView,
        GitHubAnalysisStatusView, ReviewGitHubAnalysisImportInput, UpdateGitHubAnalysisImportInput,
    },
    domain::{LifecycleStage, ProjectDetails, ProjectStatus},
};

use super::{dto::GitHubImportDraftDto, github_analysis::StatusDto, IpcError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisImportLocator {
    pub workspace_id: String,
    pub analysis_run_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnalysisImportRequest {
    pub workspace_id: String,
    pub draft_id: String,
    pub expected_draft_version: i64,
    pub expected_proposal_version: i64,
    pub project_name: String,
    pub project_slug: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub project_status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAnalysisImportRequest {
    pub workspace_id: String,
    pub draft_id: String,
    pub expected_draft_version: i64,
    pub expected_proposal_version: i64,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshAnalysisImportRequest {
    pub workspace_id: String,
    pub analysis_run_id: String,
    pub expected_draft_version: i64,
    pub expected_proposal_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisImportProposalDto {
    pub draft_id: String,
    pub workspace_id: String,
    pub analysis_run_id: String,
    pub source_commit_sha: String,
    pub source_content_hash: String,
    pub project_name: String,
    pub project_slug: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub project_status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<crate::domain::GitHubAnalysisImportProposal> for AnalysisImportProposalDto {
    fn from(value: crate::domain::GitHubAnalysisImportProposal) -> Self {
        Self {
            draft_id: value.draft_id,
            workspace_id: value.workspace_id,
            analysis_run_id: value.analysis_run_id,
            source_commit_sha: value.source_commit_sha,
            source_content_hash: value.source_content_hash,
            project_name: value.project.name,
            project_slug: value.project.slug,
            description: value.project.description,
            vision: value.project.vision,
            target_user: value.project.target_user,
            scope: value.project.scope,
            non_goals: value.project.non_goals,
            tech_stack: value.project.tech_stack,
            project_status: value.project.status.as_str().into(),
            lifecycle_stage: value.project.lifecycle_stage.as_str().into(),
            priority: value.project.priority,
            partial_acknowledged: value.partial_acknowledged,
            stale_acknowledged: value.stale_acknowledged,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
            version: value.version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisImportViewDto {
    pub analysis: StatusDto,
    pub draft: Option<GitHubImportDraftDto>,
    pub proposal: Option<AnalysisImportProposalDto>,
    pub stale: bool,
    pub source_changed: bool,
    pub existing_project_id: Option<String>,
}

impl From<GitHubAnalysisImportView> for AnalysisImportViewDto {
    fn from(value: GitHubAnalysisImportView) -> Self {
        Self {
            analysis: GitHubAnalysisStatusView {
                run: value.analysis,
                stale: value.stale,
            }
            .into(),
            draft: value.draft.map(Into::into),
            proposal: value.proposal.map(Into::into),
            stale: value.stale,
            source_changed: value.source_changed,
            existing_project_id: value.existing_project_id,
        }
    }
}

#[tauri::command]
pub async fn import_github_analysis(
    kernel: State<'_, GitHubAnalysisImportKernel>,
    input: AnalysisImportLocator,
) -> Result<AnalysisImportViewDto, IpcError> {
    kernel
        .prepare(&input.workspace_id, &input.analysis_run_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_github_analysis_import(
    kernel: State<'_, GitHubAnalysisImportKernel>,
    input: AnalysisImportLocator,
) -> Result<AnalysisImportViewDto, IpcError> {
    kernel
        .get(&input.workspace_id, &input.analysis_run_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_github_analysis_import(
    kernel: State<'_, GitHubAnalysisImportKernel>,
    input: UpdateAnalysisImportRequest,
) -> Result<AnalysisImportViewDto, IpcError> {
    let project = project_details(&input)?;
    kernel
        .update(
            &input.workspace_id,
            &input.draft_id,
            UpdateGitHubAnalysisImportInput {
                expected_draft_version: input.expected_draft_version,
                expected_proposal_version: input.expected_proposal_version,
                project,
                partial_acknowledged: input.partial_acknowledged,
                stale_acknowledged: input.stale_acknowledged,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn review_github_analysis_import(
    kernel: State<'_, GitHubAnalysisImportKernel>,
    input: ReviewAnalysisImportRequest,
) -> Result<AnalysisImportViewDto, IpcError> {
    kernel
        .review(
            &input.workspace_id,
            &input.draft_id,
            ReviewGitHubAnalysisImportInput {
                expected_draft_version: input.expected_draft_version,
                expected_proposal_version: input.expected_proposal_version,
                partial_acknowledged: input.partial_acknowledged,
                stale_acknowledged: input.stale_acknowledged,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn refresh_github_analysis_import_from_report(
    kernel: State<'_, GitHubAnalysisImportKernel>,
    input: RefreshAnalysisImportRequest,
) -> Result<AnalysisImportViewDto, IpcError> {
    kernel
        .refresh_from_report(
            &input.workspace_id,
            &input.analysis_run_id,
            input.expected_draft_version,
            input.expected_proposal_version,
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn project_details(input: &UpdateAnalysisImportRequest) -> Result<ProjectDetails, IpcError> {
    Ok(ProjectDetails {
        slug: input.project_slug.clone(),
        name: input.project_name.clone(),
        description: input.description.clone(),
        vision: input.vision.clone(),
        target_user: input.target_user.clone(),
        scope: input.scope.clone(),
        non_goals: input.non_goals.clone(),
        tech_stack: input.tech_stack.clone(),
        status: ProjectStatus::from_str(&input.project_status)
            .map_err(GitHubAnalysisImportError::Validation)
            .map_err(IpcError::from)?,
        lifecycle_stage: LifecycleStage::from_str(&input.lifecycle_stage)
            .map_err(GitHubAnalysisImportError::Validation)
            .map_err(IpcError::from)?,
        priority: input.priority,
    })
}
