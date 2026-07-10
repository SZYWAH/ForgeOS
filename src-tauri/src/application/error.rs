use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Validation(#[from] DomainError),

    #[error("ForgeOS already has a Workspace")]
    WorkspaceAlreadyExists,

    #[error("create a Workspace before creating or reading projects")]
    WorkspaceNotFound,

    #[error("project not found")]
    ProjectNotFound,

    #[error("another project already uses this slug in the current Workspace")]
    ProjectSlugConflict,

    #[error("project version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },

    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}
