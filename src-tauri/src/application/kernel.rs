use std::sync::Arc;

use crate::{
    domain::{DevelopmentSession, Project, ProjectSnapshot, Workspace},
    ports::ProjectKernelRepository,
};

use super::{
    project::{self, CreateProjectInput, UpdateProjectInput},
    session::{
        self, AbandonSessionInput, CompleteSessionInput, StartSessionInput, UpdateSessionInput,
    },
    snapshot::{self, CreateSnapshotInput, UpdateSnapshotInput},
    workspace::{self, CreateWorkspaceInput},
    ApplicationError,
};

#[derive(Clone)]
pub struct ProjectKernel {
    repository: Arc<dyn ProjectKernelRepository>,
}

impl ProjectKernel {
    pub fn new(repository: Arc<dyn ProjectKernelRepository>) -> Self {
        Self { repository }
    }

    pub async fn get_current_workspace(&self) -> Result<Option<Workspace>, ApplicationError> {
        workspace::get_current_workspace(self.repository.as_ref()).await
    }

    pub async fn create_workspace(
        &self,
        input: CreateWorkspaceInput,
    ) -> Result<Workspace, ApplicationError> {
        workspace::create_workspace(self.repository.as_ref(), input).await
    }

    pub async fn create_project(
        &self,
        input: CreateProjectInput,
    ) -> Result<Project, ApplicationError> {
        project::create_project(self.repository.as_ref(), input).await
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, ApplicationError> {
        project::list_projects(self.repository.as_ref()).await
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Project, ApplicationError> {
        project::get_project(self.repository.as_ref(), project_id).await
    }

    pub async fn update_project(
        &self,
        project_id: &str,
        input: UpdateProjectInput,
    ) -> Result<Project, ApplicationError> {
        project::update_project(self.repository.as_ref(), project_id, input).await
    }

    pub async fn archive_project(
        &self,
        project_id: &str,
        expected_version: i64,
    ) -> Result<Project, ApplicationError> {
        project::archive_project(self.repository.as_ref(), project_id, expected_version).await
    }

    pub async fn create_snapshot(
        &self,
        project_id: &str,
        input: CreateSnapshotInput,
    ) -> Result<ProjectSnapshot, ApplicationError> {
        snapshot::create_snapshot(self.repository.as_ref(), project_id, input).await
    }

    pub async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, ApplicationError> {
        snapshot::get_current_snapshot(self.repository.as_ref(), project_id).await
    }

    pub async fn get_snapshot_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSnapshot>, ApplicationError> {
        snapshot::get_snapshot_history(self.repository.as_ref(), project_id).await
    }

    pub async fn update_snapshot(
        &self,
        project_id: &str,
        input: UpdateSnapshotInput,
    ) -> Result<ProjectSnapshot, ApplicationError> {
        snapshot::update_snapshot(self.repository.as_ref(), project_id, input).await
    }

    pub async fn start_session(
        &self,
        project_id: &str,
        input: StartSessionInput,
    ) -> Result<DevelopmentSession, ApplicationError> {
        session::start_session(self.repository.as_ref(), project_id, input).await
    }

    pub async fn get_current_session(
        &self,
        project_id: &str,
    ) -> Result<Option<DevelopmentSession>, ApplicationError> {
        session::get_current_session(self.repository.as_ref(), project_id).await
    }

    pub async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, ApplicationError> {
        session::list_sessions(self.repository.as_ref(), project_id).await
    }

    pub async fn update_session(
        &self,
        project_id: &str,
        session_id: &str,
        input: UpdateSessionInput,
    ) -> Result<DevelopmentSession, ApplicationError> {
        session::update_session(self.repository.as_ref(), project_id, session_id, input).await
    }

    pub async fn complete_session(
        &self,
        project_id: &str,
        session_id: &str,
        input: CompleteSessionInput,
    ) -> Result<DevelopmentSession, ApplicationError> {
        session::complete_session(self.repository.as_ref(), project_id, session_id, input).await
    }

    pub async fn abandon_session(
        &self,
        project_id: &str,
        session_id: &str,
        input: AbandonSessionInput,
    ) -> Result<DevelopmentSession, ApplicationError> {
        session::abandon_session(self.repository.as_ref(), project_id, session_id, input).await
    }
}
