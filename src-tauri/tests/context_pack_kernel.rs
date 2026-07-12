use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use forgeos_lib::{
    application::{
        ai_provider::{CreateAiProviderInput, SetAiProviderEnabledInput},
        AiProviderKernel, ContextPackError, ContextPackKernel,
    },
    infrastructure::{
        context_renderer::MarkdownContextRenderer, database, repositories::SqliteProjectRepository,
    },
    ports::{
        AiClientError, AiCompletion, AiMessage, AiProviderClient, AiTokenUsage, ApiKeyStore,
        SecretStoreError,
    },
};
use tempfile::TempDir;

#[derive(Default)]
struct MemoryApiKeyStore {
    values: Mutex<HashMap<String, String>>,
}

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn set(&self, provider_id: &str, api_key: &str) -> Result<(), SecretStoreError> {
        self.values
            .lock()
            .unwrap()
            .insert(provider_id.into(), api_key.into());
        Ok(())
    }

    async fn get(&self, provider_id: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self.values.lock().unwrap().get(provider_id).cloned())
    }

    async fn clear(&self, provider_id: &str) -> Result<(), SecretStoreError> {
        self.values.lock().unwrap().remove(provider_id);
        Ok(())
    }
}

#[derive(Default)]
struct FakeAiClient {
    calls: AtomicUsize,
    messages: Mutex<Vec<AiMessage>>,
}

#[async_trait]
impl AiProviderClient for FakeAiClient {
    async fn complete(
        &self,
        provider: &forgeos_lib::domain::AiProvider,
        _api_key: Option<String>,
        messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        *self.messages.lock().unwrap() = messages.to_vec();
        Ok(AiCompletion {
            content: "Risk: validate the current blocker first.".into(),
            model: provider.model().into(),
            finish_reason: Some("stop".into()),
            usage: Some(AiTokenUsage {
                prompt_tokens: Some(100),
                completion_tokens: Some(10),
                total_tokens: Some(110),
            }),
        })
    }
}

#[tokio::test]
async fn deterministic_isolated_context_and_manual_ai_do_not_modify_facts() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("forgeos-context-pack.db");
    let database = database::connect(&path).await.unwrap();
    seed_three_projects(&database.pool).await;

    let invalid_template = sqlx::query(
        "INSERT INTO context_exports (id, project_id, template_type, content_markdown, \
         content_hash, created_at) VALUES ('invalid-template', 'project-codelens', 'agent', \
         'x', '0000000000000000000000000000000000000000000000000000000000000000', \
         '2026-07-11T04:00:00Z')",
    )
    .execute(&database.pool)
    .await;
    assert!(
        invalid_template.is_err(),
        "template CHECK must reject unknown values"
    );
    let orphan_export = sqlx::query(
        "INSERT INTO context_exports (id, project_id, template_type, content_markdown, \
         content_hash, created_at) VALUES ('orphan-export', 'missing-project', 'planning', \
         'x', '0000000000000000000000000000000000000000000000000000000000000000', \
         '2026-07-11T04:00:00Z')",
    )
    .execute(&database.pool)
    .await;
    assert!(
        orphan_export.is_err(),
        "Project foreign key must reject orphan exports"
    );

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let secrets = Arc::new(MemoryApiKeyStore::default());
    let client = Arc::new(FakeAiClient::default());
    let ai_kernel = Arc::new(AiProviderKernel::new(
        repository.clone(),
        secrets.clone(),
        client.clone(),
    ));
    let context_kernel = ContextPackKernel::new(
        repository,
        Arc::new(MarkdownContextRenderer),
        ai_kernel.clone(),
    );

    let first = context_kernel
        .generate_context_pack("project-codelens", "planning")
        .await
        .unwrap();
    let second = context_kernel
        .generate_context_pack("project-codelens", "planning")
        .await
        .unwrap();
    assert_eq!(first.content_markdown(), second.content_markdown());
    assert_eq!(first.content_hash(), second.content_hash());
    assert_eq!(first.snapshot_version(), Some(2));
    assert_eq!(
        first.generated_from_session_ids(),
        &["session-codelens".to_string()]
    );
    assert!(first.content_markdown().contains("CodeLens-Pro"));
    assert!(first.content_markdown().contains("README evidence"));
    assert!(!first.content_markdown().contains("ForgeOS private marker"));
    assert!(!first.content_markdown().contains("Test project marker"));

    let forgeos = context_kernel
        .generate_context_pack("project-forgeos", "debugging")
        .await
        .unwrap();
    let test_project = context_kernel
        .generate_context_pack("project-test", "handoff")
        .await
        .unwrap();
    assert!(forgeos
        .content_markdown()
        .contains("ForgeOS private marker"));
    assert!(test_project
        .content_markdown()
        .contains("Test project marker"));
    assert_ne!(forgeos.content_hash(), test_project.content_hash());

    let provider = ai_kernel
        .create_provider(CreateAiProviderInput {
            name: "Test Provider".into(),
            base_url: "http://localhost:1234/v1".into(),
            model: "test-model".into(),
        })
        .await
        .unwrap();
    ai_kernel
        .set_api_key(provider.provider.id(), "not-in-sqlite".into())
        .await
        .unwrap();

    let facts_before = fact_signature(&database.pool).await;
    let disabled = context_kernel
        .invoke_context_with_ai(
            "project-codelens",
            first.id(),
            provider.provider.id(),
            "Analyze the largest risk".into(),
        )
        .await;
    assert!(matches!(disabled, Err(ContextPackError::AiProvider(_))));
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);

    let provider = ai_kernel
        .set_provider_enabled(
            provider.provider.id(),
            SetAiProviderEnabledInput {
                enabled: true,
                expected_version: provider.provider.version(),
            },
        )
        .await
        .unwrap();
    let result = context_kernel
        .invoke_context_with_ai(
            "project-codelens",
            first.id(),
            provider.provider.id(),
            "Analyze the largest risk".into(),
        )
        .await
        .unwrap();
    assert_eq!(
        result.ai.completion.content,
        "Risk: validate the current blocker first."
    );
    assert_eq!(result.context_export.status().as_str(), "sent_to_ai");
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    let messages = client.messages.lock().unwrap();
    assert_eq!(messages.len(), 2);
    assert!(messages[0]
        .content
        .contains("analysis and suggestions only"));
    assert!(messages[1].content.contains("CodeLens-Pro"));
    assert!(messages[1].content.contains("Analyze the largest risk"));
    drop(messages);

    assert_eq!(facts_before, fact_signature(&database.pool).await);
    let leaked_transient_data: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM context_exports \
         WHERE content_markdown LIKE '%Analyze the largest risk%' \
         OR content_markdown LIKE '%Risk: validate the current blocker first.%'",
    )
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(
        leaked_transient_data, 0,
        "prompt and response must not be persisted"
    );
    let key_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('ai_providers') WHERE lower(name) LIKE '%key%'",
    )
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(key_columns, 0, "API keys must stay outside SQLite");

    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(integrity, "ok");
    let foreign_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(foreign_keys, 0);

    let first_id = first.id().to_owned();
    database.pool.close().await;
    let reopened = database::connect(&path).await.unwrap();
    let reopened_repository = Arc::new(SqliteProjectRepository::new(reopened.pool.clone()));
    let reopened_ai = Arc::new(AiProviderKernel::new(
        reopened_repository.clone(),
        secrets,
        client,
    ));
    let reopened_context = ContextPackKernel::new(
        reopened_repository,
        Arc::new(MarkdownContextRenderer),
        reopened_ai,
    );
    let persisted = reopened_context
        .get_context_export("project-codelens", &first_id)
        .await
        .unwrap();
    assert_eq!(persisted.status().as_str(), "sent_to_ai");
    assert_eq!(persisted.content_hash(), first.content_hash());
    reopened.pool.close().await;
}

