use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        ai_candidate::CreateAiCandidateInput,
        ai_provider::{AiInvocationResult, AiProviderView},
        codex_task_proposal::CreateCodexTaskProposalInput,
        context_pack::ContextAiInvocationResult,
        repository::RepositoryRecord,
    },
    domain::{
        AiCandidate, CodexTaskProposal, ContextExport, DevelopmentSession, DiffSummary, Evidence,
        EvidenceLink, Project, ProjectSnapshot, Repository, WorkingCopy, Workspace,
    },
    ports::{GitCommit, GitInspection},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub timezone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Workspace> for WorkspaceDto {
    fn from(workspace: Workspace) -> Self {
        Self {
            id: workspace.id,
            name: workspace.name,
            timezone: workspace.timezone,
            created_at: timestamp(workspace.created_at),
            updated_at: timestamp(workspace.updated_at),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub target_user: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub non_goals: String,
    #[serde(default)]
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub target_user: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub non_goals: String,
    #[serde(default)]
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub workspace_id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            workspace_id: project.workspace_id,
            slug: project.slug,
            name: project.name,
            description: project.description,
            vision: project.vision,
            target_user: project.target_user,
            scope: project.scope,
            non_goals: project.non_goals,
            tech_stack: project.tech_stack,
            status: project.status.as_str().to_owned(),
            lifecycle_stage: project.lifecycle_stage.as_str().to_owned(),
            priority: project.priority,
            created_at: timestamp(project.created_at),
            updated_at: timestamp(project.updated_at),
            version: project.version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub updated_at: String,
    pub version: i64,
}

impl From<Project> for ProjectSummaryDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            slug: project.slug,
            name: project.name,
            description: project.description,
            status: project.status.as_str().to_owned(),
            lifecycle_stage: project.lifecycle_stage.as_str().to_owned(),
            priority: project.priority,
            updated_at: timestamp(project.updated_at),
            version: project.version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotRequest {
    pub current_goal: String,
    pub status_summary: String,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_steps: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshotRequest {
    pub current_goal: String,
    pub status_summary: String,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_steps: Vec<String>,
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshotDto {
    pub id: String,
    pub project_id: String,
    pub version: i64,
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub confirmed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProjectSnapshot> for ProjectSnapshotDto {
    fn from(snapshot: ProjectSnapshot) -> Self {
        Self {
            id: snapshot.id().to_owned(),
            project_id: snapshot.project_id().to_owned(),
            version: snapshot.version(),
            current_goal: snapshot.current_goal().to_owned(),
            status_summary: snapshot.status_summary().to_owned(),
            achievements: snapshot.achievements().to_vec(),
            blockers: snapshot.blockers().to_vec(),
            next_steps: snapshot.next_steps().to_vec(),
            confirmed_at: timestamp(snapshot.confirmed_at()),
            created_at: timestamp(snapshot.created_at()),
            updated_at: timestamp(snapshot.updated_at()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub goal: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionRequest {
    pub goal: String,
    #[serde(default)]
    pub notes: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSessionRequest {
    pub outcome: String,
    pub validation: String,
    #[serde(default)]
    pub handoff: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbandonSessionRequest {
    #[serde(default)]
    pub handoff: String,
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevelopmentSessionDto {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub goal: String,
    pub notes: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<DevelopmentSession> for DevelopmentSessionDto {
    fn from(session: DevelopmentSession) -> Self {
        Self {
            id: session.id().to_owned(),
            project_id: session.project_id().to_owned(),
            status: session.status().as_str().to_owned(),
            goal: session.goal().to_owned(),
            notes: session.notes().to_owned(),
            outcome: session.outcome().to_owned(),
            validation: session.validation().to_owned(),
            handoff: session.handoff().to_owned(),
            started_at: timestamp(session.started_at()),
            completed_at: session.completed_at().map(timestamp),
            created_at: timestamp(session.created_at()),
            updated_at: timestamp(session.updated_at()),
            version: session.version(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEvidenceRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub evidence_type: String,
    pub source_type: String,
    #[serde(default)]
    pub source_uri: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEvidenceRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub evidence_type: String,
    pub source_type: String,
    #[serde(default)]
    pub source_uri: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEvidenceStatusRequest {
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkEvidenceRequest {
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceDto {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub evidence_type: String,
    pub source_type: String,
    pub source_uri: String,
    pub verification_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<Evidence> for EvidenceDto {
    fn from(evidence: Evidence) -> Self {
        Self {
            id: evidence.id().to_owned(),
            project_id: evidence.project_id().to_owned(),
            title: evidence.title().to_owned(),
            description: evidence.description().to_owned(),
            evidence_type: evidence.evidence_type().as_str().to_owned(),
            source_type: evidence.source_type().as_str().to_owned(),
            source_uri: evidence.source_uri().to_owned(),
            verification_status: evidence.verification_status().as_str().to_owned(),
            created_at: timestamp(evidence.created_at()),
            updated_at: timestamp(evidence.updated_at()),
            version: evidence.version(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceLinkDto {
    pub id: String,
    pub evidence_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
}

impl From<EvidenceLink> for EvidenceLinkDto {
    fn from(link: EvidenceLink) -> Self {
        Self {
            id: link.id().to_owned(),
            evidence_id: link.evidence_id().to_owned(),
            target_type: link.target_type().as_str().to_owned(),
            target_id: link.target_id().to_owned(),
            relation_type: link.relation_type().as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceRecordDto {
    pub evidence: EvidenceDto,
    pub links: Vec<EvidenceLinkDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAiProviderRequest {
    pub name: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAiProviderRequest {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAiProviderEnabledRequest {
    pub enabled: bool,
    pub expected_version: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAiProviderApiKeyRequest {
    pub api_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeAiProviderRequest {
    pub messages: Vec<AiMessageRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub api_key_configured: bool,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<AiProviderView> for AiProviderDto {
    fn from(view: AiProviderView) -> Self {
        Self {
            id: view.provider.id().to_owned(),
            name: view.provider.name().to_owned(),
            base_url: view.provider.base_url().to_owned(),
            model: view.provider.model().to_owned(),
            enabled: view.provider.enabled(),
            api_key_configured: view.api_key_configured,
            created_at: timestamp(view.provider.created_at()),
            updated_at: timestamp(view.provider.updated_at()),
            version: view.provider.version(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTokenUsageDto {
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub total_tokens: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiInvocationResultDto {
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<AiTokenUsageDto>,
}

impl From<AiInvocationResult> for AiInvocationResultDto {
    fn from(result: AiInvocationResult) -> Self {
        Self {
            provider_id: result.provider_id,
            provider_name: result.provider_name,
            model: result.completion.model,
            content: result.completion.content,
            finish_reason: result.completion.finish_reason,
            usage: result.completion.usage.map(|usage| AiTokenUsageDto {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateContextPackRequest {
    pub template_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeContextWithAiRequest {
    pub provider_id: String,
    pub user_prompt: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextExportDto {
    pub id: String,
    pub project_id: String,
    pub template_type: String,
    pub snapshot_version: Option<i64>,
    pub generated_from_session_ids: Vec<String>,
    pub content_markdown: String,
    pub content_hash: String,
    pub status: String,
    pub created_at: String,
    pub version: i64,
}

impl From<ContextExport> for ContextExportDto {
    fn from(export: ContextExport) -> Self {
        Self {
            id: export.id().to_owned(),
            project_id: export.project_id().to_owned(),
            template_type: export.template_type().as_str().to_owned(),
            snapshot_version: export.snapshot_version(),
            generated_from_session_ids: export.generated_from_session_ids().to_vec(),
            content_markdown: export.content_markdown().to_owned(),
            content_hash: export.content_hash().to_owned(),
            status: export.status().as_str().to_owned(),
            created_at: timestamp(export.created_at()),
            version: export.version(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextAiInvocationResultDto {
    pub context_export: ContextExportDto,
    pub ai: AiInvocationResultDto,
}

impl From<ContextAiInvocationResult> for ContextAiInvocationResultDto {
    fn from(result: ContextAiInvocationResult) -> Self {
        Self {
            context_export: result.context_export.into(),
            ai: result.ai.into(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAiCandidateRequest {
    pub candidate_type: String,
    pub source_context_export_id: String,
    pub title: String,
    pub content_markdown: String,
}

impl From<CreateAiCandidateRequest> for CreateAiCandidateInput {
    fn from(input: CreateAiCandidateRequest) -> Self {
        Self {
            candidate_type: input.candidate_type,
            source_context_export_id: input.source_context_export_id,
            title: input.title,
            content_markdown: input.content_markdown,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewAiCandidateRequest {
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCandidateDto {
    pub id: String,
    pub project_id: String,
    pub candidate_type: String,
    pub source_context_export_id: String,
    pub title: String,
    pub content_markdown: String,
    pub status: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AiCandidate> for AiCandidateDto {
    fn from(candidate: AiCandidate) -> Self {
        Self {
            id: candidate.id().to_owned(),
            project_id: candidate.project_id().to_owned(),
            candidate_type: candidate.candidate_type().as_str().to_owned(),
            source_context_export_id: candidate.source_context_export_id().to_owned(),
            title: candidate.title().to_owned(),
            content_markdown: candidate.content_markdown().to_owned(),
            status: candidate.status().as_str().to_owned(),
            version: candidate.version(),
            created_at: timestamp(candidate.created_at()),
            updated_at: timestamp(candidate.updated_at()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodexTaskProposalRequest {
    pub context_export_id: String,
    pub title: String,
    pub description: String,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub allowed_scope: Vec<String>,
    pub forbidden_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub validation_commands: Vec<String>,
}

impl From<CreateCodexTaskProposalRequest> for CreateCodexTaskProposalInput {
    fn from(value: CreateCodexTaskProposalRequest) -> Self {
        Self {
            context_export_id: value.context_export_id,
            title: value.title,
            description: value.description,
            goal: value.goal,
            non_goals: value.non_goals,
            allowed_scope: value.allowed_scope,
            forbidden_scope: value.forbidden_scope,
            acceptance_criteria: value.acceptance_criteria,
            validation_commands: value.validation_commands,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionCodexTaskProposalRequest {
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexTaskProposalDto {
    pub id: String,
    pub project_id: String,
    pub context_export_id: String,
    pub title: String,
    pub description: String,
    pub goal: String,
    pub non_goals: Vec<String>,
    pub allowed_scope: Vec<String>,
    pub forbidden_scope: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub validation_commands: Vec<String>,
    pub status: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CodexTaskProposal> for CodexTaskProposalDto {
    fn from(value: CodexTaskProposal) -> Self {
        let details = value.details().clone();
        Self {
            id: value.id().into(),
            project_id: value.project_id().into(),
            context_export_id: value.context_export_id().into(),
            title: details.title,
            description: details.description,
            goal: details.goal,
            non_goals: details.non_goals,
            allowed_scope: details.allowed_scope,
            forbidden_scope: details.forbidden_scope,
            acceptance_criteria: details.acceptance_criteria,
            validation_commands: details.validation_commands,
            status: value.status().as_str().into(),
            version: value.version(),
            created_at: timestamp(value.created_at()),
            updated_at: timestamp(value.updated_at()),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRepositoryRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectRepositoryRequest {
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRepositoryRequest {
    pub expected_version: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommitEvidenceRequest {
    pub commit_sha: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSummaryDto {
    pub changed_files: i64,
    pub staged_files: i64,
    pub unstaged_files: i64,
    pub untracked_files: i64,
    pub insertions: i64,
    pub deletions: i64,
}

impl From<DiffSummary> for DiffSummaryDto {
    fn from(summary: DiffSummary) -> Self {
        Self {
            changed_files: summary.changed_files,
            staged_files: summary.staged_files,
            unstaged_files: summary.unstaged_files,
            untracked_files: summary.untracked_files,
            insertions: summary.insertions,
            deletions: summary.deletions,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryDto {
    pub id: String,
    pub project_id: String,
    pub path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub last_checked_at: String,
    pub version: i64,
}

impl From<Repository> for RepositoryDto {
    fn from(repository: Repository) -> Self {
        Self {
            id: repository.id().to_owned(),
            project_id: repository.project_id().to_owned(),
            path: repository.path().to_owned(),
            remote_url: repository.remote_url().map(str::to_owned),
            default_branch: repository.default_branch().to_owned(),
            last_checked_at: timestamp(repository.last_checked_at()),
            version: repository.version(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkingCopyDto {
    pub id: String,
    pub repository_id: String,
    pub branch: String,
    pub head_commit: String,
    pub is_dirty: bool,
    pub diff_summary: DiffSummaryDto,
    pub checked_at: String,
}

impl From<WorkingCopy> for WorkingCopyDto {
    fn from(working_copy: WorkingCopy) -> Self {
        Self {
            id: working_copy.id().to_owned(),
            repository_id: working_copy.repository_id().to_owned(),
            branch: working_copy.branch().to_owned(),
            head_commit: working_copy.head_commit().to_owned(),
            is_dirty: working_copy.is_dirty(),
            diff_summary: working_copy.diff_summary().into(),
            checked_at: timestamp(working_copy.checked_at()),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDto {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub authored_at: String,
}

impl From<GitCommit> for GitCommitDto {
    fn from(commit: GitCommit) -> Self {
        Self {
            sha: commit.sha,
            message: commit.message,
            author_name: commit.author_name,
            author_email: commit.author_email,
            authored_at: timestamp(commit.authored_at),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInspectionDto {
    pub root_path: String,
    pub remote_url: Option<String>,
    pub default_branch: String,
    pub branch: String,
    pub head_commit: String,
    pub is_dirty: bool,
    pub diff_summary: DiffSummaryDto,
    pub checked_at: String,
    pub recent_commits: Vec<GitCommitDto>,
}

impl From<GitInspection> for GitInspectionDto {
    fn from(inspection: GitInspection) -> Self {
        Self {
            root_path: inspection.root_path,
            remote_url: inspection.remote_url,
            default_branch: inspection.default_branch,
            branch: inspection.branch,
            head_commit: inspection.head_commit,
            is_dirty: inspection.is_dirty,
            diff_summary: inspection.diff_summary.into(),
            checked_at: timestamp(inspection.checked_at),
            recent_commits: inspection
                .recent_commits
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryRecordDto {
    pub repository: RepositoryDto,
    pub working_copy: WorkingCopyDto,
    pub recent_commits: Vec<GitCommitDto>,
}

impl From<RepositoryRecord> for RepositoryRecordDto {
    fn from(record: RepositoryRecord) -> Self {
        Self {
            repository: record.repository.into(),
            working_copy: record.working_copy.into(),
            recent_commits: record.recent_commits.into_iter().map(Into::into).collect(),
        }
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
