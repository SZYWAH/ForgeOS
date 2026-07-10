use std::{path::Path, time::Duration};

use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    SqlitePool,
};

static MIGRATOR: Migrator = sqlx::migrate!("../database/migrations");

pub struct DatabaseState {
    pub pool: SqlitePool,
}

pub async fn connect(path: &Path) -> Result<DatabaseState, DatabaseInitError> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?;

    MIGRATOR.run(&pool).await?;
    verify_schema(&pool).await?;

    Ok(DatabaseState { pool })
}

async fn verify_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let table_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name IN ('workspaces', 'projects')",
    )
    .fetch_one(pool)
    .await?;

    if table_count != 2 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS foundation schema is incomplete".to_owned(),
        ));
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseInitError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migration_creates_the_d0_business_tables() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");

        MIGRATOR.run(&pool).await.expect("D0 migration should run");

        verify_schema(&pool)
            .await
            .expect("D0 schema should contain workspaces and projects");

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read projects");

        assert_eq!(project_count, 0);
    }
}
