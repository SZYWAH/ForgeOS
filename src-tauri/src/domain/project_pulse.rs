use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_TEXT: usize = 20_000;
const MAX_ITEM: usize = 2_000;
const MAX_ID: usize = 256;
const MAX_NEXT_STEPS: usize = 3;

macro_rules! string_enum {
    ($name:ident, $error:ident, {$($variant:ident => $value:literal),+ $(,)?}) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.as_str()) }
        }
        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(DomainError::$error(value.to_owned())),
                }
            }
        }
    };
}

string_enum!(SyncMode, InvalidActivitySyncMode, {
    OpenManual => "open_manual",
    Interval30m => "interval_30m"
});
string_enum!(SyncTrigger, InvalidActivitySyncTrigger, {
    ProjectOpen => "project_open",
    Manual => "manual",
    Interval => "interval"
});
string_enum!(SyncStatus, InvalidActivitySyncStatus, {
    Queued => "queued",
    Running => "running",
    Succeeded => "succeeded",
    Partial => "partial",
    Failed => "failed"
});
string_enum!(SyncStage, InvalidActivitySyncStage, {
    Connecting => "connecting",
    ReadingCodex => "reading_codex",
    ReadingGit => "reading_git",
    Analyzing => "analyzing",
    Persisting => "persisting"
});
string_enum!(PulseStatus, InvalidProjectPulseStatus, {
    PendingReview => "pending_review",
    Confirmed => "confirmed",
    Dismissed => "dismissed",
    Superseded => "superseded"
});
string_enum!(PulseAnalysisMode, InvalidProjectPulseAnalysisMode, {
    Rules => "rules",
    Ai => "ai"
});
string_enum!(PulseActivityType, InvalidProjectPulseActivityType, {
    Implementation => "implementation",
    Debugging => "debugging",
    Release => "release",
    Review => "review",
    SupportQuestion => "support_question",
    Unknown => "unknown"
});
string_enum!(PulseAnalysisRevision, InvalidProjectPulseAnalysisRevision, {
    LegacyV1 => "legacy-v1",
    RulesV2 => "rules-v2",
    AiV2 => "ai-v2"
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexConnectorSettings {
    pub workspace_id: String,
    pub executable_path: Option<String>,
    pub last_detected_version: Option<String>,
    pub last_health_check_at: Option<DateTime<Utc>>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CodexConnectorSettings {
    pub fn new(workspace_id: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        Self::restore(workspace_id, None, None, None, 1, now, now)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        workspace_id: String,
        executable_path: Option<String>,
        last_detected_version: Option<String>,
        last_health_check_at: Option<DateTime<Utc>>,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidProjectActivityVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidTimestamps);
        }
        Ok(Self {
            workspace_id: required("workspace id", workspace_id, MAX_ID)?,
            executable_path: optional(executable_path, 32_768)?,
            last_detected_version: optional(last_detected_version, 256)?,
            last_health_check_at,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn update_path(
        &mut self,
        executable_path: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.executable_path = optional(executable_path, 32_768)?;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::ProjectPulseVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn record_health(
        &mut self,
        path: Option<String>,
        version: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.executable_path = optional(path, 32_768)?;
        self.last_detected_version = optional(Some(version), 256)?;
        self.last_health_check_at = Some(now);
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectActivitySettings {
    pub project_id: String,
    pub sync_mode: SyncMode,
    pub ai_enabled: bool,
    pub ai_provider_id: Option<String>,
    pub initial_backfill_completed: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectActivitySettings {
    pub fn new(project_id: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
        Self::restore(
            project_id,
            SyncMode::OpenManual,
            false,
            None,
            false,
            1,
            now,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        project_id: String,
        sync_mode: SyncMode,
        ai_enabled: bool,
        ai_provider_id: Option<String>,
        initial_backfill_completed: bool,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidProjectActivityVersion);
        }
        if ai_enabled
            && ai_provider_id
                .as_ref()
                .is_none_or(|id| id.trim().is_empty())
        {
            return Err(DomainError::ProjectActivityAiProviderRequired);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidTimestamps);
        }
        Ok(Self {
            project_id: required("project id", project_id, MAX_ID)?,
            sync_mode,
            ai_enabled,
            ai_provider_id: optional(ai_provider_id, MAX_ID)?,
            initial_backfill_completed,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn update(
        &mut self,
        sync_mode: SyncMode,
        ai_enabled: bool,
        ai_provider_id: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let provider = optional(ai_provider_id, MAX_ID)?;
        if ai_enabled && provider.is_none() {
            return Err(DomainError::ProjectActivityAiProviderRequired);
        }
        self.sync_mode = sync_mode;
        self.ai_enabled = ai_enabled;
        self.ai_provider_id = provider;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::ProjectPulseVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn mark_backfill_completed(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !self.initial_backfill_completed {
            self.initial_backfill_completed = true;
            self.version = self
                .version
                .checked_add(1)
                .ok_or(DomainError::ProjectPulseVersionOverflow)?;
            self.updated_at = now.max(self.updated_at);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivitySyncRun {
    pub id: String,
    pub project_id: String,
    pub trigger: SyncTrigger,
    pub status: SyncStatus,
    pub stage: SyncStage,
    pub threads_scanned: i64,
    pub turns_imported: i64,
    pub observations_created: i64,
    pub commits_imported: i64,
    pub ai_invoked: bool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
}

impl ActivitySyncRun {
    pub fn queued(
        id: String,
        project_id: String,
        trigger: SyncTrigger,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: required("sync run id", id, MAX_ID)?,
            project_id: required("project id", project_id, MAX_ID)?,
            trigger,
            status: SyncStatus::Queued,
            stage: SyncStage::Connecting,
            threads_scanned: 0,
            turns_imported: 0,
            observations_created: 0,
            commits_imported: 0,
            ai_invoked: false,
            started_at: now,
            completed_at: None,
            error_code: None,
            error_summary: None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexActivityObservation {
    pub id: String,
    pub project_id: String,
    pub repository_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub source_kind: String,
    pub turn_status: String,
    pub goal_hint: String,
    pub activity_summary: String,
    pub outcome_summary: String,
    pub validation_summary: String,
    pub handoff_summary: String,
    pub blockers: Vec<String>,
    pub source_updated_at: DateTime<Utc>,
    pub source_hash: String,
    pub observed_at: DateTime<Utc>,
}

impl CodexActivityObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
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
        blockers: Vec<String>,
        source_updated_at: DateTime<Utc>,
        source_hash: String,
        observed_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if !matches!(
            turn_status.as_str(),
            "in_progress" | "completed" | "interrupted" | "failed" | "unknown"
        ) {
            return Err(DomainError::InvalidActivitySyncStatus(turn_status));
        }
        validate_hash(&source_hash).map_err(|_| DomainError::InvalidCodexObservationHash)?;
        Ok(Self {
            id: required("observation id", id, MAX_ID)?,
            project_id: required("project id", project_id, MAX_ID)?,
            repository_id: required("repository id", repository_id, MAX_ID)?,
            thread_id: required("Codex thread id", thread_id, MAX_ID)?,
            turn_id: required("Codex turn id", turn_id, MAX_ID)?,
            source_kind: required("Codex source kind", source_kind, 128)?,
            turn_status,
            goal_hint: text("goal hint", goal_hint)?,
            activity_summary: text("activity summary", activity_summary)?,
            outcome_summary: text("outcome summary", outcome_summary)?,
            validation_summary: text("validation summary", validation_summary)?,
            handoff_summary: text("handoff summary", handoff_summary)?,
            blockers: list("observation blocker", blockers, usize::MAX)?,
            source_updated_at,
            source_hash,
            observed_at,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPulseDetails {
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub validation_summary: String,
    pub handoff: String,
    pub git_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPulseQuality {
    pub activity_type: PulseActivityType,
    pub analysis_revision: PulseAnalysisRevision,
    pub source_window_started_at: Option<DateTime<Utc>>,
    pub source_window_ended_at: Option<DateTime<Utc>>,
    pub selected_observation_count: i64,
    pub available_observation_count: i64,
}

impl ProjectPulseQuality {
    pub fn new(
        activity_type: PulseActivityType,
        analysis_revision: PulseAnalysisRevision,
        source_window_started_at: Option<DateTime<Utc>>,
        source_window_ended_at: Option<DateTime<Utc>>,
        selected_observation_count: i64,
        available_observation_count: i64,
    ) -> Result<Self, DomainError> {
        if selected_observation_count < 0
            || available_observation_count < 0
            || selected_observation_count > available_observation_count
        {
            return Err(DomainError::InvalidProjectPulseObservationCounts);
        }
        if source_window_started_at
            .zip(source_window_ended_at)
            .is_some_and(|(started, ended)| started > ended)
        {
            return Err(DomainError::InvalidProjectPulseSourceWindow);
        }
        Ok(Self {
            activity_type,
            analysis_revision,
            source_window_started_at,
            source_window_ended_at,
            selected_observation_count,
            available_observation_count,
        })
    }
}

impl ProjectPulseDetails {
    pub fn validated(self) -> Result<Self, DomainError> {
        Ok(Self {
            current_goal: required("Pulse current goal", self.current_goal, MAX_TEXT)?,
            status_summary: required("Pulse status summary", self.status_summary, MAX_TEXT)?,
            achievements: list("Pulse achievement", self.achievements, usize::MAX)?,
            blockers: list("Pulse blocker", self.blockers, usize::MAX)?,
            next_steps: list("Pulse next step", self.next_steps, MAX_NEXT_STEPS)?,
            validation_summary: text("Pulse validation summary", self.validation_summary)?,
            handoff: text("Pulse handoff", self.handoff)?,
            git_summary: text("Pulse Git summary", self.git_summary)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectPulse {
    pub id: String,
    pub project_id: String,
    pub status: PulseStatus,
    pub analysis_mode: PulseAnalysisMode,
    pub ai_provider_id: Option<String>,
    pub ai_model: Option<String>,
    pub quality: ProjectPulseQuality,
    pub details: ProjectPulseDetails,
    pub snapshot_basis_version: Option<i64>,
    pub session_basis_id: Option<String>,
    pub session_basis_version: Option<i64>,
    pub working_copy_id: Option<String>,
    pub content_hash: String,
    pub generated_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
    pub confirmed_snapshot_id: Option<String>,
    pub confirmed_session_id: Option<String>,
}

impl ProjectPulse {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: String,
        project_id: String,
        analysis_mode: PulseAnalysisMode,
        ai_provider_id: Option<String>,
        ai_model: Option<String>,
        quality: ProjectPulseQuality,
        details: ProjectPulseDetails,
        snapshot_basis_version: Option<i64>,
        session_basis_id: Option<String>,
        session_basis_version: Option<i64>,
        working_copy_id: Option<String>,
        content_hash: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            PulseStatus::PendingReview,
            analysis_mode,
            ai_provider_id,
            ai_model,
            quality,
            details,
            snapshot_basis_version,
            session_basis_id,
            session_basis_version,
            working_copy_id,
            content_hash,
            now,
            now,
            1,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        status: PulseStatus,
        analysis_mode: PulseAnalysisMode,
        ai_provider_id: Option<String>,
        ai_model: Option<String>,
        quality: ProjectPulseQuality,
        details: ProjectPulseDetails,
        snapshot_basis_version: Option<i64>,
        session_basis_id: Option<String>,
        session_basis_version: Option<i64>,
        working_copy_id: Option<String>,
        content_hash: String,
        generated_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
        confirmed_snapshot_id: Option<String>,
        confirmed_session_id: Option<String>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidProjectActivityVersion);
        }
        if updated_at < generated_at {
            return Err(DomainError::InvalidProjectPulseTimestamps);
        }
        validate_hash(&content_hash)?;
        let provider = optional(ai_provider_id, MAX_ID)?;
        if analysis_mode == PulseAnalysisMode::Ai && provider.is_none() {
            return Err(DomainError::ProjectActivityAiProviderRequired);
        }
        Ok(Self {
            id: required("Pulse id", id, MAX_ID)?,
            project_id: required("project id", project_id, MAX_ID)?,
            status,
            analysis_mode,
            ai_provider_id: provider,
            ai_model: optional(ai_model, 512)?,
            quality,
            details: details.validated()?,
            snapshot_basis_version,
            session_basis_id: optional(session_basis_id, MAX_ID)?,
            session_basis_version,
            working_copy_id: optional(working_copy_id, MAX_ID)?,
            content_hash,
            generated_at,
            updated_at,
            version,
            confirmed_snapshot_id: optional(confirmed_snapshot_id, MAX_ID)?,
            confirmed_session_id: optional(confirmed_session_id, MAX_ID)?,
        })
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::ProjectPulseProjectMismatch);
        }
        Ok(())
    }

    pub fn quality_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.quality.analysis_revision == PulseAnalysisRevision::LegacyV1 {
            warnings.push("旧版规则摘要：建议重新同步后再确认。".to_owned());
        }
        if contains_prompt_wrapper(&self.details.current_goal)
            || contains_prompt_wrapper(&self.details.status_summary)
            || self
                .details
                .achievements
                .iter()
                .chain(self.details.blockers.iter())
                .chain(self.details.next_steps.iter())
                .any(|value| contains_prompt_wrapper(value))
        {
            warnings.push("内容仍包含附件前缀或提示词包装，请先编辑。".to_owned());
        }
        if self.details.current_goal.chars().count() > 200 {
            warnings.push("当前目标超过 200 字，请先精简。".to_owned());
        }
        if self.details.status_summary.chars().count() > 400 {
            warnings.push("状态摘要超过 400 字，请先精简。".to_owned());
        }
        if self
            .details
            .achievements
            .iter()
            .chain(self.details.blockers.iter())
            .chain(self.details.next_steps.iter())
            .any(|value| value.chars().count() > 240)
        {
            warnings.push("列表中存在超过 240 字的条目，请先精简。".to_owned());
        }
        warnings
    }

    pub fn ensure_confirmable_quality(&self) -> Result<(), DomainError> {
        let warnings = self.quality_warnings();
        if warnings
            .iter()
            .any(|warning| !warning.starts_with("旧版规则摘要"))
        {
            Err(DomainError::ProjectPulseQualityRequiresReview)
        } else {
            Ok(())
        }
    }

    pub fn edit(
        &mut self,
        details: ProjectPulseDetails,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_pending()?;
        self.details = details.validated()?;
        self.bump(now)
    }

    pub fn mark_confirmed(
        &mut self,
        snapshot_id: Option<String>,
        session_id: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_pending()?;
        self.status = PulseStatus::Confirmed;
        self.confirmed_snapshot_id = optional(snapshot_id, MAX_ID)?;
        self.confirmed_session_id = optional(session_id, MAX_ID)?;
        self.bump(now)
    }

    pub fn dismiss(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(PulseStatus::Dismissed, now)
    }
    pub fn supersede(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(PulseStatus::Superseded, now)
    }

    fn transition(&mut self, status: PulseStatus, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_pending()?;
        self.status = status;
        self.bump(now)
    }
    fn ensure_pending(&self) -> Result<(), DomainError> {
        if self.status != PulseStatus::PendingReview {
            Err(DomainError::ProjectPulseNotPending)
        } else {
            Ok(())
        }
    }
    fn bump(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::ProjectPulseVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }
}

fn contains_prompt_wrapper(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("my request for codex")
        || lower.contains("please implement this plan")
        || lower.contains("attached")
        || value.contains("附件")
}

fn required(field: &'static str, value: String, max: usize) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(DomainError::Required { field });
    }
    if value.chars().count() > max {
        return Err(DomainError::TooLong { field, max });
    }
    Ok(value)
}
fn optional(value: Option<String>, max: usize) -> Result<Option<String>, DomainError> {
    value
        .map(|value| {
            let value = value.trim().to_owned();
            if value.chars().count() > max {
                return Err(DomainError::TooLong {
                    field: "optional Project Activity field",
                    max,
                });
            }
            Ok(value)
        })
        .transpose()
        .map(|value| value.filter(|value| !value.is_empty()))
}
fn text(field: &'static str, value: String) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.chars().count() > MAX_TEXT {
        return Err(DomainError::TooLong {
            field,
            max: MAX_TEXT,
        });
    }
    Ok(value)
}
fn list(field: &'static str, values: Vec<String>, max: usize) -> Result<Vec<String>, DomainError> {
    let mut output = Vec::new();
    for value in values {
        let value = value.trim().to_owned();
        if value.is_empty() || output.contains(&value) {
            continue;
        }
        if value.chars().count() > MAX_ITEM {
            return Err(DomainError::TooLong {
                field,
                max: MAX_ITEM,
            });
        }
        output.push(value);
    }
    if output.len() > max {
        return Err(DomainError::TooManyNextSteps { max });
    }
    Ok(output)
}
fn validate_hash(value: &str) -> Result<(), DomainError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(DomainError::InvalidProjectPulseHash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> ProjectPulseDetails {
        ProjectPulseDetails {
            current_goal: "Ship D6".into(),
            status_summary: "Implementation active".into(),
            achievements: vec!["Migration ready".into()],
            blockers: vec![],
            next_steps: vec!["Build intake".into()],
            validation_summary: "cargo test".into(),
            handoff: "Continue adapter".into(),
            git_summary: "dirty".into(),
        }
    }

    #[test]
    fn pulse_is_pending_and_limits_next_steps() {
        let now = Utc::now();
        let pulse = ProjectPulse::create(
            "pulse-1".into(),
            "project-1".into(),
            PulseAnalysisMode::Rules,
            None,
            None,
            ProjectPulseQuality::new(
                PulseActivityType::Implementation,
                PulseAnalysisRevision::RulesV2,
                None,
                None,
                0,
                0,
            )
            .unwrap(),
            details(),
            None,
            None,
            None,
            None,
            "a".repeat(64),
            now,
        )
        .unwrap();
        assert_eq!(pulse.status, PulseStatus::PendingReview);
        let mut invalid = details();
        invalid.next_steps = vec!["1".into(), "2".into(), "3".into(), "4".into()];
        assert_eq!(
            invalid.validated(),
            Err(DomainError::TooManyNextSteps { max: 3 })
        );
    }

    #[test]
    fn reviewed_pulse_cannot_be_changed_again() {
        let now = Utc::now();
        let mut pulse = ProjectPulse::create(
            "pulse-1".into(),
            "project-1".into(),
            PulseAnalysisMode::Rules,
            None,
            None,
            ProjectPulseQuality::new(
                PulseActivityType::Implementation,
                PulseAnalysisRevision::RulesV2,
                None,
                None,
                0,
                0,
            )
            .unwrap(),
            details(),
            None,
            None,
            None,
            None,
            "a".repeat(64),
            now,
        )
        .unwrap();
        pulse.dismiss(now).unwrap();
        assert_eq!(pulse.dismiss(now), Err(DomainError::ProjectPulseNotPending));
    }

    #[test]
    fn editing_a_draft_preserves_the_source_fingerprint() {
        let now = Utc::now();
        let mut pulse = ProjectPulse::create(
            "pulse-1".into(),
            "project-1".into(),
            PulseAnalysisMode::Rules,
            None,
            None,
            ProjectPulseQuality::new(
                PulseActivityType::Implementation,
                PulseAnalysisRevision::RulesV2,
                None,
                None,
                0,
                0,
            )
            .unwrap(),
            details(),
            None,
            None,
            None,
            None,
            "a".repeat(64),
            now,
        )
        .unwrap();
        let fingerprint = pulse.content_hash.clone();
        let mut edited = details();
        edited.current_goal = "人工修订后的目标".into();

        pulse.edit(edited, now).unwrap();

        assert_eq!(pulse.content_hash, fingerprint);
    }

    #[test]
    fn prompt_wrappers_block_confirmation_until_the_draft_is_cleaned() {
        let now = Utc::now();
        let mut wrapped = details();
        wrapped.current_goal = "## My request for Codex: implement this".into();
        let pulse = ProjectPulse::create(
            "pulse-1".into(),
            "project-1".into(),
            PulseAnalysisMode::Rules,
            None,
            None,
            ProjectPulseQuality::new(
                PulseActivityType::Implementation,
                PulseAnalysisRevision::RulesV2,
                None,
                None,
                1,
                1,
            )
            .unwrap(),
            wrapped,
            None,
            None,
            None,
            None,
            "a".repeat(64),
            now,
        )
        .unwrap();

        assert_eq!(
            pulse.ensure_confirmable_quality(),
            Err(DomainError::ProjectPulseQualityRequiresReview)
        );
    }
}
