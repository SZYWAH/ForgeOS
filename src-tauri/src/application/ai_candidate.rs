use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{AiCandidate, AiCandidateType, DomainError},
    ports::{AiCandidateKernelRepository, RepositoryError},
};

#[derive(Debug, Error)]
pub enum AiCandidateError {
    #[error(transparent)]
    Validation(#[from] DomainError),

    #[error("create a Workspace before using AI Candidates")]
    WorkspaceNotFound,

    #[error("Project not found")]
    ProjectNotFound,

    #[error("source Context Export not found")]
    ContextExportNotFound,

    #[error("AI Candidate not found")]
    CandidateNotFound,

    #[error("AI Candidate version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },

    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct CreateAiCandidateInput {
    pub candidate_type: String,
    pub source_context_export_id: String,
    pub title: String,
    pub content_markdown: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ReviewAiCandidateInput {
    pub expected_version: i64,
}

#[derive(Clone)]
pub struct AiCandidateKernel {
    repository: Arc<dyn AiCandidateKernelRepository>,
}

impl AiCandidateKernel {
    pub fn new(repository: Arc<dyn AiCandidateKernelRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_ai_candidate(
        &self,
        project_id: &str,
        input: CreateAiCandidateInput,
    ) -> Result<AiCandidate, AiCandidateError> {
        self.require_project(project_id).await?;
        let source = self
            .repository
            .get_context_export(&input.source_context_export_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AiCandidateError::ContextExportNotFound)?;
        source.ensure_project(project_id)?;

        let candidate = AiCandidate::create(
            Uuid::now_v7().to_string(),
            project_id.to_owned(),
            AiCandidateType::from_str(&input.candidate_type)?,
            source.id().to_owned(),
            input.title,
            input.content_markdown,
            Utc::now(),
        )?;
        candidate.ensure_source_project(source.project_id())?;
        self.repository
            .insert_ai_candidate(&candidate)
            .await
            .map_err(map_repository_error)?;
        Ok(candidate)
    }

    pub async fn list_candidates(
        &self,
        project_id: &str,
    ) -> Result<Vec<AiCandidate>, AiCandidateError> {
        self.require_project(project_id).await?;
        self.repository
            .list_ai_candidates(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn get_candidate(
        &self,
        project_id: &str,
        candidate_id: &str,
    ) -> Result<AiCandidate, AiCandidateError> {
        self.require_project(project_id).await?;
        let candidate = self
            .repository
            .get_ai_candidate(candidate_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AiCandidateError::CandidateNotFound)?;
        candidate.ensure_project(project_id)?;
        Ok(candidate)
    }

    pub async fn accept_candidate(
        &self,
        project_id: &str,
        candidate_id: &str,
        input: ReviewAiCandidateInput,
    ) -> Result<AiCandidate, AiCandidateError> {
        self.transition(project_id, candidate_id, input, AiCandidate::accept)
            .await
    }

    pub async fn reject_candidate(
        &self,
        project_id: &str,
        candidate_id: &str,
        input: ReviewAiCandidateInput,
    ) -> Result<AiCandidate, AiCandidateError> {
        self.transition(project_id, candidate_id, input, AiCandidate::reject)
            .await
    }

    pub async fn archive_candidate(
        &self,
        project_id: &str,
        candidate_id: &str,
        input: ReviewAiCandidateInput,
    ) -> Result<AiCandidate, AiCandidateError> {
        self.transition(project_id, candidate_id, input, AiCandidate::archive)
            .await
    }

    async fn transition(
        &self,
        project_id: &str,
        candidate_id: &str,
        input: ReviewAiCandidateInput,
        transition: fn(&mut AiCandidate, chrono::DateTime<Utc>) -> Result<(), DomainError>,
    ) -> Result<AiCandidate, AiCandidateError> {
        let mut candidate = self.get_candidate(project_id, candidate_id).await?;
        ensure_version(input.expected_version, candidate.version())?;
        transition(&mut candidate, Utc::now())?;
        match self
            .repository
            .update_ai_candidate_status(&candidate, input.expected_version)
            .await
        {
            Ok(()) => Ok(candidate),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.get_candidate(project_id, candidate_id).await?;
                Err(AiCandidateError::VersionConflict {
                    expected: input.expected_version,
                    actual: current.version(),
                })
            }
            Err(error) => Err(map_repository_error(error)),
        }
    }

    async fn require_project(&self, project_id: &str) -> Result<(), AiCandidateError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(AiCandidateError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AiCandidateError::ProjectNotFound)?;
        Ok(())
    }
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), AiCandidateError> {
    if expected != actual {
        return Err(AiCandidateError::VersionConflict { expected, actual });
    }
    Ok(())
}

fn map_repository_error(error: RepositoryError) -> AiCandidateError {
    match error {
        RepositoryError::OptimisticConflict => AiCandidateError::Storage(error.to_string()),
        RepositoryError::CorruptData(message) => AiCandidateError::CorruptData(message),
        RepositoryError::Storage(message) => AiCandidateError::Storage(message),
        other => AiCandidateError::Storage(other.to_string()),
    }
}
