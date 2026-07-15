use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::DomainError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageMode {
    System,
    SimplifiedChinese,
    English,
}

impl LanguageMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::SimplifiedChinese => "zh-CN",
            Self::English => "en-US",
        }
    }
}

impl FromStr for LanguageMode {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "system" => Ok(Self::System),
            "zh-CN" => Ok(Self::SimplifiedChinese),
            "en-US" => Ok(Self::English),
            other => Err(DomainError::InvalidLanguageMode(other.to_owned())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPreferences {
    language_mode: LanguageMode,
    show_technical_terms: bool,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl AppPreferences {
    pub fn restore(
        language_mode: LanguageMode,
        show_technical_terms: bool,
        version: i64,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if version < 1 {
            return Err(DomainError::InvalidAppPreferencesVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidAppPreferencesTimestamps);
        }
        Ok(Self {
            language_mode,
            show_technical_terms,
            version,
            created_at,
            updated_at,
        })
    }

    pub fn update(
        &self,
        language_mode: LanguageMode,
        show_technical_terms: bool,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let version = self
            .version
            .checked_add(1)
            .ok_or(DomainError::AppPreferencesVersionOverflow)?;
        Self::restore(
            language_mode,
            show_technical_terms,
            version,
            self.created_at,
            now,
        )
    }

    pub const fn language_mode(&self) -> LanguageMode {
        self.language_mode
    }

    pub const fn show_technical_terms(&self) -> bool {
        self.show_technical_terms
    }

    pub const fn version(&self) -> i64 {
        self.version
    }

    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_and_updates_preferences() {
        let created_at = Utc::now();
        let value = AppPreferences::restore(LanguageMode::System, false, 1, created_at, created_at)
            .unwrap();
        let updated = value
            .update(LanguageMode::SimplifiedChinese, true, created_at)
            .unwrap();
        assert_eq!(updated.language_mode(), LanguageMode::SimplifiedChinese);
        assert!(updated.show_technical_terms());
        assert_eq!(updated.version(), 2);
    }

    #[test]
    fn rejects_invalid_language_and_version() {
        assert!(LanguageMode::from_str("fr-FR").is_err());
        let now = Utc::now();
        assert!(AppPreferences::restore(LanguageMode::System, false, 0, now, now).is_err());
    }
}
