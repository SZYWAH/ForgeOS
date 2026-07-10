use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{DevelopmentSession, Project, ProjectSnapshot, Workspace};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a record with the same unique identity already exists")]
    Conflict,

    #[error("the record changed after it was read")]
    OptimisticConflict,

    #[error("the Project is archived and cannot accept this change")]
    ProjectArchived,

    #[error("persisted ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("storage operation failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError>;
    async fn insert_workspace(&self, workspace: &Workspace) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn insert_project(&self, project: &Project) -> Result<(), RepositoryError>;

    async fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, RepositoryError>;

    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError>;

    async fn update_project(
        &self,
        project: &Project,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    async fn insert_snapshot(&self, snapshot: &ProjectSnapshot) -> Result<(), RepositoryError>;

    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError>;

    async fn list_snapshot_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSnapshot>, RepositoryError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn insert_session(&self, session: &DevelopmentSession) -> Result<(), RepositoryError>;

    async fn get_current_session(
        &self,
        project_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError>;

    async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, RepositoryError>;

    async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError>;

    async fn update_open_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;

    async fn finish_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

pub trait ProjectKernelRepository:
    WorkspaceRepository + ProjectRepository + SnapshotRepository + SessionRepository
{
}

impl<T> ProjectKernelRepository for T where
    T: WorkspaceRepository + ProjectRepository + SnapshotRepository + SessionRepository
{
}
