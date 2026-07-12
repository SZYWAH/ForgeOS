use std::str::FromStr;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::DomainError;

const MAX_ID_LENGTH: usize = 128;
const MAX_CONTENT_LENGTH: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextTemplateType {
    Planning,
    Coding,
    Debugging,
    Review,
    Handoff,
}

impl ContextTemplateType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Coding => "coding",
            Self::Debugging => "debugging",
            Self::Review => "review",
            Self::Handoff => "handoff",
        }
    }

    pub const fn purpose(self) -> &'static str {
        match self {
            Self::Planning => "Plan the next engineering steps from confirmed project facts.",
            Self::Coding => "Prepare implementation work from confirmed project facts.",
            Self::Debugging => "Investigate current blockers using confirmed project facts.",
            Self::Review => "Review engineering state and risks from confirmed project facts.",
            Self::Handoff => "Restore context and continue work from confirmed project facts.",
        }
    }
}

impl FromStr for ContextTemplateType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "planning" => Ok(Self::Planning),
            "coding" => Ok(Self::Coding),
            "debugging" => Ok(Self::Debugging),
            "review" => Ok(Self::Review),
            "handoff" => Ok(Self::Handoff),
            _ => Err(DomainError::InvalidContextTemplateType(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextExportStatus {
    Generated,
    SentToAi,
}

impl ContextExportStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::SentToAi => "sent_to_ai",
        }
    }
}

impl FromStr for ContextExportStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "generated" => Ok(Self::Generated),
            "sent_to_ai" => Ok(Self::SentToAi),
            _ => Err(DomainError::InvalidContextExportStatus(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextExport {
    id: String,
    project_id: String,
    template_type: ContextTemplateType,
    snapshot_version: Option<i64>,
    generated_from_session_ids: Vec<String>,
    content_markdown: String,
    content_hash: String,
    status: ContextExportStatus,
    created_at: DateTime<Utc>,
    version: i64,
}

impl ContextExport {
    #[allow(clippy::too_many_arguments)]
    pub fn generate(
        id: String,
        project_id: String,
        template_type: ContextTemplateType,
        snapshot_version: Option<i64>,
        generated_from_session_ids: Vec<String>,
        content_markdown: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let content_hash = Self::calculate_hash(&content_markdown);
        Self::restore(
            id,
            project_id,
            template_type,
            snapshot_version,
            generated_from_session_ids,
            content_markdown,
            content_hash,
            ContextExportStatus::Generated,
            now,
            1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        template_type: ContextTemplateType,
        snapshot_version: Option<i64>,
        generated_from_session_ids: Vec<String>,
        content_markdown: String,
        content_hash: String,
        status: ContextExportStatus,
        created_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let id = required("Context Export id", id, MAX_ID_LENGTH)?;
        let project_id = required("project id", project_id, MAX_ID_LENGTH)?;
        if snapshot_version.is_some_and(|value| value < 1) {
            return Err(DomainError::InvalidSnapshotVersion);
        }
        let mut session_ids = Vec::with_capacity(generated_from_session_ids.len());
        for id in generated_from_session_ids {
            session_ids.push(required("source Session id", id, MAX_ID_LENGTH)?);
        }
        session_ids.sort();
        session_ids.dedup();
        if content_markdown.trim().is_empty() {
            return Err(DomainError::Required {
                field: "Context Pack Markdown",
            });
        }
        if content_markdown.chars().count() > MAX_CONTENT_LENGTH {
            return Err(DomainError::TooLong {
                field: "Context Pack Markdown",
                max: MAX_CONTENT_LENGTH,
            });
        }
        if version < 1 {
            return Err(DomainError::InvalidContextExportVersion);
        }
        if content_hash != Self::calculate_hash(&content_markdown) {
            return Err(DomainError::InvalidContextExportHash);
        }
        Ok(Self {
            id,
            project_id,
            template_type,
            snapshot_version,
            generated_from_session_ids: session_ids,
            content_markdown,
            content_hash,
            status,
            created_at,
            version,
        })
    }

    pub fn validate_sources(
        project_id: &str,
        snapshot_project_id: Option<&str>,
        session_project_ids: &[&str],
        evidence_project_ids: &[&str],
    ) -> Result<(), DomainError> {
        if snapshot_project_id.is_some_and(|value| value != project_id)
            || session_project_ids.iter().any(|value| *value != project_id)
            || evidence_project_ids
                .iter()
                .any(|value| *value != project_id)
        {
            return Err(DomainError::ContextSourceProjectMismatch);
        }
        Ok(())
    }

    pub fn calculate_hash(content_markdown: &str) -> String {
        Sha256::digest(content_markdown.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn mark_sent_to_ai(&mut self) -> Result<(), DomainError> {
        if self.status == ContextExportStatus::SentToAi {
            return Ok(());
        }
        self.status = ContextExportStatus::SentToAi;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::ContextExportVersionOverflow)?;
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::ContextExportProjectMismatch);
        }
        Ok(())
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn template_type(&self) -> ContextTemplateType {
        self.template_type
    }
    pub fn snapshot_version(&self) -> Option<i64> {
        self.snapshot_version
    }
    pub fn generated_from_session_ids(&self) -> &[String] {
        &self.generated_from_session_ids
    }
    pub fn content_markdown(&self) -> &str {
        &self.content_markdown
    }
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }
    pub fn status(&self) -> ContextExportStatus {
        self.status
    }
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_sha256() {
        assert_eq!(
            ContextExport::calculate_hash("ForgeOS"),
            "b305ed29a319746e8e99000507da27d76159c946f49edc2837129e2864b72275"
        );
    }

    #[test]
    fn sent_status_changes_only_once() {
        let mut export = ContextExport::generate(
            "export-1".into(),
            "project-1".into(),
            ContextTemplateType::Planning,
            None,
            vec![],
            "# Context".into(),
            Utc::now(),
        )
        .unwrap();
        export.mark_sent_to_ai().unwrap();
        export.mark_sent_to_ai().unwrap();
        assert_eq!(export.status(), ContextExportStatus::SentToAi);
        assert_eq!(export.version(), 2);
    }
}
