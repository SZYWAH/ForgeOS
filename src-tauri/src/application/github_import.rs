use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        parse_github_repository_url, DomainError, Evidence, EvidenceDetails, EvidenceSourceType,
        EvidenceType, GitHubAuthType, GitHubConnector, GitHubImportDraft, GitHubRepositoryFacts,
        LifecycleStage, Project, ProjectDetails, ProjectStatus,
    },
    ports::{ApiKeyStore, GitHubClient, GitHubImportKernelRepository, RepositoryError},
};

const MAX_TOKEN_LENGTH: usize = 16_384;

#[derive(Debug, Error)]
pub enum GitHubImportError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before importing GitHub repositories")]
    WorkspaceNotFound,
    #[error("GitHub import draft not found")]
    DraftNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("this GitHub repository already has an import draft")]
    DraftConflict,
    #[error("the imported Project slug already exists")]
    ProjectSlugConflict,
    #[error("GitHub import version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },
    #[error("GitHub token must not be empty or exceed {MAX_TOKEN_LENGTH} characters")]
    InvalidToken,
    #[error("GitHub credential store is unavailable: {0}")]
    SecretStore(String),
    #[error("GitHub read failed: {0}")]
    GitHub(String),
    #[error("stored GitHub import data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct GitHubConnectorView {
    pub connector: GitHubConnector,
    pub token_configured: bool,
}

#[derive(Clone, Debug)]
pub struct GitHubImportResult {
    pub draft: GitHubImportDraft,
    pub project: Project,
    pub evidences: Vec<Evidence>,
}

#[derive(Clone)]
pub struct GitHubImportKernel {
    repository: Arc<dyn GitHubImportKernelRepository>,
    client: Arc<dyn GitHubClient>,
    token_store: Arc<dyn ApiKeyStore>,
}

impl GitHubImportKernel {
    pub fn new(
        repository: Arc<dyn GitHubImportKernelRepository>,
        client: Arc<dyn GitHubClient>,
        token_store: Arc<dyn ApiKeyStore>,
    ) -> Self {
        Self {
            repository,
            client,
            token_store,
        }
    }

    pub async fn get_connector(&self) -> Result<GitHubConnectorView, GitHubImportError> {
        let connector = match self
            .repository
            .get_github_connector()
            .await
            .map_err(map_store)?
        {
            Some(connector) => connector,
            None => {
                let connector = GitHubConnector::create(Uuid::now_v7().to_string(), Utc::now())?;
                match self.repository.insert_github_connector(&connector).await {
                    Ok(()) => connector,
                    Err(RepositoryError::Conflict) => self
                        .repository
                        .get_github_connector()
                        .await
                        .map_err(map_store)?
                        .ok_or_else(|| {
                            GitHubImportError::Storage(
                                "connector creation raced but no connector exists".into(),
                            )
                        })?,
                    Err(error) => return Err(map_store(error)),
                }
            }
        };
        self.connector_view(connector).await
    }

    pub async fn set_token(&self, token: String) -> Result<GitHubConnectorView, GitHubImportError> {
        let token = token.trim();
        if token.is_empty() || token.chars().count() > MAX_TOKEN_LENGTH {
            return Err(GitHubImportError::InvalidToken);
        }
        let mut connector = self.get_connector().await?.connector;
        self.token_store
            .set(connector.id(), token)
            .await
            .map_err(secret)?;
        let expected = connector.version();
        connector.set_auth_type(GitHubAuthType::TokenReference, Utc::now());
        if let Err(error) = self
            .repository
            .update_github_connector(&connector, expected)
            .await
        {
            let _ = self.token_store.clear(connector.id()).await;
            return Err(map_store(error));
        }
        self.connector_view(connector).await
    }

    pub async fn clear_token(&self) -> Result<GitHubConnectorView, GitHubImportError> {
        let mut connector = self.get_connector().await?.connector;
        self.token_store
            .clear(connector.id())
            .await
            .map_err(secret)?;
        let expected = connector.version();
        connector.set_auth_type(GitHubAuthType::None, Utc::now());
        if connector.version() != expected {
            self.repository
                .update_github_connector(&connector, expected)
                .await
                .map_err(map_store)?;
        }
        self.connector_view(connector).await
    }

