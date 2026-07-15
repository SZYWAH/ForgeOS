use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{FromRow, Sqlite, Transaction};

use crate::{
    domain::{
        AchievementCategory, AchievementDetails, AchievementSource, AchievementSourceMode,
        AchievementSourceType, AchievementStatus, ProjectAchievement,
    },
    ports::{AchievementRepository, RepositoryError},
};

use super::repositories::SqliteProjectRepository;

#[derive(FromRow)]
struct AchievementRow {
    id: String,
    project_id: String,
    category: String,
    title: String,
    problem: String,
    approach: String,
    result: String,
    validation: String,
    impact: String,
    technologies: String,
    resume_bullet: String,
    status: String,
    source_mode: String,
    featured: i64,
    source_pulse_id: Option<String>,
    confirmed_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl AchievementRow {
    fn into_domain(self) -> Result<ProjectAchievement, RepositoryError> {
        ProjectAchievement::restore(
            self.id,
            self.project_id,
            AchievementDetails {
                category: AchievementCategory::from_str(&self.category).map_err(corrupt)?,
                title: self.title,
                problem: self.problem,
                approach: self.approach,
                result: self.result,
                validation: self.validation,
                impact: self.impact,
                technologies: serde_json::from_str(&self.technologies).map_err(corrupt)?,
                resume_bullet: self.resume_bullet,
                featured: boolean("project_achievements.featured", self.featured)?,
            },
            AchievementStatus::from_str(&self.status).map_err(corrupt)?,
            AchievementSourceMode::from_str(&self.source_mode).map_err(corrupt)?,
            self.source_pulse_id,
            optional_time("project_achievements.confirmed_at", self.confirmed_at)?,
            parse_time("project_achievements.created_at", &self.created_at)?,
            parse_time("project_achievements.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const ACHIEVEMENT_COLUMNS: &str = "id, project_id, category, title, problem, approach, result, validation, impact, technologies, resume_bullet, status, source_mode, featured, source_pulse_id, confirmed_at, created_at, updated_at, version";

#[derive(FromRow)]
struct SourceRow {
    achievement_id: String,
    project_id: String,
    source_type: String,
    source_id: String,
    source_order: i64,
}

impl SourceRow {
    fn into_domain(self) -> Result<AchievementSource, RepositoryError> {
        AchievementSource::create(
            self.achievement_id,
            self.project_id,
            AchievementSourceType::from_str(&self.source_type).map_err(corrupt)?,
            self.source_id,
            self.source_order,
        )
        .map_err(corrupt)
    }
}

#[async_trait]
impl AchievementRepository for SqliteProjectRepository {
    async fn insert_achievement(
        &self,
        achievement: &ProjectAchievement,
        sources: &[AchievementSource],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        insert_achievement_tx(&mut transaction, achievement, sources).await?;
        transaction.commit().await.map_err(storage)
    }

    async fn list_achievements(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAchievement>, RepositoryError> {
        let query = format!("SELECT {ACHIEVEMENT_COLUMNS} FROM project_achievements WHERE project_id = ? ORDER BY CASE status WHEN 'draft' THEN 0 WHEN 'confirmed' THEN 1 ELSE 2 END, featured DESC, updated_at DESC, id DESC");
        sqlx::query_as::<_, AchievementRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(AchievementRow::into_domain)
            .collect()
    }

    async fn get_achievement(
        &self,
        achievement_id: &str,
    ) -> Result<Option<ProjectAchievement>, RepositoryError> {
        let query = format!("SELECT {ACHIEVEMENT_COLUMNS} FROM project_achievements WHERE id = ?");
        sqlx::query_as::<_, AchievementRow>(&query)
            .bind(achievement_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(AchievementRow::into_domain)
            .transpose()
    }

    async fn list_achievement_sources(
        &self,
        achievement_id: &str,
    ) -> Result<Vec<AchievementSource>, RepositoryError> {
        sqlx::query_as::<_, SourceRow>("SELECT achievement_id, project_id, source_type, source_id, source_order FROM project_achievement_sources WHERE achievement_id = ? ORDER BY source_order, source_type, source_id")
            .bind(achievement_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(SourceRow::into_domain)
            .collect()
    }

    async fn update_achievement(
        &self,
        achievement: &ProjectAchievement,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE project_achievements SET category = ?, title = ?, problem = ?, approach = ?, result = ?, validation = ?, impact = ?, technologies = ?, resume_bullet = ?, status = ?, featured = ?, confirmed_at = ?, updated_at = ?, version = ? WHERE id = ? AND project_id = ? AND version = ?")
            .bind(achievement.details.category.as_str())
            .bind(&achievement.details.title)
            .bind(&achievement.details.problem)
            .bind(&achievement.details.approach)
            .bind(&achievement.details.result)
            .bind(&achievement.details.validation)
            .bind(&achievement.details.impact)
            .bind(serde_json::to_string(&achievement.details.technologies).map_err(storage_json)?)
            .bind(&achievement.details.resume_bullet)
            .bind(achievement.status.as_str())
            .bind(i64::from(achievement.details.featured))
            .bind(achievement.confirmed_at.map(time_string))
            .bind(time_string(achievement.updated_at))
            .bind(achievement.version)
            .bind(&achievement.id)
            .bind(&achievement.project_id)
            .bind(expected_version)
            .execute(&self.pool)
            .await
            .map_err(write_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

pub(crate) async fn insert_achievement_tx(
    transaction: &mut Transaction<'_, Sqlite>,
    achievement: &ProjectAchievement,
    sources: &[AchievementSource],
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO project_achievements (id, project_id, category, title, problem, approach, result, validation, impact, technologies, resume_bullet, status, source_mode, featured, source_pulse_id, confirmed_at, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&achievement.id)
        .bind(&achievement.project_id)
        .bind(achievement.details.category.as_str())
        .bind(&achievement.details.title)
        .bind(&achievement.details.problem)
        .bind(&achievement.details.approach)
        .bind(&achievement.details.result)
        .bind(&achievement.details.validation)
        .bind(&achievement.details.impact)
        .bind(serde_json::to_string(&achievement.details.technologies).map_err(storage_json)?)
        .bind(&achievement.details.resume_bullet)
        .bind(achievement.status.as_str())
        .bind(achievement.source_mode.as_str())
        .bind(i64::from(achievement.details.featured))
        .bind(&achievement.source_pulse_id)
        .bind(achievement.confirmed_at.map(time_string))
        .bind(time_string(achievement.created_at))
        .bind(time_string(achievement.updated_at))
        .bind(achievement.version)
        .execute(&mut **transaction)
        .await
        .map_err(write_error)?;
    for source in sources {
        sqlx::query("INSERT INTO project_achievement_sources (achievement_id, project_id, source_type, source_id, source_order) VALUES (?, ?, ?, ?, ?)")
            .bind(&source.achievement_id)
            .bind(&source.project_id)
            .bind(source.source_type.as_str())
            .bind(&source.source_id)
            .bind(source.source_order)
            .execute(&mut **transaction)
            .await
            .map_err(write_error)?;
    }
    Ok(())
}

fn parse_time(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn optional_time(
    field: &str,
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, RepositoryError> {
    value.map(|value| parse_time(field, &value)).transpose()
}

fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn boolean(field: &str, value: i64) -> Result<bool, RepositoryError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(RepositoryError::CorruptData(format!(
            "{field}: invalid boolean"
        ))),
    }
}

fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}

fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn storage_json(error: serde_json::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn write_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            RepositoryError::Conflict
        }
        sqlx::Error::Database(database_error)
            if database_error
                .message()
                .contains("achievement_source_project_mismatch") =>
        {
            RepositoryError::EvidenceTargetInvalid
        }
        _ => storage(error),
    }
}
