use tauri::State;

use crate::application::{
    repository::{RefreshRepositoryInput, RegisterRepositoryInput},
    RepositoryKernel,
};

use super::{
    dto::{
        CreateCommitEvidenceRequest, EvidenceDto, GitInspectionDto, InspectRepositoryRequest,
        RefreshRepositoryRequest, RegisterRepositoryRequest, RepositoryRecordDto,
    },
    IpcError,
};

#[tauri::command]
pub async fn inspect_repository(
    kernel: State<'_, RepositoryKernel>,
    input: InspectRepositoryRequest,
) -> Result<GitInspectionDto, IpcError> {
    kernel
        .inspect_repository(&input.path)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn register_repository(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
    input: RegisterRepositoryRequest,
) -> Result<RepositoryRecordDto, IpcError> {
    kernel
        .register_repository(&project_id, RegisterRepositoryInput { path: input.path })
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_repositories(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
) -> Result<Vec<RepositoryRecordDto>, IpcError> {
    kernel
        .list_repositories(&project_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn refresh_repository(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
    repository_id: String,
    input: RefreshRepositoryRequest,
) -> Result<RepositoryRecordDto, IpcError> {
    kernel
        .refresh_repository(
            &project_id,
            &repository_id,
            RefreshRepositoryInput {
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn create_commit_evidence(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
    repository_id: String,
    input: CreateCommitEvidenceRequest,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .create_commit_evidence(&project_id, &repository_id, &input.commit_sha)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn create_status_evidence(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
    repository_id: String,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .create_status_evidence(&project_id, &repository_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_repository_evidence(
    kernel: State<'_, RepositoryKernel>,
    project_id: String,
    repository_id: String,
) -> Result<Vec<EvidenceDto>, IpcError> {
    kernel
        .list_repository_evidence(&project_id, &repository_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
