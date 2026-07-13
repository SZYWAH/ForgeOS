//! Application use cases coordinate domain rules and repository ports.

pub mod ai_candidate;
pub mod ai_provider;
pub mod codex_execution;
pub mod codex_task_proposal;
pub mod context_pack;
mod error;
pub mod evidence;
pub mod github_import;
mod kernel;
pub mod mcp_query;
pub mod project;
pub mod repository;
pub mod session;
pub mod snapshot;
pub mod workspace;

pub use ai_candidate::{AiCandidateError, AiCandidateKernel};
pub use ai_provider::{AiProviderError, AiProviderKernel};
pub use codex_execution::{CodexExecutionError, CodexExecutionKernel};
pub use codex_task_proposal::{CodexTaskProposalError, CodexTaskProposalKernel};
pub use context_pack::{ContextPackError, ContextPackKernel};
pub use error::ApplicationError;
pub use github_import::{GitHubImportError, GitHubImportKernel};
pub use kernel::ProjectKernel;
pub use mcp_query::{
    ContextPackView, CurrentSnapshotView, CurrentStateView, EvidenceRelationView,
    EvidenceSummaryView, EvidenceView, GitActivityView, McpQueryError, McpQueryKernel,
    ProjectIdentityView, ProjectOverviewView, RecentSessionView, RecommendedNextStepView,
};
pub use repository::{RepositoryKernel, RepositoryKernelError};
