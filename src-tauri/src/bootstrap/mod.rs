mod error;

use std::{env, fs, path::PathBuf, sync::Arc};

use tauri::{AppHandle, Manager};

pub use error::BootstrapError;

use crate::{
    application::ProjectKernel,
    infrastructure::{
        database::{self, DatabaseState},
        repositories::SqliteProjectRepository,
    },
};

const DATA_DIR_ENV: &str = "FORGEOS_DATA_DIR";

pub struct BootstrapState {
    pub database: DatabaseState,
    pub project_kernel: ProjectKernel,
}

pub fn initialize(app: &AppHandle) -> Result<BootstrapState, BootstrapError> {
    let data_dir = resolve_data_directory(app)?;
    fs::create_dir_all(&data_dir)?;

    let database_path = data_dir.join("forgeos.db");
    let database = tauri::async_runtime::block_on(database::connect(&database_path))?;

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let project_kernel = ProjectKernel::new(repository);

    println!("ForgeOS database ready: {}", database_path.display());
    Ok(BootstrapState {
        database,
        project_kernel,
    })
}

fn resolve_data_directory(app: &AppHandle) -> Result<PathBuf, BootstrapError> {
    if let Some(configured) = env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(configured));
    }

    Ok(app.path().app_data_dir()?.join("database"))
}