    pub async fn preview_repository(
        &self,
        repository_url: &str,
    ) -> Result<GitHubImportDraft, GitHubImportError> {
        let (owner, name) = parse_github_repository_url(repository_url)?;
        let connector = self.get_connector().await?.connector;
        let token = self.token_store.get(connector.id()).await.map_err(secret)?;
        let facts = self
            .client
            .inspect_repository(&owner, &name, token.as_deref())
            .await
            .map_err(|error| GitHubImportError::GitHub(error.to_string()))?;
        let draft = GitHubImportDraft::create(
            Uuid::now_v7().to_string(),
            connector.id().to_owned(),
            facts,
            Utc::now(),
        )?;
        match self.repository.insert_github_import_draft(&draft).await {
            Ok(()) => Ok(draft),
            Err(RepositoryError::Conflict) => Err(GitHubImportError::DraftConflict),
            Err(error) => Err(map_store(error)),
        }
    }

    pub async fn list_drafts(&self) -> Result<Vec<GitHubImportDraft>, GitHubImportError> {
        self.repository
            .list_github_import_drafts()
            .await
            .map_err(map_store)
    }

    pub async fn review_draft(
        &self,
        draft_id: &str,
        expected_version: i64,
    ) -> Result<GitHubImportDraft, GitHubImportError> {
        let mut draft = self.require_draft(draft_id).await?;
        ensure_version(expected_version, draft.version())?;
        draft.review(Utc::now())?;
        self.persist_draft(&draft, expected_version).await?;
        Ok(draft)
    }

    pub async fn reject_draft(
        &self,
        draft_id: &str,
        expected_version: i64,
    ) -> Result<GitHubImportDraft, GitHubImportError> {
        let mut draft = self.require_draft(draft_id).await?;
        ensure_version(expected_version, draft.version())?;
        draft.reject(Utc::now())?;
        self.persist_draft(&draft, expected_version).await?;
        Ok(draft)
    }

    pub async fn import_draft(
        &self,
        draft_id: &str,
        expected_version: i64,
    ) -> Result<GitHubImportResult, GitHubImportError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_store)?
            .ok_or(GitHubImportError::WorkspaceNotFound)?;
        let mut draft = self.require_draft(draft_id).await?;
        ensure_version(expected_version, draft.version())?;
        let now = Utc::now();
        let project = project_from_draft(&workspace.id, draft.facts(), now)?;
        draft.mark_imported(project.id.clone(), now)?;
        let evidence_links = evidence_from_draft(&project.id, draft.facts(), now)?;
        self.repository
            .import_github_draft(&draft, expected_version, &project, &evidence_links)
            .await
            .map_err(|error| match error {
                RepositoryError::Conflict => GitHubImportError::ProjectSlugConflict,
                other => map_store(other),
            })?;
        Ok(GitHubImportResult {
            draft,
            project,
            evidences: evidence_links.into_iter().map(|item| item.0).collect(),
        })
    }

    pub async fn get_project_import(
        &self,
        project_id: &str,
    ) -> Result<Option<GitHubImportDraft>, GitHubImportError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_store)?
            .ok_or(GitHubImportError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_store)?
            .ok_or(GitHubImportError::ProjectNotFound)?;
        self.repository
            .get_github_import_for_project(project_id)
            .await
            .map_err(map_store)
    }

    async fn require_draft(&self, draft_id: &str) -> Result<GitHubImportDraft, GitHubImportError> {
        self.repository
            .get_github_import_draft(draft_id)
            .await
            .map_err(map_store)?
            .ok_or(GitHubImportError::DraftNotFound)
    }

    async fn persist_draft(
        &self,
        draft: &GitHubImportDraft,
        expected_version: i64,
    ) -> Result<(), GitHubImportError> {
        match self
            .repository
            .update_github_import_draft(draft, expected_version)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.require_draft(draft.id()).await?;
                Err(GitHubImportError::VersionConflict {
                    expected: expected_version,
                    actual: current.version(),
                })
            }
            Err(error) => Err(map_store(error)),
        }
    }

    async fn connector_view(
        &self,
        connector: GitHubConnector,
    ) -> Result<GitHubConnectorView, GitHubImportError> {
        let token_configured = self.token_store.has(connector.id()).await.map_err(secret)?;
        Ok(GitHubConnectorView {
            connector,
            token_configured,
        })
    }
}

