use std::{str::FromStr, sync::Arc, time::Instant};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        AiAuthScheme, AiProvider, AiProviderDetails, AiProviderProtocol, AiSecretSource,
        DomainError,
    },
    ports::{
        AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiProviderRepository,
        ApiKeyStore, RepositoryError,
    },
};

const MAX_API_KEY_LENGTH: usize = 16_384;
const MAX_MESSAGES: usize = 50;
const MAX_MESSAGE_LENGTH: usize = 65_000;

#[derive(Debug, Error)]
pub enum AiProviderError {
    #[error(transparent)]
    Validation(#[from] DomainError),

    #[error("AI Provider not found")]
    NotFound,

    #[error("another AI Provider already uses this name")]
    NameConflict,

    #[error("AI Provider version conflict: expected {expected}, current {actual}")]
    VersionConflict { expected: i64, actual: i64 },

    #[error("API key must be at most {MAX_API_KEY_LENGTH} characters")]
    ApiKeyTooLong,

    #[error(
        "AI request must contain 1-{MAX_MESSAGES} non-empty messages including a user message"
    )]
    InvalidMessages,

    #[error("credential store is unavailable: {0}")]
    SecretStore(String),

    #[error("AI Provider credential is missing")]
    SecretMissing,

    #[error("AI Provider credential source does not accept a stored API key")]
    SecretSourceMismatch,

    #[error("AI Provider invocation failed: {0}")]
    Invocation(String),

    #[error("AI Provider connection test failed: {0}")]
    ConnectionTest(String),

    #[error("stored AI Provider data is invalid: {0}")]
    CorruptData(String),

    #[error("AI Provider storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct CreateAiProviderInput {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub protocol: String,
    pub auth_scheme: String,
    pub secret_source: String,
    pub secret_required: bool,
    pub environment_variable: Option<String>,
    pub custom_auth_header: Option<String>,
    pub api_version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpdateAiProviderInput {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub protocol: String,
    pub auth_scheme: String,
    pub secret_source: String,
    pub secret_required: bool,
    pub environment_variable: Option<String>,
    pub custom_auth_header: Option<String>,
    pub api_version: Option<String>,
    pub expected_version: i64,
}

#[derive(Clone, Debug)]
pub struct SetAiProviderEnabledInput {
    pub enabled: bool,
    pub expected_version: i64,
}

#[derive(Clone)]
pub struct InvokeAiProviderInput {
    pub messages: Vec<AiMessageInput>,
}

#[derive(Clone)]
pub struct AiMessageInput {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug)]
pub struct AiProviderView {
    pub provider: AiProvider,
    pub api_key_configured: bool,
    pub secret_available: bool,
}

#[derive(Clone, Debug)]
pub struct AiInvocationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub completion: AiCompletion,
}

#[derive(Clone, Debug)]
pub struct AiProviderConnectionTest {
    pub provider_id: String,
    pub protocol: String,
    pub model: String,
    pub latency_ms: u128,
    pub warning_codes: Vec<String>,
}

#[derive(Clone)]
pub struct AiProviderKernel {
    repository: Arc<dyn AiProviderRepository>,
    api_key_store: Arc<dyn ApiKeyStore>,
    client: Arc<dyn AiProviderClient>,
}

impl AiProviderKernel {
    pub fn new(
        repository: Arc<dyn AiProviderRepository>,
        api_key_store: Arc<dyn ApiKeyStore>,
        client: Arc<dyn AiProviderClient>,
    ) -> Self {
        Self {
            repository,
            api_key_store,
            client,
        }
    }

    pub async fn create_provider(
        &self,
        input: CreateAiProviderInput,
    ) -> Result<AiProviderView, AiProviderError> {
        let provider = AiProvider::create(
            Uuid::now_v7().to_string(),
            details_from_create(input)?,
            Utc::now(),
        )?;
        match self.repository.insert_ai_provider(&provider).await {
            Ok(()) => self.view(provider).await,
            Err(RepositoryError::Conflict) => Err(AiProviderError::NameConflict),
            Err(error) => Err(map_repository_error(error)),
        }
    }

    pub async fn list_providers(&self) -> Result<Vec<AiProviderView>, AiProviderError> {
        let providers = self
            .repository
            .list_ai_providers()
            .await
            .map_err(map_repository_error)?;
        let mut views = Vec::with_capacity(providers.len());
        for provider in providers {
            views.push(self.view(provider).await?);
        }
        Ok(views)
    }

