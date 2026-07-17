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
            jobs::spawn_scheduler(app.handle().clone(), state.project_pulse_kernel.clone());
            app.manage(state.database);
            app.manage(state.app_preferences_kernel);
            app.manage(state.project_kernel);
            app.manage(state.ai_provider_kernel);
            app.manage(state.context_pack_kernel);
            app.manage(state.ai_candidate_kernel);
            app.manage(state.repository_kernel);
            app.manage(state.codex_task_proposal_kernel);
            app.manage(state.codex_execution_kernel);
            app.manage(state.github_import_kernel);
            app.manage(state.project_pulse_kernel);
            app.manage(state.project_memory_kernel);
            app.manage(state.achievement_kernel);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc::app_preferences::get_app_preferences,
            ipc::app_preferences::update_app_preferences,
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
            ipc::ai_candidates::list_candidates,
            ipc::ai_candidates::get_candidate,
            ipc::repositories::inspect_repository,
            ipc::repositories::register_repository,
            ipc::repositories::list_repositories,
            ipc::repositories::refresh_repository,
            ipc::repositories::create_commit_evidence,
            ipc::repositories::create_status_evidence,
            ipc::repositories::list_repository_evidence,
            ipc::codex_task_proposals::list_proposals,
            ipc::codex_task_proposals::get_proposal,
            ipc::codex_executions::list_codex_executions,
            ipc::codex_executions::get_codex_execution,
            ipc::github::get_github_connector,
            ipc::github::set_github_token,
            ipc::github::clear_github_token,
            ipc::github::preview_github_repository,
            ipc::github::list_github_import_drafts,
            ipc::github::review_github_import,
            ipc::github::reject_github_import,
            ipc::github::import_github_project,
            ipc::github::get_project_github_import,
            ipc::project_pulse::get_codex_connector_status,
            ipc::project_pulse::update_codex_connector_settings,
            ipc::project_pulse::get_project_activity_settings,
            ipc::project_pulse::update_project_activity_settings,
            ipc::project_pulse::sync_project_activity,
            ipc::project_pulse::get_activity_sync_run,
            ipc::project_pulse::list_activity_sync_runs,
            ipc::project_pulse::get_latest_project_pulse,
            ipc::project_pulse::list_project_pulses,
            ipc::project_pulse::get_project_pulse_sources,
            ipc::project_pulse::update_project_pulse_draft,
            ipc::project_pulse::confirm_project_pulse,
            ipc::project_pulse::dismiss_project_pulse,
            ipc::project_memory::list_project_memory_summaries,
            ipc::project_memory::get_project_memory_overview,
            ipc::project_memory::list_development_history,
            ipc::project_memory::build_project_ai_context,
            ipc::project_memory::build_snapshot_repair_draft,
            ipc::achievements::list_project_achievements,
            ipc::achievements::get_project_achievement,
            ipc::achievements::create_project_achievement,
            ipc::achievements::create_achievement_draft_from_history,
            ipc::achievements::update_project_achievement,
            ipc::achievements::confirm_project_achievement,
            ipc::achievements::archive_project_achievement,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ForgeOS");
}
