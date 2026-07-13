use std::collections::BTreeMap;

use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;

use crate::{
    domain::{GitHubCommitFact, GitHubReleaseFact, GitHubRepositoryFacts},
    ports::{GitHubClient, GitHubClientError},
};

const API_ROOT: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GitHubRestClient {
    client: Client,
}

impl GitHubRestClient {
    pub fn new() -> Result<Self, GitHubClientError> {
        let client = Client::builder()
            .user_agent("ForgeOS/0.1 read-only GitHub connector")
            .build()
            .map_err(|error| GitHubClientError::Transport(error.to_string()))?;
        Ok(Self { client })
    }

    fn request(&self, url: String, token: Option<&str>) -> reqwest::RequestBuilder {
        let request = self
            .client
            .get(url)
            .header(header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        match token.filter(|value| !value.trim().is_empty()) {
            Some(token) => request.bearer_auth(token),
            None => request,
        }
    }

    async fn json<T: for<'de> Deserialize<'de>>(
        &self,
        url: String,
        token: Option<&str>,
    ) -> Result<T, GitHubClientError> {
        let response = self.request(url, token).send().await.map_err(transport)?;
        let status = response.status();
        if !status.is_success() {
            return Err(response_error(status, response).await);
        }
        response
            .json()
            .await
            .map_err(|error| GitHubClientError::InvalidResponse(error.to_string()))
    }

    async fn readme(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<String, GitHubClientError> {
        let response = self
            .request(format!("{API_ROOT}/repos/{owner}/{name}/readme"), token)
            .header(header::ACCEPT, "application/vnd.github.raw+json")
            .send()
            .await
            .map_err(transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(String::new());
        }
        let status = response.status();
        if !status.is_success() {
            return Err(response_error(status, response).await);
        }
        let content = response.text().await.map_err(transport)?;
        Ok(summarize_readme(&content))
    }

    async fn latest_release(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<Option<GitHubReleaseFact>, GitHubClientError> {
        let response = self
            .request(
                format!("{API_ROOT}/repos/{owner}/{name}/releases/latest"),
                token,
            )
            .send()
            .await
            .map_err(transport)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = response.status();
        if !status.is_success() {
            return Err(response_error(status, response).await);
        }
        let release: ReleaseResponse = response
            .json()
            .await
            .map_err(|error| GitHubClientError::InvalidResponse(error.to_string()))?;
        Ok(Some(GitHubReleaseFact {
            id: release.id.to_string(),
            tag_name: release.tag_name,
            name: release.name.unwrap_or_default(),
            published_at: release.published_at,
            url: release.html_url,
        }))
    }
}

#[async_trait]
impl GitHubClient for GitHubRestClient {
    async fn inspect_repository(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<GitHubRepositoryFacts, GitHubClientError> {
        let repository: RepositoryResponse = self
            .json(format!("{API_ROOT}/repos/{owner}/{name}"), token)
            .await?;
        let languages: BTreeMap<String, u64> = self
            .json(format!("{API_ROOT}/repos/{owner}/{name}/languages"), token)
            .await?;
        let commits: Vec<CommitResponse> = self
            .json(
                format!("{API_ROOT}/repos/{owner}/{name}/commits?per_page=5"),
                token,
            )
            .await?;
        let readme_summary = self.readme(owner, name, token).await?;
        let latest_release = self.latest_release(owner, name, token).await?;

        Ok(GitHubRepositoryFacts {
            repository_id: repository.id.to_string(),
            owner: repository.owner.login,
            name: repository.name,
            url: repository.html_url,
            description: repository.description.unwrap_or_default(),
            homepage: repository.homepage.filter(|value| !value.trim().is_empty()),
            default_branch: repository.default_branch,
            language: repository.language,
            languages: languages.into_iter().collect(),
            topics: repository.topics,
            readme_summary,
            recent_commits: commits
                .into_iter()
                .map(|item| {
                    let (author, authored_at) = item
                        .commit
                        .author
                        .map(|author| (author.name, author.date))
                        .unwrap_or_else(|| ("Unknown".into(), String::new()));
                    GitHubCommitFact {
                        sha: item.sha,
                        message: item
                            .commit
                            .message
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                        author,
                        authored_at,
                        url: item.html_url,
                    }
                })
                .collect(),
            latest_release,
        })
    }
}

#[derive(Deserialize)]
struct RepositoryResponse {
    id: u64,
    name: String,
    owner: OwnerResponse,
    html_url: String,
    description: Option<String>,
    homepage: Option<String>,
    default_branch: String,
    language: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
}

#[derive(Deserialize)]
struct OwnerResponse {
    login: String,
}

#[derive(Deserialize)]
struct CommitResponse {
    sha: String,
    html_url: String,
    commit: CommitDetails,
}

#[derive(Deserialize)]
struct CommitDetails {
    message: String,
    author: Option<CommitAuthor>,
}

#[derive(Deserialize)]
struct CommitAuthor {
    name: String,
    date: String,
}

#[derive(Deserialize)]
struct ReleaseResponse {
    id: u64,
    tag_name: String,
    name: Option<String>,
    published_at: Option<String>,
    html_url: String,
}

async fn response_error(status: StatusCode, response: reqwest::Response) -> GitHubClientError {
    match status {
        StatusCode::NOT_FOUND => GitHubClientError::NotFound,
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN if status == StatusCode::UNAUTHORIZED => {
            GitHubClientError::Unauthorized
        }
        StatusCode::FORBIDDEN => {
            let remaining = response
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|value| value.to_str().ok());
            if remaining == Some("0") {
                GitHubClientError::RateLimited
            } else {
                GitHubClientError::Unauthorized
            }
        }
        _ => {
            let message = response.text().await.unwrap_or_default();
            GitHubClientError::Http {
                status: status.as_u16(),
                message: sanitize_error(&message),
            }
        }
    }
}

fn transport(error: reqwest::Error) -> GitHubClientError {
    GitHubClientError::Transport(error.to_string())
}

fn sanitize_error(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(500).collect()
}

fn summarize_readme(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("![") && !line.starts_with("<img"))
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(4_000)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_summary_is_deterministic_and_bounded() {
        assert_eq!(
            summarize_readme("# Title\n\n![badge](x)\nDescription"),
            "# Title\nDescription"
        );
        assert_eq!(summarize_readme(&"x".repeat(5_000)).chars().count(), 4_000);
    }
}
