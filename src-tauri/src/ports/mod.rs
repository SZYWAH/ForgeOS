//! Persistence interfaces required by ForgeOS application use cases.

mod ai;
mod codex;
mod context;
mod git;
mod github;
mod repositories;

pub use ai::{
    AiClientError, AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiTokenUsage,
    ApiKeyStore, SecretStoreError,
};
pub use codex::{CodexExecutionRequest, CodexRunner, CodexRunnerError};
pub use context::{
    redact_external_text, redact_git_evidence_description, ContextRenderError, ContextRenderInput,
    ContextRenderer,
};
pub use git::{GitAdapter, GitAdapterError, GitCommit, GitInspection};
pub use github::{GitHubClient, GitHubClientError};
pub use repositories::{
    AiCandidateKernelRepository, AiCandidateRepository, AiProviderRepository,
    CodexExecutionKernelRepository, CodexExecutionRepository, CodexTaskProposalKernelRepository,
    CodexTaskProposalRepository, ContextExportRepository, ContextPackRepository,
    EvidenceRepository, GitHubImportKernelRepository, GitHubImportRepository, McpGitEvidenceRecord,
    McpQueryRepository, ProjectKernelRepository, ProjectRepository, RepositoryError,
    RepositoryKernelStore, RepositoryStore, SessionRepository, SnapshotRepository,
    WorkspaceRepository,
};
