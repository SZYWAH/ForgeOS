use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use super::DomainError;

const MIN_SLUG_LENGTH: usize = 2;
const MAX_SLUG_LENGTH: usize = 64;
const MAX_NAME_LENGTH: usize = 200;
const MAX_DESCRIPTION_LENGTH: usize = 2_000;
const MAX_LONG_TEXT_LENGTH: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectStatus {
    Idea,
    Planned,
    Active,
    Paused,
    Maintenance,
    Archived,
}

impl ProjectStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idea => "idea",
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Maintenance => "maintenance",
            Self::Archived => "archived",
        }
    }
}

impl fmt::Display for ProjectStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProjectStatus {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "idea" => Ok(Self::Idea),
            "planned" => Ok(Self::Planned),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "maintenance" => Ok(Self::Maintenance),
            "archived" => Ok(Self::Archived),
            _ => Err(DomainError::InvalidProjectStatus(value.to_owned())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleStage {
    Discovery,
    Definition,
    Design,
    Implementation,
    Validation,
    Release,
    Operation,
    Retrospective,
}

impl LifecycleStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Definition => "definition",
            Self::Design => "design",
            Self::Implementation => "implementation",
            Self::Validation => "validation",
            Self::Release => "release",
            Self::Operation => "operation",
            Self::Retrospective => "retrospective",
        }
    }
}

