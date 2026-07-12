pub mod application;
pub mod bootstrap;
pub mod domain;
pub mod infrastructure;
pub mod ipc;
pub mod jobs;
pub mod ports;
pub mod read_models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state = bootstrap::initialize(app.handle())?;
            app.manage(state.database);
            app.manage(state.project_kernel);
            app.manage(state.ai_provider_kernel);
            app.manage(state.context_pack_kernel);
            app.manage(state.ai_candidate_kernel);
            app.manage(state.repository_kernel);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::health::get_system_status,
            ipc::workspaces::get_current_workspace,
            ipc::workspaces::create_workspace,
            ipc::projects::create_project,
            ipc::projects::list_projects,
            ipc::projects::get_project,
            ipc::projects::update_project,
            ipc::projects::archive_project,
            ipc::snapshots::create_snapshot,
            ipc::snapshots::get_current_snapshot,
            ipc::snapshots::get_snapshot_history,
            ipc::snapshots::update_snapshot,
            ipc::sessions::start_session,
            ipc::sessions::get_current_session,
            ipc::sessions::list_sessions,
            ipc::sessions::update_session,
            ipc::sessions::complete_session,
            ipc::sessions::abandon_session,
            ipc::evidences::create_evidence,
            ipc::evidences::list_evidences,
            ipc::evidences::get_evidence,
            ipc::evidences::update_evidence,
            ipc::evidences::verify_evidence,
            ipc::evidences::invalidate_evidence,
            ipc::evidences::link_evidence,
            ipc::ai_providers::create_ai_provider,
            ipc::ai_providers::list_ai_providers,
            ipc::ai_providers::get_ai_provider,
            ipc::ai_providers::update_ai_provider,
            ipc::ai_providers::set_ai_provider_enabled,
            ipc::ai_providers::set_ai_provider_api_key,
            ipc::ai_providers::clear_ai_provider_api_key,
            ipc::ai_providers::invoke_ai_provider,
            ipc::context_packs::generate_context_pack,
            ipc::context_packs::list_context_exports,
            ipc::context_packs::get_context_export,
            ipc::context_packs::invoke_context_with_ai,
            ipc::ai_candidates::create_ai_candidate,
            ipc::ai_candidates::list_candidates,
            ipc::ai_candidates::get_candidate,
            ipc::ai_candidates::accept_candidate,
            ipc::ai_candidates::reject_candidate,
            ipc::ai_candidates::archive_candidate,
            ipc::repositories::inspect_repository,
            ipc::repositories::register_repository,
            ipc::repositories::list_repositories,
            ipc::repositories::refresh_repository,
            ipc::repositories::create_commit_evidence,
            ipc::repositories::create_status_evidence,
            ipc::repositories::list_repository_evidence,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ForgeOS");
}
