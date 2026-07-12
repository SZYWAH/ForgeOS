use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_TITLE_LENGTH: usize = 200;
const MAX_DESCRIPTION_LENGTH: usize = 20_000;
const MAX_SOURCE_URI_LENGTH: usize = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceType {
    Document,
    Screenshot,
    Link,
    TestResult,
    Note,
    Artifact,
    GitCommit,
    GitStatus,
}

impl EvidenceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::Screenshot => "screenshot",
            Self::Link => "link",
            Self::TestResult => "test_result",
            Self::Note => "note",
            Self::Artifact => "artifact",
            Self::GitCommit => "git_commit",
            Self::GitStatus => "git_status",
        }
    }
}

impl fmt::Display for EvidenceType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for EvidenceType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "document" => Ok(Self::Document),
            "screenshot" => Ok(Self::Screenshot),
            "link" => Ok(Self::Link),
            "test_result" => Ok(Self::TestResult),
            "note" => Ok(Self::Note),
            "artifact" => Ok(Self::Artifact),
            "git_commit" => Ok(Self::GitCommit),
            "git_status" => Ok(Self::GitStatus),
            _ => Err(DomainError::InvalidEvidenceType(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceSourceType {
    LocalFile,
    ExternalUrl,
    ManualNote,
}

impl EvidenceSourceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalFile => "local_file",
            Self::ExternalUrl => "external_url",
            Self::ManualNote => "manual_note",
        }
    }
}

impl FromStr for EvidenceSourceType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local_file" => Ok(Self::LocalFile),
            "external_url" => Ok(Self::ExternalUrl),
            "manual_note" => Ok(Self::ManualNote),
            _ => Err(DomainError::InvalidEvidenceSourceType(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Unverified,
    Verified,
    Invalid,
}

impl VerificationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Verified => "verified",
            Self::Invalid => "invalid",
        }
    }
}

