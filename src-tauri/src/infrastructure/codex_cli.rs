use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use tokio::{process::Command, time::timeout};

use crate::ports::{
    redact_external_text, CodexMcpClient, CodexMcpClientError, CodexMcpServerConfig,
};

use super::codex_locator::{discover_codex_executable, hide_tokio_window};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const BACKUP_LIMIT: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct CodexCliMcpClient {
    configured_executable: Option<PathBuf>,
}

impl CodexCliMcpClient {
    pub fn new(configured_executable: Option<PathBuf>) -> Self {
        Self {
            configured_executable,
        }
    }

    async fn output(
        &self,
        arguments: &[&str],
    ) -> Result<std::process::Output, CodexMcpClientError> {
        let executable =
            discover_codex_executable(self.configured_executable.as_deref().and_then(Path::to_str))
                .ok_or(CodexMcpClientError::NotFound)?;
        let mut command = Command::new(executable);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_tokio_window(&mut command);
        timeout(COMMAND_TIMEOUT, command.output())
            .await
            .map_err(|_| CodexMcpClientError::Process("command timed out".into()))?
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    CodexMcpClientError::NotFound
                } else {
                    CodexMcpClientError::Process(redact_external_text(&error.to_string()))
                }
            })
    }

    fn config_path() -> Result<PathBuf, CodexMcpClientError> {
        if let Some(home) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(home).join("config.toml"));
        }
        env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .map(PathBuf::from)
            .map(|home| home.join(".codex").join("config.toml"))
            .ok_or_else(|| {
                CodexMcpClientError::Backup("Codex home directory is unavailable".into())
            })
    }

    fn failure(output: &std::process::Output) -> String {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let value = if stderr.trim().is_empty() {
            stdout
        } else {
            stderr
        };
        redact_external_text(value.trim())
            .chars()
            .take(1_000)
            .collect()
    }

    fn rotate_backups(directory: &Path) -> Result<(), CodexMcpClientError> {
        let mut backups = fs::read_dir(directory)
            .map_err(|error| CodexMcpClientError::Backup(error.to_string()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("config.toml.forgeos-") && name.ends_with(".bak")
                    })
            })
            .collect::<Vec<_>>();
        // Timestamped names sort in creation order and remain deterministic even
        // on filesystems whose modified-time resolution is too coarse.
        backups.sort();
        let remove_count = backups.len().saturating_sub(BACKUP_LIMIT);
        for path in backups.into_iter().take(remove_count) {
            fs::remove_file(path)
                .map_err(|error| CodexMcpClientError::Backup(error.to_string()))?;
        }
        Ok(())
    }
}

#[async_trait]
impl CodexMcpClient for CodexCliMcpClient {
    async fn version(&self) -> Result<String, CodexMcpClientError> {
        let output = self.output(&["--version"]).await?;
        if !output.status.success() {
            return Err(CodexMcpClientError::Process(Self::failure(&output)));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .chars()
            .take(256)
            .collect())
    }

    async fn get(&self, name: &str) -> Result<Option<CodexMcpServerConfig>, CodexMcpClientError> {
        let output = self.output(&["mcp", "get", name, "--json"]).await?;
        if !output.status.success() {
            let failure = Self::failure(&output);
            if failure.contains("No MCP server named") {
                return Ok(None);
            }
            return Err(CodexMcpClientError::Process(failure));
        }
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| CodexMcpClientError::InvalidResponse(error.to_string()))?;
        let transport = value
            .get("transport")
            .ok_or_else(|| CodexMcpClientError::InvalidResponse("transport is missing".into()))?;
        let transport_type = transport
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CodexMcpClientError::InvalidResponse("transport type is missing".into())
            })?;
        let command = transport
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let args = transport
            .get("args")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        Ok(Some(CodexMcpServerConfig {
            command,
            args,
            enabled: value
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            transport_type: transport_type.to_owned(),
        }))
    }

    async fn add_stdio(&self, name: &str, command: &str) -> Result<(), CodexMcpClientError> {
        let output = self.output(&["mcp", "add", name, "--", command]).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(CodexMcpClientError::Process(Self::failure(&output)))
        }
    }

    async fn remove(&self, name: &str) -> Result<(), CodexMcpClientError> {
        let output = self.output(&["mcp", "remove", name]).await?;
        if output.status.success() {
            Ok(())
        } else {
            Err(CodexMcpClientError::Process(Self::failure(&output)))
        }
    }

    async fn backup_config(&self) -> Result<Option<PathBuf>, CodexMcpClientError> {
        let config = Self::config_path()?;
        if !config.is_file() {
            return Ok(None);
        }
        let directory = config
            .parent()
            .ok_or_else(|| {
                CodexMcpClientError::Backup("Codex config parent is unavailable".into())
            })?
            .join("backups");
        fs::create_dir_all(&directory)
            .map_err(|error| CodexMcpClientError::Backup(error.to_string()))?;
        let backup = directory.join(format!(
            "config.toml.forgeos-{}.bak",
            Utc::now().format("%Y%m%d%H%M%S%3f")
        ));
        fs::copy(&config, &backup)
            .map_err(|error| CodexMcpClientError::Backup(error.to_string()))?;
        Self::rotate_backups(&directory)?;
        Ok(Some(backup))
    }

    async fn restore_config(&self, backup: &PathBuf) -> Result<(), CodexMcpClientError> {
        let config = Self::config_path()?;
        fs::copy(backup, config)
            .map(|_| ())
            .map_err(|error| CodexMcpClientError::Backup(error.to_string()))
    }
}
