use std::str::FromStr;

use chrono::Utc;
use uuid::Uuid;

use crate::{
    domain::{
        Evidence, EvidenceDetails, EvidenceLink, EvidenceRelationType, EvidenceSourceType,
        EvidenceTargetType, EvidenceType,
    },
    ports::{
        EvidenceRepository, ProjectRepository, RepositoryError, SessionRepository,
        SnapshotRepository, WorkspaceRepository,
    },
};

use super::{project, workspace::map_read_error, ApplicationError};

#[derive(Clone, Debug)]
pub struct CreateEvidenceInput {
    pub title: String,
    pub description: String,
    pub evidence_type: String,
    pub source_type: String,
    pub source_uri: String,
}

#[derive(Clone, Debug)]
pub struct UpdateEvidenceInput {
    pub title: String,
    pub description: String,
    pub evidence_type: String,
    pub source_type: String,
    pub source_uri: String,
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct ChangeEvidenceStatusInput {
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct LinkEvidenceInput {
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
}

#[derive(Clone, Debug)]
pub struct EvidenceRecord {
    pub evidence: Evidence,
    pub links: Vec<EvidenceLink>,
}

pub async fn create_evidence<R>(
    repository: &R,
    project_id: &str,
    input: CreateEvidenceInput,
) -> Result<Evidence, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_evidence_changes()?;
    let details = details(
        input.title,
        input.description,
        input.evidence_type,
        input.source_type,
        input.source_uri,
    )?;
    if matches!(
        details.evidence_type,
        EvidenceType::GitCommit | EvidenceType::GitStatus
    ) {
        return Err(crate::domain::DomainError::GitEvidenceRequiresRepositoryIntegration.into());
    }
    let evidence = Evidence::create(Uuid::now_v7().to_string(), project.id, details, Utc::now())?;

    match repository.insert_evidence(&evidence).await {
        Ok(()) => Ok(evidence),
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptEvidenceChanges.into())
        }
        Err(error) => Err(map_read_error(error)),
    }
}

pub async fn get_project_evidences<R>(
    repository: &R,
    project_id: &str,
) -> Result<Vec<Evidence>, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    repository
        .list_evidences(&project.id)
        .await
        .map_err(map_evidence_read_error)
}

pub async fn get_evidence<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
) -> Result<EvidenceRecord, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    let evidence = require_evidence(repository, evidence_id).await?;
    evidence.ensure_project(&project.id)?;
    let links = repository
        .list_evidence_links(evidence.id())
        .await
        .map_err(map_evidence_read_error)?;
    Ok(EvidenceRecord { evidence, links })
}

pub async fn update_evidence<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
    input: UpdateEvidenceInput,
) -> Result<Evidence, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_evidence_changes()?;
    let mut evidence = require_evidence(repository, evidence_id).await?;
    evidence.ensure_project(&project.id)?;
    if evidence.is_git_evidence() {
        return Err(crate::domain::DomainError::GitEvidenceContentReadOnly.into());
    }
    ensure_version(input.expected_version, evidence.version())?;
    let expected_version = input.expected_version;
    let details = details(
        input.title,
        input.description,
        input.evidence_type,
        input.source_type,
        input.source_uri,
    )?;
    if matches!(
        details.evidence_type,
        EvidenceType::GitCommit | EvidenceType::GitStatus
    ) {
        return Err(crate::domain::DomainError::GitEvidenceRequiresRepositoryIntegration.into());
    }
    evidence.update(details, Utc::now())?;
    persist_update(repository, &evidence, expected_version).await?;
    Ok(evidence)
}

pub async fn verify_evidence<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
    input: ChangeEvidenceStatusInput,
) -> Result<Evidence, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    change_status(repository, project_id, evidence_id, input, true).await
}

pub async fn invalidate_evidence<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
    input: ChangeEvidenceStatusInput,
) -> Result<Evidence, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    change_status(repository, project_id, evidence_id, input, false).await
}

async fn change_status<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
    input: ChangeEvidenceStatusInput,
    verify: bool,
) -> Result<Evidence, ApplicationError>
where
    R: WorkspaceRepository + ProjectRepository + EvidenceRepository + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_evidence_changes()?;
    let mut evidence = require_evidence(repository, evidence_id).await?;
    evidence.ensure_project(&project.id)?;
    ensure_version(input.expected_version, evidence.version())?;
    let expected_version = input.expected_version;
    if verify {
        evidence.verify(Utc::now())?;
    } else {
        evidence.invalidate(Utc::now())?;
    }
    persist_update(repository, &evidence, expected_version).await?;
    Ok(evidence)
}

