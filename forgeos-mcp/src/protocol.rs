use forgeos_lib::{
    application::{DevelopmentHistoryItem, ProjectAiContext, ProjectMemorySummary},
    domain::ProjectAchievement,
};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectListItem {
    pub project_id: String,
    pub name: String,
    pub status: String,
    pub repository_connected: bool,
}

impl From<ProjectMemorySummary> for ProjectListItem {
    fn from(value: ProjectMemorySummary) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
            status: value.status,
            repository_connected: value.has_repository,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectListResult {
    pub projects: Vec<ProjectListItem>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct CurrentStateResult {
    pub available: bool,
    pub current_goal: Option<String>,
    pub status_summary: Option<String>,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub version: Option<i64>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ObservedStateResult {
    pub available: bool,
    pub fact_status: Option<String>,
    pub current_goal: Option<String>,
    pub status_summary: Option<String>,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub generated_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectContextResult {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub tech_stack: String,
    pub lifecycle_stage: String,
    pub status: String,
    pub confirmed_state: CurrentStateResult,
    pub observed_state: ObservedStateResult,
}

impl ProjectContextResult {
    pub fn from_safe_context(
        value: ProjectAiContext,
        lifecycle_stage: String,
        status: String,
    ) -> Self {
        let snapshot = value.confirmed_state;
        let pulse = value.observed_state;
        Self {
            project_id: value.project_id,
            name: value.identity.name,
            description: value.identity.description,
            vision: value.identity.vision,
            target_user: value.identity.target_user,
            scope: value.identity.scope_summary,
            tech_stack: value.identity.tech_stack,
            lifecycle_stage,
            status,
            confirmed_state: CurrentStateResult {
                available: snapshot.is_some(),
                current_goal: snapshot.as_ref().map(|item| item.current_goal.clone()),
                status_summary: snapshot.as_ref().map(|item| item.status_summary.clone()),
                achievements: snapshot
                    .as_ref()
                    .map(|item| item.achievements.clone())
                    .unwrap_or_default(),
                blockers: snapshot
                    .as_ref()
                    .map(|item| item.blockers.clone())
                    .unwrap_or_default(),
                next_steps: snapshot
                    .as_ref()
                    .map(|item| item.next_steps.clone())
                    .unwrap_or_default(),
                version: snapshot.as_ref().map(|item| item.version),
            },
            observed_state: ObservedStateResult {
                available: pulse.is_some(),
                fact_status: pulse.as_ref().map(|item| item.fact_status.clone()),
                current_goal: pulse.as_ref().map(|item| item.current_goal.clone()),
                status_summary: pulse.as_ref().map(|item| item.status_summary.clone()),
                achievements: pulse
                    .as_ref()
                    .map(|item| item.achievements.clone())
                    .unwrap_or_default(),
                blockers: pulse
                    .as_ref()
                    .map(|item| item.blockers.clone())
                    .unwrap_or_default(),
                next_steps: pulse
                    .as_ref()
                    .map(|item| item.next_steps.clone())
                    .unwrap_or_default(),
                generated_at: pulse.as_ref().map(|item| item.generated_at.to_rfc3339()),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct GitCommitResult {
    pub sha: String,
    pub message: String,
    pub author_name: String,
    pub authored_at: String,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct DevelopmentHistoryItemResult {
    pub id: String,
    pub kind: String,
    pub trust_status: String,
    pub title: String,
    pub goal: String,
    pub rationale: String,
    pub activity: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub codex_activity_count: usize,
    pub git_commits: Vec<GitCommitResult>,
}

impl From<DevelopmentHistoryItem> for DevelopmentHistoryItemResult {
    fn from(value: DevelopmentHistoryItem) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            trust_status: value.trust_status,
            title: value.title,
            goal: value.goal,
            rationale: value.rationale,
            activity: value.activity,
            outcome: value.outcome,
            validation: value.validation,
            handoff: value.handoff,
            started_at: value.started_at.to_rfc3339(),
            ended_at: value.ended_at.map(|item| item.to_rfc3339()),
            codex_activity_count: value.codex_turn_count,
            git_commits: value
                .git_commits
                .into_iter()
                .map(|item| GitCommitResult {
                    sha: item.sha,
                    message: item.message,
                    author_name: item.author_name,
                    authored_at: item.authored_at.to_rfc3339(),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectHistoryResult {
    pub project_id: String,
    pub items: Vec<DevelopmentHistoryItemResult>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct AchievementResult {
    pub id: String,
    pub category: String,
    pub title: String,
    pub problem: String,
    pub approach: String,
    pub result: String,
    pub validation: String,
    pub impact: String,
    pub technologies: Vec<String>,
    pub resume_bullet: String,
    pub featured: bool,
    pub confirmed_at: Option<String>,
}

impl From<ProjectAchievement> for AchievementResult {
    fn from(value: ProjectAchievement) -> Self {
        Self {
            id: value.id,
            category: value.details.category.as_str().into(),
            title: value.details.title,
            problem: value.details.problem,
            approach: value.details.approach,
            result: value.details.result,
            validation: value.details.validation,
            impact: value.details.impact,
            technologies: value.details.technologies,
            resume_bullet: value.details.resume_bullet,
            featured: value.details.featured,
            confirmed_at: value.confirmed_at.map(|item| item.to_rfc3339()),
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct ProjectAchievementsResult {
    pub project_id: String,
    pub achievements: Vec<AchievementResult>,
}

#[derive(Clone, Debug, Serialize, JsonSchema)]
pub struct AiContextResult {
    pub project_id: String,
    pub content_markdown: String,
    pub content_hash: String,
}

impl From<ProjectAiContext> for AiContextResult {
    fn from(value: ProjectAiContext) -> Self {
        Self {
            project_id: value.project_id,
            content_markdown: value.content_markdown,
            content_hash: value.content_hash,
        }
    }
}
