use tauri::State;

use crate::application::{
    session::{AbandonSessionInput, CompleteSessionInput, StartSessionInput, UpdateSessionInput},
    ProjectKernel,
};

use super::{
    dto::{
        AbandonSessionRequest, CompleteSessionRequest, DevelopmentSessionDto, StartSessionRequest,
        UpdateSessionRequest,
    },
    IpcError,
};

#[tauri::command]
pub async fn start_session(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    input: StartSessionRequest,
) -> Result<DevelopmentSessionDto, IpcError> {
    kernel
        .start_session(&project_id, StartSessionInput { goal: input.goal })
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_current_session(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<Option<DevelopmentSessionDto>, IpcError> {
    kernel
        .get_current_session(&project_id)
        .await
        .map(|session| session.map(Into::into))
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_sessions(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<Vec<DevelopmentSessionDto>, IpcError> {
    kernel
        .list_sessions(&project_id)
        .await
        .map(|sessions| sessions.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_session(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    session_id: String,
    input: UpdateSessionRequest,
) -> Result<DevelopmentSessionDto, IpcError> {
    kernel
        .update_session(
            &project_id,
            &session_id,
            UpdateSessionInput {
                goal: input.goal,
                notes: input.notes,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn complete_session(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    session_id: String,
    input: CompleteSessionRequest,
) -> Result<DevelopmentSessionDto, IpcError> {
    kernel
        .complete_session(
            &project_id,
            &session_id,
            CompleteSessionInput {
                outcome: input.outcome,
                validation: input.validation,
                handoff: input.handoff,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn abandon_session(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    session_id: String,
    input: AbandonSessionRequest,
) -> Result<DevelopmentSessionDto, IpcError> {
    kernel
        .abandon_session(
            &project_id,
            &session_id,
            AbandonSessionInput {
                handoff: input.handoff,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}
