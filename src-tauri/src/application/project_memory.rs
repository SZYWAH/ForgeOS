use std::{collections::HashSet, sync::Arc};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    domain::{
        CodexActivityObservation, DevelopmentSession, Project, ProjectPulse, PulseStatus,
        RepositoryCommitObservation, SyncMode,
    },
    ports::{ProjectMemoryKernelRepository, RepositoryError},
};

use super::{
    activity_episode::{ActivityEpisode, ActivityEpisodeBuilder},
    memory_content::{semantically_duplicate, MemoryContentPolicy},
};

use super::memory_content::{dedupe_issues, MemoryQualityIssue, MemoryQualitySeverity};

#[derive(Debug, Error)]
pub enum ProjectMemoryError {
    #[error("create a Workspace before reading project memory")]
    WorkspaceNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("invalid development history cursor")]
    InvalidCursor,
    #[error("provide exactly one of project_id or repository_path")]
    InvalidProjectLocator,
    #[error("the repository directory is not connected to a ForgeOS Project")]
    RepositoryPathNotConnected,
    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct ProjectMemorySummary {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub current_goal: String,
    pub next_steps: Vec<String>,
    pub blocker_count: usize,
    pub recent_activity_at: Option<DateTime<Utc>>,
    pub pulse_status: Option<String>,
    pub pulse_freshness: Option<String>,
    pub has_repository: bool,
    pub dirty_files: i64,
    pub ahead_count: Option<i64>,
    pub behind_count: Option<i64>,
    pub pending_achievement_count: i64,
}

