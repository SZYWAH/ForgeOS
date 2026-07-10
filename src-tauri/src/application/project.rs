use std::str::FromStr;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{LifecycleStage, Project, ProjectDetails, ProjectStatus, ProjectUpdate},
    ports::{ProjectRepository, RepositoryError, WorkspaceRepository},
};

use super::{workspace::require_current_workspace, ApplicationError};

#[derive(Debug)]
pub struct CreateProjectInput {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
}

#[derive(Debug)]
pub struct UpdateProjectInput {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub expected_version: i64,
}

pub async fn create_project<R>(
    repository: &R,
    input: CreateProjectInput,
) -> Result<Project, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + ?Sized,
{
    let workspace = require_current_workspace(repository).await?;
    let project = Project::new(
        Uuid::now_v7().to_string(),
        workspace.id,
        details(
            input.slug,
            input.name,
            input.description,
            input.vision,
            input.target_user,
            input.scope,
            input.non_goals,
            input.tech_stack,
            input.status,
            input.lifecycle_stage,
            input.priority,
        )?,
        Utc::now(),
    )?;

    match repository.insert_project(&project).await {
        Ok(()) => Ok(project),
        Err(RepositoryError::Conflict) => Err(ApplicationError::ProjectSlugConflict),
        Err(error) => Err(super::workspace::map_read_error(error)),
    }
}

pub async fn list_projects<R>(repository: &R) -> Result<Vec<Project>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + ?Sized,
{
    let workspace = require_current_workspace(repository).await?;
    repository
        .list_projects(&workspace.id)
        .await
        .map_err(super::workspace::map_read_error)
}

pub async fn get_project<R>(repository: &R, project_id: &str) -> Result<Project, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + ?Sized,
{
    let workspace = require_current_workspace(repository).await?;
    repository
        .get_project(&workspace.id, project_id)
        .await
        .map_err(super::workspace::map_read_error)?
        .ok_or(ApplicationError::ProjectNotFound)
}

pub async fn update_project<R>(
    repository: &R,
    project_id: &str,
    input: UpdateProjectInput,
) -> Result<Project, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + ?Sized,
{
    let mut project = get_project(repository, project_id).await?;
    ensure_version(input.expected_version, project.version)?;

    let expected_version = input.expected_version;
    project.update(
        ProjectUpdate(details(
            input.slug,
            input.name,
            input.description,
            input.vision,
            input.target_user,
            input.scope,
            input.non_goals,
            input.tech_stack,
            input.status,
            input.lifecycle_stage,
            input.priority,
        )?),
        Utc::now(),
    )?;

    persist_update(repository, &project, expected_version).await?;
    Ok(project)
}

pub async fn archive_project<R>(
    repository: &R,
    project_id: &str,
    expected_version: i64,
) -> Result<Project, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + ?Sized,
{
    let mut project = get_project(repository, project_id).await?;
    ensure_version(expected_version, project.version)?;

    if project.archive(Utc::now()) {
        persist_update(repository, &project, expected_version).await?;
    }

    Ok(project)
}

#[allow(clippy::too_many_arguments)]
fn details(
    slug: String,
    name: String,
    description: String,
    vision: String,
    target_user: String,
    scope: String,
    non_goals: String,
    tech_stack: String,
    status: String,
    lifecycle_stage: String,
    priority: i32,
) -> Result<ProjectDetails, ApplicationError> {
    Ok(ProjectDetails {
        slug,
        name,
        description,
        vision,
        target_user,
        scope,
        non_goals,
        tech_stack,
        status: ProjectStatus::from_str(&status)?,
        lifecycle_stage: LifecycleStage::from_str(&lifecycle_stage)?,
        priority,
    })
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), ApplicationError> {
    if expected != actual {
        return Err(ApplicationError::VersionConflict { expected, actual });
    }
    Ok(())
}

async fn persist_update<R>(
    repository: &R,
    project: &Project,
    expected_version: i64,
) -> Result<(), ApplicationError>
where
    R: ProjectRepository + ?Sized,
{
    match repository.update_project(project, expected_version).await {
        Ok(()) => Ok(()),
        Err(RepositoryError::OptimisticConflict) => {
            let current = repository
                .get_project(&project.workspace_id, &project.id)
                .await
                .map_err(super::workspace::map_read_error)?
                .ok_or(ApplicationError::ProjectNotFound)?;
            Err(ApplicationError::VersionConflict {
                expected: expected_version,
                actual: current.version,
            })
        }
        Err(RepositoryError::Conflict) => Err(ApplicationError::ProjectSlugConflict),
        Err(error) => Err(super::workspace::map_read_error(error)),
    }
}
