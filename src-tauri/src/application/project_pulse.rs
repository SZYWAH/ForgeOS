use std::{collections::HashSet, path::Path, str::FromStr, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AchievementCategory, AchievementDetails, AchievementSource, AchievementSourceMode,
        AchievementSourceType, ActivitySyncRun, CodexActivityObservation, CodexConnectorSettings,
        DevelopmentSession, DomainError, Project, ProjectAchievement, ProjectActivitySettings,
        ProjectPulse, ProjectPulseDetails, ProjectPulseQuality, ProjectSnapshot, PulseActivityType,
        PulseAnalysisMode, PulseAnalysisRevision, RepositoryCommitObservation,
        RepositoryHistoryCursor, SessionStatus, SnapshotDetails, SyncMode, SyncStage, SyncStatus,
        SyncTrigger, WorkingCopy,
    },
    ports::{
        redact_external_text, CodexActivitySource, CodexActivitySourceError, CodexSyncCursor,
        GitAdapter, ProjectPulseKernelRepository, PulseConfirmationWrite, RepositoryError,
    },
};

use super::activity_episode::{ActivityEpisode, ActivityEpisodeBuilder};
use super::ai_provider::{
    AiMessageInput, AiProviderError, AiProviderKernel, InvokeAiProviderInput,
};

const MAX_THREADS: usize = 50;
const MAX_TURNS: usize = 500;
const MAX_NORMALIZED_BYTES: usize = 2 * 1024 * 1024;
const RECENT_EPISODE_DAYS: i64 = 14;
const PULSE_QUALITY_ALGORITHM: &str = "episode-v2.1";

