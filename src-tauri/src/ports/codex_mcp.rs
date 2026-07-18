use std::path::PathBuf;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexMcpServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub enabled: bool,
    pub transport_type: String,
}

#[derive(Debug, Error)]
pub enum CodexMcpClientError {
    #[error("Codex executable was not found")]
    NotFound,
    #[error("Codex returned an invalid MCP configuration: {0}")]
    InvalidResponse(String),
    #[error("Codex MCP configuration command failed: {0}")]
    Process(String),
    #[error("Codex configuration backup failed: {0}")]
    Backup(String),
}

#[async_trait]
pub trait CodexMcpClient: Send + Sync {
    async fn version(&self) -> Result<String, CodexMcpClientError>;

    async fn get(&self, name: &str) -> Result<Option<CodexMcpServerConfig>, CodexMcpClientError>;

    async fn add_stdio(&self, name: &str, command: &str) -> Result<(), CodexMcpClientError>;

    async fn remove(&self, name: &str) -> Result<(), CodexMcpClientError>;

    async fn backup_config(&self) -> Result<Option<PathBuf>, CodexMcpClientError>;

    async fn restore_config(&self, backup: &PathBuf) -> Result<(), CodexMcpClientError>;
}
