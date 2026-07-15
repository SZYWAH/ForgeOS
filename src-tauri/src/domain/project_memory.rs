use chrono::{DateTime, Utc};

use super::{validate_commit_sha, DomainError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryCommitObservation {
    pub id: String,
    pub repository_id: String,
    pub project_id: String,
    pub commit_sha: String,
    pub message: String,
    pub author_name: String,
    pub authored_at: DateTime<Utc>,
    pub observed_at: DateTime<Utc>,
    pub source_hash: String,
}

impl RepositoryCommitObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: String,
        repository_id: String,
        project_id: String,
        commit_sha: String,
        message: String,
        author_name: String,
        authored_at: DateTime<Utc>,
        observed_at: DateTime<Utc>,
        source_hash: String,
    ) -> Result<Self, DomainError> {
        let message = required("commit subject", message, 1_000)?;
        if source_hash.len() != 64
            || !source_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DomainError::InvalidRepositoryCommitHash);
        }
        Ok(Self {
            id: required("repository commit observation id", id, 128)?,
            repository_id: required("repository id", repository_id, 128)?,
            project_id: required("project id", project_id, 128)?,
            commit_sha: validate_commit_sha(&commit_sha)?,
            message,
            author_name: text(author_name, 500)?,
            authored_at,
            observed_at,
            source_hash,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryHistoryCursor {
    pub repository_id: String,
    pub project_id: String,
    pub initial_backfill_completed: bool,
    pub last_scanned_head_commit: Option<String>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub version: i64,
}

impl RepositoryHistoryCursor {
    pub fn restore(
        repository_id: String,
        project_id: String,
        initial_backfill_completed: bool,
        last_scanned_head_commit: Option<String>,
        last_success_at: Option<DateTime<Utc>>,
        version: i64,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidRepositoryHistoryCursor);
        }
        Ok(Self {
            repository_id: required("repository id", repository_id, 128)?,
            project_id: required("project id", project_id, 128)?,
            initial_backfill_completed,
            last_scanned_head_commit: last_scanned_head_commit
                .map(|value| validate_commit_sha(&value))
                .transpose()?,
            last_success_at,
            version,
        })
    }
}

fn required(field: &'static str, value: String, max: usize) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(DomainError::Required { field });
    }
    if value.chars().count() > max {
        return Err(DomainError::TooLong { field, max });
    }
    Ok(value)
}

fn text(value: String, max: usize) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.chars().count() > max {
        return Err(DomainError::TooLong {
            field: "commit author name",
            max,
        });
    }
    Ok(value)
}
