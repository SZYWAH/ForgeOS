use std::str::FromStr;

use chrono::{DateTime, Utc};

use super::DomainError;

const MAX_NAME_LENGTH: usize = 100;
const MAX_BASE_URL_LENGTH: usize = 2_048;
const MAX_MODEL_LENGTH: usize = 200;
const MAX_ENVIRONMENT_VARIABLE_LENGTH: usize = 128;
const MAX_HEADER_LENGTH: usize = 128;
const MAX_API_VERSION_LENGTH: usize = 100;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }
        }

        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value { $($value => Ok(Self::$variant),)+ _ => Err(DomainError::InvalidAiProviderProtocol) }
            }
        }
    };
}

string_enum!(AiProviderProtocol {
    OpenAiChatCompletions => "openai_chat_completions",
    OpenAiResponses => "openai_responses",
    AnthropicMessages => "anthropic_messages",
    AzureOpenAi => "azure_openai",
    OpenAiCompatible => "openai_compatible",
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiAuthScheme {
    Bearer,
    ApiKey,
    XApiKey,
    CustomHeader,
    None,
}

impl AiAuthScheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bearer => "bearer",
            Self::ApiKey => "api_key",
            Self::XApiKey => "x_api_key",
            Self::CustomHeader => "custom_header",
            Self::None => "none",
        }
    }
}

impl FromStr for AiAuthScheme {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bearer" => Ok(Self::Bearer),
            "api_key" => Ok(Self::ApiKey),
            "x_api_key" => Ok(Self::XApiKey),
            "custom_header" => Ok(Self::CustomHeader),
            "none" => Ok(Self::None),
            _ => Err(DomainError::InvalidAiProviderAuthentication),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiSecretSource {
    CredentialManager,
    Environment,
    None,
}

impl AiSecretSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CredentialManager => "credential_manager",
            Self::Environment => "environment",
            Self::None => "none",
        }
    }
}

