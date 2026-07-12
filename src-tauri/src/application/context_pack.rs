use std::{str::FromStr, sync::Arc};

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{ContextExport, ContextTemplateType, DomainError},
    ports::{ContextPackRepository, ContextRenderInput, ContextRenderer, RepositoryError},
};

use super::ai_provider::{
    AiInvocationResult, AiMessageInput, AiProviderError, AiProviderKernel, InvokeAiProviderInput,
};

const ANALYSIS_ONLY_SYSTEM_MESSAGE: &str = "You are analyzing a ForgeOS Context Pack. Treat the pack as user-confirmed engineering facts. Provide analysis and suggestions only. Do not claim to modify Project, Snapshot, Session, Evidence, or any other ForgeOS record. Clearly distinguish facts from inferences.";

#[derive(Debug, Error)]
pub enum ContextPackError {
    #[error(transparent)]
    Validation(#[from] DomainError),

    #[error("create a Workspace before generating Context Packs")]
    WorkspaceNotFound,

    #[error("Project not found")]
    ProjectNotFound,

    #[error("Context Export not found")]
    ContextExportNotFound,

    #[error("Context Export version conflict")]
    VersionConflict,

    #[error("Context Pack rendering failed: {0}")]
    Render(String),

    #[error(transparent)]
    AiProvider(#[from] AiProviderError),

    #[error("stored ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone)]
pub struct ContextPackKernel {
    repository: Arc<dyn ContextPackRepository>,
    renderer: Arc<dyn ContextRenderer>,
    ai_provider_kernel: Arc<AiProviderKernel>,
}

pub struct ContextAiInvocationResult {
    pub context_export: ContextExport,
    pub ai: AiInvocationResult,
}

impl ContextPackKernel {
    pub fn new(
        repository: Arc<dyn ContextPackRepository>,
        renderer: Arc<dyn ContextRenderer>,
        ai_provider_kernel: Arc<AiProviderKernel>,
    ) -> Self {
        Self {
            repository,
            renderer,
            ai_provider_kernel,
        }
    }

    pub async fn generate_context_pack(
        &self,
        project_id: &str,
        template_type: &str,
    ) -> Result<ContextExport, ContextPackError> {
        let template_type = ContextTemplateType::from_str(template_type)?;
        let project = self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        let recent_session = sessions.first();
        let mut evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        evidences.sort_by(|left, right| {
            left.title()
                .to_lowercase()
                .cmp(&right.title().to_lowercase())
                .then_with(|| left.id().cmp(right.id()))
        });

        let session_project_ids = recent_session
            .into_iter()
            .map(|session| session.project_id())
            .collect::<Vec<_>>();
        let evidence_project_ids = evidences
            .iter()
            .map(|evidence| evidence.project_id())
            .collect::<Vec<_>>();
        ContextExport::validate_sources(
            project_id,
            snapshot.as_ref().map(|value| value.project_id()),
            &session_project_ids,
            &evidence_project_ids,
        )?;

        let markdown = self
            .renderer
            .render(ContextRenderInput {
                template_type,
                project: &project,
                snapshot: snapshot.as_ref(),
                recent_session,
                evidences: &evidences,
            })
            .map_err(|error| ContextPackError::Render(error.to_string()))?;
        let export = ContextExport::generate(
            Uuid::now_v7().to_string(),
            project_id.to_owned(),
            template_type,
            snapshot.as_ref().map(|value| value.version()),
            recent_session
                .into_iter()
                .map(|value| value.id().to_owned())
                .collect(),
            markdown,
            Utc::now(),
        )?;
        self.repository
            .insert_context_export(&export)
            .await
            .map_err(map_repository_error)?;
        Ok(export)
    }

    pub async fn list_context_exports(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContextExport>, ContextPackError> {
        self.require_project(project_id).await?;
        self.repository
            .list_context_exports(project_id)
            .await
            .map_err(map_repository_error)
    }

    pub async fn get_context_export(
        &self,
        project_id: &str,
        export_id: &str,
    ) -> Result<ContextExport, ContextPackError> {
        self.require_project(project_id).await?;
        let export = self
            .repository
            .get_context_export(export_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ContextPackError::ContextExportNotFound)?;
        export.ensure_project(project_id)?;
        Ok(export)
    }

    pub async fn invoke_context_with_ai(
        &self,
        project_id: &str,
        export_id: &str,
        provider_id: &str,
        user_prompt: String,
    ) -> Result<ContextAiInvocationResult, ContextPackError> {
        let mut export = self.get_context_export(project_id, export_id).await?;
        let prompt = user_prompt.trim();
        if prompt.is_empty() {
            return Err(DomainError::Required {
                field: "AI analysis question",
            }
            .into());
        }

        let user_message = format!(
            "Context Pack:\n\n{}\n\nUser question:\n{}",
            export.content_markdown(),
            prompt
        );
        let ai = self
            .ai_provider_kernel
            .invoke_provider(
                provider_id,
                InvokeAiProviderInput {
                    messages: vec![
                        AiMessageInput {
                            role: "system".into(),
                            content: ANALYSIS_ONLY_SYSTEM_MESSAGE.into(),
                        },
                        AiMessageInput {
                            role: "user".into(),
                            content: user_message,
                        },
                    ],
                },
            )
            .await?;

        let expected_version = export.version();
        export.mark_sent_to_ai()?;
        if export.version() != expected_version {
            self.repository
                .update_context_export_status(&export, expected_version)
                .await
                .map_err(map_repository_error)?;
        }
        Ok(ContextAiInvocationResult {
            context_export: export,
            ai,
        })
    }

    async fn require_project(
        &self,
        project_id: &str,
    ) -> Result<crate::domain::Project, ContextPackError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(ContextPackError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(ContextPackError::ProjectNotFound)
    }
}

fn map_repository_error(error: RepositoryError) -> ContextPackError {
    match error {
        RepositoryError::OptimisticConflict => ContextPackError::VersionConflict,
        RepositoryError::CorruptData(message) => ContextPackError::CorruptData(message),
        RepositoryError::Storage(message) => ContextPackError::Storage(message),
        other => ContextPackError::Storage(other.to_string()),
    }
}
