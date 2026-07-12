use tauri::State;

use crate::application::ContextPackKernel;

use super::{
    dto::{
        ContextAiInvocationResultDto, ContextExportDto, GenerateContextPackRequest,
        InvokeContextWithAiRequest,
    },
    IpcError,
};

#[tauri::command]
pub async fn generate_context_pack(
    kernel: State<'_, ContextPackKernel>,
    project_id: String,
    input: GenerateContextPackRequest,
) -> Result<ContextExportDto, IpcError> {
    kernel
        .generate_context_pack(&project_id, &input.template_type)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_context_exports(
    kernel: State<'_, ContextPackKernel>,
    project_id: String,
) -> Result<Vec<ContextExportDto>, IpcError> {
    kernel
        .list_context_exports(&project_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_context_export(
    kernel: State<'_, ContextPackKernel>,
    project_id: String,
    export_id: String,
) -> Result<ContextExportDto, IpcError> {
    kernel
        .get_context_export(&project_id, &export_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn invoke_context_with_ai(
    kernel: State<'_, ContextPackKernel>,
    project_id: String,
    export_id: String,
    input: InvokeContextWithAiRequest,
) -> Result<ContextAiInvocationResultDto, IpcError> {
    kernel
        .invoke_context_with_ai(
            &project_id,
            &export_id,
            &input.provider_id,
            input.user_prompt,
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}
