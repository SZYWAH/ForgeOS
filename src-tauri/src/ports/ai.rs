use async_trait::async_trait;
use thiserror::Error;

use crate::domain::AiProvider;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AiMessageRole {
    System,
    User,
    Assistant,
}

impl AiMessageRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiMessage {
    pub role: AiMessageRole,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiTokenUsage {
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiCompletion {
    pub content: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub usage: Option<AiTokenUsage>,
}

#[derive(Debug, Error)]
pub enum SecretStoreError {
    #[error("credential store operation failed: {0}")]
    Unavailable(String),
}

#[derive(Debug, Error)]
pub enum AiClientError {
    #[error("invalid provider endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("provider request failed: {0}")]
    Transport(String),

    #[error("provider returned HTTP {status}: {message}")]
    Http { status: u16, message: String },

    #[error("provider returned an unsupported response: {0}")]
    InvalidResponse(String),
}

#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    async fn set(&self, provider_id: &str, api_key: &str) -> Result<(), SecretStoreError>;
    async fn get(&self, provider_id: &str) -> Result<Option<String>, SecretStoreError>;
    async fn clear(&self, provider_id: &str) -> Result<(), SecretStoreError>;

    async fn has(&self, provider_id: &str) -> Result<bool, SecretStoreError> {
        self.get(provider_id).await.map(|value| value.is_some())
    }
}

#[async_trait]
pub trait AiProviderClient: Send + Sync {
    async fn complete(
        &self,
        provider: &AiProvider,
        api_key: Option<String>,
        messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError>;
}
