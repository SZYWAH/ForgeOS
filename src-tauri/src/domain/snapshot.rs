use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_TEXT_LENGTH: usize = 20_000;
const MAX_LIST_ITEM_LENGTH: usize = 2_000;
const MAX_NEXT_STEPS: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotDetails {
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
}

impl SnapshotDetails {
    pub fn validated(self) -> Result<Self, DomainError> {
        let next_steps = normalize_list("next step", self.next_steps)?;
        if next_steps.len() > MAX_NEXT_STEPS {
            return Err(DomainError::TooManyNextSteps {
                max: MAX_NEXT_STEPS,
            });
        }

        Ok(Self {
            current_goal: required("current goal", self.current_goal, MAX_TEXT_LENGTH)?,
            status_summary: required("status summary", self.status_summary, MAX_TEXT_LENGTH)?,
            achievements: normalize_list("achievement", self.achievements)?,
            blockers: normalize_list("blocker", self.blockers)?,
            next_steps,
        })
    }
}

/// An immutable, confirmed statement of a Project's current engineering state.
///
/// Changes are represented by a new `ProjectSnapshot` with the next version;
/// this entity intentionally exposes no in-place update operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectSnapshot {
    id: String,
    project_id: String,
    version: i64,
    current_goal: String,
    status_summary: String,
    achievements: Vec<String>,
    blockers: Vec<String>,
    next_steps: Vec<String>,
    confirmed_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ProjectSnapshot {
    pub fn new(
        id: String,
        project_id: String,
        details: SnapshotDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(id, project_id, 1, details, now, now, now)
    }

    pub fn next_version(
        id: String,
        project_id: String,
        current: &Self,
        details: SnapshotDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let project_id = required("project id", project_id, 64)?;
        if project_id != current.project_id {
            return Err(DomainError::SnapshotProjectMismatch);
        }

        let version = current
            .version
            .checked_add(1)
            .ok_or(DomainError::SnapshotVersionOverflow)?;
        let effective_now = now.max(current.updated_at).max(current.confirmed_at);

        Self::restore(
            id,
            project_id,
            version,
            details,
            effective_now,
            effective_now,
            effective_now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: String,
        project_id: String,
        version: i64,
        details: SnapshotDetails,
        confirmed_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let id = required("Snapshot id", id, 64)?;
        let project_id = required("project id", project_id, 64)?;
        let details = details.validated()?;

        if version < 1 {
            return Err(DomainError::InvalidSnapshotVersion);
        }
        if confirmed_at < created_at || updated_at < confirmed_at {
            return Err(DomainError::InvalidSnapshotTimestamps);
        }

        Ok(Self {
            id,
            project_id,
            version,
            current_goal: details.current_goal,
            status_summary: details.status_summary,
            achievements: details.achievements,
            blockers: details.blockers,
            next_steps: details.next_steps,
            confirmed_at,
            created_at,
            updated_at,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn version(&self) -> i64 {
        self.version
    }

    pub fn current_goal(&self) -> &str {
        &self.current_goal
    }

    pub fn status_summary(&self) -> &str {
        &self.status_summary
    }

    pub fn achievements(&self) -> &[String] {
        &self.achievements
    }

    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }

    pub fn next_steps(&self) -> &[String] {
        &self.next_steps
    }

    pub fn confirmed_at(&self) -> DateTime<Utc> {
        self.confirmed_at
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

fn normalize_list(field: &'static str, values: Vec<String>) -> Result<Vec<String>, DomainError> {
    let mut normalized = Vec::with_capacity(values.len());
    for value in values {
        let value = value.trim().to_owned();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_LIST_ITEM_LENGTH {
            return Err(DomainError::TooLong {
                field,
                max: MAX_LIST_ITEM_LENGTH,
            });
        }
        normalized.push(value);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::*;

    fn details() -> SnapshotDetails {
        SnapshotDetails {
            current_goal: "  Ship the Current Snapshot kernel  ".into(),
            status_summary: "  D1.2 is in implementation  ".into(),
            achievements: vec![" Migration designed ".into(), "Migration designed".into()],
            blockers: vec![" ".into()],
            next_steps: vec!["Write tests".into(), "Run checks".into()],
        }
    }

    #[test]
    fn creates_version_one_and_normalizes_lists() {
        let now = Utc::now();
        let snapshot =
            ProjectSnapshot::new("snapshot-1".into(), "project-1".into(), details(), now)
                .expect("Snapshot should be valid");

        assert_eq!(snapshot.version(), 1);
        assert_eq!(snapshot.current_goal(), "Ship the Current Snapshot kernel");
        assert_eq!(
            snapshot.achievements(),
            ["Migration designed", "Migration designed"]
        );
        assert!(snapshot.blockers().is_empty());
    }

    #[test]
    fn rejects_more_than_three_normalized_next_steps() {
        let mut input = details();
        input.next_steps = vec!["one".into(), "one".into(), "three".into(), "four".into()];

        assert_eq!(
            ProjectSnapshot::new("snapshot-1".into(), "project-1".into(), input, Utc::now()),
            Err(DomainError::TooManyNextSteps { max: 3 })
        );
    }

    #[test]
    fn next_version_is_project_bound_and_time_monotonic() {
        let now = Utc::now();
        let current = ProjectSnapshot::new("snapshot-1".into(), "project-1".into(), details(), now)
            .expect("Snapshot should be valid");

        assert_eq!(
            ProjectSnapshot::next_version(
                "snapshot-2".into(),
                "project-2".into(),
                &current,
                details(),
                now,
            ),
            Err(DomainError::SnapshotProjectMismatch)
        );

        let next = ProjectSnapshot::next_version(
            "snapshot-2".into(),
            "project-1".into(),
            &current,
            details(),
            now - Duration::hours(1),
        )
        .expect("clock rollback must not corrupt Snapshot chronology");
        assert_eq!(next.version(), 2);
        assert_eq!(next.confirmed_at(), now);
        assert_eq!(next.created_at(), now);
        assert_eq!(next.updated_at(), now);
    }
}
