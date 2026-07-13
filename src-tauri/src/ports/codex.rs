use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexExecutionRequest {
    pub execution_id: String,
    pub proposal_id: String,
    pub project_id: String,
    pub context_export_id: String,
    pub title: String,
    pub goal: String,
    pub allowed_scope: Vec<String>,
    pub forbidden_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub validation_commands: Vec<String>,
}

#[derive(Debug, Error)]
pub enum CodexRunnerError {
    #[error("Codex runner rejected the execution request: {0}")]
    Rejected(String),
}

/// Boundary for a future Codex execution adapter.
///
/// D5 implementations may accept request metadata only. They must not launch
/// Codex, execute validation commands, or modify a working tree.
#[async_trait]
pub trait CodexRunner: Send + Sync {
    async fn accept(&self, request: &CodexExecutionRequest) -> Result<(), CodexRunnerError>;
}
