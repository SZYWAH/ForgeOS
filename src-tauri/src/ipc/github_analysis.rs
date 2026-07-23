use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::{
    application::{
        GitHubAnalysisKernel, GitHubAnalysisListView, GitHubAnalysisReportView,
        GitHubAnalysisStatusView, StartGitHubAnalysisInput,
    },
    domain::{GitHubAnalysisReport, GitHubAnalysisSource},
};

use super::IpcError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRequest {
    pub workspace_id: String,
    pub repository_url: String,
    pub focus_type: String,
    pub custom_prompt: Option<String>,
    pub analysis_mode: String,
    pub ai_provider_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartResultDto {
    pub analysis_run_id: String,
    pub reused_existing_run: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocatorRequest {
    pub workspace_id: String,
    pub analysis_run_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRequest {
    pub workspace_id: String,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveRequest {
    pub workspace_id: String,
    pub analysis_run_id: String,
    pub expected_version: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub analysis_run_id: String,
    pub owner: String,
    pub name: String,
    pub repository_url: String,
    pub focus_type: String,
    pub custom_prompt: Option<String>,
    pub analysis_mode: String,
    pub ai_provider_id: Option<String>,
    pub ai_model: Option<String>,
    pub analysis_method: String,
    pub ai_fallback_used: bool,
    pub status: String,
    pub readiness: String,
    pub stage: Option<String>,
    pub default_branch: String,
    pub source_commit_sha: String,
    pub analyzer_revision: String,
    pub stale: bool,
    pub tree_entries_seen: i64,
    pub files_read: i64,
    pub files_skipped: i64,
    pub source_bytes: i64,
    pub api_requests_used: i64,
    pub api_requests_remaining: Option<i64>,
    pub api_rate_limit_reset_at: Option<String>,
    pub ai_invoked: bool,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub version: i64,
}

impl From<GitHubAnalysisStatusView> for StatusDto {
    fn from(value: GitHubAnalysisStatusView) -> Self {
        let run = value.run;
        let ai_fallback_used = run
            .report_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<GitHubAnalysisReport>(value).ok())
            .is_some_and(|report| {
                report
                    .quality_warnings
                    .contains(&crate::domain::GitHubAnalysisWarning::AiFallback)
            });
        let analysis_method = if run.analysis_mode.as_str() == "rules" {
            "local_rules"
        } else if ai_fallback_used {
            "rules_fallback"
        } else {
            "ai_enhanced"
        };
        Self {
            analysis_run_id: run.id,
            owner: run.owner,
            name: run.name,
            repository_url: run.repository_url,
            focus_type: run.focus_type.as_str().into(),
            custom_prompt: run.custom_prompt,
            analysis_mode: run.analysis_mode.as_str().into(),
            ai_provider_id: run.ai_provider_id,
            ai_model: run.ai_model,
            analysis_method: analysis_method.into(),
            ai_fallback_used,
            status: run.status.as_str().into(),
            readiness: run.readiness.as_str().into(),
            stage: run.stage.map(|value| value.as_str().into()),
            default_branch: run.default_branch,
            source_commit_sha: run.source_commit_sha,
            analyzer_revision: run.analyzer_revision,
            stale: value.stale,
            tree_entries_seen: run.tree_entries_seen,
            files_read: run.files_read,
            files_skipped: run.files_skipped,
            source_bytes: run.source_bytes,
            api_requests_used: run.api_requests_used,
            api_requests_remaining: run.api_requests_remaining,
            api_rate_limit_reset_at: run.api_rate_limit_reset_at.map(|value| value.to_rfc3339()),
            ai_invoked: run.ai_invoked,
            started_at: run.started_at.map(|value| value.to_rfc3339()),
            completed_at: run.completed_at.map(|value| value.to_rfc3339()),
            created_at: run.created_at.to_rfc3339(),
            error_code: run.error_code,
            error_summary: run.error_summary,
            version: run.version,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDto {
    pub id: String,
    pub source_type: String,
    pub source_path: String,
    pub source_sha: Option<String>,
    pub excerpt: String,
    pub source_order: i64,
}
impl From<GitHubAnalysisSource> for SourceDto {
    fn from(value: GitHubAnalysisSource) -> Self {
        Self {
            id: value.id,
            source_type: value.source_type.as_str().into(),
            source_path: value.source_path,
            source_sha: value.source_sha,
            excerpt: value.excerpt,
            source_order: value.source_order,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDto {
    pub status: StatusDto,
    pub report: Option<GitHubAnalysisReport>,
    pub content_markdown: Option<String>,
    pub content_hash: Option<String>,
    pub sources: Vec<SourceDto>,
}
impl From<GitHubAnalysisReportView> for ReportDto {
    fn from(value: GitHubAnalysisReportView) -> Self {
        Self {
            status: value.status.into(),
            report: value.report,
            content_markdown: value.content_markdown,
            content_hash: value.content_hash,
            sources: value.sources.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDto {
    pub items: Vec<StatusDto>,
    pub next_cursor: Option<String>,
}
impl From<GitHubAnalysisListView> for ListDto {
    fn from(value: GitHubAnalysisListView) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            next_cursor: value.next_cursor,
        }
    }
}

#[tauri::command]
pub async fn start_github_analysis(
    app: AppHandle,
    kernel: State<'_, GitHubAnalysisKernel>,
    input: StartRequest,
) -> Result<StartResultDto, IpcError> {
    let workspace_id = input.workspace_id.clone();
    let result = kernel
        .start(StartGitHubAnalysisInput {
            workspace_id: input.workspace_id,
            repository_url: input.repository_url,
            focus_type: input.focus_type,
            custom_prompt: input.custom_prompt,
            analysis_mode: input.analysis_mode,
            ai_provider_id: input.ai_provider_id,
        })
        .await?;
    if !result.reused_existing_run {
        let run_id = result.analysis_run_id.clone();
        let background = kernel.inner().clone();
        tauri::async_runtime::spawn(async move {
            let progress_app = app.clone();
            let status = background
                .execute(&workspace_id, &run_id, move |event| {
                    let _ = progress_app.emit("github-analysis://progress", event);
                })
                .await;
            match status {
                Ok(value) => {
                    let _ = app.emit("github-analysis://completed", StatusDto::from(value));
                }
                Err(error) => {
                    let _ = app.emit("github-analysis://completed", serde_json::json!({"analysisRunId": run_id, "status": "failed", "error": error.to_string()}));
                }
            }
        });
    }
    Ok(StartResultDto {
        analysis_run_id: result.analysis_run_id,
        reused_existing_run: result.reused_existing_run,
        warnings: result.warnings,
    })
}

#[tauri::command]
pub async fn get_github_analysis_status(
    kernel: State<'_, GitHubAnalysisKernel>,
    input: LocatorRequest,
) -> Result<StatusDto, IpcError> {
    kernel
        .status(&input.workspace_id, &input.analysis_run_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn get_github_analysis_report(
    kernel: State<'_, GitHubAnalysisKernel>,
    input: LocatorRequest,
) -> Result<ReportDto, IpcError> {
    kernel
        .report(&input.workspace_id, &input.analysis_run_id)
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn list_github_analyses(
    kernel: State<'_, GitHubAnalysisKernel>,
    input: ListRequest,
) -> Result<ListDto, IpcError> {
    kernel
        .list(
            &input.workspace_id,
            input.cursor.as_deref(),
            input.limit.unwrap_or(20),
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[tauri::command]
pub async fn reanalyze_github_project(
    app: AppHandle,
    kernel: State<'_, GitHubAnalysisKernel>,
    input: LocatorRequest,
) -> Result<StartResultDto, IpcError> {
    let result = kernel
        .reanalyze(&input.workspace_id, &input.analysis_run_id)
        .await?;
    if !result.reused_existing_run {
        let run_id = result.analysis_run_id.clone();
        let workspace_id = input.workspace_id;
        let background = kernel.inner().clone();
        let progress_app = app.clone();
        tauri::async_runtime::spawn(async move {
            let status = background
                .execute(&workspace_id, &run_id, move |event| {
                    let _ = progress_app.emit("github-analysis://progress", event);
                })
                .await;
            if let Ok(value) = status {
                let _ = app.emit("github-analysis://completed", StatusDto::from(value));
            }
        });
    }
    Ok(StartResultDto {
        analysis_run_id: result.analysis_run_id,
        reused_existing_run: result.reused_existing_run,
        warnings: result.warnings,
    })
}

#[tauri::command]
pub async fn archive_github_analysis(
    kernel: State<'_, GitHubAnalysisKernel>,
    input: ArchiveRequest,
) -> Result<StatusDto, IpcError> {
    kernel
        .archive(
            &input.workspace_id,
            &input.analysis_run_id,
            input.expected_version,
        )
        .await
        .map(Into::into)
        .map_err(Into::into)
}
