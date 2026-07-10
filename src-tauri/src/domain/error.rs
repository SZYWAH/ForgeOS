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
}
