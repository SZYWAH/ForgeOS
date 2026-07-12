//! Application use cases coordinate domain rules and repository ports.

pub mod ai_candidate;
pub mod ai_provider;
pub mod context_pack;
mod error;
pub mod evidence;
mod kernel;
pub mod mcp_query;
pub mod project;
pub mod repository;
pub mod session;
pub mod snapshot;
pub mod workspace;

pub use ai_candidate::{AiCandidateError, AiCandidateKernel};
pub use ai_provider::{AiProviderError, AiProviderKernel};
pub use context_pack::{ContextPackError, ContextPackKernel};
pub use error::ApplicationError;
pub use kernel::ProjectKernel;
pub use mcp_query::{
    ContextPackView, CurrentSnapshotView, EvidenceView, McpQueryError, McpQueryKernel,
    ProjectIdentityView, RecentSessionView,
};
pub use repository::{RepositoryKernel, RepositoryKernelError};
