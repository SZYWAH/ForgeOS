//! Persistence interfaces required by ForgeOS application use cases.

mod ai;
mod context;
mod git;
mod repositories;

pub use ai::{
    AiClientError, AiCompletion, AiMessage, AiMessageRole, AiProviderClient, AiTokenUsage,
    ApiKeyStore, SecretStoreError,
};
pub use context::{ContextRenderError, ContextRenderInput, ContextRenderer};
pub use git::{GitAdapter, GitAdapterError, GitCommit, GitInspection};
pub use repositories::{
    AiCandidateKernelRepository, AiCandidateRepository, AiProviderRepository,
    ContextExportRepository, ContextPackRepository, EvidenceRepository, McpQueryRepository,
    ProjectKernelRepository, ProjectRepository, RepositoryError, RepositoryKernelStore,
    RepositoryStore, SessionRepository, SnapshotRepository, WorkspaceRepository,
};
