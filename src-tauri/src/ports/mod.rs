//! Persistence interfaces required by ForgeOS application use cases.

mod repositories;

pub use repositories::{
    ProjectKernelRepository, ProjectRepository, RepositoryError, WorkspaceRepository,
};
