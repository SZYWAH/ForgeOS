use serde::{Deserialize, Serialize};
use tauri::State;

use crate::application::{CodexMcpKernel, CodexMcpRegistrationStatus, ConfigureCodexMcpResult};

use super::IpcError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexMcpRegistrationStatusDto {
    pub state: String,
    pub codex_version: Option<String>,
    pub expected_command: Option<String>,
    pub configured_command: Option<String>,
    pub enabled: bool,
    pub can_configure: bool,
}

impl From<CodexMcpRegistrationStatus> for CodexMcpRegistrationStatusDto {
    fn from(value: CodexMcpRegistrationStatus) -> Self {
        Self {
            state: value.state.as_str().into(),
            codex_version: value.codex_version,
            expected_command: value.expected_command,
            configured_command: value.configured_command,
            enabled: value.enabled,
            can_configure: value.can_configure,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureCodexMcpResultDto {
    pub changed: bool,
    pub restart_required: bool,
    pub backup_created: bool,
}

impl From<ConfigureCodexMcpResult> for ConfigureCodexMcpResultDto {
    fn from(value: ConfigureCodexMcpResult) -> Self {
        Self {
            changed: value.changed,
            restart_required: value.restart_required,
            backup_created: value.backup_created,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureCodexMcpRequest {
    pub replace_existing: bool,
}

#[tauri::command]
pub async fn get_codex_mcp_registration(
    kernel: State<'_, CodexMcpKernel>,
) -> Result<CodexMcpRegistrationStatusDto, IpcError> {
    kernel.status().await.map(Into::into).map_err(Into::into)
}

#[tauri::command]
pub async fn configure_codex_mcp(
    kernel: State<'_, CodexMcpKernel>,
    input: ConfigureCodexMcpRequest,
) -> Result<ConfigureCodexMcpResultDto, IpcError> {
    kernel
        .configure(input.replace_existing)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn remove_codex_mcp_registration(
    kernel: State<'_, CodexMcpKernel>,
) -> Result<ConfigureCodexMcpResultDto, IpcError> {
    kernel
        .remove_owned()
        .await
        .map(Into::into)
        .map_err(Into::into)
}
