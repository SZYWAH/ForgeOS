//! Application use cases coordinate domain rules and repository ports.

mod error;
mod kernel;
pub mod project;
pub mod workspace;

pub use error::ApplicationError;
pub use kernel::ProjectKernel;