impl FromStr for AiSecretSource {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "credential_manager" => Ok(Self::CredentialManager),
            "environment" => Ok(Self::Environment),
            "none" => Ok(Self::None),
            _ => Err(DomainError::InvalidAiProviderAuthentication),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiProviderDetails {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub protocol: AiProviderProtocol,
    pub auth_scheme: AiAuthScheme,
    pub secret_source: AiSecretSource,
    pub secret_required: bool,
    pub environment_variable: Option<String>,
    pub custom_auth_header: Option<String>,
    pub api_version: Option<String>,
}

impl AiProviderDetails {
    fn validated(self) -> Result<Self, DomainError> {
        let name = required("AI Provider name", self.name, MAX_NAME_LENGTH)?;
        let base_url = normalize_base_url(self.base_url)?;
        let model = required("AI Provider model", self.model, MAX_MODEL_LENGTH)?;
        let environment_variable = optional(
            "AI Provider environment variable",
            self.environment_variable,
            MAX_ENVIRONMENT_VARIABLE_LENGTH,
        )?;
        let custom_auth_header = optional(
            "AI Provider custom authentication header",
            self.custom_auth_header,
            MAX_HEADER_LENGTH,
        )?;
        let api_version = optional(
            "AI Provider API version",
            self.api_version,
            MAX_API_VERSION_LENGTH,
        )?;

        if self.secret_source == AiSecretSource::Environment {
            if !environment_variable
                .as_deref()
                .is_some_and(valid_environment_variable)
            {
                return Err(DomainError::InvalidAiProviderEnvironmentVariable);
            }
        } else if environment_variable.is_some() {
            return Err(DomainError::InvalidAiProviderAuthentication);
        }

        if self.auth_scheme == AiAuthScheme::CustomHeader {
            if !custom_auth_header.as_deref().is_some_and(valid_header_name) {
                return Err(DomainError::InvalidAiProviderAuthHeader);
            }
        } else if custom_auth_header.is_some() {
            return Err(DomainError::InvalidAiProviderAuthentication);
        }

        if (self.auth_scheme == AiAuthScheme::None) != (self.secret_source == AiSecretSource::None)
            || (self.secret_required && self.secret_source == AiSecretSource::None)
        {
            return Err(DomainError::InvalidAiProviderAuthentication);
        }

        Ok(Self {
            name,
            base_url,
            model,
            protocol: self.protocol,
            auth_scheme: self.auth_scheme,
            secret_source: self.secret_source,
            secret_required: self.secret_required,
            environment_variable,
            custom_auth_header,
            api_version,
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
        self.details = details.validated()?;
        self.version = self.next_version()?;
        self.updated_at = now.max(self.updated_at);
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.enabled == enabled {
            return Ok(());
        }
        self.version = self.next_version()?;
        self.enabled = enabled;
        self.updated_at = now.max(self.updated_at);
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
    pub const fn protocol(&self) -> AiProviderProtocol {
        self.details.protocol
    }
    pub const fn auth_scheme(&self) -> AiAuthScheme {
        self.details.auth_scheme
    }
    pub const fn secret_source(&self) -> AiSecretSource {
        self.details.secret_source
    }
    pub const fn secret_required(&self) -> bool {
        self.details.secret_required
    }
    pub fn environment_variable(&self) -> Option<&str> {
        self.details.environment_variable.as_deref()
    }
    pub fn custom_auth_header(&self) -> Option<&str> {
        self.details.custom_auth_header.as_deref()
    }
    pub fn api_version(&self) -> Option<&str> {
        self.details.api_version.as_deref()
    }
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    pub const fn version(&self) -> i64 {
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
    if value.starts_with("http://") && !is_loopback_http_url(&value) {
        return Err(DomainError::InvalidAiProviderBaseUrl);
    }
    if base_url_contains_credentials(&value) {
        return Err(DomainError::InvalidAiProviderBaseUrl);
    }
    Ok(value)
}

fn base_url_contains_credentials(value: &str) -> bool {
    let after_scheme = value
        .split_once("://")
        .map(|(_, value)| value)
        .unwrap_or_default();
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.contains('@') {
        return true;
    }
    let query = value
        .split_once('?')
        .map(|(_, query)| query.split('#').next().unwrap_or_default())
        .unwrap_or_default();
    query.split('&').any(|pair| {
        let key = pair
            .split_once('=')
            .map(|(key, _)| key)
            .unwrap_or(pair)
            .to_ascii_lowercase()
            .replace('-', "_");
        key == "key"
            || key.contains("api_key")
            || key.contains("token")
            || key.contains("auth")
            || key.contains("password")
            || key.contains("secret")
    })
}

fn is_loopback_http_url(value: &str) -> bool {
    let authority = value
        .strip_prefix("http://")
        .and_then(|value| value.split('/').next())
        .unwrap_or_default()
        .to_ascii_lowercase();
    authority == "localhost"
        || authority.starts_with("localhost:")
        || authority == "127.0.0.1"
        || authority.starts_with("127.0.0.1:")
        || authority == "[::1]"
        || authority.starts_with("[::1]:")
}

fn optional(
    field: &'static str,
    value: Option<String>,
    max: usize,
) -> Result<Option<String>, DomainError> {
    let value = value.map(|value| value.trim().to_owned());
    if value
        .as_deref()
        .is_some_and(|value| value.chars().count() > max)
    {
        return Err(DomainError::TooLong { field, max });
    }
    Ok(value.filter(|value| !value.is_empty()))
}

fn valid_environment_variable(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn valid_header_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !matches!(
        lower.as_str(),
        "host"
            | "content-length"
            | "cookie"
            | "set-cookie"
            | "connection"
            | "transfer-encoding"
            | "user-agent"
    ) && value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
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
            protocol: AiProviderProtocol::OpenAiCompatible,
            auth_scheme: AiAuthScheme::None,
            secret_source: AiSecretSource::None,
            secret_required: false,
            environment_variable: None,
            custom_auth_header: None,
            api_version: None,
        }
    }

    #[test]
    fn provider_normalizes_configuration_and_starts_disabled() {
        let provider = AiProvider::create("provider-1".into(), details(), Utc::now()).unwrap();
        assert_eq!(provider.base_url(), "http://localhost:11434/v1");
        assert_eq!(provider.protocol(), AiProviderProtocol::OpenAiCompatible);
        assert!(!provider.enabled());
    }

    #[test]
    fn validates_environment_and_custom_header_configuration() {
        let mut env = details();
        env.auth_scheme = AiAuthScheme::Bearer;
        env.secret_source = AiSecretSource::Environment;
        env.secret_required = true;
        env.environment_variable = Some("DEEPSEEK_API_KEY".into());
        assert!(AiProvider::create("provider-1".into(), env, Utc::now()).is_ok());

        let mut invalid = details();
        invalid.auth_scheme = AiAuthScheme::CustomHeader;
        invalid.secret_source = AiSecretSource::CredentialManager;
        invalid.custom_auth_header = Some("Host".into());
        assert_eq!(
            AiProvider::create("provider-2".into(), invalid, Utc::now()),
            Err(DomainError::InvalidAiProviderAuthHeader)
        );
    }

    #[test]
    fn base_url_requires_http_or_https() {
        let mut invalid = details();
        invalid.base_url = "file:///tmp/model".into();
        assert_eq!(
            AiProvider::create("provider-1".into(), invalid, Utc::now()),
            Err(DomainError::InvalidAiProviderBaseUrl)
        );

        let mut remote_http = details();
        remote_http.base_url = "http://api.example.com/v1".into();
        assert_eq!(
            AiProvider::create("provider-2".into(), remote_http, Utc::now()),
            Err(DomainError::InvalidAiProviderBaseUrl)
        );

        for base_url in [
            "https://user:secret@api.example.com/v1",
            "https://api.example.com/v1?api_key=secret",
        ] {
            let mut embedded = details();
            embedded.base_url = base_url.into();
            assert_eq!(
                AiProvider::create("provider-3".into(), embedded, Utc::now()),
                Err(DomainError::InvalidAiProviderBaseUrl)
            );
        }
    }
}
