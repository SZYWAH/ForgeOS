//! ForgeOS domain model and invariants.
//!
//! This module has no dependency on Tauri, SQLx, the file system, or external
//! integrations. Workspace, Project identity, Current Snapshot, and Development
//! Session rules live here so every delivery surface observes the same invariants.

mod ai_candidate;
mod ai_provider;
mod codex_task_proposal;
mod context_export;
mod error;
mod evidence;
mod project;
mod repository;
mod session;
mod snapshot;
mod workspace;

pub use ai_candidate::{AiCandidate, AiCandidateStatus, AiCandidateType};
pub use ai_provider::{AiProvider, AiProviderDetails};
pub use codex_task_proposal::{
    CodexTaskProposal, CodexTaskProposalDetails, CodexTaskProposalStatus,
};
pub use context_export::{ContextExport, ContextExportStatus, ContextTemplateType};
pub use error::DomainError;
pub use evidence::{
    Evidence, EvidenceDetails, EvidenceLink, EvidenceRelationType, EvidenceSourceType,
    EvidenceTargetType, EvidenceType, VerificationStatus,
};
pub use project::{LifecycleStage, Project, ProjectDetails, ProjectStatus, ProjectUpdate};
pub use repository::{validate_commit_sha, DiffSummary, Repository, WorkingCopy};
pub use session::{DevelopmentSession, SessionStatus};
pub use snapshot::{ProjectSnapshot, SnapshotDetails};
pub use workspace::Workspace;
