use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{Project, Workspace};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a record with the same unique identity already exists")]
    Conflict,

    #[error("the record changed after it was read")]
    OptimisticConflict,

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

pub trait ProjectKernelRepository: WorkspaceRepository + ProjectRepository {}

impl<T> ProjectKernelRepository for T where T: WorkspaceRepository + ProjectRepository {}
