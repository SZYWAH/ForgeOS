use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AchievementCategory, AchievementDetails, AchievementSource, AchievementSourceMode,
        AchievementSourceType, DomainError, Project, ProjectAchievement, ProjectStatus,
    },
    ports::{ProjectMemoryKernelRepository, RepositoryError},
};

use super::{DevelopmentHistoryItem, ProjectMemoryQueryKernel};

#[derive(Debug, Error)]
pub enum AchievementError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before using achievements")]
    WorkspaceNotFound,
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Achievement not found")]
    AchievementNotFound,
    #[error("Development history item not found")]
    HistoryItemNotFound,
    #[error("the Project is archived")]
    ProjectArchived,
    #[error("achievement version conflict")]
    VersionConflict,
    #[error("achievement source belongs to another Project")]
    SourceProjectMismatch,
    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct AchievementInput {
    pub category: String,
    pub title: String,
    pub problem: String,
    pub approach: String,
    pub result: String,
    pub validation: String,
    pub impact: String,
    pub technologies: Vec<String>,
    pub resume_bullet: String,
    pub featured: bool,
}

impl AchievementInput {
    fn into_details(self) -> Result<AchievementDetails, DomainError> {
        Ok(AchievementDetails {
            category: AchievementCategory::from_str(&self.category)?,
            title: self.title,
            problem: self.problem,
            approach: self.approach,
            result: self.result,
            validation: self.validation,
            impact: self.impact,
            technologies: self.technologies,
            resume_bullet: self.resume_bullet,
            featured: self.featured,
        })
    }
}

#[derive(Clone)]
pub struct AchievementKernel {
    repository: Arc<dyn ProjectMemoryKernelRepository>,
    memory: ProjectMemoryQueryKernel,
}

impl AchievementKernel {
    pub fn new(repository: Arc<dyn ProjectMemoryKernelRepository>) -> Self {
        Self {
            memory: ProjectMemoryQueryKernel::new(repository.clone()),
            repository,
        }
    }

