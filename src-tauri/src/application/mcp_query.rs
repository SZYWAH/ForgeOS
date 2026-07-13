use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
    sync::Arc,
};

use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;

use crate::{
    domain::{
        AiCandidateStatus, ContextExport, ContextTemplateType, DomainError, EvidenceType, Project,
        VerificationStatus,
    },
    ports::{
        redact_external_text, redact_git_evidence_description, ContextRenderInput, ContextRenderer,
        McpQueryRepository, RepositoryError,
    },
};

const RECENT_SESSION_LIMIT: usize = 5;
const RECENT_GIT_ACTIVITY_LIMIT: usize = 20;
const RECOMMENDATION_LIMIT: usize = 20;

#[derive(Debug, Error)]
pub enum McpQueryError {
    #[error("Project not found in the current ForgeOS Workspace")]
    ProjectNotFound,
    #[error("ForgeOS has no Workspace")]
    WorkspaceNotFound,
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("Context Pack rendering failed: {0}")]
    Render(String),
    #[error("persisted ForgeOS data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProjectIdentityView {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub tech_stack: String,
    pub lifecycle_stage: String,
    pub status: String,
}

pub type ProjectOverviewView = ProjectIdentityView;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CurrentSnapshotView {
    pub project_id: String,
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub snapshot_version: i64,
    pub confirmed_at: DateTime<Utc>,
}

