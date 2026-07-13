use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_ID: usize = 128;
const MAX_RESULT_TEXT: usize = 50_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

impl CodexExecutionStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failed | Self::Cancelled)
    }
}

impl FromStr for CodexExecutionStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DomainError::InvalidCodexExecutionStatus(value.into())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexExecutionRecord {
    id: String,
    proposal_id: String,
    project_id: String,
    status: CodexExecutionStatus,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    output_summary: String,
    validation_result: String,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CodexExecutionRecord {
    pub fn create(
        id: String,
        proposal_id: String,
        project_id: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            proposal_id,
            project_id,
            CodexExecutionStatus::Pending,
            None,
            None,
            String::new(),
            String::new(),
            1,
            now,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        proposal_id: String,
        project_id: String,
        status: CodexExecutionStatus,
        started_at: Option<DateTime<Utc>>,
        ended_at: Option<DateTime<Utc>>,
        output_summary: String,
        validation_result: String,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidCodexExecutionVersion);
        }
        if updated_at < created_at || started_at.is_some_and(|value| value < created_at) {
            return Err(DomainError::InvalidCodexExecutionTimestamps);
        }
        if ended_at.is_some_and(|value| {
            value < created_at || started_at.is_some_and(|started| value < started)
        }) {
            return Err(DomainError::InvalidCodexExecutionTimestamps);
        }

        let lifecycle_valid = match status {
            CodexExecutionStatus::Pending => started_at.is_none() && ended_at.is_none(),
            CodexExecutionStatus::Running => started_at.is_some() && ended_at.is_none(),
            CodexExecutionStatus::Success | CodexExecutionStatus::Failed => {
                started_at.is_some() && ended_at.is_some()
            }
            CodexExecutionStatus::Cancelled => ended_at.is_some(),
        };
        if !lifecycle_valid {
            return Err(DomainError::InvalidCodexExecutionLifecycle);
        }

        let output_summary = text("execution output summary", output_summary)?;
        let validation_result = text("execution validation result", validation_result)?;
        if matches!(
            status,
            CodexExecutionStatus::Success | CodexExecutionStatus::Failed
        ) && (output_summary.is_empty() || validation_result.is_empty())
        {
            return Err(DomainError::CodexExecutionResultRequired);
        }

        Ok(Self {
            id: required_id("Codex Execution id", id)?,
            proposal_id: required_id("Codex Task Proposal id", proposal_id)?,
            project_id: required_id("project id", project_id)?,
            status,
            started_at,
            ended_at,
            output_summary,
            validation_result,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn start(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.require_status(CodexExecutionStatus::Pending, CodexExecutionStatus::Running)?;
        self.status = CodexExecutionStatus::Running;
        self.started_at = Some(now.max(self.created_at));
        self.advance(now)?;
        Ok(())
    }

    pub fn succeed(
        &mut self,
        output_summary: String,
        validation_result: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.finish(
            CodexExecutionStatus::Success,
            output_summary,
            validation_result,
            now,
        )
    }

    pub fn fail(
        &mut self,
        output_summary: String,
        validation_result: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.finish(
            CodexExecutionStatus::Failed,
            output_summary,
            validation_result,
            now,
        )
    }

    pub fn cancel(
        &mut self,
        output_summary: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.status,
            CodexExecutionStatus::Pending | CodexExecutionStatus::Running
        ) {
            return Err(DomainError::InvalidCodexExecutionTransition {
                from: self.status.as_str(),
                to: CodexExecutionStatus::Cancelled.as_str(),
            });
        }
        self.output_summary = text("execution cancellation summary", output_summary)?;
        self.validation_result.clear();
        self.status = CodexExecutionStatus::Cancelled;
        self.ended_at = Some(now.max(self.started_at.unwrap_or(self.created_at)));
        self.advance(now)?;
        Ok(())
    }

    fn finish(
        &mut self,
        target: CodexExecutionStatus,
        output_summary: String,
        validation_result: String,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.require_status(CodexExecutionStatus::Running, target)?;
        let output_summary = text("execution output summary", output_summary)?;
        let validation_result = text("execution validation result", validation_result)?;
        if output_summary.is_empty() || validation_result.is_empty() {
            return Err(DomainError::CodexExecutionResultRequired);
        }
        self.output_summary = output_summary;
        self.validation_result = validation_result;
        self.status = target;
        self.ended_at = Some(now.max(self.started_at.expect("running execution has start time")));
        self.advance(now)?;
        Ok(())
    }

    fn require_status(
        &self,
        expected: CodexExecutionStatus,
        target: CodexExecutionStatus,
    ) -> Result<(), DomainError> {
        if self.status != expected {
            Err(DomainError::InvalidCodexExecutionTransition {
                from: self.status.as_str(),
                to: target.as_str(),
            })
        } else {
            Ok(())
        }
    }

    fn advance(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::CodexExecutionVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            Err(DomainError::CodexExecutionProjectMismatch)
        } else {
            Ok(())
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn status(&self) -> CodexExecutionStatus {
        self.status
    }
    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }
    pub fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }
    pub fn output_summary(&self) -> &str {
        &self.output_summary
    }
    pub fn validation_result(&self) -> &str {
        &self.validation_result
    }
    pub fn version(&self) -> i64 {
        self.version
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

fn required_id(field: &'static str, value: String) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(DomainError::Required { field })
    } else if value.chars().count() > MAX_ID {
        Err(DomainError::TooLong { field, max: MAX_ID })
    } else {
        Ok(value)
    }
}

fn text(field: &'static str, value: String) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.chars().count() > MAX_RESULT_TEXT {
        Err(DomainError::TooLong {
            field,
            max: MAX_RESULT_TEXT,
        })
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> CodexExecutionRecord {
        CodexExecutionRecord::create(
            "execution-1".into(),
            "proposal-1".into(),
            "project-1".into(),
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn success_requires_running_and_a_result() {
        let mut execution = pending();
        assert!(matches!(
            execution.succeed("done".into(), "tests passed".into(), Utc::now()),
            Err(DomainError::InvalidCodexExecutionTransition { .. })
        ));
        execution.start(Utc::now()).unwrap();
        assert!(matches!(
            execution.succeed(String::new(), "tests passed".into(), Utc::now()),
            Err(DomainError::CodexExecutionResultRequired)
        ));
        execution
            .succeed("implemented".into(), "cargo test passed".into(), Utc::now())
            .unwrap();
        assert_eq!(execution.status(), CodexExecutionStatus::Success);
        assert_eq!(execution.version(), 3);
    }

    #[test]
    fn cancellation_is_terminal_and_preserves_history_shape() {
        let mut execution = pending();
        execution
            .cancel("user cancelled".into(), Utc::now())
            .unwrap();
        assert_eq!(execution.status(), CodexExecutionStatus::Cancelled);
        assert!(execution.ended_at().is_some());
        assert!(execution.start(Utc::now()).is_err());
    }
}