async fn seed_three_projects(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z')")
        .execute(pool).await.unwrap();
    for (id, slug, name, description) in [
        (
            "project-codelens",
            "codelens-pro",
            "CodeLens-Pro",
            "CodeLens isolated marker",
        ),
        (
            "project-forgeos",
            "forgeos",
            "ForgeOS",
            "ForgeOS private marker",
        ),
        (
            "project-test",
            "test-project",
            "测试项目",
            "Test project marker",
        ),
    ] {
        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES (?, 'workspace-1', ?, ?, ?, 'Reliable engineering context', 'Independent developer', 'Local-first kernel', 'No task management', 'React, Tauri, Rust, SQLite', 'active', 'implementation', 80, '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z', 1)")
            .bind(id).bind(slug).bind(name).bind(description).execute(pool).await.unwrap();
    }
    for (project, marker) in [
        ("project-codelens", "CodeLens snapshot"),
        ("project-forgeos", "ForgeOS private marker"),
        ("project-test", "Test project marker"),
    ] {
        sqlx::query("INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at) VALUES (?, ?, 2, ?, 'Implementation active', '[\"Kernel complete\"]', '[\"Manual blocker\"]', '[\"Validate\"]', '2026-07-11T02:00:00Z', '2026-07-11T02:00:00Z', '2026-07-11T02:00:00Z')")
            .bind(format!("snapshot-{project}")).bind(project).bind(marker).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version) VALUES ('session-codelens', 'project-codelens', 'completed', 'Optimize indexing', '', 'Index improved', 'cargo test passed', 'Continue benchmarks', '2026-07-11T01:00:00Z', '2026-07-11T01:30:00Z', '2026-07-11T01:00:00Z', '2026-07-11T01:30:00Z', 2)")
        .execute(pool).await.unwrap();
    for (id, project, title, description) in [
        (
            "evidence-codelens-1",
            "project-codelens",
            "README evidence",
            "CodeLens documentation",
        ),
        (
            "evidence-codelens-2",
            "project-codelens",
            "Test result",
            "cargo test passed",
        ),
        (
            "evidence-forgeos-1",
            "project-forgeos",
            "Architecture",
            "ForgeOS private marker",
        ),
        (
            "evidence-forgeos-2",
            "project-forgeos",
            "Build",
            "ForgeOS build passed",
        ),
        (
            "evidence-test-1",
            "project-test",
            "Manual note",
            "Test project marker",
        ),
        (
            "evidence-test-2",
            "project-test",
            "Screenshot",
            "Test screen",
        ),
    ] {
        sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES (?, ?, ?, ?, 'note', 'manual_note', '', 'verified', '2026-07-11T03:00:00Z', '2026-07-11T03:00:00Z', 1)")
            .bind(id).bind(project).bind(title).bind(description).execute(pool).await.unwrap();
    }
}

async fn fact_signature(pool: &sqlx::SqlitePool) -> (i64, i64, i64, i64, i64) {
    (
        sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM projects")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM project_snapshots")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM development_sessions")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM evidences")
            .fetch_one(pool)
            .await
            .unwrap(),
    )
}