    pub async fn get_provider(&self, provider_id: &str) -> Result<AiProviderView, AiProviderError> {
        self.view(self.require_provider(provider_id).await?).await
    }

    pub async fn update_provider(
        &self,
        provider_id: &str,
        input: UpdateAiProviderInput,
    ) -> Result<AiProviderView, AiProviderError> {
        let mut provider = self.require_provider(provider_id).await?;
        ensure_version(input.expected_version, provider.version())?;
        provider.update(details_from_update(&input)?, Utc::now())?;
        self.persist(&provider, input.expected_version).await?;
        self.view(provider).await
    }

    pub async fn set_provider_enabled(
        &self,
        provider_id: &str,
        input: SetAiProviderEnabledInput,
    ) -> Result<AiProviderView, AiProviderError> {
        let mut provider = self.require_provider(provider_id).await?;
        ensure_version(input.expected_version, provider.version())?;
        let previous_version = provider.version();
        provider.set_enabled(input.enabled, Utc::now())?;
        if provider.version() != previous_version {
            self.persist(&provider, input.expected_version).await?;
        }
        self.view(provider).await
    }

    pub async fn set_api_key(
        &self,
        provider_id: &str,
        api_key: String,
    ) -> Result<AiProviderView, AiProviderError> {
        let provider = self.require_provider(provider_id).await?;
        if provider.secret_source() != AiSecretSource::CredentialManager {
            return Err(AiProviderError::SecretSourceMismatch);
        }
        let api_key = api_key.trim();
        if api_key.chars().count() > MAX_API_KEY_LENGTH {
            return Err(AiProviderError::ApiKeyTooLong);
        }
        if api_key.is_empty() {
            self.api_key_store
                .clear(provider.id())
                .await
                .map_err(|error| AiProviderError::SecretStore(error.to_string()))?;
        } else {
            self.api_key_store
                .set(provider.id(), api_key)
                .await
                .map_err(|error| AiProviderError::SecretStore(error.to_string()))?;
        }
        self.view(provider).await
    }

    pub async fn clear_api_key(
        &self,
        provider_id: &str,
    ) -> Result<AiProviderView, AiProviderError> {
        let provider = self.require_provider(provider_id).await?;
        self.api_key_store
            .clear(provider.id())
            .await
            .map_err(|error| AiProviderError::SecretStore(error.to_string()))?;
        self.view(provider).await
    }

    pub async fn invoke_provider(
        &self,
        provider_id: &str,
        input: InvokeAiProviderInput,
    ) -> Result<AiInvocationResult, AiProviderError> {
        let provider = self.require_provider(provider_id).await?;
        provider.ensure_enabled()?;
        let messages = validate_messages(input.messages)?;
        let api_key = self.resolve_secret(&provider).await?;
        let completion = self
            .client
            .complete(&provider, api_key, &messages)
            .await
            .map_err(|error| AiProviderError::Invocation(error.to_string()))?;
        Ok(AiInvocationResult {
            provider_id: provider.id().to_owned(),
            provider_name: provider.name().to_owned(),
            completion,
        })
    }

    pub async fn test_connection(
        &self,
        provider_id: &str,
    ) -> Result<AiProviderConnectionTest, AiProviderError> {
        let provider = self.require_provider(provider_id).await?;
        let secret = self.resolve_secret(&provider).await?;
        let started = Instant::now();
        self.client
            .complete(
                &provider,
                secret,
                &[AiMessage {
                    role: AiMessageRole::User,
                    content: "Reply with OK only.".into(),
                }],
            )
            .await
            .map_err(|error| AiProviderError::ConnectionTest(error.to_string()))?;
        Ok(AiProviderConnectionTest {
            provider_id: provider.id().to_owned(),
            protocol: provider.protocol().as_str().into(),
            model: provider.model().into(),
            latency_ms: started.elapsed().as_millis(),
            warning_codes: Vec::new(),
        })
    }

