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
            let database = bootstrap::initialize(app.handle())?;
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ipc::health::get_system_status])
        .run(tauri::generate_context!())
        .expect("failed to run ForgeOS");
}