impl fmt::Display for LifecycleStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for LifecycleStage {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "discovery" => Ok(Self::Discovery),
            "definition" => Ok(Self::Definition),
            "design" => Ok(Self::Design),
            "implementation" => Ok(Self::Implementation),
            "validation" => Ok(Self::Validation),
            "release" => Ok(Self::Release),
            "operation" => Ok(Self::Operation),
            "retrospective" => Ok(Self::Retrospective),
            _ => Err(DomainError::InvalidLifecycleStage(value.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDetails {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: ProjectStatus,
    pub lifecycle_stage: LifecycleStage,
    pub priority: i32,
}

impl ProjectDetails {
    pub fn validated(self) -> Result<Self, DomainError> {
        Ok(Self {
            slug: validate_slug(self.slug)?,
            name: required("project name", self.name, MAX_NAME_LENGTH)?,
            description: text("description", self.description, MAX_DESCRIPTION_LENGTH)?,
            vision: text("vision", self.vision, MAX_LONG_TEXT_LENGTH)?,
            target_user: text("target user", self.target_user, MAX_LONG_TEXT_LENGTH)?,
            scope: text("scope", self.scope, MAX_LONG_TEXT_LENGTH)?,
            non_goals: text("non-goals", self.non_goals, MAX_LONG_TEXT_LENGTH)?,
            tech_stack: text("tech stack", self.tech_stack, MAX_LONG_TEXT_LENGTH)?,
            status: self.status,
            lifecycle_stage: self.lifecycle_stage,
            priority: validate_priority(self.priority)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectUpdate(pub ProjectDetails);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    pub id: String,
    pub workspace_id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: ProjectStatus,
    pub lifecycle_stage: LifecycleStage,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

impl Project {
    pub fn new(
        id: String,
        workspace_id: String,
        details: ProjectDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let details = details.validated()?;
        if details.status == ProjectStatus::Archived {
            return Err(DomainError::ArchiveRequiresDedicatedOperation);
        }

        Self::restore_validated(id, workspace_id, details, now, now, 1)
    }

    pub fn restore(
        id: String,
        workspace_id: String,
        details: ProjectDetails,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let details = details.validated()?;

        Self::restore_validated(id, workspace_id, details, created_at, updated_at, version)
    }

    fn restore_validated(
        id: String,
        workspace_id: String,
        details: ProjectDetails,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let id = required("project id", id, 64)?;
        let workspace_id = required("workspace id", workspace_id, 64)?;

        if version < 1 {
            return Err(DomainError::InvalidVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidTimestamps);
        }

        Ok(Self {
            id,
            workspace_id,
            slug: details.slug,
            name: details.name,
            description: details.description,
            vision: details.vision,
            target_user: details.target_user,
            scope: details.scope,
            non_goals: details.non_goals,
            tech_stack: details.tech_stack,
            status: details.status,
            lifecycle_stage: details.lifecycle_stage,
            priority: details.priority,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn update(&mut self, update: ProjectUpdate, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::ArchivedProjectCannotBeEdited);
        }

        let details = update.0.validated()?;
        if details.status == ProjectStatus::Archived {
            return Err(DomainError::ArchiveRequiresDedicatedOperation);
        }

        self.slug = details.slug;
        self.name = details.name;
        self.description = details.description;
        self.vision = details.vision;
        self.target_user = details.target_user;
        self.scope = details.scope;
        self.non_goals = details.non_goals;
        self.tech_stack = details.tech_stack;
        self.status = details.status;
        self.lifecycle_stage = details.lifecycle_stage;
        self.priority = details.priority;
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        Ok(())
    }

    pub fn archive(&mut self, now: DateTime<Utc>) -> bool {
        if self.status == ProjectStatus::Archived {
            return false;
        }

        self.status = ProjectStatus::Archived;
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
        true
    }

    pub fn details(&self) -> ProjectDetails {
        ProjectDetails {
            slug: self.slug.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            vision: self.vision.clone(),
            target_user: self.target_user.clone(),
            scope: self.scope.clone(),
            non_goals: self.non_goals.clone(),
            tech_stack: self.tech_stack.clone(),
            status: self.status,
            lifecycle_stage: self.lifecycle_stage,
            priority: self.priority,
        }
    }

    pub fn ensure_accepts_snapshot_changes(&self) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::ArchivedProjectCannotAcceptSnapshots);
        }
        Ok(())
    }

    pub fn ensure_accepts_session_changes(&self) -> Result<(), DomainError> {
        if self.status == ProjectStatus::Archived {
            return Err(DomainError::ArchivedProjectCannotAcceptSessionChanges);
        }
        Ok(())
    }
}

fn validate_slug(value: String) -> Result<String, DomainError> {
    let value = value.trim().to_ascii_lowercase();
    let length = value.chars().count();
    if length == 0 {
        return Err(DomainError::Required { field: "slug" });
    }

    let valid = (MIN_SLUG_LENGTH..=MAX_SLUG_LENGTH).contains(&length)
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value.bytes().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == b'-'
        });

    if !valid {
        return Err(DomainError::InvalidSlug);
    }

    Ok(value)
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

fn validate_priority(value: i32) -> Result<i32, DomainError> {
    if !(0..=100).contains(&value) {
        return Err(DomainError::InvalidPriority);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details(status: ProjectStatus) -> ProjectDetails {
        ProjectDetails {
            slug: "forgeos".into(),
            name: "ForgeOS".into(),
            description: String::new(),
            vision: String::new(),
            target_user: String::new(),
            scope: String::new(),
            non_goals: String::new(),
            tech_stack: String::new(),
            status,
            lifecycle_stage: LifecycleStage::Implementation,
            priority: 80,
        }
    }

    #[test]
    fn slug_is_normalized_and_validated() {
        let mut input = details(ProjectStatus::Active);
        input.slug = "  ForgeOS  ".into();
        assert_eq!(input.validated().expect("valid details").slug, "forgeos");

        let mut invalid = details(ProjectStatus::Active);
        invalid.slug = "forge_os".into();
        assert_eq!(invalid.validated(), Err(DomainError::InvalidSlug));

        let mut too_short = details(ProjectStatus::Active);
        too_short.slug = "f".into();
        assert_eq!(too_short.validated(), Err(DomainError::InvalidSlug));
    }

    #[test]
    fn archive_is_a_dedicated_transition() {
        let now = Utc::now();
        assert_eq!(
            Project::new(
                "archived-project".into(),
                "workspace-id".into(),
                details(ProjectStatus::Archived),
                now,
            ),
            Err(DomainError::ArchiveRequiresDedicatedOperation)
        );

        let mut project = Project::new(
            "project-id".into(),
            "workspace-id".into(),
            details(ProjectStatus::Active),
            now,
        )
        .expect("project should be valid");

        let result = project.update(ProjectUpdate(details(ProjectStatus::Archived)), now);
        assert_eq!(result, Err(DomainError::ArchiveRequiresDedicatedOperation));

        assert!(project.archive(now));
        assert_eq!(project.status, ProjectStatus::Archived);
        assert_eq!(project.version, 2);
        assert_eq!(
            project.ensure_accepts_snapshot_changes(),
            Err(DomainError::ArchivedProjectCannotAcceptSnapshots)
        );
        assert_eq!(
            project.update(ProjectUpdate(details(ProjectStatus::Active)), now),
            Err(DomainError::ArchivedProjectCannotBeEdited)
        );
    }

    #[test]
    fn project_timestamps_do_not_move_backwards() {
        let created_at = Utc::now();
        let earlier = created_at - chrono::Duration::hours(1);
        let mut project = Project::new(
            "project-id".into(),
            "workspace-id".into(),
            details(ProjectStatus::Active),
            created_at,
        )
        .expect("project should be valid");

        project
            .update(ProjectUpdate(details(ProjectStatus::Paused)), earlier)
            .expect("a clock rollback must not corrupt an update");
        assert_eq!(project.updated_at, created_at);

        assert!(project.archive(earlier));
        assert_eq!(project.updated_at, created_at);
    }
}
