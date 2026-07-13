use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use forgeos_lib::{
    application::{
        codex_execution::{
            CancelCodexExecutionInput, RecordCodexExecutionResultInput,
            TransitionCodexExecutionInput,
        },
        CodexExecutionError, CodexExecutionKernel,
    },
    domain::{CodexExecutionStatus, DomainError},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{CodexExecutionRequest, CodexRunner, CodexRunnerError},
};

#[derive(Default)]
struct SpyRunner {
    accepted: AtomicUsize,
}

#[async_trait]
impl CodexRunner for SpyRunner {
    async fn accept(&self, request: &CodexExecutionRequest) -> Result<(), CodexRunnerError> {
        assert_eq!(request.project_id, "project-a");
        assert_eq!(request.proposal_id, "proposal-approved");
        assert_eq!(request.context_export_id, "export-a");
        assert_eq!(request.validation_commands, vec!["cargo test"]);
        self.accepted.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn executions_are_proposal_bound_versioned_persistent_and_fact_isolated() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("forgeos.db");
    let database = database::connect(&path).await.unwrap();
    seed(&database.pool).await;
    let runner = Arc::new(SpyRunner::default());
    let kernel = CodexExecutionKernel::new(
        Arc::new(SqliteProjectRepository::new(database.pool.clone())),
        runner.clone(),
    );

    assert!(matches!(
        kernel.create_execution("project-a", "proposal-draft").await,
        Err(CodexExecutionError::ProposalNotApproved)
    ));
    assert!(matches!(
        kernel
            .create_execution("project-b", "proposal-approved")
            .await,
        Err(CodexExecutionError::Validation(
            DomainError::CodexTaskProposalProjectMismatch
        ))
    ));

    let pending = kernel
        .create_execution("project-a", "proposal-approved")
        .await
        .unwrap();
    assert_eq!(pending.status(), CodexExecutionStatus::Pending);
    assert_eq!(runner.accepted.load(Ordering::SeqCst), 1);
    assert!(kernel
        .list_executions("project-b")
        .await
        .unwrap()
        .is_empty());
    assert!(matches!(
        kernel.get_execution("project-b", pending.id()).await,
        Err(CodexExecutionError::Validation(
            DomainError::CodexExecutionProjectMismatch
        ))
    ));
    assert!(matches!(
        kernel
            .start_execution(
                "project-a",
                pending.id(),
                TransitionCodexExecutionInput {
                    expected_version: 99,
                },
            )
            .await,
        Err(CodexExecutionError::VersionConflict { .. })
    ));

    let running = kernel
        .start_execution(
            "project-a",
            pending.id(),
            TransitionCodexExecutionInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(running.status(), CodexExecutionStatus::Running);
    assert!(running.started_at().is_some());
    assert!(matches!(
        kernel
            .complete_execution(
                "project-a",
                running.id(),
                RecordCodexExecutionResultInput {
                    output_summary: String::new(),
                    validation_result: "cargo test passed".into(),
                    expected_version: 2,
                },
            )
            .await,
        Err(CodexExecutionError::Validation(
            DomainError::CodexExecutionResultRequired
        ))
    ));
    let success = kernel
        .complete_execution(
            "project-a",
            running.id(),
            RecordCodexExecutionResultInput {
                output_summary: "Recorded the implementation result".into(),
                validation_result: "cargo test passed".into(),
                expected_version: 2,
            },
        )
        .await
        .unwrap();
    assert_eq!(success.status(), CodexExecutionStatus::Success);
    assert_eq!(success.version(), 3);
    assert!(success.ended_at().is_some());

    let failed = kernel
        .create_execution("project-a", "proposal-approved")
        .await
        .unwrap();
    let failed = kernel
        .start_execution(
            "project-a",
            failed.id(),
            TransitionCodexExecutionInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    let failed = kernel
        .fail_execution(
            "project-a",
            failed.id(),
            RecordCodexExecutionResultInput {
                output_summary: "Runner reported a failure".into(),
                validation_result: "Validation did not pass".into(),
                expected_version: 2,
            },
        )
        .await
        .unwrap();
    assert_eq!(failed.status(), CodexExecutionStatus::Failed);

    let cancelled = kernel
        .create_execution("project-a", "proposal-approved")
        .await
        .unwrap();
    let cancelled = kernel
        .cancel_execution(
            "project-a",
            cancelled.id(),
            CancelCodexExecutionInput {
                output_summary: "Cancelled before start".into(),
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert_eq!(cancelled.status(), CodexExecutionStatus::Cancelled);
    assert!(cancelled.started_at().is_none());
    assert!(cancelled.ended_at().is_some());

    for table in [
        "projects",
        "project_snapshots",
        "development_sessions",
        "evidences",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&database.pool)
            .await
            .unwrap();
        assert_eq!(count, if table == "projects" { 3 } else { 0 });
    }
    let history_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM codex_execution_status_history WHERE execution_id = ?",
    )
    .bind(success.id())
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(history_count, 3);

    database.pool.close().await;
    let reopened = database::connect(&path).await.unwrap();
    let reopened_kernel = CodexExecutionKernel::new(
        Arc::new(SqliteProjectRepository::new(reopened.pool.clone())),
        Arc::new(SpyRunner::default()),
    );
    let persisted = reopened_kernel.list_executions("project-a").await.unwrap();
    assert_eq!(persisted.len(), 3);
    assert!(persisted
        .iter()
        .any(|item| item.status() == CodexExecutionStatus::Success));
    assert!(persisted
        .iter()
        .any(|item| item.status() == CodexExecutionStatus::Failed));
    assert!(persisted
        .iter()
        .any(|item| item.status() == CodexExecutionStatus::Cancelled));
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&reopened.pool)
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&reopened.pool)
            .await
            .unwrap(),
        0
    );
}

