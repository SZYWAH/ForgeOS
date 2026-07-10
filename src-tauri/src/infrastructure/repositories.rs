use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::{
    domain::{
        LifecycleStage, Project, ProjectDetails, ProjectSnapshot, ProjectStatus, SnapshotDetails,
        Workspace,
    },
    ports::{ProjectRepository, RepositoryError, SnapshotRepository, WorkspaceRepository},
};

#[derive(Clone)]
pub struct SqliteProjectRepository {
    pool: SqlitePool,
}

impl SqliteProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct WorkspaceRow {
    id: String,
    name: String,
    timezone: String,
    created_at: String,
    updated_at: String,
}

impl WorkspaceRow {
    fn into_domain(self) -> Result<Workspace, RepositoryError> {
        Workspace::restore(
            self.id,
            self.name,
            self.timezone,
            parse_timestamp("workspaces.created_at", &self.created_at)?,
            parse_timestamp("workspaces.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    workspace_id: String,
    slug: String,
    name: String,
    description: String,
    vision: String,
    target_user: String,
    scope: String,
    non_goals: String,
    tech_stack: String,
    status: String,
    lifecycle_stage: String,
    priority: i32,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl ProjectRow {
    fn into_domain(self) -> Result<Project, RepositoryError> {
        let status = ProjectStatus::from_str(&self.status).map_err(corrupt)?;
        let lifecycle_stage = LifecycleStage::from_str(&self.lifecycle_stage).map_err(corrupt)?;
        Project::restore(
            self.id,
            self.workspace_id,
            ProjectDetails {
                slug: self.slug,
                name: self.name,
                description: self.description,
                vision: self.vision,
                target_user: self.target_user,
                scope: self.scope,
                non_goals: self.non_goals,
                tech_stack: self.tech_stack,
                status,
                lifecycle_stage,
                priority: self.priority,
            },
            parse_timestamp("projects.created_at", &self.created_at)?,
            parse_timestamp("projects.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const PROJECT_COLUMNS: &str = "id, workspace_id, slug, name, description, vision, \
    target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
    created_at, updated_at, version";

#[derive(FromRow)]
struct SnapshotRow {
    id: String,
    project_id: String,
    version: i64,
    current_goal: String,
    status_summary: String,
    achievements: String,
    blockers: String,
    next_steps: String,
    confirmed_at: String,
    created_at: String,
    updated_at: String,
}

impl SnapshotRow {
    fn into_domain(self) -> Result<ProjectSnapshot, RepositoryError> {
        ProjectSnapshot::restore(
            self.id,
            self.project_id,
            self.version,
            SnapshotDetails {
                current_goal: self.current_goal,
                status_summary: self.status_summary,
                achievements: parse_list("project_snapshots.achievements", &self.achievements)?,
                blockers: parse_list("project_snapshots.blockers", &self.blockers)?,
                next_steps: parse_list("project_snapshots.next_steps", &self.next_steps)?,
            },
            parse_timestamp("project_snapshots.confirmed_at", &self.confirmed_at)?,
            parse_timestamp("project_snapshots.created_at", &self.created_at)?,
            parse_timestamp("project_snapshots.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

const SNAPSHOT_COLUMNS: &str = "id, project_id, version, current_goal, status_summary, \
    achievements, blockers, next_steps, confirmed_at, created_at, updated_at";

#[async_trait]
impl WorkspaceRepository for SqliteProjectRepository {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT id, name, timezone, created_at, updated_at \
             FROM workspaces ORDER BY created_at, id LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?;

        row.map(WorkspaceRow::into_domain).transpose()
    }

    async fn insert_workspace(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(&workspace.timezone)
        .bind(timestamp(workspace.created_at))
        .bind(timestamp(workspace.updated_at))
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn insert_project(&self, project: &Project) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, vision, \
             target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
             created_at, updated_at, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&project.id)
        .bind(&project.workspace_id)
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.vision)
        .bind(&project.target_user)
        .bind(&project.scope)
        .bind(&project.non_goals)
        .bind(&project.tech_stack)
        .bind(project.status.as_str())
        .bind(project.lifecycle_stage.as_str())
        .bind(project.priority)
        .bind(timestamp(project.created_at))
        .bind(timestamp(project.updated_at))
        .bind(project.version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, RepositoryError> {
        let query = format!(
            "SELECT {PROJECT_COLUMNS} FROM projects \
             WHERE workspace_id = ? ORDER BY updated_at DESC, name COLLATE NOCASE, id"
        );
        let rows = sqlx::query_as::<_, ProjectRow>(&query)
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(ProjectRow::into_domain).collect()
    }

    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError> {
        let query =
            format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE workspace_id = ? AND id = ?");
        let row = sqlx::query_as::<_, ProjectRow>(&query)
            .bind(workspace_id)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(ProjectRow::into_domain).transpose()
    }

    async fn update_project(
        &self,
        project: &Project,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE projects SET slug = ?, name = ?, description = ?, vision = ?, \
             target_user = ?, scope = ?, non_goals = ?, tech_stack = ?, status = ?, \
             lifecycle_stage = ?, priority = ?, updated_at = ?, version = ? \
             WHERE id = ? AND workspace_id = ? AND version = ?",
        )
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.vision)
        .bind(&project.target_user)
        .bind(&project.scope)
        .bind(&project.non_goals)
        .bind(&project.tech_stack)
        .bind(project.status.as_str())
        .bind(project.lifecycle_stage.as_str())
        .bind(project.priority)
        .bind(timestamp(project.updated_at))
        .bind(project.version)
        .bind(&project.id)
        .bind(&project.workspace_id)
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl SnapshotRepository for SqliteProjectRepository {
    async fn insert_snapshot(&self, snapshot: &ProjectSnapshot) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
             WHERE NOT EXISTS (SELECT 1 FROM projects WHERE id = ? AND status = 'archived')",
        )
        .bind(snapshot.id())
        .bind(snapshot.project_id())
        .bind(snapshot.version())
        .bind(snapshot.current_goal())
        .bind(snapshot.status_summary())
        .bind(encode_list("achievements", snapshot.achievements())?)
        .bind(encode_list("blockers", snapshot.blockers())?)
        .bind(encode_list("next_steps", snapshot.next_steps())?)
        .bind(timestamp(snapshot.confirmed_at()))
        .bind(timestamp(snapshot.created_at()))
        .bind(timestamp(snapshot.updated_at()))
        .bind(snapshot.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError> {
        let query = format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM project_snapshots \
             WHERE project_id = ? ORDER BY version DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, SnapshotRow>(&query)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(SnapshotRow::into_domain).transpose()
    }

    async fn list_snapshot_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSnapshot>, RepositoryError> {
        let query = format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM project_snapshots \
             WHERE project_id = ? ORDER BY version DESC"
        );
        let rows = sqlx::query_as::<_, SnapshotRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(SnapshotRow::into_domain).collect()
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_timestamp(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn encode_list(field: &str, values: &[String]) -> Result<String, RepositoryError> {
    serde_json::to_string(values)
        .map_err(|error| RepositoryError::Storage(format!("encode {field}: {error}")))
}

fn parse_list(field: &str, value: &str) -> Result<Vec<String>, RepositoryError> {
    serde_json::from_str(value)
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}

fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn map_write_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            RepositoryError::Conflict
        }
        _ => storage(error),
    }
}
