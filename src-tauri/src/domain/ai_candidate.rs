use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_ID_LENGTH: usize = 128;
const MAX_TITLE_LENGTH: usize = 200;
const MAX_CONTENT_LENGTH: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiCandidateType {
    SnapshotUpdate,
    SessionSummary,
    EvidenceSuggestion,
    RiskFinding,
    GeneralNote,
}

impl AiCandidateType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SnapshotUpdate => "snapshot_update",
            Self::SessionSummary => "session_summary",
            Self::EvidenceSuggestion => "evidence_suggestion",
            Self::RiskFinding => "risk_finding",
            Self::GeneralNote => "general_note",
        }
    }
}

impl FromStr for AiCandidateType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "snapshot_update" => Ok(Self::SnapshotUpdate),
            "session_summary" => Ok(Self::SessionSummary),
            "evidence_suggestion" => Ok(Self::EvidenceSuggestion),
            "risk_finding" => Ok(Self::RiskFinding),
            "general_note" => Ok(Self::GeneralNote),
            _ => Err(DomainError::InvalidAiCandidateType(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiCandidateStatus {
    Pending,
    Accepted,
    Rejected,
    Archived,
}

impl AiCandidateStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for AiCandidateStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::InvalidAiCandidateStatus(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiCandidate {
    id: String,
    project_id: String,
    candidate_type: AiCandidateType,
    source_context_export_id: String,
    title: String,
    content_markdown: String,
    status: AiCandidateStatus,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AiCandidate {
    pub fn create(
        id: String,
        project_id: String,
        candidate_type: AiCandidateType,
        source_context_export_id: String,
        title: String,
        content_markdown: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            candidate_type,
            source_context_export_id,
            title,
            content_markdown,
            AiCandidateStatus::Pending,
            1,
            now,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        candidate_type: AiCandidateType,
        source_context_export_id: String,
        title: String,
        content_markdown: String,
        status: AiCandidateStatus,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidAiCandidateVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidAiCandidateTimestamps);
        }
        Ok(Self {
            id: required("AI Candidate id", id, MAX_ID_LENGTH)?,
            project_id: required("project id", project_id, MAX_ID_LENGTH)?,
            candidate_type,
            source_context_export_id: required(
                "source Context Export id",
                source_context_export_id,
                MAX_ID_LENGTH,
            )?,
            title: required("AI Candidate title", title, MAX_TITLE_LENGTH)?,
            content_markdown: required(
                "AI Candidate content",
                content_markdown,
                MAX_CONTENT_LENGTH,
            )?,
            status,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn accept(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(AiCandidateStatus::Accepted, now)
    }

    pub fn reject(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(AiCandidateStatus::Rejected, now)
    }

    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.transition(AiCandidateStatus::Archived, now)
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::AiCandidateProjectMismatch);
        }
        Ok(())
    }

    pub fn ensure_source_project(&self, source_project_id: &str) -> Result<(), DomainError> {
        if self.project_id != source_project_id {
            return Err(DomainError::AiCandidateSourceProjectMismatch);
        }
        Ok(())
    }

    fn transition(
        &mut self,
        target: AiCandidateStatus,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != AiCandidateStatus::Pending {
            return Err(DomainError::AiCandidateNotPending);
        }
        self.status = target;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::AiCandidateVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn candidate_type(&self) -> AiCandidateType {
        self.candidate_type
    }
    pub fn source_context_export_id(&self) -> &str {
        &self.source_context_export_id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn content_markdown(&self) -> &str {
        &self.content_markdown
    }
    pub fn status(&self) -> AiCandidateStatus {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> AiCandidate {
        AiCandidate::create(
            "candidate-1".into(),
            "project-1".into(),
            AiCandidateType::RiskFinding,
            "export-1".into(),
            "Performance risk".into(),
            "Validate repository indexing.".into(),
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn ai_created_candidate_is_always_pending() {
        let candidate = pending();
        assert_eq!(candidate.status(), AiCandidateStatus::Pending);
        assert_eq!(candidate.version(), 1);
    }

    #[test]
    fn review_is_a_single_terminal_transition() {
        for transition in [
            AiCandidate::accept as fn(&mut AiCandidate, DateTime<Utc>) -> _,
            AiCandidate::reject,
            AiCandidate::archive,
        ] {
            let mut candidate = pending();
            transition(&mut candidate, Utc::now()).unwrap();
            assert_eq!(candidate.version(), 2);
            assert_eq!(
                candidate.accept(Utc::now()),
                Err(DomainError::AiCandidateNotPending)
            );
        }
    }
}
