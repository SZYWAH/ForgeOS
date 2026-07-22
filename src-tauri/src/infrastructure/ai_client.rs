use std::time::Duration;

use async_trait::async_trait;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::{
    domain::AiProvider,
    ports::{AiClientError, AiCompletion, AiMessage, AiProviderClient, AiTokenUsage},
};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct OpenAiCompatibleClient {
    client: Client,
}

impl OpenAiCompatibleClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .user_agent("ForgeOS/0.2 AI-Provider-Foundation")
            .build()?;
        Ok(Self { client })
    }
}

#[derive(Serialize)]
struct ChatCompletionRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    model: Option<String>,
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatUsage {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[async_trait]
impl AiProviderClient for OpenAiCompatibleClient {
    async fn complete(
        &self,
        provider: &AiProvider,
        api_key: Option<String>,
        messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        let endpoint = completion_endpoint(provider.base_url())?;
        let request = ChatCompletionRequest {
            model: provider.model(),
            messages: messages
                .iter()
                .map(|message| ChatMessage {
                    role: message.role.as_str(),
                    content: &message.content,
                })
                .collect(),
            stream: false,
        };
        let mut builder = self.client.post(endpoint).json(&request);
        if let Some(api_key) = api_key.filter(|value| !value.is_empty()) {
            builder = builder.bearer_auth(api_key);
        }

        let response = builder
            .send()
            .await
            .map_err(|error| AiClientError::Transport(error.to_string()))?;
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(AiClientError::InvalidResponse(
                "response exceeded 2 MiB".to_owned(),
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| AiClientError::Transport(error.to_string()))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(AiClientError::InvalidResponse(
                "response exceeded 2 MiB".to_owned(),
            ));
        }
        if !status.is_success() {
            let message = String::from_utf8_lossy(&bytes);
            return Err(AiClientError::Http {
                status: status.as_u16(),
                message: truncate(&message, 1_000),
            });
        }

        let parsed: ChatCompletionResponse = serde_json::from_slice(&bytes)
            .map_err(|error| AiClientError::InvalidResponse(error.to_string()))?;
        let choice = parsed.choices.into_iter().next().ok_or_else(|| {
            AiClientError::InvalidResponse("response contained no choices".to_owned())
        })?;
        let content = choice.message.content.ok_or_else(|| {
            AiClientError::InvalidResponse("first choice contained no text content".to_owned())
        })?;
        Ok(AiCompletion {
            content,
            model: parsed.model.unwrap_or_else(|| provider.model().to_owned()),
            finish_reason: choice.finish_reason,
            usage: parsed.usage.map(|usage| AiTokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
        })
    }
}

fn completion_endpoint(base_url: &str) -> Result<Url, AiClientError> {
    let mut url =
        Url::parse(base_url).map_err(|error| AiClientError::InvalidEndpoint(error.to_string()))?;
    if !url
        .path()
        .trim_end_matches('/')
        .ends_with("/chat/completions")
    {
        let path = format!("{}/chat/completions", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    Ok(url)
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_chat_completions_to_api_roots() {
        assert_eq!(
            completion_endpoint("https://api.openai.com/v1")
                .expect("OpenAI endpoint should parse")
                .as_str(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            completion_endpoint("http://localhost:11434/v1/chat/completions")
                .expect("full endpoint should parse")
                .as_str(),
            "http://localhost:11434/v1/chat/completions"
        );
    }

    #[test]
    fn preserves_query_parameters_for_custom_endpoints() {
        assert_eq!(
            completion_endpoint("https://example.test/openai/v1?api-version=v1")
                .expect("custom endpoint should parse")
                .as_str(),
            "https://example.test/openai/v1/chat/completions?api-version=v1"
        );
    }
}
