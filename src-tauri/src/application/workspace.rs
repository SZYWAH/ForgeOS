use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{DomainError, Workspace},
    ports::{RepositoryError, WorkspaceRepository},
};

use super::ApplicationError;

#[derive(Debug)]
pub struct CreateWorkspaceInput {
    pub name: String,
    pub timezone: String,
}

pub async fn get_current_workspace<R>(repository: &R) -> Result<Option<Workspace>, ApplicationError>
where
    R: WorkspaceRepository + ?Sized,
{
    repository
        .get_current_workspace()
        .await
        .map_err(map_read_error)
}

pub async fn require_current_workspace<R>(repository: &R) -> Result<Workspace, ApplicationError>
where
    R: WorkspaceRepository + ?Sized,
{
    get_current_workspace(repository)
        .await?
        .ok_or(ApplicationError::WorkspaceNotFound)
}

pub async fn create_workspace<R>(
    repository: &R,
    input: CreateWorkspaceInput,
) -> Result<Workspace, ApplicationError>
where
    R: WorkspaceRepository + ?Sized,
{
    if repository
        .get_current_workspace()
        .await
        .map_err(map_read_error)?
        .is_some()
    {
        return Err(ApplicationError::WorkspaceAlreadyExists);
    }

    let workspace = Workspace::new(
        Uuid::now_v7().to_string(),
        input.name,
        input.timezone,
        Utc::now(),
    )?;

    match repository.insert_workspace(&workspace).await {
        Ok(()) => Ok(workspace),
        Err(RepositoryError::Conflict) => Err(ApplicationError::WorkspaceAlreadyExists),
        Err(error) => Err(map_read_error(error)),
    }
}

pub(crate) fn map_read_error(error: RepositoryError) -> ApplicationError {
    match error {
        RepositoryError::CorruptData(message) => ApplicationError::CorruptData(message),
        RepositoryError::Storage(message) => ApplicationError::Storage(message),
        RepositoryError::ProjectArchived => {
            ApplicationError::Validation(DomainError::ArchivedProjectCannotAcceptSnapshots)
        }
        RepositoryError::EvidenceTargetInvalid => ApplicationError::CorruptData(error.to_string()),
        RepositoryError::Conflict | RepositoryError::OptimisticConflict => {
            ApplicationError::Storage(error.to_string())
        }
    }
}
