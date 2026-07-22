use std::{path::Path, time::Duration};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

static MIGRATOR: Migrator = sqlx::migrate!("../database/migrations");

pub struct DatabaseState {
    pub pool: SqlitePool,
}

pub async fn connect(path: &Path) -> Result<DatabaseState, DatabaseInitError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;
    verify_core_schema(&pool).await?;
    verify_desktop_schema(&pool).await?;

    Ok(DatabaseState { pool })
}

/// Opens an existing ForgeOS database without migration or write capability.
///
/// This connection mode is reserved for read-only consumers such as the local
/// MCP server. It deliberately does not create files, change journal settings,
/// or execute migrations.
pub async fn connect_read_only(path: &Path) -> Result<DatabaseState, DatabaseInitError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;

    verify_core_schema(&pool).await?;
    Ok(DatabaseState { pool })
}

async fn verify_core_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name IN (\
            'workspaces', 'projects', 'project_snapshots', 'development_sessions', \
            'evidences', 'evidence_links', 'ai_providers', 'context_exports', 'ai_candidates', \
            'repositories', 'working_copies', 'repository_evidence_links', \
            'codex_task_proposals', 'codex_task_proposal_status_history', \
            'github_connectors', 'github_import_drafts', 'github_evidence_links', \
            'codex_execution_records', 'codex_execution_status_history', \
            'codex_connector_settings', 'project_activity_settings', \
            'project_activity_sync_runs', 'codex_sync_cursors', \
            'codex_activity_observations', 'project_pulses', 'project_pulse_sources', \
            'app_preferences', 'repository_commit_observations', \
            'repository_history_cursors', 'project_achievements', \
            'project_achievement_sources'\
         )",
    )
    .fetch_one(pool)
    .await?;

    if table_count != 31 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS foundation schema is incomplete".to_owned(),
        ));
    }

    let project_history_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('repository_commit_observations') \
         WHERE name IN ('repository_id', 'project_id', 'commit_sha', 'message', \
         'author_name', 'authored_at', 'source_hash')",
    )
    .fetch_one(pool)
    .await?;
    if project_history_columns != 7 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS repository history schema is incomplete".to_owned(),
        ));
    }

    let achievement_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('project_achievements') \
         WHERE name IN ('project_id', 'category', 'title', 'result', 'status', \
         'source_mode', 'featured', 'version')",
    )
    .fetch_one(pool)
    .await?;
    if achievement_columns != 8 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS achievement memory schema is incomplete".to_owned(),
        ));
    }

    let app_preferences_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('app_preferences') \
         WHERE name IN ('id', 'language_mode', 'show_technical_terms', 'version', \
         'created_at', 'updated_at')",
    )
    .fetch_one(pool)
    .await?;

    if app_preferences_columns != 6 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS application preferences schema is incomplete".to_owned(),
        ));
    }

    let project_kernel_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') \
         WHERE name IN ('vision', 'target_user', 'scope', 'non_goals', 'tech_stack', \
         'status', 'lifecycle_stage', 'priority', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if project_kernel_columns != 9 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Project Identity schema is incomplete".to_owned(),
        ));
    }

    let snapshot_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('project_snapshots') \
         WHERE name IN ('id', 'project_id', 'version', 'current_goal', 'status_summary', \
         'achievements', 'blockers', 'next_steps', 'confirmed_at', 'created_at', 'updated_at')",
    )
    .fetch_one(pool)
    .await?;

    if snapshot_columns != 11 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Current Snapshot schema is incomplete".to_owned(),
        ));
    }

    let session_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('development_sessions') \
         WHERE name IN ('id', 'project_id', 'status', 'goal', 'notes', 'outcome', \
         'validation', 'handoff', 'started_at', 'completed_at', 'created_at', \
         'updated_at', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if session_columns != 13 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Development Session schema is incomplete".to_owned(),
        ));
    }

    let evidence_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('evidences') \
         WHERE name IN ('id', 'project_id', 'title', 'description', 'type', 'source_type', \
         'source_uri', 'verification_status', 'created_at', 'updated_at', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if evidence_columns != 11 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Evidence schema is incomplete".to_owned(),
        ));
    }

    let evidence_link_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('evidence_links') \
         WHERE name IN ('id', 'evidence_id', 'target_type', 'target_id', 'relation_type')",
    )
    .fetch_one(pool)
    .await?;

    if evidence_link_columns != 5 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Evidence link schema is incomplete".to_owned(),
        ));
    }

    let ai_provider_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('ai_providers') \
         WHERE name IN ('id', 'name', 'base_url', 'model', 'enabled', 'created_at', \
         'updated_at', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if ai_provider_columns != 8 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS AI Provider schema is incomplete".to_owned(),
        ));
    }

    let context_export_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('context_exports') \
         WHERE name IN ('id', 'project_id', 'template_type', 'snapshot_version', \
         'generated_from_session_ids', 'content_markdown', 'content_hash', 'status', \
         'created_at', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if context_export_columns != 10 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Context Pack schema is incomplete".to_owned(),
        ));
    }

    let ai_candidate_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('ai_candidates') \
         WHERE name IN ('id', 'project_id', 'candidate_type', 'source_context_export_id', \
         'title', 'content_markdown', 'status', 'version', 'created_at', 'updated_at')",
    )
    .fetch_one(pool)
    .await?;

    if ai_candidate_columns != 10 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS AI Candidate schema is incomplete".to_owned(),
        ));
    }

    let proposal_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('codex_task_proposals') WHERE name IN \
         ('id', 'project_id', 'context_export_id', 'title', 'description', 'goal', 'non_goals', \
          'allowed_scope', 'forbidden_scope', 'acceptance_criteria', 'validation_commands', \
          'status', 'version', 'created_at', 'updated_at')",
    )
    .fetch_one(pool)
    .await?;
    if proposal_columns != 15 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Codex Task Proposal schema is incomplete".into(),
        ));
    }

    let proposal_history_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('codex_task_proposal_status_history') WHERE name IN \
         ('id', 'proposal_id', 'from_status', 'to_status', 'proposal_version', 'changed_at')",
    ).fetch_one(pool).await?;
    if proposal_history_columns != 6 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Codex Task Proposal history schema is incomplete".into(),
        ));
    }

    let execution_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('codex_execution_records') WHERE name IN \
         ('id', 'proposal_id', 'project_id', 'status', 'start_time', 'end_time', \
          'output_summary', 'validation_result', 'version', 'created_at', 'updated_at')",
    )
    .fetch_one(pool)
    .await?;
    if execution_columns != 11 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Codex Execution schema is incomplete".into(),
        ));
    }

    let execution_history_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('codex_execution_status_history') WHERE name IN \
         ('id', 'execution_id', 'from_status', 'to_status', 'execution_version', 'changed_at')",
    )
    .fetch_one(pool)
    .await?;
    if execution_history_columns != 6 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Codex Execution history schema is incomplete".into(),
        ));
    }

    let repository_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('repositories') \
         WHERE name IN ('id', 'project_id', 'path', 'remote_url', 'default_branch', \
         'last_checked_at', 'version')",
    )
    .fetch_one(pool)
    .await?;
    if repository_columns != 7 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Repository schema is incomplete".to_owned(),
        ));
    }

    let working_copy_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('working_copies') \
         WHERE name IN ('id', 'repository_id', 'branch', 'head_commit', 'is_dirty', \
         'changed_files', 'staged_files', 'unstaged_files', 'untracked_files', \
         'insertions', 'deletions', 'upstream_branch', 'ahead_count', 'behind_count', \
         'checked_at')",
    )
    .fetch_one(pool)
    .await?;
    if working_copy_columns != 15 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Working Copy schema is incomplete".to_owned(),
        ));
    }

    let repository_evidence_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('repository_evidence_links') \
         WHERE name IN ('evidence_id', 'repository_id', 'project_id', 'commit_sha', \
         'captured_at')",
    )
    .fetch_one(pool)
    .await?;
    if repository_evidence_columns != 5 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Repository Evidence schema is incomplete".to_owned(),
        ));
    }

    let github_connector_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('github_connectors') WHERE name IN \
         ('id', 'provider', 'auth_type', 'created_at', 'updated_at', 'version')",
    )
    .fetch_one(pool)
    .await?;
    if github_connector_columns != 6 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS GitHub Connector schema is incomplete".into(),
        ));
    }

    let github_draft_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('github_import_drafts') WHERE name IN \
         ('id', 'connector_id', 'github_repository_id', 'owner', 'name', 'url', 'description', \
          'homepage', 'default_branch', 'language', 'languages', 'topics', 'readme_summary', \
          'recent_commits', 'latest_release', 'status', 'imported_project_id', 'created_at', \
          'updated_at', 'version')",
    )
    .fetch_one(pool)
    .await?;
    if github_draft_columns != 20 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS GitHub Import Draft schema is incomplete".into(),
        ));
    }

    let github_evidence_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('github_evidence_links') WHERE name IN \
         ('evidence_id', 'draft_id', 'project_id', 'github_object_type', 'github_object_id', 'captured_at')",
    ).fetch_one(pool).await?;
    if github_evidence_columns != 6 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS GitHub Evidence schema is incomplete".into(),
        ));
    }

    let activity_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('codex_activity_observations') WHERE name IN \
         ('id', 'project_id', 'repository_id', 'thread_id', 'turn_id', 'source_kind', \
          'turn_status', 'goal_hint', 'activity_summary', 'outcome_summary', \
          'validation_summary', 'handoff_summary', 'blockers', 'source_updated_at', \
          'source_hash', 'observed_at')",
    )
    .fetch_one(pool)
    .await?;
    if activity_columns != 16 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Codex Activity schema is incomplete".into(),
        ));
    }

    let pulse_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('project_pulses') WHERE name IN \
         ('id', 'project_id', 'status', 'analysis_mode', 'ai_provider_id', 'ai_model', \
          'activity_type', 'analysis_revision', 'source_window_started_at', \
          'source_window_ended_at', 'selected_observation_count', \
          'available_observation_count', \
          'current_goal', 'status_summary', 'achievements', 'blockers', 'next_steps', \
          'validation_summary', 'handoff', 'git_summary', 'snapshot_basis_version', \
          'session_basis_id', 'session_basis_version', 'working_copy_id', 'content_hash', \
          'generated_at', 'updated_at', 'version', 'confirmed_snapshot_id', \
          'confirmed_session_id')",
    )
    .fetch_one(pool)
    .await?;
    if pulse_columns != 30 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Project Pulse schema is incomplete".into(),
        ));
    }

    Ok(())
}

