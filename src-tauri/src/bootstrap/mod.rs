mod error;

use std::{env, fs, path::PathBuf, sync::Arc};

use tauri::{AppHandle, Manager};

pub use error::BootstrapError;

use crate::{
    application::{
        AiCandidateKernel, AiProviderKernel, AppPreferencesKernel, CodexExecutionKernel,
        CodexTaskProposalKernel, ContextPackKernel, GitHubImportKernel, ProjectKernel,
        ProjectPulseKernel, RepositoryKernel,
    },
    infrastructure::{
        ai_client::OpenAiCompatibleClient,
        app_preferences::SqliteAppPreferencesRepository,
        codex_activity::CodexAppServerAdapter,
        codex_runner::RecordOnlyCodexRunner,
        context_renderer::MarkdownContextRenderer,
        database::{self, DatabaseState},
        git::GitCliAdapter,
        github::GitHubRestClient,
        repositories::SqliteProjectRepository,
        secret_store::{WindowsCredentialApiKeyStore, WindowsCredentialGitHubTokenStore},
    },
};

const DATA_DIR_ENV: &str = "FORGEOS_DATA_DIR";

pub struct BootstrapState {
    pub database: DatabaseState,
    pub app_preferences_kernel: AppPreferencesKernel,
    pub project_kernel: ProjectKernel,
    pub ai_provider_kernel: AiProviderKernel,
    pub context_pack_kernel: ContextPackKernel,
    pub ai_candidate_kernel: AiCandidateKernel,
    pub repository_kernel: RepositoryKernel,
    pub codex_task_proposal_kernel: CodexTaskProposalKernel,
    pub codex_execution_kernel: CodexExecutionKernel,
    pub github_import_kernel: GitHubImportKernel,
    pub project_pulse_kernel: ProjectPulseKernel,
}

pub fn initialize(app: &AppHandle) -> Result<BootstrapState, BootstrapError> {
    let data_dir = resolve_data_directory(app)?;
    fs::create_dir_all(&data_dir)?;

    let database_path = data_dir.join("forgeos.db");
    let database = tauri::async_runtime::block_on(database::connect(&database_path))?;

    let repository = Arc::new(SqliteProjectRepository::new(database.pool.clone()));
    let app_preferences_kernel = AppPreferencesKernel::new(Arc::new(
        SqliteAppPreferencesRepository::new(database.pool.clone()),
    ));
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
    let codex_task_proposal_kernel = CodexTaskProposalKernel::new(repository.clone());
    let codex_execution_kernel =
        CodexExecutionKernel::new(repository.clone(), Arc::new(RecordOnlyCodexRunner));
    let repository_kernel = RepositoryKernel::new(repository.clone(), Arc::new(GitCliAdapter));
    let github_import_kernel = GitHubImportKernel::new(
        repository.clone(),
        Arc::new(
            GitHubRestClient::new()
                .map_err(|error| BootstrapError::GitHubClient(error.to_string()))?,
        ),
        Arc::new(WindowsCredentialGitHubTokenStore::default()),
    );
    let project_pulse_kernel = ProjectPulseKernel::new(
        repository.clone(),
        Arc::new(CodexAppServerAdapter),
        Arc::new(GitCliAdapter),
        Arc::new(ai_provider_kernel.clone()),
    );
    tauri::async_runtime::block_on(project_pulse_kernel.recover_interrupted_runs())
        .map_err(|error| BootstrapError::ProjectPulse(error.to_string()))?;

    println!("ForgeOS database ready: {}", database_path.display());
    Ok(BootstrapState {
        database,
        app_preferences_kernel,
        project_kernel,
        ai_provider_kernel,
        context_pack_kernel,
        ai_candidate_kernel,
        repository_kernel,
        codex_task_proposal_kernel,
        codex_execution_kernel,
        github_import_kernel,
        project_pulse_kernel,
    })
}

fn resolve_data_directory(app: &AppHandle) -> Result<PathBuf, BootstrapError> {
    if let Some(configured) = env::var_os(DATA_DIR_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(configured));
    }

    Ok(app.path().app_data_dir()?.join("database"))
}
