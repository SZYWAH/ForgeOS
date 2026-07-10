use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("{field} is required")]
    Required { field: &'static str },

    #[error("{field} must be at most {max} characters")]
    TooLong { field: &'static str, max: usize },

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
}
