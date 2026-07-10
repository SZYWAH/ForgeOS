use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{ProjectSnapshot, SnapshotDetails},
    ports::{ProjectRepository, RepositoryError, SnapshotRepository, WorkspaceRepository},
};

use super::{project, workspace::map_read_error, ApplicationError};

#[derive(Clone, Debug)]
pub struct CreateSnapshotInput {
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct UpdateSnapshotInput {
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub expected_version: i64,
}

pub async fn create_snapshot<R>(
    repository: &R,
    project_id: &str,
    input: CreateSnapshotInput,
) -> Result<ProjectSnapshot, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SnapshotRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_snapshot_changes()?;

    if repository
        .get_current_snapshot(&project.id)
        .await
        .map_err(map_read_error)?
        .is_some()
    {
        return Err(ApplicationError::SnapshotAlreadyExists);
    }

    let snapshot = ProjectSnapshot::new(
        Uuid::now_v7().to_string(),
        project.id,
        create_details(input),
        Utc::now(),
    )?;

    match repository.insert_snapshot(&snapshot).await {
        Ok(()) => Ok(snapshot),
        Err(RepositoryError::Conflict) => Err(ApplicationError::SnapshotAlreadyExists),
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptSnapshots.into())
        }
        Err(error) => Err(map_read_error(error)),
    }
}

pub async fn get_current_snapshot<R>(
    repository: &R,
    project_id: &str,
) -> Result<Option<ProjectSnapshot>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SnapshotRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    repository
        .get_current_snapshot(&project.id)
        .await
        .map_err(map_read_error)
}

pub async fn get_snapshot_history<R>(
    repository: &R,
    project_id: &str,
) -> Result<Vec<ProjectSnapshot>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SnapshotRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    repository
        .list_snapshot_history(&project.id)
        .await
        .map_err(map_read_error)
}

pub async fn update_snapshot<R>(
    repository: &R,
    project_id: &str,
    input: UpdateSnapshotInput,
) -> Result<ProjectSnapshot, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SnapshotRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_snapshot_changes()?;

    let current = repository
        .get_current_snapshot(&project.id)
        .await
        .map_err(map_read_error)?
        .ok_or(ApplicationError::SnapshotNotFound)?;
    ensure_snapshot_version(input.expected_version, current.version())?;

    let expected_version = input.expected_version;
    let next = ProjectSnapshot::next_version(
        Uuid::now_v7().to_string(),
        project.id,
        &current,
        update_details(input),
        Utc::now(),
    )?;

    match repository.insert_snapshot(&next).await {
        Ok(()) => Ok(next),
        Err(RepositoryError::Conflict) | Err(RepositoryError::OptimisticConflict) => {
            let latest = repository
                .get_current_snapshot(next.project_id())
                .await
                .map_err(map_read_error)?
                .ok_or(ApplicationError::SnapshotNotFound)?;
            Err(ApplicationError::SnapshotVersionConflict {
                expected: expected_version,
                actual: latest.version(),
            })
        }
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptSnapshots.into())
        }
        Err(error) => Err(map_read_error(error)),
    }
}

fn create_details(input: CreateSnapshotInput) -> SnapshotDetails {
    SnapshotDetails {
        current_goal: input.current_goal,
        status_summary: input.status_summary,
        achievements: input.achievements,
        blockers: input.blockers,
        next_steps: input.next_steps,
    }
}

fn update_details(input: UpdateSnapshotInput) -> SnapshotDetails {
    SnapshotDetails {
        current_goal: input.current_goal,
        status_summary: input.status_summary,
        achievements: input.achievements,
        blockers: input.blockers,
        next_steps: input.next_steps,
    }
}

fn ensure_snapshot_version(expected: i64, actual: i64) -> Result<(), ApplicationError> {
    if expected != actual {
        return Err(ApplicationError::SnapshotVersionConflict { expected, actual });
    }
    Ok(())
}
