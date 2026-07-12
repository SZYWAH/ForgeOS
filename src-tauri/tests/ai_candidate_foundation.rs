use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use forgeos_lib::{
    application::{
        ai_candidate::{CreateAiCandidateInput, ReviewAiCandidateInput},
        ai_provider::{CreateAiProviderInput, SetAiProviderEnabledInput},
        AiCandidateError, AiCandidateKernel, AiProviderKernel, ContextPackKernel,
    },
    domain::DomainError,
    infrastructure::{
        context_renderer::MarkdownContextRenderer, database, repositories::SqliteProjectRepository,
    },
    ports::{
        AiClientError, AiCompletion, AiMessage, AiProviderClient, ApiKeyStore, SecretStoreError,
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
struct FakeAiClient;

#[async_trait]
impl AiProviderClient for FakeAiClient {
    async fn complete(
        &self,
        provider: &forgeos_lib::domain::AiProvider,
        _api_key: Option<String>,
        _messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        Ok(AiCompletion {
            content: "AI suggests validating repository performance before release.".into(),
            model: provider.model().into(),
            finish_reason: Some("stop".into()),
            usage: None,
        })
    }
}

#[tokio::test]
async fn ai_response_candidate_review_is_isolated_versioned_and_persistent() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("forgeos-ai-candidates.db");
    let database = database::connect(&path).await.unwrap();
    seed_three_projects(&database.pool).await;

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let ai_kernel = Arc::new(AiProviderKernel::new(
        repository.clone(),
        Arc::new(MemoryApiKeyStore::default()),
        Arc::new(FakeAiClient),
    ));
    let context_kernel = ContextPackKernel::new(
        repository.clone(),
        Arc::new(MarkdownContextRenderer),
        ai_kernel.clone(),
    );
    let candidate_kernel = AiCandidateKernel::new(repository);

    let provider = ai_kernel
        .create_provider(CreateAiProviderInput {
            name: "Candidate Test Provider".into(),
            base_url: "http://localhost:1234/v1".into(),
            model: "test-model".into(),
        })
        .await
        .unwrap();
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

    let facts_before = fact_signature(&database.pool).await;
    let mut created = Vec::new();
    for (project_id, candidate_type, title) in [
        ("project-codelens", "risk_finding", "Indexing risk"),
        (
            "project-forgeos",
            "snapshot_update",
            "D1.6 state suggestion",
        ),
        ("project-test", "general_note", "Test project note"),
    ] {
        let export = context_kernel
            .generate_context_pack(project_id, "review")
            .await
            .unwrap();
        let response = context_kernel
            .invoke_context_with_ai(
                project_id,
                export.id(),
                provider.provider.id(),
                "Find the largest risk".into(),
            )
            .await
            .unwrap();
        let candidate = candidate_kernel
            .create_ai_candidate(
                project_id,
                CreateAiCandidateInput {
                    candidate_type: candidate_type.into(),
                    source_context_export_id: export.id().into(),
                    title: title.into(),
                    content_markdown: response.ai.completion.content,
                },
            )
            .await
            .unwrap();
        assert_eq!(candidate.status().as_str(), "pending");
        assert_eq!(candidate.version(), 1);
        created.push((project_id.to_owned(), export.id().to_owned(), candidate));
    }
    assert_eq!(facts_before, fact_signature(&database.pool).await);

    let codelens = candidate_kernel
        .accept_candidate(
            &created[0].0,
            created[0].2.id(),
            ReviewAiCandidateInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(codelens.status().as_str(), "accepted");
    assert_eq!(codelens.version(), 2);
    assert_eq!(facts_before, fact_signature(&database.pool).await);

    let stale = candidate_kernel
        .reject_candidate(
            &created[0].0,
            created[0].2.id(),
            ReviewAiCandidateInput {
                expected_version: 1,
            },
        )
        .await;
    assert!(matches!(
        stale,
        Err(AiCandidateError::VersionConflict {
            expected: 1,
            actual: 2
        })
    ));

    let rejected = candidate_kernel
        .reject_candidate(
            &created[1].0,
            created[1].2.id(),
            ReviewAiCandidateInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(rejected.status().as_str(), "rejected");
    let archived = candidate_kernel
        .archive_candidate(
            &created[2].0,
            created[2].2.id(),
            ReviewAiCandidateInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(archived.status().as_str(), "archived");
    assert_eq!(facts_before, fact_signature(&database.pool).await);

    let cross_project = candidate_kernel
        .create_ai_candidate(
            "project-forgeos",
            CreateAiCandidateInput {
                candidate_type: "general_note".into(),
                source_context_export_id: created[0].1.clone(),
                title: "Cross-project candidate".into(),
                content_markdown: "Must be rejected".into(),
            },
        )
        .await;
    assert!(matches!(
        cross_project,
        Err(AiCandidateError::Validation(
            DomainError::ContextExportProjectMismatch
        ))
    ));

    let direct_cross_project = sqlx::query(
        "INSERT INTO ai_candidates (id, project_id, candidate_type, \
         source_context_export_id, title, content_markdown, status, version, created_at, updated_at) \
         VALUES ('candidate-cross', 'project-forgeos', 'general_note', ?, 'Cross', 'Reject', \
         'pending', 1, '2026-07-11T06:00:00Z', '2026-07-11T06:00:00Z')",
    )
    .bind(&created[0].1)
    .execute(&database.pool)
    .await;
    assert!(
        direct_cross_project.is_err(),
        "composite foreign key must enforce Project isolation"
    );

    let all_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_candidates")
        .fetch_one(&database.pool)
        .await
        .unwrap();
    assert_eq!(
        all_count, 3,
        "reject and archive must retain Candidate history"
    );
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

    let ids = created
        .iter()
        .map(|(_, _, item)| item.id().to_owned())
        .collect::<Vec<_>>();
    database.pool.close().await;
    let reopened = database::connect(&path).await.unwrap();
    let reopened_kernel = AiCandidateKernel::new(Arc::new(SqliteProjectRepository::new(
        reopened.pool.clone(),
    )));
    let expected = ["accepted", "rejected", "archived"];
    for ((project_id, _, _), (candidate_id, status)) in created.iter().zip(ids.iter().zip(expected))
    {
        let candidate = reopened_kernel
            .get_candidate(project_id, candidate_id)
            .await
            .unwrap();
        assert_eq!(candidate.status().as_str(), status);
        assert_eq!(candidate.version(), 2);
    }
    reopened.pool.close().await;
}

async fn seed_three_projects(pool: &sqlx::SqlitePool) {
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z')")
        .execute(pool).await.unwrap();
    for (id, slug, name) in [
        ("project-codelens", "codelens-pro", "CodeLens-Pro"),
        ("project-forgeos", "forgeos", "ForgeOS"),
        ("project-test", "test-project", "测试项目"),
    ] {
        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES (?, 'workspace-1', ?, ?, 'Project description', 'Reliable engineering memory', 'Independent developer', 'Local-first', 'No task manager', 'React, Rust, SQLite', 'active', 'implementation', 80, '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z', 1)")
            .bind(id).bind(slug).bind(name).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at) VALUES (?, ?, 1, 'Complete D1.6', 'Implementation active', '[]', '[]', '[\"Review candidates\"]', '2026-07-11T01:00:00Z', '2026-07-11T01:00:00Z', '2026-07-11T01:00:00Z')")
            .bind(format!("snapshot-{id}")).bind(id).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version) VALUES (?, ?, 'completed', 'Implement kernel', '', 'Kernel complete', 'cargo test passed', 'Review output', '2026-07-11T02:00:00Z', '2026-07-11T03:00:00Z', '2026-07-11T02:00:00Z', '2026-07-11T03:00:00Z', 2)")
            .bind(format!("session-{id}")).bind(id).execute(pool).await.unwrap();
        sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES (?, ?, 'Test result', 'cargo test passed', 'test_result', 'manual_note', '', 'verified', '2026-07-11T04:00:00Z', '2026-07-11T04:00:00Z', 1)")
            .bind(format!("evidence-{id}")).bind(id).execute(pool).await.unwrap();
    }
}

async fn fact_signature(pool: &sqlx::SqlitePool) -> (i64, i64, i64, i64, i64, i64, i64, i64) {
    (
        sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM projects")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM project_snapshots")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM development_sessions")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM development_sessions")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COUNT(*) FROM evidences")
            .fetch_one(pool)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT COALESCE(SUM(version), 0) FROM evidences")
            .fetch_one(pool)
            .await
            .unwrap(),
    )
}
