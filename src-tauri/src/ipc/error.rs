use serde::Serialize;

use crate::{
    application::{
        AchievementError, AiCandidateError, AiProviderError, AppPreferencesError, ApplicationError,
        CodexExecutionError, CodexMcpError, CodexTaskProposalError, ContextPackError,
        GitHubAnalysisError, GitHubImportError, ProjectMemoryError, ProjectPulseError,
        RepositoryKernelError,
    },
    domain::DomainError,
};

impl From<CodexMcpError> for IpcError {
    fn from(error: CodexMcpError) -> Self {
        let (code, retryable) = match &error {
            CodexMcpError::CodexUnavailable => ("codex_mcp_codex_unavailable", true),
            CodexMcpError::SidecarMissing => ("codex_mcp_sidecar_missing", false),
            CodexMcpError::Conflict => ("codex_mcp_conflict", false),
            CodexMcpError::NotOwned => ("codex_mcp_not_owned", false),
            CodexMcpError::VerificationFailed => ("codex_mcp_verification_failed", true),
            CodexMcpError::Client(_) => ("codex_mcp_cli_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<GitHubAnalysisError> for IpcError {
    fn from(error: GitHubAnalysisError) -> Self {
        let (code, retryable) = match &error {
            GitHubAnalysisError::WorkspaceNotFound => ("workspace_not_found", false),
            GitHubAnalysisError::RunNotFound => ("github_analysis_not_found", false),
            GitHubAnalysisError::PublicOnly => ("github_analysis_public_only", false),
            GitHubAnalysisError::AlreadyRunning => ("github_analysis_already_running", true),
            GitHubAnalysisError::InvalidPrompt => ("github_analysis_prompt_rejected", false),
            GitHubAnalysisError::VersionConflict => ("github_analysis_version_conflict", false),
            GitHubAnalysisError::Source(crate::ports::GitHubProjectSourceError::NotFound) => {
                ("github_analysis_not_found", false)
            }
            GitHubAnalysisError::Source(crate::ports::GitHubProjectSourceError::RateLimited {
                ..
            }) => ("github_analysis_rate_limited", true),
            GitHubAnalysisError::Source(_) => ("github_analysis_source_unavailable", true),
            GitHubAnalysisError::SecretStore(_) => ("credential_store_error", true),
            GitHubAnalysisError::CorruptData(_) | GitHubAnalysisError::Validation(_) => {
                ("github_analysis_blocked", false)
            }
            GitHubAnalysisError::Storage(_) => ("github_analysis_storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl From<AppPreferencesError> for IpcError {
    fn from(error: AppPreferencesError) -> Self {
        let (code, retryable) = match &error {
            AppPreferencesError::Validation(_) => ("validation_error", false),
            AppPreferencesError::NotFound => ("app_preferences_not_found", true),
            AppPreferencesError::VersionConflict => ("app_preferences_version_conflict", false),
            AppPreferencesError::CorruptData(_) => ("corrupt_data", false),
            AppPreferencesError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
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

impl From<CodexExecutionError> for IpcError {
    fn from(error: CodexExecutionError) -> Self {
        let (code, retryable) = match &error {
            CodexExecutionError::Validation(value) => (validation_code(value), false),
            CodexExecutionError::WorkspaceNotFound => ("workspace_not_found", false),
            CodexExecutionError::ProjectNotFound => ("project_not_found", false),
            CodexExecutionError::ProjectArchived => ("archived_project_read_only", false),
            CodexExecutionError::ProposalNotFound => ("codex_task_proposal_not_found", false),
            CodexExecutionError::ProposalNotApproved => {
                ("codex_execution_proposal_not_approved", false)
            }
            CodexExecutionError::ExecutionNotFound => ("codex_execution_not_found", false),
            CodexExecutionError::VersionConflict { .. } => {
                ("codex_execution_version_conflict", false)
            }
            CodexExecutionError::Runner(_) => ("codex_runner_error", true),
            CodexExecutionError::CorruptData(_) => ("corrupt_data", false),
            CodexExecutionError::Storage(_) => ("storage_error", true),
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

impl From<GitHubImportError> for IpcError {
    fn from(error: GitHubImportError) -> Self {
        let (code, retryable) = match &error {
            GitHubImportError::Validation(value) => (validation_code(value), false),
            GitHubImportError::WorkspaceNotFound => ("workspace_not_found", false),
            GitHubImportError::DraftNotFound => ("github_import_draft_not_found", false),
            GitHubImportError::ProjectNotFound => ("project_not_found", false),
            GitHubImportError::DraftConflict => ("github_import_draft_conflict", false),
            GitHubImportError::ProjectSlugConflict => ("project_slug_conflict", false),
            GitHubImportError::VersionConflict { .. } => ("github_import_version_conflict", false),
            GitHubImportError::InvalidToken => ("github_token_invalid", false),
            GitHubImportError::SecretStore(_) => ("credential_store_error", true),
            GitHubImportError::GitHub(_) => ("github_api_error", true),
            GitHubImportError::CorruptData(_) => ("corrupt_data", false),
            GitHubImportError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<ProjectPulseError> for IpcError {
    fn from(error: ProjectPulseError) -> Self {
        let (code, retryable) = match &error {
            ProjectPulseError::Validation(domain) => (validation_code(domain), false),
            ProjectPulseError::WorkspaceNotFound => ("workspace_not_found", false),
            ProjectPulseError::ProjectNotFound => ("project_not_found", false),
            ProjectPulseError::PulseNotFound => ("project_pulse_not_found", false),
            ProjectPulseError::RepositoryNotConnected => ("repository_not_connected", false),
            ProjectPulseError::SyncRunNotFound => ("activity_sync_run_not_found", false),
            ProjectPulseError::VersionConflict => ("project_pulse_version_conflict", false),
            ProjectPulseError::BasisVersionConflict => ("project_pulse_basis_conflict", false),
            ProjectPulseError::CodexUnavailable(_) => ("codex_unavailable", true),
            ProjectPulseError::CorruptData(_) => ("corrupt_data", false),
            ProjectPulseError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<ProjectMemoryError> for IpcError {
    fn from(error: ProjectMemoryError) -> Self {
        let (code, retryable) = match &error {
            ProjectMemoryError::WorkspaceNotFound => ("workspace_not_found", false),
            ProjectMemoryError::ProjectNotFound => ("project_not_found", false),
            ProjectMemoryError::InvalidCursor => ("invalid_history_cursor", false),
            ProjectMemoryError::InvalidProjectLocator => ("invalid_project_locator", false),
            ProjectMemoryError::RepositoryPathNotConnected => ("repository_not_connected", false),
            ProjectMemoryError::CorruptData(_) => ("corrupt_data", false),
            ProjectMemoryError::Storage(_) => ("storage_error", true),
        };
        Self {
            code,
            message: error.to_string(),
            retryable,
        }
    }
}

impl From<AchievementError> for IpcError {
    fn from(error: AchievementError) -> Self {
        let (code, retryable) = match &error {
            AchievementError::Validation(value) => (validation_code(value), false),
            AchievementError::WorkspaceNotFound => ("workspace_not_found", false),
            AchievementError::ProjectNotFound => ("project_not_found", false),
            AchievementError::AchievementNotFound => ("achievement_not_found", false),
            AchievementError::HistoryItemNotFound => ("history_item_not_found", false),
            AchievementError::ProjectArchived => ("archived_project_read_only", false),
            AchievementError::VersionConflict => ("achievement_version_conflict", false),
            AchievementError::SourceProjectMismatch => {
                ("achievement_source_project_mismatch", false)
            }
            AchievementError::CorruptData(_) => ("corrupt_data", false),
            AchievementError::Storage(_) => ("storage_error", true),
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
        DomainError::AchievementConfirmationRequired => "achievement_confirmation_required",
        DomainError::AchievementNotDraft => "achievement_not_draft",
        DomainError::AchievementArchived => "achievement_archived",
        DomainError::AchievementProjectMismatch => "achievement_project_mismatch",
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
        DomainError::CodexExecutionProjectMismatch => "codex_execution_project_mismatch",
        DomainError::InvalidCodexExecutionTransition { .. } => "codex_execution_invalid_transition",
        DomainError::CodexExecutionResultRequired => "codex_execution_result_required",
        DomainError::RepositoryProjectMismatch => "repository_project_mismatch",
        DomainError::ProjectPulseProjectMismatch => "project_pulse_project_mismatch",
        DomainError::ProjectPulseNotPending => "project_pulse_not_pending",
        DomainError::ProjectPulseCompletionRequired => "project_pulse_completion_required",
        DomainError::ProjectPulseQualityRequiresReview => "project_pulse_quality_requires_review",
        DomainError::InvalidGitHubRepositoryUrl => "invalid_github_repository_url",
        DomainError::InvalidGitHubImportTransition { .. } => "invalid_github_import_transition",
        DomainError::ArchivedProjectCannotAcceptRepositoryChanges => "archived_project_read_only",
        DomainError::TerminalSessionCannotBeModified => "session_terminal_read_only",
        DomainError::SessionProjectMismatch => "session_project_mismatch",
        _ => "validation_error",
    }
}
