use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use thiserror::Error;

use crate::ports::{CodexMcpClient, CodexMcpClientError, CodexMcpServerConfig};

pub const FORGEOS_MCP_NAME: &str = "forgeos";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexMcpRegistrationState {
    CodexUnavailable,
    SidecarMissing,
    NotConfigured,
    Configured,
    Conflict,
    Invalid,
}

impl CodexMcpRegistrationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CodexUnavailable => "codex_unavailable",
            Self::SidecarMissing => "sidecar_missing",
            Self::NotConfigured => "not_configured",
            Self::Configured => "configured",
            Self::Conflict => "conflict",
            Self::Invalid => "invalid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexMcpRegistrationStatus {
    pub state: CodexMcpRegistrationState,
    pub codex_version: Option<String>,
    pub expected_command: Option<String>,
    pub configured_command: Option<String>,
    pub enabled: bool,
    pub can_configure: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigureCodexMcpResult {
    pub changed: bool,
    pub restart_required: bool,
    pub backup_created: bool,
}

#[derive(Debug, Error)]
pub enum CodexMcpError {
    #[error("Codex is not available")]
    CodexUnavailable,
    #[error("the packaged ForgeOS MCP executable is unavailable")]
    SidecarMissing,
    #[error("an MCP server named forgeos already points to a different command")]
    Conflict,
    #[error("the forgeos MCP entry is not owned by this ForgeOS installation")]
    NotOwned,
    #[error("Codex did not retain the expected ForgeOS MCP configuration")]
    VerificationFailed,
    #[error(transparent)]
    Client(#[from] CodexMcpClientError),
}

#[derive(Clone)]
pub struct CodexMcpKernel {
    client: Arc<dyn CodexMcpClient>,
    expected_sidecar: Option<PathBuf>,
}

impl CodexMcpKernel {
    pub fn new(client: Arc<dyn CodexMcpClient>, expected_sidecar: Option<PathBuf>) -> Self {
        Self {
            client,
            expected_sidecar,
        }
    }

    pub async fn status(&self) -> Result<CodexMcpRegistrationStatus, CodexMcpError> {
        let expected = self.valid_sidecar();
        let version = match self.client.version().await {
            Ok(version) => Some(version),
            Err(CodexMcpClientError::NotFound) => {
                return Ok(CodexMcpRegistrationStatus {
                    state: CodexMcpRegistrationState::CodexUnavailable,
                    codex_version: None,
                    expected_command: display_path(expected.as_deref()),
                    configured_command: None,
                    enabled: false,
                    can_configure: false,
                })
            }
            Err(error) => return Err(error.into()),
        };
        let Some(expected) = expected else {
            return Ok(CodexMcpRegistrationStatus {
                state: CodexMcpRegistrationState::SidecarMissing,
                codex_version: version,
                expected_command: self
                    .expected_sidecar
                    .as_deref()
                    .and_then(Path::to_str)
                    .map(str::to_owned),
                configured_command: None,
                enabled: false,
                can_configure: false,
            });
        };

        let configured = match self.client.get(FORGEOS_MCP_NAME).await {
            Ok(value) => value,
            Err(CodexMcpClientError::InvalidResponse(_)) => {
                return Ok(CodexMcpRegistrationStatus {
                    state: CodexMcpRegistrationState::Invalid,
                    codex_version: version,
                    expected_command: display_path(Some(&expected)),
                    configured_command: None,
                    enabled: false,
                    can_configure: false,
                })
            }
            Err(error) => return Err(error.into()),
        };
        let (state, configured_command, enabled) = match configured {
            None => (CodexMcpRegistrationState::NotConfigured, None, false),
            Some(value) if owns_config(&value, &expected) => (
                CodexMcpRegistrationState::Configured,
                Some(value.command),
                value.enabled,
            ),
            Some(value) => (
                CodexMcpRegistrationState::Conflict,
                Some(value.command),
                value.enabled,
            ),
        };
        Ok(CodexMcpRegistrationStatus {
            state,
            codex_version: version,
            expected_command: display_path(Some(&expected)),
            configured_command,
            enabled,
            can_configure: true,
        })
    }