fn project_from_draft(
    workspace_id: &str,
    facts: &GitHubRepositoryFacts,
    now: chrono::DateTime<Utc>,
) -> Result<Project, DomainError> {
    let tech_stack = facts
        .languages
        .iter()
        .map(|(name, _)| name.as_str())
        .chain(facts.topics.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(", ");
    Project::new(
        Uuid::now_v7().to_string(),
        workspace_id.to_owned(),
        ProjectDetails {
            slug: slugify(&facts.name),
            name: facts.name.clone(),
            description: facts.description.clone(),
            vision: String::new(),
            target_user: String::new(),
            scope: facts.readme_summary.clone(),
            non_goals: String::new(),
            tech_stack,
            status: ProjectStatus::Active,
            lifecycle_stage: LifecycleStage::Implementation,
            priority: 50,
        },
        now,
    )
}

fn evidence_from_draft(
    project_id: &str,
    facts: &GitHubRepositoryFacts,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<(Evidence, String, String)>, DomainError> {
    let languages = facts
        .languages
        .iter()
        .map(|(name, bytes)| format!("{name}: {bytes} bytes"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut items = vec![(
        Evidence::create(Uuid::now_v7().to_string(), project_id.to_owned(), EvidenceDetails {
            title: format!("GitHub repository {}/{}", facts.owner, facts.name),
            description: format!("Repository:\n{}/{}\n\nDescription:\n{}\n\nDefault Branch:\n{}\n\nLanguages:\n{}\n\nTopics:\n{}", facts.owner, facts.name, facts.description, facts.default_branch, languages, facts.topics.join(", ")),
            evidence_type: EvidenceType::GitHubRepository,
            source_type: EvidenceSourceType::GitHubApi,
            source_uri: facts.url.clone(),
        }, now)?,
        "repository".into(),
        facts.repository_id.clone(),
    )];
    for commit in &facts.recent_commits {
        items.push((
            Evidence::create(
                Uuid::now_v7().to_string(),
                project_id.to_owned(),
                EvidenceDetails {
                    title: truncate(
                        &format!(
                            "GitHub commit {}: {}",
                            &commit.sha.chars().take(12).collect::<String>(),
                            commit.message
                        ),
                        200,
                    ),
                    description: format!(
                        "SHA:\n{}\n\nMessage:\n{}\n\nAuthor:\n{}\n\nDate:\n{}",
                        commit.sha, commit.message, commit.author, commit.authored_at
                    ),
                    evidence_type: EvidenceType::GitHubCommit,
                    source_type: EvidenceSourceType::GitHubApi,
                    source_uri: commit.url.clone(),
                },
                now,
            )?,
            "commit".into(),
            commit.sha.clone(),
        ));
    }
    if let Some(release) = &facts.latest_release {
        items.push((
            Evidence::create(
                Uuid::now_v7().to_string(),
                project_id.to_owned(),
                EvidenceDetails {
                    title: truncate(&format!("GitHub release {}", release.tag_name), 200),
                    description: format!(
                        "Tag:\n{}\n\nName:\n{}\n\nPublished At:\n{}",
                        release.tag_name,
                        release.name,
                        release.published_at.as_deref().unwrap_or("Unknown")
                    ),
                    evidence_type: EvidenceType::GitHubRelease,
                    source_type: EvidenceSourceType::GitHubApi,
                    source_uri: release.url.clone(),
                },
                now,
            )?,
            "release".into(),
            release.id.clone(),
        ));
    }
    Ok(items)
}

fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut hyphen = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            hyphen = false;
        } else if !hyphen && !result.is_empty() {
            result.push('-');
            hyphen = true;
        }
    }
    let mut result = result
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned();
    if result.len() < 2 {
        result = format!("gh-{result}");
    }
    result
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
fn ensure_version(expected: i64, actual: i64) -> Result<(), GitHubImportError> {
    if expected == actual {
        Ok(())
    } else {
        Err(GitHubImportError::VersionConflict { expected, actual })
    }
}
fn secret(error: crate::ports::SecretStoreError) -> GitHubImportError {
    GitHubImportError::SecretStore(error.to_string())
}
fn map_store(error: RepositoryError) -> GitHubImportError {
    match error {
        RepositoryError::Conflict => GitHubImportError::DraftConflict,
        RepositoryError::OptimisticConflict => {
            GitHubImportError::Storage("unexpected optimistic conflict".into())
        }
        RepositoryError::CorruptData(message) => GitHubImportError::CorruptData(message),
        other => GitHubImportError::Storage(other.to_string()),
    }
}
