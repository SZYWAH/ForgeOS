use async_trait::async_trait;
use thiserror::Error;

use crate::domain::GitHubRepositoryFacts;

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
