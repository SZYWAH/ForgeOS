use std::sync::Arc;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        DomainError, Evidence, EvidenceDetails, EvidenceSourceType, EvidenceType,
        GitHubAnalysisFocus, GitHubAnalysisImportProposal, GitHubAnalysisReadiness,
        GitHubAnalysisReport, GitHubAnalysisRun, GitHubAnalysisStatus, GitHubConnector,
        GitHubImportDraft, GitHubImportStatus, GitHubRecentActivityKind, GitHubReleaseFact,
        GitHubRepositoryFacts, LifecycleStage, Project, ProjectDetails, ProjectStatus,
    },
    ports::{
        GitHubAnalysisImportKernelRepository, GitHubAnalysisImportRepository,
        GitHubAnalysisImportWrite, GitHubAnalysisRepository, GitHubImportRepository,
        ProjectRepository, RepositoryError, WorkspaceRepository,
    },
};

use super::github_import::GitHubImportResult;

#[derive(Debug, Error)]
pub enum GitHubAnalysisImportError {
    #[error(transparent)]
    Validation(#[from] DomainError),
    #[error("create a Workspace before importing a GitHub analysis")]
    WorkspaceNotFound,
    #[error("GitHub analysis run not found")]
    AnalysisNotFound,
    #[error("only ready or partial GitHub analysis reports can be imported")]
    AnalysisNotEligible,
    #[error("the GitHub analysis report is blocked by a safety check")]
    AnalysisBlocked,
    #[error("GitHub import draft not found")]
    DraftNotFound,
    #[error("GitHub analysis import proposal not found")]
    ProposalNotFound,
    #[error("this GitHub repository is already connected to Project {0}")]
    ExistingProject(String),
    #[error("the import draft was rejected and must be explicitly reopened")]
    DraftRejected,
    #[error("the import draft is already imported")]
    DraftImported,
    #[error("the partial analysis must be acknowledged before review")]
    PartialAcknowledgementRequired,
    #[error("the stale analysis must be acknowledged before review")]
    StaleAcknowledgementRequired,
    #[error("GitHub analysis import version conflict")]
    VersionConflict,
    #[error("the imported Project slug already exists")]
    ProjectSlugConflict,
    #[error("stored GitHub analysis import data is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
}

#[derive(Clone, Debug)]
pub struct GitHubAnalysisImportView {
    pub analysis: GitHubAnalysisRun,
    pub draft: Option<GitHubImportDraft>,
    pub proposal: Option<GitHubAnalysisImportProposal>,
    pub stale: bool,
    pub source_changed: bool,
    pub existing_project_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpdateGitHubAnalysisImportInput {
    pub expected_draft_version: i64,
    pub expected_proposal_version: i64,
    pub project: ProjectDetails,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
}

#[derive(Clone, Debug)]
pub struct ReviewGitHubAnalysisImportInput {
    pub expected_draft_version: i64,
    pub expected_proposal_version: i64,
    pub partial_acknowledged: bool,
    pub stale_acknowledged: bool,
}

#[derive(Clone)]
pub struct GitHubAnalysisImportKernel {
    repository: Arc<dyn GitHubAnalysisImportKernelRepository>,
}

impl GitHubAnalysisImportKernel {
    pub fn new(repository: Arc<dyn GitHubAnalysisImportKernelRepository>) -> Self {
        Self { repository }
    }