async fn seed(pool: &sqlx::SqlitePool) {
    sqlx::raw_sql(
        "INSERT INTO workspaces (id, name, timezone, created_at, updated_at)
         VALUES ('workspace-1', 'Personal', 'UTC', '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');
         INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user,
           scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version)
         VALUES
           ('project-a', 'workspace-1', 'codelens-pro', 'CodeLens-Pro', '', '', '', '', '', 'Rust', 'active', 'implementation', 90, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z', 1),
           ('project-b', 'workspace-1', 'forgeos', 'ForgeOS', '', '', '', '', '', 'Rust', 'active', 'implementation', 100, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z', 1),
           ('project-c', 'workspace-1', 'test-project', '测试项目', '', '', '', '', '', 'TypeScript', 'planned', 'definition', 50, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z', 1);
         INSERT INTO context_exports (id, project_id, template_type, snapshot_version,
           generated_from_session_ids, content_markdown, content_hash, status, created_at, version)
         VALUES ('export-a', 'project-a', 'coding', NULL, '[]', '# A',
           '327f031b25e00b1a7cd9b0c18f05948b60f55d09f9b3d177d21083f83a3cb6df',
           'generated', '2026-07-13T00:00:00Z', 1);
         INSERT INTO codex_task_proposals (id, project_id, context_export_id, title, description,
           goal, non_goals, allowed_scope, forbidden_scope, acceptance_criteria,
           validation_commands, status, version, created_at, updated_at)
         VALUES
           ('proposal-approved', 'project-a', 'export-a', 'Implement D5', 'Proposal only',
            'Build execution records', '[]', '[\"src-tauri/*\"]', '[\"codex exec\"]',
            '[\"Tests pass\"]', '[\"cargo test\"]', 'approved', 3,
            '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z'),
           ('proposal-draft', 'project-a', 'export-a', 'Draft only', '', 'Not approved',
            '[]', '[]', '[]', '[\"Review first\"]', '[]', 'draft', 1,
            '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');",
    )
    .execute(pool)
    .await
    .unwrap();
}
