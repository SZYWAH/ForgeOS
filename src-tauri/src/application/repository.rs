use std::sync::Arc;

use chrono::{SecondsFormat, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        validate_commit_sha, DomainError, Evidence, EvidenceDetails, EvidenceSourceType,
        EvidenceType, Repository, WorkingCopy,
    },
    ports::{
        GitAdapter, GitAdapterError, GitCommit, GitInspection, RepositoryError,
        RepositoryKernelStore,
    },
};

#[derive(Debug, Error)]
pub enum RepositoryKernelError {
    #[error(transparent)]
    Validation(#[from] DomainError),

    #[error("create a Workspace before registering Repositories")]
    WorkspaceNotFound,

    #[error("Project not found")]
    ProjectNotFound,

    #[error("Repository not found")]
    RepositoryNotFound,

    #[error("the local Git repository is already registered")]
    RepositoryAlreadyRegistered,

    #[error("Repository version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },

    #[error("Repository has no Working Copy observation")]
    WorkingCopyNotFound,

    #[error("Git inspection failed: {0}")]
    Git(String),

    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct RegisterRepositoryInput {
    pub path: String,
}

#[derive(Clone, Copy, Debug)]
pub struct RefreshRepositoryInput {
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct RepositoryRecord {
    pub repository: Repository,
    pub working_copy: WorkingCopy,
    pub recent_commits: Vec<GitCommit>,
}

#[derive(Clone)]
pub struct RepositoryKernel {
    store: Arc<dyn RepositoryKernelStore>,
    git: Arc<dyn GitAdapter>,
}

impl RepositoryKernel {
    pub fn new(store: Arc<dyn RepositoryKernelStore>, git: Arc<dyn GitAdapter>) -> Self {
        Self { store, git }
    }

    pub async fn inspect_repository(
        &self,
        path: &str,
    ) -> Result<GitInspection, RepositoryKernelError> {
        self.git.inspect(path.trim()).await.map_err(map_git_error)
    }

    pub async fn register_repository(
        &self,
        project_id: &str,
        input: RegisterRepositoryInput,
    ) -> Result<RepositoryRecord, RepositoryKernelError> {
        let project = self.require_project(project_id).await?;
        project.ensure_accepts_repository_changes()?;
        let inspection = self.inspect_repository(&input.path).await?;
        let repository = Repository::create(
            Uuid::now_v7().to_string(),
            project.id,
            inspection.root_path.clone(),
            inspection.remote_url.clone(),
            inspection.default_branch.clone(),
            inspection.checked_at,
        )?;
        let working_copy = working_copy(repository.id(), &inspection)?;
        match self
            .store
            .insert_repository_with_working_copy(&repository, &working_copy)
            .await
        {
            Ok(()) => Ok(record(repository, working_copy, inspection.recent_commits)),
            Err(RepositoryError::Conflict) => {
                Err(RepositoryKernelError::RepositoryAlreadyRegistered)
            }
            Err(RepositoryError::ProjectArchived) => {
                Err(DomainError::ArchivedProjectCannotAcceptRepositoryChanges.into())
            }
            Err(error) => Err(map_store_error(error)),
        }
    }

    pub async fn list_repositories(
        &self,
        project_id: &str,
    ) -> Result<Vec<RepositoryRecord>, RepositoryKernelError> {
        self.require_project(project_id).await?;
        let repositories = self
            .store
            .list_repositories(project_id)
            .await
            .map_err(map_store_error)?;
        let mut records = Vec::with_capacity(repositories.len());
        for repository in repositories {
            let working_copy = self
                .store
                .get_current_working_copy(repository.id())
                .await
                .map_err(map_store_error)?
                .ok_or(RepositoryKernelError::WorkingCopyNotFound)?;
            records.push(record(repository, working_copy, vec![]));
        }
        Ok(records)
    }

    pub async fn refresh_repository(
        &self,
        project_id: &str,
        repository_id: &str,
        input: RefreshRepositoryInput,
    ) -> Result<RepositoryRecord, RepositoryKernelError> {
        let project = self.require_project(project_id).await?;
        project.ensure_accepts_repository_changes()?;
        let mut repository = self.require_repository(project_id, repository_id).await?;
        ensure_version(input.expected_version, repository.version())?;
        let inspection = self
            .git
            .inspect(repository.path())
            .await
            .map_err(map_git_error)?;
        let expected_version = repository.version();
        repository.refresh(
            inspection.remote_url.clone(),
            inspection.default_branch.clone(),
            inspection.checked_at,
        )?;
        let working_copy = working_copy(repository.id(), &inspection)?;
        match self
            .store
            .update_repository_with_working_copy(&repository, expected_version, &working_copy)
            .await
        {
            Ok(()) => Ok(record(repository, working_copy, inspection.recent_commits)),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.require_repository(project_id, repository_id).await?;
                Err(RepositoryKernelError::VersionConflict {
                    expected: input.expected_version,
                    actual: current.version(),
                })
            }
            Err(RepositoryError::ProjectArchived) => {
                Err(DomainError::ArchivedProjectCannotAcceptRepositoryChanges.into())
            }
            Err(error) => Err(map_store_error(error)),
        }
    }

    pub async fn create_commit_evidence(
        &self,
        project_id: &str,
        repository_id: &str,
        commit_sha: &str,
    ) -> Result<Evidence, RepositoryKernelError> {
        let project = self.require_project(project_id).await?;
        project.ensure_accepts_evidence_changes()?;
        let repository = self.require_repository(project_id, repository_id).await?;
        let sha = validate_commit_sha(commit_sha)?;
        let commit = self
            .git
            .get_commit(repository.path(), &sha)
            .await
            .map_err(map_git_error)?;
        let short_sha: String = commit.sha.chars().take(12).collect();
        let title = truncate(&format!("Git commit {short_sha}: {}", commit.message), 200);
        let description = format!(
            "Commit:\n{}\n\nMessage:\n{}\n\nAuthor:\n{} <{}>\n\nDate:\n{}\n\nRepository:\n{}",
            commit.sha,
            commit.message,
            commit.author_name,
            commit.author_email,
            commit
                .authored_at
                .to_rfc3339_opts(SecondsFormat::Secs, true),
            repository.path(),
        );
        self.persist_git_evidence(
            &project.id,
            &repository,
            title,
            description,
            EvidenceType::GitCommit,
            Some(&commit.sha),
        )
        .await
    }

    pub async fn create_status_evidence(
        &self,
        project_id: &str,
        repository_id: &str,
    ) -> Result<Evidence, RepositoryKernelError> {
        let project = self.require_project(project_id).await?;
        project.ensure_accepts_evidence_changes()?;
        let repository = self.require_repository(project_id, repository_id).await?;
        let working_copy = self
            .store
            .get_current_working_copy(repository.id())
            .await
            .map_err(map_store_error)?
            .ok_or(RepositoryKernelError::WorkingCopyNotFound)?;
        let summary = working_copy.diff_summary();
        let description = format!(
            "Branch:\n{}\n\nHEAD:\n{}\n\nDirty:\n{}\n\nDiff Summary:\nChanged files: {}\nStaged files: {}\nUnstaged files: {}\nUntracked files: {}\nInsertions: {}\nDeletions: {}\n\nChecked At:\n{}\n\nRepository:\n{}",
            working_copy.branch(),
            working_copy.head_commit(),
            working_copy.is_dirty(),
            summary.changed_files,
            summary.staged_files,
            summary.unstaged_files,
            summary.untracked_files,
            summary.insertions,
            summary.deletions,
            working_copy.checked_at().to_rfc3339_opts(SecondsFormat::Secs, true),
            repository.path(),
        );
        self.persist_git_evidence(
            &project.id,
            &repository,
            format!(
                "Git status: {} @ {}",
                working_copy.branch(),
                &working_copy.head_commit()[..12]
            ),
            description,
            EvidenceType::GitStatus,
            None,
        )
        .await
    }

    pub async fn list_repository_evidence(
        &self,
        project_id: &str,
        repository_id: &str,
    ) -> Result<Vec<Evidence>, RepositoryKernelError> {
        self.require_project(project_id).await?;
        self.require_repository(project_id, repository_id).await?;
        self.store
            .list_repository_evidence(repository_id)
            .await
            .map_err(map_store_error)
    }

    async fn persist_git_evidence(
        &self,
        project_id: &str,
        repository: &Repository,
        title: String,
        description: String,
        evidence_type: EvidenceType,
        commit_sha: Option<&str>,
    ) -> Result<Evidence, RepositoryKernelError> {
        let evidence = Evidence::create(
            Uuid::now_v7().to_string(),
            project_id.to_owned(),
            EvidenceDetails {
                title,
                description,
                evidence_type,
                source_type: EvidenceSourceType::LocalFile,
                source_uri: repository.path().to_owned(),
            },
            Utc::now(),
        )?;
        match self
            .store
            .insert_git_evidence(&evidence, repository.id(), commit_sha)
            .await
        {
            Ok(()) => Ok(evidence),
            Err(RepositoryError::ProjectArchived) => {
                Err(DomainError::ArchivedProjectCannotAcceptEvidenceChanges.into())
            }
            Err(error) => Err(map_store_error(error)),
        }
    }

    async fn require_project(
        &self,
        project_id: &str,
    ) -> Result<crate::domain::Project, RepositoryKernelError> {
        let workspace = self
            .store
            .get_current_workspace()
            .await
            .map_err(map_store_error)?
            .ok_or(RepositoryKernelError::WorkspaceNotFound)?;
        self.store
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_store_error)?
            .ok_or(RepositoryKernelError::ProjectNotFound)
    }

    async fn require_repository(
        &self,
        project_id: &str,
        repository_id: &str,
    ) -> Result<Repository, RepositoryKernelError> {
        let repository = self
            .store
            .get_repository(repository_id)
            .await
            .map_err(map_store_error)?
            .ok_or(RepositoryKernelError::RepositoryNotFound)?;
        repository.ensure_project(project_id)?;
        Ok(repository)
    }
}

fn working_copy(
    repository_id: &str,
    inspection: &GitInspection,
) -> Result<WorkingCopy, DomainError> {
    WorkingCopy::create(
        Uuid::now_v7().to_string(),
        repository_id.to_owned(),
        inspection.branch.clone(),
        inspection.head_commit.clone(),
        inspection.is_dirty,
        inspection.diff_summary,
        inspection.checked_at,
    )
}

fn record(
    repository: Repository,
    working_copy: WorkingCopy,
    recent_commits: Vec<GitCommit>,
) -> RepositoryRecord {
    RepositoryRecord {
        repository,
        working_copy,
        recent_commits,
    }
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), RepositoryKernelError> {
    if expected != actual {
        return Err(RepositoryKernelError::VersionConflict { expected, actual });
    }
    Ok(())
}

fn map_git_error(error: GitAdapterError) -> RepositoryKernelError {
    RepositoryKernelError::Git(error.to_string())
}

fn map_store_error(error: RepositoryError) -> RepositoryKernelError {
    match error {
        RepositoryError::Conflict => RepositoryKernelError::RepositoryAlreadyRegistered,
        RepositoryError::CorruptData(message) => RepositoryKernelError::CorruptData(message),
        RepositoryError::Storage(message) => RepositoryKernelError::Storage(message),
        other => RepositoryKernelError::Storage(other.to_string()),
    }
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
