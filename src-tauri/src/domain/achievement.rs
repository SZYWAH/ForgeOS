use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_TEXT: usize = 20_000;

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
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(DomainError::$error(value.into())),
                }
            }
        }
    };
}

string_enum!(AchievementCategory, InvalidAchievementCategory, {
    Feature => "feature",
    ProblemSolution => "problem_solution",
    Release => "release",
    TechnicalHighlight => "technical_highlight"
});
string_enum!(AchievementStatus, InvalidAchievementStatus, {
    Draft => "draft",
    Confirmed => "confirmed",
    Archived => "archived"
});
string_enum!(AchievementSourceMode, InvalidAchievementSourceMode, {
    Manual => "manual",
    PulseRule => "pulse_rule"
});
string_enum!(AchievementSourceType, InvalidAchievementSourceType, {
    Pulse => "pulse",
    Snapshot => "snapshot",
    Session => "session",
    Evidence => "evidence",
    CodexObservation => "codex_observation",
    RepositoryCommit => "repository_commit"
});

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AchievementDetails {
    pub category: AchievementCategory,
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

impl AchievementDetails {
    fn validated(self) -> Result<Self, DomainError> {
        Ok(Self {
            category: self.category,
            title: text("achievement title", self.title, 500)?,
            problem: text("achievement problem", self.problem, MAX_TEXT)?,
            approach: text("achievement approach", self.approach, MAX_TEXT)?,
            result: text("achievement result", self.result, MAX_TEXT)?,
            validation: text("achievement validation", self.validation, MAX_TEXT)?,
            impact: text("achievement impact", self.impact, MAX_TEXT)?,
            technologies: normalize_list(self.technologies)?,
            resume_bullet: text("achievement resume bullet", self.resume_bullet, 2_000)?,
            featured: self.featured,
        })
    }

    fn ensure_confirmable(&self) -> Result<(), DomainError> {
        if self.title.is_empty()
            || self.result.is_empty()
            || (self.approach.is_empty() && self.validation.is_empty() && self.impact.is_empty())
        {
            return Err(DomainError::AchievementConfirmationRequired);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectAchievement {
    pub id: String,
    pub project_id: String,
    pub details: AchievementDetails,
    pub status: AchievementStatus,
    pub source_mode: AchievementSourceMode,
    pub source_pulse_id: Option<String>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

impl ProjectAchievement {
    pub fn draft(
        id: String,
        project_id: String,
        details: AchievementDetails,
        source_mode: AchievementSourceMode,
        source_pulse_id: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(
            id,
            project_id,
            details,
            AchievementStatus::Draft,
            source_mode,
            source_pulse_id,
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
        details: AchievementDetails,
        status: AchievementStatus,
        source_mode: AchievementSourceMode,
        source_pulse_id: Option<String>,
        confirmed_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let details = details.validated()?;
        if version < 1 || updated_at < created_at {
            return Err(DomainError::InvalidAchievementLifecycle);
        }
        if source_mode == AchievementSourceMode::PulseRule && source_pulse_id.is_none() {
            return Err(DomainError::InvalidAchievementLifecycle);
        }
        if status == AchievementStatus::Confirmed {
            details.ensure_confirmable()?;
            if confirmed_at.is_none() {
                return Err(DomainError::InvalidAchievementLifecycle);
            }
        } else if confirmed_at.is_some() && status == AchievementStatus::Draft {
            return Err(DomainError::InvalidAchievementLifecycle);
        }
        Ok(Self {
            id: required("achievement id", id, 128)?,
            project_id: required("project id", project_id, 128)?,
            details,
            status,
            source_mode,
            source_pulse_id: source_pulse_id
                .map(|value| required("source pulse id", value, 128))
                .transpose()?,
            confirmed_at,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn ensure_project(&self, project_id: &str) -> Result<(), DomainError> {
        if self.project_id != project_id {
            return Err(DomainError::AchievementProjectMismatch);
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        details: AchievementDetails,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status == AchievementStatus::Archived {
            return Err(DomainError::AchievementArchived);
        }
        let details = details.validated()?;
        if self.status == AchievementStatus::Confirmed {
            details.ensure_confirmable()?;
        }
        self.details = details;
        self.bump(now)
    }

    pub fn confirm(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != AchievementStatus::Draft {
            return Err(DomainError::AchievementNotDraft);
        }
        self.details.ensure_confirmable()?;
        self.status = AchievementStatus::Confirmed;
        self.confirmed_at = Some(now);
        self.bump(now)
    }

    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == AchievementStatus::Archived {
            return Err(DomainError::AchievementArchived);
        }
        self.status = AchievementStatus::Archived;
        self.bump(now)
    }

    fn bump(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::AchievementVersionOverflow)?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AchievementSource {
    pub achievement_id: String,
    pub project_id: String,
    pub source_type: AchievementSourceType,
    pub source_id: String,
    pub source_order: i64,
}

impl AchievementSource {
    pub fn create(
        achievement_id: String,
        project_id: String,
        source_type: AchievementSourceType,
        source_id: String,
        source_order: i64,
    ) -> Result<Self, DomainError> {
        if source_order < 0 {
            return Err(DomainError::InvalidAchievementSourceOrder);
        }
        Ok(Self {
            achievement_id: required("achievement id", achievement_id, 128)?,
            project_id: required("project id", project_id, 128)?,
            source_type,
            source_id: required("achievement source id", source_id, 128)?,
            source_order,
        })
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

fn normalize_list(values: Vec<String>) -> Result<Vec<String>, DomainError> {
    let mut result = Vec::new();
    for value in values {
        let value = text("achievement technology", value, 200)?;
        if !value.is_empty() && !result.contains(&value) {
            result.push(value);
        }
    }
    if result.len() > 30 {
        return Err(DomainError::TooManyItems {
            field: "achievement technologies",
            max: 30,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> AchievementDetails {
        AchievementDetails {
            category: AchievementCategory::Feature,
            title: "Project memory".into(),
            problem: String::new(),
            approach: "Aggregate activity".into(),
            result: "Context restored quickly".into(),
            validation: String::new(),
            impact: String::new(),
            technologies: vec!["Rust".into(), "Rust".into()],
            resume_bullet: String::new(),
            featured: false,
        }
    }

    #[test]
    fn draft_requires_human_confirmation_and_preserves_history() {
        let now = Utc::now();
        let mut value = ProjectAchievement::draft(
            "a-1".into(),
            "p-1".into(),
            details(),
            AchievementSourceMode::Manual,
            None,
            now,
        )
        .unwrap();
        assert_eq!(value.status, AchievementStatus::Draft);
        value.confirm(now).unwrap();
        assert_eq!(value.status, AchievementStatus::Confirmed);
        value.archive(now).unwrap();
        assert_eq!(value.status, AchievementStatus::Archived);
    }
}
