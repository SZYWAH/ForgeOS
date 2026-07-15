use thiserror::Error;

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("failed to resolve the ForgeOS data directory: {0}")]
    AppPath(#[from] tauri::Error),

    #[error("failed to prepare the ForgeOS data directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to initialize SQLite: {0}")]
    Database(#[from] crate::infrastructure::database::DatabaseInitError),

    #[error("failed to initialize the AI Provider HTTP client: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("failed to initialize the GitHub read-only client: {0}")]
    GitHubClient(String),

    #[error("failed to recover Project Pulse background state: {0}")]
    ProjectPulse(String),
}
