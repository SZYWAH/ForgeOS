use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_GOAL_LENGTH: usize = 2_000;
const MAX_BODY_LENGTH: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Open,
    Completed,
    Abandoned,
}

impl SessionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Completed => "completed",
            Self::Abandoned => "abandoned",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Abandoned)
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "open" => Ok(Self::Open),
            "completed" => Ok(Self::Completed),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(DomainError::InvalidSessionStatus(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevelopmentSession {
    id: String,
    project_id: String,
    status: SessionStatus,
    goal: String,
    notes: String,
    outcome: String,
    validation: String,
    handoff: String,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl DevelopmentSession {
    pub fn start(
        id: String,
        project_id: String,
        goal: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            SessionStatus::Open,
            goal,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            now,
            None,
            now,
            now,
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        status: SessionStatus,
        goal: String,
        notes: String,
        outcome: String,
        validation: String,
        handoff: String,
        started_at: DateTime<Utc>,
        completed_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let id = required("Development Session id", id, 64)?;
        let project_id = required("project id", project_id, 64)?;
        let goal = required("session goal", goal, MAX_GOAL_LENGTH)?;
        let notes = text("session notes", notes, MAX_BODY_LENGTH)?;
        let outcome = text("session outcome", outcome, MAX_BODY_LENGTH)?;
        let validation = text("session validation", validation, MAX_BODY_LENGTH)?;
        let handoff = text("session handoff", handoff, MAX_BODY_LENGTH)?;

        if version < 1 {
            return Err(DomainError::InvalidSessionVersion);
        }
        if updated_at < created_at || started_at > updated_at {
            return Err(DomainError::InvalidSessionTimestamps);
        }

        match status {
            SessionStatus::Open => {
                if completed_at.is_some()
                    || !outcome.is_empty()
                    || !validation.is_empty()
                    || !handoff.is_empty()
                {
                    return Err(DomainError::InvalidSessionLifecycle);
                }
            }
            SessionStatus::Completed => {
                let finished = completed_at.ok_or(DomainError::InvalidSessionLifecycle)?;
                if outcome.is_empty()
                    || validation.is_empty()
                    || handoff.is_empty()
                    || finished < started_at
                    || finished < created_at
                    || finished > updated_at
                {
                    return Err(DomainError::InvalidSessionLifecycle);
                }
            }
            SessionStatus::Abandoned => {
                let finished = completed_at.ok_or(DomainError::InvalidSessionLifecycle)?;
                if !outcome.is_empty()
                    || !validation.is_empty()
                    || handoff.is_empty()
                    || finished < started_at
                    || finished < created_at
                    || finished > updated_at
                {
                    return Err(DomainError::InvalidSessionLifecycle);
                }
            }
        }

        Ok(Self {
            id,
            project_id,
            status,
            goal,
            notes,
            outcome,
            validation,
            handoff,
            started_at,
            completed_at,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn update_open(
        &mut self,
        goal: String,
        notes: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_open()?;
        let goal = required("session goal", goal, MAX_GOAL_LENGTH)?;
        let notes = text("session notes", notes, MAX_BODY_LENGTH)?;
        let next_version = self.next_version()?;
        self.goal = goal;
        self.notes = notes;
        self.updated_at = self.effective_now(now);
        self.version = next_version;
        Ok(())
    }

    pub fn complete(
        &mut self,
        outcome: String,
        validation: String,
        handoff: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_open()?;
        let outcome = required("session outcome", outcome, MAX_BODY_LENGTH)?;
        let validation = required("session validation", validation, MAX_BODY_LENGTH)?;
        let handoff = required("session handoff", handoff, MAX_BODY_LENGTH)?;
        let finished = self.effective_now(now);
        let next_version = self.next_version()?;
        self.outcome = outcome;
        self.validation = validation;
        self.handoff = handoff;
        self.status = SessionStatus::Completed;
        self.completed_at = Some(finished);
        self.updated_at = finished;
        self.version = next_version;
        Ok(())
    }

    pub fn abandon(&mut self, handoff: String, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_open()?;
        let handoff = required("session handoff", handoff, MAX_BODY_LENGTH)?;
        let finished = self.effective_now(now);
        let next_version = self.next_version()?;
        self.handoff = handoff;
        self.status = SessionStatus::Abandoned;
        self.completed_at = Some(finished);
        self.updated_at = finished;
        self.version = next_version;
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::SessionProjectMismatch);
        }
        Ok(())
    }

    fn ensure_open(&self) -> Result<(), DomainError> {
        if self.status != SessionStatus::Open {
            return Err(DomainError::TerminalSessionCannotBeModified);
        }
        Ok(())
    }

    fn effective_now(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now.max(self.updated_at).max(self.started_at)
    }

    fn next_version(&self) -> Result<i64, DomainError> {
        self.version
            .checked_add(1)
            .ok_or(DomainError::SessionVersionOverflow)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn status(&self) -> SessionStatus {
        self.status
    }

    pub fn goal(&self) -> &str {
        &self.goal
    }

    pub fn notes(&self) -> &str {
        &self.notes
    }

    pub fn outcome(&self) -> &str {
        &self.outcome
    }

    pub fn validation(&self) -> &str {
        &self.validation
    }

    pub fn handoff(&self) -> &str {
        &self.handoff
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn version(&self) -> i64 {
        self.version
    }
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

fn text(field: &'static str, value: String, max: usize) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.chars().count() > max {
        return Err(DomainError::TooLong { field, max });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    #[test]
    fn lifecycle_is_open_then_exactly_one_terminal_state() {
        let now = Utc::now();
        let mut session = DevelopmentSession::start(
            "session-1".into(),
            "project-1".into(),
            " Build D1.3 ".into(),
            now,
        )
        .expect("Session should start");
        assert_eq!(session.goal(), "Build D1.3");
        assert_eq!(session.status(), SessionStatus::Open);
        assert_eq!(session.version(), 1);

        session
            .update_open(
                "Build D1.3 safely".into(),
                "Repository implemented".into(),
                now - Duration::minutes(1),
            )
            .expect("open Session should update");
        assert_eq!(session.version(), 2);
        assert_eq!(session.updated_at(), now);

        session
            .complete(
                "Kernel complete".into(),
                "cargo test passed".into(),
                "Start UI acceptance".into(),
                now,
            )
            .expect("open Session should complete");
        assert_eq!(session.status(), SessionStatus::Completed);
        assert_eq!(session.version(), 3);
        assert_eq!(
            session.abandon(String::new(), now),
            Err(DomainError::TerminalSessionCannotBeModified)
        );
        assert_eq!(
            session.update_open("Rewrite".into(), String::new(), now),
            Err(DomainError::TerminalSessionCannotBeModified)
        );
    }

    #[test]
    fn completion_requires_outcome_and_validation() {
        let now = Utc::now();
        let mut session = DevelopmentSession::start(
            "session-1".into(),
            "project-1".into(),
            "Build D1.3".into(),
            now,
        )
        .expect("Session should start");

        assert_eq!(
            session.complete(String::new(), "cargo test".into(), String::new(), now),
            Err(DomainError::Required {
                field: "session outcome"
            })
        );
        assert_eq!(session.status(), SessionStatus::Open);
        assert_eq!(session.version(), 1);

        assert_eq!(
            session.complete(
                "Kernel complete".into(),
                "cargo test passed".into(),
                String::new(),
                now,
            ),
            Err(DomainError::Required {
                field: "session handoff"
            })
        );
        assert_eq!(session.version(), 1);
    }

    #[test]
    fn abandonment_requires_handoff() {
        let now = Utc::now();
        let mut session = DevelopmentSession::start(
            "session-1".into(),
            "project-1".into(),
            "Build D1.3".into(),
            now,
        )
        .expect("Session should start");

        assert_eq!(
            session.abandon(String::new(), now),
            Err(DomainError::Required {
                field: "session handoff"
            })
        );
        assert_eq!(session.status(), SessionStatus::Open);
        assert_eq!(session.version(), 1);
    }

    #[test]
    fn session_is_project_bound() {
        let session = DevelopmentSession::start(
            "session-1".into(),
            "project-1".into(),
            "Build D1.3".into(),
            Utc::now(),
        )
        .expect("Session should start");

        assert_eq!(
            session.ensure_project("project-2"),
            Err(DomainError::SessionProjectMismatch)
        );
    }
}
