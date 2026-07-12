use schemars::JsonSchema;
use serde::Serialize;

use forgeos_lib::application::{
    ContextPackView, CurrentSnapshotView, EvidenceView, ProjectIdentityView, RecentSessionView,
};

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectIdentityResult {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub tech_stack: String,
    pub lifecycle_stage: String,
    pub status: String,
}

impl From<ProjectIdentityView> for ProjectIdentityResult {
    fn from(value: ProjectIdentityView) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
            description: value.description,
            vision: value.vision,
            target_user: value.target_user,
            tech_stack: value.tech_stack,
            lifecycle_stage: value.lifecycle_stage,
            status: value.status,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CurrentSnapshotResult {
    pub project_id: String,
    pub available: bool,
    pub current_goal: Option<String>,
    pub status_summary: Option<String>,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub snapshot_version: Option<i64>,
    pub confirmed_at: Option<String>,
}

impl CurrentSnapshotResult {
    pub fn from_optional(project_id: String, value: Option<CurrentSnapshotView>) -> Self {
        match value {
            Some(value) => Self {
                project_id,
                available: true,
                current_goal: Some(value.current_goal),
                status_summary: Some(value.status_summary),
                achievements: value.achievements,
                blockers: value.blockers,
                next_steps: value.next_steps,
                snapshot_version: Some(value.snapshot_version),
                confirmed_at: Some(value.confirmed_at.to_rfc3339()),
            },
            None => Self {
                project_id,
                available: false,
                current_goal: None,
                status_summary: None,
                achievements: Vec::new(),
                blockers: Vec::new(),
                next_steps: Vec::new(),
                snapshot_version: None,
                confirmed_at: None,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecentSessionResult {
    pub goal: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: String,
    pub completed_at: Option<String>,
}

impl From<RecentSessionView> for RecentSessionResult {
    fn from(value: RecentSessionView) -> Self {
        Self {
            goal: value.goal,
            outcome: value.outcome,
            validation: value.validation,
            handoff: value.handoff,
            started_at: value.started_at.to_rfc3339(),
            completed_at: value.completed_at.map(|time| time.to_rfc3339()),
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecentSessionsResult {
    pub project_id: String,
    pub sessions: Vec<RecentSessionResult>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct EvidenceResult {
    pub title: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub description: String,
    pub verification_status: String,
}

impl From<EvidenceView> for EvidenceResult {
    fn from(value: EvidenceView) -> Self {
        Self {
            title: value.title,
            evidence_type: value.evidence_type,
            description: value.description,
            verification_status: value.verification_status,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct EvidenceListResult {
    pub project_id: String,
    pub evidence: Vec<EvidenceResult>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ContextPackResult {
    pub project_id: String,
    pub template_type: String,
    pub snapshot_version: Option<i64>,
    pub generated_from_session_ids: Vec<String>,
    pub content_markdown: String,
    pub content_hash: String,
}

impl From<ContextPackView> for ContextPackResult {
    fn from(value: ContextPackView) -> Self {
        Self {
            project_id: value.project_id,
            template_type: value.template_type,
            snapshot_version: value.snapshot_version,
            generated_from_session_ids: value.generated_from_session_ids,
            content_markdown: value.content_markdown,
            content_hash: value.content_hash,
        }
    }
}