    pub async fn configure(
        &self,
        replace_existing: bool,
    ) -> Result<ConfigureCodexMcpResult, CodexMcpError> {
        self.client.version().await.map_err(|error| match error {
            CodexMcpClientError::NotFound => CodexMcpError::CodexUnavailable,
            other => other.into(),
        })?;
        let expected = self.valid_sidecar().ok_or(CodexMcpError::SidecarMissing)?;
        let current = self.client.get(FORGEOS_MCP_NAME).await?;
        if current
            .as_ref()
            .is_some_and(|value| owns_config(value, &expected) && value.enabled)
        {
            return Ok(ConfigureCodexMcpResult {
                changed: false,
                restart_required: false,
                backup_created: false,
            });
        }
        if current
            .as_ref()
            .is_some_and(|value| !owns_config(value, &expected))
            && !replace_existing
        {
            return Err(CodexMcpError::Conflict);
        }

        let backup = self.client.backup_config().await?;
        if current.is_some() {
            if let Err(error) = self.client.remove(FORGEOS_MCP_NAME).await {
                self.rollback(backup.as_ref()).await;
                return Err(error.into());
            }
        }
        let expected_text = expected.to_string_lossy().into_owned();
        if let Err(error) = self
            .client
            .add_stdio(FORGEOS_MCP_NAME, &expected_text)
            .await
        {
            self.rollback(backup.as_ref()).await;
            return Err(error.into());
        }
        match self.client.get(FORGEOS_MCP_NAME).await {
            Ok(Some(value)) if owns_config(&value, &expected) && value.enabled => {
                Ok(ConfigureCodexMcpResult {
                    changed: true,
                    restart_required: true,
                    backup_created: backup.is_some(),
                })
            }
            _ => {
                self.rollback(backup.as_ref()).await;
                Err(CodexMcpError::VerificationFailed)
            }
        }
    }

    pub async fn remove_owned(&self) -> Result<ConfigureCodexMcpResult, CodexMcpError> {
        let expected = self.valid_sidecar().ok_or(CodexMcpError::SidecarMissing)?;
        let Some(current) = self.client.get(FORGEOS_MCP_NAME).await? else {
            return Ok(ConfigureCodexMcpResult {
                changed: false,
                restart_required: false,
                backup_created: false,
            });
        };
        if !owns_config(&current, &expected) {
            return Err(CodexMcpError::NotOwned);
        }
        let backup = self.client.backup_config().await?;
        if let Err(error) = self.client.remove(FORGEOS_MCP_NAME).await {
            self.rollback(backup.as_ref()).await;
            return Err(error.into());
        }
        match self.client.get(FORGEOS_MCP_NAME).await {
            Ok(None) => Ok(ConfigureCodexMcpResult {
                changed: true,
                restart_required: true,
                backup_created: backup.is_some(),
            }),
            _ => {
                self.rollback(backup.as_ref()).await;
                Err(CodexMcpError::VerificationFailed)
            }
        }
    }

    async fn rollback(&self, backup: Option<&PathBuf>) {
        if let Some(backup) = backup {
            let _ = self.client.restore_config(backup).await;
        } else {
            let _ = self.client.remove(FORGEOS_MCP_NAME).await;
        }
    }

    fn valid_sidecar(&self) -> Option<PathBuf> {
        self.expected_sidecar
            .as_ref()
            .filter(|path| path.is_file())
            .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.clone()))
    }
}

pub fn resolve_expected_sidecar() -> Option<PathBuf> {
    let path = if let Some(path) = std::env::var_os("FORGEOS_MCP_PATH")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        path
    } else {
        if cfg!(debug_assertions) {
            return None;
        }
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))?
            .join("forgeos-mcp.exe")
    };
    sidecar_diagnostic_passes(&path).then_some(path)
}

fn sidecar_diagnostic_passes(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut command = Command::new(path);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = String::new();
                return status.success()
                    && child
                        .stdout
                        .take()
                        .is_some_and(|mut pipe| pipe.read_to_string(&mut stdout).is_ok())
                    && stdout.trim() == format!("forgeos-mcp {}", env!("CARGO_PKG_VERSION"));
            }
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

fn owns_config(value: &CodexMcpServerConfig, expected: &Path) -> bool {
    value.transport_type.eq_ignore_ascii_case("stdio")
        && value.args.is_empty()
        && path_equals(Path::new(&value.command), expected)
}

