use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_NAME_LENGTH: usize = 120;
const MAX_TIMEZONE_LENGTH: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(
        id: String,
        name: String,
        timezone: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(id, name, timezone, now, now)
    }

    pub fn restore(
        id: String,
        name: String,
        timezone: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let id = required("workspace id", id, 64)?;
        let name = required("workspace name", name, MAX_NAME_LENGTH)?;
        let timezone = required("workspace timezone", timezone, MAX_TIMEZONE_LENGTH)?;

        if updated_at < created_at {
            return Err(DomainError::InvalidTimestamps);
        }

        Ok(Self {
            id,
            name,
            timezone,
            created_at,
            updated_at,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_requires_a_name_and_timezone() {
        let now = Utc::now();
        assert!(matches!(
            Workspace::new("id".into(), " ".into(), "Asia/Shanghai".into(), now),
            Err(DomainError::Required {
                field: "workspace name"
            })
        ));
    }
}
