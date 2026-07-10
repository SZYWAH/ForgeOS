use tauri::State;

use crate::application::{
    snapshot::{CreateSnapshotInput, UpdateSnapshotInput},
    ProjectKernel,
};

use super::{
    dto::{CreateSnapshotRequest, ProjectSnapshotDto, UpdateSnapshotRequest},
    IpcError,
};

#[tauri::command]
pub async fn create_snapshot(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    input: CreateSnapshotRequest,
) -> Result<ProjectSnapshotDto, IpcError> {
    kernel
        .create_snapshot(&project_id, create_input(input))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_current_snapshot(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<Option<ProjectSnapshotDto>, IpcError> {
    kernel
        .get_current_snapshot(&project_id)
        .await
        .map(|snapshot| snapshot.map(Into::into))
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_snapshot_history(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<Vec<ProjectSnapshotDto>, IpcError> {
    kernel
        .get_snapshot_history(&project_id)
        .await
        .map(|snapshots| snapshots.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_snapshot(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    input: UpdateSnapshotRequest,
) -> Result<ProjectSnapshotDto, IpcError> {
    kernel
        .update_snapshot(&project_id, update_input(input))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn create_input(input: CreateSnapshotRequest) -> CreateSnapshotInput {
    CreateSnapshotInput {
        current_goal: input.current_goal,
        status_summary: input.status_summary,
        achievements: input.achievements,
        blockers: input.blockers,
        next_steps: input.next_steps,
    }
}

fn update_input(input: UpdateSnapshotRequest) -> UpdateSnapshotInput {
    UpdateSnapshotInput {
        current_goal: input.current_goal,
        status_summary: input.status_summary,
        achievements: input.achievements,
        blockers: input.blockers,
        next_steps: input.next_steps,
        expected_version: input.expected_version,
    }
}