    async fn require_provider(&self, provider_id: &str) -> Result<AiProvider, AiProviderError> {
        self.repository
            .get_ai_provider(provider_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(AiProviderError::NotFound)
    }

    async fn view(&self, provider: AiProvider) -> Result<AiProviderView, AiProviderError> {
        let api_key_configured = self
            .api_key_store
            .has(provider.id())
            .await
            .map_err(|error| AiProviderError::SecretStore(error.to_string()))?;
        let secret_available = match provider.secret_source() {
            AiSecretSource::CredentialManager => api_key_configured,
            AiSecretSource::Environment => provider.environment_variable().is_some_and(|name| {
                std::env::var(name).is_ok_and(|value| !value.trim().is_empty())
            }),
            AiSecretSource::None => true,
        };
        Ok(AiProviderView {
            provider,
            api_key_configured,
            secret_available,
        })
    }

    async fn resolve_secret(
        &self,
        provider: &AiProvider,
    ) -> Result<Option<String>, AiProviderError> {
        let secret = match provider.secret_source() {
            AiSecretSource::CredentialManager => self
                .api_key_store
                .get(provider.id())
                .await
                .map_err(|error| AiProviderError::SecretStore(error.to_string()))?,
            AiSecretSource::Environment => provider
                .environment_variable()
                .and_then(|name| std::env::var(name).ok())
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty()),
            AiSecretSource::None => None,
        };
        if provider.secret_required() && secret.is_none() {
            return Err(AiProviderError::SecretMissing);
        }
        Ok(secret)
    }

    async fn persist(
        &self,
        provider: &AiProvider,
        expected_version: i64,
    ) -> Result<(), AiProviderError> {
        match self
            .repository
            .update_ai_provider(provider, expected_version)
            .await
        {
            Ok(()) => Ok(()),
            Err(RepositoryError::Conflict) => Err(AiProviderError::NameConflict),
            Err(RepositoryError::OptimisticConflict) => {
                let current = self.require_provider(provider.id()).await?;
                Err(AiProviderError::VersionConflict {
                    expected: expected_version,
                    actual: current.version(),
                })
            }
            Err(error) => Err(map_repository_error(error)),
        }
    }
}

fn details_from_create(input: CreateAiProviderInput) -> Result<AiProviderDetails, AiProviderError> {
    details(
        input.name,
        input.base_url,
        input.model,
        input.protocol,
        input.auth_scheme,
        input.secret_source,
        input.secret_required,
        input.environment_variable,
        input.custom_auth_header,
        input.api_version,
    )
}

fn details_from_update(
    input: &UpdateAiProviderInput,
) -> Result<AiProviderDetails, AiProviderError> {
    details(
        input.name.clone(),
        input.base_url.clone(),
        input.model.clone(),
        input.protocol.clone(),
        input.auth_scheme.clone(),
        input.secret_source.clone(),
        input.secret_required,
        input.environment_variable.clone(),
        input.custom_auth_header.clone(),
        input.api_version.clone(),
    )
}

#[allow(clippy::too_many_arguments)]
fn details(
    name: String,
    base_url: String,
    model: String,
    protocol: String,
    auth_scheme: String,
    secret_source: String,
    secret_required: bool,
    environment_variable: Option<String>,
    custom_auth_header: Option<String>,
    api_version: Option<String>,
) -> Result<AiProviderDetails, AiProviderError> {
    Ok(AiProviderDetails {
        name,
        base_url,
        model,
        protocol: AiProviderProtocol::from_str(&protocol)?,
        auth_scheme: AiAuthScheme::from_str(&auth_scheme)?,
        secret_source: AiSecretSource::from_str(&secret_source)?,
        secret_required,
        environment_variable,
        custom_auth_header,
        api_version,
    })
}

fn ensure_version(expected: i64, actual: i64) -> Result<(), AiProviderError> {
    if expected != actual {
        return Err(AiProviderError::VersionConflict { expected, actual });
    }
    Ok(())
}

fn validate_messages(inputs: Vec<AiMessageInput>) -> Result<Vec<AiMessage>, AiProviderError> {
    if inputs.is_empty() || inputs.len() > MAX_MESSAGES {
        return Err(AiProviderError::InvalidMessages);
    }
    let mut has_user = false;
    let mut messages = Vec::with_capacity(inputs.len());
    for input in inputs {
        let role = AiMessageRole::from_str(&input.role)?;
        let content = input.content.trim().to_owned();
        if content.is_empty() || content.chars().count() > MAX_MESSAGE_LENGTH {
            return Err(AiProviderError::InvalidMessages);
        }
        has_user |= role == AiMessageRole::User;
        messages.push(AiMessage { role, content });
    }
    if !has_user {
        return Err(AiProviderError::InvalidMessages);
    }
    Ok(messages)
}

impl FromStr for AiMessageRole {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            _ => Err(DomainError::InvalidAiMessageRole(value.to_owned())),
        }
    }
}

fn map_repository_error(error: RepositoryError) -> AiProviderError {
    match error {
        RepositoryError::CorruptData(message) => AiProviderError::CorruptData(message),
        RepositoryError::Storage(message) => AiProviderError::Storage(message),
        other => AiProviderError::Storage(other.to_string()),
    }
}
