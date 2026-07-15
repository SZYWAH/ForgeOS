use schemars::JsonSchema;
use serde::Serialize;

use forgeos_lib::application::{
    ContextPackView, CurrentSnapshotView, EvidenceRelationView, EvidenceSummaryView, EvidenceView,
    GitActivityView, ProjectIdentityView, ProjectPulseView, RecentSessionView,
    RecommendedNextStepView,
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

pub type ProjectOverviewResult = ProjectIdentityResult;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectPulseResult {
    pub project_id: String,
    pub available: bool,
    pub fact_status: Option<String>,
    pub activity_type: Option<String>,
    pub analysis_revision: Option<String>,
    pub generated_at: Option<String>,
    pub freshness: Option<String>,
    pub current_goal: Option<String>,
    pub status_summary: Option<String>,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub validation_summary: Option<String>,
    pub git_summary: Option<String>,
    pub source_count: usize,
    pub selected_source_count: i64,
    pub available_source_count: i64,
    pub content_hash: Option<String>,
}

impl ProjectPulseResult {
    pub fn from_optional(project_id: String, value: Option<ProjectPulseView>) -> Self {
        match value {
            Some(value) => Self {
                project_id,
                available: true,
                fact_status: Some(value.fact_status),
                activity_type: Some(value.activity_type),
                analysis_revision: Some(value.analysis_revision),
                generated_at: Some(value.generated_at.to_rfc3339()),
                freshness: Some(value.freshness),
                current_goal: Some(value.current_goal),
                status_summary: Some(value.status_summary),
                achievements: value.achievements,
                blockers: value.blockers,
                next_steps: value.next_steps,
                validation_summary: Some(value.validation_summary),
                git_summary: Some(value.git_summary),
                source_count: value.source_count,
                selected_source_count: value.selected_source_count,
                available_source_count: value.available_source_count,
                content_hash: Some(value.content_hash),
            },
            None => Self {
                project_id,
                available: false,
                fact_status: None,
                activity_type: None,
                analysis_revision: None,
                generated_at: None,
                freshness: None,
                current_goal: None,
                status_summary: None,
                achievements: Vec::new(),
                blockers: Vec::new(),
                next_steps: Vec::new(),
                validation_summary: None,
                git_summary: None,
                source_count: 0,
                selected_source_count: 0,
                available_source_count: 0,
                content_hash: None,
            },
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

pub type CurrentStateResult = CurrentSnapshotResult;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecentSessionResult {
    pub session_id: String,
    pub status: String,
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
            session_id: value.session_id,
            status: value.status,
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

pub type DevelopmentHistoryResult = RecentSessionsResult;

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
pub struct EvidenceRelationResult {
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
}

impl From<EvidenceRelationView> for EvidenceRelationResult {
    fn from(value: EvidenceRelationView) -> Self {
        Self {
            target_type: value.target_type,
            target_id: value.target_id,
            relation_type: value.relation_type,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct EvidenceSummaryResult {
    pub evidence_id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub verification_status: String,
    pub relations: Vec<EvidenceRelationResult>,
}

impl From<EvidenceSummaryView> for EvidenceSummaryResult {
    fn from(value: EvidenceSummaryView) -> Self {
        Self {
            evidence_id: value.evidence_id,
            title: value.title,
            evidence_type: value.evidence_type,
            verification_status: value.verification_status,
            relations: value
                .relations
                .into_iter()
                .map(EvidenceRelationResult::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectEvidenceSummaryResult {
    pub project_id: String,
    pub evidence: Vec<EvidenceSummaryResult>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct GitActivityResult {
    pub evidence_id: String,
    pub title: String,
    pub activity_type: String,
    pub commit_sha: Option<String>,
    pub message: Option<String>,
    pub author: Option<String>,
    pub date: String,
    pub status_summary: Option<String>,
    pub verification_status: String,
    pub captured_at: String,
}

impl From<GitActivityView> for GitActivityResult {
    fn from(value: GitActivityView) -> Self {
        Self {
            evidence_id: value.evidence_id,
            title: value.title,
            activity_type: value.activity_type,
            commit_sha: value.commit_sha,
            message: value.message,
            author: value.author,
            date: value.date,
            status_summary: value.status_summary,
            verification_status: value.verification_status,
            captured_at: value.captured_at.to_rfc3339(),
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecentGitActivityResult {
    pub project_id: String,
    pub activities: Vec<GitActivityResult>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecommendedNextStepResult {
    pub priority: usize,
    pub source_type: String,
    pub source_id: String,
    pub recommendation: String,
    pub reason: String,
}

impl From<RecommendedNextStepView> for RecommendedNextStepResult {
    fn from(value: RecommendedNextStepView) -> Self {
        Self {
            priority: value.priority,
            source_type: value.source_type,
            source_id: value.source_id,
            recommendation: value.recommendation,
            reason: value.reason,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct RecommendedNextStepsResult {
    pub project_id: String,
    pub strategy: String,
    pub recommendations: Vec<RecommendedNextStepResult>,
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
