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
        ])
        .run(tauri::generate_context!())
        .expect("failed to run ForgeOS");
}
