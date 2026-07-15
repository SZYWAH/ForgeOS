use std::sync::Arc;

use forgeos_lib::{
    application::ProjectMemoryQueryKernel,
    infrastructure::{database, repositories::SqliteProjectRepository},
};
use forgeos_mcp::server::ForgeOsMcpServer;
use rmcp::{
    model::{CallToolRequestParams, ClientInfo},
    ClientHandler, ServiceExt,
};
use serde_json::{json, Value};
use sqlx::SqlitePool;

const PRIVATE_PATH: &str = "D:/private/CodeLens-Pro";

#[derive(Debug, Clone, Default)]
struct TestClient;
impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

#[tokio::test]
async fn stdio_memory_tools_are_isolated_safe_and_read_only() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let database_path = directory.path().join("forgeos.db");
    let writable = database::connect(&database_path).await?;
    seed(&writable.pool).await?;
    let cross_project_source = sqlx::query("INSERT INTO project_achievement_sources (achievement_id, project_id, source_type, source_id, source_order) VALUES ('achievement-a', 'project-a', 'session', 'session-b', 0)")
        .execute(&writable.pool)
        .await;
    assert!(
        cross_project_source.is_err(),
        "achievement sources must stay inside one Project"
    );
    writable.pool.close().await;

    let readonly = database::connect_read_only(&database_path).await?;
    let readonly_pool = readonly.pool.clone();
    let repository = Arc::new(SqliteProjectRepository::new(readonly.pool));
    let kernel = Arc::new(ProjectMemoryQueryKernel::new(repository));
    let before = fact_fingerprint(&readonly_pool).await?;

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

    let mut names = client
        .list_all_tools()
        .await?
        .iter()
        .map(|tool| tool.name.to_string())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "build_ai_context",
            "get_project_achievements",
            "get_project_context",
            "get_project_history",
            "list_projects"
        ]
    );

    let projects = call(&client, "list_projects", json!({})).await?;
    assert!(projects.contains("CodeLens-Pro"));
    assert!(!projects.contains(PRIVATE_PATH));

    let context = call(
        &client,
        "get_project_context",
        json!({"repositoryPath": "d:\\PRIVATE\\codelens-pro\\src\\main.rs"}),
    )
    .await?;
    assert!(context.contains("Fast code understanding"));
    assert!(!context.contains("ForgeOS secret vision"));
    assert_safe(&context);

    let nested = call(
        &client,
        "get_project_context",
        json!({"repositoryPath": "D:/private/CodeLens-Pro/packages/forgeos/src"}),
    )
    .await?;
    assert!(nested.contains("ForgeOS secret vision"));
    assert!(!nested.contains("Fast code understanding"));

    let history = call(
        &client,
        "get_project_history",
        json!({"projectId": "project-a", "limit": 20}),
    )
    .await?;
    assert!(history.contains("Implement cache"));
    assert!(history.contains("Optimize index cache"));
    assert!(!history.contains("ForgeOS private task"));
    assert_safe(&history);

    let achievements = call(
        &client,
        "get_project_achievements",
        json!({"projectId": "project-a"}),
    )
    .await?;
    assert!(achievements.contains("Reduced indexing latency"));
    assert!(!achievements.contains("draft must stay hidden"));

    let ai_context = call(
        &client,
        "build_ai_context",
        json!({"repositoryPath": "D:/private/CodeLens-Pro/src"}),
    )
    .await?;
    let ai_context_again = call(
        &client,
        "build_ai_context",
        json!({"projectId": "project-a"}),
    )
    .await?;
    assert_eq!(
        extract_content_hash(&ai_context),
        extract_content_hash(&ai_context_again)
    );
    assert!(ai_context.contains("## Project Identity"));
    assert!(ai_context.contains("## Recent Development History"));
    assert!(ai_context.contains("## Confirmed Achievements"));
    assert_safe(&ai_context);

    let invalid = call(
        &client,
        "get_project_context",
        json!({"projectId": "project-a", "repositoryPath": PRIVATE_PATH}),
    )
    .await?;
    assert!(invalid.contains("exactly one"));
    let missing = call(
        &client,
        "get_project_context",
        json!({"repositoryPath": "D:/unregistered/repository"}),
    )
    .await?;
    assert!(missing.contains("not connected"));

    assert_eq!(before, fact_fingerprint(&readonly_pool).await?);
    assert!(sqlx::query("UPDATE projects SET name = 'mutated'")
        .execute(&readonly_pool)
        .await
        .is_err());
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&readonly_pool)
        .await?;
    let foreign_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
        .fetch_one(&readonly_pool)
        .await?;
    assert_eq!(integrity, "ok");
    assert_eq!(foreign_keys, 0);

    client.cancel().await?;
    server_handle.await??;
    Ok(())
}

