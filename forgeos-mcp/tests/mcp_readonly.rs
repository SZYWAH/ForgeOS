use std::sync::Arc;

use forgeos_lib::{
    application::McpQueryKernel,
    infrastructure::{
        context_renderer::MarkdownContextRenderer, database, repositories::SqliteProjectRepository,
    },
};
use forgeos_mcp::server::ForgeOsMcpServer;
use rmcp::{
    model::{CallToolRequestParams, ClientInfo},
    ClientHandler, ServiceExt,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const PROJECT_A_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PRIVATE_PATH: &str = "D:/private/CodeLens-Pro/TOKEN-CREDENTIAL";
const PRIVATE_EMAIL: &str = "private-author@example.com";

#[derive(Debug, Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

#[tokio::test]
async fn stdio_intelligence_queries_are_stable_isolated_safe_and_read_only() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let database_path = directory.path().join("forgeos.db");
    let writable = database::connect(&database_path).await?;
    seed(&writable.pool).await?;
    writable.pool.close().await;

    let readonly = database::connect_read_only(&database_path).await?;
    let readonly_pool = readonly.pool.clone();
    let repository = Arc::new(SqliteProjectRepository::new(readonly.pool));
    let kernel = Arc::new(McpQueryKernel::new(
        repository,
        Arc::new(MarkdownContextRenderer),
    ));

    let facts_before = fact_fingerprint(&readonly_pool).await?;
    let (server_transport, client_transport) = tokio::io::duplex(128 * 1024);
    let server_handle = tokio::spawn(async move {
        ForgeOsMcpServer::new(kernel)
            .serve(server_transport)
            .await?
            .waiting()
            .await?;
        anyhow::Ok(())
    });
    let client = TestClient.serve(client_transport).await?;

    let tools = client.list_all_tools().await?;
    let mut names = tools
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "build_context_pack",
            "get_current_snapshot",
            "get_current_state",
            "get_development_history",
            "get_project_evidence_summary",
            "get_project_identity",
            "get_project_overview",
            "get_recent_git_activity",
            "get_recent_sessions",
            "get_recommended_next_steps",
            "list_evidence",
        ]
    );
    assert!(names.iter().all(|name| {
        ![
            "create", "update", "delete", "write", "accept", "execute", "shell", "commit", "push",
            "pull", "merge", "checkout", "reset", "stash",
        ]
        .iter()
        .any(|verb| name.contains(verb))
    }));

    let identity = call(
        &client,
        "get_project_identity",
        json!({"project_id": "project-a"}),
    )
    .await?;
    let overview = call(
        &client,
        "get_project_overview",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(identity.contains("CodeLens-Pro"));
    assert!(overview.contains("Fast code understanding"));
    assert!(overview.contains("implementation"));
    assert!(!overview.contains("ForgeOS secret vision"));

    let snapshot = call(
        &client,
        "get_current_snapshot",
        json!({"project_id": "project-a"}),
    )
    .await?;
    let current_state = call(
        &client,
        "get_current_state",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(snapshot.contains("Improve index performance"));
    assert!(current_state.contains("Large repositories are slow"));
    assert!(!current_state.contains("Ship D3"));

    let sessions = call(
        &client,
        "get_recent_sessions",
        json!({"project_id": "project-a"}),
    )
    .await?;
    let history = call(
        &client,
        "get_development_history",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(sessions.contains("Profile repository analysis"));
    assert!(history.contains("Continue cache implementation"));
    assert!(history.contains("session-a-open"));
    assert!(!history.contains("Implement MCP"));

    let legacy_evidence =
        call(&client, "list_evidence", json!({"project_id": "project-a"})).await?;
    assert!(legacy_evidence.contains("Benchmark report"));
    assert!(legacy_evidence.contains("git_commit"));
    assert!(!legacy_evidence.contains("Architecture decision"));
    assert_safe(&legacy_evidence);

    let evidence_summary = call(
        &client,
        "get_project_evidence_summary",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(evidence_summary.contains("Benchmark report"));
    assert!(evidence_summary.contains("supports"));
    assert!(evidence_summary.contains("snapshot"));
    assert!(!evidence_summary.contains("Architecture decision"));
    assert_safe(&evidence_summary);

    let git_activity = call(
        &client,
        "get_recent_git_activity",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(git_activity.contains(PROJECT_A_SHA));
    assert!(git_activity.contains("Optimize index cache"));
    assert!(git_activity.contains("CodeLens Developer"));
    assert!(git_activity.contains("Changed files: 2"));
    assert!(!git_activity.contains("ForgeOS secret commit"));
    assert_safe(&git_activity);

    let recommendations = call(
        &client,
        "get_recommended_next_steps",
        json!({"project_id": "project-a"}),
    )
    .await?;
    let recommendations_again = call(
        &client,
        "get_recommended_next_steps",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert_eq!(recommendations, recommendations_again);
    assert!(recommendations.contains("rule_based"));
    assert!(recommendations.contains("Resolve confirmed blocker"));
    assert!(recommendations.contains("Continue cache implementation"));
    assert!(recommendations.contains("Add cache"));
    assert!(recommendations.contains("Review cache invalidation risk"));
    assert!(recommendations.contains("Verify Evidence: Profiling notes"));
    assert!(recommendations.contains("Replace or reconfirm invalid Evidence"));
    assert!(!recommendations.contains("Accepted candidate must stay silent"));
    assert!(!recommendations.contains("ForgeOS pending secret"));
    assert_safe(&recommendations);

    let context = call(
        &client,
        "build_context_pack",
        json!({"project_id": "project-a", "template_type": "coding"}),
    )
    .await?;
    let context_again = call(
        &client,
        "build_context_pack",
        json!({"project_id": "project-a", "template_type": "coding"}),
    )
    .await?;
    assert_eq!(context, context_again);
    assert!(context.contains("## Project Overview"));
    assert!(context.contains("## Current Snapshot"));
    assert!(context.contains("## Recent Development Session"));
    assert!(context.contains("## Evidence"));
    assert!(context.contains("## Recent Git Activity"));
    assert!(!context.contains("ForgeOS secret vision"));
    assert_safe(&context);

    let project_b_overview = call(
        &client,
        "get_project_overview",
        json!({"project_id": "project-b"}),
    )
    .await?;
    assert!(project_b_overview.contains("ForgeOS secret vision"));
    assert!(!project_b_overview.contains("Fast code understanding"));

    let unknown = call(
        &client,
        "get_project_overview",
        json!({"project_id": "missing-project"}),
    )
    .await?;
    assert!(unknown.contains("Project not found"));
    let missing_scope = call(&client, "get_current_state", json!({"project_id": ""})).await?;
    assert!(missing_scope.contains("project_id is required"));

    let facts_after = fact_fingerprint(&readonly_pool).await?;
    assert_eq!(
        facts_before, facts_after,
        "MCP queries must not mutate facts"
    );
    let write_attempt = sqlx::query("UPDATE projects SET name = 'mutated' WHERE id = 'project-a'")
        .execute(&readonly_pool)
        .await;
    assert!(
        write_attempt.is_err(),
        "SQLite connection must be read-only"
    );
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&readonly_pool)
        .await?;
    let foreign_key_violations: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&readonly_pool)
            .await?;
    assert_eq!(integrity, "ok");
    assert_eq!(foreign_key_violations, 0);

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

fn assert_safe(result: &str) {
    for secret in [
        PRIVATE_PATH,
        PRIVATE_EMAIL,
        "sk-test-secret",
        "secret-provider-token",
        "Credential",
    ] {
        assert!(!result.contains(secret), "MCP leaked sentinel: {secret}");
    }
}

async fn fact_fingerprint(pool: &SqlitePool) -> anyhow::Result<(i64, i64, i64, i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT \
         (SELECT COUNT(*) FROM projects), \
         (SELECT COALESCE(SUM(version), 0) FROM project_snapshots), \
         (SELECT COALESCE(SUM(version), 0) FROM development_sessions), \
         (SELECT COALESCE(SUM(version), 0) FROM evidences), \
         (SELECT COALESCE(SUM(version), 0) FROM ai_candidates), \
         (SELECT COUNT(*) FROM context_exports)",
    )
    .fetch_one(pool)
    .await?)
}

async fn call(
    client: &rmcp::service::RunningService<rmcp::RoleClient, TestClient>,
    name: &str,
    arguments: Value,
) -> anyhow::Result<String> {
    let arguments = arguments
        .as_object()
        .cloned()
        .expect("test arguments must be an object");
    let result = client
        .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
        .await?;
    Ok(serde_json::to_string(&result)?)
}

async fn seed(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql(
        r#"
        INSERT INTO workspaces (id, name, timezone, created_at, updated_at)
        VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z');

        INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version)
        VALUES
          ('project-a', 'workspace-1', 'codelens-pro', 'CodeLens-Pro', 'Repository analysis', 'Fast code understanding', 'Independent developers', 'Local analysis', 'No issue tracking', 'Rust', 'active', 'implementation', 90, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1),
          ('project-b', 'workspace-1', 'forgeos', 'ForgeOS', 'Engineering memory', 'ForgeOS secret vision', 'Independent developers', 'Local-first', 'No task manager', 'Tauri + Rust', 'active', 'implementation', 100, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1);

        INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at)
        VALUES
          ('snapshot-a', 'project-a', 1, 'Improve index performance', 'Profiling complete', '["Baseline measured"]', '["Large repositories are slow"]', '["Add cache"]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z'),
          ('snapshot-b', 'project-b', 1, 'Ship D3', 'MCP design frozen', '[]', '[]', '["Implement MCP"]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z');

        INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version)
        VALUES
          ('session-a', 'project-a', 'completed', 'Profile repository analysis', '', 'Found index bottleneck', 'Benchmark reproduced', 'Start cache design', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2),
          ('session-a-open', 'project-a', 'open', 'Continue cache implementation', 'Cache key drafted', '', '', '', '2026-07-12T06:00:00Z', NULL, '2026-07-12T06:00:00Z', '2026-07-12T06:00:00Z', 1),
          ('session-b', 'project-b', 'completed', 'Implement MCP', '', 'Designed read-only server', 'cargo test', 'Add tools', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2);

        INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version)
        VALUES
          ('evidence-a', 'project-a', 'Benchmark report', 'Index timing results', 'test_result', 'local_file', 'D:/private/CodeLens-Pro/benchmark.md', 'verified', '2026-07-12T04:00:00Z', '2026-07-12T04:00:00Z', 1),
          ('evidence-a-unverified', 'project-a', 'Profiling notes', 'Needs confirmation', 'note', 'manual_note', '', 'unverified', '2026-07-12T04:01:00Z', '2026-07-12T04:01:00Z', 1),
          ('evidence-a-invalid', 'project-a', 'Old benchmark', 'Superseded data', 'test_result', 'manual_note', '', 'invalid', '2026-07-12T04:02:00Z', '2026-07-12T04:02:00Z', 1),
          ('evidence-a-git', 'project-a', 'Git commit aaaaaaaaaaaa: Optimize index cache private-author@example.com D:/private/CodeLens-Pro/TOKEN-CREDENTIAL', 'placeholder', 'git_commit', 'local_file', 'D:/private/CodeLens-Pro/TOKEN-CREDENTIAL', 'verified', '2026-07-12T04:03:00Z', '2026-07-12T04:03:00Z', 1),
          ('evidence-a-status', 'project-a', 'Git status: main @ aaaaaaaaaaaa', 'placeholder', 'git_status', 'local_file', 'D:/private/CodeLens-Pro/TOKEN-CREDENTIAL', 'unverified', '2026-07-12T04:04:00Z', '2026-07-12T04:04:00Z', 1),
          ('evidence-b', 'project-b', 'Architecture decision', 'Read-only MCP', 'document', 'local_file', 'D:/private/ForgeOS/docs/architecture.md', 'verified', '2026-07-12T04:00:00Z', '2026-07-12T04:00:00Z', 1),
          ('evidence-b-git', 'project-b', 'Git commit bbbbbbbbbbbb: ForgeOS secret commit', 'placeholder', 'git_commit', 'local_file', 'D:/private/ForgeOS', 'verified', '2026-07-12T04:05:00Z', '2026-07-12T04:05:00Z', 1);

        INSERT INTO evidence_links (id, evidence_id, target_type, target_id, relation_type)
        VALUES
          ('link-a-project', 'evidence-a', 'project', 'project-a', 'supports'),
          ('link-a-snapshot', 'evidence-a-unverified', 'snapshot', 'snapshot-a', 'relates_to'),
          ('link-b-project', 'evidence-b', 'project', 'project-b', 'supports');

        INSERT INTO repositories (id, project_id, path, remote_url, default_branch, last_checked_at, version)
        VALUES
          ('repository-a', 'project-a', 'D:/private/CodeLens-Pro/TOKEN-CREDENTIAL', 'https://example.invalid/a.git', 'main', '2026-07-12T04:04:00Z', 1),
          ('repository-b', 'project-b', 'D:/private/ForgeOS', 'https://example.invalid/b.git', 'main', '2026-07-12T04:05:00Z', 1);

        INSERT INTO repository_evidence_links (evidence_id, repository_id, project_id, commit_sha, captured_at)
        VALUES
          ('evidence-a-git', 'repository-a', 'project-a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', '2026-07-12T04:03:00Z'),
          ('evidence-a-status', 'repository-a', 'project-a', NULL, '2026-07-12T04:04:00Z'),
          ('evidence-b-git', 'repository-b', 'project-b', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', '2026-07-12T04:05:00Z');

        INSERT INTO context_exports (id, project_id, template_type, snapshot_version, generated_from_session_ids, content_markdown, content_hash, status, created_at, version)
        VALUES
          ('context-a', 'project-a', 'coding', 1, '["session-a"]', '# Context A', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'generated', '2026-07-12T05:00:00Z', 1),
          ('context-b', 'project-b', 'coding', 1, '["session-b"]', '# Context B', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb', 'generated', '2026-07-12T05:00:00Z', 1);

        INSERT INTO ai_candidates (id, project_id, candidate_type, source_context_export_id, title, content_markdown, status, version, created_at, updated_at)
        VALUES
          ('candidate-a-pending', 'project-a', 'risk_finding', 'context-a', 'Review cache invalidation risk', 'Pending analysis', 'pending', 1, '2026-07-12T05:01:00Z', '2026-07-12T05:01:00Z'),
          ('candidate-a-accepted', 'project-a', 'general_note', 'context-a', 'Accepted candidate must stay silent', 'Already reviewed', 'accepted', 2, '2026-07-12T05:02:00Z', '2026-07-12T05:03:00Z'),
          ('candidate-b-pending', 'project-b', 'risk_finding', 'context-b', 'ForgeOS pending secret', 'Other Project', 'pending', 1, '2026-07-12T05:01:00Z', '2026-07-12T05:01:00Z');

        INSERT INTO ai_providers (id, name, base_url, model, enabled, created_at, updated_at, version)
        VALUES ('provider-1', 'Secret provider', 'https://secret-provider-token.example/v1', 'sk-test-secret', 1, '2026-07-12T05:00:00Z', '2026-07-12T05:00:00Z', 1);
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("UPDATE evidences SET description = ? WHERE id = 'evidence-a-git'")
        .bind(format!(
            "Commit:\n{PROJECT_A_SHA}\n\nMessage:\nOptimize index cache by {PRIVATE_EMAIL} at {PRIVATE_PATH}\n\nAuthor:\nCodeLens Developer <{PRIVATE_EMAIL}>\n\nDate:\n2026-07-12T04:03:00Z\n\nRepository:\n{PRIVATE_PATH}"
        ))
        .execute(pool)
        .await?;
    sqlx::query("UPDATE evidences SET description = ? WHERE id = 'evidence-a-status'")
        .bind(format!(
            "Branch:\nmain\n\nHEAD:\n{PROJECT_A_SHA}\n\nDirty:\ntrue\n\nDiff Summary:\nChanged files: 2\nStaged files: 0\nUnstaged files: 1\nUntracked files: 1\nInsertions: 8\nDeletions: 3\n\nChecked At:\n2026-07-12T04:04:00Z\n\nRepository:\n{PRIVATE_PATH}"
        ))
        .execute(pool)
        .await?;
    sqlx::query("UPDATE evidences SET description = ? WHERE id = 'evidence-b-git'")
        .bind("Commit:\nbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n\nMessage:\nForgeOS secret commit\n\nAuthor:\nForgeOS Developer <forgeos-private@example.com>\n\nDate:\n2026-07-12T04:05:00Z\n\nRepository:\nD:/private/ForgeOS")
        .execute(pool)
        .await?;
    Ok(())
}
