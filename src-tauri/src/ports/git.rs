use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::domain::DiffSummary;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCommit {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitInspection {
    pub root_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub branch: String,
    pub head_commit: String,
    pub is_dirty: bool,
    pub diff_summary: DiffSummary,
    pub upstream_branch: Option<String>,
    pub ahead_count: Option<i64>,
    pub behind_count: Option<i64>,
    pub checked_at: DateTime<Utc>,
    pub recent_commits: Vec<GitCommit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCommitHistory {
    pub head_commit: String,
    pub commits: Vec<GitCommit>,
    pub full_rescan: bool,
}

#[derive(Debug, Error)]
pub enum GitAdapterError {
    #[error("Git executable is unavailable: {0}")]
    GitUnavailable(String),

    #[error("path is not a readable Git working tree")]
    NotRepository,

    #[error("Git repository has no readable HEAD commit")]
    HeadUnavailable,

    #[error("Git read-only command timed out")]
    Timeout,

    #[error("Git read-only command failed: {0}")]
    CommandFailed(String),

    #[error("Git returned unsupported machine output: {0}")]
    InvalidOutput(String),
}

#[async_trait]
pub trait GitAdapter: Send + Sync {
    async fn inspect(&self, path: &str) -> Result<GitInspection, GitAdapterError>;
    async fn get_commit(&self, path: &str, sha: &str) -> Result<GitCommit, GitAdapterError>;
    async fn list_commit_history(
        &self,
        path: &str,
        previous_head: Option<&str>,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<GitCommitHistory, GitAdapterError>;
}