async fn verify_desktop_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('github_analysis_runs', 'github_analysis_sources')",
    )
    .fetch_one(pool)
    .await?;
    if table_count != 2 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS desktop analysis schema is incomplete".to_owned(),
        ));
    }
    let run_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('github_analysis_runs') WHERE name IN ('workspace_id', 'github_repository_id', 'source_commit_sha', 'request_hash', 'status', 'readiness', 'report_json', 'version')",
    )
    .fetch_one(pool)
    .await?;
    if run_columns != 8 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS GitHub analysis run schema is incomplete".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseInitError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pulse_quality_migration_preserves_legacy_pulse_and_source_links() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");
        for migration in [
            include_str!("../../../database/migrations/0001_create_workspaces_and_projects.sql"),
            include_str!("../../../database/migrations/0002_project_identity_kernel.sql"),
            include_str!("../../../database/migrations/0003_current_snapshot.sql"),
            include_str!("../../../database/migrations/0004_development_session.sql"),
            include_str!("../../../database/migrations/0005_evidence_kernel.sql"),
            include_str!("../../../database/migrations/0006_ai_provider_foundation.sql"),
            include_str!("../../../database/migrations/0007_context_pack.sql"),
            include_str!("../../../database/migrations/0008_ai_candidate.sql"),
            include_str!("../../../database/migrations/0009_git_evidence.sql"),
            include_str!("../../../database/migrations/0010_codex_task_proposal.sql"),
            include_str!("../../../database/migrations/0011_github_readonly_import.sql"),
            include_str!("../../../database/migrations/0012_codex_execution_foundation.sql"),
            include_str!("../../../database/migrations/0013_codex_activity_intake.sql"),
            include_str!("../../../database/migrations/0014_project_pulse.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO project_pulses (id, project_id, status, analysis_mode, current_goal, status_summary, achievements, blockers, next_steps, validation_summary, handoff, git_summary, content_hash, generated_at, updated_at, version, confirmed_snapshot_id) VALUES ('pulse-old', 'project-old', 'confirmed', 'rules', 'legacy goal', 'legacy status', '[]', '[]', '[]', '', '', '', ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', 1, 'snapshot-old')")
            .bind("a".repeat(64))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO project_pulse_sources (pulse_id, project_id, observation_id, source_order) VALUES ('pulse-old', 'project-old', 'observation-old', 0)")
            .execute(&pool)
            .await
            .unwrap();

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0015_project_pulse_quality.sql"
        ))
        .execute(&pool)
        .await
        .expect("D6.1 migration should preserve legacy Pulse rows");

        let row: (String, String, String, i64) = sqlx::query_as(
            "SELECT analysis_revision, activity_type, confirmed_snapshot_id, selected_observation_count FROM project_pulses WHERE id = 'pulse-old'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            (
                "legacy-v1".into(),
                "unknown".into(),
                "snapshot-old".into(),
                0
            )
        );
        let sources: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM project_pulse_sources WHERE pulse_id = 'pulse-old'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sources, 1);
    }

    #[tokio::test]
    async fn migrations_create_the_git_evidence_schema() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");

        MIGRATOR
            .run(&pool)
            .await
            .expect("Context Pack migrations should run");

        verify_core_schema(&pool)
            .await
            .expect("schema should contain the Context Pack foundation");
        verify_desktop_schema(&pool)
            .await
            .expect("schema should contain the GitHub analysis foundation");

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read projects");

        assert_eq!(project_count, 0);

        let snapshot_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read project_snapshots");
        assert_eq!(snapshot_count, 0);

        let session_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM development_sessions")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read development_sessions");
        assert_eq!(session_count, 0);

        let evidence_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evidences")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read evidences");
        assert_eq!(evidence_count, 0);

        let link_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evidence_links")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read evidence_links");
        assert_eq!(link_count, 0);

        let provider_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_providers")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read ai_providers");
        assert_eq!(provider_count, 0);

        let export_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM context_exports")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read context_exports");
        assert_eq!(export_count, 0);

        let candidate_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_candidates")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read ai_candidates");
        assert_eq!(candidate_count, 0);

        let repository_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM repositories")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read repositories");
        assert_eq!(repository_count, 0);

        let working_copy_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM working_copies")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read working_copies");
        assert_eq!(working_copy_count, 0);

        let execution_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM codex_execution_records")
                .fetch_one(&pool)
                .await
                .expect("Rust should be able to read codex_execution_records");
        assert_eq!(execution_count, 0);
    }

    #[tokio::test]
    async fn github_analysis_migration_enforces_workspace_concurrency_and_safe_sources() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        MIGRATOR.run(&pool).await.expect("migrations");
        sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('w', 'Workspace', 'UTC', '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z')")
            .execute(&pool).await.unwrap();
        let insert = |id: &str, hash: String| {
            sqlx::query("INSERT INTO github_analysis_runs (id, workspace_id, github_repository_id, owner, name, repository_url, default_branch, source_commit_sha, focus_type, status, analysis_mode, readiness, analyzer_revision, request_hash, created_at, updated_at) VALUES (?, 'w', 'repo', 'owner', 'repo', 'https://github.com/owner/repo', 'main', '0123456789abcdef', 'overview', 'queued', 'rules', 'incomplete', 'github-rules-v1', ?, '2026-07-20T00:00:00Z', '2026-07-20T00:00:00Z')")
                .bind(id.to_owned()).bind(hash)
        };
        insert("run-1", "a".repeat(64))
            .execute(&pool)
            .await
            .expect("first active run");
        assert!(insert("run-2", "b".repeat(64))
            .execute(&pool)
            .await
            .is_err());
        sqlx::query("UPDATE github_analysis_runs SET status = 'failed' WHERE id = 'run-1'")
            .execute(&pool)
            .await
            .unwrap();
        insert("run-2", "b".repeat(64))
            .execute(&pool)
            .await
            .expect("terminal run releases active slot");
        let unsafe_source = sqlx::query("INSERT INTO github_analysis_sources (id, analysis_run_id, source_type, source_path, content_hash, excerpt, source_order) VALUES ('s', 'run-2', 'readme', 'C:/secret/README.md', ?, 'safe', 0)")
            .bind("c".repeat(64)).execute(&pool).await;
        assert!(unsafe_source.is_err());
        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(integrity, "ok");
        let foreign_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(foreign_keys, 0);
    }

    #[tokio::test]
    async fn git_evidence_migration_preserves_existing_evidence_and_links() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();

        for migration in [
            include_str!("../../../database/migrations/0001_create_workspaces_and_projects.sql"),
            include_str!("../../../database/migrations/0002_project_identity_kernel.sql"),
            include_str!("../../../database/migrations/0003_current_snapshot.sql"),
            include_str!("../../../database/migrations/0004_development_session.sql"),
            include_str!("../../../database/migrations/0005_evidence_kernel.sql"),
            include_str!("../../../database/migrations/0006_ai_provider_foundation.sql"),
            include_str!("../../../database/migrations/0007_context_pack.sql"),
            include_str!("../../../database/migrations/0008_ai_candidate.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace-1', 'Personal', 'UTC', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES ('project-1', 'workspace-1', 'forgeos', 'ForgeOS', '', '', '', '', '', '', 'active', 'implementation', 80, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES ('evidence-1', 'project-1', 'Architecture', 'Preserve me', 'document', 'local_file', 'docs/architecture.md', 'verified', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 2)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO evidence_links (id, evidence_id, target_type, target_id, relation_type) VALUES ('link-1', 'evidence-1', 'project', 'project-1', 'supports')")
            .execute(&pool).await.unwrap();

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0009_git_evidence.sql"
        ))
        .execute(&pool)
        .await
        .expect("D2 migration should preserve populated D1.4 tables");

        let evidence: (String, String, i64) = sqlx::query_as(
            "SELECT title, verification_status, version FROM evidences WHERE id = 'evidence-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(evidence, ("Architecture".into(), "verified".into(), 2));
        let link_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM evidence_links WHERE id = 'link-1' AND evidence_id = 'evidence-1'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(link_count, 1);
        let git_type = sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES ('git-1', 'project-1', 'Commit', '', 'git_commit', 'local_file', 'D:/repo', 'unverified', '2026-07-12T01:00:00Z', '2026-07-12T01:00:00Z', 1)")
            .execute(&pool).await;
        assert!(git_type.is_ok());
        let foreign_keys: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(foreign_keys, 0);
    }

    #[tokio::test]
    async fn github_migration_preserves_existing_manual_and_git_evidence() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        for migration in [
            include_str!("../../../database/migrations/0001_create_workspaces_and_projects.sql"),
            include_str!("../../../database/migrations/0002_project_identity_kernel.sql"),
            include_str!("../../../database/migrations/0003_current_snapshot.sql"),
            include_str!("../../../database/migrations/0004_development_session.sql"),
            include_str!("../../../database/migrations/0005_evidence_kernel.sql"),
            include_str!("../../../database/migrations/0006_ai_provider_foundation.sql"),
            include_str!("../../../database/migrations/0007_context_pack.sql"),
            include_str!("../../../database/migrations/0008_ai_candidate.sql"),
            include_str!("../../../database/migrations/0009_git_evidence.sql"),
            include_str!("../../../database/migrations/0010_codex_task_proposal.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql("INSERT INTO workspaces VALUES ('w', 'Personal', 'UTC', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z');
            INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES ('p', 'w', 'project', 'Project', '', '', '', '', '', '', 'active', 'implementation', 1, '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 1);
            INSERT INTO evidences VALUES ('e', 'p', 'Existing Git fact', '', 'git_status', 'local_file', 'D:/repo', 'verified', '2026-07-12T00:00:00Z', '2026-07-12T00:00:00Z', 2);
            INSERT INTO evidence_links VALUES ('el', 'e', 'project', 'p', 'supports');
            INSERT INTO repositories VALUES ('r', 'p', 'D:/repo', NULL, 'main', '2026-07-12T00:00:00Z', 1);
            INSERT INTO working_copies VALUES ('wc', 'r', 'main', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 0, 0, 0, 0, 0, 0, 0, '2026-07-12T00:00:00Z');
            INSERT INTO repository_evidence_links VALUES ('e', 'r', 'p', NULL, '2026-07-12T00:00:00Z');")
            .execute(&pool).await.unwrap();

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0011_github_readonly_import.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evidences WHERE id = 'e' AND verification_status = 'verified' AND version = 2").fetch_one(&pool).await.unwrap(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM evidence_links WHERE id = 'el'")
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
        assert_eq!(sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM repository_evidence_links WHERE evidence_id = 'e' AND repository_id = 'r'").fetch_one(&pool).await.unwrap(), 1);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn codex_execution_migration_preserves_approved_proposals_and_enforces_project_scope() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        for migration in [
            include_str!("../../../database/migrations/0001_create_workspaces_and_projects.sql"),
            include_str!("../../../database/migrations/0002_project_identity_kernel.sql"),
            include_str!("../../../database/migrations/0003_current_snapshot.sql"),
            include_str!("../../../database/migrations/0004_development_session.sql"),
            include_str!("../../../database/migrations/0005_evidence_kernel.sql"),
            include_str!("../../../database/migrations/0006_ai_provider_foundation.sql"),
            include_str!("../../../database/migrations/0007_context_pack.sql"),
            include_str!("../../../database/migrations/0008_ai_candidate.sql"),
            include_str!("../../../database/migrations/0009_git_evidence.sql"),
            include_str!("../../../database/migrations/0010_codex_task_proposal.sql"),
            include_str!("../../../database/migrations/0011_github_readonly_import.sql"),
        ] {
            sqlx::raw_sql(migration).execute(&pool).await.unwrap();
        }
        sqlx::raw_sql(
            "INSERT INTO workspaces VALUES ('w', 'Personal', 'UTC', '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');
             INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version)
             VALUES ('p1', 'w', 'one', 'One', '', '', '', '', '', '', 'active', 'implementation', 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z', 1),
                    ('p2', 'w', 'two', 'Two', '', '', '', '', '', '', 'active', 'implementation', 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z', 1);
             INSERT INTO context_exports VALUES ('ctx', 'p1', 'coding', NULL, '[]', '# context', '327f031b25e00b1a7cd9b0c18f05948b60f55d09f9b3d177d21083f83a3cb6df', 'generated', '2026-07-13T00:00:00Z', 1);
             INSERT INTO codex_task_proposals VALUES ('proposal', 'p1', 'ctx', 'D5', '', 'Record execution', '[]', '[]', '[]', '[\"Preserved\"]', '[]', 'approved', 3, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z');",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0012_codex_execution_foundation.sql"
        ))
        .execute(&pool)
        .await
        .unwrap();

        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status FROM codex_task_proposals WHERE id = 'proposal'"
            )
            .fetch_one(&pool)
            .await
            .unwrap(),
            "approved"
        );
        let cross_project = sqlx::query(
            "INSERT INTO codex_execution_records (id, proposal_id, project_id, status, version, created_at, updated_at) VALUES ('execution', 'proposal', 'p2', 'pending', 1, '2026-07-13T00:00:00Z', '2026-07-13T00:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(cross_project.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn d1_migration_preserves_d0_projects_and_backfills_defaults() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0001_create_workspaces_and_projects.sql"
        ))
        .execute(&pool)
        .await
        .expect("D0 schema should be created");

        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D0 workspace should be inserted");
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, created_at, updated_at) \
             VALUES ('project-1', 'workspace-1', 'legacy-project', 'Legacy Project', NULL, \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D0 project should be inserted");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0002_project_identity_kernel.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.1 migration should evolve the D0 schema");

        let migrated: (String, String, String, i32, i64) = sqlx::query_as(
            "SELECT description, status, lifecycle_stage, priority, version \
             FROM projects WHERE id = 'project-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("the D0 project should survive migration");

        assert_eq!(migrated.0, "");
        assert_eq!(migrated.1, "idea");
        assert_eq!(migrated.2, "discovery");
        assert_eq!(migrated.3, 50);
        assert_eq!(migrated.4, 1);

        let description_not_null: i64 = sqlx::query_scalar(
            "SELECT [notnull] FROM pragma_table_info('projects') WHERE name = 'description'",
        )
        .fetch_one(&pool)
        .await
        .expect("description metadata should be readable");
        assert_eq!(description_not_null, 1);

        let second_workspace = sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-2', 'Second', 'UTC', \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            second_workspace.is_err(),
            "SQLite must enforce the single Workspace invariant"
        );
    }

    #[tokio::test]
    async fn snapshot_migration_preserves_d1_projects_and_enforces_constraints() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("foreign keys should be enabled");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0001_create_workspaces_and_projects.sql"
        ))
        .execute(&pool)
        .await
        .expect("D0 schema should be created");
        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0002_project_identity_kernel.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.1 schema should be created");

        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D1.1 Workspace should be inserted");
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, vision, \
             target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
             created_at, updated_at, version) VALUES ('project-1', 'workspace-1', \
             'forgeos', 'ForgeOS', '', '', '', '', '', '', 'active', 'implementation', 100, \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z', 1)",
        )
        .execute(&pool)
        .await
        .expect("D1.1 Project should be inserted");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0003_current_snapshot.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.2 migration should run over D1.1 data");

        let project_name: String =
            sqlx::query_scalar("SELECT name FROM projects WHERE id = 'project-1'")
                .fetch_one(&pool)
                .await
                .expect("D1.1 Project should survive D1.2 migration");
        assert_eq!(project_name, "ForgeOS");
        let initial_snapshots: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(&pool)
            .await
            .expect("Snapshot table should be readable");
        assert_eq!(initial_snapshots, 0);

        sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) VALUES ('snapshot-1', 'project-1', 1, 'Ship D1.2', 'Ready', \
             '[\"Migration complete\"]', '[]', '[\"Verify\"]', '2026-07-10T01:00:00Z', \
             '2026-07-10T01:00:00Z', '2026-07-10T01:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("valid Snapshot should be inserted");

        let duplicate_version = sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) VALUES ('snapshot-duplicate', 'project-1', 1, 'Duplicate', 'Reject', \
             '[]', '[]', '[]', '2026-07-10T01:00:00Z', '2026-07-10T01:00:00Z', \
             '2026-07-10T01:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            duplicate_version.is_err(),
            "version identity must be unique"
        );

        let invalid_json = sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) VALUES ('snapshot-json', 'project-1', 2, 'Invalid JSON', 'Reject', \
             'not-json', '[]', '[]', '2026-07-10T01:00:00Z', '2026-07-10T01:00:00Z', \
             '2026-07-10T01:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            invalid_json.is_err(),
            "list columns must contain JSON arrays"
        );

        let orphan = sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) VALUES ('snapshot-orphan', 'missing-project', 1, 'Orphan', 'Reject', \
             '[]', '[]', '[]', '2026-07-10T01:00:00Z', '2026-07-10T01:00:00Z', \
             '2026-07-10T01:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(orphan.is_err(), "foreign key must reject orphan Snapshots");

        let foreign_key_violations: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
                .fetch_one(&pool)
                .await
                .expect("foreign_key_check should run");
        assert_eq!(foreign_key_violations, 0);
    }

    #[tokio::test]
    async fn session_migration_preserves_prior_data_and_enforces_lifecycle_constraints() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .expect("foreign keys should be enabled");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0001_create_workspaces_and_projects.sql"
        ))
        .execute(&pool)
        .await
        .expect("D0 schema should be created");
        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0002_project_identity_kernel.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.1 schema should be created");
        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0003_current_snapshot.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.2 schema should be created");

        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', \
             '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("Workspace should be inserted");
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, vision, \
             target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
             created_at, updated_at, version) VALUES ('project-1', 'workspace-1', \
             'forgeos', 'ForgeOS', '', '', '', '', '', '', 'active', 'implementation', 100, \
             '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z', 1)",
        )
        .execute(&pool)
        .await
        .expect("Project should be inserted");
        sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) VALUES ('snapshot-1', 'project-1', 1, 'Ship D1.3', 'Ready', \
             '[]', '[]', '[]', '2026-07-11T00:00:00Z', '2026-07-11T00:00:00Z', \
             '2026-07-11T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D1.2 Snapshot should be inserted");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0004_development_session.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.3 migration should run over prior data");

        let snapshot_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(&pool)
            .await
            .expect("prior Snapshot should remain readable");
        assert_eq!(snapshot_count, 1);

        sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, \
             validation, handoff, started_at, completed_at, created_at, updated_at, version) \
             VALUES ('session-open', 'project-1', 'open', 'Implement D1.3', '', '', '', '', \
             '2026-07-11T01:00:00Z', NULL, '2026-07-11T01:00:00Z', \
             '2026-07-11T01:00:00Z', 1)",
        )
        .execute(&pool)
        .await
        .expect("valid open Session should be inserted");

        let duplicate_open = sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, started_at, \
             created_at, updated_at) VALUES ('session-open-2', 'project-1', 'open', \
             'Race', '2026-07-11T01:01:00Z', '2026-07-11T01:01:00Z', \
             '2026-07-11T01:01:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            duplicate_open.is_err(),
            "partial unique index must reject a second open Session"
        );

        sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, outcome, \
             validation, handoff, started_at, completed_at, created_at, updated_at, version) \
             VALUES ('session-completed', 'project-1', 'completed', 'Prior work', 'Done', \
             'cargo test passed', 'Continue from the handoff', '2026-07-10T01:00:00Z', \
             '2026-07-10T02:00:00Z', \
             '2026-07-10T01:00:00Z', '2026-07-10T02:00:00Z', 2)",
        )
        .execute(&pool)
        .await
        .expect("terminal history can coexist with an open Session");

        let invalid_status = sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, started_at, \
             created_at, updated_at) VALUES ('session-status', 'project-1', 'paused', \
             'Invalid', '2026-07-11T02:00:00Z', '2026-07-11T02:00:00Z', \
             '2026-07-11T02:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            invalid_status.is_err(),
            "unsupported status must be rejected"
        );

        let invalid_lifecycle = sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, started_at, \
             completed_at, created_at, updated_at) VALUES ('session-lifecycle', 'project-1', \
             'open', 'Invalid', '2026-07-11T02:00:00Z', '2026-07-11T03:00:00Z', \
             '2026-07-11T02:00:00Z', '2026-07-11T03:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            invalid_lifecycle.is_err(),
            "open Session cannot carry a completion timestamp"
        );

        let orphan = sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, started_at, \
             created_at, updated_at) VALUES ('session-orphan', 'missing-project', 'open', \
             'Orphan', '2026-07-11T02:00:00Z', '2026-07-11T02:00:00Z', \
             '2026-07-11T02:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(orphan.is_err(), "foreign key must reject orphan Sessions");

        let index_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' \
             AND name IN ('development_sessions_one_open_per_project_index', \
             'development_sessions_project_started_index')",
        )
        .fetch_one(&pool)
        .await
        .expect("Session indexes should be inspectable");
        assert_eq!(index_count, 2);

        let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
            .fetch_one(&pool)
            .await
            .expect("integrity_check should run");
        assert_eq!(integrity, "ok");
        let foreign_key_violations: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pragma_foreign_key_check")
                .fetch_one(&pool)
                .await
                .expect("foreign_key_check should run");
        assert_eq!(foreign_key_violations, 0);
    }
}
