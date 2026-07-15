use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use chrono::Utc;
use forgeos_lib::{
    application::{
        ai_provider::AiProviderKernel,
        project_pulse::{ConfirmPulseInput, ProjectPulseKernel},
    },
    domain::{DiffSummary, PulseAnalysisRevision, SyncTrigger},
    infrastructure::{database, repositories::SqliteProjectRepository},
    ports::{
        AiClientError, AiCompletion, AiMessage, AiProviderClient, ApiKeyStore, CodexActivityBatch,
        CodexActivitySource, CodexActivitySourceError, CodexConnectorHealth, CodexTurnActivity,
        GitAdapter, GitAdapterError, GitCommit, GitInspection, SecretStoreError,
    },
};

#[derive(Default)]
struct FakeCodex;

#[async_trait]
impl CodexActivitySource for FakeCodex {
    async fn health_check(
        &self,
        _configured: Option<&str>,
    ) -> Result<CodexConnectorHealth, CodexActivitySourceError> {
        Ok(CodexConnectorHealth {
            available: true,
            executable_path: Some("codex".into()),
            version: Some("codex-cli test".into()),
            message: "ok".into(),
        })
    }
    async fn read_repository_activity(
        &self,
        _configured: Option<&str>,
        _path: &Path,
        _after: Option<chrono::DateTime<Utc>>,
        _threads: usize,
        _turns: usize,
        _bytes: usize,
    ) -> Result<CodexActivityBatch, CodexActivitySourceError> {
        let now = Utc::now();
        Ok(CodexActivityBatch {
            threads_scanned: 1,
            turns_seen: 1,
            latest_thread_updated_at: Some(now),
            partial_reason: None,
            activities: vec![CodexTurnActivity {
                thread_id: "thread-a".into(),
                turn_id: "turn-a".into(),
                source_kind: "cli".into(),
                status: "completed".into(),
                goal_hint: "Implement Project Pulse".into(),
                activity_summary: "Added the observation pipeline".into(),
                outcome_summary: "Project Pulse pipeline is ready".into(),
                validation_summary: "cargo: exit 0".into(),
                handoff_summary: "Review the generated Pulse".into(),
                blockers: vec![],
                source_updated_at: now,
            }],
        })
    }
}

#[derive(Default)]
struct FakeGit;

#[async_trait]
impl GitAdapter for FakeGit {
    async fn inspect(&self, path: &str) -> Result<GitInspection, GitAdapterError> {
        Ok(GitInspection {
            root_path: path.into(),
            remote_url: None,
            default_branch: "main".into(),
            branch: "main".into(),
            head_commit: "a".repeat(40),
            is_dirty: true,
            diff_summary: DiffSummary {
                changed_files: 2,
                unstaged_files: 2,
                ..Default::default()
            },
            upstream_branch: Some("origin/main".into()),
            ahead_count: Some(1),
            behind_count: Some(0),
            checked_at: Utc::now(),
            recent_commits: vec![],
        })
    }
    async fn get_commit(&self, _path: &str, _sha: &str) -> Result<GitCommit, GitAdapterError> {
        Err(GitAdapterError::InvalidOutput("unused".into()))
    }
}

#[derive(Default)]
struct FakeSecrets;
#[async_trait]
impl ApiKeyStore for FakeSecrets {
    async fn set(&self, _provider_id: &str, _api_key: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }
    async fn get(&self, _provider_id: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(None)
    }
    async fn clear(&self, _provider_id: &str) -> Result<(), SecretStoreError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeAi;
#[async_trait]
impl AiProviderClient for FakeAi {
    async fn complete(
        &self,
        _provider: &forgeos_lib::domain::AiProvider,
        _key: Option<String>,
        _messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        Err(AiClientError::Transport(
            "AI must not be called by default".into(),
        ))
    }
}

#[tokio::test]
async fn sync_is_idempotent_and_confirmation_is_the_only_formal_write() {
    let directory = tempfile::tempdir().unwrap();
    let data = database::connect(&directory.path().join("forgeos.db"))
        .await
        .unwrap();
    let repository_path = directory.path().display().to_string();
    sqlx::query("INSERT INTO workspaces (id, name, timezone, created_at, updated_at) VALUES ('workspace', 'Personal', 'Asia/Shanghai', ?, ?)").bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).execute(&data.pool).await.unwrap();
    sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES ('project-a', 'workspace', 'project-a', 'Project A', '', '', '', '', '', 'Rust', 'active', 'implementation', 50, ?, ?, 1)").bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).execute(&data.pool).await.unwrap();
    sqlx::query("INSERT INTO repositories (id, project_id, path, remote_url, default_branch, last_checked_at, version) VALUES ('repo-a', 'project-a', ?, NULL, 'main', ?, 1)").bind(&repository_path).bind(Utc::now().to_rfc3339()).execute(&data.pool).await.unwrap();
    sqlx::query("INSERT INTO working_copies (id, repository_id, branch, head_commit, is_dirty, checked_at) VALUES ('wc-initial', 'repo-a', 'main', ?, 0, ?)").bind("b".repeat(40)).bind(Utc::now().to_rfc3339()).execute(&data.pool).await.unwrap();

    let repository = Arc::new(SqliteProjectRepository::new(data.pool.clone()));
    let ai = Arc::new(AiProviderKernel::new(
        repository.clone(),
        Arc::new(FakeSecrets),
        Arc::new(FakeAi),
    ));
    let kernel = ProjectPulseKernel::new(repository, Arc::new(FakeCodex), Arc::new(FakeGit), ai);

    let run = kernel
        .queue_sync("project-a", SyncTrigger::Manual)
        .await
        .unwrap();
    let outcome = kernel.execute_sync(&run.id, |_| {}).await.unwrap();
    let pulse = outcome.pulse.expect("first sync creates a Pulse");
    assert_eq!(pulse.details.current_goal, "Implement Project Pulse");
    assert!(!pulse
        .details
        .blockers
        .iter()
        .any(|value| value.contains("未提交")));
    assert!(pulse
        .details
        .next_steps
        .iter()
        .any(|value| value.contains("未提交")));
    assert_eq!(
        pulse.quality.analysis_revision,
        PulseAnalysisRevision::RulesV2
    );
    assert_eq!(pulse.quality.selected_observation_count, 1);
    assert_eq!(pulse.quality.available_observation_count, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM development_sessions")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        0
    );

    let duplicate = kernel
        .queue_sync("project-a", SyncTrigger::Manual)
        .await
        .unwrap();
    let duplicate_outcome = kernel.execute_sync(&duplicate.id, |_| {}).await.unwrap();
    assert!(
        duplicate_outcome.pulse.is_none(),
        "unchanged sources reuse the existing Pulse"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM codex_activity_observations")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM project_pulses")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        1
    );

    let confirmed = kernel
        .confirm_pulse(
            "project-a",
            &pulse.id,
            ConfirmPulseInput {
                expected_version: pulse.version,
                update_snapshot: true,
                apply_session: true,
                complete_session: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(confirmed.status.as_str(), "confirmed");
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM project_snapshots")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM development_sessions WHERE status = 'open'"
        )
        .fetch_one(&data.pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("PRAGMA integrity_check")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM pragma_foreign_key_check")
            .fetch_one(&data.pool)
            .await
            .unwrap(),
        0
    );
}
