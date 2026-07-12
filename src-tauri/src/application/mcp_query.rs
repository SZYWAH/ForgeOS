use std::{str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;

use crate::{
    domain::{ContextExport, ContextTemplateType, DomainError, Project},
    ports::{ContextRenderInput, ContextRenderer, McpQueryRepository, RepositoryError},
};

const RECENT_SESSION_LIMIT: usize = 5;

#[derive(Debug, Error)]
pub enum McpQueryError {
    #[error("Project not found in the current ForgeOS Workspace")]
    ProjectNotFound,
    #[error("ForgeOS has no Workspace")]
    WorkspaceNotFound,
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("Context Pack rendering failed: {0}")]
    Render(String),
    #[error("persisted ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProjectIdentityView {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub tech_stack: String,
    pub lifecycle_stage: String,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CurrentSnapshotView {
    pub project_id: String,
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub snapshot_version: i64,
    pub confirmed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecentSessionView {
    pub goal: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceView {
    pub title: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub description: String,
    pub verification_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ContextPackView {
    pub project_id: String,
    pub template_type: String,
    pub snapshot_version: Option<i64>,
    pub generated_from_session_ids: Vec<String>,
    pub content_markdown: String,
    pub content_hash: String,
}

#[derive(Clone)]
pub struct McpQueryKernel {
    repository: Arc<dyn McpQueryRepository>,
    renderer: Arc<dyn ContextRenderer>,
}

impl McpQueryKernel {
    pub fn new(
        repository: Arc<dyn McpQueryRepository>,
        renderer: Arc<dyn ContextRenderer>,
    ) -> Self {
        Self {
            repository,
            renderer,
        }
    }

    pub async fn get_project_identity(
        &self,
        project_id: &str,
    ) -> Result<ProjectIdentityView, McpQueryError> {
        let project = self.require_project(project_id).await?;
        Ok(ProjectIdentityView {
            project_id: project.id,
            name: project.name,
            description: project.description,
            vision: project.vision,
            target_user: project.target_user,
            tech_stack: project.tech_stack,
            lifecycle_stage: project.lifecycle_stage.as_str().into(),
            status: project.status.as_str().into(),
        })
    }

    pub async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<CurrentSnapshotView>, McpQueryError> {
        self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(snapshot.map(|value| CurrentSnapshotView {
            project_id: value.project_id().into(),
            current_goal: value.current_goal().into(),
            status_summary: value.status_summary().into(),
            achievements: value.achievements().to_vec(),
            blockers: value.blockers().to_vec(),
            next_steps: value.next_steps().to_vec(),
            snapshot_version: value.version(),
            confirmed_at: value.confirmed_at(),
        }))
    }

    pub async fn get_recent_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<RecentSessionView>, McpQueryError> {
        self.require_project(project_id).await?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(sessions
            .into_iter()
            .take(RECENT_SESSION_LIMIT)
            .map(|value| RecentSessionView {
                goal: value.goal().into(),
                outcome: value.outcome().into(),
                validation: value.validation().into(),
                handoff: value.handoff().into(),
                started_at: value.started_at(),
                completed_at: value.completed_at(),
            })
            .collect())
    }

    pub async fn list_evidence(
        &self,
        project_id: &str,
    ) -> Result<Vec<EvidenceView>, McpQueryError> {
        self.require_project(project_id).await?;
        let evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(evidences
            .into_iter()
            .map(|value| EvidenceView {
                title: value.title().into(),
                evidence_type: value.evidence_type().as_str().into(),
                description: value.description().into(),
                verification_status: value.verification_status().as_str().into(),
            })
            .collect())
    }

    /// Builds the existing deterministic Context Pack representation in memory.
    /// No ContextExport row is inserted or updated by this read-only query.
    pub async fn build_context_pack(
        &self,
        project_id: &str,
        template_type: &str,
    ) -> Result<ContextPackView, McpQueryError> {
        let template_type = ContextTemplateType::from_str(template_type)?;
        let project = self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        let recent_session = sessions.first();
        let mut evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        evidences.sort_by(|left, right| {
            left.title()
                .to_lowercase()
                .cmp(&right.title().to_lowercase())
                .then_with(|| left.id().cmp(right.id()))
        });

        let session_project_ids = recent_session
            .into_iter()
            .map(|session| session.project_id())
            .collect::<Vec<_>>();
        let evidence_project_ids = evidences
            .iter()
            .map(|evidence| evidence.project_id())
            .collect::<Vec<_>>();
        ContextExport::validate_sources(
            project_id,
            snapshot.as_ref().map(|value| value.project_id()),
            &session_project_ids,
            &evidence_project_ids,
        )?;

        let markdown = self
            .renderer
            .render(ContextRenderInput {
                template_type,
                project: &project,
                snapshot: snapshot.as_ref(),
                recent_session,
                evidences: &evidences,
            })
            .map_err(|error| McpQueryError::Render(error.to_string()))?;
        Ok(ContextPackView {
            project_id: project.id,
            template_type: template_type.as_str().into(),
            snapshot_version: snapshot.as_ref().map(|value| value.version()),
            generated_from_session_ids: recent_session
                .into_iter()
                .map(|value| value.id().into())
                .collect(),
            content_hash: ContextExport::calculate_hash(&markdown),
            content_markdown: markdown,
        })
    }

    async fn require_project(&self, project_id: &str) -> Result<Project, McpQueryError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(McpQueryError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(McpQueryError::ProjectNotFound)
    }
}

fn map_repository_error(error: RepositoryError) -> McpQueryError {
    match error {
        RepositoryError::CorruptData(message) => McpQueryError::CorruptData(message),
        other => McpQueryError::Storage(other.to_string()),
    }
}
