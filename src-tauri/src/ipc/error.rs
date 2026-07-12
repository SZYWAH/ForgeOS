use serde::Serialize;

use crate::{
    application::{
        AiCandidateError, AiProviderError, ApplicationError, CodexTaskProposalError,
        ContextPackError, RepositoryKernelError,
    },
    domain::DomainError,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl From<ApplicationError> for IpcError {
    fn from(error: ApplicationError) -> Self {
        let (code, retryable) = match &error {
            ApplicationError::Validation(domain_error) => (validation_code(domain_error), false),
            ApplicationError::WorkspaceAlreadyExists => ("workspace_already_exists", false),
            ApplicationError::WorkspaceNotFound => ("workspace_not_found", false),
            ApplicationError::ProjectNotFound => ("project_not_found", false),
            ApplicationError::ProjectSlugConflict => ("project_slug_conflict", false),
            ApplicationError::VersionConflict { .. } => ("version_conflict", false),
            ApplicationError::SnapshotAlreadyExists => ("snapshot_already_exists", false),
            ApplicationError::SnapshotNotFound => ("snapshot_not_found", false),
            ApplicationError::SnapshotVersionConflict { .. } => {
                ("snapshot_version_conflict", false)
            }
            ApplicationError::SessionAlreadyOpen => ("session_already_open", false),
            ApplicationError::SessionNotFound => ("session_not_found", false),
            ApplicationError::SessionVersionConflict { .. } => ("session_version_conflict", false),
            ApplicationError::EvidenceNotFound => ("evidence_not_found", false),
            ApplicationError::EvidenceVersionConflict { .. } => {
                ("evidence_version_conflict", false)
            }
            ApplicationError::EvidenceLinkAlreadyExists => ("evidence_link_already_exists", false),
            ApplicationError::EvidenceTargetNotFound => ("evidence_target_not_found", false),
            ApplicationError::EvidenceTargetProjectMismatch => {
                ("evidence_target_project_mismatch", false)
            }
            ApplicationError::CorruptData(_) => ("corrupt_data", false),
            ApplicationError::Storage(_) => ("storage_error", true),
        };

        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<sqlx::Error> for IpcError {
    fn from(error: sqlx::Error) -> Self {
        Self {
            code: "storage_error",
            message: format!("Unable to read the ForgeOS database: {error}"),
            retryable: true,
        }
    }
}

impl From<AiProviderError> for IpcError {
    fn from(error: AiProviderError) -> Self {
        let (code, retryable) = match &error {
            AiProviderError::Validation(DomainError::AiProviderDisabled) => {
                ("ai_provider_disabled", false)
            }
            AiProviderError::Validation(_) => ("validation_error", false),
            AiProviderError::NotFound => ("ai_provider_not_found", false),
            AiProviderError::NameConflict => ("ai_provider_name_conflict", false),
            AiProviderError::VersionConflict { .. } => ("ai_provider_version_conflict", false),
            AiProviderError::ApiKeyTooLong => ("api_key_too_long", false),
            AiProviderError::InvalidMessages => ("invalid_ai_messages", false),
            AiProviderError::SecretStore(_) => ("credential_store_error", true),
            AiProviderError::Invocation(_) => ("ai_provider_invocation_error", true),
            AiProviderError::CorruptData(_) => ("corrupt_data", false),
            AiProviderError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<ContextPackError> for IpcError {
    fn from(error: ContextPackError) -> Self {
        let error = match error {
            ContextPackError::AiProvider(provider_error) => return provider_error.into(),
            other => other,
        };
        let (code, retryable) = match &error {
            ContextPackError::Validation(domain_error) => (validation_code(domain_error), false),
            ContextPackError::WorkspaceNotFound => ("workspace_not_found", false),
            ContextPackError::ProjectNotFound => ("project_not_found", false),
            ContextPackError::ContextExportNotFound => ("context_export_not_found", false),
            ContextPackError::VersionConflict => ("context_export_version_conflict", false),
            ContextPackError::Render(_) => ("context_render_error", false),
            ContextPackError::AiProvider(_) => unreachable!("handled above"),
            ContextPackError::CorruptData(_) => ("corrupt_data", false),
            ContextPackError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<AiCandidateError> for IpcError {
    fn from(error: AiCandidateError) -> Self {
        let (code, retryable) = match &error {
            AiCandidateError::Validation(domain_error) => (validation_code(domain_error), false),
            AiCandidateError::WorkspaceNotFound => ("workspace_not_found", false),
            AiCandidateError::ProjectNotFound => ("project_not_found", false),
            AiCandidateError::ContextExportNotFound => ("context_export_not_found", false),
            AiCandidateError::CandidateNotFound => ("ai_candidate_not_found", false),
            AiCandidateError::VersionConflict { .. } => ("ai_candidate_version_conflict", false),
            AiCandidateError::CorruptData(_) => ("corrupt_data", false),
            AiCandidateError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<CodexTaskProposalError> for IpcError {
    fn from(error: CodexTaskProposalError) -> Self {
        let (code, retryable) = match &error {
            CodexTaskProposalError::Validation(value) => (validation_code(value), false),
            CodexTaskProposalError::WorkspaceNotFound => ("workspace_not_found", false),
            CodexTaskProposalError::ProjectNotFound => ("project_not_found", false),
            CodexTaskProposalError::ContextExportNotFound => ("context_export_not_found", false),
            CodexTaskProposalError::ProposalNotFound => ("codex_task_proposal_not_found", false),
            CodexTaskProposalError::VersionConflict { .. } => {
                ("codex_task_proposal_version_conflict", false)
            }
            CodexTaskProposalError::CorruptData(_) => ("corrupt_data", false),
            CodexTaskProposalError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<RepositoryKernelError> for IpcError {
    fn from(error: RepositoryKernelError) -> Self {
        let (code, retryable) = match &error {
            RepositoryKernelError::Validation(domain_error) => {
                (validation_code(domain_error), false)
            }
            RepositoryKernelError::WorkspaceNotFound => ("workspace_not_found", false),
            RepositoryKernelError::ProjectNotFound => ("project_not_found", false),
            RepositoryKernelError::RepositoryNotFound => ("repository_not_found", false),
            RepositoryKernelError::RepositoryAlreadyRegistered => {
                ("repository_already_registered", false)
            }
            RepositoryKernelError::VersionConflict { .. } => ("repository_version_conflict", false),
            RepositoryKernelError::WorkingCopyNotFound => ("working_copy_not_found", false),
            RepositoryKernelError::Git(_) => ("git_read_error", true),
            RepositoryKernelError::CorruptData(_) => ("corrupt_data", false),
            RepositoryKernelError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

fn validation_code(error: &DomainError) -> &'static str {
    match error {
        DomainError::ArchiveRequiresDedicatedOperation => "archive_requires_dedicated_operation",
        DomainError::ArchivedProjectCannotBeEdited => "archived_project_read_only",
        DomainError::ArchivedProjectCannotAcceptSnapshots => "archived_project_read_only",
        DomainError::ArchivedProjectCannotAcceptSessionChanges => "archived_project_read_only",
        DomainError::ArchivedProjectCannotAcceptEvidenceChanges => "archived_project_read_only",
        DomainError::EvidenceProjectMismatch => "evidence_project_mismatch",
        DomainError::ContextExportProjectMismatch => "context_export_project_mismatch",
        DomainError::ContextSourceProjectMismatch => "context_source_project_mismatch",
        DomainError::AiCandidateProjectMismatch => "ai_candidate_project_mismatch",
        DomainError::AiCandidateSourceProjectMismatch => "ai_candidate_source_project_mismatch",
        DomainError::AiCandidateNotPending => "ai_candidate_not_pending",
        DomainError::CodexTaskProposalProjectMismatch => "codex_task_proposal_project_mismatch",
        DomainError::InvalidCodexTaskProposalTransition { .. } => {
            "codex_task_proposal_invalid_transition"
        }
        DomainError::RepositoryProjectMismatch => "repository_project_mismatch",
        DomainError::ArchivedProjectCannotAcceptRepositoryChanges => "archived_project_read_only",
        DomainError::TerminalSessionCannotBeModified => "session_terminal_read_only",
        DomainError::SessionProjectMismatch => "session_project_mismatch",
        _ => "validation_error",
    }
}