pub async fn link_evidence<R>(
    repository: &R,
    project_id: &str,
    evidence_id: &str,
    input: LinkEvidenceInput,
) -> Result<EvidenceLink, ApplicationError>
where
    R: WorkspaceRepository
        + ProjectRepository
        + SnapshotRepository
        + SessionRepository
        + EvidenceRepository
        + ?Sized,
{
    let project = project::get_project(repository, project_id).await?;
    project.ensure_accepts_evidence_changes()?;
    let evidence = require_evidence(repository, evidence_id).await?;
    evidence.ensure_project(&project.id)?;

    let target_type = EvidenceTargetType::from_str(&input.target_type)?;
    validate_target(repository, &project, target_type, &input.target_id).await?;
    let link = EvidenceLink::create(
        Uuid::now_v7().to_string(),
        evidence.id().to_owned(),
        target_type,
        input.target_id,
        EvidenceRelationType::from_str(&input.relation_type)?,
    )?;

    match repository.insert_evidence_link(&link).await {
        Ok(()) => Ok(link),
        Err(RepositoryError::Conflict) => Err(ApplicationError::EvidenceLinkAlreadyExists),
        Err(RepositoryError::EvidenceTargetInvalid) => {
            Err(ApplicationError::EvidenceTargetProjectMismatch)
        }
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptEvidenceChanges.into())
        }
        Err(error) => Err(map_read_error(error)),
    }
}

async fn validate_target<R>(
    repository: &R,
    project: &crate::domain::Project,
    target_type: EvidenceTargetType,
    target_id: &str,
) -> Result<(), ApplicationError>
where
    R: ProjectRepository + SnapshotRepository + SessionRepository + ?Sized,
{
    match target_type {
        EvidenceTargetType::Project => {
            let target = repository
                .get_project(&project.workspace_id, target_id)
                .await
                .map_err(map_read_error)?
                .ok_or(ApplicationError::EvidenceTargetNotFound)?;
            if target.id != project.id {
                return Err(ApplicationError::EvidenceTargetProjectMismatch);
            }
        }
        EvidenceTargetType::Snapshot => {
            let target = repository
                .get_snapshot(target_id)
                .await
                .map_err(map_read_error)?
                .ok_or(ApplicationError::EvidenceTargetNotFound)?;
            if target.project_id() != project.id {
                return Err(ApplicationError::EvidenceTargetProjectMismatch);
            }
        }
        EvidenceTargetType::Session => {
            let target = repository
                .get_session(target_id)
                .await
                .map_err(map_read_error)?
                .ok_or(ApplicationError::EvidenceTargetNotFound)?;
            if target.project_id() != project.id {
                return Err(ApplicationError::EvidenceTargetProjectMismatch);
            }
        }
    }
    Ok(())
}

fn details(
    title: String,
    description: String,
    evidence_type: String,
    source_type: String,
    source_uri: String,
) -> Result<EvidenceDetails, ApplicationError> {
    Ok(EvidenceDetails {
        title,
        description,
        evidence_type: EvidenceType::from_str(&evidence_type)?,
        source_type: EvidenceSourceType::from_str(&source_type)?,
        source_uri,
    })
}

async fn require_evidence<R>(
    repository: &R,
    evidence_id: &str,
) -> Result<Evidence, ApplicationError>
where
    R: EvidenceRepository + ?Sized,
{
    repository
        .get_evidence(evidence_id)
        .await
        .map_err(map_evidence_read_error)?
        .ok_or(ApplicationError::EvidenceNotFound)
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), ApplicationError> {
    if expected != actual {
        return Err(ApplicationError::EvidenceVersionConflict { expected, actual });
    }
    Ok(())
}

async fn persist_update<R>(
    repository: &R,
    evidence: &Evidence,
    expected_version: i64,
) -> Result<(), ApplicationError>
where
    R: EvidenceRepository + ?Sized,
{
    match repository.update_evidence(evidence, expected_version).await {
        Ok(()) => Ok(()),
        Err(RepositoryError::OptimisticConflict) => {
            let current = require_evidence(repository, evidence.id()).await?;
            Err(ApplicationError::EvidenceVersionConflict {
                expected: expected_version,
                actual: current.version(),
            })
        }
        Err(RepositoryError::ProjectArchived) => {
            Err(crate::domain::DomainError::ArchivedProjectCannotAcceptEvidenceChanges.into())
        }
        Err(error) => Err(map_evidence_read_error(error)),
    }
}

fn map_evidence_read_error(error: RepositoryError) -> ApplicationError {
    match error {
        RepositoryError::EvidenceTargetInvalid => ApplicationError::CorruptData(error.to_string()),
        other => map_read_error(other),
    }
}
