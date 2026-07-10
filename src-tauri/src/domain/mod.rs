//! ForgeOS domain model and invariants.
//!
//! This module has no dependency on Tauri, SQLx, the file system, or external
//! integrations. Workspace, Project identity, Current Snapshot, and Development
//! Session rules live here so every delivery surface observes the same invariants.

mod error;
mod project;
mod session;
mod snapshot;
mod workspace;

pub use error::DomainError;
pub use project::{LifecycleStage, Project, ProjectDetails, ProjectStatus, ProjectUpdate};
pub use session::{DevelopmentSession, SessionStatus};
pub use snapshot::{ProjectSnapshot, SnapshotDetails};
pub use workspace::Workspace;
