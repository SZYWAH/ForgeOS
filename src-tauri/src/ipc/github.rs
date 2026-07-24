use tauri::State;

use crate::application::{GitHubAnalysisImportKernel, GitHubImportKernel};

use super::{
    dto::{
        GitHubConnectorDto, GitHubImportDraftDto, GitHubImportResultDto,
        PreviewGitHubRepositoryRequest, SetGitHubTokenRequest, TransitionGitHubImportRequest,
    },
    IpcError,
};

#[tauri::command]
pub async fn get_github_connector(
    kernel: State<'_, GitHubImportKernel>,
) -> Result<GitHubConnectorDto, IpcError> {
    kernel
        .get_connector()
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn set_github_token(
    kernel: State<'_, GitHubImportKernel>,
    input: SetGitHubTokenRequest,
) -> Result<GitHubConnectorDto, IpcError> {
    kernel
        .set_token(input.token)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn clear_github_token(
    kernel: State<'_, GitHubImportKernel>,
) -> Result<GitHubConnectorDto, IpcError> {
    kernel
        .clear_token()
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn preview_github_repository(
    kernel: State<'_, GitHubImportKernel>,
    input: PreviewGitHubRepositoryRequest,
) -> Result<GitHubImportDraftDto, IpcError> {
    kernel
        .preview_repository(&input.repository_url)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_github_import_drafts(
    kernel: State<'_, GitHubImportKernel>,
) -> Result<Vec<GitHubImportDraftDto>, IpcError> {
    kernel
        .list_drafts()
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn review_github_import(
    kernel: State<'_, GitHubImportKernel>,
    draft_id: String,
    input: TransitionGitHubImportRequest,
) -> Result<GitHubImportDraftDto, IpcError> {
    kernel
        .review_draft(&draft_id, input.expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn reject_github_import(
    kernel: State<'_, GitHubImportKernel>,
    draft_id: String,
    input: TransitionGitHubImportRequest,
) -> Result<GitHubImportDraftDto, IpcError> {
    kernel
        .reject_draft(&draft_id, input.expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn import_github_project(
    kernel: State<'_, GitHubImportKernel>,
    analysis_import_kernel: State<'_, GitHubAnalysisImportKernel>,
    draft_id: String,
    input: TransitionGitHubImportRequest,
) -> Result<GitHubImportResultDto, IpcError> {
    if let (Some(workspace_id), Some(proposal_expected_version)) = (
        input.workspace_id.as_deref(),
        input.proposal_expected_version,
    ) {
        return analysis_import_kernel
            .import_reviewed(
                workspace_id,
                &draft_id,
                input.expected_version,
                proposal_expected_version,
            )
            .await
            .map(Into::into)
            .map_err(Into::into);
    }
    kernel
        .import_draft(&draft_id, input.expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_project_github_import(
    kernel: State<'_, GitHubImportKernel>,
    project_id: String,
) -> Result<Option<GitHubImportDraftDto>, IpcError> {
    kernel
        .get_project_import(&project_id)
        .await
        .map(|value| value.map(Into::into))
        .map_err(Into::into)
}
