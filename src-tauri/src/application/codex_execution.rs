use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        CodexExecutionRecord, CodexTaskProposal, CodexTaskProposalStatus, DomainError,
        ProjectStatus,
    },
    ports::{
        CodexExecutionKernelRepository, CodexExecutionRequest, CodexRunner, CodexRunnerError,
        RepositoryError,
    },
};

#[derive(Debug, Error)]
pub enum CodexExecutionError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before using Codex Executions")]
    WorkspaceNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("an archived Project cannot create a Codex Execution request")]
    ProjectArchived,
    #[error("Codex Task Proposal not found")]
    ProposalNotFound,
    #[error("only an approved Codex Task Proposal can create an Execution request")]
    ProposalNotApproved,
    #[error("Codex Execution not found")]
    ExecutionNotFound,
    #[error("Codex Execution version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },
    #[error(transparent)]
    Runner(#[from] CodexRunnerError),
    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Copy, Debug)]
pub struct TransitionCodexExecutionInput {
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct RecordCodexExecutionResultInput {
    pub output_summary: String,
    pub validation_result: String,
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct CancelCodexExecutionInput {
    pub output_summary: String,
    pub expected_version: i64,
}

#[derive(Clone)]
pub struct CodexExecutionKernel {
    repository: Arc<dyn CodexExecutionKernelRepository>,
    runner: Arc<dyn CodexRunner>,
}

impl CodexExecutionKernel {
    pub fn new(
        repository: Arc<dyn CodexExecutionKernelRepository>,
        runner: Arc<dyn CodexRunner>,
    ) -> Self {
        Self { repository, runner }
    }

    pub async fn create_execution(
        &self,
        project_id: &str,
        proposal_id: &str,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        let project = self.require_project(project_id).await?;
        if project.details().status == ProjectStatus::Archived {
            return Err(CodexExecutionError::ProjectArchived);
        }
        let proposal = self.require_proposal(project_id, proposal_id).await?;
        if proposal.status() != CodexTaskProposalStatus::Approved {
            return Err(CodexExecutionError::ProposalNotApproved);
        }

        let execution = CodexExecutionRecord::create(
            Uuid::now_v7().to_string(),
            proposal.id().into(),
            project_id.into(),
            Utc::now(),
        )?;
        self.runner
            .accept(&runner_request(&execution, &proposal))
            .await?;
        self.repository
            .insert_codex_execution(&execution)
            .await
            .map_err(map_repository_error)?;
        Ok(execution)
    }

    pub async fn list_executions(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexExecutionRecord>, CodexExecutionError> {
        self.require_project(project_id).await?;
        self.repository
            .list_codex_executions(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn get_execution(
        &self,
        project_id: &str,
        execution_id: &str,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        self.require_project(project_id).await?;
        let execution = self
            .repository
            .get_codex_execution(execution_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexExecutionError::ExecutionNotFound)?;
        execution.ensure_project(project_id)?;
        Ok(execution)
    }

    pub async fn start_execution(
        &self,
        project_id: &str,
        execution_id: &str,
        input: TransitionCodexExecutionInput,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        self.transition(project_id, execution_id, input.expected_version, |value| {
            value.start(Utc::now())
        })
        .await
    }

    pub async fn complete_execution(
        &self,
        project_id: &str,
        execution_id: &str,
        input: RecordCodexExecutionResultInput,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        self.transition(project_id, execution_id, input.expected_version, |value| {
            value.succeed(input.output_summary, input.validation_result, Utc::now())
        })
        .await
    }

    pub async fn fail_execution(
        &self,
        project_id: &str,
        execution_id: &str,
        input: RecordCodexExecutionResultInput,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        self.transition(project_id, execution_id, input.expected_version, |value| {
            value.fail(input.output_summary, input.validation_result, Utc::now())
        })
        .await
    }

    pub async fn cancel_execution(
        &self,
        project_id: &str,
        execution_id: &str,
        input: CancelCodexExecutionInput,
    ) -> Result<CodexExecutionRecord, CodexExecutionError> {
        self.transition(project_id, execution_id, input.expected_version, |value| {
            value.cancel(input.output_summary, Utc::now())
        })
        .await
    }

    async fn transition<F>(
        &self,
        project_id: &str,
        execution_id: &str,
        expected_version: i64,
        action: F,
    ) -> Result<CodexExecutionRecord, CodexExecutionError>
    where
        F: FnOnce(&mut CodexExecutionRecord) -> Result<(), DomainError>,
    {
        let mut execution = self.get_execution(project_id, execution_id).await?;
        if execution.version() != expected_version {
            return Err(CodexExecutionError::VersionConflict {
                expected: expected_version,
                actual: execution.version(),
            });
        }
        let from_status = execution.status().as_str();
        action(&mut execution)?;
        match self
            .repository
            .transition_codex_execution(&execution, from_status, expected_version)
            .await
        {
            Ok(()) => Ok(execution),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.get_execution(project_id, execution_id).await?;
                Err(CodexExecutionError::VersionConflict {
                    expected: expected_version,
                    actual: current.version(),
                })
            }
            Err(error) => Err(map_repository_error(error)),
        }
    }

    async fn require_project(
        &self,
        project_id: &str,
    ) -> Result<crate::domain::Project, CodexExecutionError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexExecutionError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexExecutionError::ProjectNotFound)
    }

    async fn require_proposal(
        &self,
        project_id: &str,
        proposal_id: &str,
    ) -> Result<CodexTaskProposal, CodexExecutionError> {
        let proposal = self
            .repository
            .get_codex_task_proposal(proposal_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(CodexExecutionError::ProposalNotFound)?;
        proposal.ensure_project(project_id)?;
        Ok(proposal)
    }
}

fn runner_request(
    execution: &CodexExecutionRecord,
    proposal: &CodexTaskProposal,
) -> CodexExecutionRequest {
    let details = proposal.details();
    CodexExecutionRequest {
        execution_id: execution.id().into(),
        proposal_id: proposal.id().into(),
        project_id: proposal.project_id().into(),
        context_export_id: proposal.context_export_id().into(),
        title: details.title.clone(),
        goal: details.goal.clone(),
        allowed_scope: details.allowed_scope.clone(),
        forbidden_scope: details.forbidden_scope.clone(),
        acceptance_criteria: details.acceptance_criteria.clone(),
        validation_commands: details.validation_commands.clone(),
    }
}

fn map_repository_error(error: RepositoryError) -> CodexExecutionError {
    match error {
        RepositoryError::CorruptData(value) => CodexExecutionError::CorruptData(value),
        RepositoryError::Storage(value) => CodexExecutionError::Storage(value),
        other => CodexExecutionError::Storage(other.to_string()),
    }
}
