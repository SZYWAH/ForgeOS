use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};

use crate::{
    domain::{
        AiCandidate, AiCandidateStatus, AiCandidateType, AiProvider, AiProviderDetails,
        CodexExecutionRecord, CodexExecutionStatus, CodexTaskProposal, CodexTaskProposalDetails,
        CodexTaskProposalStatus, ContextExport, ContextExportStatus, ContextTemplateType,
        DevelopmentSession, DiffSummary, Evidence, EvidenceDetails, EvidenceLink,
        EvidenceRelationType, EvidenceSourceType, EvidenceTargetType, EvidenceType, GitHubAuthType,
        GitHubCommitFact, GitHubConnector, GitHubImportDraft, GitHubImportStatus,
        GitHubReleaseFact, GitHubRepositoryFacts, LifecycleStage, Project, ProjectDetails,
        ProjectSnapshot, ProjectStatus, Repository, SessionStatus, SnapshotDetails,
        VerificationStatus, WorkingCopy, Workspace,
    },
    ports::{
        AiCandidateRepository, AiProviderRepository, CodexExecutionRepository,
        CodexTaskProposalRepository, ContextExportRepository, EvidenceRepository,
        GitHubImportRepository, McpQueryRepository, ProjectRepository, RepositoryError,
        RepositoryStore, SessionRepository, SnapshotRepository, WorkspaceRepository,
    },
};

