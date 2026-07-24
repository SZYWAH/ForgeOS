use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DomainError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitHubAuthType {
    None,
    TokenReference,
}

impl GitHubAuthType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::TokenReference => "token_reference",
        }
    }
}

impl FromStr for GitHubAuthType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(Self::None),
            "token_reference" => Ok(Self::TokenReference),
            _ => Err(DomainError::InvalidGitHubAuthType(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubConnector {
    id: String,
    auth_type: GitHubAuthType,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl GitHubConnector {
    pub fn create(id: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        Self::restore(id, GitHubAuthType::None, now, now, 1)
    }

    pub fn restore(
        id: String,
        auth_type: GitHubAuthType,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        if id.trim().is_empty() || version < 1 || updated_at < created_at {
            return Err(DomainError::InvalidGitHubConnector);
        }
        Ok(Self {
            id,
            auth_type,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn set_auth_type(&mut self, auth_type: GitHubAuthType, now: DateTime<Utc>) {
        if self.auth_type != auth_type {
            self.auth_type = auth_type;
            self.updated_at = now.max(self.updated_at);
            self.version += 1;
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn auth_type(&self) -> GitHubAuthType {
        self.auth_type
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    pub fn version(&self) -> i64 {
        self.version
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitHubImportStatus {
    Draft,
    Reviewed,
    Imported,
    Rejected,
}

impl GitHubImportStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Reviewed => "reviewed",
            Self::Imported => "imported",
            Self::Rejected => "rejected",
        }
    }
}

impl fmt::Display for GitHubImportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GitHubImportStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "reviewed" => Ok(Self::Reviewed),
            "imported" => Ok(Self::Imported),
            "rejected" => Ok(Self::Rejected),
            _ => Err(DomainError::InvalidGitHubImportStatus(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubCommitFact {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub authored_at: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubReleaseFact {
    pub id: String,
    pub tag_name: String,
    pub name: String,
    pub published_at: Option<String>,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubRepositoryFacts {
    pub repository_id: String,
    pub owner: String,
    pub name: String,
    pub url: String,
    pub description: String,
    pub homepage: Option<String>,
    pub default_branch: String,
    pub language: Option<String>,
    pub languages: Vec<(String, u64)>,
    pub topics: Vec<String>,
    pub readme_summary: String,
    pub recent_commits: Vec<GitHubCommitFact>,
    pub latest_release: Option<GitHubReleaseFact>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubImportDraft {
    id: String,
    connector_id: String,
    facts: GitHubRepositoryFacts,
    status: GitHubImportStatus,
    imported_project_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl GitHubImportDraft {
    pub fn create(
        id: String,
        connector_id: String,
        facts: GitHubRepositoryFacts,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            connector_id,
            facts,
            GitHubImportStatus::Draft,
            None,
            now,
            now,
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        connector_id: String,
        mut facts: GitHubRepositoryFacts,
        status: GitHubImportStatus,
        imported_project_id: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        if id.trim().is_empty()
            || connector_id.trim().is_empty()
            || version < 1
            || updated_at < created_at
        {
            return Err(DomainError::InvalidGitHubImportDraft);
        }
        facts.owner = required(facts.owner)?;
        facts.name = required(facts.name)?;
        facts.repository_id = required(facts.repository_id)?;
        facts.default_branch = required(facts.default_branch)?;
        if !facts.url.starts_with("https://github.com/") {
            return Err(DomainError::InvalidGitHubRepositoryUrl);
        }
        if status == GitHubImportStatus::Imported && imported_project_id.is_none() {
            return Err(DomainError::InvalidGitHubImportDraft);
        }
        if status != GitHubImportStatus::Imported && imported_project_id.is_some() {
            return Err(DomainError::InvalidGitHubImportDraft);
        }
        facts.topics.sort();
        facts.topics.dedup();
        facts.languages.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(Self {
            id,
            connector_id,
            facts,
            status,
            imported_project_id,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn review(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(GitHubImportStatus::Draft, GitHubImportStatus::Reviewed, now)
    }

    pub fn reject(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(
            self.status,
            GitHubImportStatus::Draft | GitHubImportStatus::Reviewed
        ) {
            return Err(DomainError::InvalidGitHubImportTransition {
                from: self.status.to_string(),
                to: "rejected".into(),
            });
        }
        self.status = GitHubImportStatus::Rejected;
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        Ok(())
    }

    pub fn return_to_draft(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(
            self.status,
            GitHubImportStatus::Reviewed | GitHubImportStatus::Rejected
        ) {
            return Err(DomainError::InvalidGitHubImportTransition {
                from: self.status.to_string(),
                to: "draft".into(),
            });
        }
        self.status = GitHubImportStatus::Draft;
        self.imported_project_id = None;
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        Ok(())
    }

    pub fn mark_imported(
        &mut self,
        project_id: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != GitHubImportStatus::Reviewed || project_id.trim().is_empty() {
            return Err(DomainError::InvalidGitHubImportTransition {
                from: self.status.to_string(),
                to: "imported".into(),
            });
        }
        self.status = GitHubImportStatus::Imported;
        self.imported_project_id = Some(project_id);
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        Ok(())
    }

    fn transition(
        &mut self,
        from: GitHubImportStatus,
        to: GitHubImportStatus,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != from {
            return Err(DomainError::InvalidGitHubImportTransition {
                from: self.status.to_string(),
                to: to.to_string(),
            });
        }
        self.status = to;
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn connector_id(&self) -> &str {
        &self.connector_id
    }
    pub fn facts(&self) -> &GitHubRepositoryFacts {
        &self.facts
    }
    pub fn status(&self) -> GitHubImportStatus {
        self.status
    }
    pub fn imported_project_id(&self) -> Option<&str> {
        self.imported_project_id.as_deref()
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    pub fn version(&self) -> i64 {
        self.version
    }
}

pub fn parse_github_repository_url(value: &str) -> Result<(String, String), DomainError> {
    let trimmed = value.trim();
    let without_fragment = trimmed.split('#').next().unwrap_or_default();
    let without_query = without_fragment.split('?').next().unwrap_or_default();
    let path = without_query
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")
        .or_else(|| {
            without_query
                .trim_end_matches('/')
                .strip_prefix("http://github.com/")
        })
        .or_else(|| {
            without_query
                .trim_end_matches('/')
                .strip_prefix("github.com/")
        })
        .ok_or(DomainError::InvalidGitHubRepositoryUrl)?;
    let mut parts = path.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default().trim_end_matches(".git");
    if owner.is_empty() || name.is_empty() || !valid_segment(owner) || !valid_segment(name) {
        return Err(DomainError::InvalidGitHubRepositoryUrl);
    }
    let remainder = parts.collect::<Vec<_>>();
    if let Some(section) = remainder.first().copied() {
        let supported = match section {
            "issues" => true,
            "releases" => remainder.len() == 1 || remainder.get(1) == Some(&"tag"),
            "tree" | "blob" => remainder.len() >= 3,
            _ => false,
        };
        if !supported {
            return Err(DomainError::InvalidGitHubRepositoryUrl);
        }
    }
    Ok((owner.to_owned(), name.to_owned()))
}

fn required(value: String) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(DomainError::InvalidGitHubImportDraft)
    } else {
        Ok(value)
    }
}

fn valid_segment(value: &str) -> bool {
    value.len() <= 100
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_repository_urls() {
        assert_eq!(
            parse_github_repository_url("https://github.com/SZYWAH/ForgeOS.git").unwrap(),
            ("SZYWAH".into(), "ForgeOS".into())
        );
        for value in [
            "https://github.com/SZYWAH/ForgeOS/issues",
            "https://github.com/SZYWAH/ForgeOS/releases/tag/v0.2.0",
            "https://github.com/SZYWAH/ForgeOS/tree/master/src-tauri",
            "https://github.com/SZYWAH/ForgeOS/blob/master/README.md?plain=1#readme",
        ] {
            assert_eq!(
                parse_github_repository_url(value).unwrap(),
                ("SZYWAH".into(), "ForgeOS".into())
            );
        }
        assert!(parse_github_repository_url("https://example.com/a/b").is_err());
        assert!(parse_github_repository_url("https://github.com/SZYWAH").is_err());
        assert!(parse_github_repository_url("https://github.com/SZYWAH/ForgeOS/actions").is_err());
        assert!(parse_github_repository_url("https://github.com/SZYWAH/ForgeOS/tree").is_err());
    }
}
