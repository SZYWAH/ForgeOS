use serde::Serialize;

use crate::{application::ApplicationError, domain::DomainError};

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

fn validation_code(error: &DomainError) -> &'static str {
    match error {
        DomainError::ArchiveRequiresDedicatedOperation => "archive_requires_dedicated_operation",
        DomainError::ArchivedProjectCannotBeEdited => "archived_project_read_only",
        DomainError::ArchivedProjectCannotAcceptSnapshots => "archived_project_read_only",
        _ => "validation_error",
    }
}
