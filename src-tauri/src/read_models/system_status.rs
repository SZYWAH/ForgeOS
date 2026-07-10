use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStatus {
    pub app_name: &'static str,
    pub stage: &'static str,
    pub database_connected: bool,
    pub workspace_count: i64,
    pub project_count: i64,
}

pub async fn load(pool: &SqlitePool) -> Result<SystemStatus, sqlx::Error> {
    let workspace_count = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
        .fetch_one(pool)
        .await?;
    let project_count = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(pool)
        .await?;

    Ok(SystemStatus {
        app_name: "ForgeOS",
        stage: "D0 Engineering Foundation",
        database_connected: true,
        workspace_count,
        project_count,
    })
}