fn assert_safe(result: &str) {
    for secret in [
        PRIVATE_PATH,
        "private-author@example.com",
        "sk-test-secret",
        "Credential",
    ] {
        assert!(!result.contains(secret), "MCP leaked sentinel: {secret}");
    }
}

fn extract_content_hash(value: &str) -> String {
    let value: Value = serde_json::from_str(value).expect("tool response JSON");
    value
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|body| {
            body.get("content_hash")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

async fn fact_fingerprint(pool: &SqlitePool) -> anyhow::Result<(i64, i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT COUNT(*) FROM projects), (SELECT COUNT(*) FROM development_sessions), (SELECT COUNT(*) FROM repository_commit_observations), (SELECT COUNT(*) FROM project_achievements)").fetch_one(pool).await?)
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
    sqlx::raw_sql(r#"
      INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z');
      INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES
        ('project-a', 'workspace-1', 'codelens', 'CodeLens-Pro', 'Repository analysis', 'Fast code understanding', 'Developers', 'Local analysis', 'No issue tracking', 'Rust', 'active', 'implementation', 90, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1),
        ('project-b', 'workspace-1', 'forgeos', 'ForgeOS', 'Engineering memory', 'ForgeOS secret vision', 'Developers', 'Local first', 'No task manager', 'Rust', 'active', 'implementation', 90, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1);
      INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at) VALUES
        ('snapshot-a', 'project-a', 1, 'Improve index performance', 'Profiling complete', '["Baseline measured"]', '["Large repositories are slow"]', '["Add cache"]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z'),
        ('snapshot-b', 'project-b', 1, 'Ship MCP', 'Private project', '[]', '[]', '[]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z');
      INSERT INTO repositories (id, project_id, path, remote_url, default_branch, last_checked_at, version) VALUES
        ('repository-a', 'project-a', 'D:/private/CodeLens-Pro', NULL, 'main', '2026-07-12T02:00:00Z', 1),
        ('repository-b', 'project-b', 'D:/private/CodeLens-Pro/packages/forgeos', NULL, 'main', '2026-07-12T02:00:00Z', 1);
      INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version) VALUES
        ('session-a', 'project-a', 'completed', 'Implement cache', 'Measured before change', 'Index cache complete', 'cargo test passed', 'Benchmark large repository', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2),
        ('session-b', 'project-b', 'completed', 'ForgeOS private task', '', 'Done', 'cargo test', 'Private handoff', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2);
      INSERT INTO repository_commit_observations (id, repository_id, project_id, commit_sha, message, author_name, authored_at, observed_at, source_hash) VALUES
        ('commit-a', 'repository-a', 'project-a', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 'Optimize index cache', 'CodeLens Developer', '2026-07-12T03:30:00Z', '2026-07-12T04:00:00Z', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');
      INSERT INTO project_achievements (id, project_id, category, title, problem, approach, result, validation, impact, technologies, resume_bullet, status, source_mode, featured, source_pulse_id, confirmed_at, created_at, updated_at, version) VALUES
        ('achievement-a', 'project-a', 'technical_highlight', 'Index cache', 'Slow indexing', 'Added scoped cache', 'Reduced indexing latency', 'cargo test passed', 'Faster analysis', '["Rust"]', 'Improved repository indexing with a scoped Rust cache.', 'confirmed', 'manual', 1, NULL, '2026-07-12T05:00:00Z', '2026-07-12T04:00:00Z', '2026-07-12T05:00:00Z', 2),
        ('achievement-draft', 'project-a', 'feature', 'draft must stay hidden', '', '', '', '', '', '[]', '', 'draft', 'manual', 0, NULL, NULL, '2026-07-12T05:00:00Z', '2026-07-12T05:00:00Z', 1);
    "#).execute(pool).await?;
    Ok(())
}
