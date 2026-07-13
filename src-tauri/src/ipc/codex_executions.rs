use tauri::State;

use super::{
    dto::{
        CancelCodexExecutionRequest, CodexExecutionDto, RecordCodexExecutionResultRequest,
        TransitionCodexExecutionRequest,
    },
    IpcError,
};
use crate::application::{codex_execution::TransitionCodexExecutionInput, CodexExecutionKernel};

#[tauri::command]
pub async fn create_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    proposal_id: String,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .create_execution(&project_id, &proposal_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_codex_executions(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
) -> Result<Vec<CodexExecutionDto>, IpcError> {
    kernel
        .list_executions(&project_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    execution_id: String,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .get_execution(&project_id, &execution_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn start_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    execution_id: String,
    input: TransitionCodexExecutionRequest,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .start_execution(
            &project_id,
            &execution_id,
            TransitionCodexExecutionInput {
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn complete_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    execution_id: String,
    input: RecordCodexExecutionResultRequest,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .complete_execution(&project_id, &execution_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn fail_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    execution_id: String,
    input: RecordCodexExecutionResultRequest,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .fail_execution(&project_id, &execution_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn cancel_codex_execution(
    kernel: State<'_, CodexExecutionKernel>,
    project_id: String,
    execution_id: String,
    input: CancelCodexExecutionRequest,
) -> Result<CodexExecutionDto, IpcError> {
    kernel
        .cancel_execution(&project_id, &execution_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}
