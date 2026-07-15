use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("unsupported application language mode: {0}")]
    InvalidLanguageMode(String),
    #[error("application preferences version must be at least 1")]
    InvalidAppPreferencesVersion,
    #[error("application preferences timestamps are not chronologically valid")]
    InvalidAppPreferencesTimestamps,
    #[error("application preferences version cannot be incremented")]
    AppPreferencesVersionOverflow,
    #[error("unsupported activity sync mode: {0}")]
    InvalidActivitySyncMode(String),
    #[error("unsupported activity sync trigger: {0}")]
    InvalidActivitySyncTrigger(String),
    #[error("unsupported activity sync status: {0}")]
    InvalidActivitySyncStatus(String),
    #[error("unsupported activity sync stage: {0}")]
    InvalidActivitySyncStage(String),
    #[error("unsupported Project Pulse status: {0}")]
    InvalidProjectPulseStatus(String),
    #[error("unsupported Project Pulse analysis mode: {0}")]
    InvalidProjectPulseAnalysisMode(String),
    #[error("unsupported Project Pulse activity type: {0}")]
    InvalidProjectPulseActivityType(String),
    #[error("unsupported Project Pulse analysis revision: {0}")]
    InvalidProjectPulseAnalysisRevision(String),
    #[error("Project Pulse source window is not chronologically valid")]
    InvalidProjectPulseSourceWindow,
    #[error("Project Pulse observation counts are invalid")]
    InvalidProjectPulseObservationCounts,
    #[error("Project Pulse quality warnings must be resolved before confirmation")]
    ProjectPulseQualityRequiresReview,
    #[error("Project Pulse cannot be used through a different Project")]
    ProjectPulseProjectMismatch,
    #[error("only a pending Project Pulse can be edited, confirmed, or dismissed")]
    ProjectPulseNotPending,
    #[error("Project Pulse version cannot be incremented")]
    ProjectPulseVersionOverflow,
    #[error("Project Pulse content hash must be a lowercase SHA-256 value")]
    InvalidProjectPulseHash,
    #[error("Project Pulse timestamps are not chronologically valid")]
    InvalidProjectPulseTimestamps,
    #[error("Project Activity settings require an AI Provider when AI is enabled")]
    ProjectActivityAiProviderRequired,
    #[error("Project Activity version must be at least 1")]
    InvalidProjectActivityVersion,
    #[error("Codex Activity observation hash must be a lowercase SHA-256 value")]
    InvalidCodexObservationHash,
    #[error("Codex Activity observation belongs to another Project or Repository")]
    CodexObservationProjectMismatch,
    #[error("Project Pulse confirmation requires non-empty outcome, validation, and handoff")]
    ProjectPulseCompletionRequired,
    #[error("invalid Codex Execution status: {0}")]
    InvalidCodexExecutionStatus(String),
    #[error("invalid Codex Execution version")]
    InvalidCodexExecutionVersion,
    #[error("invalid Codex Execution timestamps")]
    InvalidCodexExecutionTimestamps,
    #[error("stored Codex Execution fields do not match its lifecycle status")]
    InvalidCodexExecutionLifecycle,
    #[error("invalid Codex Execution transition from {from} to {to}")]
    InvalidCodexExecutionTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("Codex Execution result summary and validation result are required")]
    CodexExecutionResultRequired,
    #[error("Codex Execution version overflow")]
    CodexExecutionVersionOverflow,
    #[error("Codex Execution belongs to another Project")]
    CodexExecutionProjectMismatch,
    #[error("unsupported GitHub connector auth type: {0}")]
    InvalidGitHubAuthType(String),
    #[error("invalid GitHub connector state")]
    InvalidGitHubConnector,
    #[error("GitHub repository URL must identify exactly one github.com owner/repository")]
    InvalidGitHubRepositoryUrl,
    #[error("unsupported GitHub import status: {0}")]
    InvalidGitHubImportStatus(String),
    #[error("invalid GitHub import draft")]
    InvalidGitHubImportDraft,
    #[error("invalid GitHub import transition from {from} to {to}")]
    InvalidGitHubImportTransition { from: String, to: String },
    #[error("invalid Codex Task Proposal status: {0}")]
    InvalidCodexTaskProposalStatus(String),
    #[error("invalid Codex Task Proposal version")]
    InvalidCodexTaskProposalVersion,
    #[error("invalid Codex Task Proposal timestamps")]
    InvalidCodexTaskProposalTimestamps,
    #[error("invalid Codex Task Proposal transition from {from} to {to}")]
    InvalidCodexTaskProposalTransition {
        from: &'static str,
        to: &'static str,
    },
    #[error("Codex Task Proposal version overflow")]
    CodexTaskProposalVersionOverflow,
    #[error("Codex Task Proposal belongs to another Project")]
    CodexTaskProposalProjectMismatch,
    #[error("{field} is required")]
    Required { field: &'static str },

    #[error("{field} must be at most {max} characters")]
    TooLong { field: &'static str, max: usize },

    #[error("{field} must contain at most {max} items")]
    TooManyItems { field: &'static str, max: usize },

    #[error(
        "slug must be 2-64 characters using lowercase ASCII letters, digits, and single hyphens"
    )]
    InvalidSlug,

    #[error("priority must be between 0 and 100")]
    InvalidPriority,

    #[error("unsupported project status: {0}")]
    InvalidProjectStatus(String),

    #[error("unsupported lifecycle stage: {0}")]
    InvalidLifecycleStage(String),

    #[error("project version must be at least 1")]
    InvalidVersion,

    #[error("updated_at cannot be earlier than created_at")]
    InvalidTimestamps,

    #[error("archiving requires the dedicated archive operation")]
    ArchiveRequiresDedicatedOperation,

    #[error("an archived project cannot be edited")]
    ArchivedProjectCannotBeEdited,

    #[error("an archived project cannot accept Snapshot changes")]
    ArchivedProjectCannotAcceptSnapshots,

    #[error("Snapshot version must be at least 1")]
    InvalidSnapshotVersion,

    #[error("Snapshot timestamps are not chronologically valid")]
    InvalidSnapshotTimestamps,

    #[error("a Snapshot cannot be versioned for a different Project")]
    SnapshotProjectMismatch,

    #[error("Snapshot version cannot be incremented")]
    SnapshotVersionOverflow,

    #[error("next steps must contain at most {max} items")]
    TooManyNextSteps { max: usize },

    #[error("unsupported Development Session status: {0}")]
    InvalidSessionStatus(String),

    #[error("Development Session version must be at least 1")]
    InvalidSessionVersion,

    #[error("Development Session timestamps are not chronologically valid")]
    InvalidSessionTimestamps,

    #[error("stored Development Session fields do not match its lifecycle status")]
    InvalidSessionLifecycle,

    #[error("a Development Session cannot be used through a different Project")]
    SessionProjectMismatch,

    #[error("a completed or abandoned Development Session cannot be modified")]
    TerminalSessionCannotBeModified,

    #[error("Development Session version cannot be incremented")]
    SessionVersionOverflow,

    #[error("an archived Project cannot start or edit Development Sessions")]
    ArchivedProjectCannotAcceptSessionChanges,

    #[error("unsupported Evidence type: {0}")]
    InvalidEvidenceType(String),

    #[error("unsupported Evidence source type: {0}")]
    InvalidEvidenceSourceType(String),

    #[error("unsupported Evidence verification status: {0}")]
    InvalidEvidenceVerificationStatus(String),

    #[error("Evidence source URI is invalid for its source type")]
    InvalidEvidenceSourceUri,

    #[error("Evidence version must be at least 1")]
    InvalidEvidenceVersion,

    #[error("Evidence timestamps are not chronologically valid")]
    InvalidEvidenceTimestamps,

    #[error("Evidence cannot be used through a different Project")]
    EvidenceProjectMismatch,

    #[error("Evidence version cannot be incremented")]
    EvidenceVersionOverflow,

    #[error("unsupported Evidence target type: {0}")]
    InvalidEvidenceTargetType(String),

    #[error("unsupported Evidence relation type: {0}")]
    InvalidEvidenceRelationType(String),

    #[error("an archived Project cannot create or modify Evidence")]
    ArchivedProjectCannotAcceptEvidenceChanges,

    #[error("AI Provider base URL must be an HTTP or HTTPS URL without whitespace")]
    InvalidAiProviderBaseUrl,

    #[error("AI Provider version must be at least 1")]
    InvalidAiProviderVersion,

    #[error("AI Provider timestamps are not chronologically valid")]
    InvalidAiProviderTimestamps,

    #[error("AI Provider version cannot be incremented")]
    AiProviderVersionOverflow,

    #[error("AI Provider is disabled")]
    AiProviderDisabled,

    #[error("unsupported AI message role: {0}")]
    InvalidAiMessageRole(String),

    #[error("unsupported Context Pack template type: {0}")]
    InvalidContextTemplateType(String),

    #[error("unsupported Context Export status: {0}")]
    InvalidContextExportStatus(String),

    #[error("Context Export version must be at least 1")]
    InvalidContextExportVersion,

    #[error("Context Export content hash does not match its Markdown")]
    InvalidContextExportHash,

    #[error("Context Export cannot be used through a different Project")]
    ContextExportProjectMismatch,

    #[error("Context Export version cannot be incremented")]
    ContextExportVersionOverflow,

    #[error("Context Pack sources must belong to the requested Project")]
    ContextSourceProjectMismatch,

    #[error("unsupported AI Candidate type: {0}")]
    InvalidAiCandidateType(String),

    #[error("unsupported AI Candidate status: {0}")]
    InvalidAiCandidateStatus(String),

    #[error("AI Candidate version must be at least 1")]
    InvalidAiCandidateVersion,

    #[error("AI Candidate timestamps are not chronologically valid")]
    InvalidAiCandidateTimestamps,

    #[error("AI Candidate cannot be used through a different Project")]
    AiCandidateProjectMismatch,

    #[error("AI Candidate source Context Export must belong to the same Project")]
    AiCandidateSourceProjectMismatch,

    #[error("only a pending AI Candidate can be reviewed or archived")]
    AiCandidateNotPending,

    #[error("AI Candidate version cannot be incremented")]
    AiCandidateVersionOverflow,

    #[error("Repository path must be an absolute local path")]
    InvalidRepositoryPath,

    #[error("Repository version must be at least 1")]
    InvalidRepositoryVersion,

    #[error("Repository version cannot be incremented")]
    RepositoryVersionOverflow,

    #[error("Repository cannot be used through a different Project")]
    RepositoryProjectMismatch,

    #[error("Repository timestamps are not chronologically valid")]
    InvalidRepositoryTimestamps,

    #[error("an archived Project cannot register or refresh Repositories")]
    ArchivedProjectCannotAcceptRepositoryChanges,

    #[error("Working Copy commit SHA must contain 40 or 64 lowercase hexadecimal characters")]
    InvalidGitCommitSha,

    #[error("Working Copy counters cannot be negative")]
    InvalidWorkingCopySummary,

    #[error("Git Evidence must be created through the read-only Repository integration")]
    GitEvidenceRequiresRepositoryIntegration,

    #[error("captured Git Evidence content is read-only")]
    GitEvidenceContentReadOnly,
}
