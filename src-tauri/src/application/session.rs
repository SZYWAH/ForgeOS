use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::DevelopmentSession,
    ports::{ProjectRepository, RepositoryError, SessionRepository, WorkspaceRepository},
};

use super::{project, workspace::map_read_error, ApplicationError};

#[derive(Clone, Debug)]
pub struct StartSessionInput {
    pub goal: String,
}

#[derive(Clone, Debug)]
pub struct UpdateSessionInput {
    pub goal: String,
    pub notes: String,
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct CompleteSessionInput {
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct AbandonSessionInput {
    pub handoff: String,
    pub expected_version: i64,
}

pub async fn start_session<R>(
    repository: &R,
    project_id: &str,
    input: StartSessionInput,
) -> Result<DevelopmentSession, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_session_changes()?;

    if repository
        .get_current_session(&project.id)
        .await
        .map_err(map_read_error)?
        .is_some()
    {
        return Err(ApplicationError::SessionAlreadyOpen);
    }

    let session = DevelopmentSession::start(
        Uuid::now_v7().to_string(),
        project.id,
        input.goal,
        Utc::now(),
    )?;

    match repository.insert_session(&session).await {
        Ok(()) => Ok(session),
        Err(RepositoryError::Conflict) => Err(ApplicationError::SessionAlreadyOpen),
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptSessionChanges.into())
        }
        Err(error) => Err(map_read_error(error)),
    }
}

pub async fn get_current_session<R>(
    repository: &R,
    project_id: &str,
) -> Result<Option<DevelopmentSession>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    repository
        .get_current_session(&project.id)
        .await
        .map_err(map_read_error)
}

pub async fn list_sessions<R>(
    repository: &R,
    project_id: &str,
) -> Result<Vec<DevelopmentSession>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    repository
        .list_sessions(&project.id)
        .await
        .map_err(map_read_error)
}

pub async fn update_session<R>(
    repository: &R,
    project_id: &str,
    session_id: &str,
    input: UpdateSessionInput,
) -> Result<DevelopmentSession, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_session_changes()?;
    let mut session = require_session(repository, session_id).await?;
    session.ensure_project(&project.id)?;
    ensure_version(input.expected_version, session.version())?;

    let expected_version = input.expected_version;
    session.update_open(input.goal, input.notes, Utc::now())?;
    persist_open_update(repository, &session, expected_version).await?;
    Ok(session)
}

pub async fn complete_session<R>(
    repository: &R,
    project_id: &str,
    session_id: &str,
    input: CompleteSessionInput,
) -> Result<DevelopmentSession, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    let mut session = require_session(repository, session_id).await?;
    session.ensure_project(&project.id)?;
    ensure_version(input.expected_version, session.version())?;

    let expected_version = input.expected_version;
    session.complete(input.outcome, input.validation, input.handoff, Utc::now())?;
    persist_terminal_transition(repository, &session, expected_version).await?;
    Ok(session)
}

pub async fn abandon_session<R>(
    repository: &R,
    project_id: &str,
    session_id: &str,
    input: AbandonSessionInput,
) -> Result<DevelopmentSession, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + SessionRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    let mut session = require_session(repository, session_id).await?;
    session.ensure_project(&project.id)?;
    ensure_version(input.expected_version, session.version())?;

    let expected_version = input.expected_version;
    session.abandon(input.handoff, Utc::now())?;
    persist_terminal_transition(repository, &session, expected_version).await?;
    Ok(session)
}

async fn require_session<R>(
    repository: &R,
    session_id: &str,
) -> Result<DevelopmentSession, ApplicationError>
where
    R: SessionRepository + ?Sized,
{
    repository
        .get_session(session_id)
        .await
        .map_err(map_read_error)?
        .ok_or(ApplicationError::SessionNotFound)
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), ApplicationError> {
    if expected != actual {
        return Err(ApplicationError::SessionVersionConflict { expected, actual });
    }
    Ok(())
}

async fn persist_open_update<R>(
    repository: &R,
    session: &DevelopmentSession,
    expected_version: i64,
) -> Result<(), ApplicationError>
where
    R: SessionRepository + ?Sized,
{
    match repository
        .update_open_session(session, expected_version)
        .await
    {
        Ok(()) => Ok(()),
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptSessionChanges.into())
        }
        Err(RepositoryError::OptimisticConflict) => {
            version_conflict(repository, session.id(), expected_version).await
        }
        Err(error) => Err(map_read_error(error)),
    }
}

async fn persist_terminal_transition<R>(
    repository: &R,
    session: &DevelopmentSession,
    expected_version: i64,
) -> Result<(), ApplicationError>
where
    R: SessionRepository + ?Sized,
{
    match repository.finish_session(session, expected_version).await {
        Ok(()) => Ok(()),
        Err(RepositoryError::OptimisticConflict) => {
            version_conflict(repository, session.id(), expected_version).await
        }
        Err(error) => Err(map_read_error(error)),
    }
}

async fn version_conflict<R>(
    repository: &R,
    session_id: &str,
    expected: i64,
) -> Result<(), ApplicationError>
where
    R: SessionRepository + ?Sized,
{
    let current = require_session(repository, session_id).await?;
    Err(ApplicationError::SessionVersionConflict {
        expected,
        actual: current.version(),
    })
}
