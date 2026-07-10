use tauri::State;

use crate::application::{workspace::CreateWorkspaceInput, ProjectKernel};

use super::{
    dto::{CreateWorkspaceRequest, WorkspaceDto},
    IpcError,
};

#[tauri::command]
pub async fn get_current_workspace(
    kernel: State<'_, ProjectKernel>,
) -> Result<Option<WorkspaceDto>, IpcError> {
    kernel
        .get_current_workspace()
        .await
        .map(|workspace| workspace.map(Into::into))
        .map_err(Into::into)
}

#[tauri::command]
pub async fn create_workspace(
    kernel: State<'_, ProjectKernel>,
    input: CreateWorkspaceRequest,
) -> Result<WorkspaceDto, IpcError> {
    kernel
        .create_workspace(CreateWorkspaceInput {
            name: input.name,
            timezone: input.timezone,
        })
        .await
        .map(Into::into)
        .map_err(Into::into)
}
