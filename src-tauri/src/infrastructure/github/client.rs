use std::collections::BTreeMap;

use async_trait::async_trait;
use reqwest::{header, Client, StatusCode};
use serde::Deserialize;

use crate::{
    domain::{GitHubCommitFact, GitHubReleaseFact, GitHubRepositoryFacts},
    ports::{
        GitHubAnalysisRepositoryMetadata, GitHubClient, GitHubClientError, GitHubHead,
        GitHubProjectSource, GitHubProjectSourceError, GitHubRateLimit, GitHubSupplementalSources,
        GitHubTextSource, GitHubTree, GitHubTreeEntry,
    },
};

const API_ROOT: &str = "https://api.github.com";

#[derive(Clone)]
pub struct GitHubRestClient {
    client: Client,
}

impl GitHubRestClient {
    pub fn new() -> Result<Self, GitHubClientError> {
        let client = Client::builder()
            .user_agent("ForgeOS/0.2 read-only GitHub connector")
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

    async fn analysis_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: String,
        token: Option<&str>,
    ) -> Result<(T, GitHubRateLimit), GitHubProjectSourceError> {
        let response = self
            .request(url, token)
            .send()
            .await
            .map_err(analysis_transport)?;
        let rate = analysis_rate_limit(&response);
        let status = response.status();
        if !status.is_success() {
            return Err(analysis_response_error(status, response).await);
        }
        let body = response
            .json()
            .await
            .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
        Ok((body, rate))
    }

