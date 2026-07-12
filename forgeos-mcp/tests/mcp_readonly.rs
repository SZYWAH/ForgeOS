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

#[derive(Debug, Clone, Default)]
struct TestClient;

impl ClientHandler for TestClient {
    fn get_info(&self) -> ClientInfo {
        ClientInfo::default()
    }
}

#[tokio::test]
async fn stdio_protocol_lists_only_read_tools_and_enforces_project_isolation() -> anyhow::Result<()>
{
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

    let (server_transport, client_transport) = tokio::io::duplex(64 * 1024);
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
            "get_project_identity",
            "get_recent_sessions",
            "list_evidence",
        ]
    );
    assert!(names.iter().all(|name| {
        !["create", "update", "delete", "write", "accept"]
            .iter()
            .any(|verb| name.contains(verb))
    }));

    let identity = call(
        &client,
        "get_project_identity",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(identity.contains("CodeLens-Pro"));
    assert!(!identity.contains("ForgeOS secret vision"));

    let snapshot = call(
        &client,
        "get_current_snapshot",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(snapshot.contains("Improve index performance"));
    assert!(!snapshot.contains("Ship D3"));

    let sessions = call(
        &client,
        "get_recent_sessions",
        json!({"project_id": "project-a"}),
    )
    .await?;
    assert!(sessions.contains("Profile repository analysis"));
    assert!(!sessions.contains("Implement MCP"));

    let evidence = call(&client, "list_evidence", json!({"project_id": "project-a"})).await?;
    assert!(evidence.contains("Benchmark report"));
    assert!(evidence.contains("git_commit"));
    assert!(!evidence.contains("Architecture decision"));
    assert!(!evidence.contains("D:/private/CodeLens-Pro"));

    let context = call(
        &client,
        "build_context_pack",
        json!({"project_id": "project-a", "template_type": "coding"}),
    )
    .await?;
    assert!(context.contains("CodeLens-Pro"));
    assert!(context.contains("Improve index performance"));
    assert!(!context.contains("ForgeOS secret vision"));
    assert!(!context.contains("sk-test-secret"));
    assert!(!context.contains("D:/private/CodeLens-Pro"));

    let unknown = call(
        &client,
        "get_project_identity",
        json!({"project_id": "missing-project"}),
    )
    .await?;
    assert!(unknown.contains("Project not found"));

    let export_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM context_exports")
        .fetch_one(&readonly_pool)
        .await?;
    assert_eq!(
        export_count, 0,
        "MCP Context Pack must not persist an export"
    );
    let write_attempt = sqlx::query("UPDATE projects SET name = 'mutated' WHERE id = 'project-a'")
        .execute(&readonly_pool)
        .await;
    assert!(
        write_attempt.is_err(),
        "SQLite connection must be read-only"
    );

    client.cancel().await?;
    server_handle.await??;
    Ok(())
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
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')")
        .execute(pool).await?;
    sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES ('project-a', 'workspace-1', 'codelens-pro', 'CodeLens-Pro', 'Repository analysis', 'Fast code understanding', 'Independent developers', 'Local analysis', 'No issue tracking', 'Rust', 'active', 'implementation', 90, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1), ('project-b', 'workspace-1', 'forgeos', 'ForgeOS', 'Engineering memory', 'ForgeOS secret vision', 'Independent developers', 'Local-first', 'No task manager', 'Tauri + Rust', 'active', 'implementation', 100, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1)")
        .execute(pool).await?;
    sqlx::query("INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at) VALUES ('snapshot-a', 'project-a', 1, 'Improve index performance', 'Profiling complete', '[\"Baseline measured\"]', '[\"Large repositories are slow\"]', '[\"Add cache\"]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z'), ('snapshot-b', 'project-b', 1, 'Ship D3', 'MCP design frozen', '[]', '[]', '[\"Implement MCP\"]', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z')")
        .execute(pool).await?;
    sqlx::query("INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version) VALUES ('session-a', 'project-a', 'completed', 'Profile repository analysis', '', 'Found index bottleneck', 'Benchmark reproduced', 'Start cache design', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2), ('session-b', 'project-b', 'completed', 'Implement MCP', '', 'Designed read-only server', 'cargo test', 'Add tools', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', '2026-07-12T02:00:00Z', '2026-07-12T03:00:00Z', 2)")
        .execute(pool).await?;
    sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES ('evidence-a', 'project-a', 'Benchmark report', 'Index timing results', 'test_result', 'local_file', 'D:/private/CodeLens-Pro/benchmark.md', 'verified', '2026-07-12T04:00:00Z', '2026-07-12T04:00:00Z', 1), ('evidence-a-git', 'project-a', 'Commit abc123', 'Commit: abc123', 'git_commit', 'local_file', 'D:/private/CodeLens-Pro', 'verified', '2026-07-12T04:00:00Z', '2026-07-12T04:00:00Z', 1), ('evidence-b', 'project-b', 'Architecture decision', 'Read-only MCP', 'document', 'local_file', 'D:/private/ForgeOS/docs/architecture.md', 'verified', '2026-07-12T04:00:00Z', '2026-07-12T04:00:00Z', 1)")
        .execute(pool).await?;
    sqlx::query("INSERT INTO ai_providers (id, name, base_url, model, enabled, created_at, updated_at, version) VALUES ('provider-1', 'Secret provider', 'https://secret.example/v1', 'secret-model', 1, '2026-07-12T05:00:00Z', '2026-07-12T05:00:00Z', 1)")
        .execute(pool).await?;
    Ok(())
}
