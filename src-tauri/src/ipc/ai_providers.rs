use tauri::State;

use crate::application::{
    ai_provider::{
        AiMessageInput, CreateAiProviderInput, InvokeAiProviderInput, SetAiProviderEnabledInput,
        UpdateAiProviderInput,
    },
    AiProviderKernel,
};

use super::{
    dto::{
        AiInvocationResultDto, AiProviderConnectionTestDto, AiProviderDto, CreateAiProviderRequest,
        InvokeAiProviderRequest, SetAiProviderApiKeyRequest, SetAiProviderEnabledRequest,
        UpdateAiProviderRequest,
    },
    IpcError,
};

#[tauri::command]
pub async fn create_ai_provider(
    kernel: State<'_, AiProviderKernel>,
    input: CreateAiProviderRequest,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .create_provider(CreateAiProviderInput {
            name: input.name,
            base_url: input.base_url,
            model: input.model,
            protocol: input.protocol,
            auth_scheme: input.auth_scheme,
            secret_source: input.secret_source,
            secret_required: input.secret_required,
            environment_variable: input.environment_variable,
            custom_auth_header: input.custom_auth_header,
            api_version: input.api_version,
        })
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_ai_providers(
    kernel: State<'_, AiProviderKernel>,
) -> Result<Vec<AiProviderDto>, IpcError> {
    kernel
        .list_providers()
        .await
        .map(|providers| providers.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_ai_provider(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .get_provider(&provider_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_ai_provider(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
    input: UpdateAiProviderRequest,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .update_provider(
            &provider_id,
            UpdateAiProviderInput {
                name: input.name,
                base_url: input.base_url,
                model: input.model,
                protocol: input.protocol,
                auth_scheme: input.auth_scheme,
                secret_source: input.secret_source,
                secret_required: input.secret_required,
                environment_variable: input.environment_variable,
                custom_auth_header: input.custom_auth_header,
                api_version: input.api_version,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn test_ai_provider_connection(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
) -> Result<AiProviderConnectionTestDto, IpcError> {
    kernel
        .test_connection(&provider_id)
        .await
        .map(|result| AiProviderConnectionTestDto {
            provider_id: result.provider_id,
            protocol: result.protocol,
            model: result.model,
            latency_ms: result.latency_ms,
            warning_codes: result.warning_codes,
        })
        .map_err(Into::into)
}

#[tauri::command]
pub async fn set_ai_provider_enabled(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
    input: SetAiProviderEnabledRequest,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .set_provider_enabled(
            &provider_id,
            SetAiProviderEnabledInput {
                enabled: input.enabled,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn set_ai_provider_api_key(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
    input: SetAiProviderApiKeyRequest,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .set_api_key(&provider_id, input.api_key)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn clear_ai_provider_api_key(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
) -> Result<AiProviderDto, IpcError> {
    kernel
        .clear_api_key(&provider_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn invoke_ai_provider(
    kernel: State<'_, AiProviderKernel>,
    provider_id: String,
    input: InvokeAiProviderRequest,
) -> Result<AiInvocationResultDto, IpcError> {
    kernel
        .invoke_provider(
            &provider_id,
            InvokeAiProviderInput {
                messages: input
                    .messages
                    .into_iter()
                    .map(|message| AiMessageInput {
                        role: message.role,
                        content: message.content,
                    })
                    .collect(),
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}