impl FromStr for VerificationStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "unverified" => Ok(Self::Unverified),
            "verified" => Ok(Self::Verified),
            "invalid" => Ok(Self::Invalid),
            _ => Err(DomainError::InvalidEvidenceVerificationStatus(
                value.to_owned(),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDetails {
    pub title: String,
    pub description: String,
    pub evidence_type: EvidenceType,
    pub source_type: EvidenceSourceType,
    pub source_uri: String,
}

impl EvidenceDetails {
    fn validated(self) -> Result<Self, DomainError> {
        let title = required("evidence title", self.title, MAX_TITLE_LENGTH)?;
        let description = text(
            "evidence description",
            self.description,
            MAX_DESCRIPTION_LENGTH,
        )?;
        let source_uri = text(
            "evidence source URI",
            self.source_uri,
            MAX_SOURCE_URI_LENGTH,
        )?;

        match self.source_type {
            EvidenceSourceType::LocalFile if source_uri.is_empty() => {
                return Err(DomainError::Required {
                    field: "evidence source URI",
                });
            }
            EvidenceSourceType::ExternalUrl
                if !(source_uri.starts_with("https://") || source_uri.starts_with("http://")) =>
            {
                return Err(DomainError::InvalidEvidenceSourceUri);
            }
            _ => {}
        }

        Ok(Self {
            title,
            description,
            evidence_type: self.evidence_type,
            source_type: self.source_type,
            source_uri,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    id: String,
    project_id: String,
    details: EvidenceDetails,
    verification_status: VerificationStatus,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl Evidence {
    pub fn create(
        id: String,
        project_id: String,
        details: EvidenceDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            details,
            VerificationStatus::Unverified,
            now,
            now,
            1,
        )
    }

    pub fn restore(
        id: String,
        project_id: String,
        details: EvidenceDetails,
        verification_status: VerificationStatus,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let id = required("evidence id", id, 64)?;
        let project_id = required("project id", project_id, 64)?;
        let details = details.validated()?;
        if version < 1 {
            return Err(DomainError::InvalidEvidenceVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidEvidenceTimestamps);
        }

        Ok(Self {
            id,
            project_id,
            details,
            verification_status,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn update(
        &mut self,
        details: EvidenceDetails,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let details = details.validated()?;
        let next_version = self.next_version()?;
        self.details = details;
        self.verification_status = VerificationStatus::Unverified;
        self.updated_at = now.max(self.updated_at);
        self.version = next_version;
        Ok(())
    }

    pub fn verify(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        let next_version = self.next_version()?;
        self.verification_status = VerificationStatus::Verified;
        self.updated_at = now.max(self.updated_at);
        self.version = next_version;
        Ok(())
    }

    pub fn invalidate(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        let next_version = self.next_version()?;
        self.verification_status = VerificationStatus::Invalid;
        self.updated_at = now.max(self.updated_at);
        self.version = next_version;
        Ok(())
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::EvidenceProjectMismatch);
        }
        Ok(())
    }

    pub fn is_git_evidence(&self) -> bool {
        matches!(
            self.details.evidence_type,
            EvidenceType::GitCommit | EvidenceType::GitStatus
        )
    }

    fn next_version(&self) -> Result<i64, DomainError> {
        self.version
            .checked_add(1)
            .ok_or(DomainError::EvidenceVersionOverflow)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn title(&self) -> &str {
        &self.details.title
    }

    pub fn description(&self) -> &str {
        &self.details.description
    }

    pub fn evidence_type(&self) -> EvidenceType {
        self.details.evidence_type
    }

    pub fn source_type(&self) -> EvidenceSourceType {
        self.details.source_type
    }

    pub fn source_uri(&self) -> &str {
        &self.details.source_uri
    }

    pub fn verification_status(&self) -> VerificationStatus {
        self.verification_status
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceTargetType {
    Project,
    Snapshot,
    Session,
}

impl EvidenceTargetType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Snapshot => "snapshot",
            Self::Session => "session",
        }
    }
}

impl FromStr for EvidenceTargetType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "project" => Ok(Self::Project),
            "snapshot" => Ok(Self::Snapshot),
            "session" => Ok(Self::Session),
            _ => Err(DomainError::InvalidEvidenceTargetType(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceRelationType {
    Supports,
    RelatesTo,
}

impl EvidenceRelationType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::RelatesTo => "relates_to",
        }
    }
}

impl FromStr for EvidenceRelationType {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "supports" => Ok(Self::Supports),
            "relates_to" => Ok(Self::RelatesTo),
            _ => Err(DomainError::InvalidEvidenceRelationType(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceLink {
    id: String,
    evidence_id: String,
    target_type: EvidenceTargetType,
    target_id: String,
    relation_type: EvidenceRelationType,
}

impl EvidenceLink {
    pub fn create(
        id: String,
        evidence_id: String,
        target_type: EvidenceTargetType,
        target_id: String,
        relation_type: EvidenceRelationType,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: required("evidence link id", id, 64)?,
            evidence_id: required("evidence id", evidence_id, 64)?,
            target_type,
            target_id: required("evidence target id", target_id, 64)?,
            relation_type,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }

    pub fn target_type(&self) -> EvidenceTargetType {
        self.target_type
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn relation_type(&self) -> EvidenceRelationType {
        self.relation_type
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
    use super::*;

    fn details() -> EvidenceDetails {
        EvidenceDetails {
            title: " Architecture document ".into(),
            description: "D1.4 design evidence".into(),
            evidence_type: EvidenceType::Document,
            source_type: EvidenceSourceType::LocalFile,
            source_uri: " docs/architecture.md ".into(),
        }
    }

    #[test]
    fn evidence_changes_are_versioned_and_content_updates_require_reverification() {
        let now = Utc::now();
        let mut evidence =
            Evidence::create("evidence-1".into(), "project-1".into(), details(), now)
                .expect("Evidence should be valid");
        assert_eq!(evidence.title(), "Architecture document");
        assert_eq!(evidence.version(), 1);
        assert_eq!(
            evidence.verification_status(),
            VerificationStatus::Unverified
        );

        evidence.verify(now).expect("Evidence should verify");
        assert_eq!(evidence.verification_status(), VerificationStatus::Verified);
        assert_eq!(evidence.version(), 2);

        let mut changed = details();
        changed.description = "Updated source interpretation".into();
        evidence
            .update(changed, now)
            .expect("Evidence should update");
        assert_eq!(
            evidence.verification_status(),
            VerificationStatus::Unverified
        );
        assert_eq!(evidence.version(), 3);

        evidence
            .invalidate(now)
            .expect("Evidence should invalidate");
        assert_eq!(evidence.verification_status(), VerificationStatus::Invalid);
        assert_eq!(evidence.version(), 4);
    }

    #[test]
    fn external_sources_require_http_urls() {
        let mut invalid = details();
        invalid.source_type = EvidenceSourceType::ExternalUrl;
        invalid.source_uri = "not-a-url".into();
        assert_eq!(
            Evidence::create("evidence-1".into(), "project-1".into(), invalid, Utc::now(),),
            Err(DomainError::InvalidEvidenceSourceUri)
        );
    }
}
