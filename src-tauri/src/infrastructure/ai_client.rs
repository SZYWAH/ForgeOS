use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    header::{HeaderName, HeaderValue, AUTHORIZATION},
    redirect::Policy,
    Client, RequestBuilder, Url,
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{AiAuthScheme, AiProvider, AiProviderProtocol},
    ports::{
        AiClientError, AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiTokenUsage,
    },
};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone)]
pub struct OpenAiCompatibleClient {
    client: Client,
}

impl OpenAiCompatibleClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(15))
                .timeout(Duration::from_secs(90))
                .redirect(Policy::none())
                .user_agent("ForgeOS/0.2 AI-Provider-Compatibility")
                .build()?,
        })
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

#[derive(Serialize)]
struct ResponsesRequest<'a> {
    model: &'a str,
    input: Vec<ChatMessage<'a>>,
}

#[derive(Deserialize)]
struct ResponsesResponse {
    model: Option<String>,
    output: Vec<ResponseOutput>,
    usage: Option<ResponsesUsage>,
    status: Option<String>,
}

#[derive(Deserialize)]
struct ResponseOutput {
    content: Vec<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct ResponsesUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    model: Option<String>,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

#[async_trait]
impl AiProviderClient for OpenAiCompatibleClient {
    async fn complete(
        &self,
        provider: &AiProvider,
        secret: Option<String>,
        messages: &[AiMessage],
    ) -> Result<AiCompletion, AiClientError> {
        let endpoint = endpoint_for(provider)?;
        let builder = match provider.protocol() {
            AiProviderProtocol::OpenAiResponses => {
                self.client.post(endpoint).json(&ResponsesRequest {
                    model: provider.model(),
                    input: chat_messages(messages),
                })
            }
            AiProviderProtocol::AnthropicMessages => {
                let system = messages
                    .iter()
                    .filter(|message| message.role == AiMessageRole::System)
                    .map(|message| message.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n");
                let messages = messages
                    .iter()
                    .filter(|message| message.role != AiMessageRole::System)
                    .map(|message| ChatMessage {
                        role: message.role.as_str(),
                        content: &message.content,
                    })
                    .collect();
                self.client.post(endpoint).json(&AnthropicRequest {
                    model: provider.model(),
                    max_tokens: 8_192,
                    system: (!system.is_empty()).then_some(system),
                    messages,
                })
            }
            _ => self.client.post(endpoint).json(&ChatCompletionRequest {
                model: provider.model(),
                messages: chat_messages(messages),
                stream: false,
            }),
        };
        let builder = apply_auth(builder, provider, secret.as_deref())?;
        let response = builder
            .send()
            .await
            .map_err(|error| AiClientError::Transport(error.to_string()))?;
        let status = response.status();
        if status.is_redirection() {
            return Err(AiClientError::Http {
                status: status.as_u16(),
                message: "redirects are disabled for credential safety".into(),
            });
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
        {
            return Err(AiClientError::InvalidResponse(
                "response exceeded 2 MiB".into(),
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| AiClientError::Transport(error.to_string()))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(AiClientError::InvalidResponse(
                "response exceeded 2 MiB".into(),
            ));
        }
        if !status.is_success() {
            let mut message = truncate(&String::from_utf8_lossy(&bytes), 1_000);
            if let Some(secret) = secret.as_deref().filter(|value| !value.is_empty()) {
                message = message.replace(secret, "[REDACTED]");
            }
            return Err(AiClientError::Http {
                status: status.as_u16(),
                message,
            });
        }
        match provider.protocol() {
            AiProviderProtocol::OpenAiResponses => parse_responses(provider, &bytes),
            AiProviderProtocol::AnthropicMessages => parse_anthropic(provider, &bytes),
            _ => parse_chat(provider, &bytes),
        }
    }
}

fn chat_messages(messages: &[AiMessage]) -> Vec<ChatMessage<'_>> {
    messages
        .iter()
        .map(|message| ChatMessage {
            role: message.role.as_str(),
            content: &message.content,
        })
        .collect()
}

fn apply_auth(
    mut builder: RequestBuilder,
    provider: &AiProvider,
    secret: Option<&str>,
) -> Result<RequestBuilder, AiClientError> {
    let Some(secret) = secret.filter(|value| !value.is_empty()) else {
        return Ok(builder);
    };
    builder = match provider.auth_scheme() {
        AiAuthScheme::Bearer => builder.header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {secret}"))
                .map_err(|error| AiClientError::InvalidEndpoint(error.to_string()))?,
        ),
        AiAuthScheme::ApiKey => builder.header("api-key", secret),
        AiAuthScheme::XApiKey => builder.header("x-api-key", secret),
        AiAuthScheme::CustomHeader => {
            let name = HeaderName::from_bytes(
                provider.custom_auth_header().unwrap_or_default().as_bytes(),
            )
            .map_err(|error| AiClientError::InvalidEndpoint(error.to_string()))?;
            builder.header(name, secret)
        }
        AiAuthScheme::None => builder,
    };
    if provider.protocol() == AiProviderProtocol::AnthropicMessages {
        builder = builder.header(
            "anthropic-version",
            provider.api_version().unwrap_or("2023-06-01"),
        );
    }
    Ok(builder)
}

fn endpoint_for(provider: &AiProvider) -> Result<Url, AiClientError> {
    let mut url = Url::parse(provider.base_url())
        .map_err(|error| AiClientError::InvalidEndpoint(error.to_string()))?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(AiClientError::InvalidEndpoint(
            "embedded credentials are forbidden".into(),
        ));
    }
    if url.query_pairs().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        key.contains("key")
            || key.contains("token")
            || key.contains("auth")
            || key.contains("password")
    }) {
        return Err(AiClientError::InvalidEndpoint(
            "credentials are forbidden in query parameters".into(),
        ));
    }
    if url.scheme() == "http" && !is_loopback(&url) {
        return Err(AiClientError::InvalidEndpoint(
            "remote AI endpoints must use HTTPS".into(),
        ));
    }
    let suffix = match provider.protocol() {
        AiProviderProtocol::OpenAiResponses => "/responses",
        AiProviderProtocol::AnthropicMessages => "/messages",
        _ => "/chat/completions",
    };
    if !url.path().trim_end_matches('/').ends_with(suffix) {
        url.set_path(&format!("{}{}", url.path().trim_end_matches('/'), suffix));
    }
    Ok(url)
}

