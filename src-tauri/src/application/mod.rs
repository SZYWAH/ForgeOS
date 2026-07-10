//! Application use cases coordinate domain rules and repository ports.

mod error;
mod kernel;
pub mod project;
pub mod session;
pub mod snapshot;
pub mod workspace;

pub use error::ApplicationError;
pub use kernel::ProjectKernel;
