use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, Sqlite, Transaction};

use crate::{
    domain::{
        ActivitySyncRun, CodexActivityObservation, CodexConnectorSettings, ProjectActivitySettings,
        ProjectPulse, ProjectPulseDetails, ProjectPulseQuality, PulseActivityType,
        PulseAnalysisMode, PulseAnalysisRevision, PulseStatus, SyncMode, SyncStage, SyncStatus,
        SyncTrigger,
    },
    ports::{CodexSyncCursor, ProjectActivityRepository, PulseConfirmationWrite, RepositoryError},
};

use super::achievement_repository::insert_achievement_tx;
use super::repositories::SqliteProjectRepository;

#[derive(FromRow)]
struct ConnectorRow {
    workspace_id: String,
    executable_path: Option<String>,
    last_detected_version: Option<String>,
    last_health_check_at: Option<String>,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl ConnectorRow {
    fn into_domain(self) -> Result<CodexConnectorSettings, RepositoryError> {
        CodexConnectorSettings::restore(
            self.workspace_id,
            self.executable_path,
            self.last_detected_version,
            optional_timestamp(
                "codex_connector_settings.last_health_check_at",
                self.last_health_check_at,
            )?,
            self.version,
            timestamp_from("codex_connector_settings.created_at", &self.created_at)?,
            timestamp_from("codex_connector_settings.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct ActivitySettingsRow {
    project_id: String,
    sync_mode: String,
    ai_enabled: i64,
    ai_provider_id: Option<String>,
    initial_backfill_completed: i64,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl ActivitySettingsRow {
    fn into_domain(self) -> Result<ProjectActivitySettings, RepositoryError> {
        ProjectActivitySettings::restore(
            self.project_id,
            SyncMode::from_str(&self.sync_mode).map_err(corrupt)?,
            boolean("project_activity_settings.ai_enabled", self.ai_enabled)?,
            self.ai_provider_id,
            boolean(
                "project_activity_settings.initial_backfill_completed",
                self.initial_backfill_completed,
            )?,
            self.version,
            timestamp_from("project_activity_settings.created_at", &self.created_at)?,
            timestamp_from("project_activity_settings.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct SyncRunRow {
    id: String,
    project_id: String,
    trigger: String,
    status: String,
    stage: String,
    threads_scanned: i64,
    turns_imported: i64,
    observations_created: i64,
    commits_imported: i64,
    ai_invoked: i64,
    started_at: String,
    completed_at: Option<String>,
    error_code: Option<String>,
    error_summary: Option<String>,
}

impl SyncRunRow {
    fn into_domain(self) -> Result<ActivitySyncRun, RepositoryError> {
        Ok(ActivitySyncRun {
            id: self.id,
            project_id: self.project_id,
            trigger: SyncTrigger::from_str(&self.trigger).map_err(corrupt)?,
            status: SyncStatus::from_str(&self.status).map_err(corrupt)?,
            stage: SyncStage::from_str(&self.stage).map_err(corrupt)?,
            threads_scanned: self.threads_scanned,
            turns_imported: self.turns_imported,
            observations_created: self.observations_created,
            commits_imported: self.commits_imported,
            ai_invoked: boolean("project_activity_sync_runs.ai_invoked", self.ai_invoked)?,
            started_at: timestamp_from("project_activity_sync_runs.started_at", &self.started_at)?,
            completed_at: optional_timestamp(
                "project_activity_sync_runs.completed_at",
                self.completed_at,
            )?,
            error_code: self.error_code,
            error_summary: self.error_summary,
        })
    }
}

const SYNC_RUN_COLUMNS: &str = "id, project_id, trigger, status, stage, threads_scanned, turns_imported, observations_created, commits_imported, ai_invoked, started_at, completed_at, error_code, error_summary";

#[derive(FromRow)]
struct CursorRow {
    repository_id: String,
    project_id: String,
    last_thread_updated_at: Option<String>,
    last_success_at: Option<String>,
    version: i64,
}

impl CursorRow {
    fn into_domain(self) -> Result<CodexSyncCursor, RepositoryError> {
        Ok(CodexSyncCursor {
            repository_id: self.repository_id,
            project_id: self.project_id,
            last_thread_updated_at: optional_timestamp(
                "codex_sync_cursors.last_thread_updated_at",
                self.last_thread_updated_at,
            )?,
            last_success_at: optional_timestamp(
                "codex_sync_cursors.last_success_at",
                self.last_success_at,
            )?,
            version: self.version,
        })
    }
}

#[derive(FromRow)]
struct ObservationRow {
    id: String,
    project_id: String,
    repository_id: String,
    thread_id: String,
    turn_id: String,
    source_kind: String,
    turn_status: String,
    goal_hint: String,
    activity_summary: String,
    outcome_summary: String,
    validation_summary: String,
    handoff_summary: String,
    blockers: String,
    source_updated_at: String,
    source_hash: String,
    observed_at: String,
}

impl ObservationRow {
    fn into_domain(self) -> Result<CodexActivityObservation, RepositoryError> {
        CodexActivityObservation::create(
            self.id,
            self.project_id,
            self.repository_id,
            self.thread_id,
            self.turn_id,
            self.source_kind,
            self.turn_status,
            self.goal_hint,
            self.activity_summary,
            self.outcome_summary,
            self.validation_summary,
            self.handoff_summary,
            decode_list("codex_activity_observations.blockers", &self.blockers)?,
            timestamp_from(
                "codex_activity_observations.source_updated_at",
                &self.source_updated_at,
            )?,
            self.source_hash,
            timestamp_from("codex_activity_observations.observed_at", &self.observed_at)?,
        )
        .map_err(corrupt)
    }
}

const OBSERVATION_COLUMNS: &str = "id, project_id, repository_id, thread_id, turn_id, source_kind, turn_status, goal_hint, activity_summary, outcome_summary, validation_summary, handoff_summary, blockers, source_updated_at, source_hash, observed_at";

#[derive(FromRow)]
struct PulseRow {
    id: String,
    project_id: String,
    status: String,
    analysis_mode: String,
    ai_provider_id: Option<String>,
    ai_model: Option<String>,
    activity_type: String,
    analysis_revision: String,
    source_window_started_at: Option<String>,
    source_window_ended_at: Option<String>,
    selected_observation_count: i64,
    available_observation_count: i64,
    current_goal: String,
    status_summary: String,
    achievements: String,
    blockers: String,
    next_steps: String,
    validation_summary: String,
    handoff: String,
    git_summary: String,
    snapshot_basis_version: Option<i64>,
    session_basis_id: Option<String>,
    session_basis_version: Option<i64>,
    working_copy_id: Option<String>,
    content_hash: String,
    generated_at: String,
    updated_at: String,
    version: i64,
    confirmed_snapshot_id: Option<String>,
    confirmed_session_id: Option<String>,
}

impl PulseRow {
    fn into_domain(self) -> Result<ProjectPulse, RepositoryError> {
        ProjectPulse::restore(
            self.id,
            self.project_id,
            PulseStatus::from_str(&self.status).map_err(corrupt)?,
            PulseAnalysisMode::from_str(&self.analysis_mode).map_err(corrupt)?,
            self.ai_provider_id,
            self.ai_model,
            ProjectPulseQuality::new(
                PulseActivityType::from_str(&self.activity_type).map_err(corrupt)?,
                PulseAnalysisRevision::from_str(&self.analysis_revision).map_err(corrupt)?,
                optional_timestamp(
                    "project_pulses.source_window_started_at",
                    self.source_window_started_at,
                )?,
                optional_timestamp(
                    "project_pulses.source_window_ended_at",
                    self.source_window_ended_at,
                )?,
                self.selected_observation_count,
                self.available_observation_count,
            )
            .map_err(corrupt)?,
            ProjectPulseDetails {
                current_goal: self.current_goal,
                status_summary: self.status_summary,
                achievements: decode_list("project_pulses.achievements", &self.achievements)?,
                blockers: decode_list("project_pulses.blockers", &self.blockers)?,
                next_steps: decode_list("project_pulses.next_steps", &self.next_steps)?,
                validation_summary: self.validation_summary,
                handoff: self.handoff,
                git_summary: self.git_summary,
            },
            self.snapshot_basis_version,
            self.session_basis_id,
            self.session_basis_version,
            self.working_copy_id,
            self.content_hash,
            timestamp_from("project_pulses.generated_at", &self.generated_at)?,
            timestamp_from("project_pulses.updated_at", &self.updated_at)?,
            self.version,
            self.confirmed_snapshot_id,
            self.confirmed_session_id,
        )
        .map_err(corrupt)
    }
}

const PULSE_COLUMNS: &str = "id, project_id, status, analysis_mode, ai_provider_id, ai_model, activity_type, analysis_revision, source_window_started_at, source_window_ended_at, selected_observation_count, available_observation_count, current_goal, status_summary, achievements, blockers, next_steps, validation_summary, handoff, git_summary, snapshot_basis_version, session_basis_id, session_basis_version, working_copy_id, content_hash, generated_at, updated_at, version, confirmed_snapshot_id, confirmed_session_id";

#[async_trait]
impl ProjectActivityRepository for SqliteProjectRepository {
    async fn get_codex_connector_settings(
        &self,
        workspace_id: &str,
    ) -> Result<Option<CodexConnectorSettings>, RepositoryError> {
        sqlx::query_as::<_, ConnectorRow>("SELECT workspace_id, executable_path, last_detected_version, last_health_check_at, version, created_at, updated_at FROM codex_connector_settings WHERE workspace_id = ?")
            .bind(workspace_id).fetch_optional(&self.pool).await.map_err(storage)?
            .map(ConnectorRow::into_domain).transpose()
    }

    async fn upsert_codex_connector_settings(
        &self,
        settings: &CodexConnectorSettings,
        expected_version: Option<i64>,
    ) -> Result<(), RepositoryError> {
        let result = if let Some(expected) = expected_version {
            sqlx::query("UPDATE codex_connector_settings SET executable_path = ?, last_detected_version = ?, last_health_check_at = ?, version = ?, updated_at = ? WHERE workspace_id = ? AND version = ?")
                .bind(&settings.executable_path).bind(&settings.last_detected_version).bind(settings.last_health_check_at.map(time_string)).bind(settings.version).bind(time_string(settings.updated_at)).bind(&settings.workspace_id).bind(expected).execute(&self.pool).await.map_err(write_error)?
        } else {
            sqlx::query("INSERT INTO codex_connector_settings (workspace_id, executable_path, last_detected_version, last_health_check_at, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(&settings.workspace_id).bind(&settings.executable_path).bind(&settings.last_detected_version).bind(settings.last_health_check_at.map(time_string)).bind(settings.version).bind(time_string(settings.created_at)).bind(time_string(settings.updated_at)).execute(&self.pool).await.map_err(write_error)?
        };
        affected(result.rows_affected(), expected_version.is_some())
    }

    async fn get_project_activity_settings(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectActivitySettings>, RepositoryError> {
        sqlx::query_as::<_, ActivitySettingsRow>("SELECT project_id, sync_mode, ai_enabled, ai_provider_id, initial_backfill_completed, version, created_at, updated_at FROM project_activity_settings WHERE project_id = ?")
            .bind(project_id).fetch_optional(&self.pool).await.map_err(storage)?
            .map(ActivitySettingsRow::into_domain).transpose()
    }

    async fn get_latest_successful_activity_sync_at(
        &self,
        project_id: &str,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        let value = sqlx::query_scalar::<_, Option<String>>(
            "SELECT MAX(completed_at) FROM project_activity_sync_runs WHERE project_id = ? AND status IN ('succeeded', 'partial')",
        )
        .bind(project_id)
        .fetch_one(&self.pool)
        .await
        .map_err(storage)?;
        optional_timestamp("latest successful activity sync", value)
    }

    async fn upsert_project_activity_settings(
        &self,
        settings: &ProjectActivitySettings,
        expected_version: Option<i64>,
    ) -> Result<(), RepositoryError> {
        let result = if let Some(expected) = expected_version {
            sqlx::query("UPDATE project_activity_settings SET sync_mode = ?, ai_enabled = ?, ai_provider_id = ?, initial_backfill_completed = ?, version = ?, updated_at = ? WHERE project_id = ? AND version = ?")
                .bind(settings.sync_mode.as_str()).bind(i64::from(settings.ai_enabled)).bind(&settings.ai_provider_id).bind(i64::from(settings.initial_backfill_completed)).bind(settings.version).bind(time_string(settings.updated_at)).bind(&settings.project_id).bind(expected).execute(&self.pool).await.map_err(write_error)?
        } else {
            sqlx::query("INSERT INTO project_activity_settings (project_id, sync_mode, ai_enabled, ai_provider_id, initial_backfill_completed, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&settings.project_id).bind(settings.sync_mode.as_str()).bind(i64::from(settings.ai_enabled)).bind(&settings.ai_provider_id).bind(i64::from(settings.initial_backfill_completed)).bind(settings.version).bind(time_string(settings.created_at)).bind(time_string(settings.updated_at)).execute(&self.pool).await.map_err(write_error)?
        };
        affected(result.rows_affected(), expected_version.is_some())
    }

    async fn insert_activity_sync_run(&self, run: &ActivitySyncRun) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO project_activity_sync_runs (id, project_id, trigger, status, stage, threads_scanned, turns_imported, observations_created, commits_imported, ai_invoked, started_at, completed_at, error_code, error_summary) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&run.id).bind(&run.project_id).bind(run.trigger.as_str()).bind(run.status.as_str()).bind(run.stage.as_str()).bind(run.threads_scanned).bind(run.turns_imported).bind(run.observations_created).bind(run.commits_imported).bind(i64::from(run.ai_invoked)).bind(time_string(run.started_at)).bind(run.completed_at.map(time_string)).bind(&run.error_code).bind(&run.error_summary).execute(&self.pool).await.map_err(write_error)?;
        Ok(())
    }

    async fn update_activity_sync_run(&self, run: &ActivitySyncRun) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE project_activity_sync_runs SET status = ?, stage = ?, threads_scanned = ?, turns_imported = ?, observations_created = ?, commits_imported = ?, ai_invoked = ?, completed_at = ?, error_code = ?, error_summary = ? WHERE id = ? AND project_id = ?")
            .bind(run.status.as_str()).bind(run.stage.as_str()).bind(run.threads_scanned).bind(run.turns_imported).bind(run.observations_created).bind(run.commits_imported).bind(i64::from(run.ai_invoked)).bind(run.completed_at.map(time_string)).bind(&run.error_code).bind(&run.error_summary).bind(&run.id).bind(&run.project_id).execute(&self.pool).await.map_err(storage)?;
        affected(result.rows_affected(), true)
    }

    async fn get_activity_sync_run(
        &self,
        run_id: &str,
    ) -> Result<Option<ActivitySyncRun>, RepositoryError> {
        let query =
            format!("SELECT {SYNC_RUN_COLUMNS} FROM project_activity_sync_runs WHERE id = ?");
        sqlx::query_as::<_, SyncRunRow>(&query)
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(SyncRunRow::into_domain)
            .transpose()
    }

    async fn list_activity_sync_runs(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<ActivitySyncRun>, RepositoryError> {
        let query = format!("SELECT {SYNC_RUN_COLUMNS} FROM project_activity_sync_runs WHERE project_id = ? ORDER BY started_at DESC, id DESC LIMIT ?");
        sqlx::query_as::<_, SyncRunRow>(&query)
            .bind(project_id)
            .bind(limit.clamp(1, 100))
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(SyncRunRow::into_domain)
            .collect()
    }

    async fn recover_interrupted_sync_runs(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query("UPDATE project_activity_sync_runs SET status = 'failed', stage = 'persisting', completed_at = ?, error_code = 'interrupted', error_summary = 'ForgeOS 上次运行在同步完成前结束。' WHERE status IN ('queued', 'running')")
            .bind(time_string(now)).execute(&self.pool).await.map_err(storage)?;
        Ok(result.rows_affected())
    }

    async fn get_codex_sync_cursor(
        &self,
        repository_id: &str,
    ) -> Result<Option<CodexSyncCursor>, RepositoryError> {
        sqlx::query_as::<_, CursorRow>("SELECT repository_id, project_id, last_thread_updated_at, last_success_at, version FROM codex_sync_cursors WHERE repository_id = ?")
            .bind(repository_id).fetch_optional(&self.pool).await.map_err(storage)?.map(CursorRow::into_domain).transpose()
    }

    async fn upsert_codex_sync_cursor(
        &self,
        cursor: &CodexSyncCursor,
    ) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO codex_sync_cursors (repository_id, project_id, last_thread_updated_at, last_success_at, version) VALUES (?, ?, ?, ?, ?) ON CONFLICT(repository_id) DO UPDATE SET last_thread_updated_at = excluded.last_thread_updated_at, last_success_at = excluded.last_success_at, version = codex_sync_cursors.version + 1 WHERE codex_sync_cursors.project_id = excluded.project_id")
            .bind(&cursor.repository_id).bind(&cursor.project_id).bind(cursor.last_thread_updated_at.map(time_string)).bind(cursor.last_success_at.map(time_string)).bind(cursor.version.max(1)).execute(&self.pool).await.map_err(write_error)?;
        Ok(())
    }

    async fn insert_codex_observation(
        &self,
        observation: &CodexActivityObservation,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query("INSERT INTO codex_activity_observations (id, project_id, repository_id, thread_id, turn_id, source_kind, turn_status, goal_hint, activity_summary, outcome_summary, validation_summary, handoff_summary, blockers, source_updated_at, source_hash, observed_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(repository_id, thread_id, turn_id) DO NOTHING")
            .bind(&observation.id).bind(&observation.project_id).bind(&observation.repository_id).bind(&observation.thread_id).bind(&observation.turn_id).bind(&observation.source_kind).bind(&observation.turn_status).bind(&observation.goal_hint).bind(&observation.activity_summary).bind(&observation.outcome_summary).bind(&observation.validation_summary).bind(&observation.handoff_summary).bind(encode_list(&observation.blockers)?).bind(time_string(observation.source_updated_at)).bind(&observation.source_hash).bind(time_string(observation.observed_at)).execute(&self.pool).await.map_err(write_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn list_codex_observations(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<CodexActivityObservation>, RepositoryError> {
        let query = format!("SELECT {OBSERVATION_COLUMNS} FROM codex_activity_observations WHERE project_id = ? ORDER BY source_updated_at DESC, id DESC LIMIT ?");
        sqlx::query_as::<_, ObservationRow>(&query)
            .bind(project_id)
            .bind(limit.clamp(1, 500))
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(ObservationRow::into_domain)
            .collect()
    }

    async fn insert_project_pulse(
        &self,
        pulse: &ProjectPulse,
        observation_ids: &[String],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        sqlx::query("UPDATE project_pulses SET status = 'superseded', version = version + 1, updated_at = ? WHERE project_id = ? AND status = 'pending_review'")
            .bind(time_string(pulse.generated_at)).bind(&pulse.project_id).execute(&mut *transaction).await.map_err(storage)?;
        insert_pulse(&mut transaction, pulse).await?;
        for (order, observation_id) in observation_ids.iter().enumerate() {
            sqlx::query("INSERT INTO project_pulse_sources (pulse_id, project_id, observation_id, source_order) VALUES (?, ?, ?, ?)")
                .bind(&pulse.id).bind(&pulse.project_id).bind(observation_id).bind(order as i64).execute(&mut *transaction).await.map_err(write_error)?;
        }
        transaction.commit().await.map_err(storage)
    }

    async fn get_latest_project_pulse(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectPulse>, RepositoryError> {
        let query = format!("SELECT {PULSE_COLUMNS} FROM project_pulses WHERE project_id = ? AND status IN ('pending_review', 'confirmed') ORDER BY generated_at DESC, id DESC LIMIT 1");
        sqlx::query_as::<_, PulseRow>(&query)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(PulseRow::into_domain)
            .transpose()
    }

    async fn list_project_pulses(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectPulse>, RepositoryError> {
        let query = format!("SELECT {PULSE_COLUMNS} FROM project_pulses WHERE project_id = ? ORDER BY generated_at DESC, id DESC");
        sqlx::query_as::<_, PulseRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(PulseRow::into_domain)
            .collect()
    }

    async fn list_project_pulse_sources(
        &self,
        pulse_id: &str,
    ) -> Result<Vec<CodexActivityObservation>, RepositoryError> {
        let query = format!("SELECT {} FROM codex_activity_observations o JOIN project_pulse_sources s ON s.observation_id = o.id AND s.project_id = o.project_id WHERE s.pulse_id = ? ORDER BY s.source_order", OBSERVATION_COLUMNS.split(", ").map(|field| format!("o.{field}")).collect::<Vec<_>>().join(", "));
        sqlx::query_as::<_, ObservationRow>(&query)
            .bind(pulse_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(ObservationRow::into_domain)
            .collect()
    }

    async fn update_project_pulse(
        &self,
        pulse: &ProjectPulse,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE project_pulses SET status = ?, current_goal = ?, status_summary = ?, achievements = ?, blockers = ?, next_steps = ?, validation_summary = ?, handoff = ?, git_summary = ?, content_hash = ?, updated_at = ?, version = ?, confirmed_snapshot_id = ?, confirmed_session_id = ? WHERE id = ? AND project_id = ? AND version = ?")
            .bind(pulse.status.as_str()).bind(&pulse.details.current_goal).bind(&pulse.details.status_summary).bind(encode_list(&pulse.details.achievements)?).bind(encode_list(&pulse.details.blockers)?).bind(encode_list(&pulse.details.next_steps)?).bind(&pulse.details.validation_summary).bind(&pulse.details.handoff).bind(&pulse.details.git_summary).bind(&pulse.content_hash).bind(time_string(pulse.updated_at)).bind(pulse.version).bind(&pulse.confirmed_snapshot_id).bind(&pulse.confirmed_session_id).bind(&pulse.id).bind(&pulse.project_id).bind(expected_version).execute(&self.pool).await.map_err(storage)?;
        affected(result.rows_affected(), true)
    }

    async fn confirm_project_pulse(
        &self,
        write: PulseConfirmationWrite,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let current_snapshot_version: Option<i64> =
            sqlx::query_scalar("SELECT MAX(version) FROM project_snapshots WHERE project_id = ?")
                .bind(&write.pulse.project_id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(storage)?;
        if current_snapshot_version != write.expected_snapshot_version {
            return Err(RepositoryError::OptimisticConflict);
        }
        let current_session_version: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM development_sessions WHERE project_id = ? AND status = 'open'",
        )
        .bind(&write.pulse.project_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(storage)?;
        if current_session_version != write.expected_session_version {
            return Err(RepositoryError::OptimisticConflict);
        }

        if let Some(snapshot) = &write.snapshot {
            sqlx::query("INSERT INTO project_snapshots (id, project_id, version, current_goal, status_summary, achievements, blockers, next_steps, confirmed_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(snapshot.id()).bind(snapshot.project_id()).bind(snapshot.version()).bind(snapshot.current_goal()).bind(snapshot.status_summary()).bind(encode_list(snapshot.achievements())?).bind(encode_list(snapshot.blockers())?).bind(encode_list(snapshot.next_steps())?).bind(time_string(snapshot.confirmed_at())).bind(time_string(snapshot.created_at())).bind(time_string(snapshot.updated_at())).execute(&mut *transaction).await.map_err(write_error)?;
        }
        if let Some(session) = &write.session {
            if write.session_is_new {
                insert_session(&mut transaction, session).await?;
            } else {
                let expected = write
                    .expected_session_version
                    .ok_or(RepositoryError::OptimisticConflict)?;
                let result = sqlx::query("UPDATE development_sessions SET status = ?, goal = ?, notes = ?, outcome = ?, validation = ?, handoff = ?, completed_at = ?, updated_at = ?, version = ? WHERE id = ? AND project_id = ? AND status = 'open' AND version = ?")
                    .bind(session.status().as_str()).bind(session.goal()).bind(session.notes()).bind(session.outcome()).bind(session.validation()).bind(session.handoff()).bind(session.completed_at().map(time_string)).bind(time_string(session.updated_at())).bind(session.version()).bind(session.id()).bind(session.project_id()).bind(expected).execute(&mut *transaction).await.map_err(storage)?;
                affected(result.rows_affected(), true)?;
            }
        }
        if let Some(achievement) = &write.achievement {
            insert_achievement_tx(&mut transaction, achievement, &write.achievement_sources)
                .await?;
        }
        let pulse_result = sqlx::query("UPDATE project_pulses SET status = 'confirmed', confirmed_snapshot_id = ?, confirmed_session_id = ?, updated_at = ?, version = ? WHERE id = ? AND project_id = ? AND status = 'pending_review' AND version = ?")
            .bind(&write.pulse.confirmed_snapshot_id).bind(&write.pulse.confirmed_session_id).bind(time_string(write.pulse.updated_at)).bind(write.pulse.version).bind(&write.pulse.id).bind(&write.pulse.project_id).bind(write.expected_pulse_version).execute(&mut *transaction).await.map_err(storage)?;
        affected(pulse_result.rows_affected(), true)?;
        transaction.commit().await.map_err(storage)
    }
}

async fn insert_pulse(
    transaction: &mut Transaction<'_, Sqlite>,
    pulse: &ProjectPulse,
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO project_pulses (id, project_id, status, analysis_mode, ai_provider_id, ai_model, activity_type, analysis_revision, source_window_started_at, source_window_ended_at, selected_observation_count, available_observation_count, current_goal, status_summary, achievements, blockers, next_steps, validation_summary, handoff, git_summary, snapshot_basis_version, session_basis_id, session_basis_version, working_copy_id, content_hash, generated_at, updated_at, version, confirmed_snapshot_id, confirmed_session_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&pulse.id).bind(&pulse.project_id).bind(pulse.status.as_str()).bind(pulse.analysis_mode.as_str()).bind(&pulse.ai_provider_id).bind(&pulse.ai_model).bind(pulse.quality.activity_type.as_str()).bind(pulse.quality.analysis_revision.as_str()).bind(pulse.quality.source_window_started_at.map(time_string)).bind(pulse.quality.source_window_ended_at.map(time_string)).bind(pulse.quality.selected_observation_count).bind(pulse.quality.available_observation_count).bind(&pulse.details.current_goal).bind(&pulse.details.status_summary).bind(encode_list(&pulse.details.achievements)?).bind(encode_list(&pulse.details.blockers)?).bind(encode_list(&pulse.details.next_steps)?).bind(&pulse.details.validation_summary).bind(&pulse.details.handoff).bind(&pulse.details.git_summary).bind(pulse.snapshot_basis_version).bind(&pulse.session_basis_id).bind(pulse.session_basis_version).bind(&pulse.working_copy_id).bind(&pulse.content_hash).bind(time_string(pulse.generated_at)).bind(time_string(pulse.updated_at)).bind(pulse.version).bind(&pulse.confirmed_snapshot_id).bind(&pulse.confirmed_session_id).execute(&mut **transaction).await.map_err(write_error)?;
    Ok(())
}

async fn insert_session(
    transaction: &mut Transaction<'_, Sqlite>,
    session: &crate::domain::DevelopmentSession,
) -> Result<(), RepositoryError> {
    sqlx::query("INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, validation, handoff, started_at, completed_at, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(session.id()).bind(session.project_id()).bind(session.status().as_str()).bind(session.goal()).bind(session.notes()).bind(session.outcome()).bind(session.validation()).bind(session.handoff()).bind(time_string(session.started_at())).bind(session.completed_at().map(time_string)).bind(time_string(session.created_at())).bind(time_string(session.updated_at())).bind(session.version()).execute(&mut **transaction).await.map_err(write_error)?;
    Ok(())
}

fn affected(rows: u64, optimistic: bool) -> Result<(), RepositoryError> {
    if rows == 1 {
        Ok(())
    } else if optimistic {
        Err(RepositoryError::OptimisticConflict)
    } else {
        Err(RepositoryError::Storage("write affected no rows".into()))
    }
}
fn boolean(field: &str, value: i64) -> Result<bool, RepositoryError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(RepositoryError::CorruptData(format!(
            "{field} contains {other}"
        ))),
    }
}
fn time_string(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn timestamp_from(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}
fn optional_timestamp(
    field: &str,
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, RepositoryError> {
    value.map(|value| timestamp_from(field, &value)).transpose()
}
fn encode_list(values: &[String]) -> Result<String, RepositoryError> {
    serde_json::to_string(values).map_err(|error| RepositoryError::Storage(error.to_string()))
}
fn decode_list(field: &str, value: &str) -> Result<Vec<String>, RepositoryError> {
    serde_json::from_str(value)
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}
fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}
fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}
fn write_error(error: sqlx::Error) -> RepositoryError {
    if let sqlx::Error::Database(database) = &error {
        if database.is_unique_violation() || database.is_foreign_key_violation() {
            return RepositoryError::Conflict;
        }
    }
    storage(error)
}