    pub async fn prepare(
        &self,
        workspace_id: &str,
        analysis_run_id: &str,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let analysis = self.require_analysis(workspace_id, analysis_run_id).await?;
        ensure_eligible(&analysis)?;
        if let Some(project_id) = analysis.imported_project_id.clone() {
            return self.view_for_analysis(analysis, Some(project_id)).await;
        }
        if let Some(project_id) =
            GitHubAnalysisImportRepository::get_project_id_by_github_repository(
                self.repository.as_ref(),
                workspace_id,
                &analysis.github_repository_id,
            )
            .await
            .map_err(map_store)?
        {
            return self.view_for_analysis(analysis, Some(project_id)).await;
        }

        let report = self.validate_saved_report(&analysis).await?;
        let existing = GitHubAnalysisImportRepository::get_import_draft_by_repository(
            self.repository.as_ref(),
            &analysis.github_repository_id,
        )
        .await
        .map_err(map_store)?;
        if let Some(draft) = existing {
            let proposal = GitHubAnalysisImportRepository::get_analysis_import_proposal_by_draft(
                self.repository.as_ref(),
                draft.id(),
            )
            .await
            .map_err(map_store)?;
            if proposal.is_none() && draft.status() != GitHubImportStatus::Imported {
                let proposal = proposal_from_report(&analysis, &report, draft.id().to_owned())?;
                GitHubAnalysisImportRepository::insert_analysis_import(
                    self.repository.as_ref(),
                    &draft,
                    &proposal,
                    false,
                )
                .await
                .map_err(map_store)?;
            }
            return self.view_for_analysis(analysis, None).await;
        }

        let connector = self.ensure_connector().await?;
        let facts = facts_from_report(&analysis, &report)?;
        let draft = GitHubImportDraft::create(
            Uuid::now_v7().to_string(),
            connector.id().to_owned(),
            facts,
            Utc::now(),
        )?;
        let proposal = proposal_from_report(&analysis, &report, draft.id().to_owned())?;
        GitHubAnalysisImportRepository::insert_analysis_import(
            self.repository.as_ref(),
            &draft,
            &proposal,
            true,
        )
        .await
        .map_err(map_store)?;
        self.view_for_analysis(analysis, None).await
    }

    pub async fn get(
        &self,
        workspace_id: &str,
        analysis_run_id: &str,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let analysis = self.require_analysis(workspace_id, analysis_run_id).await?;
        self.view_for_analysis(analysis, None).await
    }

    pub async fn update(
        &self,
        workspace_id: &str,
        draft_id: &str,
        input: UpdateGitHubAnalysisImportInput,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let (mut draft, mut proposal, analysis) =
            self.require_import(workspace_id, draft_id).await?;
        ensure_versions(
            &draft,
            &proposal,
            input.expected_draft_version,
            input.expected_proposal_version,
        )?;
        ensure_editable(&draft)?;
        let now = Utc::now();
        if draft.status() == GitHubImportStatus::Reviewed {
            draft.return_to_draft(now)?;
        }
        proposal.update(
            input.project,
            input.partial_acknowledged,
            input.stale_acknowledged,
            now,
        )?;
        GitHubAnalysisImportRepository::update_analysis_import(
            self.repository.as_ref(),
            &draft,
            input.expected_draft_version,
            &proposal,
            input.expected_proposal_version,
        )
        .await
        .map_err(map_store)?;
        self.view_for_analysis(analysis, None).await
    }

    pub async fn refresh_from_report(
        &self,
        workspace_id: &str,
        analysis_run_id: &str,
        expected_draft_version: i64,
        expected_proposal_version: i64,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let analysis = self.require_analysis(workspace_id, analysis_run_id).await?;
        ensure_eligible(&analysis)?;
        let report = self.validate_saved_report(&analysis).await?;
        let mut draft = GitHubAnalysisImportRepository::get_import_draft_by_repository(
            self.repository.as_ref(),
            &analysis.github_repository_id,
        )
        .await
        .map_err(map_store)?
        .ok_or(GitHubAnalysisImportError::DraftNotFound)?;
        let mut proposal = GitHubAnalysisImportRepository::get_analysis_import_proposal_by_draft(
            self.repository.as_ref(),
            draft.id(),
        )
        .await
        .map_err(map_store)?
        .ok_or(GitHubAnalysisImportError::ProposalNotFound)?;
        ensure_versions(
            &draft,
            &proposal,
            expected_draft_version,
            expected_proposal_version,
        )?;
        ensure_editable(&draft)?;
        let now = Utc::now();
        if matches!(
            draft.status(),
            GitHubImportStatus::Reviewed | GitHubImportStatus::Rejected
        ) {
            draft.return_to_draft(now)?;
        }
        proposal.replace_source(
            analysis.id.clone(),
            analysis.source_commit_sha.clone(),
            analysis
                .content_hash
                .clone()
                .ok_or_else(|| GitHubAnalysisImportError::CorruptData("missing hash".into()))?,
            project_details_from_report(&analysis, &report)?,
            now,
        )?;
        GitHubAnalysisImportRepository::update_analysis_import(
            self.repository.as_ref(),
            &draft,
            expected_draft_version,
            &proposal,
            expected_proposal_version,
        )
        .await
        .map_err(map_store)?;
        self.view_for_analysis(analysis, None).await
    }