#[derive(FromRow)]
struct GitHubConnectorRow {
    id: String,
    auth_type: String,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl GitHubConnectorRow {
    fn into_domain(self) -> Result<GitHubConnector, RepositoryError> {
        GitHubConnector::restore(
            self.id,
            GitHubAuthType::from_str(&self.auth_type).map_err(corrupt)?,
            parse_timestamp("github_connectors.created_at", &self.created_at)?,
            parse_timestamp("github_connectors.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct GitHubImportDraftRow {
    id: String,
    connector_id: String,
    github_repository_id: String,
    owner: String,
    name: String,
    url: String,
    description: String,
    homepage: Option<String>,
    default_branch: String,
    language: Option<String>,
    languages: String,
    topics: String,
    readme_summary: String,
    recent_commits: String,
    latest_release: Option<String>,
    status: String,
    imported_project_id: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl GitHubImportDraftRow {
    fn into_domain(self) -> Result<GitHubImportDraft, RepositoryError> {
        let languages: std::collections::BTreeMap<String, u64> =
            serde_json::from_str(&self.languages).map_err(|error| {
                RepositoryError::CorruptData(format!("github_import_drafts.languages: {error}"))
            })?;
        let topics = serde_json::from_str(&self.topics).map_err(|error| {
            RepositoryError::CorruptData(format!("github_import_drafts.topics: {error}"))
        })?;
        let recent_commits: Vec<GitHubCommitFact> = serde_json::from_str(&self.recent_commits)
            .map_err(|error| {
                RepositoryError::CorruptData(format!(
                    "github_import_drafts.recent_commits: {error}"
                ))
            })?;
        let latest_release: Option<GitHubReleaseFact> = self
            .latest_release
            .map(|value| {
                serde_json::from_str(&value).map_err(|error| {
                    RepositoryError::CorruptData(format!(
                        "github_import_drafts.latest_release: {error}"
                    ))
                })
            })
            .transpose()?;
        GitHubImportDraft::restore(
            self.id,
            self.connector_id,
            GitHubRepositoryFacts {
                repository_id: self.github_repository_id,
                owner: self.owner,
                name: self.name,
                url: self.url,
                description: self.description,
                homepage: self.homepage,
                default_branch: self.default_branch,
                language: self.language,
                languages: languages.into_iter().collect(),
                topics,
                readme_summary: self.readme_summary,
                recent_commits,
                latest_release,
            },
            GitHubImportStatus::from_str(&self.status).map_err(corrupt)?,
            self.imported_project_id,
            parse_timestamp("github_import_drafts.created_at", &self.created_at)?,
            parse_timestamp("github_import_drafts.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const GITHUB_DRAFT_COLUMNS: &str = "id, connector_id, github_repository_id, owner, name, url, description, homepage, default_branch, language, languages, topics, readme_summary, recent_commits, latest_release, status, imported_project_id, created_at, updated_at, version";

#[derive(Clone)]
pub struct SqliteProjectRepository {
    pool: SqlitePool,
}

impl SqliteProjectRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl McpQueryRepository for SqliteProjectRepository {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError> {
        WorkspaceRepository::get_current_workspace(self).await
    }

    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError> {
        ProjectRepository::get_project(self, workspace_id, project_id).await
    }

    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError> {
        SnapshotRepository::get_current_snapshot(self, project_id).await
    }

    async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, RepositoryError> {
        SessionRepository::list_sessions(self, project_id).await
    }

    async fn list_evidences(&self, project_id: &str) -> Result<Vec<Evidence>, RepositoryError> {
        EvidenceRepository::list_evidences(self, project_id).await
    }
}

#[derive(FromRow)]
struct WorkspaceRow {
    id: String,
    name: String,
    timezone: String,
    created_at: String,
    updated_at: String,
}

impl WorkspaceRow {
    fn into_domain(self) -> Result<Workspace, RepositoryError> {
        Workspace::restore(
            self.id,
            self.name,
            self.timezone,
            parse_timestamp("workspaces.created_at", &self.created_at)?,
            parse_timestamp("workspaces.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

#[derive(FromRow)]
struct ProjectRow {
    id: String,
    workspace_id: String,
    slug: String,
    name: String,
    description: String,
    vision: String,
    target_user: String,
    scope: String,
    non_goals: String,
    tech_stack: String,
    status: String,
    lifecycle_stage: String,
    priority: i32,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl ProjectRow {
    fn into_domain(self) -> Result<Project, RepositoryError> {
        let status = ProjectStatus::from_str(&self.status).map_err(corrupt)?;
        let lifecycle_stage = LifecycleStage::from_str(&self.lifecycle_stage).map_err(corrupt)?;
        Project::restore(
            self.id,
            self.workspace_id,
            ProjectDetails {
                slug: self.slug,
                name: self.name,
                description: self.description,
                vision: self.vision,
                target_user: self.target_user,
                scope: self.scope,
                non_goals: self.non_goals,
                tech_stack: self.tech_stack,
                status,
                lifecycle_stage,
                priority: self.priority,
            },
            parse_timestamp("projects.created_at", &self.created_at)?,
            parse_timestamp("projects.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const PROJECT_COLUMNS: &str = "id, workspace_id, slug, name, description, vision, \
    target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
    created_at, updated_at, version";

#[derive(FromRow)]
struct SnapshotRow {
    id: String,
    project_id: String,
    version: i64,
    current_goal: String,
    status_summary: String,
    achievements: String,
    blockers: String,
    next_steps: String,
    confirmed_at: String,
    created_at: String,
    updated_at: String,
}

impl SnapshotRow {
    fn into_domain(self) -> Result<ProjectSnapshot, RepositoryError> {
        ProjectSnapshot::restore(
            self.id,
            self.project_id,
            self.version,
            SnapshotDetails {
                current_goal: self.current_goal,
                status_summary: self.status_summary,
                achievements: parse_list("project_snapshots.achievements", &self.achievements)?,
                blockers: parse_list("project_snapshots.blockers", &self.blockers)?,
                next_steps: parse_list("project_snapshots.next_steps", &self.next_steps)?,
            },
            parse_timestamp("project_snapshots.confirmed_at", &self.confirmed_at)?,
            parse_timestamp("project_snapshots.created_at", &self.created_at)?,
            parse_timestamp("project_snapshots.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

const SNAPSHOT_COLUMNS: &str = "id, project_id, version, current_goal, status_summary, \
    achievements, blockers, next_steps, confirmed_at, created_at, updated_at";

#[derive(FromRow)]
struct SessionRow {
    id: String,
    project_id: String,
    status: String,
    goal: String,
    notes: String,
    outcome: String,
    validation: String,
    handoff: String,
    started_at: String,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl SessionRow {
    fn into_domain(self) -> Result<DevelopmentSession, RepositoryError> {
        DevelopmentSession::restore(
            self.id,
            self.project_id,
            SessionStatus::from_str(&self.status).map_err(corrupt)?,
            self.goal,
            self.notes,
            self.outcome,
            self.validation,
            self.handoff,
            parse_timestamp("development_sessions.started_at", &self.started_at)?,
            self.completed_at
                .as_deref()
                .map(|value| parse_timestamp("development_sessions.completed_at", value))
                .transpose()?,
            parse_timestamp("development_sessions.created_at", &self.created_at)?,
            parse_timestamp("development_sessions.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const SESSION_COLUMNS: &str = "id, project_id, status, goal, notes, outcome, validation, \
    handoff, started_at, completed_at, created_at, updated_at, version";

#[derive(FromRow)]
struct EvidenceRow {
    id: String,
    project_id: String,
    title: String,
    description: String,
    evidence_type: String,
    source_type: String,
    source_uri: String,
    verification_status: String,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl EvidenceRow {
    fn into_domain(self) -> Result<Evidence, RepositoryError> {
        Evidence::restore(
            self.id,
            self.project_id,
            EvidenceDetails {
                title: self.title,
                description: self.description,
                evidence_type: EvidenceType::from_str(&self.evidence_type).map_err(corrupt)?,
                source_type: EvidenceSourceType::from_str(&self.source_type).map_err(corrupt)?,
                source_uri: self.source_uri,
            },
            VerificationStatus::from_str(&self.verification_status).map_err(corrupt)?,
            parse_timestamp("evidences.created_at", &self.created_at)?,
            parse_timestamp("evidences.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const EVIDENCE_COLUMNS: &str = "id, project_id, title, description, type AS evidence_type, \
    source_type, source_uri, verification_status, created_at, updated_at, version";

#[derive(FromRow)]
struct EvidenceLinkRow {
    id: String,
    evidence_id: String,
    target_type: String,
    target_id: String,
    relation_type: String,
}

#[derive(FromRow)]
struct AiProviderRow {
    id: String,
    name: String,
    base_url: String,
    model: String,
    enabled: i64,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl AiProviderRow {
    fn into_domain(self) -> Result<AiProvider, RepositoryError> {
        let enabled = match self.enabled {
            0 => false,
            1 => true,
            value => {
                return Err(RepositoryError::CorruptData(format!(
                    "ai_providers.enabled contains {value}"
                )))
            }
        };
        AiProvider::restore(
            self.id,
            AiProviderDetails {
                name: self.name,
                base_url: self.base_url,
                model: self.model,
            },
            enabled,
            parse_timestamp("ai_providers.created_at", &self.created_at)?,
            parse_timestamp("ai_providers.updated_at", &self.updated_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const AI_PROVIDER_COLUMNS: &str =
    "id, name, base_url, model, enabled, created_at, updated_at, version";

#[derive(FromRow)]
struct ContextExportRow {
    id: String,
    project_id: String,
    template_type: String,
    snapshot_version: Option<i64>,
    generated_from_session_ids: String,
    content_markdown: String,
    content_hash: String,
    status: String,
    created_at: String,
    version: i64,
}

impl ContextExportRow {
    fn into_domain(self) -> Result<ContextExport, RepositoryError> {
        ContextExport::restore(
            self.id,
            self.project_id,
            ContextTemplateType::from_str(&self.template_type).map_err(corrupt)?,
            self.snapshot_version,
            parse_list(
                "context_exports.generated_from_session_ids",
                &self.generated_from_session_ids,
            )?,
            self.content_markdown,
            self.content_hash,
            ContextExportStatus::from_str(&self.status).map_err(corrupt)?,
            parse_timestamp("context_exports.created_at", &self.created_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const CONTEXT_EXPORT_COLUMNS: &str = "id, project_id, template_type, snapshot_version, \
    generated_from_session_ids, content_markdown, content_hash, status, created_at, version";

#[derive(FromRow)]
struct AiCandidateRow {
    id: String,
    project_id: String,
    candidate_type: String,
    source_context_export_id: String,
    title: String,
    content_markdown: String,
    status: String,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl AiCandidateRow {
    fn into_domain(self) -> Result<AiCandidate, RepositoryError> {
        AiCandidate::restore(
            self.id,
            self.project_id,
            AiCandidateType::from_str(&self.candidate_type).map_err(corrupt)?,
            self.source_context_export_id,
            self.title,
            self.content_markdown,
            AiCandidateStatus::from_str(&self.status).map_err(corrupt)?,
            self.version,
            parse_timestamp("ai_candidates.created_at", &self.created_at)?,
            parse_timestamp("ai_candidates.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

const AI_CANDIDATE_COLUMNS: &str = "id, project_id, candidate_type, \
    source_context_export_id, title, content_markdown, status, version, created_at, updated_at";

#[derive(FromRow)]
struct CodexTaskProposalRow {
    id: String,
    project_id: String,
    context_export_id: String,
    title: String,
    description: String,
    goal: String,
    non_goals: String,
    allowed_scope: String,
    forbidden_scope: String,
    acceptance_criteria: String,
    validation_commands: String,
    status: String,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl CodexTaskProposalRow {
    fn into_domain(self) -> Result<CodexTaskProposal, RepositoryError> {
        CodexTaskProposal::restore(
            self.id,
            self.project_id,
            self.context_export_id,
            CodexTaskProposalDetails {
                title: self.title,
                description: self.description,
                goal: self.goal,
                non_goals: parse_list("codex_task_proposals.non_goals", &self.non_goals)?,
                allowed_scope: parse_list(
                    "codex_task_proposals.allowed_scope",
                    &self.allowed_scope,
                )?,
                forbidden_scope: parse_list(
                    "codex_task_proposals.forbidden_scope",
                    &self.forbidden_scope,
                )?,
                acceptance_criteria: parse_list(
                    "codex_task_proposals.acceptance_criteria",
                    &self.acceptance_criteria,
                )?,
                validation_commands: parse_list(
                    "codex_task_proposals.validation_commands",
                    &self.validation_commands,
                )?,
            },
            CodexTaskProposalStatus::from_str(&self.status).map_err(corrupt)?,
            self.version,
            parse_timestamp("codex_task_proposals.created_at", &self.created_at)?,
            parse_timestamp("codex_task_proposals.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

const CODEX_TASK_PROPOSAL_COLUMNS: &str = "id, project_id, context_export_id, title, description, goal, non_goals, allowed_scope, forbidden_scope, acceptance_criteria, validation_commands, status, version, created_at, updated_at";

#[derive(FromRow)]
struct CodexExecutionRow {
    id: String,
    proposal_id: String,
    project_id: String,
    status: String,
    start_time: Option<String>,
    end_time: Option<String>,
    output_summary: String,
    validation_result: String,
    version: i64,
    created_at: String,
    updated_at: String,
}

impl CodexExecutionRow {
    fn into_domain(self) -> Result<CodexExecutionRecord, RepositoryError> {
        CodexExecutionRecord::restore(
            self.id,
            self.proposal_id,
            self.project_id,
            CodexExecutionStatus::from_str(&self.status).map_err(corrupt)?,
            self.start_time
                .as_deref()
                .map(|value| parse_timestamp("codex_execution_records.start_time", value))
                .transpose()?,
            self.end_time
                .as_deref()
                .map(|value| parse_timestamp("codex_execution_records.end_time", value))
                .transpose()?,
            self.output_summary,
            self.validation_result,
            self.version,
            parse_timestamp("codex_execution_records.created_at", &self.created_at)?,
            parse_timestamp("codex_execution_records.updated_at", &self.updated_at)?,
        )
        .map_err(corrupt)
    }
}

const CODEX_EXECUTION_COLUMNS: &str = "id, proposal_id, project_id, status, start_time, end_time, output_summary, validation_result, version, created_at, updated_at";

#[derive(FromRow)]
struct RepositoryRow {
    id: String,
    project_id: String,
    path: String,
    remote_url: Option<String>,
    default_branch: String,
    last_checked_at: String,
    version: i64,
}

impl RepositoryRow {
    fn into_domain(self) -> Result<Repository, RepositoryError> {
        Repository::restore(
            self.id,
            self.project_id,
            self.path,
            self.remote_url,
            self.default_branch,
            parse_timestamp("repositories.last_checked_at", &self.last_checked_at)?,
            self.version,
        )
        .map_err(corrupt)
    }
}

const REPOSITORY_COLUMNS: &str =
    "id, project_id, path, remote_url, default_branch, last_checked_at, version";

#[derive(FromRow)]
struct WorkingCopyRow {
    id: String,
    repository_id: String,
    branch: String,
    head_commit: String,
    is_dirty: i64,
    changed_files: i64,
    staged_files: i64,
    unstaged_files: i64,
    untracked_files: i64,
    insertions: i64,
    deletions: i64,
    checked_at: String,
}

impl WorkingCopyRow {
    fn into_domain(self) -> Result<WorkingCopy, RepositoryError> {
        let is_dirty = match self.is_dirty {
            0 => false,
            1 => true,
            value => {
                return Err(RepositoryError::CorruptData(format!(
                    "working_copies.is_dirty contains {value}"
                )))
            }
        };
        WorkingCopy::restore(
            self.id,
            self.repository_id,
            self.branch,
            self.head_commit,
            is_dirty,
            DiffSummary {
                changed_files: self.changed_files,
                staged_files: self.staged_files,
                unstaged_files: self.unstaged_files,
                untracked_files: self.untracked_files,
                insertions: self.insertions,
                deletions: self.deletions,
            },
            parse_timestamp("working_copies.checked_at", &self.checked_at)?,
        )
        .map_err(corrupt)
    }
}

const WORKING_COPY_COLUMNS: &str = "id, repository_id, branch, head_commit, is_dirty, \
    changed_files, staged_files, unstaged_files, untracked_files, insertions, deletions, checked_at";

impl EvidenceLinkRow {
    fn into_domain(self) -> Result<EvidenceLink, RepositoryError> {
        EvidenceLink::create(
            self.id,
            self.evidence_id,
            EvidenceTargetType::from_str(&self.target_type).map_err(corrupt)?,
            self.target_id,
            EvidenceRelationType::from_str(&self.relation_type).map_err(corrupt)?,
        )
        .map_err(corrupt)
    }
}

#[async_trait]
impl WorkspaceRepository for SqliteProjectRepository {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT id, name, timezone, created_at, updated_at \
             FROM workspaces ORDER BY created_at, id LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?;

        row.map(WorkspaceRow::into_domain).transpose()
    }

    async fn insert_workspace(&self, workspace: &Workspace) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO workspaces (id, name, timezone, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&workspace.id)
        .bind(&workspace.name)
        .bind(&workspace.timezone)
        .bind(timestamp(workspace.created_at))
        .bind(timestamp(workspace.updated_at))
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }
}

#[async_trait]
impl ProjectRepository for SqliteProjectRepository {
    async fn insert_project(&self, project: &Project) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO projects (id, workspace_id, slug, name, description, vision, \
             target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, \
             created_at, updated_at, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&project.id)
        .bind(&project.workspace_id)
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.vision)
        .bind(&project.target_user)
        .bind(&project.scope)
        .bind(&project.non_goals)
        .bind(&project.tech_stack)
        .bind(project.status.as_str())
        .bind(project.lifecycle_stage.as_str())
        .bind(project.priority)
        .bind(timestamp(project.created_at))
        .bind(timestamp(project.updated_at))
        .bind(project.version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, RepositoryError> {
        let query = format!(
            "SELECT {PROJECT_COLUMNS} FROM projects \
             WHERE workspace_id = ? ORDER BY updated_at DESC, name COLLATE NOCASE, id"
        );
        let rows = sqlx::query_as::<_, ProjectRow>(&query)
            .bind(workspace_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(ProjectRow::into_domain).collect()
    }

    async fn get_project(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<Project>, RepositoryError> {
        let query =
            format!("SELECT {PROJECT_COLUMNS} FROM projects WHERE workspace_id = ? AND id = ?");
        let row = sqlx::query_as::<_, ProjectRow>(&query)
            .bind(workspace_id)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(ProjectRow::into_domain).transpose()
    }

    async fn update_project(
        &self,
        project: &Project,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE projects SET slug = ?, name = ?, description = ?, vision = ?, \
             target_user = ?, scope = ?, non_goals = ?, tech_stack = ?, status = ?, \
             lifecycle_stage = ?, priority = ?, updated_at = ?, version = ? \
             WHERE id = ? AND workspace_id = ? AND version = ?",
        )
        .bind(&project.slug)
        .bind(&project.name)
        .bind(&project.description)
        .bind(&project.vision)
        .bind(&project.target_user)
        .bind(&project.scope)
        .bind(&project.non_goals)
        .bind(&project.tech_stack)
        .bind(project.status.as_str())
        .bind(project.lifecycle_stage.as_str())
        .bind(project.priority)
        .bind(timestamp(project.updated_at))
        .bind(project.version)
        .bind(&project.id)
        .bind(&project.workspace_id)
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl SnapshotRepository for SqliteProjectRepository {
    async fn insert_snapshot(&self, snapshot: &ProjectSnapshot) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO project_snapshots (id, project_id, version, current_goal, \
             status_summary, achievements, blockers, next_steps, confirmed_at, created_at, \
             updated_at) SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
             WHERE NOT EXISTS (SELECT 1 FROM projects WHERE id = ? AND status = 'archived')",
        )
        .bind(snapshot.id())
        .bind(snapshot.project_id())
        .bind(snapshot.version())
        .bind(snapshot.current_goal())
        .bind(snapshot.status_summary())
        .bind(encode_list("achievements", snapshot.achievements())?)
        .bind(encode_list("blockers", snapshot.blockers())?)
        .bind(encode_list("next_steps", snapshot.next_steps())?)
        .bind(timestamp(snapshot.confirmed_at()))
        .bind(timestamp(snapshot.created_at()))
        .bind(timestamp(snapshot.updated_at()))
        .bind(snapshot.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError> {
        let query = format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM project_snapshots \
             WHERE project_id = ? ORDER BY version DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, SnapshotRow>(&query)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(SnapshotRow::into_domain).transpose()
    }

    async fn list_snapshot_history(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectSnapshot>, RepositoryError> {
        let query = format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM project_snapshots \
             WHERE project_id = ? ORDER BY version DESC"
        );
        let rows = sqlx::query_as::<_, SnapshotRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(SnapshotRow::into_domain).collect()
    }

    async fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<ProjectSnapshot>, RepositoryError> {
        let query = format!("SELECT {SNAPSHOT_COLUMNS} FROM project_snapshots WHERE id = ?");
        let row = sqlx::query_as::<_, SnapshotRow>(&query)
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(SnapshotRow::into_domain).transpose()
    }
}

#[async_trait]
impl SessionRepository for SqliteProjectRepository {
    async fn insert_session(&self, session: &DevelopmentSession) -> Result<(), RepositoryError> {
        if session.status() != SessionStatus::Open || session.version() != 1 {
            return Err(RepositoryError::CorruptData(
                "insert_session requires a new open Development Session".to_owned(),
            ));
        }

        let result = sqlx::query(
            "INSERT INTO development_sessions (id, project_id, status, goal, notes, outcome, \
             validation, handoff, started_at, completed_at, created_at, updated_at, version) \
             SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
             WHERE NOT EXISTS (SELECT 1 FROM projects WHERE id = ? AND status = 'archived')",
        )
        .bind(session.id())
        .bind(session.project_id())
        .bind(session.status().as_str())
        .bind(session.goal())
        .bind(session.notes())
        .bind(session.outcome())
        .bind(session.validation())
        .bind(session.handoff())
        .bind(timestamp(session.started_at()))
        .bind(session.completed_at().map(timestamp))
        .bind(timestamp(session.created_at()))
        .bind(timestamp(session.updated_at()))
        .bind(session.version())
        .bind(session.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        Ok(())
    }

    async fn get_current_session(
        &self,
        project_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError> {
        let query = format!(
            "SELECT {SESSION_COLUMNS} FROM development_sessions \
             WHERE project_id = ? AND status = 'open' \
             ORDER BY started_at DESC, created_at DESC, id DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, SessionRow>(&query)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(SessionRow::into_domain).transpose()
    }

    async fn list_sessions(
        &self,
        project_id: &str,
    ) -> Result<Vec<DevelopmentSession>, RepositoryError> {
        let query = format!(
            "SELECT {SESSION_COLUMNS} FROM development_sessions \
             WHERE project_id = ? \
             ORDER BY started_at DESC, created_at DESC, id DESC"
        );
        let rows = sqlx::query_as::<_, SessionRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(SessionRow::into_domain).collect()
    }

    async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<DevelopmentSession>, RepositoryError> {
        let query = format!("SELECT {SESSION_COLUMNS} FROM development_sessions WHERE id = ?");
        let row = sqlx::query_as::<_, SessionRow>(&query)
            .bind(session_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(SessionRow::into_domain).transpose()
    }

    async fn update_open_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        if session.status() != SessionStatus::Open || session.completed_at().is_some() {
            return Err(RepositoryError::CorruptData(
                "update_open_session requires an open Development Session".to_owned(),
            ));
        }

        let result = sqlx::query(
            "UPDATE development_sessions SET goal = ?, notes = ?, updated_at = ?, version = ? \
             WHERE id = ? AND project_id = ? AND status = 'open' AND version = ? \
             AND NOT EXISTS (SELECT 1 FROM projects WHERE id = ? AND status = 'archived')",
        )
        .bind(session.goal())
        .bind(session.notes())
        .bind(timestamp(session.updated_at()))
        .bind(session.version())
        .bind(session.id())
        .bind(session.project_id())
        .bind(expected_version)
        .bind(session.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 1 {
            return Ok(());
        }
        if self.project_is_archived(session.project_id()).await? {
            return Err(RepositoryError::ProjectArchived);
        }
        Err(RepositoryError::OptimisticConflict)
    }

    async fn finish_session(
        &self,
        session: &DevelopmentSession,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        if !session.status().is_terminal() || session.completed_at().is_none() {
            return Err(RepositoryError::CorruptData(
                "finish_session requires a terminal Development Session".to_owned(),
            ));
        }

        let result = sqlx::query(
            "UPDATE development_sessions SET status = ?, outcome = ?, validation = ?, \
             handoff = ?, completed_at = ?, updated_at = ?, version = ? \
             WHERE id = ? AND project_id = ? AND status = 'open' AND version = ?",
        )
        .bind(session.status().as_str())
        .bind(session.outcome())
        .bind(session.validation())
        .bind(session.handoff())
        .bind(session.completed_at().map(timestamp))
        .bind(timestamp(session.updated_at()))
        .bind(session.version())
        .bind(session.id())
        .bind(session.project_id())
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl EvidenceRepository for SqliteProjectRepository {
    async fn insert_evidence(&self, evidence: &Evidence) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO evidences (id, project_id, title, description, type, source_type, \
             source_uri, verification_status, created_at, updated_at, version) \
             SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? \
             WHERE EXISTS (SELECT 1 FROM projects WHERE id = ? AND status <> 'archived')",
        )
        .bind(evidence.id())
        .bind(evidence.project_id())
        .bind(evidence.title())
        .bind(evidence.description())
        .bind(evidence.evidence_type().as_str())
        .bind(evidence.source_type().as_str())
        .bind(evidence.source_uri())
        .bind(evidence.verification_status().as_str())
        .bind(timestamp(evidence.created_at()))
        .bind(timestamp(evidence.updated_at()))
        .bind(evidence.version())
        .bind(evidence.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        Ok(())
    }

    async fn list_evidences(&self, project_id: &str) -> Result<Vec<Evidence>, RepositoryError> {
        let query = format!(
            "SELECT {EVIDENCE_COLUMNS} FROM evidences WHERE project_id = ? \
             ORDER BY updated_at DESC, created_at DESC, id DESC"
        );
        let rows = sqlx::query_as::<_, EvidenceRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;

        rows.into_iter().map(EvidenceRow::into_domain).collect()
    }

    async fn get_evidence(&self, evidence_id: &str) -> Result<Option<Evidence>, RepositoryError> {
        let query = format!("SELECT {EVIDENCE_COLUMNS} FROM evidences WHERE id = ?");
        let row = sqlx::query_as::<_, EvidenceRow>(&query)
            .bind(evidence_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;

        row.map(EvidenceRow::into_domain).transpose()
    }

    async fn update_evidence(
        &self,
        evidence: &Evidence,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE evidences SET title = ?, description = ?, type = ?, source_type = ?, \
             source_uri = ?, verification_status = ?, updated_at = ?, version = ? \
             WHERE id = ? AND project_id = ? AND version = ? \
             AND EXISTS (SELECT 1 FROM projects WHERE id = ? AND status <> 'archived')",
        )
        .bind(evidence.title())
        .bind(evidence.description())
        .bind(evidence.evidence_type().as_str())
        .bind(evidence.source_type().as_str())
        .bind(evidence.source_uri())
        .bind(evidence.verification_status().as_str())
        .bind(timestamp(evidence.updated_at()))
        .bind(evidence.version())
        .bind(evidence.id())
        .bind(evidence.project_id())
        .bind(expected_version)
        .bind(evidence.project_id())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;

        if result.rows_affected() == 1 {
            return Ok(());
        }
        if self.project_is_archived(evidence.project_id()).await? {
            return Err(RepositoryError::ProjectArchived);
        }
        Err(RepositoryError::OptimisticConflict)
    }

    async fn insert_evidence_link(&self, link: &EvidenceLink) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "INSERT INTO evidence_links (id, evidence_id, target_type, target_id, relation_type) \
             SELECT ?, ?, ?, ?, ? WHERE EXISTS ( \
                SELECT 1 FROM evidences e JOIN projects p ON p.id = e.project_id \
                WHERE e.id = ? AND p.status <> 'archived' \
             )",
        )
        .bind(link.id())
        .bind(link.evidence_id())
        .bind(link.target_type().as_str())
        .bind(link.target_id())
        .bind(link.relation_type().as_str())
        .bind(link.evidence_id())
        .execute(&self.pool)
        .await
        .map_err(map_evidence_link_write_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        Ok(())
    }

    async fn list_evidence_links(
        &self,
        evidence_id: &str,
    ) -> Result<Vec<EvidenceLink>, RepositoryError> {
        let rows = sqlx::query_as::<_, EvidenceLinkRow>(
            "SELECT id, evidence_id, target_type, target_id, relation_type \
             FROM evidence_links WHERE evidence_id = ? \
             ORDER BY target_type, target_id, relation_type, id",
        )
        .bind(evidence_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?;

        rows.into_iter().map(EvidenceLinkRow::into_domain).collect()
    }
}

#[async_trait]
impl AiProviderRepository for SqliteProjectRepository {
    async fn insert_ai_provider(&self, provider: &AiProvider) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO ai_providers \
             (id, name, base_url, model, enabled, created_at, updated_at, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(provider.id())
        .bind(provider.name())
        .bind(provider.base_url())
        .bind(provider.model())
        .bind(i64::from(provider.enabled()))
        .bind(timestamp(provider.created_at()))
        .bind(timestamp(provider.updated_at()))
        .bind(provider.version())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn list_ai_providers(&self) -> Result<Vec<AiProvider>, RepositoryError> {
        let query = format!(
            "SELECT {AI_PROVIDER_COLUMNS} FROM ai_providers \
             ORDER BY enabled DESC, name COLLATE NOCASE, id"
        );
        let rows = sqlx::query_as::<_, AiProviderRow>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        rows.into_iter().map(AiProviderRow::into_domain).collect()
    }

    async fn get_ai_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<AiProvider>, RepositoryError> {
        let query = format!("SELECT {AI_PROVIDER_COLUMNS} FROM ai_providers WHERE id = ?");
        let row = sqlx::query_as::<_, AiProviderRow>(&query)
            .bind(provider_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        row.map(AiProviderRow::into_domain).transpose()
    }

    async fn update_ai_provider(
        &self,
        provider: &AiProvider,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE ai_providers SET name = ?, base_url = ?, model = ?, enabled = ?, \
             updated_at = ?, version = ? WHERE id = ? AND version = ?",
        )
        .bind(provider.name())
        .bind(provider.base_url())
        .bind(provider.model())
        .bind(i64::from(provider.enabled()))
        .bind(timestamp(provider.updated_at()))
        .bind(provider.version())
        .bind(provider.id())
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl ContextExportRepository for SqliteProjectRepository {
    async fn insert_context_export(&self, export: &ContextExport) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO context_exports (id, project_id, template_type, snapshot_version, \
             generated_from_session_ids, content_markdown, content_hash, status, created_at, version) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(export.id())
        .bind(export.project_id())
        .bind(export.template_type().as_str())
        .bind(export.snapshot_version())
        .bind(encode_list(
            "context_exports.generated_from_session_ids",
            export.generated_from_session_ids(),
        )?)
        .bind(export.content_markdown())
        .bind(export.content_hash())
        .bind(export.status().as_str())
        .bind(timestamp(export.created_at()))
        .bind(export.version())
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn list_context_exports(
        &self,
        project_id: &str,
    ) -> Result<Vec<ContextExport>, RepositoryError> {
        let query = format!(
            "SELECT {CONTEXT_EXPORT_COLUMNS} FROM context_exports WHERE project_id = ? \
             ORDER BY created_at DESC, id DESC"
        );
        let rows = sqlx::query_as::<_, ContextExportRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        rows.into_iter()
            .map(ContextExportRow::into_domain)
            .collect()
    }

    async fn get_context_export(
        &self,
        export_id: &str,
    ) -> Result<Option<ContextExport>, RepositoryError> {
        let query = format!("SELECT {CONTEXT_EXPORT_COLUMNS} FROM context_exports WHERE id = ?");
        let row = sqlx::query_as::<_, ContextExportRow>(&query)
            .bind(export_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        row.map(ContextExportRow::into_domain).transpose()
    }

    async fn update_context_export_status(
        &self,
        export: &ContextExport,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE context_exports SET status = ?, version = ? \
             WHERE id = ? AND project_id = ? AND version = ?",
        )
        .bind(export.status().as_str())
        .bind(export.version())
        .bind(export.id())
        .bind(export.project_id())
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl AiCandidateRepository for SqliteProjectRepository {
    async fn insert_ai_candidate(&self, candidate: &AiCandidate) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO ai_candidates (id, project_id, candidate_type, \
             source_context_export_id, title, content_markdown, status, version, \
             created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(candidate.id())
        .bind(candidate.project_id())
        .bind(candidate.candidate_type().as_str())
        .bind(candidate.source_context_export_id())
        .bind(candidate.title())
        .bind(candidate.content_markdown())
        .bind(candidate.status().as_str())
        .bind(candidate.version())
        .bind(timestamp(candidate.created_at()))
        .bind(timestamp(candidate.updated_at()))
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        Ok(())
    }

    async fn list_ai_candidates(
        &self,
        project_id: &str,
    ) -> Result<Vec<AiCandidate>, RepositoryError> {
        let query = format!(
            "SELECT {AI_CANDIDATE_COLUMNS} FROM ai_candidates WHERE project_id = ? \
             ORDER BY updated_at DESC, created_at DESC, id DESC"
        );
        let rows = sqlx::query_as::<_, AiCandidateRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        rows.into_iter().map(AiCandidateRow::into_domain).collect()
    }

    async fn get_ai_candidate(
        &self,
        candidate_id: &str,
    ) -> Result<Option<AiCandidate>, RepositoryError> {
        let query = format!("SELECT {AI_CANDIDATE_COLUMNS} FROM ai_candidates WHERE id = ?");
        let row = sqlx::query_as::<_, AiCandidateRow>(&query)
            .bind(candidate_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        row.map(AiCandidateRow::into_domain).transpose()
    }

    async fn update_ai_candidate_status(
        &self,
        candidate: &AiCandidate,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query(
            "UPDATE ai_candidates SET status = ?, version = ?, updated_at = ? \
             WHERE id = ? AND project_id = ? AND status = 'pending' AND version = ?",
        )
        .bind(candidate.status().as_str())
        .bind(candidate.version())
        .bind(timestamp(candidate.updated_at()))
        .bind(candidate.id())
        .bind(candidate.project_id())
        .bind(expected_version)
        .execute(&self.pool)
        .await
        .map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }
}

#[async_trait]
impl CodexTaskProposalRepository for SqliteProjectRepository {
    async fn insert_codex_task_proposal(
        &self,
        proposal: &CodexTaskProposal,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let details = proposal.details();
        sqlx::query("INSERT INTO codex_task_proposals (id, project_id, context_export_id, title, description, goal, non_goals, allowed_scope, forbidden_scope, acceptance_criteria, validation_commands, status, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(proposal.id()).bind(proposal.project_id()).bind(proposal.context_export_id())
            .bind(&details.title).bind(&details.description).bind(&details.goal)
            .bind(encode_list("codex_task_proposals.non_goals", &details.non_goals)?).bind(encode_list("codex_task_proposals.allowed_scope", &details.allowed_scope)?)
            .bind(encode_list("codex_task_proposals.forbidden_scope", &details.forbidden_scope)?).bind(encode_list("codex_task_proposals.acceptance_criteria", &details.acceptance_criteria)?)
            .bind(encode_list("codex_task_proposals.validation_commands", &details.validation_commands)?).bind(proposal.status().as_str())
            .bind(proposal.version()).bind(timestamp(proposal.created_at())).bind(timestamp(proposal.updated_at()))
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        sqlx::query("INSERT INTO codex_task_proposal_status_history (id, proposal_id, from_status, to_status, proposal_version, changed_at) VALUES (?, ?, NULL, 'draft', ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string()).bind(proposal.id()).bind(proposal.version()).bind(timestamp(proposal.created_at()))
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        transaction.commit().await.map_err(storage)?;
        Ok(())
    }

    async fn list_codex_task_proposals(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexTaskProposal>, RepositoryError> {
        let query = format!("SELECT {CODEX_TASK_PROPOSAL_COLUMNS} FROM codex_task_proposals WHERE project_id = ? ORDER BY updated_at DESC, created_at DESC, id DESC");
        sqlx::query_as::<_, CodexTaskProposalRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(CodexTaskProposalRow::into_domain)
            .collect()
    }

    async fn get_codex_task_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<Option<CodexTaskProposal>, RepositoryError> {
        let query =
            format!("SELECT {CODEX_TASK_PROPOSAL_COLUMNS} FROM codex_task_proposals WHERE id = ?");
        sqlx::query_as::<_, CodexTaskProposalRow>(&query)
            .bind(proposal_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(CodexTaskProposalRow::into_domain)
            .transpose()
    }

    async fn transition_codex_task_proposal(
        &self,
        proposal: &CodexTaskProposal,
        from_status: &str,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let result = sqlx::query("UPDATE codex_task_proposals SET status = ?, version = ?, updated_at = ? WHERE id = ? AND project_id = ? AND status = ? AND version = ?")
            .bind(proposal.status().as_str()).bind(proposal.version()).bind(timestamp(proposal.updated_at()))
            .bind(proposal.id()).bind(proposal.project_id()).bind(from_status).bind(expected_version)
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        sqlx::query("INSERT INTO codex_task_proposal_status_history (id, proposal_id, from_status, to_status, proposal_version, changed_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string()).bind(proposal.id()).bind(from_status)
            .bind(proposal.status().as_str()).bind(proposal.version()).bind(timestamp(proposal.updated_at()))
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        transaction.commit().await.map_err(storage)?;
        Ok(())
    }
}

#[async_trait]
impl CodexExecutionRepository for SqliteProjectRepository {
    async fn insert_codex_execution(
        &self,
        execution: &CodexExecutionRecord,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        sqlx::query("INSERT INTO codex_execution_records (id, proposal_id, project_id, status, start_time, end_time, output_summary, validation_result, version, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(execution.id())
            .bind(execution.proposal_id())
            .bind(execution.project_id())
            .bind(execution.status().as_str())
            .bind(execution.started_at().map(timestamp))
            .bind(execution.ended_at().map(timestamp))
            .bind(execution.output_summary())
            .bind(execution.validation_result())
            .bind(execution.version())
            .bind(timestamp(execution.created_at()))
            .bind(timestamp(execution.updated_at()))
            .execute(&mut *transaction)
            .await
            .map_err(map_write_error)?;
        sqlx::query("INSERT INTO codex_execution_status_history (id, execution_id, from_status, to_status, execution_version, changed_at) VALUES (?, ?, NULL, 'pending', ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(execution.id())
            .bind(execution.version())
            .bind(timestamp(execution.created_at()))
            .execute(&mut *transaction)
            .await
            .map_err(map_write_error)?;
        transaction.commit().await.map_err(storage)?;
        Ok(())
    }

    async fn list_codex_executions(
        &self,
        project_id: &str,
    ) -> Result<Vec<CodexExecutionRecord>, RepositoryError> {
        let query = format!(
            "SELECT {CODEX_EXECUTION_COLUMNS} FROM codex_execution_records WHERE project_id = ? ORDER BY created_at DESC, id DESC"
        );
        sqlx::query_as::<_, CodexExecutionRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(CodexExecutionRow::into_domain)
            .collect()
    }

    async fn get_codex_execution(
        &self,
        execution_id: &str,
    ) -> Result<Option<CodexExecutionRecord>, RepositoryError> {
        let query =
            format!("SELECT {CODEX_EXECUTION_COLUMNS} FROM codex_execution_records WHERE id = ?");
        sqlx::query_as::<_, CodexExecutionRow>(&query)
            .bind(execution_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(CodexExecutionRow::into_domain)
            .transpose()
    }

    async fn transition_codex_execution(
        &self,
        execution: &CodexExecutionRecord,
        from_status: &str,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let result = sqlx::query("UPDATE codex_execution_records SET status = ?, start_time = ?, end_time = ?, output_summary = ?, validation_result = ?, version = ?, updated_at = ? WHERE id = ? AND project_id = ? AND status = ? AND version = ?")
            .bind(execution.status().as_str())
            .bind(execution.started_at().map(timestamp))
            .bind(execution.ended_at().map(timestamp))
            .bind(execution.output_summary())
            .bind(execution.validation_result())
            .bind(execution.version())
            .bind(timestamp(execution.updated_at()))
            .bind(execution.id())
            .bind(execution.project_id())
            .bind(from_status)
            .bind(expected_version)
            .execute(&mut *transaction)
            .await
            .map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        sqlx::query("INSERT INTO codex_execution_status_history (id, execution_id, from_status, to_status, execution_version, changed_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(uuid::Uuid::now_v7().to_string())
            .bind(execution.id())
            .bind(from_status)
            .bind(execution.status().as_str())
            .bind(execution.version())
            .bind(timestamp(execution.updated_at()))
            .execute(&mut *transaction)
            .await
            .map_err(map_write_error)?;
        transaction.commit().await.map_err(storage)?;
        Ok(())
    }
}

#[async_trait]
impl GitHubImportRepository for SqliteProjectRepository {
    async fn get_github_connector(&self) -> Result<Option<GitHubConnector>, RepositoryError> {
        sqlx::query_as::<_, GitHubConnectorRow>(
            "SELECT id, auth_type, created_at, updated_at, version FROM github_connectors WHERE provider = 'github' LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(GitHubConnectorRow::into_domain)
        .transpose()
    }

    async fn insert_github_connector(
        &self,
        connector: &GitHubConnector,
    ) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO github_connectors (id, provider, auth_type, created_at, updated_at, version) VALUES (?, 'github', ?, ?, ?, ?)")
            .bind(connector.id())
            .bind(connector.auth_type().as_str())
            .bind(timestamp(connector.created_at()))
            .bind(timestamp(connector.updated_at()))
            .bind(connector.version())
            .execute(&self.pool).await.map_err(map_write_error)?;
        Ok(())
    }

    async fn update_github_connector(
        &self,
        connector: &GitHubConnector,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE github_connectors SET auth_type = ?, updated_at = ?, version = ? WHERE id = ? AND version = ?")
            .bind(connector.auth_type().as_str())
            .bind(timestamp(connector.updated_at()))
            .bind(connector.version())
            .bind(connector.id())
            .bind(expected_version)
            .execute(&self.pool).await.map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }

    async fn insert_github_import_draft(
        &self,
        draft: &GitHubImportDraft,
    ) -> Result<(), RepositoryError> {
        let facts = draft.facts();
        let languages = facts
            .languages
            .iter()
            .cloned()
            .collect::<std::collections::BTreeMap<_, _>>();
        sqlx::query("INSERT INTO github_import_drafts (id, connector_id, github_repository_id, owner, name, url, description, homepage, default_branch, language, languages, topics, readme_summary, recent_commits, latest_release, status, imported_project_id, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(draft.id()).bind(draft.connector_id()).bind(&facts.repository_id).bind(&facts.owner).bind(&facts.name)
            .bind(&facts.url).bind(&facts.description).bind(&facts.homepage).bind(&facts.default_branch).bind(&facts.language)
            .bind(serde_json::to_string(&languages).map_err(|error| RepositoryError::Storage(error.to_string()))?)
            .bind(serde_json::to_string(&facts.topics).map_err(|error| RepositoryError::Storage(error.to_string()))?)
            .bind(&facts.readme_summary)
            .bind(serde_json::to_string(&facts.recent_commits).map_err(|error| RepositoryError::Storage(error.to_string()))?)
            .bind(facts.latest_release.as_ref().map(serde_json::to_string).transpose().map_err(|error| RepositoryError::Storage(error.to_string()))?)
            .bind(draft.status().as_str()).bind(draft.imported_project_id()).bind(timestamp(draft.created_at())).bind(timestamp(draft.updated_at())).bind(draft.version())
            .execute(&self.pool).await.map_err(map_write_error)?;
        Ok(())
    }

    async fn list_github_import_drafts(&self) -> Result<Vec<GitHubImportDraft>, RepositoryError> {
        let query = format!("SELECT {GITHUB_DRAFT_COLUMNS} FROM github_import_drafts ORDER BY updated_at DESC, created_at DESC, id DESC");
        sqlx::query_as::<_, GitHubImportDraftRow>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(GitHubImportDraftRow::into_domain)
            .collect()
    }

    async fn get_github_import_draft(
        &self,
        draft_id: &str,
    ) -> Result<Option<GitHubImportDraft>, RepositoryError> {
        let query = format!("SELECT {GITHUB_DRAFT_COLUMNS} FROM github_import_drafts WHERE id = ?");
        sqlx::query_as::<_, GitHubImportDraftRow>(&query)
            .bind(draft_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(GitHubImportDraftRow::into_domain)
            .transpose()
    }

    async fn update_github_import_draft(
        &self,
        draft: &GitHubImportDraft,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE github_import_drafts SET status = ?, imported_project_id = ?, updated_at = ?, version = ? WHERE id = ? AND version = ?")
            .bind(draft.status().as_str()).bind(draft.imported_project_id()).bind(timestamp(draft.updated_at())).bind(draft.version()).bind(draft.id()).bind(expected_version)
            .execute(&self.pool).await.map_err(map_write_error)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }

    async fn import_github_draft(
        &self,
        draft: &GitHubImportDraft,
        expected_version: i64,
        project: &Project,
        evidences: &[(Evidence, String, String)],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        sqlx::query("INSERT INTO projects (id, workspace_id, slug, name, description, vision, target_user, scope, non_goals, tech_stack, status, lifecycle_stage, priority, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&project.id).bind(&project.workspace_id).bind(&project.slug).bind(&project.name).bind(&project.description).bind(&project.vision)
            .bind(&project.target_user).bind(&project.scope).bind(&project.non_goals).bind(&project.tech_stack).bind(project.status.as_str())
            .bind(project.lifecycle_stage.as_str()).bind(project.priority).bind(timestamp(project.created_at)).bind(timestamp(project.updated_at)).bind(project.version)
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        let updated = sqlx::query("UPDATE github_import_drafts SET status = 'imported', imported_project_id = ?, updated_at = ?, version = ? WHERE id = ? AND status = 'reviewed' AND version = ?")
            .bind(&project.id).bind(timestamp(draft.updated_at())).bind(draft.version()).bind(draft.id()).bind(expected_version)
            .execute(&mut *transaction).await.map_err(map_write_error)?;
        if updated.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        for (evidence, object_type, object_id) in evidences {
            sqlx::query("INSERT INTO evidences (id, project_id, title, description, type, source_type, source_uri, verification_status, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(evidence.id()).bind(evidence.project_id()).bind(evidence.title()).bind(evidence.description()).bind(evidence.evidence_type().as_str())
                .bind(evidence.source_type().as_str()).bind(evidence.source_uri()).bind(evidence.verification_status().as_str())
                .bind(timestamp(evidence.created_at())).bind(timestamp(evidence.updated_at())).bind(evidence.version())
                .execute(&mut *transaction).await.map_err(map_write_error)?;
            sqlx::query("INSERT INTO github_evidence_links (evidence_id, draft_id, project_id, github_object_type, github_object_id, captured_at) VALUES (?, ?, ?, ?, ?, ?)")
                .bind(evidence.id()).bind(draft.id()).bind(&project.id).bind(object_type).bind(object_id).bind(timestamp(evidence.created_at()))
                .execute(&mut *transaction).await.map_err(map_write_error)?;
        }
        transaction.commit().await.map_err(storage)
    }

    async fn get_github_import_for_project(
        &self,
        project_id: &str,
    ) -> Result<Option<GitHubImportDraft>, RepositoryError> {
        let query = format!("SELECT {GITHUB_DRAFT_COLUMNS} FROM github_import_drafts WHERE imported_project_id = ? AND status = 'imported' LIMIT 1");
        sqlx::query_as::<_, GitHubImportDraftRow>(&query)
            .bind(project_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(GitHubImportDraftRow::into_domain)
            .transpose()
    }
}

#[async_trait]
impl RepositoryStore for SqliteProjectRepository {
    async fn insert_repository_with_working_copy(
        &self,
        repository: &Repository,
        working_copy: &WorkingCopy,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let inserted = sqlx::query(
            "INSERT INTO repositories \
             (id, project_id, path, remote_url, default_branch, last_checked_at, version) \
             SELECT ?, ?, ?, ?, ?, ?, ? WHERE EXISTS ( \
                SELECT 1 FROM projects WHERE id = ? AND status <> 'archived' \
             )",
        )
        .bind(repository.id())
        .bind(repository.project_id())
        .bind(repository.path())
        .bind(repository.remote_url())
        .bind(repository.default_branch())
        .bind(timestamp(repository.last_checked_at()))
        .bind(repository.version())
        .bind(repository.project_id())
        .execute(&mut *transaction)
        .await
        .map_err(map_write_error)?;
        if inserted.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }

        insert_working_copy(&mut transaction, working_copy).await?;
        transaction.commit().await.map_err(storage)
    }

    async fn list_repositories(
        &self,
        project_id: &str,
    ) -> Result<Vec<Repository>, RepositoryError> {
        let query = format!(
            "SELECT {REPOSITORY_COLUMNS} FROM repositories WHERE project_id = ? \
             ORDER BY last_checked_at DESC, path COLLATE NOCASE, id"
        );
        let rows = sqlx::query_as::<_, RepositoryRow>(&query)
            .bind(project_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        rows.into_iter().map(RepositoryRow::into_domain).collect()
    }

    async fn get_repository(
        &self,
        repository_id: &str,
    ) -> Result<Option<Repository>, RepositoryError> {
        let query = format!("SELECT {REPOSITORY_COLUMNS} FROM repositories WHERE id = ?");
        let row = sqlx::query_as::<_, RepositoryRow>(&query)
            .bind(repository_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        row.map(RepositoryRow::into_domain).transpose()
    }

    async fn get_current_working_copy(
        &self,
        repository_id: &str,
    ) -> Result<Option<WorkingCopy>, RepositoryError> {
        let query = format!(
            "SELECT {WORKING_COPY_COLUMNS} FROM working_copies WHERE repository_id = ? \
             ORDER BY checked_at DESC, id DESC LIMIT 1"
        );
        let row = sqlx::query_as::<_, WorkingCopyRow>(&query)
            .bind(repository_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
        row.map(WorkingCopyRow::into_domain).transpose()
    }

    async fn update_repository_with_working_copy(
        &self,
        repository: &Repository,
        expected_version: i64,
        working_copy: &WorkingCopy,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let updated = sqlx::query(
            "UPDATE repositories SET remote_url = ?, default_branch = ?, \
             last_checked_at = ?, version = ? WHERE id = ? AND project_id = ? \
             AND version = ? AND EXISTS ( \
                SELECT 1 FROM projects WHERE id = ? AND status <> 'archived' \
             )",
        )
        .bind(repository.remote_url())
        .bind(repository.default_branch())
        .bind(timestamp(repository.last_checked_at()))
        .bind(repository.version())
        .bind(repository.id())
        .bind(repository.project_id())
        .bind(expected_version)
        .bind(repository.project_id())
        .execute(&mut *transaction)
        .await
        .map_err(map_write_error)?;
        if updated.rows_affected() != 1 {
            let archived: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM projects WHERE id = ? AND status = 'archived' LIMIT 1",
            )
            .bind(repository.project_id())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?;
            return if archived.is_some() {
                Err(RepositoryError::ProjectArchived)
            } else {
                Err(RepositoryError::OptimisticConflict)
            };
        }
        insert_working_copy(&mut transaction, working_copy).await?;
        transaction.commit().await.map_err(storage)
    }

    async fn insert_git_evidence(
        &self,
        evidence: &Evidence,
        repository_id: &str,
        commit_sha: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        let inserted = sqlx::query(
            "INSERT INTO evidences (id, project_id, title, description, type, source_type, \
             source_uri, verification_status, created_at, updated_at, version) \
             SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ? WHERE EXISTS ( \
                SELECT 1 FROM projects WHERE id = ? AND status <> 'archived' \
             )",
        )
        .bind(evidence.id())
        .bind(evidence.project_id())
        .bind(evidence.title())
        .bind(evidence.description())
        .bind(evidence.evidence_type().as_str())
        .bind(evidence.source_type().as_str())
        .bind(evidence.source_uri())
        .bind(evidence.verification_status().as_str())
        .bind(timestamp(evidence.created_at()))
        .bind(timestamp(evidence.updated_at()))
        .bind(evidence.version())
        .bind(evidence.project_id())
        .execute(&mut *transaction)
        .await
        .map_err(map_write_error)?;
        if inserted.rows_affected() == 0 {
            return Err(RepositoryError::ProjectArchived);
        }
        sqlx::query(
            "INSERT INTO repository_evidence_links \
             (evidence_id, repository_id, project_id, commit_sha, captured_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(evidence.id())
        .bind(repository_id)
        .bind(evidence.project_id())
        .bind(commit_sha)
        .bind(timestamp(evidence.created_at()))
        .execute(&mut *transaction)
        .await
        .map_err(map_write_error)?;
        transaction.commit().await.map_err(storage)
    }

    async fn list_repository_evidence(
        &self,
        repository_id: &str,
    ) -> Result<Vec<Evidence>, RepositoryError> {
        let query = "SELECT e.id, e.project_id, e.title, e.description, e.type AS evidence_type, \
             e.source_type, e.source_uri, e.verification_status, e.created_at, e.updated_at, \
             e.version FROM evidences e \
             JOIN repository_evidence_links rel ON rel.evidence_id = e.id \
             WHERE rel.repository_id = ? \
             ORDER BY rel.captured_at DESC, e.id DESC";
        let rows = sqlx::query_as::<_, EvidenceRow>(query)
            .bind(repository_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        rows.into_iter().map(EvidenceRow::into_domain).collect()
    }
}

async fn insert_working_copy(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    working_copy: &WorkingCopy,
) -> Result<(), RepositoryError> {
    let summary = working_copy.diff_summary();
    sqlx::query(
        "INSERT INTO working_copies (id, repository_id, branch, head_commit, is_dirty, \
         changed_files, staged_files, unstaged_files, untracked_files, insertions, \
         deletions, checked_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(working_copy.id())
    .bind(working_copy.repository_id())
    .bind(working_copy.branch())
    .bind(working_copy.head_commit())
    .bind(i64::from(working_copy.is_dirty()))
    .bind(summary.changed_files)
    .bind(summary.staged_files)
    .bind(summary.unstaged_files)
    .bind(summary.untracked_files)
    .bind(summary.insertions)
    .bind(summary.deletions)
    .bind(timestamp(working_copy.checked_at()))
    .execute(&mut **transaction)
    .await
    .map_err(map_write_error)?;
    Ok(())
}

impl SqliteProjectRepository {
    async fn project_is_archived(&self, project_id: &str) -> Result<bool, RepositoryError> {
        let archived: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM projects WHERE id = ? AND status = 'archived' LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?;
        Ok(archived.is_some())
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_timestamp(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn encode_list(field: &str, values: &[String]) -> Result<String, RepositoryError> {
    serde_json::to_string(values)
        .map_err(|error| RepositoryError::Storage(format!("encode {field}: {error}")))
}

fn parse_list(field: &str, value: &str) -> Result<Vec<String>, RepositoryError> {
    serde_json::from_str(value)
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}

fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}

fn storage(error: sqlx::Error) -> RepositoryError {
    RepositoryError::Storage(error.to_string())
}

fn map_write_error(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(database_error) if database_error.is_unique_violation() => {
            RepositoryError::Conflict
        }
        _ => storage(error),
    }
}

fn map_evidence_link_write_error(error: sqlx::Error) -> RepositoryError {
    if error.to_string().contains("evidence_link_target_invalid") {
        return RepositoryError::EvidenceTargetInvalid;
    }
    map_write_error(error)
}
