use tauri::State;

use crate::application::{ai_candidate::ReviewAiCandidateInput, AiCandidateKernel};

use super::{
    dto::{AiCandidateDto, CreateAiCandidateRequest, ReviewAiCandidateRequest},
    IpcError,
};

#[tauri::command]
pub async fn create_ai_candidate(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    input: CreateAiCandidateRequest,
) -> Result<AiCandidateDto, IpcError> {
    kernel
        .create_ai_candidate(&project_id, input.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_candidates(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
) -> Result<Vec<AiCandidateDto>, IpcError> {
    kernel
        .list_candidates(&project_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_candidate(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    candidate_id: String,
) -> Result<AiCandidateDto, IpcError> {
    kernel
        .get_candidate(&project_id, &candidate_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn accept_candidate(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    candidate_id: String,
    input: ReviewAiCandidateRequest,
) -> Result<AiCandidateDto, IpcError> {
    transition(kernel, project_id, candidate_id, input, Transition::Accept).await
}

#[tauri::command]
pub async fn reject_candidate(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    candidate_id: String,
    input: ReviewAiCandidateRequest,
) -> Result<AiCandidateDto, IpcError> {
    transition(kernel, project_id, candidate_id, input, Transition::Reject).await
}

#[tauri::command]
pub async fn archive_candidate(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    candidate_id: String,
    input: ReviewAiCandidateRequest,
) -> Result<AiCandidateDto, IpcError> {
    transition(kernel, project_id, candidate_id, input, Transition::Archive).await
}

enum Transition {
    Accept,
    Reject,
    Archive,
}

async fn transition(
    kernel: State<'_, AiCandidateKernel>,
    project_id: String,
    candidate_id: String,
    input: ReviewAiCandidateRequest,
    transition: Transition,
) -> Result<AiCandidateDto, IpcError> {
    let input = ReviewAiCandidateInput {
        expected_version: input.expected_version,
    };
    let result = match transition {
        Transition::Accept => {
            kernel
                .accept_candidate(&project_id, &candidate_id, input)
                .await
        }
        Transition::Reject => {
            kernel
                .reject_candidate(&project_id, &candidate_id, input)
                .await
        }
        Transition::Archive => {
            kernel
                .archive_candidate(&project_id, &candidate_id, input)
                .await
        }
    };
    result.map(Into::into).map_err(Into::into)
}
