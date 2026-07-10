use std::sync::Arc;

use crate::{
    domain::{Project, Workspace},
    ports::ProjectKernelRepository,
};

use super::{
    project::{self, CreateProjectInput, UpdateProjectInput},
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

}
