use std::path::Path;

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_ID_LENGTH: usize = 128;
const MAX_PATH_LENGTH: usize = 32_768;
const MAX_REMOTE_URL_LENGTH: usize = 2_048;
const MAX_BRANCH_LENGTH: usize = 512;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DiffSummary {
    pub changed_files: i64,
    pub staged_files: i64,
    pub unstaged_files: i64,
    pub untracked_files: i64,
    pub insertions: i64,
    pub deletions: i64,
}

impl DiffSummary {
    pub fn validated(self) -> Result<Self, DomainError> {
        if [
            self.changed_files,
            self.staged_files,
            self.unstaged_files,
            self.untracked_files,
            self.insertions,
            self.deletions,
        ]
        .into_iter()
        .any(|value| value < 0)
        {
            return Err(DomainError::InvalidWorkingCopySummary);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    id: String,
    project_id: String,
    path: String,
    remote_url: Option<String>,
    default_branch: String,
    last_checked_at: DateTime<Utc>,
    version: i64,
}

impl Repository {
    pub fn create(
        id: String,
        project_id: String,
        path: String,
        remote_url: Option<String>,
        default_branch: String,
        checked_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            path,
            remote_url,
            default_branch,
            checked_at,
            1,
        )
    }

    pub fn restore(
        id: String,
        project_id: String,
        path: String,
        remote_url: Option<String>,
        default_branch: String,
        last_checked_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidRepositoryVersion);
        }
        let path = required("repository path", path, MAX_PATH_LENGTH)?;
        if !Path::new(&path).is_absolute() {
            return Err(DomainError::InvalidRepositoryPath);
        }
        Ok(Self {
            id: required("repository id", id, MAX_ID_LENGTH)?,
            project_id: required("project id", project_id, MAX_ID_LENGTH)?,
            path,
            remote_url: optional(remote_url, MAX_REMOTE_URL_LENGTH)?,
            default_branch: required("default branch", default_branch, MAX_BRANCH_LENGTH)?,
            last_checked_at,
            version,
        })
    }

    pub fn refresh(
        &mut self,
        remote_url: Option<String>,
        default_branch: String,
        checked_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.remote_url = optional(remote_url, MAX_REMOTE_URL_LENGTH)?;
        self.default_branch = required("default branch", default_branch, MAX_BRANCH_LENGTH)?;
        self.last_checked_at = checked_at.max(self.last_checked_at);
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::RepositoryVersionOverflow)?;
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::RepositoryProjectMismatch);
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn remote_url(&self) -> Option<&str> {
        self.remote_url.as_deref()
    }
    pub fn default_branch(&self) -> &str {
        &self.default_branch
    }
    pub fn last_checked_at(&self) -> DateTime<Utc> {
        self.last_checked_at
    }
    pub fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingCopy {
    id: String,
    repository_id: String,
    branch: String,
    head_commit: String,
    is_dirty: bool,
    diff_summary: DiffSummary,
    checked_at: DateTime<Utc>,
}

impl WorkingCopy {
    pub fn create(
        id: String,
        repository_id: String,
        branch: String,
        head_commit: String,
        is_dirty: bool,
        diff_summary: DiffSummary,
        checked_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            repository_id,
            branch,
            head_commit,
            is_dirty,
            diff_summary,
            checked_at,
        )
    }

    pub fn restore(
        id: String,
        repository_id: String,
        branch: String,
        head_commit: String,
        is_dirty: bool,
        diff_summary: DiffSummary,
        checked_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: required("Working Copy id", id, MAX_ID_LENGTH)?,
            repository_id: required("repository id", repository_id, MAX_ID_LENGTH)?,
            branch: required("Working Copy branch", branch, MAX_BRANCH_LENGTH)?,
            head_commit: validate_commit_sha(&head_commit)?,
            is_dirty,
            diff_summary: diff_summary.validated()?,
            checked_at,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }
    pub fn branch(&self) -> &str {
        &self.branch
    }
    pub fn head_commit(&self) -> &str {
        &self.head_commit
    }
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }
    pub fn diff_summary(&self) -> DiffSummary {
        self.diff_summary
    }
    pub fn checked_at(&self) -> DateTime<Utc> {
        self.checked_at
    }
}

pub fn validate_commit_sha(value: &str) -> Result<String, DomainError> {
    let value = value.trim();
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DomainError::InvalidGitCommitSha);
    }
    Ok(value.to_owned())
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

fn optional(value: Option<String>, max: usize) -> Result<Option<String>, DomainError> {
    value
        .map(|value| {
            let value = value.trim().to_owned();
            if value.chars().count() > max {
                return Err(DomainError::TooLong {
                    field: "repository remote URL",
                    max,
                });
            }
            Ok(value)
        })
        .transpose()
        .map(|value| value.filter(|value| !value.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_refresh_is_versioned_and_project_bound() {
        let now = Utc::now();
        let path = std::env::current_dir().unwrap().display().to_string();
        let mut repository = Repository::create(
            "repository-1".into(),
            "project-1".into(),
            path,
            None,
            "main".into(),
            now,
        )
        .unwrap();
        repository
            .refresh(
                Some("https://example.com/repo.git".into()),
                "main".into(),
                now,
            )
            .unwrap();
        assert_eq!(repository.version(), 2);
        assert_eq!(
            repository.ensure_project("project-2"),
            Err(DomainError::RepositoryProjectMismatch)
        );
    }

    #[test]
    fn working_copy_requires_machine_commit_identity() {
        assert_eq!(
            validate_commit_sha("HEAD"),
            Err(DomainError::InvalidGitCommitSha)
        );
        assert!(validate_commit_sha(&"a".repeat(40)).is_ok());
    }
}
