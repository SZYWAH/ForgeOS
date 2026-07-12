use tauri::State;

use super::{
    dto::{
        CodexTaskProposalDto, CreateCodexTaskProposalRequest, TransitionCodexTaskProposalRequest,
    },
    IpcError,
};
use crate::application::{
    codex_task_proposal::TransitionCodexTaskProposalInput, CodexTaskProposalKernel,
};

#[tauri::command]
pub async fn create_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    input: CreateCodexTaskProposalRequest,
) -> Result<CodexTaskProposalDto, IpcError> {
    kernel
        .create_proposal(&project_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}
#[tauri::command]
pub async fn list_proposals(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
) -> Result<Vec<CodexTaskProposalDto>, IpcError> {
    kernel
        .list_proposals(&project_id)
        .await
        .map(|values| values.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}
#[tauri::command]
pub async fn get_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
) -> Result<CodexTaskProposalDto, IpcError> {
    kernel
        .get_proposal(&project_id, &proposal_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

enum Action {
    Submit,
    Approve,
    Reject,
    Archive,
}
async fn transition(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
    input: TransitionCodexTaskProposalRequest,
    action: Action,
) -> Result<CodexTaskProposalDto, IpcError> {
    let input = TransitionCodexTaskProposalInput {
        expected_version: input.expected_version,
    };
    let result = match action {
        Action::Submit => {
            kernel
                .submit_proposal(&project_id, &proposal_id, input)
                .await
        }
        Action::Approve => {
            kernel
                .approve_proposal(&project_id, &proposal_id, input)
                .await
        }
        Action::Reject => {
            kernel
                .reject_proposal(&project_id, &proposal_id, input)
                .await
        }
        Action::Archive => {
            kernel
                .archive_proposal(&project_id, &proposal_id, input)
                .await
        }
    };
    result.map(Into::into).map_err(Into::into)
}
#[tauri::command]
pub async fn submit_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
    input: TransitionCodexTaskProposalRequest,
) -> Result<CodexTaskProposalDto, IpcError> {
    transition(kernel, project_id, proposal_id, input, Action::Submit).await
}
#[tauri::command]
pub async fn approve_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
    input: TransitionCodexTaskProposalRequest,
) -> Result<CodexTaskProposalDto, IpcError> {
    transition(kernel, project_id, proposal_id, input, Action::Approve).await
}
#[tauri::command]
pub async fn reject_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
    input: TransitionCodexTaskProposalRequest,
) -> Result<CodexTaskProposalDto, IpcError> {
    transition(kernel, project_id, proposal_id, input, Action::Reject).await
}
#[tauri::command]
pub async fn archive_proposal(
    kernel: State<'_, CodexTaskProposalKernel>,
    project_id: String,
    proposal_id: String,
    input: TransitionCodexTaskProposalRequest,
) -> Result<CodexTaskProposalDto, IpcError> {
    transition(kernel, project_id, proposal_id, input, Action::Archive).await
}
