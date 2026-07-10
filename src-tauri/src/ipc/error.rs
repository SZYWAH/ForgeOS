use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
    pub retryable: bool,
}

impl From<sqlx::Error> for IpcError {
    fn from(error: sqlx::Error) -> Self {
        Self {
            code: "storage_error",
            message: format!("Unable to read the ForgeOS foundation database: {error}"),
            retryable: true,
        }
    }
}
