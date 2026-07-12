use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{AiProvider, AiProviderDetails, DomainError},
    ports::{
        AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiProviderRepository,
        ApiKeyStore, RepositoryError,
    },
};

const MAX_API_KEY_LENGTH: usize = 16_384;
const MAX_MESSAGES: usize = 50;
const MAX_MESSAGE_LENGTH: usize = 50_000;

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

    #[error("AI Provider invocation failed: {0}")]
    Invocation(String),

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
}

#[derive(Clone, Debug)]
pub struct UpdateAiProviderInput {
    pub name: String,
    pub base_url: String,
    pub model: String,
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
}

#[derive(Clone, Debug)]
pub struct AiInvocationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub completion: AiCompletion,
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
            details(input.name, input.base_url, input.model),
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
        provider.update(details(input.name, input.base_url, input.model), Utc::now())?;
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
        let api_key = self
            .api_key_store
            .get(provider.id())
            .await
            .map_err(|error| AiProviderError::SecretStore(error.to_string()))?;
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
        Ok(AiProviderView {
            provider,
            api_key_configured,
        })
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

fn details(name: String, base_url: String, model: String) -> AiProviderDetails {
    AiProviderDetails {
        name,
        base_url,
        model,
    }
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