#[derive(Clone, Debug)]
pub struct ProjectMemoryOverview {
    pub project: Project,
    pub pulse: Option<ProjectPulse>,
    pub current_snapshot: Option<crate::domain::ProjectSnapshot>,
    pub current_session: Option<DevelopmentSession>,
    pub recent_history: Vec<DevelopmentHistoryItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitCommitSummary {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub authored_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevelopmentHistoryItem {
    pub id: String,
    pub kind: String,
    pub trust_status: String,
    pub title: String,
    pub goal: String,
    pub rationale: String,
    pub activity: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub codex_turn_count: usize,
    pub git_commits: Vec<GitCommitSummary>,
    pub session_id: Option<String>,
    pub pulse_id: Option<String>,
    pub source_observation_ids: Vec<String>,
    pub source_commit_observation_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DevelopmentHistoryPage {
    pub items: Vec<DevelopmentHistoryItem>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ProjectAiContext {
    pub project_id: String,
    pub content_markdown: String,
    pub content_hash: String,
}

#[derive(Clone)]
pub struct ProjectMemoryQueryKernel {
    repository: Arc<dyn ProjectMemoryKernelRepository>,
}

impl ProjectMemoryQueryKernel {
    pub fn new(repository: Arc<dyn ProjectMemoryKernelRepository>) -> Self {
        Self { repository }
    }

    pub async fn list_project_memory_summaries(
        &self,
    ) -> Result<Vec<ProjectMemorySummary>, ProjectMemoryError> {
        let workspace = self.require_workspace().await?;
        let mut summaries = Vec::new();
        for project in self
            .repository
            .list_projects(&workspace.id)
            .await
            .map_err(map_repository_error)?
        {
            let pulse = self
                .repository
                .get_latest_project_pulse(&project.id)
                .await
                .map_err(map_repository_error)?;
            let snapshot = self
                .repository
                .get_current_snapshot(&project.id)
                .await
                .map_err(map_repository_error)?;
            let repositories = self
                .repository
                .list_repositories(&project.id)
                .await
                .map_err(map_repository_error)?;
            let mut dirty_files = 0;
            let mut ahead_count = Some(0_i64);
            let mut behind_count = Some(0_i64);
            let mut working_activity = None;
            for repository in &repositories {
                if let Some(working) = self
                    .repository
                    .get_current_working_copy(repository.id())
                    .await
                    .map_err(map_repository_error)?
                {
                    dirty_files += working.diff_summary().changed_files;
                    ahead_count = combine_optional_count(ahead_count, working.ahead_count());
                    behind_count = combine_optional_count(behind_count, working.behind_count());
                    working_activity = max_time(working_activity, Some(working.checked_at()));
                }
            }
            let latest_sync = self
                .repository
                .get_latest_successful_activity_sync_at(&project.id)
                .await
                .map_err(map_repository_error)?;
            let latest_observation = self
                .repository
                .list_codex_observations(&project.id, 1)
                .await
                .map_err(map_repository_error)?
                .first()
                .map(|value| value.source_updated_at);
            let latest_commit = self
                .repository
                .list_repository_commit_observations(&project.id, 1)
                .await
                .map_err(map_repository_error)?
                .first()
                .map(|value| value.authored_at);
            let latest_session = self
                .repository
                .list_sessions(&project.id)
                .await
                .map_err(map_repository_error)?
                .into_iter()
                .map(|value| value.updated_at())
                .max();
            let sync_mode = self
                .repository
                .get_project_activity_settings(&project.id)
                .await
                .map_err(map_repository_error)?
                .map(|value| value.sync_mode)
                .unwrap_or(SyncMode::OpenManual);
            let current_goal = pulse
                .as_ref()
                .map(|value| value.details.current_goal.clone())
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    snapshot
                        .as_ref()
                        .map(|value| value.current_goal().to_owned())
                })
                .unwrap_or_default();
            let next_steps = pulse
                .as_ref()
                .map(|value| value.details.next_steps.clone())
                .or_else(|| snapshot.as_ref().map(|value| value.next_steps().to_vec()))
                .unwrap_or_default();
            let blocker_count = pulse
                .as_ref()
                .map(|value| value.details.blockers.len())
                .or_else(|| snapshot.as_ref().map(|value| value.blockers().len()))
                .unwrap_or(0);
            let pending_achievement_count = self
                .repository
                .list_achievements(&project.id)
                .await
                .map_err(map_repository_error)?
                .into_iter()
                .filter(|value| value.status.as_str() == "draft")
                .count() as i64;
            summaries.push(ProjectMemorySummary {
                project_id: project.id.clone(),
                name: project.name.clone(),
                description: project.description.clone(),
                status: project.status.as_str().into(),
                lifecycle_stage: project.lifecycle_stage.as_str().into(),
                current_goal,
                next_steps,
                blocker_count,
                recent_activity_at: [
                    working_activity,
                    pulse.as_ref().map(|value| value.generated_at),
                    latest_observation,
                    latest_commit,
                    latest_session,
                ]
                .into_iter()
                .flatten()
                .max(),
                pulse_status: pulse.as_ref().map(|value| value.status.as_str().into()),
                pulse_freshness: latest_sync.map(|at| freshness(sync_mode, at, Utc::now())),
                has_repository: !repositories.is_empty(),
                dirty_files,
                ahead_count,
                behind_count,
                pending_achievement_count,
            });
        }
        summaries.sort_by(|left, right| attention_rank(left).cmp(&attention_rank(right)));
        Ok(summaries)
    }

    pub async fn get_project_memory_overview(
        &self,
        project_id: &str,
    ) -> Result<ProjectMemoryOverview, ProjectMemoryError> {
        let project = self.require_project(project_id).await?;
        let history = self.list_development_history(project_id, None, 3).await?;
        Ok(ProjectMemoryOverview {
            pulse: self
                .repository
                .get_latest_project_pulse(project_id)
                .await
                .map_err(map_repository_error)?,
            current_snapshot: self
                .repository
                .get_current_snapshot(project_id)
                .await
                .map_err(map_repository_error)?,
            current_session: self
                .repository
                .get_current_session(project_id)
                .await
                .map_err(map_repository_error)?,
            project,
            recent_history: history.items,
        })
    }

    pub async fn resolve_project_locator(
        &self,
        project_id: Option<&str>,
        repository_path: Option<&str>,
    ) -> Result<String, ProjectMemoryError> {
        match (
            project_id.map(str::trim).filter(|value| !value.is_empty()),
            repository_path
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        ) {
            (Some(project_id), None) => {
                self.require_project(project_id).await?;
                Ok(project_id.to_owned())
            }
            (None, Some(path)) => {
                let workspace = self.require_workspace().await?;
                let requested = normalize_windows_path(path);
                let mut best: Option<(usize, String)> = None;
                for project in self
                    .repository
                    .list_projects(&workspace.id)
                    .await
                    .map_err(map_repository_error)?
                {
                    for repository in self
                        .repository
                        .list_repositories(&project.id)
                        .await
                        .map_err(map_repository_error)?
                    {
                        let registered = normalize_windows_path(repository.path());
                        if requested == registered
                            || requested
                                .strip_prefix(&registered)
                                .is_some_and(|suffix| suffix.starts_with('/'))
                        {
                            let score = registered.len();
                            if best.as_ref().is_none_or(|(current, _)| score > *current) {
                                best = Some((score, project.id.clone()));
                            }
                        }
                    }
                }
                best.map(|(_, project_id)| project_id)
                    .ok_or(ProjectMemoryError::RepositoryPathNotConnected)
            }
            _ => Err(ProjectMemoryError::InvalidProjectLocator),
        }
    }

    pub async fn list_project_achievements(
        &self,
        project_id: &str,
    ) -> Result<Vec<crate::domain::ProjectAchievement>, ProjectMemoryError> {
        self.require_project(project_id).await?;
        self.repository
            .list_achievements(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn list_development_history(
        &self,
        project_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DevelopmentHistoryPage, ProjectMemoryError> {
        self.require_project(project_id).await?;
        let observations = self
            .repository
            .list_codex_observations(project_id, 2_000)
            .await
            .map_err(map_repository_error)?;
        let commits = self
            .repository
            .list_repository_commit_observations(project_id, 2_000)
            .await
            .map_err(map_repository_error)?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        let pulses = self
            .repository
            .list_project_pulses(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut items = build_history(&observations, &commits, &sessions, &pulses);
        items.sort_by(|left, right| {
            right
                .started_at
                .cmp(&left.started_at)
                .then_with(|| kind_rank(&left.kind).cmp(&kind_rank(&right.kind)))
                .then_with(|| right.id.cmp(&left.id))
        });
        if let Some(cursor) = cursor {
            let (at, id) = decode_cursor(cursor)?;
            items.retain(|item| item.started_at < at || (item.started_at == at && item.id < id));
        }
        let limit = limit.clamp(1, 50);
        let has_more = items.len() > limit;
        items.truncate(limit);
        let next_cursor = has_more
            .then(|| {
                items
                    .last()
                    .map(|item| encode_cursor(item.started_at, &item.id))
            })
            .flatten();
        Ok(DevelopmentHistoryPage { items, next_cursor })
    }

    pub async fn build_project_ai_context(
        &self,
        project_id: &str,
    ) -> Result<ProjectAiContext, ProjectMemoryError> {
        let project = self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let pulse = self
            .repository
            .get_latest_project_pulse(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut history = self.list_development_history(project_id, None, 5).await?;
        let history_cutoff = Utc::now() - Duration::days(30);
        history
            .items
            .retain(|item| item.started_at >= history_cutoff);
        let mut achievements = self
            .repository
            .list_achievements(project_id)
            .await
            .map_err(map_repository_error)?
            .into_iter()
            .filter(|value| value.status.as_str() == "confirmed")
            .collect::<Vec<_>>();
        achievements.sort_by(|left, right| {
            right
                .details
                .featured
                .cmp(&left.details.featured)
                .then_with(|| right.confirmed_at.cmp(&left.confirmed_at))
                .then_with(|| left.id.cmp(&right.id))
        });
        achievements.truncate(10);
        let repositories = self
            .repository
            .list_repositories(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut git_lines = Vec::new();
        for repository in repositories {
            if let Some(working) = self
                .repository
                .get_current_working_copy(repository.id())
                .await
                .map_err(map_repository_error)?
            {
                git_lines.push(format!(
                    "- Branch `{}`; dirty: {}; changed files: {}; ahead: {}; behind: {}",
                    working.branch(),
                    working.is_dirty(),
                    working.diff_summary().changed_files,
                    optional_count(working.ahead_count()),
                    optional_count(working.behind_count()),
                ));
            }
        }

        let mut markdown = String::from("# ForgeOS Project Context\n\n");
        markdown.push_str("## Project Identity\n\n");
        push_field(&mut markdown, "Name", &project.name);
        push_field(&mut markdown, "Description", &project.description);
        push_field(&mut markdown, "Vision", &project.vision);
        push_field(&mut markdown, "Target User", &project.target_user);
        push_field(&mut markdown, "Scope", &project.scope);
        push_field(&mut markdown, "Tech Stack", &project.tech_stack);

        markdown.push_str("\n## Confirmed Current State\n\n");
        if let Some(snapshot) = snapshot.as_ref() {
            push_field(&mut markdown, "Current Goal", snapshot.current_goal());
            push_field(&mut markdown, "Status", snapshot.status_summary());
            push_list(&mut markdown, "Achievements", snapshot.achievements());
            push_list(&mut markdown, "Blockers", snapshot.blockers());
            push_list(&mut markdown, "Next Steps", snapshot.next_steps());
        } else {
            markdown.push_str("No confirmed current state.\n");
        }

        markdown.push_str("\n## Observed Activity — Unconfirmed\n\n");
        if let Some(pulse) = pulse.as_ref() {
            markdown.push_str("Fact status: observed_unconfirmed\n\n");
            push_field(&mut markdown, "Current Goal", &pulse.details.current_goal);
            push_field(&mut markdown, "Status", &pulse.details.status_summary);
            push_list(&mut markdown, "Achievements", &pulse.details.achievements);
            push_list(&mut markdown, "Blockers", &pulse.details.blockers);
            push_list(&mut markdown, "Next Steps", &pulse.details.next_steps);
        } else {
            markdown.push_str("No saved observed activity.\n");
        }

        markdown.push_str("\n## Recent Development History\n\n");
        for item in history.items {
            markdown.push_str(&format!("### {}\n\n", one_line(&item.title)));
            push_field(&mut markdown, "Trust", &item.trust_status);
            push_field(&mut markdown, "Goal", &item.goal);
            push_field(&mut markdown, "Activity", &item.activity);
            push_field(&mut markdown, "Outcome", &item.outcome);
            push_field(&mut markdown, "Validation", &item.validation);
            push_field(&mut markdown, "Handoff", &item.handoff);
        }

        markdown.push_str("\n## Confirmed Achievements\n\n");
        if achievements.is_empty() {
            markdown.push_str("No confirmed achievements.\n");
        }
        for achievement in achievements {
            markdown.push_str(&format!(
                "- **{}**: {}\n",
                one_line(&achievement.details.title),
                one_line(&achievement.details.result)
            ));
        }

        markdown.push_str("\n## Current Git State\n\n");
        if git_lines.is_empty() {
            markdown.push_str("No connected local repository state.\n");
        } else {
            markdown.push_str(&git_lines.join("\n"));
            markdown.push('\n');
        }

        markdown.push_str("\n## Next Steps\n\n");
        let next_steps = pulse
            .as_ref()
            .map(|value| value.details.next_steps.as_slice())
            .filter(|value| !value.is_empty())
            .or_else(|| snapshot.as_ref().map(|value| value.next_steps()))
            .unwrap_or(&[]);
        if next_steps.is_empty() {
            markdown.push_str("No next steps recorded.\n");
        } else {
            for (index, value) in next_steps.iter().take(3).enumerate() {
                markdown.push_str(&format!("{}. {}\n", index + 1, one_line(value)));
            }
        }
        let content_hash = Sha256::digest(markdown.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        Ok(ProjectAiContext {
            project_id: project_id.to_owned(),
            content_markdown: markdown,
            content_hash,
        })
    }

    async fn require_workspace(&self) -> Result<crate::domain::Workspace, ProjectMemoryError> {
        self.repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(ProjectMemoryError::WorkspaceNotFound)
    }

    async fn require_project(&self, project_id: &str) -> Result<Project, ProjectMemoryError> {
        let workspace = self.require_workspace().await?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ProjectMemoryError::ProjectNotFound)
    }
}

fn push_field(markdown: &mut String, label: &str, value: &str) {
    if !value.trim().is_empty() {
        markdown.push_str(&format!("**{label}:** {}\n\n", one_line(value)));
    }
}

fn push_list(markdown: &mut String, label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    markdown.push_str(&format!("**{label}:**\n"));
    for value in values {
        markdown.push_str(&format!("- {}\n", one_line(value)));
    }
    markdown.push('\n');
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn optional_count(value: Option<i64>) -> String {
    value
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn normalize_windows_path(value: &str) -> String {
    value
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

fn build_history(
    observations: &[CodexActivityObservation],
    commits: &[RepositoryCommitObservation],
    sessions: &[DevelopmentSession],
    pulses: &[ProjectPulse],
) -> Vec<DevelopmentHistoryItem> {
    let confirmed_session_ids = pulses
        .iter()
        .filter_map(|pulse| pulse.confirmed_session_id.clone())
        .collect::<HashSet<_>>();
    let mut matched_commits = HashSet::new();
    let mut items = Vec::new();
    for episode in ActivityEpisodeBuilder::build(observations) {
        let first = match episode.observations.first() {
            Some(value) => value,
            None => continue,
        };
        let related_commit_values = commits
            .iter()
            .filter(|commit| {
                commit.repository_id == first.repository_id
                    && commit.authored_at >= episode.start()
                    && commit.authored_at <= episode.end() + Duration::hours(2)
            })
            .collect::<Vec<_>>();
        for commit in &related_commit_values {
            matched_commits.insert(commit.id.clone());
        }
        let related_commits = related_commit_values
            .iter()
            .map(|commit| commit_summary(commit))
            .collect::<Vec<_>>();
        let related_commit_ids = related_commit_values
            .iter()
            .map(|commit| commit.id.clone())
            .collect::<Vec<_>>();
        let matching_pulse = pulses.iter().find(|pulse| {
            pulse.status == PulseStatus::Confirmed
                && pulse
                    .quality
                    .source_window_started_at
                    .is_some_and(|start| start <= episode.end())
                && pulse
                    .quality
                    .source_window_ended_at
                    .is_some_and(|end| end >= episode.start())
        });
        let session = matching_pulse
            .and_then(|pulse| pulse.confirmed_session_id.as_deref())
            .and_then(|id| sessions.iter().find(|session| session.id() == id));
        items.push(episode_item(
            episode,
            related_commits,
            related_commit_ids,
            matching_pulse,
            session,
        ));
    }
    let mut days: Vec<(String, NaiveDate, Vec<&RepositoryCommitObservation>)> = Vec::new();
    for commit in commits
        .iter()
        .filter(|commit| !matched_commits.contains(&commit.id))
    {
        let date = commit.authored_at.date_naive();
        if let Some((_, _, values)) = days
            .iter_mut()
            .find(|(repository_id, day, _)| repository_id == &commit.repository_id && day == &date)
        {
            values.push(commit);
        } else {
            days.push((commit.repository_id.clone(), date, vec![commit]));
        }
    }
    for (repository_id, date, values) in days {
        let started_at = values
            .iter()
            .map(|value| value.authored_at)
            .min()
            .unwrap_or_else(Utc::now);
        let ended_at = values.iter().map(|value| value.authored_at).max();
        items.push(DevelopmentHistoryItem {
            id: format!("git:{repository_id}:{date}"),
            kind: "git_activity".into(),
            trust_status: "observed".into(),
            title: format!("{} 次代码提交", values.len()),
            goal: values
                .first()
                .map(|value| value.message.clone())
                .unwrap_or_default(),
            rationale: String::new(),
            activity: values
                .iter()
                .map(|value| value.message.as_str())
                .collect::<Vec<_>>()
                .join("；"),
            outcome: String::new(),
            validation: String::new(),
            handoff: String::new(),
            started_at,
            ended_at,
            codex_turn_count: 0,
            git_commits: values.iter().map(|value| commit_summary(value)).collect(),
            session_id: None,
            pulse_id: None,
            source_observation_ids: vec![],
            source_commit_observation_ids: values.iter().map(|value| value.id.clone()).collect(),
        });
    }
    for session in sessions
        .iter()
        .filter(|session| !confirmed_session_ids.contains(session.id()))
    {
        items.push(DevelopmentHistoryItem {
            id: format!("session:{}", session.id()),
            kind: "manual_session".into(),
            trust_status: "confirmed".into(),
            title: compact_title(session.goal()),
            goal: clean_history_text(session.goal(), 300, "history.goal"),
            rationale: String::new(),
            activity: clean_history_text(session.notes(), 800, "history.activity"),
            outcome: clean_history_text(session.outcome(), 600, "history.outcome"),
            validation: clean_history_text(session.validation(), 400, "history.validation"),
            handoff: clean_history_text(session.handoff(), 400, "history.handoff"),
            started_at: session.started_at(),
            ended_at: session.completed_at(),
            codex_turn_count: 0,
            git_commits: vec![],
            session_id: Some(session.id().into()),
            pulse_id: None,
            source_observation_ids: vec![],
            source_commit_observation_ids: vec![],
        });
    }
    items
}

fn episode_item(
    episode: ActivityEpisode,
    git_commits: Vec<GitCommitSummary>,
    source_commit_observation_ids: Vec<String>,
    pulse: Option<&ProjectPulse>,
    session: Option<&DevelopmentSession>,
) -> DevelopmentHistoryItem {
    let first = episode.observations.first();
    let latest = episode.latest();
    let goal = clean_history_text(episode.root_goal().unwrap_or_default(), 300, "history.goal");
    let activity = latest
        .map(|value| clean_history_text(&value.activity_summary, 800, "history.activity"))
        .unwrap_or_default();
    let mut outcome = latest
        .map(|value| clean_history_text(&value.outcome_summary, 600, "history.outcome"))
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            session.map(|value| clean_history_text(value.outcome(), 600, "history.outcome"))
        })
        .unwrap_or_default();
    if semantically_duplicate(&activity, &outcome) {
        outcome.clear();
    }
    DevelopmentHistoryItem {
        id: format!(
            "codex:{}",
            first.map(|value| value.id.as_str()).unwrap_or("unknown")
        ),
        kind: "codex_activity".into(),
        trust_status: if pulse.is_some() || session.is_some() {
            "confirmed".into()
        } else {
            "observed".into()
        },
        title: compact_title(&goal),
        goal,
        rationale: String::new(),
        activity,
        outcome,
        validation: latest
            .map(|value| clean_history_text(&value.validation_summary, 400, "history.validation"))
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                session
                    .map(|value| clean_history_text(value.validation(), 400, "history.validation"))
            })
            .unwrap_or_default(),
        handoff: latest
            .map(|value| clean_history_text(&value.handoff_summary, 400, "history.handoff"))
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                session.map(|value| clean_history_text(value.handoff(), 400, "history.handoff"))
            })
            .unwrap_or_default(),
        started_at: episode.start(),
        ended_at: Some(episode.end()),
        codex_turn_count: episode.observations.len(),
        git_commits,
        session_id: session.map(|value| value.id().into()),
        pulse_id: pulse.map(|value| value.id.clone()),
        source_observation_ids: episode
            .observations
            .iter()
            .map(|value| value.id.clone())
            .collect(),
        source_commit_observation_ids,
    }
}

fn clean_history_text(value: &str, max_chars: usize, section: &str) -> String {
    MemoryContentPolicy::sanitize_fact(value, max_chars, section).value
}

fn commit_summary(value: &RepositoryCommitObservation) -> GitCommitSummary {
    GitCommitSummary {
        sha: value.commit_sha.clone(),
        message: value.message.clone(),
        author_name: value.author_name.clone(),
        authored_at: value.authored_at,
    }
}

fn compact_title(value: &str) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut result = value.chars().take(100).collect::<String>();
    if value.chars().count() > 100 {
        result.push('…');
    }
    if result.is_empty() {
        "研发活动".into()
    } else {
        result
    }
}

fn attention_rank(value: &ProjectMemorySummary) -> (u8, i64, String) {
    let level = if value.pulse_status.as_deref() == Some("pending_review") {
        0
    } else if value.blocker_count > 0 {
        1
    } else if value.dirty_files > 0 || value.ahead_count.unwrap_or(0) > 0 {
        2
    } else if value.pulse_freshness.as_deref() == Some("stale") {
        3
    } else {
        4
    };
    (
        level,
        -value
            .recent_activity_at
            .map(|time| time.timestamp())
            .unwrap_or_default(),
        value.name.to_ascii_lowercase(),
    )
}

fn combine_optional_count(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        _ => None,
    }
}

fn max_time(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    left.into_iter().chain(right).max()
}

fn freshness(mode: SyncMode, at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let threshold = match mode {
        SyncMode::Interval30m => Duration::minutes(60),
        SyncMode::OpenManual => Duration::hours(24),
    };
    if now - at > threshold {
        "stale".into()
    } else {
        "fresh".into()
    }
}

fn kind_rank(value: &str) -> u8 {
    match value {
        "codex_activity" => 0,
        "manual_session" => 1,
        "git_activity" => 2,
        _ => 3,
    }
}

fn encode_cursor(at: DateTime<Utc>, id: &str) -> String {
    let raw = format!("{}\n{}", at.to_rfc3339(), id);
    raw.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_cursor(value: &str) -> Result<(DateTime<Utc>, String), ProjectMemoryError> {
    if value.len() % 2 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ProjectMemoryError::InvalidCursor);
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ProjectMemoryError::InvalidCursor)?;
    let raw = String::from_utf8(bytes).map_err(|_| ProjectMemoryError::InvalidCursor)?;
    let (time, id) = raw
        .split_once('\n')
        .ok_or(ProjectMemoryError::InvalidCursor)?;
    let time = DateTime::parse_from_rfc3339(time)
        .map_err(|_| ProjectMemoryError::InvalidCursor)?
        .with_timezone(&Utc);
    if id.trim().is_empty() {
        return Err(ProjectMemoryError::InvalidCursor);
    }
    Ok((time, id.into()))
}

fn map_repository_error(error: RepositoryError) -> ProjectMemoryError {
    match error {
        RepositoryError::CorruptData(message) => ProjectMemoryError::CorruptData(message),
        other => ProjectMemoryError::Storage(other.to_string()),
    }
}