fn is_loopback(url: &Url) -> bool {
    matches!(
        url.host_str()
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("localhost" | "127.0.0.1" | "::1")
    )
}

fn parse_chat(provider: &AiProvider, bytes: &[u8]) -> Result<AiCompletion, AiClientError> {
    let parsed: ChatCompletionResponse = serde_json::from_slice(bytes)
        .map_err(|error| AiClientError::InvalidResponse(error.to_string()))?;
    let choice =
        parsed.choices.into_iter().next().ok_or_else(|| {
            AiClientError::InvalidResponse("response contained no choices".into())
        })?;
    let content = choice
        .message
        .content
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AiClientError::InvalidResponse("first choice contained no text content".into())
        })?;
    Ok(AiCompletion {
        content,
        model: parsed.model.unwrap_or_else(|| provider.model().into()),
        finish_reason: choice.finish_reason,
        usage: parsed.usage.map(|usage| AiTokenUsage {
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        }),
    })
}

fn parse_responses(provider: &AiProvider, bytes: &[u8]) -> Result<AiCompletion, AiClientError> {
    let parsed: ResponsesResponse = serde_json::from_slice(bytes)
        .map_err(|error| AiClientError::InvalidResponse(error.to_string()))?;
    let content = parsed
        .output
        .into_iter()
        .flat_map(|output| output.content)
        .filter_map(|content| (!content.text.is_empty()).then_some(content.text))
        .collect::<Vec<_>>()
        .join("\n");
    if content.is_empty() {
        return Err(AiClientError::InvalidResponse(
            "response contained no output text".into(),
        ));
    }
    Ok(AiCompletion {
        content,
        model: parsed.model.unwrap_or_else(|| provider.model().into()),
        finish_reason: parsed.status,
        usage: parsed.usage.map(|usage| AiTokenUsage {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
        }),
    })
}

