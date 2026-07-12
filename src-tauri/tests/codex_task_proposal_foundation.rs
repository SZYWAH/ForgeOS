use std::sync::Arc;

use forgeos_lib::{
    application::{
        codex_task_proposal::{CreateCodexTaskProposalInput, TransitionCodexTaskProposalInput},
        CodexTaskProposalError, CodexTaskProposalKernel,
    },
    domain::CodexTaskProposalStatus,
    infrastructure::{database, repositories::SqliteProjectRepository},
};

#[tokio::test]
async fn proposal_governance_is_context_bound_versioned_and_fact_isolated() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("forgeos.db");
    let database = database::connect(&path).await.unwrap();
    seed(&database.pool).await;
    let kernel = CodexTaskProposalKernel::new(Arc::new(SqliteProjectRepository::new(
        database.pool.clone(),
    )));

    let input = || CreateCodexTaskProposalInput {
        context_export_id: "export-a".into(),
        title: "Optimize index".into(),
        description: "Proposal only".into(),
        goal: "Reduce analysis latency".into(),
        non_goals: vec!["No database migration".into()],
        allowed_scope: vec!["backend/agent/*".into()],
        forbidden_scope: vec!["auth/*".into()],
        acceptance_criteria: vec!["Tests pass".into(), "Latency improves 20%".into()],
        validation_commands: vec!["cargo test".into()],
    };
    let draft = kernel.create_proposal("project-a", input()).await.unwrap();
    assert_eq!(draft.status(), CodexTaskProposalStatus::Draft);

    let mut cross = input();
    cross.context_export_id = "export-b".into();
    assert!(matches!(
        kernel.create_proposal("project-a", cross).await,
        Err(CodexTaskProposalError::Validation(_))
    ));

    let proposed = kernel
        .submit_proposal(
            "project-a",
            draft.id(),
            TransitionCodexTaskProposalInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        kernel
            .approve_proposal(
                "project-a",
                proposed.id(),
                TransitionCodexTaskProposalInput {
                    expected_version: 1
                }
            )
            .await,
        Err(CodexTaskProposalError::VersionConflict { .. })
    ));
    let approved = kernel
        .approve_proposal(
            "project-a",
            proposed.id(),
            TransitionCodexTaskProposalInput {
                expected_version: 2,
            },
        )
        .await
        .unwrap();
    assert_eq!(approved.status(), CodexTaskProposalStatus::Approved);

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
    let archived = kernel
        .archive_proposal(
            "project-a",
            approved.id(),
            TransitionCodexTaskProposalInput {
                expected_version: 3,
            },
        )
        .await
        .unwrap();
    assert_eq!(archived.status(), CodexTaskProposalStatus::Archived);
    let history: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM codex_task_proposal_status_history WHERE proposal_id = ?",
    )
    .bind(approved.id())
    .fetch_one(&database.pool)
    .await
    .unwrap();
    assert_eq!(history, 4);

    let rejected_draft = kernel.create_proposal("project-a", input()).await.unwrap();
    let rejected_proposed = kernel
        .submit_proposal(
            "project-a",
            rejected_draft.id(),
            TransitionCodexTaskProposalInput {
                expected_version: 1,
            },
        )
        .await
        .unwrap();
    let rejected = kernel
        .reject_proposal(
            "project-a",
            rejected_proposed.id(),
            TransitionCodexTaskProposalInput {
                expected_version: 2,
            },
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), CodexTaskProposalStatus::Rejected);
    database.pool.close().await;

    let reopened = database::connect(&path).await.unwrap();
    let kernel = CodexTaskProposalKernel::new(Arc::new(SqliteProjectRepository::new(
        reopened.pool.clone(),
    )));
    assert_eq!(kernel.list_proposals("project-a").await.unwrap().len(), 2);
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
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'UTC', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES ('project-a', 'workspace-1', 'codelens-pro', 'CodeLens-Pro', '', '', '', '', '', 'Rust', 'active', 'implementation', 90, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1), ('project-b', 'workspace-1', 'forgeos', 'ForgeOS', '', '', '', '', '', 'Rust', 'active', 'implementation', 100, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1), ('project-c', 'workspace-1', 'test-project', '测试项目', '', '', '', '', '', 'TypeScript', 'planned', 'definition', 50, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1)").execute(pool).await.unwrap();
    sqlx::query("INSERT INTO context_exports (id, project_id, template_type, snapshot_version, generated_from_session_ids, content_markdown, content_hash, status, created_at, version) VALUES ('export-a', 'project-a', 'coding', NULL, '[]', '# A', '327f031b25e00b1a7cd9b0c18f05948b60f55d09f9b3d177d21083f83a3cb6df', 'generated', '2026-07-12T00:00:00Z', 1), ('export-b', 'project-b', 'coding', NULL, '[]', '# B', '216f4c6ac9ef17add951aa097934202649a6c6ea5f29263b00c59772ea6cff61', 'generated', '2026-07-12T00:00:00Z', 1)").execute(pool).await.unwrap();
}
