use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_ID: usize = 128;
const MAX_TITLE: usize = 200;
const MAX_TEXT: usize = 20_000;
const MAX_ITEMS: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexTaskProposalStatus {
    Draft,
    Proposed,
    Approved,
    Rejected,
    Archived,
}

impl CodexTaskProposalStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for CodexTaskProposalStatus {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "draft" => Ok(Self::Draft),
            "proposed" => Ok(Self::Proposed),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::InvalidCodexTaskProposalStatus(value.into())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexTaskProposalDetails {
    pub title: String,
    pub description: String,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub allowed_scope: Vec<String>,
    pub forbidden_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub validation_commands: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexTaskProposal {
    id: String,
    project_id: String,
    context_export_id: String,
    details: CodexTaskProposalDetails,
    status: CodexTaskProposalStatus,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CodexTaskProposal {
    pub fn create(
        id: String,
        project_id: String,
        context_export_id: String,
        details: CodexTaskProposalDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            context_export_id,
            details,
            CodexTaskProposalStatus::Draft,
            1,
            now,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        context_export_id: String,
        details: CodexTaskProposalDetails,
        status: CodexTaskProposalStatus,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidCodexTaskProposalVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidCodexTaskProposalTimestamps);
        }
        Ok(Self {
            id: required("Codex Task Proposal id", id, MAX_ID)?,
            project_id: required("project id", project_id, MAX_ID)?,
            context_export_id: required("Context Export id", context_export_id, MAX_ID)?,
            details: validate_details(details)?,
            status,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn propose(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(
            CodexTaskProposalStatus::Draft,
            CodexTaskProposalStatus::Proposed,
            now,
        )
    }
    pub fn approve(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(
            CodexTaskProposalStatus::Proposed,
            CodexTaskProposalStatus::Approved,
            now,
        )
    }
    pub fn reject(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(
            CodexTaskProposalStatus::Proposed,
            CodexTaskProposalStatus::Rejected,
            now,
        )
    }
    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(
            CodexTaskProposalStatus::Approved,
            CodexTaskProposalStatus::Archived,
            now,
        )
    }

    fn transition(
        &mut self,
        expected: CodexTaskProposalStatus,
        target: CodexTaskProposalStatus,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != expected {
            return Err(DomainError::InvalidCodexTaskProposalTransition {
                from: self.status.as_str(),
                to: target.as_str(),
            });
        }
        self.status = target;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::CodexTaskProposalVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            Err(DomainError::CodexTaskProposalProjectMismatch)
        } else {
            Ok(())
        }
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn context_export_id(&self) -> &str {
        &self.context_export_id
    }
    pub fn details(&self) -> &CodexTaskProposalDetails {
        &self.details
    }
    pub fn status(&self) -> CodexTaskProposalStatus {
        self.status
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

fn validate_details(
    mut value: CodexTaskProposalDetails,
) -> Result<CodexTaskProposalDetails, DomainError> {
    value.title = required("proposal title", value.title, MAX_TITLE)?;
    value.description = text("proposal description", value.description, MAX_TEXT)?;
    value.goal = required("proposal goal", value.goal, MAX_TEXT)?;
    value.non_goals = list("non-goals", value.non_goals)?;
    value.allowed_scope = list("allowed scope", value.allowed_scope)?;
    value.forbidden_scope = list("forbidden scope", value.forbidden_scope)?;
    value.acceptance_criteria = list("acceptance criteria", value.acceptance_criteria)?;
    value.validation_commands = list("validation commands", value.validation_commands)?;
    if value.acceptance_criteria.is_empty() {
        return Err(DomainError::Required {
            field: "acceptance criteria",
        });
    }
    Ok(value)
}

fn required(field: &'static str, value: String, max: usize) -> Result<String, DomainError> {
    let value = text(field, value, max)?;
    if value.is_empty() {
        Err(DomainError::Required { field })
    } else {
        Ok(value)
    }
}
fn text(field: &'static str, value: String, max: usize) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.chars().count() > max {
        Err(DomainError::TooLong { field, max })
    } else {
        Ok(value)
    }
}
fn list(field: &'static str, values: Vec<String>) -> Result<Vec<String>, DomainError> {
    let mut result = Vec::new();
    for value in values {
        let value = text(field, value, MAX_TEXT)?;
        if !value.is_empty() && !result.contains(&value) {
            result.push(value);
        }
    }
    if result.len() > MAX_ITEMS {
        return Err(DomainError::TooManyItems {
            field,
            max: MAX_ITEMS,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> CodexTaskProposal {
        CodexTaskProposal::create(
            "p1".into(),
            "project-1".into(),
            "export-1".into(),
            CodexTaskProposalDetails {
                title: "Implement cache".into(),
                description: "Proposal only".into(),
                goal: "Improve performance".into(),
                non_goals: vec!["Database migration".into()],
                allowed_scope: vec!["backend/agent/*".into()],
                forbidden_scope: vec!["auth/*".into()],
                acceptance_criteria: vec!["Tests pass".into()],
                validation_commands: vec!["cargo test".into()],
            },
            Utc::now(),
        )
        .unwrap()
    }
    #[test]
    fn lifecycle_requires_human_review_order() {
        let mut value = draft();
        assert_eq!(value.status(), CodexTaskProposalStatus::Draft);
        value.propose(Utc::now()).unwrap();
        value.approve(Utc::now()).unwrap();
        value.archive(Utc::now()).unwrap();
        assert_eq!(value.version(), 4);
    }
    #[test]
    fn approval_cannot_skip_proposal() {
        assert!(matches!(
            draft().approve(Utc::now()),
            Err(DomainError::InvalidCodexTaskProposalTransition { .. })
        ));
    }
}
