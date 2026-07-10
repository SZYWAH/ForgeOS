use tauri::State;

use crate::application::{
    project::{CreateProjectInput, UpdateProjectInput},
    ProjectKernel,
};

use super::{
    dto::{CreateProjectRequest, ProjectDto, ProjectSummaryDto, UpdateProjectRequest},
    IpcError,
};

#[tauri::command]
pub async fn create_project(
    kernel: State<'_, ProjectKernel>,
    input: CreateProjectRequest,
) -> Result<ProjectDto, IpcError> {
    kernel
        .create_project(create_input(input))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_projects(
    kernel: State<'_, ProjectKernel>,
) -> Result<Vec<ProjectSummaryDto>, IpcError> {
    kernel
        .list_projects()
        .await
        .map(|projects| projects.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_project(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<ProjectDto, IpcError> {
    kernel
        .get_project(&project_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_project(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    input: UpdateProjectRequest,
) -> Result<ProjectDto, IpcError> {
    kernel
        .update_project(&project_id, update_input(input))
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn archive_project(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    expected_version: i64,
) -> Result<ProjectDto, IpcError> {
    kernel
        .archive_project(&project_id, expected_version)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

fn create_input(input: CreateProjectRequest) -> CreateProjectInput {
    CreateProjectInput {
        slug: input.slug,
        name: input.name,
        description: input.description,
        vision: input.vision,
        target_user: input.target_user,
        scope: input.scope,
        non_goals: input.non_goals,
        tech_stack: input.tech_stack,
        status: input.status,
        lifecycle_stage: input.lifecycle_stage,
        priority: input.priority,
    }
}

fn update_input(input: UpdateProjectRequest) -> UpdateProjectInput {
    UpdateProjectInput {
        slug: input.slug,
        name: input.name,
        description: input.description,
        vision: input.vision,
        target_user: input.target_user,
        scope: input.scope,
        non_goals: input.non_goals,
        tech_stack: input.tech_stack,
        status: input.status,
        lifecycle_stage: input.lifecycle_stage,
        priority: input.priority,
        expected_version: input.expected_version,
    }
}