    pub async fn review(
        &self,
        workspace_id: &str,
        draft_id: &str,
        input: ReviewGitHubAnalysisImportInput,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let (mut draft, mut proposal, analysis) =
            self.require_import(workspace_id, draft_id).await?;
        ensure_versions(
            &draft,
            &proposal,
            input.expected_draft_version,
            input.expected_proposal_version,
        )?;
        ensure_editable(&draft)?;
        if draft.status() == GitHubImportStatus::Rejected {
            return Err(GitHubAnalysisImportError::DraftRejected);
        }
        let stale = self.is_stale(&analysis).await?;
        if analysis.status == GitHubAnalysisStatus::Partial && !input.partial_acknowledged {
            return Err(GitHubAnalysisImportError::PartialAcknowledgementRequired);
        }
        if stale && !input.stale_acknowledged {
            return Err(GitHubAnalysisImportError::StaleAcknowledgementRequired);
        }
        let now = Utc::now();
        proposal.update(
            proposal.project.clone(),
            input.partial_acknowledged,
            input.stale_acknowledged,
            now,
        )?;
        draft.review(now)?;
        GitHubAnalysisImportRepository::update_analysis_import(
            self.repository.as_ref(),
            &draft,
            input.expected_draft_version,
            &proposal,
            input.expected_proposal_version,
        )
        .await
        .map_err(map_store)?;
        self.view_for_analysis(analysis, None).await
    }

    pub async fn import_reviewed(
        &self,
        workspace_id: &str,
        draft_id: &str,
        expected_draft_version: i64,
        expected_proposal_version: i64,
    ) -> Result<GitHubImportResult, GitHubAnalysisImportError> {
        let (mut draft, proposal, mut analysis) =
            self.require_import(workspace_id, draft_id).await?;
        self.validate_saved_report(&analysis).await?;
        if draft.status() == GitHubImportStatus::Imported {
            let project_id = draft
                .imported_project_id()
                .ok_or_else(|| GitHubAnalysisImportError::CorruptData("missing Project".into()))?;
            let project =
                ProjectRepository::get_project(self.repository.as_ref(), workspace_id, project_id)
                    .await
                    .map_err(map_store)?
                    .ok_or_else(|| {
                        GitHubAnalysisImportError::CorruptData("missing Project".into())
                    })?;
            return Ok(GitHubImportResult {
                draft,
                project,
                evidences: Vec::new(),
            });
        }
        ensure_versions(
            &draft,
            &proposal,
            expected_draft_version,
            expected_proposal_version,
        )?;
        if draft.status() != GitHubImportStatus::Reviewed {
            return Err(GitHubAnalysisImportError::DraftNotFound);
        }
        let stale = self.is_stale(&analysis).await?;
        if analysis.status == GitHubAnalysisStatus::Partial && !proposal.partial_acknowledged {
            return Err(GitHubAnalysisImportError::PartialAcknowledgementRequired);
        }
        if stale && !proposal.stale_acknowledged {
            return Err(GitHubAnalysisImportError::StaleAcknowledgementRequired);
        }
        let now = Utc::now();
        let project = Project::new(
            Uuid::now_v7().to_string(),
            workspace_id.to_owned(),
            proposal.project.clone(),
            now,
        )?;
        let analysis_expected_version = analysis.version;
        analysis.mark_imported(project.id.clone(), now)?;
        draft.mark_imported(project.id.clone(), now)?;
        let evidences = analysis_evidence(&project.id, &analysis, draft.facts(), now)?;
        GitHubAnalysisImportRepository::import_analysis_proposal(
            self.repository.as_ref(),
            GitHubAnalysisImportWrite {
                draft: draft.clone(),
                expected_draft_version,
                proposal,
                expected_proposal_version,
                analysis,
                expected_analysis_version: analysis_expected_version,
                project: project.clone(),
                evidences: evidences.clone(),
            },
        )
        .await
        .map_err(|error| match error {
            RepositoryError::Conflict => GitHubAnalysisImportError::ProjectSlugConflict,
            other => map_store(other),
        })?;
        Ok(GitHubImportResult {
            draft,
            project,
            evidences: evidences.into_iter().map(|item| item.0).collect(),
        })
    }