fn parse_anthropic(provider: &AiProvider, bytes: &[u8]) -> Result<AiCompletion, AiClientError> {
    let parsed: AnthropicResponse = serde_json::from_slice(bytes)
        .map_err(|error| AiClientError::InvalidResponse(error.to_string()))?;
    let content = parsed
        .content
        .into_iter()
        .filter(|value| value.content_type == "text")
        .filter_map(|value| value.text)
        .collect::<Vec<_>>()
        .join("\n");
    if content.is_empty() {
        return Err(AiClientError::InvalidResponse(
            "response contained no text block".into(),
        ));
    }
    let usage = parsed.usage.map(|usage| {
        let total = match (usage.input_tokens, usage.output_tokens) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        };
        AiTokenUsage {
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: total,
        }
    });
    Ok(AiCompletion {
        content,
        model: parsed.model.unwrap_or_else(|| provider.model().into()),
        finish_reason: parsed.stop_reason,
        usage,
    })
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AiProviderDetails, AiSecretSource, DomainError};
    use chrono::Utc;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn serve_once(
        status: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock AI server");
        let address = listener.local_addr().expect("mock address");
        let status = status.to_owned();
        let headers = headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect::<Vec<_>>();
        let body = body.to_owned();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept mock request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let read = stream.read(&mut buffer).expect("read mock request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n")
                else {
                    continue;
                };
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                    })
                    .unwrap_or_default();
                if request.len() >= header_end + 4 + content_length {
                    break;
                }
            }
            let mut response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
                body.len()
            );
            for (name, value) in headers {
                response.push_str(&format!("{name}: {value}\r\n"));
            }
            response.push_str("\r\n");
            response.push_str(&body);
            stream
                .write_all(response.as_bytes())
                .expect("write mock response");
            String::from_utf8_lossy(&request).into_owned()
        });
        (format!("http://{address}/v1"), handle)
    }

    fn request_header<'a>(request: &'a str, expected_name: &str) -> Option<&'a str> {
        request
            .split_once("\r\n\r\n")
            .map(|(headers, _)| headers)
            .unwrap_or(request)
            .lines()
            .skip(1)
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case(expected_name)
                        .then(|| value.trim())
                })
            })
    }

    fn protocol_provider(
        protocol: AiProviderProtocol,
        base_url: String,
        auth_scheme: AiAuthScheme,
        api_version: Option<&str>,
    ) -> AiProvider {
        let authenticated = auth_scheme != AiAuthScheme::None;
        AiProvider::create(
            "protocol-provider".into(),
            AiProviderDetails {
                name: "Protocol Provider".into(),
                base_url,
                model: "model".into(),
                protocol,
                auth_scheme,
                secret_source: if authenticated {
                    AiSecretSource::CredentialManager
                } else {
                    AiSecretSource::None
                },
                secret_required: authenticated,
                environment_variable: None,
                custom_auth_header: None,
                api_version: api_version.map(str::to_owned),
            },
            Utc::now(),
        )
        .expect("protocol provider")
    }

    fn provider(protocol: AiProviderProtocol, base_url: &str) -> AiProvider {
        AiProvider::create(
            "provider".into(),
            AiProviderDetails {
                name: "Test".into(),
                base_url: base_url.into(),
                model: "model".into(),
                protocol,
                auth_scheme: AiAuthScheme::None,
                secret_source: AiSecretSource::None,
                secret_required: false,
                environment_variable: None,
                custom_auth_header: None,
                api_version: None,
            },
            Utc::now(),
        )
        .unwrap()
    }

    fn authenticated_provider(
        protocol: AiProviderProtocol,
        auth_scheme: AiAuthScheme,
        custom_auth_header: Option<&str>,
        api_version: Option<&str>,
    ) -> AiProvider {
        AiProvider::create(
            "provider".into(),
            AiProviderDetails {
                name: "Test".into(),
                base_url: "https://api.example.test/v1".into(),
                model: "model".into(),
                protocol,
                auth_scheme,
                secret_source: AiSecretSource::CredentialManager,
                secret_required: true,
                environment_variable: None,
                custom_auth_header: custom_auth_header.map(str::to_owned),
                api_version: api_version.map(str::to_owned),
            },
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn builds_protocol_endpoints_and_rejects_remote_http() {
        assert_eq!(
            endpoint_for(&provider(
                AiProviderProtocol::OpenAiResponses,
                "https://api.openai.com/v1"
            ))
            .unwrap()
            .as_str(),
            "https://api.openai.com/v1/responses"
        );
        assert_eq!(
            endpoint_for(&provider(
                AiProviderProtocol::AnthropicMessages,
                "https://api.anthropic.com/v1"
            ))
            .unwrap()
            .as_str(),
            "https://api.anthropic.com/v1/messages"
        );
        assert!(endpoint_for(&provider(
            AiProviderProtocol::OpenAiCompatible,
            "http://localhost:11434/v1"
        ))
        .is_ok());
    }

    #[test]
    fn parses_openai_responses_and_anthropic_text_blocks() {
        let responses = br#"{"model":"gpt-test","status":"completed","output":[{"content":[{"text":"first"},{"text":"second"}]}],"usage":{"input_tokens":3,"output_tokens":2,"total_tokens":5}}"#;
        let completion = parse_responses(
            &provider(
                AiProviderProtocol::OpenAiResponses,
                "https://api.openai.com/v1",
            ),
            responses,
        )
        .unwrap();
        assert_eq!(completion.content, "first\nsecond");
        assert_eq!(
            completion.usage.and_then(|usage| usage.total_tokens),
            Some(5)
        );

        let anthropic = br#"{"model":"claude-test","content":[{"type":"text","text":"ok"}],"stop_reason":"end_turn","usage":{"input_tokens":4,"output_tokens":1}}"#;
        let completion = parse_anthropic(
            &provider(
                AiProviderProtocol::AnthropicMessages,
                "https://api.anthropic.com/v1",
            ),
            anthropic,
        )
        .unwrap();
        assert_eq!(completion.content, "ok");
        assert_eq!(
            completion.usage.and_then(|usage| usage.total_tokens),
            Some(5)
        );
    }

    #[test]
    fn applies_only_the_selected_authentication_header() {
        let client = Client::new();
        let cases = [
            (AiAuthScheme::Bearer, "authorization", "Bearer secret"),
            (AiAuthScheme::ApiKey, "api-key", "secret"),
            (AiAuthScheme::XApiKey, "x-api-key", "secret"),
        ];
        for (scheme, header, expected) in cases {
            let provider =
                authenticated_provider(AiProviderProtocol::OpenAiCompatible, scheme, None, None);
            let request = apply_auth(
                client.post("https://api.example.test/v1/chat/completions"),
                &provider,
                Some("secret"),
            )
            .unwrap()
            .build()
            .unwrap();
            assert_eq!(request.headers()[header], expected);
        }

        let custom = authenticated_provider(
            AiProviderProtocol::OpenAiCompatible,
            AiAuthScheme::CustomHeader,
            Some("X-Custom-Secret"),
            None,
        );
        let request = apply_auth(
            client.post("https://api.example.test/v1/chat/completions"),
            &custom,
            Some("secret"),
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(request.headers()["x-custom-secret"], "secret");
        assert!(request.headers().get(AUTHORIZATION).is_none());

        let anthropic = authenticated_provider(
            AiProviderProtocol::AnthropicMessages,
            AiAuthScheme::XApiKey,
            None,
            Some("2024-01-01"),
        );
        let request = apply_auth(
            client.post("https://api.example.test/v1/messages"),
            &anthropic,
            Some("secret"),
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(request.headers()["anthropic-version"], "2024-01-01");
    }

    #[test]
    fn endpoint_preserves_non_credential_query_parameters() {
        let azure = AiProvider::create(
            "azure-provider".into(),
            AiProviderDetails {
                name: "Azure".into(),
                base_url: "https://resource.openai.azure.com/openai/v1?api-version=preview".into(),
                model: "deployment".into(),
                protocol: AiProviderProtocol::AzureOpenAi,
                auth_scheme: AiAuthScheme::ApiKey,
                secret_source: AiSecretSource::CredentialManager,
                secret_required: true,
                environment_variable: None,
                custom_auth_header: None,
                api_version: None,
            },
            Utc::now(),
        )
        .unwrap();
        let endpoint = endpoint_for(&azure).unwrap();
        assert_eq!(endpoint.path(), "/openai/v1/chat/completions");
        assert_eq!(endpoint.query(), Some("api-version=preview"));
        let credential_query = AiProvider::create(
            "invalid-provider".into(),
            AiProviderDetails {
                name: "Invalid".into(),
                base_url: "https://api.example.test/v1?api_key=secret".into(),
                model: "model".into(),
                protocol: AiProviderProtocol::OpenAiCompatible,
                auth_scheme: AiAuthScheme::None,
                secret_source: AiSecretSource::None,
                secret_required: false,
                environment_variable: None,
                custom_auth_header: None,
                api_version: None,
            },
            Utc::now(),
        );
        assert_eq!(credential_query, Err(DomainError::InvalidAiProviderBaseUrl));
    }

    #[tokio::test]
    async fn protocol_gateway_round_trips_mainstream_request_shapes() {
        let client = OpenAiCompatibleClient::new().expect("client");
        let messages = vec![
            AiMessage {
                role: AiMessageRole::System,
                content: "system".into(),
            },
            AiMessage {
                role: AiMessageRole::User,
                content: "ping".into(),
            },
        ];
        let cases = [
            (
                AiProviderProtocol::OpenAiCompatible,
                AiAuthScheme::None,
                None,
                r#"{"model":"model","choices":[{"message":{"content":"chat-ok"},"finish_reason":"stop"}]}"#,
                "/v1/chat/completions",
                "chat-ok",
            ),
            (
                AiProviderProtocol::OpenAiResponses,
                AiAuthScheme::Bearer,
                None,
                r#"{"model":"model","status":"completed","output":[{"content":[{"text":"responses-ok"}]}]}"#,
                "/v1/responses",
                "responses-ok",
            ),
            (
                AiProviderProtocol::AnthropicMessages,
                AiAuthScheme::XApiKey,
                Some("2023-06-01"),
                r#"{"model":"model","content":[{"type":"text","text":"anthropic-ok"}],"stop_reason":"end_turn"}"#,
                "/v1/messages",
                "anthropic-ok",
            ),
            (
                AiProviderProtocol::AzureOpenAi,
                AiAuthScheme::ApiKey,
                None,
                r#"{"model":"model","choices":[{"message":{"content":"azure-ok"},"finish_reason":"stop"}]}"#,
                "/v1/chat/completions",
                "azure-ok",
            ),
        ];
        for (protocol, auth, api_version, body, expected_path, expected_text) in cases {
            let (base_url, request) =
                serve_once("200 OK", &[("Content-Type", "application/json")], body);
            let provider = protocol_provider(protocol, base_url, auth, api_version);
            let secret = (auth != AiAuthScheme::None).then(|| "test-secret".to_owned());
            let completion = client
                .complete(&provider, secret, &messages)
                .await
                .expect("protocol completion");
            assert_eq!(completion.content, expected_text);
            let request = request.join().expect("mock request");
            assert!(request.starts_with(&format!("POST {expected_path} HTTP/1.1")));
            assert!(request.contains("\"model\":\"model\""));
            match auth {
                AiAuthScheme::Bearer => assert_eq!(
                    request_header(&request, "authorization"),
                    Some("Bearer test-secret")
                ),
                AiAuthScheme::ApiKey => {
                    assert_eq!(request_header(&request, "api-key"), Some("test-secret"))
                }
                AiAuthScheme::XApiKey => {
                    assert_eq!(request_header(&request, "x-api-key"), Some("test-secret"));
                    assert_eq!(
                        request_header(&request, "anthropic-version"),
                        Some("2023-06-01")
                    );
                    assert!(request.contains("\"system\":\"system\""));
                }
                _ => assert!(!request.contains("test-secret")),
            }
        }
    }

    #[tokio::test]
    async fn protocol_gateway_rejects_redirects_and_redacts_error_bodies() {
        let client = OpenAiCompatibleClient::new().expect("client");
        let messages = vec![AiMessage {
            role: AiMessageRole::User,
            content: "ping".into(),
        }];

        let (base_url, request) = serve_once(
            "302 Found",
            &[("Location", "https://other.example.test/v1")],
            "",
        );
        let provider = protocol_provider(
            AiProviderProtocol::OpenAiCompatible,
            base_url,
            AiAuthScheme::Bearer,
            None,
        );
        let redirect = client
            .complete(&provider, Some("redirect-secret".into()), &messages)
            .await
            .expect_err("redirect must fail");
        assert!(matches!(redirect, AiClientError::Http { status: 302, .. }));
        request.join().expect("redirect request");

        let (base_url, request) = serve_once(
            "401 Unauthorized",
            &[("Content-Type", "text/plain")],
            "credential rejected: error-secret",
        );
        let provider = protocol_provider(
            AiProviderProtocol::OpenAiCompatible,
            base_url,
            AiAuthScheme::Bearer,
            None,
        );
        let unauthorized = client
            .complete(&provider, Some("error-secret".into()), &messages)
            .await
            .expect_err("unauthorized must fail");
        let rendered = unauthorized.to_string();
        assert!(!rendered.contains("error-secret"));
        assert!(rendered.contains("[REDACTED]"));
        request.join().expect("unauthorized request");
    }
}
