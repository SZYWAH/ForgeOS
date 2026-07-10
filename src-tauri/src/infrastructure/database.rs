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
            "ForgeOS Project Identity schema is incomplete".to_owned(),
        ));
    }

    let project_kernel_columns: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') \
         WHERE name IN ('vision', 'target_user', 'scope', 'non_goals', 'tech_stack', \
         'status', 'lifecycle_stage', 'priority', 'version')",
    )
    .fetch_one(pool)
    .await?;

    if project_kernel_columns != 9 {
        return Err(sqlx::Error::Protocol(
            "ForgeOS Project Identity schema is incomplete".to_owned(),
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
    async fn migrations_create_the_project_identity_schema() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");

        MIGRATOR
            .run(&pool)
            .await
            .expect("Project Identity migrations should run");

        verify_schema(&pool)
            .await
            .expect("schema should contain the Project Identity kernel");

        let project_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&pool)
            .await
            .expect("Rust should be able to read projects");
        assert_eq!(project_count, 0);
    }

    #[tokio::test]
    async fn d1_migration_preserves_d0_projects_and_backfills_defaults() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory SQLite should connect");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0001_create_workspaces_and_projects.sql"
        ))
        .execute(&pool)
        .await
        .expect("D0 schema should be created");

        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-1', 'Personal', 'Asia/Shanghai', \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D0 workspace should be inserted");
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, created_at, updated_at) \
             VALUES ('project-1', 'workspace-1', 'legacy-project', 'Legacy Project', NULL, \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await
        .expect("D0 project should be inserted");

        sqlx::raw_sql(include_str!(
            "../../../database/migrations/0002_project_identity_kernel.sql"
        ))
        .execute(&pool)
        .await
        .expect("D1.1 migration should evolve the D0 schema");

        let migrated: (String, String, String, i32, i64) = sqlx::query_as(
            "SELECT description, status, lifecycle_stage, priority, version \
             FROM projects WHERE id = 'project-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("the D0 project should survive migration");

        assert_eq!(migrated.0, "");
        assert_eq!(migrated.1, "idea");
        assert_eq!(migrated.2, "discovery");
        assert_eq!(migrated.3, 50);
        assert_eq!(migrated.4, 1);

        let description_not_null: i64 = sqlx::query_scalar(
            "SELECT [notnull] FROM pragma_table_info('projects') WHERE name = 'description'",
        )
        .fetch_one(&pool)
        .await
        .expect("description metadata should be readable");
        assert_eq!(description_not_null, 1);

        let second_workspace = sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES ('workspace-2', 'Second', 'UTC', \
             '2026-07-10T00:00:00Z', '2026-07-10T00:00:00Z')",
        )
        .execute(&pool)
        .await;
        assert!(
            second_workspace.is_err(),
            "SQLite must enforce the single Workspace invariant"
        );
    }

}
