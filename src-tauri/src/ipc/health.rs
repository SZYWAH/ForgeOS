use tauri::State;

use crate::{
    infrastructure::database::DatabaseState,
    read_models::system_status::{self, SystemStatus},
};

use super::IpcError;

#[tauri::command]
pub async fn get_system_status(
    database: State<'_, DatabaseState>,
) -> Result<SystemStatus, IpcError> {
    system_status::load(&database.pool)
        .await
        .map_err(Into::into)
}
