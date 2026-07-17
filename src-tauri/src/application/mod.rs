//! Application use cases coordinate domain rules and repository ports.

pub mod achievement;
pub mod activity_episode;
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
pub mod memory_content;
pub mod project;
pub mod project_memory;
pub mod project_pulse;
pub mod repository;
pub mod session;
pub mod snapshot;
pub mod workspace;

pub use achievement::{AchievementError, AchievementInput, AchievementKernel};
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
    ProjectIdentityView, ProjectOverviewView, ProjectPulseView, RecentSessionView,
    RecommendedNextStepView,
};
pub use memory_content::{
    MemoryContentPolicy, MemoryQualityCode, MemoryQualityIssue, MemoryQualitySeverity,
};
pub use project_memory::{
    DevelopmentHistoryItem, DevelopmentHistoryPage, GitCommitSummary, ProjectAiContext,
    ProjectMemoryError, ProjectMemoryOverview, ProjectMemoryQueryKernel, ProjectMemorySummary,
    SnapshotRepairDraft,
};
pub use project_pulse::{ProjectPulseError, ProjectPulseKernel};
pub use repository::{RepositoryKernel, RepositoryKernelError};
pub mod app_preferences;
pub use app_preferences::{AppPreferencesError, AppPreferencesKernel};