    async fn optional_analysis_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: String,
        token: Option<&str>,
    ) -> Result<(Option<T>, GitHubRateLimit), GitHubProjectSourceError> {
        let response = self
            .request(url, token)
            .send()
            .await
            .map_err(analysis_transport)?;
        let rate = analysis_rate_limit(&response);
        if response.status() == StatusCode::NOT_FOUND {
            return Ok((None, rate));
        }
        let status = response.status();
        if !status.is_success() {
            return Err(analysis_response_error(status, response).await);
        }
        let body = response
            .json()
            .await
            .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
        Ok((Some(body), rate))
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

#[async_trait]
impl GitHubProjectSource for GitHubRestClient {
    async fn get_repository(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<GitHubAnalysisRepositoryMetadata, GitHubProjectSourceError> {
        let (repository, rate): (RepositoryResponse, _) = self
            .analysis_json(format!("{API_ROOT}/repos/{owner}/{name}"), token)
            .await?;
        Ok(GitHubAnalysisRepositoryMetadata {
            repository_id: repository.id.to_string(),
            owner: repository.owner.login,
            name: repository.name,
            url: repository.html_url,
            description: repository.description.unwrap_or_default(),
            default_branch: repository.default_branch,
            private: repository.private,
            language: repository.language,
            topics: repository.topics,
            license_spdx: repository.license.and_then(|value| value.spdx_id),
            rate_limit: rate,
        })
    }

    async fn get_default_head(
        &self,
        owner: &str,
        name: &str,
        default_branch: &str,
        token: Option<&str>,
    ) -> Result<GitHubHead, GitHubProjectSourceError> {
        let url = github_url(owner, name, &["commits", default_branch])?;
        let (commit, rate): (HeadCommitResponse, _) = self.analysis_json(url, token).await?;
        Ok(GitHubHead {
            sha: commit.sha,
            rate_limit: rate,
        })
    }

    async fn get_tree(
        &self,
        owner: &str,
        name: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubTree, GitHubProjectSourceError> {
        let mut url =
            reqwest::Url::parse(&github_url(owner, name, &["git", "trees", head_sha])?)
                .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
        url.query_pairs_mut().append_pair("recursive", "1");
        let (tree, rate): (TreeResponse, _) = self.analysis_json(url.into(), token).await?;
        Ok(GitHubTree {
            entries: tree
                .tree
                .into_iter()
                .map(|entry| GitHubTreeEntry {
                    path: entry.path,
                    kind: entry.kind,
                    sha: entry.sha,
                    size: entry.size,
                })
                .collect(),
            truncated: tree.truncated,
            rate_limit: rate,
        })
    }

    async fn get_text_source(
        &self,
        owner: &str,
        name: &str,
        path: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubTextSource, GitHubProjectSourceError> {
        let mut parts = vec!["contents"];
        parts.extend(path.split('/').filter(|part| !part.is_empty()));
        let mut url = reqwest::Url::parse(&github_url(owner, name, &parts)?)
            .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
        url.query_pairs_mut().append_pair("ref", head_sha);
        let response = self
            .request(url.into(), token)
            .header(header::ACCEPT, "application/vnd.github.raw+json")
            .send()
            .await
            .map_err(analysis_transport)?;
        let rate = analysis_rate_limit(&response);
        let status = response.status();
        if !status.is_success() {
            return Err(analysis_response_error(status, response).await);
        }
        if response
            .content_length()
            .is_some_and(|length| length > 128 * 1024)
        {
            return Err(GitHubProjectSourceError::SourceTooLarge);
        }
        let bytes = response.bytes().await.map_err(analysis_transport)?;
        if bytes.len() > 128 * 1024 {
            return Err(GitHubProjectSourceError::SourceTooLarge);
        }
        let content = String::from_utf8(bytes.to_vec()).map_err(|_| {
            GitHubProjectSourceError::InvalidResponse("source is not UTF-8 text".into())
        })?;
        Ok(GitHubTextSource {
            path: path.to_owned(),
            sha: String::new(),
            content,
            rate_limit: rate,
        })
    }

    async fn get_supplemental_sources(
        &self,
        owner: &str,
        name: &str,
        head_sha: &str,
        token: Option<&str>,
    ) -> Result<GitHubSupplementalSources, GitHubProjectSourceError> {
        let mut commits_url = reqwest::Url::parse(&github_url(owner, name, &["commits"])?)
            .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
        commits_url
            .query_pairs_mut()
            .append_pair("sha", head_sha)
            .append_pair("per_page", "10");
        let (commits, _): (Vec<CommitResponse>, _) =
            self.analysis_json(commits_url.into(), token).await?;
        let (release, _): (Option<ReleaseResponse>, _) = self
            .optional_analysis_json(
                format!("{API_ROOT}/repos/{owner}/{name}/releases/latest"),
                token,
            )
            .await?;
        let (sbom, rate): (Option<serde_json::Value>, _) = match self
            .optional_analysis_json(
                format!("{API_ROOT}/repos/{owner}/{name}/dependency-graph/sbom"),
                token,
            )
            .await
        {
            Ok(value) => value,
            // Metadata and HEAD already succeeded with the same credential. A
            // 403 here means the repository has no readable dependency graph,
            // not that the complete public analysis must fail.
            Err(GitHubProjectSourceError::Unauthorized) => (
                None,
                GitHubRateLimit {
                    remaining: None,
                    reset_at: None,
                },
            ),
            Err(error) => return Err(error),
        };
        Ok(GitHubSupplementalSources {
            commits: commits
                .into_iter()
                .map(|value| {
                    (
                        value.sha,
                        value
                            .commit
                            .message
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    )
                })
                .collect(),
            release: release.map(|value| (value.tag_name, value.name.unwrap_or_default())),
            sbom: sbom
                .and_then(|value| value.get("sbom").cloned())
                .and_then(|value| serde_json::to_string(&value).ok()),
            requests_used: 3,
            rate_limit: rate,
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
    #[serde(default)]
    private: bool,
    license: Option<LicenseResponse>,
}

#[derive(Deserialize)]
struct LicenseResponse {
    spdx_id: Option<String>,
}

#[derive(Deserialize)]
struct HeadCommitResponse {
    sha: String,
}

#[derive(Deserialize)]
struct TreeResponse {
    #[serde(default)]
    tree: Vec<TreeEntryResponse>,
    #[serde(default)]
    truncated: bool,
}

#[derive(Deserialize)]
struct TreeEntryResponse {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
    size: Option<u64>,
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

fn analysis_transport(error: reqwest::Error) -> GitHubProjectSourceError {
    GitHubProjectSourceError::Transport(error.to_string())
}

fn analysis_rate_limit(response: &reqwest::Response) -> GitHubRateLimit {
    GitHubRateLimit {
        remaining: response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok()),
        reset_at: response
            .headers()
            .get("x-ratelimit-reset")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    }
}

async fn analysis_response_error(
    status: StatusCode,
    response: reqwest::Response,
) -> GitHubProjectSourceError {
    let rate = analysis_rate_limit(&response);
    match status {
        StatusCode::NOT_FOUND => GitHubProjectSourceError::NotFound,
        StatusCode::UNAUTHORIZED => GitHubProjectSourceError::Unauthorized,
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS
            if rate.remaining == Some(0) || status == StatusCode::TOO_MANY_REQUESTS =>
        {
            GitHubProjectSourceError::RateLimited {
                remaining: rate.remaining,
                reset_at: rate.reset_at,
            }
        }
        StatusCode::FORBIDDEN => GitHubProjectSourceError::Unauthorized,
        _ => GitHubProjectSourceError::Http {
            status: status.as_u16(),
            message: sanitize_error(&response.text().await.unwrap_or_default()),
        },
    }
}

fn github_url(owner: &str, name: &str, tail: &[&str]) -> Result<String, GitHubProjectSourceError> {
    let mut url = reqwest::Url::parse(API_ROOT)
        .map_err(|error| GitHubProjectSourceError::InvalidResponse(error.to_string()))?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            GitHubProjectSourceError::InvalidResponse("invalid GitHub API base URL".into())
        })?;
        segments.push("repos").push(owner).push(name);
        for value in tail {
            segments.push(value);
        }
    }
    Ok(url.into())
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
