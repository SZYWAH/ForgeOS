mod error;

use std::{env, fs, path::PathBuf, sync::Arc};

use tauri::{AppHandle, Manager};

pub use error::BootstrapError;

use crate::{
    application::{
        AiCandidateKernel, AiProviderKernel, ContextPackKernel, ProjectKernel, RepositoryKernel,
    },
    infrastructure::{
        ai_client::OpenAiCompatibleClient,
        context_renderer::MarkdownContextRenderer,
        database::{self, DatabaseState},
        git::GitCliAdapter,
        repositories::SqliteProjectRepository,
        secret_store::WindowsCredentialApiKeyStore,
    },
};

const DATA_DIR_ENV: &str = "FORGEOS_DATA_DIR";

pub struct BootstrapState {
    pub database: DatabaseState,
    pub project_kernel: ProjectKernel,
    pub ai_provider_kernel: AiProviderKernel,
    pub context_pack_kernel: ContextPackKernel,
    pub ai_candidate_kernel: AiCandidateKernel,
    pub repository_kernel: RepositoryKernel,
}

pub fn initialize(app: &AppHandle) -> Result<BootstrapState, BootstrapError> {
    let data_dir = resolve_data_directory(app)?;
    fs::create_dir_all(&data_dir)?;

    let database_path = data_dir.join("forgeos.db");
    let database = tauri::async_runtime::block_on(database::connect(&database_path))?;

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let project_kernel = ProjectKernel::new(repository.clone());
    let ai_provider_kernel = AiProviderKernel::new(
        repository.clone(),
        Arc::new(WindowsCredentialApiKeyStore::default()),
        Arc::new(OpenAiCompatibleClient::new()?),
    );
    let context_pack_kernel = ContextPackKernel::new(
        repository.clone(),
        Arc::new(MarkdownContextRenderer),
        Arc::new(ai_provider_kernel.clone()),
    );
    let ai_candidate_kernel = AiCandidateKernel::new(repository.clone());
    let repository_kernel = RepositoryKernel::new(repository, Arc::new(GitCliAdapter));

    println!("ForgeOS database ready: {}", database_path.display());
    Ok(BootstrapState {
        database,
        project_kernel,
        ai_provider_kernel,
        context_pack_kernel,
        ai_candidate_kernel,
        repository_kernel,
    })
}

fn resolve_data_directory(app: &AppHandle) -> Result<PathBuf, BootstrapError> {
    if let Some(configured) = env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(configured));
    }

    Ok(app.path().app_data_dir()?.join("database"))
}