    async fn require_analysis(
        &self,
        workspace_id: &str,
        analysis_run_id: &str,
    ) -> Result<GitHubAnalysisRun, GitHubAnalysisImportError> {
        let workspace = WorkspaceRepository::get_current_workspace(self.repository.as_ref())
            .await
            .map_err(map_store)?
            .ok_or(GitHubAnalysisImportError::WorkspaceNotFound)?;
        if workspace.id != workspace_id {
            return Err(GitHubAnalysisImportError::WorkspaceNotFound);
        }
        GitHubAnalysisRepository::get_analysis_run(
            self.repository.as_ref(),
            workspace_id,
            analysis_run_id,
        )
        .await
        .map_err(map_store)?
        .ok_or(GitHubAnalysisImportError::AnalysisNotFound)
    }

    async fn require_import(
        &self,
        workspace_id: &str,
        draft_id: &str,
    ) -> Result<
        (
            GitHubImportDraft,
            GitHubAnalysisImportProposal,
            GitHubAnalysisRun,
        ),
        GitHubAnalysisImportError,
    > {
        let proposal = GitHubAnalysisImportRepository::get_analysis_import_proposal_by_draft(
            self.repository.as_ref(),
            draft_id,
        )
        .await
        .map_err(map_store)?
        .ok_or(GitHubAnalysisImportError::ProposalNotFound)?;
        if proposal.workspace_id != workspace_id {
            return Err(GitHubAnalysisImportError::WorkspaceNotFound);
        }
        let draft =
            GitHubImportRepository::get_github_import_draft(self.repository.as_ref(), draft_id)
                .await
                .map_err(map_store)?
                .ok_or(GitHubAnalysisImportError::DraftNotFound)?;
        let analysis = self
            .require_analysis(workspace_id, &proposal.analysis_run_id)
            .await?;
        Ok((draft, proposal, analysis))
    }

    async fn view_for_analysis(
        &self,
        analysis: GitHubAnalysisRun,
        existing_project_id: Option<String>,
    ) -> Result<GitHubAnalysisImportView, GitHubAnalysisImportError> {
        let draft = GitHubAnalysisImportRepository::get_import_draft_by_repository(
            self.repository.as_ref(),
            &analysis.github_repository_id,
        )
        .await
        .map_err(map_store)?;
        let proposal = if let Some(draft) = &draft {
            GitHubAnalysisImportRepository::get_analysis_import_proposal_by_draft(
                self.repository.as_ref(),
                draft.id(),
            )
            .await
            .map_err(map_store)?
        } else {
            None
        };
        let stale = self.is_stale(&analysis).await?;
        let source_changed = proposal
            .as_ref()
            .is_some_and(|value| value.analysis_run_id != analysis.id);
        let existing_project_id = existing_project_id
            .or_else(|| analysis.imported_project_id.clone())
            .or_else(|| {
                draft
                    .as_ref()
                    .and_then(|value| value.imported_project_id().map(str::to_owned))
            });
        Ok(GitHubAnalysisImportView {
            analysis,
            draft,
            proposal,
            stale,
            source_changed,
            existing_project_id,
        })
    }

