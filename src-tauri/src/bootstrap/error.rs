use thiserror::Error;

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error("failed to resolve the ForgeOS data directory: {0}")]
    AppPath(#[from] tauri::Error),

    #[error("failed to prepare the ForgeOS data directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to initialize SQLite: {0}")]
    Database(#[from] crate::infrastructure::database::DatabaseInitError),
}