#[derive(Debug, Error)]
pub enum ProjectPulseError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before using Project Pulse")]
    WorkspaceNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Project Pulse not found")]
    PulseNotFound,
    #[error("Project has no registered local Repository")]
    RepositoryNotConnected,
    #[error("activity sync run not found")]
    SyncRunNotFound,
    #[error("activity settings version conflict")]
    VersionConflict,
    #[error("project state changed after this Pulse was generated")]
    BasisVersionConflict,
    #[error("Codex connector is unavailable: {0}")]
    CodexUnavailable(String),
    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct UpdateConnectorSettingsInput {
    pub executable_path: Option<String>,
    pub expected_version: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct UpdateActivitySettingsInput {
    pub sync_mode: String,
    pub ai_enabled: bool,
    pub ai_provider_id: Option<String>,
    pub expected_version: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct ConfirmPulseInput {
    pub expected_version: i64,
    pub update_snapshot: bool,
    pub apply_session: bool,
    pub complete_session: bool,
}

#[derive(Clone, Debug)]
pub struct ConfirmPulseResult {
    pub pulse: ProjectPulse,
    pub achievement_draft_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SyncOutcome {
    pub run: ActivitySyncRun,
    pub pulse: Option<ProjectPulse>,
}

#[derive(Clone)]
pub struct ProjectPulseKernel {
    repository: Arc<dyn ProjectPulseKernelRepository>,
    codex_source: Arc<dyn CodexActivitySource>,
    git: Arc<dyn GitAdapter>,
    ai: Arc<AiProviderKernel>,
}

impl ProjectPulseKernel {
    pub fn new(
        repository: Arc<dyn ProjectPulseKernelRepository>,
        codex_source: Arc<dyn CodexActivitySource>,
        git: Arc<dyn GitAdapter>,
        ai: Arc<AiProviderKernel>,
    ) -> Self {
        Self {
            repository,
            codex_source,
            git,
            ai,
        }
    }

    pub async fn recover_interrupted_runs(&self) -> Result<u64, ProjectPulseError> {
        self.repository
            .recover_interrupted_sync_runs(Utc::now())
            .await
            .map_err(map_repository_error)
    }

    pub async fn get_connector_settings(
        &self,
    ) -> Result<CodexConnectorSettings, ProjectPulseError> {
        let workspace = self.require_workspace().await?;
        Ok(self
            .repository
            .get_codex_connector_settings(&workspace.id)
            .await
            .map_err(map_repository_error)?
            .unwrap_or(CodexConnectorSettings::new(workspace.id, Utc::now())?))
    }

    pub async fn check_connector(&self) -> Result<CodexConnectorSettings, ProjectPulseError> {
        let mut settings = self.get_connector_settings().await?;
        let health = self
            .codex_source
            .health_check(settings.executable_path.as_deref())
            .await
            .map_err(map_codex_error)?;
        let expected = self
            .repository
            .get_codex_connector_settings(&settings.workspace_id)
            .await
            .map_err(map_repository_error)?
            .map(|value| value.version);
        settings.record_health(
            health.executable_path,
            health.version.unwrap_or_default(),
            Utc::now(),
        )?;
        self.repository
            .upsert_codex_connector_settings(&settings, expected)
            .await
            .map_err(map_repository_error)?;
        Ok(settings)
    }

    pub async fn update_connector_settings(
        &self,
        input: UpdateConnectorSettingsInput,
    ) -> Result<CodexConnectorSettings, ProjectPulseError> {
        let mut settings = self.get_connector_settings().await?;
        let persisted = self
            .repository
            .get_codex_connector_settings(&settings.workspace_id)
            .await
            .map_err(map_repository_error)?;
        if let (Some(expected), Some(current)) = (input.expected_version, persisted.as_ref()) {
            if expected != current.version {
                return Err(ProjectPulseError::VersionConflict);
            }
        }
        settings.update_path(input.executable_path, Utc::now())?;
        self.repository
            .upsert_codex_connector_settings(&settings, persisted.map(|value| value.version))
            .await
            .map_err(map_repository_error)?;
        self.check_connector().await
    }

    pub async fn get_activity_settings(
        &self,
        project_id: &str,
    ) -> Result<ProjectActivitySettings, ProjectPulseError> {
        self.require_project(project_id).await?;
        Ok(self
            .repository
            .get_project_activity_settings(project_id)
            .await
            .map_err(map_repository_error)?
            .unwrap_or(ProjectActivitySettings::new(
                project_id.to_owned(),
                Utc::now(),
            )?))
    }

    pub async fn update_activity_settings(
        &self,
        project_id: &str,
        input: UpdateActivitySettingsInput,
    ) -> Result<ProjectActivitySettings, ProjectPulseError> {
        self.require_project(project_id).await?;
        let persisted = self
            .repository
            .get_project_activity_settings(project_id)
            .await
            .map_err(map_repository_error)?;
        if let (Some(expected), Some(current)) = (input.expected_version, persisted.as_ref()) {
            if expected != current.version {
                return Err(ProjectPulseError::VersionConflict);
            }
        }
        if input.ai_enabled {
            let provider_id = input
                .ai_provider_id
                .as_deref()
                .ok_or(DomainError::ProjectActivityAiProviderRequired)?;
            let provider = self
                .repository
                .get_ai_provider(provider_id)
                .await
                .map_err(map_repository_error)?
                .ok_or(DomainError::ProjectActivityAiProviderRequired)?;
            provider.ensure_enabled()?;
        }
        let mut settings = persisted.clone().unwrap_or(ProjectActivitySettings::new(
            project_id.to_owned(),
            Utc::now(),
        )?);
        settings.update(
            SyncMode::from_str(&input.sync_mode)?,
            input.ai_enabled,
            input.ai_provider_id,
            Utc::now(),
        )?;
        self.repository
            .upsert_project_activity_settings(&settings, persisted.map(|value| value.version))
            .await
            .map_err(map_repository_error)?;
        Ok(settings)
    }

    pub async fn queue_sync(
        &self,
        project_id: &str,
        trigger: SyncTrigger,
    ) -> Result<ActivitySyncRun, ProjectPulseError> {
        self.require_project(project_id).await?;
        if let Some(recent) = self
            .repository
            .list_activity_sync_runs(project_id, 1)
            .await
            .map_err(map_repository_error)?
            .first()
        {
            if matches!(recent.status, SyncStatus::Queued | SyncStatus::Running) {
                return Ok(recent.clone());
            }
            if trigger == SyncTrigger::ProjectOpen {
                if Utc::now() - recent.started_at < Duration::minutes(5) {
                    return Ok(recent.clone());
                }
            }
        }
        let run = ActivitySyncRun::queued(
            Uuid::now_v7().to_string(),
            project_id.to_owned(),
            trigger,
            Utc::now(),
        )?;
        self.repository
            .insert_activity_sync_run(&run)
            .await
            .map_err(map_repository_error)?;
        Ok(run)
    }

    pub async fn execute_sync<F>(
        &self,
        run_id: &str,
        mut notify: F,
    ) -> Result<SyncOutcome, ProjectPulseError>
    where
        F: FnMut(&ActivitySyncRun) + Send,
    {
        let mut run = self.get_sync_run(run_id).await?;
        if run.status != SyncStatus::Queued {
            return Ok(SyncOutcome { run, pulse: None });
        }
        run.status = SyncStatus::Running;
        self.persist_run(&run, &mut notify).await?;
        let result = self.execute_sync_inner(&mut run, &mut notify).await;
        match result {
            Ok(pulse) => Ok(SyncOutcome { run, pulse }),
            Err(error) => {
                run.status = SyncStatus::Failed;
                run.completed_at = Some(Utc::now());
                run.error_code = Some(error_code(&error).into());
                run.error_summary = Some(
                    redact_external_text(&error.to_string())
                        .chars()
                        .take(2_000)
                        .collect(),
                );
                let _ = self.persist_run(&run, &mut notify).await;
                Err(error)
            }
        }
    }

    async fn execute_sync_inner<F>(
        &self,
        run: &mut ActivitySyncRun,
        notify: &mut F,
    ) -> Result<Option<ProjectPulse>, ProjectPulseError>
    where
        F: FnMut(&ActivitySyncRun) + Send,
    {
        let project = self.require_project(&run.project_id).await?;
        let repositories = self
            .repository
            .list_repositories(&run.project_id)
            .await
            .map_err(map_repository_error)?;
        if repositories.is_empty() {
            return Err(ProjectPulseError::RepositoryNotConnected);
        }
        let mut partial_messages = Vec::new();
        let mut codex_backfill_complete = true;
        let connector = match self.check_connector().await {
            Ok(settings) => Some(settings),
            Err(ProjectPulseError::CodexUnavailable(message)) => {
                codex_backfill_complete = false;
                partial_messages.push(format!("Codex 连接不可用: {message}"));
                None
            }
            Err(error) => return Err(error),
        };

        run.stage = SyncStage::ReadingCodex;
        self.persist_run(run, notify).await?;
        for repository in connector.iter().flat_map(|_| repositories.iter()) {
            let cursor = self
                .repository
                .get_codex_sync_cursor(repository.id())
                .await
                .map_err(map_repository_error)?;
            let after = cursor
                .as_ref()
                .and_then(|value| value.last_thread_updated_at)
                .or_else(|| Some(Utc::now() - Duration::days(30)));
            match self
                .codex_source
                .read_repository_activity(
                    connector
                        .as_ref()
                        .and_then(|settings| settings.executable_path.as_deref()),
                    Path::new(repository.path()),
                    after,
                    MAX_THREADS,
                    MAX_TURNS,
                    MAX_NORMALIZED_BYTES,
                )
                .await
            {
                Ok(batch) => {
                    run.threads_scanned += batch.threads_scanned as i64;
                    run.turns_imported += batch.turns_seen as i64;
                    for activity in batch.activities {
                        let source_hash = stable_hash(
                            &json!({ "thread": activity.thread_id, "turn": activity.turn_id, "status": activity.status, "goal": activity.goal_hint, "activity": activity.activity_summary, "outcome": activity.outcome_summary, "validation": activity.validation_summary, "handoff": activity.handoff_summary, "blockers": activity.blockers, "updated": activity.source_updated_at }),
                        );
                        let observation = CodexActivityObservation::create(
                            Uuid::now_v7().to_string(),
                            run.project_id.clone(),
                            repository.id().to_owned(),
                            activity.thread_id,
                            activity.turn_id,
                            activity.source_kind,
                            activity.status,
                            activity.goal_hint,
                            activity.activity_summary,
                            activity.outcome_summary,
                            activity.validation_summary,
                            activity.handoff_summary,
                            activity.blockers,
                            activity.source_updated_at,
                            source_hash,
                            Utc::now(),
                        )?;
                        if self
                            .repository
                            .insert_codex_observation(&observation)
                            .await
                            .map_err(map_repository_error)?
                        {
                            run.observations_created += 1;
                        }
                    }
                    self.repository
                        .upsert_codex_sync_cursor(&CodexSyncCursor {
                            repository_id: repository.id().to_owned(),
                            project_id: run.project_id.clone(),
                            last_thread_updated_at: batch.latest_thread_updated_at.or(after),
                            last_success_at: Some(Utc::now()),
                            version: cursor.map_or(1, |value| value.version + 1),
                        })
                        .await
                        .map_err(map_repository_error)?;
                    if let Some(reason) = batch.partial_reason {
                        codex_backfill_complete = false;
                        partial_messages.push(reason);
                    }
                }
                Err(error) => {
                    codex_backfill_complete = false;
                    partial_messages.push(format!("Codex 读取失败: {}", map_codex_error(error)));
                }
            }
        }

        run.stage = SyncStage::ReadingGit;
        self.persist_run(run, notify).await?;
        let mut latest_working_copies = Vec::new();
        for mut repository in repositories {
            match self.git.inspect(repository.path()).await {
                Ok(inspection) => {
                    let history_cursor = self
                        .repository
                        .get_repository_history_cursor(repository.id())
                        .await
                        .map_err(map_repository_error)?;
                    match self
                        .git
                        .list_commit_history(
                            repository.path(),
                            history_cursor
                                .as_ref()
                                .and_then(|value| value.last_scanned_head_commit.as_deref()),
                            Utc::now() - Duration::days(365),
                            2_000,
                        )
                        .await
                    {
                        Ok(history) => {
                            for commit in history.commits {
                                let source_hash = stable_hash(&json!({
                                    "sha": commit.sha,
                                    "message": commit.message,
                                    "author": commit.author_name,
                                    "authored_at": commit.authored_at,
                                }));
                                let observation = RepositoryCommitObservation::create(
                                    Uuid::now_v7().to_string(),
                                    repository.id().to_owned(),
                                    run.project_id.clone(),
                                    commit.sha,
                                    commit.message,
                                    commit.author_name,
                                    commit.authored_at,
                                    Utc::now(),
                                    source_hash,
                                )?;
                                if self
                                    .repository
                                    .insert_repository_commit_observation(&observation)
                                    .await
                                    .map_err(map_repository_error)?
                                {
                                    run.commits_imported += 1;
                                }
                            }
                            self.repository
                                .upsert_repository_history_cursor(&RepositoryHistoryCursor {
                                    repository_id: repository.id().to_owned(),
                                    project_id: run.project_id.clone(),
                                    initial_backfill_completed: true,
                                    last_scanned_head_commit: Some(history.head_commit),
                                    last_success_at: Some(Utc::now()),
                                    version: history_cursor.map_or(1, |value| value.version + 1),
                                })
                                .await
                                .map_err(map_repository_error)?;
                        }
                        Err(error) => partial_messages.push(format!(
                            "Git 历史读取失败: {}",
                            redact_external_text(&error.to_string())
                        )),
                    }
                    let expected = repository.version();
                    repository.refresh(
                        inspection.remote_url.clone(),
                        inspection.default_branch.clone(),
                        inspection.checked_at,
                    )?;
                    let working_copy = WorkingCopy::create(
                        Uuid::now_v7().to_string(),
                        repository.id().to_owned(),
                        inspection.branch,
                        inspection.head_commit,
                        inspection.is_dirty,
                        inspection.diff_summary,
                        inspection.upstream_branch,
                        inspection.ahead_count,
                        inspection.behind_count,
                        inspection.checked_at,
                    )?;
                    self.repository
                        .update_repository_with_working_copy(&repository, expected, &working_copy)
                        .await
                        .map_err(map_repository_error)?;
                    latest_working_copies.push(working_copy);
                }
                Err(error) => partial_messages.push(format!(
                    "Git 检查失败: {}",
                    redact_external_text(&error.to_string())
                )),
            }
        }

        run.stage = SyncStage::Analyzing;
        self.persist_run(run, notify).await?;
        let observations = self
            .repository
            .list_codex_observations(&run.project_id, MAX_TURNS as i64)
            .await
            .map_err(map_repository_error)?;
        let snapshot = self
            .repository
            .get_current_snapshot(&run.project_id)
            .await
            .map_err(map_repository_error)?;
        let session = self
            .repository
            .get_current_session(&run.project_id)
            .await
            .map_err(map_repository_error)?;
        let pulse_history = self
            .repository
            .list_project_pulses(&run.project_id)
            .await
            .map_err(map_repository_error)?;
        let working_set = PulseWorkingSetSelector::select(&observations, Utc::now());
        let snapshot_is_legacy = snapshot.as_ref().is_some_and(|value| {
            pulse_history.iter().any(|pulse| {
                pulse.quality.analysis_revision == PulseAnalysisRevision::LegacyV1
                    && pulse.confirmed_snapshot_id.as_deref() == Some(value.id())
            })
        });
        let session_is_legacy = session.as_ref().is_some_and(|value| {
            pulse_history.iter().any(|pulse| {
                pulse.quality.analysis_revision == PulseAnalysisRevision::LegacyV1
                    && pulse.confirmed_session_id.as_deref() == Some(value.id())
            })
        });
        let trusted_snapshot = (!snapshot_is_legacy).then_some(snapshot.as_ref()).flatten();
        let trusted_session = (!session_is_legacy).then_some(session.as_ref()).flatten();
        let rules = build_rule_pulse(
            &project,
            &working_set,
            trusted_snapshot,
            trusted_session,
            &latest_working_copies,
        );
        let mut settings = self.get_activity_settings(&run.project_id).await?;
        let persisted_settings = self
            .repository
            .get_project_activity_settings(&run.project_id)
            .await
            .map_err(map_repository_error)?;
        let fingerprint_revision = if settings.ai_enabled {
            PulseAnalysisRevision::AiV2
        } else {
            PulseAnalysisRevision::RulesV2
        };
        let source_fingerprint = stable_hash(&json!({
            "analysis_revision": fingerprint_revision.as_str(),
            "quality_algorithm": PULSE_QUALITY_ALGORITHM,
            "observations": working_set.selected.iter().map(|value| &value.source_hash).collect::<Vec<_>>(),
            "snapshot": trusted_snapshot.map(|value| value.version()),
            "session": trusted_session.map(|value| (value.id(), value.version())),
            "working": latest_working_copies.iter().map(|value| (value.repository_id(), value.head_commit(), value.is_dirty(), value.diff_summary().changed_files, value.upstream_branch(), value.ahead_count(), value.behind_count())).collect::<Vec<_>>()
        }));
        if pulse_history
            .first()
            .is_some_and(|pulse| pulse.content_hash == source_fingerprint)
        {
            run.status = if partial_messages.is_empty() {
                SyncStatus::Succeeded
            } else {
                SyncStatus::Partial
            };
            run.stage = SyncStage::Persisting;
            run.completed_at = Some(Utc::now());
            run.error_code = (!partial_messages.is_empty()).then(|| "partial".into());
            run.error_summary = (!partial_messages.is_empty())
                .then(|| redact_external_text(&partial_messages.join("\n")));
            self.persist_run(run, notify).await?;
            return Ok(None);
        }

        if codex_backfill_complete && !settings.initial_backfill_completed {
            settings.mark_backfill_completed(Utc::now())?;
            self.repository
                .upsert_project_activity_settings(
                    &settings,
                    persisted_settings.as_ref().map(|value| value.version),
                )
                .await
                .map_err(map_repository_error)?;
        } else if persisted_settings.is_none() {
            self.repository
                .upsert_project_activity_settings(&settings, None)
                .await
                .map_err(map_repository_error)?;
        }
        let mut details = rules.details;
        let mut activity_type = rules.activity_type;
        let mut analysis_mode = PulseAnalysisMode::Rules;
        let mut analysis_revision = PulseAnalysisRevision::RulesV2;
        let mut ai_provider_id = None;
        let mut ai_model = None;
        if settings.ai_enabled {
            let provider_id = settings
                .ai_provider_id
                .as_deref()
                .ok_or(DomainError::ProjectActivityAiProviderRequired)?;
            run.ai_invoked = true;
            match self
                .analyze_with_ai(provider_id, activity_type, &details, &working_set)
                .await
            {
                Ok((ai_activity_type, ai_details, model)) => {
                    activity_type = ai_activity_type;
                    details = ai_details;
                    analysis_mode = PulseAnalysisMode::Ai;
                    analysis_revision = PulseAnalysisRevision::AiV2;
                    ai_provider_id = Some(provider_id.to_owned());
                    ai_model = Some(model);
                }
                Err(error) => {
                    partial_messages.push(format!("AI 分析失败，已使用规则摘要: {error}"))
                }
            }
        }

        run.stage = SyncStage::Persisting;
        self.persist_run(run, notify).await?;
        let pulse = ProjectPulse::create(
            Uuid::now_v7().to_string(),
            run.project_id.clone(),
            analysis_mode,
            ai_provider_id,
            ai_model,
            ProjectPulseQuality::new(
                activity_type,
                analysis_revision,
                working_set.source_window_started_at,
                working_set.source_window_ended_at,
                working_set.selected.len() as i64,
                observations.len() as i64,
            )?,
            details,
            snapshot.as_ref().map(|value| value.version()),
            session.as_ref().map(|value| value.id().to_owned()),
            session.as_ref().map(|value| value.version()),
            latest_working_copies
                .first()
                .map(|value| value.id().to_owned()),
            source_fingerprint,
            Utc::now(),
        )?;
        let source_ids = working_set
            .selected
            .iter()
            .map(|value| value.id.clone())
            .collect::<Vec<_>>();
        self.repository
            .insert_project_pulse(&pulse, &source_ids)
            .await
            .map_err(map_repository_error)?;
        run.status = if partial_messages.is_empty() {
            SyncStatus::Succeeded
        } else {
            SyncStatus::Partial
        };
        run.completed_at = Some(Utc::now());
        run.error_code = (!partial_messages.is_empty()).then(|| "partial".into());
        run.error_summary = (!partial_messages.is_empty())
            .then(|| redact_external_text(&partial_messages.join("\n")));
        self.persist_run(run, notify).await?;
        Ok(Some(pulse))
    }

    async fn analyze_with_ai(
        &self,
        provider_id: &str,
        activity_type: PulseActivityType,
        rules: &ProjectPulseDetails,
        working_set: &PulseWorkingSet,
    ) -> Result<(PulseActivityType, ProjectPulseDetails, String), AiProviderError> {
        let input = json!({
            "rules_v2": rules_to_json(activity_type, rules),
            "current_episode": working_set.current.as_ref().map(ActivityEpisode::to_summary_json),
            "recent_completed_episodes": working_set.recent_completed.iter().map(ActivityEpisode::to_summary_json).collect::<Vec<_>>(),
            "observations": working_set.selected.iter().take(20).map(observation_to_json).collect::<Vec<_>>()
        });
        let system = "你是 ForgeOS Project Pulse 分析器。只能基于输入的 Working Set 整理草稿，不得引用输入之外的历史或创造事实。只返回 JSON 对象，字段必须为 activity_type,current_goal,status_summary,achievements,blockers,next_steps,validation_summary,handoff。activity_type 只能是 implementation、debugging、release、review、support_question、unknown；achievements 最多 3 项，next_steps 最多 3 项。";
        let invoke = |content: String| {
            self.ai.invoke_provider(
                provider_id,
                InvokeAiProviderInput {
                    messages: vec![
                        AiMessageInput {
                            role: "system".into(),
                            content: system.into(),
                        },
                        AiMessageInput {
                            role: "user".into(),
                            content,
                        },
                    ],
                },
            )
        };
        let first = invoke(input.to_string()).await?;
        if let Ok((activity_type, details)) =
            parse_ai_pulse(&first.completion.content, rules.git_summary.clone())
        {
            return Ok((activity_type, details, first.completion.model));
        }
        let repair = invoke(format!(
            "下面的响应不是合法的目标 JSON。请只修复格式，不添加新事实：\n{}",
            first.completion.content
        ))
        .await?;
        let (activity_type, details) =
            parse_ai_pulse(&repair.completion.content, rules.git_summary.clone())
                .map_err(|error| AiProviderError::Invocation(error.to_string()))?;
        Ok((activity_type, details, repair.completion.model))
    }

    async fn persist_run<F>(
        &self,
        run: &ActivitySyncRun,
        notify: &mut F,
    ) -> Result<(), ProjectPulseError>
    where
        F: FnMut(&ActivitySyncRun) + Send,
    {
        self.repository
            .update_activity_sync_run(run)
            .await
            .map_err(map_repository_error)?;
        notify(run);
        Ok(())
    }

    pub async fn get_sync_run(&self, run_id: &str) -> Result<ActivitySyncRun, ProjectPulseError> {
        self.repository
            .get_activity_sync_run(run_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ProjectPulseError::SyncRunNotFound)
    }
    pub async fn list_sync_runs(
        &self,
        project_id: &str,
    ) -> Result<Vec<ActivitySyncRun>, ProjectPulseError> {
        self.require_project(project_id).await?;
        self.repository
            .list_activity_sync_runs(project_id, 30)
            .await
            .map_err(map_repository_error)
    }
    pub async fn latest_successful_sync_at(
        &self,
        project_id: &str,
    ) -> Result<Option<chrono::DateTime<Utc>>, ProjectPulseError> {
        self.require_project(project_id).await?;
        self.repository
            .get_latest_successful_activity_sync_at(project_id)
            .await
            .map_err(map_repository_error)
    }
    pub async fn interval_projects_due(&self) -> Result<Vec<String>, ProjectPulseError> {
        let workspace = self.require_workspace().await?;
        let projects = self
            .repository
            .list_projects(&workspace.id)
            .await
            .map_err(map_repository_error)?;
        let mut due = Vec::new();
        for project in projects {
            let settings = self
                .repository
                .get_project_activity_settings(&project.id)
                .await
                .map_err(map_repository_error)?;
            if settings
                .as_ref()
                .is_none_or(|value| value.sync_mode != SyncMode::Interval30m)
            {
                continue;
            }
            let runs = self
                .repository
                .list_activity_sync_runs(&project.id, 1)
                .await
                .map_err(map_repository_error)?;
            let should_run = runs.first().is_none_or(|run| {
                !matches!(run.status, SyncStatus::Queued | SyncStatus::Running)
                    && Utc::now() - run.completed_at.unwrap_or(run.started_at)
                        >= Duration::minutes(30)
            });
            if should_run {
                due.push(project.id);
            }
        }
        Ok(due)
    }
    pub async fn get_latest_pulse(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectPulse>, ProjectPulseError> {
        self.require_project(project_id).await?;
        self.repository
            .get_latest_project_pulse(project_id)
            .await
            .map_err(map_repository_error)
    }
    pub async fn list_pulses(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectPulse>, ProjectPulseError> {
        self.require_project(project_id).await?;
        self.repository
            .list_project_pulses(project_id)
            .await
            .map_err(map_repository_error)
    }
    pub async fn get_pulse_sources(
        &self,
        project_id: &str,
        pulse_id: &str,
    ) -> Result<Vec<CodexActivityObservation>, ProjectPulseError> {
        let pulse = self
            .repository
            .list_project_pulses(project_id)
            .await
            .map_err(map_repository_error)?
            .into_iter()
            .find(|pulse| pulse.id == pulse_id)
            .ok_or(ProjectPulseError::PulseNotFound)?;
        pulse.ensure_project(project_id)?;
        self.repository
            .list_project_pulse_sources(pulse_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn update_pulse_draft(
        &self,
        project_id: &str,
        pulse_id: &str,
        details: ProjectPulseDetails,
        expected_version: i64,
    ) -> Result<ProjectPulse, ProjectPulseError> {
        let mut pulse = self.require_pulse(project_id, pulse_id).await?;
        if pulse.version != expected_version {
            return Err(ProjectPulseError::VersionConflict);
        }
        pulse.edit(details, Utc::now())?;
        self.repository
            .update_project_pulse(&pulse, expected_version)
            .await
            .map_err(map_repository_error)?;
        Ok(pulse)
    }

    pub async fn dismiss_pulse(
        &self,
        project_id: &str,
        pulse_id: &str,
        expected_version: i64,
    ) -> Result<ProjectPulse, ProjectPulseError> {
        let mut pulse = self.require_pulse(project_id, pulse_id).await?;
        if pulse.version != expected_version {
            return Err(ProjectPulseError::VersionConflict);
        }
        pulse.dismiss(Utc::now())?;
        self.repository
            .update_project_pulse(&pulse, expected_version)
            .await
            .map_err(map_repository_error)?;
        Ok(pulse)
    }

    pub async fn confirm_pulse(
        &self,
        project_id: &str,
        pulse_id: &str,
        input: ConfirmPulseInput,
    ) -> Result<ConfirmPulseResult, ProjectPulseError> {
        self.require_project(project_id).await?;
        let mut pulse = self.require_pulse(project_id, pulse_id).await?;
        if pulse.version != input.expected_version {
            return Err(ProjectPulseError::VersionConflict);
        }
        pulse.ensure_confirmable_quality()?;
        let current_snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let current_session = self
            .repository
            .get_current_session(project_id)
            .await
            .map_err(map_repository_error)?;
        if current_snapshot.as_ref().map(|value| value.version()) != pulse.snapshot_basis_version
            || current_session.as_ref().map(|value| value.version()) != pulse.session_basis_version
            || current_session.as_ref().map(|value| value.id()) != pulse.session_basis_id.as_deref()
        {
            return Err(ProjectPulseError::BasisVersionConflict);
        }
        let now = Utc::now();
        let snapshot = if input.update_snapshot {
            let details = SnapshotDetails {
                current_goal: pulse.details.current_goal.clone(),
                status_summary: pulse.details.status_summary.clone(),
                achievements: pulse.details.achievements.clone(),
                blockers: pulse.details.blockers.clone(),
                next_steps: pulse.details.next_steps.clone(),
            };
            Some(match &current_snapshot {
                Some(current) => ProjectSnapshot::next_version(
                    Uuid::now_v7().to_string(),
                    project_id.to_owned(),
                    current,
                    details,
                    now,
                )?,
                None => ProjectSnapshot::new(
                    Uuid::now_v7().to_string(),
                    project_id.to_owned(),
                    details,
                    now,
                )?,
            })
        } else {
            None
        };
        let (session, session_is_new) = if input.apply_session {
            match current_session.clone() {
                Some(mut session) => {
                    if input.complete_session {
                        if pulse.details.achievements.is_empty()
                            || pulse.details.validation_summary.trim().is_empty()
                            || pulse.details.handoff.trim().is_empty()
                        {
                            return Err(DomainError::ProjectPulseCompletionRequired.into());
                        }
                        session.complete(
                            pulse.details.achievements.join("\n"),
                            pulse.details.validation_summary.clone(),
                            pulse.details.handoff.clone(),
                            now,
                        )?;
                        (Some(session), false)
                    } else {
                        let notes = [session.notes(), &pulse_session_notes(&pulse)]
                            .into_iter()
                            .filter(|value| !value.trim().is_empty())
                            .collect::<Vec<_>>()
                            .join("\n\n");
                        session.update_open(pulse.details.current_goal.clone(), notes, now)?;
                        (Some(session), false)
                    }
                }
                None => {
                    if input.complete_session {
                        return Err(DomainError::ProjectPulseCompletionRequired.into());
                    }
                    let mut session = DevelopmentSession::start(
                        Uuid::now_v7().to_string(),
                        project_id.to_owned(),
                        pulse.details.current_goal.clone(),
                        now,
                    )?;
                    session.update_open(
                        pulse.details.current_goal.clone(),
                        pulse_session_notes(&pulse),
                        now,
                    )?;
                    (Some(session), true)
                }
            }
        } else {
            (None, false)
        };
        let snapshot_id = snapshot.as_ref().map(|value| value.id().to_owned());
        let session_id = session.as_ref().map(|value| value.id().to_owned());
        let pulse_sources = self
            .repository
            .list_project_pulse_sources(pulse_id)
            .await
            .map_err(map_repository_error)?;
        let achievement = build_achievement_draft(&pulse, now)?;
        let achievement_sources = achievement
            .as_ref()
            .map(|value| {
                let mut sources = vec![AchievementSource::create(
                    value.id.clone(),
                    project_id.into(),
                    AchievementSourceType::Pulse,
                    pulse.id.clone(),
                    0,
                )?];
                for observation in &pulse_sources {
                    sources.push(AchievementSource::create(
                        value.id.clone(),
                        project_id.into(),
                        AchievementSourceType::CodexObservation,
                        observation.id.clone(),
                        sources.len() as i64,
                    )?);
                }
                Ok::<_, DomainError>(sources)
            })
            .transpose()?
            .unwrap_or_default();
        let achievement_draft_id = achievement.as_ref().map(|value| value.id.clone());
        pulse.mark_confirmed(snapshot_id, session_id, now)?;
        self.repository
            .confirm_project_pulse(PulseConfirmationWrite {
                pulse: pulse.clone(),
                expected_pulse_version: input.expected_version,
                snapshot,
                expected_snapshot_version: current_snapshot.as_ref().map(|value| value.version()),
                session,
                expected_session_version: current_session.as_ref().map(|value| value.version()),
                session_is_new,
                achievement,
                achievement_sources,
            })
            .await
            .map_err(|error| match error {
                RepositoryError::OptimisticConflict => ProjectPulseError::BasisVersionConflict,
                other => map_repository_error(other),
            })?;
        Ok(ConfirmPulseResult {
            pulse,
            achievement_draft_id,
        })
    }

    async fn require_workspace(&self) -> Result<crate::domain::Workspace, ProjectPulseError> {
        self.repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(ProjectPulseError::WorkspaceNotFound)
    }
    async fn require_project(&self, project_id: &str) -> Result<Project, ProjectPulseError> {
        let workspace = self.require_workspace().await?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ProjectPulseError::ProjectNotFound)
    }
    async fn require_pulse(
        &self,
        project_id: &str,
        pulse_id: &str,
    ) -> Result<ProjectPulse, ProjectPulseError> {
        self.repository
            .list_project_pulses(project_id)
            .await
            .map_err(map_repository_error)?
            .into_iter()
            .find(|pulse| pulse.id == pulse_id)
            .ok_or(ProjectPulseError::PulseNotFound)
    }
}

fn pulse_session_notes(pulse: &ProjectPulse) -> String {
    [
        format!("Project Pulse {}", pulse.generated_at),
        pulse.details.status_summary.clone(),
        (!pulse.details.validation_summary.trim().is_empty())
            .then(|| format!("验证: {}", pulse.details.validation_summary))
            .unwrap_or_default(),
        (!pulse.details.handoff.trim().is_empty())
            .then(|| format!("接手: {}", pulse.details.handoff))
            .unwrap_or_default(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

fn build_achievement_draft(
    pulse: &ProjectPulse,
    now: DateTime<Utc>,
) -> Result<Option<ProjectAchievement>, DomainError> {
    let category = match pulse.quality.activity_type {
        PulseActivityType::Implementation => AchievementCategory::Feature,
        PulseActivityType::Debugging => AchievementCategory::ProblemSolution,
        PulseActivityType::Release => AchievementCategory::Release,
        PulseActivityType::Review
        | PulseActivityType::SupportQuestion
        | PulseActivityType::Unknown => return Ok(None),
    };
    if pulse.details.achievements.is_empty() {
        return Ok(None);
    }
    let title = pulse
        .details
        .achievements
        .first()
        .map(|value| truncate_chars(&clean_text(value), 200))
        .unwrap_or_default();
    let value = ProjectAchievement::draft(
        Uuid::now_v7().to_string(),
        pulse.project_id.clone(),
        AchievementDetails {
            category,
            title,
            problem: (category == AchievementCategory::ProblemSolution)
                .then(|| pulse.details.current_goal.clone())
                .unwrap_or_default(),
            approach: pulse.details.status_summary.clone(),
            result: pulse.details.achievements.join("\n"),
            validation: pulse.details.validation_summary.clone(),
            impact: String::new(),
            technologies: vec![],
            resume_bullet: String::new(),
            featured: false,
        },
        AchievementSourceMode::PulseRule,
        Some(pulse.id.clone()),
        now,
    )?;
    Ok(Some(value))
}

#[derive(Clone, Debug)]
struct PulseWorkingSet {
    current: Option<ActivityEpisode>,
    recent_completed: Vec<ActivityEpisode>,
    selected: Vec<CodexActivityObservation>,
    source_window_started_at: Option<DateTime<Utc>>,
    source_window_ended_at: Option<DateTime<Utc>>,
}

struct PulseWorkingSetSelector;

impl PulseWorkingSetSelector {
    fn select(observations: &[CodexActivityObservation], now: DateTime<Utc>) -> PulseWorkingSet {
        let episodes = ActivityEpisodeBuilder::build(observations);
        let current = episodes.last().cloned();
        let cutoff = now - Duration::days(RECENT_EPISODE_DAYS);
        let recent_completed = episodes
            .iter()
            .rev()
            .skip(1)
            .filter(|episode| episode.is_completed() && episode.end() >= cutoff)
            .take(2)
            .cloned()
            .collect::<Vec<_>>();
        let mut selected = Vec::new();
        let mut seen = HashSet::new();
        for episode in current.iter().chain(recent_completed.iter()) {
            for observation in episode.observations.iter().rev() {
                if seen.insert(observation.id.clone()) {
                    selected.push(observation.clone());
                }
            }
        }
        selected.sort_by(|left, right| {
            right
                .source_updated_at
                .cmp(&left.source_updated_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        let source_window_started_at = selected.iter().map(|value| value.source_updated_at).min();
        let source_window_ended_at = selected.iter().map(|value| value.source_updated_at).max();
        PulseWorkingSet {
            current,
            recent_completed,
            selected,
            source_window_started_at,
            source_window_ended_at,
        }
    }
}

struct RulePulseDraft {
    activity_type: PulseActivityType,
    details: ProjectPulseDetails,
}

fn build_rule_pulse(
    project: &Project,
    working_set: &PulseWorkingSet,
    snapshot: Option<&ProjectSnapshot>,
    session: Option<&DevelopmentSession>,
    working: &[WorkingCopy],
) -> RulePulseDraft {
    let current_episode = working_set.current.as_ref();
    let current_goal = current_episode
        .and_then(ActivityEpisode::root_goal)
        .and_then(clean_goal)
        .or_else(|| {
            session.and_then(|value| {
                (value.status() == SessionStatus::Open)
                    .then(|| truncate_chars(&clean_text(value.goal()), 200))
            })
        })
        .or_else(|| snapshot.map(|value| truncate_chars(&clean_text(value.current_goal()), 200)))
        .unwrap_or_else(|| format!("梳理 {} 当前研发目标", project.name));
    let activity_type = classify_activity_type(&current_goal);

    let status_summary = current_episode
        .and_then(ActivityEpisode::latest)
        .and_then(|value| {
            nonempty(&value.activity_summary).or_else(|| nonempty(&value.outcome_summary))
        })
        .map(|value| summarize_status(&value))
        .filter(|value| !value.is_empty())
        .or_else(|| snapshot.map(|value| truncate_chars(&clean_text(value.status_summary()), 400)))
        .unwrap_or_else(|| "等待新的研发活动观察。".to_owned());

    let mut achievements = Vec::new();
    if matches!(
        activity_type,
        PulseActivityType::Implementation
            | PulseActivityType::Debugging
            | PulseActivityType::Release
            | PulseActivityType::Review
    ) {
        if let Some(episode) = current_episode.filter(|episode| episode.is_completed()) {
            if let Some(value) = extract_episode_achievement(episode) {
                achievements.push(value);
            }
        }
    }
    for episode in &working_set.recent_completed {
        if let Some(value) = extract_episode_achievement(episode) {
            achievements.push(value);
        }
    }
    if achievements.is_empty() {
        if let Some(value) = session.and_then(|value| nonempty(value.outcome())) {
            achievements.push(truncate_chars(&clean_text(&value), 240));
        }
    }

    let mut blockers = Vec::new();
    if let Some(episode) = current_episode {
        for observation in &episode.observations {
            if matches!(observation.turn_status.as_str(), "failed" | "interrupted") {
                if observation.blockers.is_empty() {
                    let goal =
                        clean_goal(&observation.goal_hint).unwrap_or_else(|| "当前活动".to_owned());
                    blockers.push(truncate_chars(&format!("{goal}未完成"), 240));
                } else {
                    blockers.extend(
                        observation
                            .blockers
                            .iter()
                            .map(|value| truncate_chars(&clean_text(value), 240)),
                    );
                }
            }
        }
    }
    if let Some(value) = snapshot {
        blockers.extend(
            value
                .blockers()
                .iter()
                .map(|item| truncate_chars(&clean_text(item), 240)),
        );
    }

    let mut next_steps = Vec::new();
    if let Some(episode) = current_episode {
        for observation in episode.observations.iter().rev() {
            if let Some(value) = nonempty(&observation.handoff_summary) {
                next_steps.push(truncate_chars(&clean_text(&value), 240));
            }
        }
        for observation in episode.observations.iter().rev() {
            next_steps.extend(extract_explicit_next_steps(&observation.outcome_summary));
        }
    }
    if let Some(value) = session.and_then(|value| nonempty(value.handoff())) {
        next_steps.push(truncate_chars(&clean_text(&value), 240));
    }
    for value in working {
        if value.is_dirty() {
            next_steps.push(truncate_chars(
                &format!("检查并整理分支 {} 的未提交 Git 修改。", value.branch()),
                240,
            ));
        }
        if value.ahead_count().unwrap_or_default() > 0 {
            next_steps.push(truncate_chars(
                &format!(
                    "确认分支 {} 的 {} 个尚未推送本地提交。",
                    value.branch(),
                    value.ahead_count().unwrap_or_default()
                ),
                240,
            ));
        }
    }
    if let Some(value) = snapshot {
        next_steps.extend(
            value
                .next_steps()
                .iter()
                .map(|item| truncate_chars(&clean_text(item), 240)),
        );
    }
    dedup_limit(&mut achievements, 3);
    dedup_limit(&mut blockers, 8);
    dedup_limit(&mut next_steps, 3);

    let validation_summary = build_validation_summary(working_set, session);
    let handoff = current_episode
        .and_then(|episode| {
            episode
                .observations
                .iter()
                .rev()
                .find_map(|value| nonempty(&value.handoff_summary))
        })
        .map(|value| truncate_chars(&clean_text(&value), 400))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| next_steps.first().cloned().unwrap_or_default());
    let git_summary = if working.is_empty() {
        "尚无可用的 Git 工作区观察。".to_owned()
    } else {
        working
            .iter()
            .enumerate()
            .map(|(index, value)| format!("仓库 {}：{}", index + 1, git_summary(value)))
            .collect::<Vec<_>>()
            .join("；")
    };
    RulePulseDraft {
        activity_type,
        details: ProjectPulseDetails {
            current_goal,
            status_summary,
            achievements,
            blockers,
            next_steps,
            validation_summary,
            handoff,
            git_summary,
        },
    }
}

fn classify_activity_type(goal: &str) -> PulseActivityType {
    let lower = goal.to_ascii_lowercase();
    let is_question = goal.contains('？')
        || goal.contains('?')
        || [
            "怎么",
            "如何",
            "为什么",
            "是什么",
            "能否",
            "可以吗",
            "怎么办",
            "问题如下",
            "遇到问题",
            "用户反馈",
            "用户反映",
        ]
        .iter()
        .any(|value| goal.contains(value));
    let explicit_change = [
        "实现",
        "修改",
        "修复",
        "开发",
        "调整",
        "implement",
        "change",
        "fix",
        "build",
    ]
    .iter()
    .any(|value| lower.contains(value));
    if is_question && !explicit_change {
        PulseActivityType::SupportQuestion
    } else if ["报错", "错误", "故障", "调试", "debug", "bug", "fix"]
        .iter()
        .any(|value| lower.contains(value))
    {
        PulseActivityType::Debugging
    } else if ["发布", "打包", "release", "deploy"]
        .iter()
        .any(|value| lower.contains(value))
    {
        PulseActivityType::Release
    } else if ["审查", "评审", "审核", "review"]
        .iter()
        .any(|value| lower.contains(value))
    {
        PulseActivityType::Review
    } else if explicit_change {
        PulseActivityType::Implementation
    } else {
        PulseActivityType::Unknown
    }
}

fn clean_goal(value: &str) -> Option<String> {
    let mut value = value.trim();
    let lower = value.to_ascii_lowercase();
    for marker in ["my request for codex:", "please implement this plan:"] {
        if let Some(index) = lower.rfind(marker) {
            value = &value[index + marker.len()..];
            break;
        }
    }
    let value = truncate_chars(&clean_text(value), 200);
    (!value.is_empty()).then_some(value)
}

fn clean_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty()
                || line.starts_with("```")
                || line.starts_with("![")
                || line.starts_with("<environment_context")
                || line.starts_with("<INSTRUCTIONS")
                || line.starts_with("</INSTRUCTIONS")
                || line.to_ascii_lowercase().starts_with("attachment")
                || line.starts_with("附件：")
            {
                return None;
            }
            let line = line.trim_start_matches('#').trim();
            let line = line
                .strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .unwrap_or(line)
                .trim();
            (!line.is_empty()).then_some(line)
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn summarize_status(value: &str) -> String {
    let cleaned = clean_text(value);
    for (index, character) in cleaned.char_indices() {
        if matches!(character, '。' | '！' | '？' | '.' | '!' | '?') && index >= 36 {
            return truncate_chars(&cleaned[..index + character.len_utf8()], 240);
        }
    }
    truncate_chars(&cleaned, 240)
}

fn extract_episode_achievement(episode: &ActivityEpisode) -> Option<String> {
    episode.observations.iter().rev().find_map(|observation| {
        clean_lines(&observation.outcome_summary)
            .into_iter()
            .find(|line| {
                [
                    "已完成",
                    "完成",
                    "已发布",
                    "发布",
                    "通过",
                    "成功",
                    "implemented",
                    "fixed",
                    "released",
                ]
                .iter()
                .any(|marker| line.to_ascii_lowercase().contains(marker))
                    && !is_prompt_wrapper(line)
            })
            .map(|line| truncate_chars(&line, 240))
    })
}

fn extract_explicit_next_steps(value: &str) -> Vec<String> {
    clean_lines(value)
        .into_iter()
        .filter(|line| {
            ["下一步", "接下来", "建议", "后续", "todo", "next"]
                .iter()
                .any(|marker| line.to_ascii_lowercase().contains(marker))
        })
        .map(|line| truncate_chars(&line, 240))
        .take(3)
        .collect()
}

fn build_validation_summary(
    working_set: &PulseWorkingSet,
    session: Option<&DevelopmentSession>,
) -> String {
    let mut facts = Vec::new();
    for observation in &working_set.selected {
        facts.extend(
            clean_lines(&observation.validation_summary)
                .into_iter()
                .map(|line| truncate_chars(&line, 200)),
        );
        facts.extend(
            clean_lines(&observation.outcome_summary)
                .into_iter()
                .filter(|line| is_explicit_validation(line))
                .map(|line| truncate_chars(&line, 200)),
        );
    }
    if facts.is_empty() {
        if let Some(value) = session.and_then(|value| nonempty(value.validation())) {
            facts.push(truncate_chars(&clean_text(&value), 200));
        }
    }
    dedup_limit(&mut facts, 5);
    if facts.is_empty() {
        "尚未捕获明确的验证结果。".to_owned()
    } else {
        truncate_chars(&format!("Codex 报告：{}", facts.join("；")), 1_000)
    }
}

fn is_explicit_validation(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if [
        "可能",
        "也许",
        "预计",
        "建议",
        "理论上",
        "should",
        "may ",
        "might",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        return false;
    }
    [
        "验证结果",
        "validation",
        "tests",
        "通过",
        "成功",
        "passed",
        "exit 0",
        "exit code 0",
        "构建完成",
        "build succeeded",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || contains_pass_ratio(&lower)
}

fn contains_pass_ratio(value: &str) -> bool {
    value.split_whitespace().any(|token| {
        let token =
            token.trim_matches(|character: char| !character.is_ascii_digit() && character != '/');
        token.split_once('/').is_some_and(|(left, right)| {
            !left.is_empty()
                && !right.is_empty()
                && left.bytes().all(|byte| byte.is_ascii_digit())
                && right.bytes().all(|byte| byte.is_ascii_digit())
        })
    })
}

fn clean_lines(value: &str) -> Vec<String> {
    value
        .replace("\r\n", "\n")
        .lines()
        .map(clean_text)
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_prompt_wrapper(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("my request for codex")
        || lower.contains("please implement this plan")
        || lower.contains("agents.md instructions")
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.trim().to_owned();
    }
    let mut output = value
        .chars()
        .take(max.saturating_sub(1))
        .collect::<String>();
    output.push('…');
    output
}

fn git_summary(value: &WorkingCopy) -> String {
    format!(
        "分支 {}，HEAD {}，{} 个未提交文件，ahead {}，behind {}{}",
        value.branch(),
        &value.head_commit()[..8],
        value.diff_summary().changed_files,
        value
            .ahead_count()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".into()),
        value
            .behind_count()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".into()),
        value
            .upstream_branch()
            .map(|value| format!("，upstream {value}"))
            .unwrap_or_default()
    )
}
fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
fn dedup_limit(values: &mut Vec<String>, limit: usize) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.trim().to_ascii_lowercase()));
    values.truncate(limit);
}
fn stable_hash(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn rules_to_json(
    activity_type: PulseActivityType,
    value: &ProjectPulseDetails,
) -> serde_json::Value {
    json!({
        "activity_type": activity_type.as_str(),
        "current_goal": redact_external_text(&value.current_goal),
        "status_summary": redact_external_text(&value.status_summary),
        "achievements": value.achievements.iter().map(|item| redact_external_text(item)).collect::<Vec<_>>(),
        "blockers": value.blockers.iter().map(|item| redact_external_text(item)).collect::<Vec<_>>(),
        "next_steps": value.next_steps.iter().map(|item| redact_external_text(item)).collect::<Vec<_>>(),
        "validation_summary": redact_external_text(&value.validation_summary),
        "handoff": redact_external_text(&value.handoff),
        "git_summary": redact_external_text(&value.git_summary)
    })
}
fn observation_to_json(value: &CodexActivityObservation) -> serde_json::Value {
    json!({
        "status": value.turn_status,
        "goal": redact_external_text(&value.goal_hint),
        "activity": redact_external_text(&value.activity_summary),
        "outcome": redact_external_text(&value.outcome_summary),
        "validation": redact_external_text(&value.validation_summary),
        "handoff": redact_external_text(&value.handoff_summary),
        "blockers": value.blockers.iter().map(|item| redact_external_text(item)).collect::<Vec<_>>(),
        "updated_at": value.source_updated_at
    })
}

#[derive(Deserialize)]
struct AiPulse {
    activity_type: String,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
    validation_summary: String,
    handoff: String,
}
fn parse_ai_pulse(
    content: &str,
    git_summary: String,
) -> Result<(PulseActivityType, ProjectPulseDetails), DomainError> {
    let content = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let value: AiPulse = serde_json::from_str(content).map_err(|_| DomainError::Required {
        field: "valid AI Pulse JSON",
    })?;
    if value.achievements.len() > 3 || value.next_steps.len() > 3 {
        return Err(DomainError::TooManyItems {
            field: "AI Pulse list",
            max: 3,
        });
    }
    let activity_type = PulseActivityType::from_str(&value.activity_type)?;
    let details = ProjectPulseDetails {
        current_goal: truncate_chars(&redact_external_text(&value.current_goal), 200),
        status_summary: truncate_chars(&redact_external_text(&value.status_summary), 400),
        achievements: value
            .achievements
            .iter()
            .map(|item| truncate_chars(&redact_external_text(item), 240))
            .collect(),
        blockers: value
            .blockers
            .iter()
            .map(|item| truncate_chars(&redact_external_text(item), 240))
            .collect(),
        next_steps: value
            .next_steps
            .iter()
            .map(|item| truncate_chars(&redact_external_text(item), 240))
            .collect(),
        validation_summary: truncate_chars(&redact_external_text(&value.validation_summary), 1_000),
        handoff: truncate_chars(&redact_external_text(&value.handoff), 400),
        git_summary: redact_external_text(&git_summary),
    }
    .validated()?;
    Ok((activity_type, details))
}
fn map_repository_error(error: RepositoryError) -> ProjectPulseError {
    match error {
        RepositoryError::OptimisticConflict => ProjectPulseError::VersionConflict,
        RepositoryError::CorruptData(value) => ProjectPulseError::CorruptData(value),
        other => ProjectPulseError::Storage(other.to_string()),
    }
}
fn map_codex_error(error: CodexActivitySourceError) -> ProjectPulseError {
    ProjectPulseError::CodexUnavailable(error.to_string())
}
fn error_code(error: &ProjectPulseError) -> &'static str {
    match error {
        ProjectPulseError::RepositoryNotConnected => "repository_not_connected",
        ProjectPulseError::CodexUnavailable(_) => "codex_unavailable",
        ProjectPulseError::BasisVersionConflict => "basis_version_conflict",
        _ => "sync_failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DiffSummary, LifecycleStage, ProjectDetails, ProjectStatus};

    fn project(now: DateTime<Utc>) -> Project {
        Project::new(
            "p".into(),
            "w".into(),
            ProjectDetails {
                slug: "project".into(),
                name: "Project".into(),
                description: "".into(),
                vision: "".into(),
                target_user: "".into(),
                scope: "".into(),
                non_goals: "".into(),
                tech_stack: "Rust".into(),
                status: ProjectStatus::Active,
                lifecycle_stage: LifecycleStage::Implementation,
                priority: 50,
            },
            now,
        )
        .unwrap()
    }

    fn observation(
        id: &str,
        thread_id: &str,
        goal: &str,
        status: &str,
        outcome: &str,
        validation: &str,
        handoff: &str,
        at: DateTime<Utc>,
    ) -> CodexActivityObservation {
        CodexActivityObservation::create(
            id.into(),
            "p".into(),
            "repo".into(),
            thread_id.into(),
            id.into(),
            "cli".into(),
            status.into(),
            goal.into(),
            goal.into(),
            outcome.into(),
            validation.into(),
            handoff.into(),
            Vec::new(),
            at,
            stable_hash(&json!({ "id": id })),
            at,
        )
        .unwrap()
    }

    #[test]
    fn episode_builder_respects_continuations_and_time_boundaries() {
        let now = Utc::now();
        let observations = vec![
            observation("1", "t", "实现导入", "completed", "完成导入", "", "", now),
            observation(
                "2",
                "t",
                "继续",
                "completed",
                "继续完成",
                "",
                "",
                now + Duration::hours(1),
            ),
            observation(
                "3",
                "t",
                "新的发布目标",
                "completed",
                "已发布",
                "",
                "",
                now + Duration::hours(2),
            ),
            observation(
                "4",
                "t",
                "继续",
                "completed",
                "完成",
                "",
                "",
                now + Duration::hours(11),
            ),
        ];

        let episodes = ActivityEpisodeBuilder::build(&observations);

        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].observations.len(), 2);
        assert_eq!(episodes[1].observations.len(), 1);
        assert_eq!(episodes[2].observations.len(), 1);
    }

    #[test]
    fn episode_builder_caps_twenty_turns_and_seventy_two_hours() {
        let now = Utc::now();
        let mut observations = vec![observation(
            "root",
            "t",
            "实现功能",
            "completed",
            "完成",
            "",
            "",
            now,
        )];
        for index in 1..=20 {
            observations.push(observation(
                &format!("continue-{index}"),
                "t",
                "继续",
                "completed",
                "完成",
                "",
                "",
                now + Duration::minutes(index as i64),
            ));
        }
        observations.push(observation(
            "late-root",
            "u",
            "实现另一功能",
            "completed",
            "完成",
            "",
            "",
            now,
        ));
        observations.push(observation(
            "late-continuation",
            "u",
            "继续",
            "completed",
            "完成",
            "",
            "",
            now + Duration::hours(73),
        ));

        let episodes = ActivityEpisodeBuilder::build(&observations);

        assert!(episodes
            .iter()
            .all(|episode| episode.observations.len() <= 20));
        assert!(episodes.len() >= 4);
    }

    #[test]
    fn goal_cleaner_removes_prompt_wrappers_and_truncates_unicode_safely() {
        let wrapped = "附件：截图\n## My request for Codex:\n为什么 WebView2 安装失败？";
        assert_eq!(
            clean_goal(wrapped).as_deref(),
            Some("为什么 WebView2 安装失败？")
        );
        let long = "测".repeat(260);
        let cleaned = clean_goal(&long).unwrap();
        assert_eq!(cleaned.chars().count(), 200);
        assert!(cleaned.ends_with('…'));
    }

    #[test]
    fn rule_pulse_distinguishes_dirty_and_ahead() {
        let now = Utc::now();
        let project = project(now);
        let working = WorkingCopy::create(
            "wc".into(),
            "repo".into(),
            "main".into(),
            "a".repeat(40),
            true,
            DiffSummary {
                changed_files: 2,
                ..Default::default()
            },
            Some("origin/main".into()),
            Some(3),
            Some(0),
            now,
        )
        .unwrap();
        let working_set = PulseWorkingSetSelector::select(&[], now);
        let pulse = build_rule_pulse(&project, &working_set, None, None, &[working]);
        assert!(!pulse
            .details
            .blockers
            .iter()
            .any(|value| value.contains("未提交")));
        assert!(pulse
            .details
            .next_steps
            .iter()
            .any(|value| value.contains("尚未推送")));
        assert!(pulse.details.next_steps.len() <= 3);
    }

    #[test]
    fn daymark_regression_selects_support_question_without_legacy_noise() {
        let now = Utc::now();
        let observations = vec![
            observation(
                "rc7",
                "rc7-thread",
                "发布 RC7",
                "interrupted",
                "RC7 中断",
                "",
                "继续 RC7",
                now - Duration::days(4),
            ),
            observation(
                "overlay",
                "overlay-thread",
                "修复全屏遮挡",
                "completed",
                "已完成全屏遮挡修复。",
                "",
                "",
                now - Duration::days(3),
            ),
            observation(
                "rc8",
                "rc8-thread",
                "发布 RC8",
                "completed",
                "RC8 已完成并发布。\n测试 53/53 passed。\n集成测试 21/21 passed。\nnpm run build 构建成功。",
                "",
                "",
                now - Duration::days(1),
            ),
            observation(
                "webview",
                "support-thread",
                "附件：安装截图\n## My request for Codex:\nWebView2 安装失败，这个怎么解决？",
                "completed",
                "同样的接口可能访问成功。\n建议使用 Evergreen Standalone Installer。",
                "",
                "",
                now,
            ),
        ];
        let working_set = PulseWorkingSetSelector::select(&observations, now);
        let pulse = build_rule_pulse(&project(now), &working_set, None, None, &[]);

        assert_eq!(pulse.activity_type, PulseActivityType::SupportQuestion);
        assert_eq!(
            pulse.details.current_goal,
            "WebView2 安装失败，这个怎么解决？"
        );
        assert!(pulse
            .details
            .achievements
            .iter()
            .any(|value| value.contains("RC8")));
        assert!(pulse.details.validation_summary.contains("53/53"));
        assert!(pulse.details.validation_summary.contains("21/21"));
        assert!(pulse.details.validation_summary.contains("构建成功"));
        assert!(pulse.details.validation_summary.starts_with("Codex 报告："));
        assert!(!pulse
            .details
            .achievements
            .iter()
            .any(|value| value.contains("可能访问成功")));
        assert!(!pulse.details.validation_summary.contains("可能访问成功"));
        assert!(!pulse
            .details
            .blockers
            .iter()
            .any(|value| value.contains("RC7")));
        assert!(!pulse
            .details
            .next_steps
            .iter()
            .any(|value| value.contains("RC7")));
        assert!(working_set.selected.len() < observations.len());
    }

    #[test]
    fn ai_parser_rejects_out_of_contract_content() {
        let invalid = r#"{"activity_type":"support_question","current_goal":"goal","status_summary":"status","achievements":[],"blockers":[],"next_steps":["1","2","3","4"],"validation_summary":"ok","handoff":"next"}"#;
        assert!(parse_ai_pulse(invalid, "clean".into()).is_err());
    }
}
