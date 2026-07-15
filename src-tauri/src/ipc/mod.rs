pub mod ai_candidates;
pub mod ai_providers;
pub mod codex_executions;
pub mod codex_task_proposals;
pub mod context_packs;
mod dto;
mod error;
pub mod evidences;
pub mod github;
pub mod health;
pub mod project_pulse;
pub mod projects;
pub mod repositories;
pub mod sessions;
pub mod snapshots;
pub mod workspaces;

pub use error::IpcError;
pub mod app_preferences;
