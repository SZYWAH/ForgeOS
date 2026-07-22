use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use forgeos_lib::{
    application::{GitHubAnalysisKernel, StartGitHubAnalysisInput},
    domain::{
        GitHubAnalysisReadiness, GitHubAnalysisStatus, GitHubConnector, GitHubRecentActivityKind,
        GitHubRepositoryKind, Workspace,
    },
    infrastructure::{database, github::GitHubRestClient, repositories::SqliteProjectRepository},
    ports::{
        ApiKeyStore, GitHubClient, GitHubImportRepository, SecretStoreError, WorkspaceRepository,
    },
};

#[derive(Default)]
struct EnvironmentGitHubTokenStore;

#[async_trait]
impl ApiKeyStore for EnvironmentGitHubTokenStore {
    async fn set(&self, _id: &str, _value: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }

    async fn get(&self, _id: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(std::env::var("GITHUB_TOKEN").ok())
    }

    async fn clear(&self, _id: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires live access to the public GitHub REST API"]
async fn reads_public_repository_through_read_only_api() {
    let client = GitHubRestClient::new().expect("GitHub client should initialize");
    let token = std::env::var("GITHUB_TOKEN").ok();
    let facts = client
        .inspect_repository("octocat", "Hello-World", token.as_deref())
        .await
        .expect("public repository should be readable without a token");

    assert_eq!(facts.owner, "octocat");
    assert_eq!(facts.name, "Hello-World");
    assert!(facts
        .url
        .starts_with("https://github.com/octocat/Hello-World"));
    assert!(!facts.default_branch.is_empty());
    assert!(!facts.recent_commits.is_empty());
}

#[tokio::test]
#[ignore = "requires live access to the public GitHub REST API"]
async fn analyzes_public_repository_with_rules_v2() {
    let repository_url = std::env::var("FORGEOS_LIVE_GITHUB_REPOSITORY")
        .unwrap_or_else(|_| "https://github.com/obsidianmd/obsidian-releases".into());
    let expected_kind = std::env::var("FORGEOS_LIVE_GITHUB_EXPECTED_KIND")
        .unwrap_or_else(|_| "release_registry".into());
    let directory = tempfile::tempdir().expect("temporary analysis database");
    let database = database::connect(&directory.path().join("forgeos.db"))
        .await
        .expect("database migrations should succeed");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    repository
        .insert_workspace(
            &Workspace::new(
                "workspace-live".into(),
                "Live GitHub quality smoke".into(),
                "Asia/Shanghai".into(),
                Utc::now(),
            )
            .expect("valid workspace"),
        )
        .await
        .expect("workspace should persist");
    repository
        .insert_github_connector(
            &GitHubConnector::create("live-github".into(), Utc::now()).expect("valid connector"),
        )
        .await
        .expect("connector should persist");

    let kernel = GitHubAnalysisKernel::new(
        repository,
        Arc::new(GitHubRestClient::new().expect("GitHub client should initialize")),
        Arc::new(EnvironmentGitHubTokenStore),
    );
    let started = kernel
        .start(StartGitHubAnalysisInput {
            workspace_id: "workspace-live".into(),
            repository_url,
            focus_type: "overview".into(),
            custom_prompt: None,
            analysis_mode: "rules".into(),
        })
        .await
        .expect("analysis should start");
    let status = kernel
        .execute("workspace-live", &started.analysis_run_id, |_| {})
        .await
        .expect("analysis should complete");
    assert_eq!(status.run.analyzer_revision, "github-rules-v2");
    assert_ne!(status.run.readiness, GitHubAnalysisReadiness::Blocked);

    let view = kernel
        .report("workspace-live", &started.analysis_run_id)
        .await
        .expect("report should load");
    let source_paths = view
        .sources
        .iter()
        .map(|source| format!("{}:{}", source.source_type.as_str(), source.source_path))
        .collect::<Vec<_>>();
    let report = view.report.expect("rules-v2 report should exist");
    assert_eq!(report.schema_version.as_deref(), Some("github-report-v2"));
    let expected_kind =
        GitHubRepositoryKind::from_str(&expected_kind).expect("valid expected kind");
    let actual_kind = report
        .repository_kind
        .as_ref()
        .expect("repository kind")
        .value;
    assert_eq!(
        actual_kind, expected_kind,
        "collected sources: {source_paths:?}"
    );
    match expected_kind {
        GitHubRepositoryKind::ReleaseRegistry => {
            assert_eq!(status.run.status, GitHubAnalysisStatus::Ready);
            assert_eq!(status.run.readiness, GitHubAnalysisReadiness::Warning);
            assert!(report.product_tech_stack.is_empty());
            assert!(report
                .recent_activity
                .iter()
                .any(|activity| activity.kind == GitHubRecentActivityKind::Commit));
            assert!(report
                .recent_activity
                .iter()
                .any(|activity| activity.kind == GitHubRecentActivityKind::Release));
        }
        GitHubRepositoryKind::SourceApplication | GitHubRepositoryKind::Monorepo => {
            assert!(!report.product_tech_stack.is_empty());
        }
        _ => {}
    }
    assert!(view
        .content_markdown
        .as_deref()
        .is_some_and(|value| !value.is_empty()));
    assert!(view
        .content_hash
        .as_deref()
        .is_some_and(|value| value.len() == 64));
}
