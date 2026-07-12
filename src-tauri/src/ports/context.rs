use thiserror::Error;

use crate::domain::{ContextTemplateType, DevelopmentSession, Evidence, Project, ProjectSnapshot};

pub struct ContextRenderInput<'a> {
    pub template_type: ContextTemplateType,
    pub project: &'a Project,
    pub snapshot: Option<&'a ProjectSnapshot>,
    pub recent_session: Option<&'a DevelopmentSession>,
    pub evidences: &'a [Evidence],
}

#[derive(Debug, Error)]
pub enum ContextRenderError {
    #[error("Context Pack rendering failed: {0}")]
    Render(String),
}

pub trait ContextRenderer: Send + Sync {
    fn render(&self, input: ContextRenderInput<'_>) -> Result<String, ContextRenderError>;
}
