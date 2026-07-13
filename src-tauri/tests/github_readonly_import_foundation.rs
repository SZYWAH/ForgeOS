use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use forgeos_lib::{
    application::{GitHubImportError, GitHubImportKernel},
    domain::{
        GitHubCommitFact, GitHubReleaseFact, GitHubRepositoryFacts, LifecycleStage, Project,
        ProjectDetails, ProjectStatus, Workspace,
    },
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{
        ApiKeyStore, GitHubClient, GitHubClientError, ProjectRepository, SecretStoreError,
        WorkspaceRepository,
    },
};
use tempfile::TempDir;

#[derive(Default)]
struct MemorySecretStore {
    value: Mutex<Option<String>>,
}

#[async_trait]
impl ApiKeyStore for MemorySecretStore {
    async fn set(&self, _id: &str, value: &str) -> Result<(), SecretStoreError> {
        *self.value.lock().unwrap() = Some(value.to_owned());
        Ok(())
    }
    async fn get(&self, _id: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self.value.lock().unwrap().clone())
    }
    async fn clear(&self, _id: &str) -> Result<(), SecretStoreError> {
        *self.value.lock().unwrap() = None;
        Ok(())
    }
}

#[derive(Default)]
struct FakeGitHubClient {
    fail: bool,
    seen_tokens: Mutex<Vec<Option<String>>>,
}

#[async_trait]
impl GitHubClient for FakeGitHubClient {
    async fn inspect_repository(
        &self,
        owner: &str,
        name: &str,
        token: Option<&str>,
    ) -> Result<GitHubRepositoryFacts, GitHubClientError> {
        self.seen_tokens
            .lock()
            .unwrap()
            .push(token.map(str::to_owned));
        if self.fail {
            return Err(GitHubClientError::Transport("simulated outage".into()));
        }
        Ok(GitHubRepositoryFacts {
            repository_id: format!("{owner}-{name}"),
            owner: owner.into(),
            name: name.into(),
            url: format!("https://github.com/{owner}/{name}"),
            description: "Imported repository".into(),
            homepage: Some("https://example.test".into()),
            default_branch: "main".into(),
            language: Some("Rust".into()),
            languages: vec![("Rust".into(), 900), ("TypeScript".into(), 100)],
            topics: vec!["tauri".into(), "engineering".into()],
            readme_summary: "# Existing project\nConfirmed README context.".into(),
            recent_commits: vec![GitHubCommitFact {
                sha: "a".repeat(40),
                message: "feat: imported fact".into(),
                author: "Developer".into(),
                authored_at: "2026-07-12T00:00:00Z".into(),
                url: format!(
                    "https://github.com/{owner}/{name}/commit/{}",
                    "a".repeat(40)
                ),
            }],
            latest_release: Some(GitHubReleaseFact {
                id: "release-1".into(),
                tag_name: "v0.1.0".into(),
                name: "Foundation".into(),
                published_at: Some("2026-07-12T01:00:00Z".into()),
                url: format!("https://github.com/{owner}/{name}/releases/tag/v0.1.0"),
            }),
        })
    }
}

async fn setup() -> (
    TempDir,
    database::DatabaseState,
    Arc<SqliteProjectRepository>,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = database::connect(&dir.path().join("forgeos.db"))
        .await
        .unwrap();
    let repository = Arc::new(SqliteProjectRepository::new(state.pool.clone()));
    repository
        .insert_workspace(
            &Workspace::new(
                "workspace-1".into(),
                "Personal".into(),
                "Asia/Shanghai".into(),
                Utc::now(),
            )
            .unwrap(),
        )
        .await
        .unwrap();
    (dir, state, repository)
}

#[tokio::test]
async fn github_import_is_reviewed_atomic_isolated_and_persistent() {
    let (dir, state, repository) = setup().await;
    let secrets = Arc::new(MemorySecretStore::default());
    let client = Arc::new(FakeGitHubClient::default());
    let kernel = GitHubImportKernel::new(repository.clone(), client.clone(), secrets.clone());

    let connector = kernel.get_connector().await.unwrap();
    assert!(!connector.token_configured);
    kernel
        .set_token("test-token-sentinel".into())
        .await
        .unwrap();
    let draft = kernel
        .preview_repository("https://github.com/SZYWAH/CodeLens-Pro")
        .await
        .unwrap();
    assert_eq!(
        client.seen_tokens.lock().unwrap().as_slice(),
        &[Some("test-token-sentinel".into())]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evidences")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );

    assert!(matches!(
        kernel.import_draft(draft.id(), draft.version()).await,
        Err(GitHubImportError::Validation(_))
    ));
    let reviewed = kernel
        .review_draft(draft.id(), draft.version())
        .await
        .unwrap();
    assert!(matches!(
        kernel.review_draft(reviewed.id(), draft.version()).await,
        Err(GitHubImportError::VersionConflict { .. })
    ));
    let imported = kernel
        .import_draft(reviewed.id(), reviewed.version())
        .await
        .unwrap();
    assert_eq!(imported.evidences.len(), 3);
    assert_eq!(imported.project.name, "CodeLens-Pro");
    assert_eq!(
        kernel
            .get_project_import(&imported.project.id)
            .await
            .unwrap()
            .unwrap()
            .id(),
        imported.draft.id()
    );

    let other = Project::new(
        "other-project".into(),
        "workspace-1".into(),
        ProjectDetails {
            slug: "other-project".into(),
            name: "Other".into(),
            description: String::new(),
            vision: String::new(),
            target_user: String::new(),
            scope: String::new(),
            non_goals: String::new(),
            tech_stack: String::new(),
            status: ProjectStatus::Active,
            lifecycle_stage: LifecycleStage::Implementation,
            priority: 1,
        },
        Utc::now(),
    )
    .unwrap();
    repository.insert_project(&other).await.unwrap();
    assert!(kernel
        .get_project_import(&other.id)
        .await
        .unwrap()
        .is_none());

    let connector_dump: String = sqlx::query_scalar("SELECT id || provider || auth_type || created_at || updated_at || version FROM github_connectors").fetch_one(&state.pool).await.unwrap();
    assert!(!connector_dump.contains("test-token-sentinel"));
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );

    state.pool.close().await;
    let reopened = database::connect(&dir.path().join("forgeos.db"))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM github_import_drafts WHERE status = 'imported'"
        )
        .fetch_one(&reopened.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM evidences WHERE source_type = 'github_api'"
        )
        .fetch_one(&reopened.pool)
        .await
        .unwrap(),
        3
    );
}

#[tokio::test]
async fn rejection_and_api_failure_do_not_create_projects_or_dirty_drafts() {
    let (_dir, state, repository) = setup().await;
    let kernel = GitHubImportKernel::new(
        repository.clone(),
        Arc::new(FakeGitHubClient::default()),
        Arc::new(MemorySecretStore::default()),
    );
    let draft = kernel
        .preview_repository("https://github.com/SZYWAH/Rejected")
        .await
        .unwrap();
    let rejected = kernel
        .reject_draft(draft.id(), draft.version())
        .await
        .unwrap();
    assert_eq!(rejected.status().as_str(), "rejected");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        0
    );

    let failing = GitHubImportKernel::new(
        repository,
        Arc::new(FakeGitHubClient {
            fail: true,
            ..Default::default()
        }),
        Arc::new(MemorySecretStore::default()),
    );
    assert!(matches!(
        failing
            .preview_repository("https://github.com/SZYWAH/Unavailable")
            .await,
        Err(GitHubImportError::GitHub(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM github_import_drafts")
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        1
    );
}