fn path_equals(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    let left = left.to_string_lossy().replace('/', "\\");
    let right = right.to_string_lossy().replace('/', "\\");
    if cfg!(windows) {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn display_path(value: Option<&Path>) -> Option<String> {
    value.and_then(Path::to_str).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;

    #[derive(Default)]
    struct FakeState {
        config: Option<CodexMcpServerConfig>,
        backup: Option<CodexMcpServerConfig>,
        fail_verification: bool,
    }

    struct FakeClient(Arc<Mutex<FakeState>>);

    #[async_trait]
    impl CodexMcpClient for FakeClient {
        async fn version(&self) -> Result<String, CodexMcpClientError> {
            Ok("codex-cli test".into())
        }

        async fn get(
            &self,
            _name: &str,
        ) -> Result<Option<CodexMcpServerConfig>, CodexMcpClientError> {
            let state = self.0.lock().unwrap();
            if state.fail_verification && state.backup.is_some() {
                return Ok(None);
            }
            Ok(state.config.clone())
        }

        async fn add_stdio(&self, _name: &str, command: &str) -> Result<(), CodexMcpClientError> {
            self.0.lock().unwrap().config = Some(CodexMcpServerConfig {
                command: command.into(),
                args: Vec::new(),
                enabled: true,
                transport_type: "stdio".into(),
            });
            Ok(())
        }

        async fn remove(&self, _name: &str) -> Result<(), CodexMcpClientError> {
            self.0.lock().unwrap().config = None;
            Ok(())
        }

        async fn backup_config(&self) -> Result<Option<PathBuf>, CodexMcpClientError> {
            let mut state = self.0.lock().unwrap();
            state.backup = state.config.clone();
            Ok(Some(PathBuf::from("backup")))
        }

        async fn restore_config(&self, _backup: &PathBuf) -> Result<(), CodexMcpClientError> {
            let mut state = self.0.lock().unwrap();
            state.config = state.backup.clone();
            state.fail_verification = false;
            Ok(())
        }
    }

    fn make_kernel(
        config: Option<CodexMcpServerConfig>,
    ) -> (CodexMcpKernel, tempfile::TempDir, Arc<Mutex<FakeState>>) {
        let directory = tempfile::tempdir().unwrap();
        let sidecar = directory.path().join("forgeos-mcp.exe");
        fs::write(&sidecar, b"mcp").unwrap();
        let state = Arc::new(Mutex::new(FakeState {
            config,
            ..Default::default()
        }));
        (
            CodexMcpKernel::new(Arc::new(FakeClient(Arc::clone(&state))), Some(sidecar)),
            directory,
            state,
        )
    }

    #[tokio::test]
    async fn configure_is_idempotent_and_reports_conflicts() {
        let (kernel, _directory, _state) = make_kernel(None);
        let first = kernel.configure(false).await.unwrap();
        assert!(first.changed);
        assert!(first.restart_required);
        let second = kernel.configure(false).await.unwrap();
        assert!(!second.changed);

        let conflict = CodexMcpServerConfig {
            command: "C:\\Elsewhere\\server.exe".into(),
            args: Vec::new(),
            enabled: true,
            transport_type: "stdio".into(),
        };
        let (kernel, _directory, _state) = make_kernel(Some(conflict));
        assert!(matches!(
            kernel.configure(false).await,
            Err(CodexMcpError::Conflict)
        ));
        assert!(kernel.configure(true).await.unwrap().changed);
    }

    #[tokio::test]
    async fn removal_preserves_unowned_entries() {
        let conflict = CodexMcpServerConfig {
            command: "C:\\Elsewhere\\server.exe".into(),
            args: Vec::new(),
            enabled: true,
            transport_type: "stdio".into(),
        };
        let (kernel, _directory, _state) = make_kernel(Some(conflict));
        assert!(matches!(
            kernel.remove_owned().await,
            Err(CodexMcpError::NotOwned)
        ));
    }

    #[tokio::test]
    async fn failed_verification_restores_the_previous_configuration() {
        let previous = CodexMcpServerConfig {
            command: "C:\\Elsewhere\\server.exe".into(),
            args: Vec::new(),
            enabled: true,
            transport_type: "stdio".into(),
        };
        let (kernel, _directory, state) = make_kernel(Some(previous.clone()));
        state.lock().unwrap().fail_verification = true;

        assert!(matches!(
            kernel.configure(true).await,
            Err(CodexMcpError::VerificationFailed)
        ));
        assert_eq!(state.lock().unwrap().config, Some(previous));
    }
}
