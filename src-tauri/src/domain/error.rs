use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
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
