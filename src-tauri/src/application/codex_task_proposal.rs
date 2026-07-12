use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{CodexTaskProposal, CodexTaskProposalDetails, DomainError},
    ports::{CodexTaskProposalKernelRepository, RepositoryError},
};

#[derive(Debug, Error)]
pub enum CodexTaskProposalError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before using Codex Task Proposals")]
    WorkspaceNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Context Export not found")]
    ContextExportNotFound,
    #[error("Codex Task Proposal not found")]
    ProposalNotFound,
    #[error("Codex Task Proposal version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },
    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct CreateCodexTaskProposalInput {
    pub context_export_id: String,
    pub title: String,
    pub description: String,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub allowed_scope: Vec<String>,
    pub forbidden_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub validation_commands: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct TransitionCodexTaskProposalInput {
    pub expected_version: i64,
}

#[derive(Clone)]
pub struct CodexTaskProposalKernel {
    repository: Arc<dyn CodexTaskProposalKernelRepository>,
}

impl CodexTaskProposalKernel {
    pub fn new(repository: Arc<dyn CodexTaskProposalKernelRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_proposal(
        &self,
        project_id: &str,
        input: CreateCodexTaskProposalInput,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.require_project(project_id).await?;
        let context = self
            .repository
            .get_context_export(&input.context_export_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexTaskProposalError::ContextExportNotFound)?;
        context.ensure_project(project_id)?;
        let proposal = CodexTaskProposal::create(
            Uuid::now_v7().to_string(),
            project_id.into(),
            context.id().into(),
            CodexTaskProposalDetails {
                title: input.title,
                description: input.description,
                goal: input.goal,
                non_goals: input.non_goals,
                allowed_scope: input.allowed_scope,
                forbidden_scope: input.forbidden_scope,
                acceptance_criteria: input.acceptance_criteria,
                validation_commands: input.validation_commands,
            },
            Utc::now(),
        )?;
        self.repository
            .insert_codex_task_proposal(&proposal)
            .await
            .map_err(map_repository_error)?;
        Ok(proposal)
    }

    pub async fn list_proposals(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexTaskProposal>, CodexTaskProposalError> {
        self.require_project(project_id).await?;
        self.repository
            .list_codex_task_proposals(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn get_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.require_project(project_id).await?;
        let value = self
            .repository
            .get_codex_task_proposal(proposal_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexTaskProposalError::ProposalNotFound)?;
        value.ensure_project(project_id)?;
        Ok(value)
    }

    pub async fn submit_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
        input: TransitionCodexTaskProposalInput,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.transition(project_id, proposal_id, input, CodexTaskProposal::propose)
            .await
    }
    pub async fn approve_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
        input: TransitionCodexTaskProposalInput,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.transition(project_id, proposal_id, input, CodexTaskProposal::approve)
            .await
    }
    pub async fn reject_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
        input: TransitionCodexTaskProposalInput,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.transition(project_id, proposal_id, input, CodexTaskProposal::reject)
            .await
    }
    pub async fn archive_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
        input: TransitionCodexTaskProposalInput,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        self.transition(project_id, proposal_id, input, CodexTaskProposal::archive)
            .await
    }

    async fn transition(
        &self,
        project_id: &str,
        proposal_id: &str,
        input: TransitionCodexTaskProposalInput,
        action: fn(&mut CodexTaskProposal, chrono::DateTime<Utc>) -> Result<(), DomainError>,
    ) -> Result<CodexTaskProposal, CodexTaskProposalError> {
        let mut proposal = self.get_proposal(project_id, proposal_id).await?;
        if proposal.version() != input.expected_version {
            return Err(CodexTaskProposalError::VersionConflict {
                expected: input.expected_version,
                actual: proposal.version(),
            });
        }
        let from_status = proposal.status().as_str();
        action(&mut proposal, Utc::now())?;
        match self
            .repository
            .transition_codex_task_proposal(&proposal, from_status, input.expected_version)
            .await
        {
            Ok(()) => Ok(proposal),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.get_proposal(project_id, proposal_id).await?;
                Err(CodexTaskProposalError::VersionConflict {
                    expected: input.expected_version,
                    actual: current.version(),
                })
            }
            Err(error) => Err(map_repository_error(error)),
        }
    }

    async fn require_project(&self, project_id: &str) -> Result<(), CodexTaskProposalError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexTaskProposalError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexTaskProposalError::ProjectNotFound)?;
        Ok(())
    }
}

fn map_repository_error(error: RepositoryError) -> CodexTaskProposalError {
    match error {
        RepositoryError::CorruptData(value) => CodexTaskProposalError::CorruptData(value),
        RepositoryError::Storage(value) => CodexTaskProposalError::Storage(value),
        other => CodexTaskProposalError::Storage(other.to_string()),
    }
}
