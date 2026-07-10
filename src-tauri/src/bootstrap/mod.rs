mod error;

use std::{env, fs, path::PathBuf};

use tauri::{AppHandle, Manager};

pub use error::BootstrapError;

use crate::infrastructure::database::{self, DatabaseState};

const DATA_DIR_ENV: &str = "FORGEOS_DATA_DIR";

pub fn initialize(app: &AppHandle) -> Result<DatabaseState, BootstrapError> {
    let data_dir = resolve_data_directory(app)?;
    fs::create_dir_all(&data_dir)?;

    let database_path = data_dir.join("forgeos.db");
    let database = tauri::async_runtime::block_on(database::connect(&database_path))?;

    println!("ForgeOS database ready: {}", database_path.display());
    Ok(database)
}

fn resolve_data_directory(app: &AppHandle) -> Result<PathBuf, BootstrapError> {
    if let Some(configured) = env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(configured));
    }

    Ok(app.path().app_data_dir()?.join("database"))
}