    async fn is_stale(
        &self,
        analysis: &GitHubAnalysisRun,
    ) -> Result<bool, GitHubAnalysisImportError> {
        GitHubAnalysisRepository::has_newer_repository_head(
            self.repository.as_ref(),
            &analysis.workspace_id,
            &analysis.github_repository_id,
            &analysis.source_commit_sha,
            analysis.created_at,
        )
        .await
        .map_err(map_store)
    }

    async fn validate_saved_report(
        &self,
        analysis: &GitHubAnalysisRun,
    ) -> Result<GitHubAnalysisReport, GitHubAnalysisImportError> {
        let report = parse_report(analysis)?;
        let sources =
            GitHubAnalysisRepository::list_analysis_sources(self.repository.as_ref(), &analysis.id)
                .await
                .map_err(map_store)?;
        super::github_analysis::validate_report_sources(&report, &sources)
            .map_err(|error| GitHubAnalysisImportError::CorruptData(error.to_string()))?;
        Ok(report)
    }

    async fn ensure_connector(&self) -> Result<GitHubConnector, GitHubAnalysisImportError> {
        if let Some(connector) =
            GitHubImportRepository::get_github_connector(self.repository.as_ref())
                .await
                .map_err(map_store)?
        {
            return Ok(connector);
        }
        let connector = GitHubConnector::create(Uuid::now_v7().to_string(), Utc::now())?;
        match GitHubImportRepository::insert_github_connector(self.repository.as_ref(), &connector)
            .await
        {
            Ok(()) => Ok(connector),
            Err(RepositoryError::Conflict) => {
                GitHubImportRepository::get_github_connector(self.repository.as_ref())
                    .await
                    .map_err(map_store)?
                    .ok_or_else(|| {
                        GitHubAnalysisImportError::Storage(
                            "connector creation raced but no connector exists".into(),
                        )
                    })
            }
            Err(error) => Err(map_store(error)),
        }
    }
}

fn ensure_eligible(analysis: &GitHubAnalysisRun) -> Result<(), GitHubAnalysisImportError> {
    if !matches!(
        analysis.status,
        GitHubAnalysisStatus::Ready | GitHubAnalysisStatus::Partial
    ) {
        return Err(GitHubAnalysisImportError::AnalysisNotEligible);
    }
    if analysis.readiness == GitHubAnalysisReadiness::Blocked {
        return Err(GitHubAnalysisImportError::AnalysisBlocked);
    }
    if analysis.report_json.is_none() || analysis.content_hash.is_none() {
        return Err(GitHubAnalysisImportError::CorruptData(
            "analysis report is incomplete".into(),
        ));
    }
    Ok(())
}

fn parse_report(
    analysis: &GitHubAnalysisRun,
) -> Result<GitHubAnalysisReport, GitHubAnalysisImportError> {
    serde_json::from_str(
        analysis
            .report_json
            .as_deref()
            .ok_or_else(|| GitHubAnalysisImportError::CorruptData("missing report".into()))?,
    )
    .map_err(|error| GitHubAnalysisImportError::CorruptData(error.to_string()))
}

fn proposal_from_report(
    analysis: &GitHubAnalysisRun,
    report: &GitHubAnalysisReport,
    draft_id: String,
) -> Result<GitHubAnalysisImportProposal, GitHubAnalysisImportError> {
    GitHubAnalysisImportProposal::create(
        draft_id,
        analysis.workspace_id.clone(),
        analysis.id.clone(),
        analysis.source_commit_sha.clone(),
        analysis
            .content_hash
            .clone()
            .ok_or_else(|| GitHubAnalysisImportError::CorruptData("missing hash".into()))?,
        project_details_from_report(analysis, report)?,
        Utc::now(),
    )
    .map_err(Into::into)
}

fn project_details_from_report(
    analysis: &GitHubAnalysisRun,
    report: &GitHubAnalysisReport,
) -> Result<ProjectDetails, GitHubAnalysisImportError> {
    let description = report
        .overview
        .as_ref()
        .map(|value| truncate(&value.value, 2_000))
        .unwrap_or_default();
    let kind = report
        .repository_kind
        .as_ref()
        .map(|value| value.value.as_str())
        .unwrap_or("unknown");
    let capabilities = report
        .capabilities
        .iter()
        .take(6)
        .map(|value| format!("- {}", value.value.trim()))
        .collect::<Vec<_>>();
    let scope = truncate(
        &if capabilities.is_empty() {
            format!("Repository type: {kind}")
        } else {
            format!("Repository type: {kind}\n\n{}", capabilities.join("\n"))
        },
        20_000,
    );
    let product_stack = if report.product_tech_stack.is_empty() {
        &report.tech_stack
    } else {
        &report.product_tech_stack
    };
    let tech_stack = product_stack
        .iter()
        .map(|value| value.value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    let lifecycle_stage = match analysis.focus_type {
        GitHubAnalysisFocus::Learning | GitHubAnalysisFocus::Reproduction => {
            LifecycleStage::Discovery
        }
        GitHubAnalysisFocus::SecondaryDevelopment => LifecycleStage::Implementation,
        GitHubAnalysisFocus::Overview
        | GitHubAnalysisFocus::Architecture
        | GitHubAnalysisFocus::Custom => LifecycleStage::Definition,
    };
    Ok(ProjectDetails {
        slug: slugify(&analysis.name),
        name: analysis.name.clone(),
        description,
        vision: String::new(),
        target_user: String::new(),
        scope,
        non_goals: String::new(),
        tech_stack: truncate(&tech_stack, 20_000),
        status: ProjectStatus::Active,
        lifecycle_stage,
        priority: 50,
    }
    .validated()?)
}

fn facts_from_report(
    analysis: &GitHubAnalysisRun,
    report: &GitHubAnalysisReport,
) -> Result<GitHubRepositoryFacts, GitHubAnalysisImportError> {
    let project = project_details_from_report(analysis, report)?;
    let latest_release = report
        .recent_activity
        .iter()
        .find(|item| item.kind == GitHubRecentActivityKind::Release)
        .map(|release| GitHubReleaseFact {
            id: release.identifier.clone(),
            tag_name: release.identifier.clone(),
            name: release.title.clone(),
            published_at: release.occurred_at.clone(),
            url: format!("{}/releases", analysis.repository_url),
        });
    Ok(GitHubRepositoryFacts {
        repository_id: analysis.github_repository_id.clone(),
        owner: analysis.owner.clone(),
        name: analysis.name.clone(),
        url: analysis.repository_url.clone(),
        description: project.description,
        homepage: None,
        default_branch: analysis.default_branch.clone(),
        language: None,
        languages: Vec::new(),
        topics: Vec::new(),
        readme_summary: project.scope,
        recent_commits: Vec::new(),
        latest_release,
    })
}

fn analysis_evidence(
    project_id: &str,
    analysis: &GitHubAnalysisRun,
    facts: &GitHubRepositoryFacts,
    now: chrono::DateTime<Utc>,
) -> Result<Vec<(Evidence, String, String)>, DomainError> {
    let mut items = vec![(
        Evidence::create(
            Uuid::now_v7().to_string(),
            project_id.to_owned(),
            EvidenceDetails {
                title: truncate(
                    &format!("GitHub repository {}/{}", facts.owner, facts.name),
                    200,
                ),
                description: format!(
                    "Repository:\n{}/{}\n\nDescription:\n{}\n\nDefault Branch:\n{}\n\nAnalyzed Commit:\n{}",
                    facts.owner,
                    facts.name,
                    facts.description,
                    facts.default_branch,
                    analysis.source_commit_sha
                ),
                evidence_type: EvidenceType::GitHubRepository,
                source_type: EvidenceSourceType::GitHubApi,
                source_uri: facts.url.clone(),
            },
            now,
        )?,
        "repository".into(),
        facts.repository_id.clone(),
    )];
    if let Some(release) = &facts.latest_release {
        items.push((
            Evidence::create(
                Uuid::now_v7().to_string(),
                project_id.to_owned(),
                EvidenceDetails {
                    title: truncate(&format!("GitHub release {}", release.tag_name), 200),
                    description: format!(
                        "Tag:\n{}\n\nName:\n{}\n\nPublished At:\n{}",
                        release.tag_name,
                        release.name,
                        release.published_at.as_deref().unwrap_or("Unknown")
                    ),
                    evidence_type: EvidenceType::GitHubRelease,
                    source_type: EvidenceSourceType::GitHubApi,
                    source_uri: release.url.clone(),
                },
                now,
            )?,
            "release".into(),
            release.id.clone(),
        ));
    }
    Ok(items)
}

fn ensure_versions(
    draft: &GitHubImportDraft,
    proposal: &GitHubAnalysisImportProposal,
    expected_draft: i64,
    expected_proposal: i64,
) -> Result<(), GitHubAnalysisImportError> {
    if draft.version() == expected_draft && proposal.version == expected_proposal {
        Ok(())
    } else {
        Err(GitHubAnalysisImportError::VersionConflict)
    }
}

fn ensure_editable(draft: &GitHubImportDraft) -> Result<(), GitHubAnalysisImportError> {
    if draft.status() == GitHubImportStatus::Imported {
        Err(GitHubAnalysisImportError::DraftImported)
    } else {
        Ok(())
    }
}

fn slugify(value: &str) -> String {
    let mut result = String::new();
    let mut hyphen = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            hyphen = false;
        } else if !hyphen && !result.is_empty() {
            result.push('-');
            hyphen = true;
        }
    }
    let mut result = result
        .trim_matches('-')
        .chars()
        .take(64)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned();
    if result.len() < 2 {
        result = format!("gh-{result}");
    }
    result
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn map_store(error: RepositoryError) -> GitHubAnalysisImportError {
    match error {
        RepositoryError::OptimisticConflict => GitHubAnalysisImportError::VersionConflict,
        RepositoryError::Conflict => GitHubAnalysisImportError::ProjectSlugConflict,
        RepositoryError::CorruptData(message) => GitHubAnalysisImportError::CorruptData(message),
        other => GitHubAnalysisImportError::Storage(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_maps_to_reviewable_lifecycle_without_creating_current_state() {
        let mut analysis = GitHubAnalysisRun::create(
            "run".into(),
            "workspace".into(),
            "repo".into(),
            "owner".into(),
            "sample".into(),
            "https://github.com/owner/sample".into(),
            "main".into(),
            "0123456789abcdef".into(),
            GitHubAnalysisFocus::Learning,
            None,
            crate::domain::GitHubAnalysisMode::Rules,
            None,
            None,
            "github-rules-v2".into(),
            "a".repeat(64),
            Utc::now(),
        )
        .expect("analysis");
        analysis.content_hash = Some("b".repeat(64));
        let report: GitHubAnalysisReport = serde_json::from_value(serde_json::json!({
            "schemaVersion": "github-report-v2",
            "repository": {"owner":"owner","name":"sample","url":"https://github.com/owner/sample","defaultBranch":"main","headCommitSha":"0123456789abcdef"},
            "overview": {"value":"Sample application","confidence":"high","sourceIds":["source"]},
            "capabilities": [],
            "techStack": [],
            "productTechStack": [],
            "repositoryToolingStack": [],
            "architecture": [],
            "entrypoints": [],
            "buildAndRun": [],
            "testStrategy": [],
            "dependencies": null,
            "license": null,
            "risks": [],
            "recommendedReadingOrder": [],
            "reproductionAssessment": {"value":{"difficulty":"unknown","prerequisites":[],"caveats":[]},"confidence":"low","sourceIds":["source"]},
            "extensionPoints": [],
            "recentActivity": [],
            "sectionAvailability": {},
            "userQuestionAnswer": null,
            "completeness": {"readiness":"ready","filesSelected":1,"filesSkipped":0,"treeEntriesSeen":1,"sourceBytes":10},
            "qualityWarnings": []
        }))
        .expect("report");
        let details = project_details_from_report(&analysis, &report).expect("details");
        assert_eq!(details.lifecycle_stage, LifecycleStage::Discovery);
        assert_eq!(details.name, "sample");
        assert_eq!(details.description, "Sample application");
    }
}