pub type CurrentStateView = CurrentSnapshotView;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecentSessionView {
    pub session_id: String,
    pub status: String,
    pub goal: String,
    pub outcome: String,
    pub validation: String,
    pub handoff: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceView {
    pub title: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub description: String,
    pub verification_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceRelationView {
    pub target_type: String,
    pub target_id: String,
    pub relation_type: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvidenceSummaryView {
    pub evidence_id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub evidence_type: String,
    pub verification_status: String,
    pub relations: Vec<EvidenceRelationView>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct GitActivityView {
    pub evidence_id: String,
    pub title: String,
    pub activity_type: String,
    pub commit_sha: Option<String>,
    pub message: Option<String>,
    pub author: Option<String>,
    pub date: String,
    pub status_summary: Option<String>,
    pub verification_status: String,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RecommendedNextStepView {
    pub priority: usize,
    pub source_type: String,
    pub source_id: String,
    pub recommendation: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ContextPackView {
    pub project_id: String,
    pub template_type: String,
    pub snapshot_version: Option<i64>,
    pub generated_from_session_ids: Vec<String>,
    pub content_markdown: String,
    pub content_hash: String,
}

#[derive(Clone)]
pub struct McpQueryKernel {
    repository: Arc<dyn McpQueryRepository>,
    renderer: Arc<dyn ContextRenderer>,
}

impl McpQueryKernel {
    pub fn new(
        repository: Arc<dyn McpQueryRepository>,
        renderer: Arc<dyn ContextRenderer>,
    ) -> Self {
        Self {
            repository,
            renderer,
        }
    }

    pub async fn get_project_overview(
        &self,
        project_id: &str,
    ) -> Result<ProjectOverviewView, McpQueryError> {
        let project = self.require_project(project_id).await?;
        Ok(ProjectOverviewView {
            project_id: project.id,
            name: redact_external_text(&project.name),
            description: redact_external_text(&project.description),
            vision: redact_external_text(&project.vision),
            target_user: redact_external_text(&project.target_user),
            tech_stack: redact_external_text(&project.tech_stack),
            lifecycle_stage: project.lifecycle_stage.as_str().into(),
            status: project.status.as_str().into(),
        })
    }

    pub async fn get_project_identity(
        &self,
        project_id: &str,
    ) -> Result<ProjectIdentityView, McpQueryError> {
        self.get_project_overview(project_id).await
    }

    pub async fn get_current_state(
        &self,
        project_id: &str,
    ) -> Result<Option<CurrentStateView>, McpQueryError> {
        self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(snapshot.map(|value| CurrentStateView {
            project_id: value.project_id().into(),
            current_goal: redact_external_text(value.current_goal()),
            status_summary: redact_external_text(value.status_summary()),
            achievements: value
                .achievements()
                .iter()
                .map(|item| redact_external_text(item))
                .collect(),
            blockers: value
                .blockers()
                .iter()
                .map(|item| redact_external_text(item))
                .collect(),
            next_steps: value
                .next_steps()
                .iter()
                .map(|item| redact_external_text(item))
                .collect(),
            snapshot_version: value.version(),
            confirmed_at: value.confirmed_at(),
        }))
    }

    pub async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<CurrentSnapshotView>, McpQueryError> {
        self.get_current_state(project_id).await
    }

    pub async fn get_development_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<RecentSessionView>, McpQueryError> {
        self.require_project(project_id).await?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(sessions
            .into_iter()
            .take(RECENT_SESSION_LIMIT)
            .map(|value| RecentSessionView {
                session_id: value.id().into(),
                status: value.status().as_str().into(),
                goal: redact_external_text(value.goal()),
                outcome: redact_external_text(value.outcome()),
                validation: redact_external_text(value.validation()),
                handoff: redact_external_text(value.handoff()),
                started_at: value.started_at(),
                completed_at: value.completed_at(),
            })
            .collect())
    }

    pub async fn get_recent_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<RecentSessionView>, McpQueryError> {
        self.get_development_history(project_id).await
    }

    pub async fn list_evidence(
        &self,
        project_id: &str,
    ) -> Result<Vec<EvidenceView>, McpQueryError> {
        self.require_project(project_id).await?;
        let evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        Ok(evidences
            .into_iter()
            .map(|value| EvidenceView {
                title: redact_external_text(value.title()),
                evidence_type: value.evidence_type().as_str().into(),
                description: safe_evidence_description(value.evidence_type(), value.description()),
                verification_status: value.verification_status().as_str().into(),
            })
            .collect())
    }

    pub async fn get_project_evidence_summary(
        &self,
        project_id: &str,
    ) -> Result<Vec<EvidenceSummaryView>, McpQueryError> {
        self.require_project(project_id).await?;
        let mut evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        let links = self
            .repository
            .list_evidence_links_for_project(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut links_by_evidence = BTreeMap::<String, Vec<EvidenceRelationView>>::new();

        for link in links {
            links_by_evidence
                .entry(link.evidence_id().into())
                .or_default()
                .push(EvidenceRelationView {
                    target_type: link.target_type().as_str().into(),
                    target_id: link.target_id().into(),
                    relation_type: link.relation_type().as_str().into(),
                });
        }
        evidences.sort_by(|left, right| {
            left.title()
                .to_lowercase()
                .cmp(&right.title().to_lowercase())
                .then_with(|| left.id().cmp(right.id()))
        });

        Ok(evidences
            .into_iter()
            .map(|evidence| EvidenceSummaryView {
                evidence_id: evidence.id().into(),
                title: redact_external_text(evidence.title()),
                evidence_type: evidence.evidence_type().as_str().into(),
                verification_status: evidence.verification_status().as_str().into(),
                relations: links_by_evidence.remove(evidence.id()).unwrap_or_default(),
            })
            .collect())
    }

    pub async fn get_recent_git_activity(
        &self,
        project_id: &str,
    ) -> Result<Vec<GitActivityView>, McpQueryError> {
        self.require_project(project_id).await?;
        let records = self
            .repository
            .list_git_evidence_activity(project_id)
            .await
            .map_err(map_repository_error)?;

        Ok(records
            .into_iter()
            .take(RECENT_GIT_ACTIVITY_LIMIT)
            .map(|record| {
                let description = redact_git_evidence_description(&record.description);
                let commit_sha = record
                    .commit_sha
                    .or_else(|| {
                        extract_labeled_value(
                            &description,
                            if record.evidence_type == EvidenceType::GitCommit {
                                "Commit"
                            } else {
                                "HEAD"
                            },
                        )
                    })
                    .filter(|value| is_commit_sha(value));
                let message = (record.evidence_type == EvidenceType::GitCommit)
                    .then(|| extract_labeled_value(&description, "Message"))
                    .flatten()
                    .map(|value| redact_external_text(&value));
                let author = (record.evidence_type == EvidenceType::GitCommit)
                    .then(|| extract_labeled_value(&description, "Author"))
                    .flatten()
                    .map(|value| redact_external_text(&strip_author_email(&value)));
                let date = extract_labeled_value(
                    &description,
                    if record.evidence_type == EvidenceType::GitCommit {
                        "Date"
                    } else {
                        "Checked At"
                    },
                )
                .unwrap_or_else(|| record.captured_at.to_rfc3339());
                let status_summary = (record.evidence_type == EvidenceType::GitStatus)
                    .then_some(description)
                    .filter(|value| !value.is_empty());

                GitActivityView {
                    evidence_id: record.evidence_id,
                    title: redact_external_text(&record.title),
                    activity_type: record.evidence_type.as_str().into(),
                    commit_sha,
                    message,
                    author,
                    date,
                    status_summary,
                    verification_status: record.verification_status.as_str().into(),
                    captured_at: record.captured_at,
                }
            })
            .collect())
    }

    pub async fn get_recommended_next_steps(
        &self,
        project_id: &str,
    ) -> Result<Vec<RecommendedNextStepView>, McpQueryError> {
        self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let open_session = self
            .repository
            .get_current_session(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut candidates = self
            .repository
            .list_ai_candidates(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        let mut recommendations = Vec::new();

        if let Some(snapshot) = snapshot.as_ref() {
            for blocker in snapshot.blockers() {
                push_recommendation(
                    &mut recommendations,
                    "snapshot_blocker",
                    snapshot.id(),
                    format!(
                        "Resolve confirmed blocker: {}",
                        redact_external_text(blocker)
                    ),
                    "The current Snapshot records this as an active blocker.".into(),
                );
            }
        }

        if let Some(session) = open_session {
            push_recommendation(
                &mut recommendations,
                "open_session",
                session.id(),
                format!(
                    "Continue the open Development Session: {}",
                    redact_external_text(session.goal())
                ),
                "The Project already has an open Session; complete or abandon it before starting another.".into(),
            );
        }

        if let Some(snapshot) = snapshot.as_ref() {
            for next_step in snapshot.next_steps() {
                push_recommendation(
                    &mut recommendations,
                    "snapshot_next_step",
                    snapshot.id(),
                    redact_external_text(next_step),
                    "This next step was explicitly confirmed in the current Snapshot.".into(),
                );
            }
        }

        candidates.sort_by(|left, right| {
            left.title()
                .to_lowercase()
                .cmp(&right.title().to_lowercase())
                .then_with(|| left.id().cmp(right.id()))
        });
        for candidate in candidates
            .into_iter()
            .filter(|candidate| candidate.status() == AiCandidateStatus::Pending)
        {
            push_recommendation(
                &mut recommendations,
                "pending_candidate",
                candidate.id(),
                format!(
                    "Review pending AI Candidate: {}",
                    redact_external_text(candidate.title())
                ),
                "AI Candidates remain suggestions until a user reviews them; no fact is changed automatically.".into(),
            );
        }

        evidences.sort_by(|left, right| {
            evidence_status_rank(left.verification_status())
                .cmp(&evidence_status_rank(right.verification_status()))
                .then_with(|| {
                    left.title()
                        .to_lowercase()
                        .cmp(&right.title().to_lowercase())
                })
                .then_with(|| left.id().cmp(right.id()))
        });
        for evidence in evidences.into_iter().filter(|evidence| {
            matches!(
                evidence.verification_status(),
                VerificationStatus::Invalid | VerificationStatus::Unverified
            )
        }) {
            let (source_type, recommendation, reason) = match evidence.verification_status() {
                VerificationStatus::Invalid => (
                    "invalid_evidence",
                    format!(
                        "Replace or reconfirm invalid Evidence: {}",
                        redact_external_text(evidence.title())
                    ),
                    "This Evidence is explicitly marked invalid.".into(),
                ),
                VerificationStatus::Unverified => (
                    "unverified_evidence",
                    format!(
                        "Verify Evidence: {}",
                        redact_external_text(evidence.title())
                    ),
                    "This Evidence has not yet been verified.".into(),
                ),
                VerificationStatus::Verified => unreachable!(),
            };
            push_recommendation(
                &mut recommendations,
                source_type,
                evidence.id(),
                recommendation,
                reason,
            );
        }

        let mut seen = BTreeSet::new();
        recommendations.retain(|recommendation| {
            seen.insert((
                recommendation.source_type.clone(),
                recommendation.source_id.clone(),
                recommendation.recommendation.clone(),
            ))
        });
        recommendations.truncate(RECOMMENDATION_LIMIT);
        for (index, recommendation) in recommendations.iter_mut().enumerate() {
            recommendation.priority = index + 1;
        }
        Ok(recommendations)
    }

    /// Builds the existing deterministic Context Pack representation in memory.
    /// No ContextExport row is inserted or updated by this read-only query.
    pub async fn build_context_pack(
        &self,
        project_id: &str,
        template_type: &str,
    ) -> Result<ContextPackView, McpQueryError> {
        let template_type = ContextTemplateType::from_str(template_type)?;
        let project = self.require_project(project_id).await?;
        let snapshot = self
            .repository
            .get_current_snapshot(project_id)
            .await
            .map_err(map_repository_error)?;
        let sessions = self
            .repository
            .list_sessions(project_id)
            .await
            .map_err(map_repository_error)?;
        let recent_session = sessions.first();
        let mut evidences = self
            .repository
            .list_evidences(project_id)
            .await
            .map_err(map_repository_error)?;
        evidences.sort_by(|left, right| {
            left.title()
                .to_lowercase()
                .cmp(&right.title().to_lowercase())
                .then_with(|| left.id().cmp(right.id()))
        });

        let session_project_ids = recent_session
            .into_iter()
            .map(|session| session.project_id())
            .collect::<Vec<_>>();
        let evidence_project_ids = evidences
            .iter()
            .map(|evidence| evidence.project_id())
            .collect::<Vec<_>>();
        ContextExport::validate_sources(
            project_id,
            snapshot.as_ref().map(|value| value.project_id()),
            &session_project_ids,
            &evidence_project_ids,
        )?;

        let markdown = self
            .renderer
            .render(ContextRenderInput {
                template_type,
                project: &project,
                snapshot: snapshot.as_ref(),
                recent_session,
                evidences: &evidences,
            })
            .map_err(|error| McpQueryError::Render(error.to_string()))?;
        Ok(ContextPackView {
            project_id: project.id,
            template_type: template_type.as_str().into(),
            snapshot_version: snapshot.as_ref().map(|value| value.version()),
            generated_from_session_ids: recent_session
                .into_iter()
                .map(|value| value.id().into())
                .collect(),
            content_hash: ContextExport::calculate_hash(&markdown),
            content_markdown: markdown,
        })
    }

    async fn require_project(&self, project_id: &str) -> Result<Project, McpQueryError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_repository_error)?
            .ok_or(McpQueryError::WorkspaceNotFound)?;
        self.repository
            .get_project(&workspace.id, project_id)
            .await
            .map_err(map_repository_error)?
            .ok_or(McpQueryError::ProjectNotFound)
    }
}

fn safe_evidence_description(evidence_type: EvidenceType, description: &str) -> String {
    if matches!(
        evidence_type,
        EvidenceType::GitCommit | EvidenceType::GitStatus
    ) {
        redact_git_evidence_description(description)
    } else {
        redact_external_text(description)
    }
}

fn extract_labeled_value(description: &str, label: &str) -> Option<String> {
    let marker = format!("{label}:\n");
    let start = description.find(&marker)? + marker.len();
    let value = description[start..]
        .split("\n\n")
        .next()
        .unwrap_or_default()
        .trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn strip_author_email(author: &str) -> String {
    author
        .rfind(" <")
        .filter(|start| author.ends_with('>') && *start > 0)
        .map(|start| author[..start].trim_end().to_owned())
        .unwrap_or_else(|| author.to_owned())
}

fn is_commit_sha(value: &String) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn evidence_status_rank(status: VerificationStatus) -> u8 {
    match status {
        VerificationStatus::Invalid => 0,
        VerificationStatus::Unverified => 1,
        VerificationStatus::Verified => 2,
    }
}

fn push_recommendation(
    recommendations: &mut Vec<RecommendedNextStepView>,
    source_type: &str,
    source_id: &str,
    recommendation: String,
    reason: String,
) {
    recommendations.push(RecommendedNextStepView {
        priority: 0,
        source_type: source_type.into(),
        source_id: source_id.into(),
        recommendation,
        reason,
    });
}

fn map_repository_error(error: RepositoryError) -> McpQueryError {
    match error {
        RepositoryError::CorruptData(message) => McpQueryError::CorruptData(message),
        other => McpQueryError::Storage(other.to_string()),
    }
}
