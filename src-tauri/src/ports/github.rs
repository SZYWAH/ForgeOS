use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{GitHubAnalysisRun, GitHubAnalysisSource, GitHubRepositoryFacts, Workspace};

#[derive(Debug, Error)]
pub enum GitHubClientError {
    #[error("GitHub repository was not found or is not accessible")]
    NotFound,
    #[error("GitHub authentication failed")]
    Unauthorized,
    #[error("GitHub API rate limit was exceeded")]
    RateLimited,
    #[error("GitHub returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("GitHub request failed: {0}")]
    Transport(String),
    #[error("GitHub returned invalid data: {0}")]
    InvalidResponse(String),
}

#[async_trait]
pub trait GitHubClient: Send + Sync {
    async fn inspect_repository(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<GitHubRepositoryFacts, GitHubClientError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubRateLimit {
    pub remaining: Option<i64>,
    pub reset_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubAnalysisRepositoryMetadata {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
    pub url: String,
    pub description: String,
    pub default_branch: String,
    pub private: bool,
    pub language: Option<String>,
    pub topics: Vec<String>,
    pub license_spdx: Option<String>,
    pub rate_limit: GitHubRateLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubHead {
    pub sha: String,
    pub rate_limit: GitHubRateLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubTreeEntry {
    pub path: String,
    pub kind: String,
    pub sha: String,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubTree {
    pub entries: Vec<GitHubTreeEntry>,
    pub truncated: bool,
    pub rate_limit: GitHubRateLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubTextSource {
    pub path: String,
    pub sha: String,
    pub content: String,
    pub rate_limit: GitHubRateLimit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubSupplementalSources {
    pub commits: Vec<(String, String)>,
    pub release: Option<(String, String)>,
    pub sbom: Option<String>,
    pub requests_used: i64,
    pub rate_limit: GitHubRateLimit,
}

#[derive(Debug, Error)]
pub enum GitHubProjectSourceError {
    #[error("GitHub repository was not found")]
    NotFound,
    #[error("GitHub authentication failed")]
    Unauthorized,
    #[error("GitHub API rate limit was exceeded")]
    RateLimited {
        remaining: Option<i64>,
        reset_at: Option<String>,
    },
    #[error("GitHub returned HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("GitHub request failed: {0}")]
    Transport(String),
    #[error("GitHub returned invalid data: {0}")]
    InvalidResponse(String),
    #[error("GitHub source exceeds the safe read limit")]
    SourceTooLarge,
}

#[async_trait]
pub trait GitHubProjectSource: Send + Sync {
    async fn get_repository(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<GitHubAnalysisRepositoryMetadata, GitHubProjectSourceError>;
    async fn get_default_head(
        &self,
        owner: &str,
        name: &str,
        default_branch: &str,
        token: Option<&str>,
    ) -> Result<GitHubHead, GitHubProjectSourceError>;
    async fn get_tree(
        &self,
        owner: &str,
        name: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubTree, GitHubProjectSourceError>;
    async fn get_text_source(
        &self,
        owner: &str,
        name: &str,
        path: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubTextSource, GitHubProjectSourceError>;
    async fn get_supplemental_sources(
        &self,
        owner: &str,
        name: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubSupplementalSources, GitHubProjectSourceError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubAnalysisPage {
    pub items: Vec<GitHubAnalysisRun>,
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait GitHubAnalysisRepository: Send + Sync {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, super::RepositoryError>;
    async fn get_github_connector_id(&self) -> Result<Option<String>, super::RepositoryError>;
    async fn insert_analysis_run(
        &self,
        run: &GitHubAnalysisRun,
    ) -> Result<(), super::RepositoryError>;
    async fn get_analysis_run(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Option<GitHubAnalysisRun>, super::RepositoryError>;
    async fn get_analysis_run_by_hash(
        &self,
        workspace_id: &str,
        request_hash: &str,
    ) -> Result<Option<GitHubAnalysisRun>, super::RepositoryError>;
    async fn list_analysis_runs(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<GitHubAnalysisPage, super::RepositoryError>;
    async fn update_analysis_run(
        &self,
        run: &GitHubAnalysisRun,
        expected_version: i64,
    ) -> Result<(), super::RepositoryError>;
    async fn replace_analysis_sources(
        &self,
        run_id: &str,
        sources: &[GitHubAnalysisSource],
    ) -> Result<(), super::RepositoryError>;
    async fn list_analysis_sources(
        &self,
        run_id: &str,
    ) -> Result<Vec<GitHubAnalysisSource>, super::RepositoryError>;
    async fn recover_interrupted_analyses(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<u64, super::RepositoryError>;
    async fn has_newer_repository_head(
        &self,
        workspace_id: &str,
        repository_id: &str,
        head_sha: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<bool, super::RepositoryError>;
}
