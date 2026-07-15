use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::FromRow;

use crate::{
    domain::{RepositoryCommitObservation, RepositoryHistoryCursor},
    ports::{ProjectMemoryRepository, RepositoryError},
};

use super::repositories::SqliteProjectRepository;

#[derive(FromRow)]
struct CommitRow {
    id: String,
    repository_id: String,
    project_id: String,
    commit_sha: String,
    message: String,
    author_name: String,
    authored_at: String,
    observed_at: String,
    source_hash: String,
}

impl CommitRow {
    fn into_domain(self) -> Result<RepositoryCommitObservation, RepositoryError> {
        RepositoryCommitObservation::create(
            self.id,
            self.repository_id,
            self.project_id,
            self.commit_sha,
            self.message,
            self.author_name,
            parse_time(
                "repository_commit_observations.authored_at",
                &self.authored_at,
            )?,
            parse_time(
                "repository_commit_observations.observed_at",
                &self.observed_at,
            )?,
            self.source_hash,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct CursorRow {
    repository_id: String,
    project_id: String,
    initial_backfill_completed: i64,
    last_scanned_head_commit: Option<String>,
    last_success_at: Option<String>,
    version: i64,
}

impl CursorRow {
    fn into_domain(self) -> Result<RepositoryHistoryCursor, RepositoryError> {
        RepositoryHistoryCursor::restore(
            self.repository_id,
            self.project_id,
            boolean(
                "repository_history_cursors.initial_backfill_completed",
                self.initial_backfill_completed,
            )?,
            self.last_scanned_head_commit,
            self.last_success_at
                .map(|value| parse_time("repository_history_cursors.last_success_at", &value))
                .transpose()?,
            self.version,
        )
        .map_err(corrupt)
    }
}

#[async_trait]
impl ProjectMemoryRepository for SqliteProjectRepository {
    async fn get_repository_history_cursor(
        &self,
        repository_id: &str,
    ) -> Result<Option<RepositoryHistoryCursor>, RepositoryError> {
        sqlx::query_as::<_, CursorRow>("SELECT repository_id, project_id, initial_backfill_completed, last_scanned_head_commit, last_success_at, version FROM repository_history_cursors WHERE repository_id = ?")
            .bind(repository_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(CursorRow::into_domain)
            .transpose()
    }

    async fn upsert_repository_history_cursor(
        &self,
        cursor: &RepositoryHistoryCursor,
    ) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO repository_history_cursors (repository_id, project_id, initial_backfill_completed, last_scanned_head_commit, last_success_at, version) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(repository_id) DO UPDATE SET initial_backfill_completed = excluded.initial_backfill_completed, last_scanned_head_commit = excluded.last_scanned_head_commit, last_success_at = excluded.last_success_at, version = repository_history_cursors.version + 1 WHERE repository_history_cursors.project_id = excluded.project_id")
            .bind(&cursor.repository_id)
            .bind(&cursor.project_id)
            .bind(i64::from(cursor.initial_backfill_completed))
            .bind(&cursor.last_scanned_head_commit)
            .bind(cursor.last_success_at.map(time_string))
            .bind(cursor.version.max(1))
            .execute(&self.pool)
            .await
            .map_err(write_error)?;
        Ok(())
    }

    async fn insert_repository_commit_observation(
        &self,
        observation: &RepositoryCommitObservation,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query("INSERT INTO repository_commit_observations (id, repository_id, project_id, commit_sha, message, author_name, authored_at, observed_at, source_hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(repository_id, commit_sha) DO NOTHING")
            .bind(&observation.id)
            .bind(&observation.repository_id)
            .bind(&observation.project_id)
            .bind(&observation.commit_sha)
            .bind(&observation.message)
            .bind(&observation.author_name)
            .bind(time_string(observation.authored_at))
            .bind(time_string(observation.observed_at))
            .bind(&observation.source_hash)
            .execute(&self.pool)
            .await
            .map_err(write_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn list_repository_commit_observations(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<RepositoryCommitObservation>, RepositoryError> {
        sqlx::query_as::<_, CommitRow>("SELECT id, repository_id, project_id, commit_sha, message, author_name, authored_at, observed_at, source_hash FROM repository_commit_observations WHERE project_id = ? ORDER BY authored_at DESC, id DESC LIMIT ?")
            .bind(project_id)
            .bind(limit.clamp(1, 2_000))
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(CommitRow::into_domain)
            .collect()
    }
}

fn parse_time(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn boolean(field: &str, value: i64) -> Result<bool, RepositoryError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(RepositoryError::CorruptData(format!(
            "{field}: expected 0 or 1"
        ))),
    }
}

fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}

fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn write_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            RepositoryError::Conflict
        }
        _ => storage(error),
    }
}
