use tauri::State;

use crate::application::{
    evidence::{
        ChangeEvidenceStatusInput, CreateEvidenceInput, LinkEvidenceInput, UpdateEvidenceInput,
    },
    ProjectKernel,
};

use super::{
    dto::{
        ChangeEvidenceStatusRequest, CreateEvidenceRequest, EvidenceDto, EvidenceLinkDto,
        EvidenceRecordDto, LinkEvidenceRequest, UpdateEvidenceRequest,
    },
    IpcError,
};

#[tauri::command]
pub async fn create_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    input: CreateEvidenceRequest,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .create_evidence(
            &project_id,
            CreateEvidenceInput {
                title: input.title,
                description: input.description,
                evidence_type: input.evidence_type,
                source_type: input.source_type,
                source_uri: input.source_uri,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_evidences(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
) -> Result<Vec<EvidenceDto>, IpcError> {
    kernel
        .list_evidences(&project_id)
        .await
        .map(|items| items.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    evidence_id: String,
) -> Result<EvidenceRecordDto, IpcError> {
    kernel
        .get_evidence(&project_id, &evidence_id)
        .await
        .map(|record| EvidenceRecordDto {
            evidence: record.evidence.into(),
            links: record.links.into_iter().map(Into::into).collect(),
        })
        .map_err(Into::into)
}

#[tauri::command]
pub async fn update_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    evidence_id: String,
    input: UpdateEvidenceRequest,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .update_evidence(
            &project_id,
            &evidence_id,
            UpdateEvidenceInput {
                title: input.title,
                description: input.description,
                evidence_type: input.evidence_type,
                source_type: input.source_type,
                source_uri: input.source_uri,
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn verify_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    evidence_id: String,
    input: ChangeEvidenceStatusRequest,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .verify_evidence(
            &project_id,
            &evidence_id,
            ChangeEvidenceStatusInput {
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn invalidate_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    evidence_id: String,
    input: ChangeEvidenceStatusRequest,
) -> Result<EvidenceDto, IpcError> {
    kernel
        .invalidate_evidence(
            &project_id,
            &evidence_id,
            ChangeEvidenceStatusInput {
                expected_version: input.expected_version,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn link_evidence(
    kernel: State<'_, ProjectKernel>,
    project_id: String,
    evidence_id: String,
    input: LinkEvidenceRequest,
) -> Result<EvidenceLinkDto, IpcError> {
    kernel
        .link_evidence(
            &project_id,
            &evidence_id,
            LinkEvidenceInput {
                target_type: input.target_type,
                target_id: input.target_id,
                relation_type: input.relation_type,
            },
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}
