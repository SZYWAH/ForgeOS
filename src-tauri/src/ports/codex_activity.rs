use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexConnectorHealth {
    pub available: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexTurnActivity {
    pub thread_id: String,
    pub turn_id: String,
    pub source_kind: String,
    pub status: String,
    pub goal_hint: String,
    pub activity_summary: String,
    pub outcome_summary: String,
    pub validation_summary: String,
    pub handoff_summary: String,
    pub blockers: Vec<String>,
    pub source_updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexActivityBatch {
    pub threads_scanned: usize,
    pub turns_seen: usize,
    pub activities: Vec<CodexTurnActivity>,
    pub latest_thread_updated_at: Option<DateTime<Utc>>,
    pub partial_reason: Option<String>,
}

#[derive(Debug, Error)]
pub enum CodexActivitySourceError {
    #[error("Codex executable was not found")]
    NotFound,
    #[error("Codex App Server timed out")]
    Timeout,
    #[error("Codex App Server protocol error: {0}")]
    Protocol(String),
    #[error("Codex App Server process failed: {0}")]
    Process(String),
}

#[async_trait]
pub trait CodexActivitySource: Send + Sync {
    async fn health_check(
        &self,
        configured_executable: Option<&str>,
    ) -> Result<CodexConnectorHealth, CodexActivitySourceError>;

    async fn read_repository_activity(
        &self,
        configured_executable: Option<&str>,
        repository_path: &Path,
        updated_after: Option<DateTime<Utc>>,
        max_threads: usize,
        max_turns: usize,
        max_normalized_bytes: usize,
    ) -> Result<CodexActivityBatch, CodexActivitySourceError>;
}
