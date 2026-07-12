use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_NAME_LENGTH: usize = 100;
const MAX_BASE_URL_LENGTH: usize = 2_048;
const MAX_MODEL_LENGTH: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiProviderDetails {
    pub name: String,
    pub base_url: String,
    pub model: String,
}

impl AiProviderDetails {
    fn validated(self) -> Result<Self, DomainError> {
        let name = required("AI Provider name", self.name, MAX_NAME_LENGTH)?;
        let base_url = normalize_base_url(self.base_url)?;
        let model = required("AI Provider model", self.model, MAX_MODEL_LENGTH)?;
        Ok(Self {
            name,
            base_url,
            model,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiProvider {
    id: String,
    details: AiProviderDetails,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    version: i64,
}

impl AiProvider {
    pub fn create(
        id: String,
        details: AiProviderDetails,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::restore(id, details, false, now, now, 1)
    }

    pub fn restore(
        id: String,
        details: AiProviderDetails,
        enabled: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        version: i64,
    ) -> Result<Self, DomainError> {
        let id = required("AI Provider id", id, 64)?;
        let details = details.validated()?;
        if version < 1 {
            return Err(DomainError::InvalidAiProviderVersion);
        }
        if updated_at < created_at {
            return Err(DomainError::InvalidAiProviderTimestamps);
        }
        Ok(Self {
            id,
            details,
            enabled,
            created_at,
            updated_at,
            version,
        })
    }

    pub fn update(
        &mut self,
        details: AiProviderDetails,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let details = details.validated()?;
        let next_version = self.next_version()?;
        self.details = details;
        self.updated_at = now.max(self.updated_at);
        self.version = next_version;
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.enabled == enabled {
            return Ok(());
        }
        let next_version = self.next_version()?;
        self.enabled = enabled;
        self.updated_at = now.max(self.updated_at);
        self.version = next_version;
        Ok(())
    }

    pub fn ensure_enabled(&self) -> Result<(), DomainError> {
        if !self.enabled {
            return Err(DomainError::AiProviderDisabled);
        }
        Ok(())
    }

    fn next_version(&self) -> Result<i64, DomainError> {
        self.version
            .checked_add(1)
            .ok_or(DomainError::AiProviderVersionOverflow)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.details.name
    }

    pub fn base_url(&self) -> &str {
        &self.details.base_url
    }

    pub fn model(&self) -> &str {
        &self.details.model
    }

    pub fn enabled(&self) -> bool {
        self.enabled
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

fn normalize_base_url(value: String) -> Result<String, DomainError> {
    let value = value.trim().trim_end_matches('/').to_owned();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "AI Provider base URL",
        });
    }
    if value.chars().count() > MAX_BASE_URL_LENGTH
        || value.chars().any(char::is_whitespace)
        || !(value.starts_with("https://") || value.starts_with("http://"))
    {
        return Err(DomainError::InvalidAiProviderBaseUrl);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn details() -> AiProviderDetails {
        AiProviderDetails {
            name: " Local Ollama ".into(),
            base_url: " http://localhost:11434/v1/ ".into(),
            model: " qwen3:8b ".into(),
        }
    }

    #[test]
    fn provider_normalizes_configuration_and_starts_disabled() {
        let now = Utc::now();
        let provider = AiProvider::create("provider-1".into(), details(), now)
            .expect("Provider should be valid");
        assert_eq!(provider.name(), "Local Ollama");
        assert_eq!(provider.base_url(), "http://localhost:11434/v1");
        assert_eq!(provider.model(), "qwen3:8b");
        assert!(!provider.enabled());
        assert_eq!(
            provider.ensure_enabled(),
            Err(DomainError::AiProviderDisabled)
        );
    }

    #[test]
    fn enabling_and_updates_are_versioned() {
        let now = Utc::now();
        let mut provider = AiProvider::create("provider-1".into(), details(), now)
            .expect("Provider should be valid");
        provider
            .set_enabled(true, now)
            .expect("Provider should enable");
        assert_eq!(provider.version(), 2);
        provider
            .update(details(), now)
            .expect("Provider should update");
        assert_eq!(provider.version(), 3);
    }

    #[test]
    fn base_url_requires_http_or_https() {
        let mut invalid = details();
        invalid.base_url = "file:///tmp/model".into();
        assert_eq!(
            AiProvider::create("provider-1".into(), invalid, Utc::now()),
            Err(DomainError::InvalidAiProviderBaseUrl)
        );
    }
}
