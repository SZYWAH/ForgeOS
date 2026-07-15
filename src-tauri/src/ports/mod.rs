//! Persistence interfaces required by ForgeOS application use cases.

mod ai;
mod codex;
mod codex_activity;
mod context;
mod git;
mod github;
mod repositories;

pub use ai::{
    AiClientError, AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiTokenUsage,
    ApiKeyStore, SecretStoreError,
};
pub use codex::{CodexExecutionRequest, CodexRunner, CodexRunnerError};
pub use codex_activity::{
    CodexActivityBatch, CodexActivitySource, CodexActivitySourceError, CodexConnectorHealth,
    CodexTurnActivity,
};
pub use context::{
    redact_external_text, redact_git_evidence_description, ContextRenderError, ContextRenderInput,
    ContextRenderer,
};
pub use git::{GitAdapter, GitAdapterError, GitCommit, GitInspection};
pub use github::{GitHubClient, GitHubClientError};
pub use repositories::{
    AiCandidateKernelRepository, AiCandidateRepository, AiProviderRepository,
    CodexExecutionKernelRepository, CodexExecutionRepository, CodexSyncCursor,
    CodexTaskProposalKernelRepository, CodexTaskProposalRepository, ContextExportRepository,
    ContextPackRepository, EvidenceRepository, GitHubImportKernelRepository,
    GitHubImportRepository, McpGitEvidenceRecord, McpQueryRepository, ProjectActivityRepository,
    ProjectKernelRepository, ProjectPulseKernelRepository, ProjectRepository,
    PulseConfirmationWrite, RepositoryError, RepositoryKernelStore, RepositoryStore,
    SessionRepository, SnapshotRepository, WorkspaceRepository,
};
mod app_preferences;
pub use app_preferences::AppPreferencesRepository;
