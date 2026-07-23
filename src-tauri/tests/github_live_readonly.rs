use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use forgeos_lib::{
    application::{
        ai_provider::{CreateAiProviderInput, SetAiProviderEnabledInput},
        AiProviderKernel, GitHubAnalysisKernel, StartGitHubAnalysisInput,
    },
    domain::{
        GitHubAnalysisReadiness, GitHubAnalysisStatus, GitHubAnalysisWarning, GitHubConnector,
        GitHubRecentActivityKind, GitHubRepositoryKind, Workspace,
    },
    infrastructure::{
        ai_client::OpenAiCompatibleClient, database, github::GitHubRestClient,
        repositories::SqliteProjectRepository,
    },
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
            ai_provider_id: None,
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

#[tokio::test]
#[ignore = "requires live GitHub access and DEEPSEEK_API_KEY"]
async fn enhances_public_repository_with_environment_deepseek_provider() {
    let repository_url = std::env::var("FORGEOS_LIVE_GITHUB_REPOSITORY")
        .unwrap_or_else(|_| "https://github.com/obsidianmd/obsidian-releases".into());
    let model =
        std::env::var("FORGEOS_LIVE_DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".into());
    assert!(
        std::env::var("DEEPSEEK_API_KEY").is_ok_and(|value| !value.trim().is_empty()),
        "DEEPSEEK_API_KEY must be available to the test process"
    );

    let directory = tempfile::tempdir().expect("temporary analysis database");
    let database = database::connect(&directory.path().join("forgeos-ai-live.db"))
        .await
        .expect("database migrations should succeed");
    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    repository
        .insert_workspace(
            &Workspace::new(
                "workspace-ai-live".into(),
                "Live GitHub AI smoke".into(),
                "Asia/Shanghai".into(),
                Utc::now(),
            )
            .expect("valid workspace"),
        )
        .await
        .expect("workspace should persist");
    repository
        .insert_github_connector(
            &GitHubConnector::create("live-github-ai".into(), Utc::now()).expect("valid connector"),
        )
        .await
        .expect("connector should persist");

    let ai_kernel = AiProviderKernel::new(
        repository.clone(),
        Arc::new(EnvironmentGitHubTokenStore),
        Arc::new(OpenAiCompatibleClient::new().expect("AI client should initialize")),
    );
    let provider = ai_kernel
        .create_provider(CreateAiProviderInput {
            name: "DeepSeek live".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            model,
            protocol: "openai_chat_completions".into(),
            auth_scheme: "bearer".into(),
            secret_source: "environment".into(),
            secret_required: true,
            environment_variable: Some("DEEPSEEK_API_KEY".into()),
            custom_auth_header: None,
            api_version: None,
        })
        .await
        .expect("DeepSeek Provider should be created");
    let provider = ai_kernel
        .set_provider_enabled(
            provider.provider.id(),
            SetAiProviderEnabledInput {
                enabled: true,
                expected_version: provider.provider.version(),
            },
        )
        .await
        .expect("DeepSeek Provider should enable");
    let diagnostic = ai_kernel
        .test_connection(provider.provider.id())
        .await
        .expect("DeepSeek connection diagnostic should succeed");
    assert_eq!(diagnostic.protocol, "openai_chat_completions");

    let kernel = GitHubAnalysisKernel::new(
        repository,
        Arc::new(GitHubRestClient::new().expect("GitHub client should initialize")),
        Arc::new(EnvironmentGitHubTokenStore),
    )
    .with_ai_provider(Arc::new(ai_kernel));
    let started = kernel
        .start(StartGitHubAnalysisInput {
            workspace_id: "workspace-ai-live".into(),
            repository_url,
            focus_type: "overview".into(),
            custom_prompt: None,
            analysis_mode: "ai".into(),
            ai_provider_id: Some(provider.provider.id().into()),
        })
        .await
        .expect("AI analysis should start");
    let status = kernel
        .execute("workspace-ai-live", &started.analysis_run_id, |_| {})
        .await
        .expect("AI analysis should complete");
    assert!(status.run.ai_invoked);
    assert_eq!(status.run.analyzer_revision, "github-ai-v1");
    let view = kernel
        .report("workspace-ai-live", &started.analysis_run_id)
        .await
        .expect("AI report should load");
    let report = view.report.expect("AI report should exist");
    assert!(
        !report
            .quality_warnings
            .contains(&GitHubAnalysisWarning::AiFallback),
        "DeepSeek fell back to rules with warnings: {:?}",
        report.quality_warnings
    );
    assert!(!report
        .quality_warnings
        .contains(&GitHubAnalysisWarning::AiInvalidResponse));
    assert!(view
        .content_markdown
        .as_deref()
        .is_some_and(|value| !value.is_empty()));
}
