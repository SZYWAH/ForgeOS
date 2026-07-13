use async_trait::async_trait;
use thiserror::Error;

use crate::domain::{
    AiCandidate, AiProvider, CodexExecutionRecord, CodexTaskProposal, ContextExport,
    DevelopmentSession, Evidence, EvidenceLink, GitHubConnector, GitHubImportDraft, Project,
    ProjectSnapshot, Repository, WorkingCopy, Workspace,
};

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("a record with the same unique identity already exists")]
    Conflict,

    #[error("the record changed after it was read")]
    OptimisticConflict,

    #[error("the Project is archived and cannot accept this change")]
    ProjectArchived,

    #[error("Evidence link target is missing or belongs to another Project")]
    EvidenceTargetInvalid,

    #[error("persisted ForgeOS data is invalid: {0}")]
    CorruptData(String),

    #[error("storage operation failed: {0}")]
    Storage(String),
}

#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError>;
    async fn insert_workspace(&self, workspace: &Workspace) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn insert_project(&self, project: &Project) -> Result<(), RepositoryError>;

    async fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, RepositoryError>;

    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError>;

    async fn update_project(
        &self,
        project: &Project,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    async fn insert_snapshot(&self, snapshot: &ProjectSnapshot) -> Result<(), RepositoryError>;

    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError>;

    async fn list_snapshot_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSnapshot>, RepositoryError>;

    async fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn insert_session(&self, session: &DevelopmentSession) -> Result<(), RepositoryError>;

    async fn get_current_session(
        &self,
        project_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError>;

    async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, RepositoryError>;

    async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError>;

    async fn update_open_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;

    async fn finish_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn insert_evidence(&self, evidence: &Evidence) -> Result<(), RepositoryError>;

    async fn list_evidences(&self, project_id: &str) -> Result<Vec<Evidence>, RepositoryError>;

    async fn get_evidence(&self, evidence_id: &str) -> Result<Option<Evidence>, RepositoryError>;

    async fn update_evidence(
        &self,
        evidence: &Evidence,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;

    async fn insert_evidence_link(&self, link: &EvidenceLink) -> Result<(), RepositoryError>;

    async fn list_evidence_links(
        &self,
        evidence_id: &str,
    ) -> Result<Vec<EvidenceLink>, RepositoryError>;
}

#[async_trait]
pub trait McpQueryRepository: Send + Sync {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError>;
    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError>;
    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError>;
    async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, RepositoryError>;
    async fn list_evidences(&self, project_id: &str) -> Result<Vec<Evidence>, RepositoryError>;
}

#[async_trait]
pub trait AiProviderRepository: Send + Sync {
    async fn insert_ai_provider(&self, provider: &AiProvider) -> Result<(), RepositoryError>;

    async fn list_ai_providers(&self) -> Result<Vec<AiProvider>, RepositoryError>;

    async fn get_ai_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<AiProvider>, RepositoryError>;

    async fn update_ai_provider(
        &self,
        provider: &AiProvider,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait ContextExportRepository: Send + Sync {
    async fn insert_context_export(&self, export: &ContextExport) -> Result<(), RepositoryError>;

    async fn list_context_exports(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContextExport>, RepositoryError>;

    async fn get_context_export(
        &self,
        export_id: &str,
    ) -> Result<Option<ContextExport>, RepositoryError>;

    async fn update_context_export_status(
        &self,
        export: &ContextExport,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait AiCandidateRepository: Send + Sync {
    async fn insert_ai_candidate(&self, candidate: &AiCandidate) -> Result<(), RepositoryError>;

    async fn list_ai_candidates(
        &self,
        project_id: &str,
    ) -> Result<Vec<AiCandidate>, RepositoryError>;

    async fn get_ai_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<AiCandidate>, RepositoryError>;

    async fn update_ai_candidate_status(
        &self,
        candidate: &AiCandidate,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait CodexTaskProposalRepository: Send + Sync {
    async fn insert_codex_task_proposal(
        &self,
        proposal: &CodexTaskProposal,
    ) -> Result<(), RepositoryError>;
    async fn list_codex_task_proposals(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexTaskProposal>, RepositoryError>;
    async fn get_codex_task_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<Option<CodexTaskProposal>, RepositoryError>;
    async fn transition_codex_task_proposal(
        &self,
        proposal: &CodexTaskProposal,
        from_status: &str,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait CodexExecutionRepository: Send + Sync {
    async fn insert_codex_execution(
        &self,
        execution: &CodexExecutionRecord,
    ) -> Result<(), RepositoryError>;
    async fn list_codex_executions(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexExecutionRecord>, RepositoryError>;
    async fn get_codex_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<CodexExecutionRecord>, RepositoryError>;
    async fn transition_codex_execution(
        &self,
        execution: &CodexExecutionRecord,
        from_status: &str,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait RepositoryStore: Send + Sync {
    async fn insert_repository_with_working_copy(
        &self,
        repository: &Repository,
        working_copy: &WorkingCopy,
    ) -> Result<(), RepositoryError>;

    async fn list_repositories(&self, project_id: &str)
        -> Result<Vec<Repository>, RepositoryError>;

    async fn get_repository(
        &self,
        repository_id: &str,
    ) -> Result<Option<Repository>, RepositoryError>;

    async fn get_current_working_copy(
        &self,
        repository_id: &str,
    ) -> Result<Option<WorkingCopy>, RepositoryError>;

    async fn update_repository_with_working_copy(
        &self,
        repository: &Repository,
        expected_version: i64,
        working_copy: &WorkingCopy,
    ) -> Result<(), RepositoryError>;

    async fn insert_git_evidence(
        &self,
        evidence: &Evidence,
        repository_id: &str,
        commit_sha: Option<&str>,
    ) -> Result<(), RepositoryError>;

    async fn list_repository_evidence(
        &self,
        repository_id: &str,
    ) -> Result<Vec<Evidence>, RepositoryError>;
}

#[async_trait]
pub trait GitHubImportRepository: Send + Sync {
    async fn get_github_connector(&self) -> Result<Option<GitHubConnector>, RepositoryError>;
    async fn insert_github_connector(
        &self,
        connector: &GitHubConnector,
    ) -> Result<(), RepositoryError>;
    async fn update_github_connector(
        &self,
        connector: &GitHubConnector,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
    async fn insert_github_import_draft(
        &self,
        draft: &GitHubImportDraft,
    ) -> Result<(), RepositoryError>;
    async fn list_github_import_drafts(&self) -> Result<Vec<GitHubImportDraft>, RepositoryError>;
    async fn get_github_import_draft(
        &self,
        draft_id: &str,
    ) -> Result<Option<GitHubImportDraft>, RepositoryError>;
    async fn update_github_import_draft(
        &self,
        draft: &GitHubImportDraft,
        expected_version: i64,
    ) -> Result<(), RepositoryError>;
    async fn import_github_draft(
        &self,
        draft: &GitHubImportDraft,
        expected_version: i64,
        project: &Project,
        evidences: &[(Evidence, String, String)],
    ) -> Result<(), RepositoryError>;
    async fn get_github_import_for_project(
        &self,
        project_id: &str,
    ) -> Result<Option<GitHubImportDraft>, RepositoryError>;
}

pub trait GitHubImportKernelRepository:
    WorkspaceRepository + ProjectRepository + GitHubImportRepository
{
}

impl<T> GitHubImportKernelRepository for T where
    T: WorkspaceRepository + ProjectRepository + GitHubImportRepository
{
}

pub trait RepositoryKernelStore: WorkspaceRepository + ProjectRepository + RepositoryStore {}

impl<T> RepositoryKernelStore for T where
    T: WorkspaceRepository + ProjectRepository + RepositoryStore
{
}

pub trait AiCandidateKernelRepository:
    WorkspaceRepository + ProjectRepository + ContextExportRepository + AiCandidateRepository
{
}

pub trait CodexTaskProposalKernelRepository:
    WorkspaceRepository + ProjectRepository + ContextExportRepository + CodexTaskProposalRepository
{
}

pub trait CodexExecutionKernelRepository:
    WorkspaceRepository + ProjectRepository + CodexTaskProposalRepository + CodexExecutionRepository
{
}

impl<T> CodexExecutionKernelRepository for T where
    T: WorkspaceRepository
        + ProjectRepository
        + CodexTaskProposalRepository
        + CodexExecutionRepository
{
}

impl<T> CodexTaskProposalKernelRepository for T where
    T: WorkspaceRepository
        + ProjectRepository
        + ContextExportRepository
        + CodexTaskProposalRepository
{
}

impl<T> AiCandidateKernelRepository for T where
    T: WorkspaceRepository + ProjectRepository + ContextExportRepository + AiCandidateRepository
{
}

pub trait ContextPackRepository:
    WorkspaceRepository
    + ProjectRepository
    + SnapshotRepository
    + SessionRepository
    + EvidenceRepository
    + ContextExportRepository
{
}

impl<T> ContextPackRepository for T where
    T: WorkspaceRepository
        + ProjectRepository
        + SnapshotRepository
        + SessionRepository
        + EvidenceRepository
        + ContextExportRepository
{
}

pub trait ProjectKernelRepository:
    WorkspaceRepository
    + ProjectRepository
    + SnapshotRepository
    + SessionRepository
    + EvidenceRepository
{
}

impl<T> ProjectKernelRepository for T where
    T: WorkspaceRepository
        + ProjectRepository
        + SnapshotRepository
        + SessionRepository
        + EvidenceRepository
{
}