    pub async fn list(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAchievement>, AchievementError> {
        self.require_project(project_id).await?;
        self.repository
            .list_achievements(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn get(
        &self,
        project_id: &str,
        achievement_id: &str,
    ) -> Result<ProjectAchievement, AchievementError> {
        let value = self
            .repository
            .get_achievement(achievement_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AchievementError::AchievementNotFound)?;
        value.ensure_project(project_id)?;
        Ok(value)
    }

    pub async fn create(
        &self,
        project_id: &str,
        input: AchievementInput,
    ) -> Result<ProjectAchievement, AchievementError> {
        self.require_writable_project(project_id).await?;
        let value = ProjectAchievement::draft(
            Uuid::now_v7().to_string(),
            project_id.into(),
            input.into_details()?,
            AchievementSourceMode::Manual,
            None,
            Utc::now(),
        )?;
        self.repository
            .insert_achievement(&value, &[])
            .await
            .map_err(map_repository_error)?;
        Ok(value)
    }

    pub async fn create_from_history(
        &self,
        project_id: &str,
        history_id: &str,
    ) -> Result<ProjectAchievement, AchievementError> {
        self.require_writable_project(project_id).await?;
        let item = self.find_history_item(project_id, history_id).await?;
        let id = Uuid::now_v7().to_string();
        let value = ProjectAchievement::draft(
            id.clone(),
            project_id.into(),
            AchievementDetails {
                category: if item.outcome.is_empty() {
                    AchievementCategory::TechnicalHighlight
                } else {
                    AchievementCategory::Feature
                },
                title: item.title.clone(),
                problem: String::new(),
                approach: item.activity.clone(),
                result: item.outcome.clone(),
                validation: item.validation.clone(),
                impact: String::new(),
                technologies: vec![],
                resume_bullet: String::new(),
                featured: false,
            },
            AchievementSourceMode::Manual,
            item.pulse_id.clone(),
            Utc::now(),
        )?;
        let sources = achievement_sources(&id, project_id, &item)?;
        self.repository
            .insert_achievement(&value, &sources)
            .await
            .map_err(map_repository_error)?;
        Ok(value)
    }

    pub async fn update(
        &self,
        project_id: &str,
        achievement_id: &str,
        expected_version: i64,
        input: AchievementInput,
    ) -> Result<ProjectAchievement, AchievementError> {
        self.require_writable_project(project_id).await?;
        let mut value = self.get(project_id, achievement_id).await?;
        if value.version != expected_version {
            return Err(AchievementError::VersionConflict);
        }
        value.update(input.into_details()?, Utc::now())?;
        self.persist_update(&value, expected_version).await?;
        Ok(value)
    }

    pub async fn confirm(
        &self,
        project_id: &str,
        achievement_id: &str,
        expected_version: i64,
    ) -> Result<ProjectAchievement, AchievementError> {
        self.require_writable_project(project_id).await?;
        let mut value = self.get(project_id, achievement_id).await?;
        if value.version != expected_version {
            return Err(AchievementError::VersionConflict);
        }
        value.confirm(Utc::now())?;
        self.persist_update(&value, expected_version).await?;
        Ok(value)
    }

    pub async fn archive(
        &self,
        project_id: &str,
        achievement_id: &str,
        expected_version: i64,
    ) -> Result<ProjectAchievement, AchievementError> {
        self.require_writable_project(project_id).await?;
        let mut value = self.get(project_id, achievement_id).await?;
        if value.version != expected_version {
            return Err(AchievementError::VersionConflict);
        }
        value.archive(Utc::now())?;
        self.persist_update(&value, expected_version).await?;
        Ok(value)
    }

    async fn persist_update(
        &self,
        value: &ProjectAchievement,
        expected_version: i64,
    ) -> Result<(), AchievementError> {
        self.repository
            .update_achievement(value, expected_version)
            .await
            .map_err(map_repository_error)
    }

    async fn find_history_item(
        &self,
        project_id: &str,
        history_id: &str,
    ) -> Result<DevelopmentHistoryItem, AchievementError> {
        let mut cursor = None;
        for _ in 0..100 {
            let page = self
                .memory
                .list_development_history(project_id, cursor.as_deref(), 50)
                .await
                .map_err(|error| AchievementError::Storage(error.to_string()))?;
            if let Some(item) = page.items.into_iter().find(|item| item.id == history_id) {
                return Ok(item);
            }
            cursor = page.next_cursor;
            if cursor.is_none() {
                break;
            }
        }
        Err(AchievementError::HistoryItemNotFound)
    }

    async fn require_project(&self, project_id: &str) -> Result<Project, AchievementError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(AchievementError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AchievementError::ProjectNotFound)
    }

    async fn require_writable_project(
        &self,
        project_id: &str,
    ) -> Result<Project, AchievementError> {
        let project = self.require_project(project_id).await?;
        if project.status == ProjectStatus::Archived {
            return Err(AchievementError::ProjectArchived);
        }
        Ok(project)
    }
}

fn achievement_sources(
    achievement_id: &str,
    project_id: &str,
    item: &DevelopmentHistoryItem,
) -> Result<Vec<AchievementSource>, DomainError> {
    let mut values = Vec::new();
    let mut push = |source_type, source_id: &str| -> Result<(), DomainError> {
        values.push(AchievementSource::create(
            achievement_id.into(),
            project_id.into(),
            source_type,
            source_id.into(),
            values.len() as i64,
        )?);
        Ok(())
    };
    if let Some(id) = item.pulse_id.as_deref() {
        push(AchievementSourceType::Pulse, id)?;
    }
    if let Some(id) = item.session_id.as_deref() {
        push(AchievementSourceType::Session, id)?;
    }
    for id in &item.source_observation_ids {
        push(AchievementSourceType::CodexObservation, id)?;
    }
    for id in &item.source_commit_observation_ids {
        push(AchievementSourceType::RepositoryCommit, id)?;
    }
    Ok(values)
}

fn map_repository_error(error: RepositoryError) -> AchievementError {
    match error {
        RepositoryError::OptimisticConflict => AchievementError::VersionConflict,
        RepositoryError::EvidenceTargetInvalid => AchievementError::SourceProjectMismatch,
        RepositoryError::CorruptData(message) => AchievementError::CorruptData(message),
        other => AchievementError::Storage(other.to_string()),
    }
}
