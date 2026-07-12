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
        AiInvocationResultDto, AiProviderDto, CreateAiProviderRequest, InvokeAiProviderRequest,
        SetAiProviderApiKeyRequest, SetAiProviderEnabledRequest, UpdateAiProviderRequest,
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
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
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
