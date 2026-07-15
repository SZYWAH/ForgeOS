use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::{
    domain::{AppPreferences, LanguageMode},
    ports::{AppPreferencesRepository, RepositoryError},
};

#[derive(Clone)]
pub struct SqliteAppPreferencesRepository {
    pool: SqlitePool,
}

impl SqliteAppPreferencesRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct AppPreferencesRow {
    language_mode: String,
    show_technical_terms: bool,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl AppPreferencesRow {
    fn into_domain(self) -> Result<AppPreferences, RepositoryError> {
        let parse_timestamp = |field: &str, value: &str| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|error| {
                    RepositoryError::CorruptData(format!("{field} is invalid: {error}"))
                })
        };

        AppPreferences::restore(
            LanguageMode::from_str(&self.language_mode).map_err(corrupt)?,
            self.show_technical_terms,
            self.version,
            parse_timestamp("app_preferences.created_at", &self.created_at)?,
            parse_timestamp("app_preferences.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

#[async_trait]
impl AppPreferencesRepository for SqliteAppPreferencesRepository {
    async fn get(&self) -> Result<Option<AppPreferences>, RepositoryError> {
        sqlx::query_as::<_, AppPreferencesRow>(
            "SELECT language_mode, show_technical_terms, version, created_at, updated_at \
             FROM app_preferences WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(AppPreferencesRow::into_domain)
        .transpose()
    }

    async fn update(
        &self,
        preferences: &AppPreferences,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE app_preferences \
             SET language_mode = ?, show_technical_terms = ?, version = ?, updated_at = ? \
             WHERE id = 1 AND version = ?",
        )
        .bind(preferences.language_mode().as_str())
        .bind(preferences.show_technical_terms())
        .bind(preferences.version())
        .bind(preferences.updated_at().to_rfc3339())
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(storage)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}

fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}
