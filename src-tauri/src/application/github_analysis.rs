use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    str::FromStr,
    sync::Arc,
};

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    domain::{
        parse_github_repository_url, GitHubAnalysisCompleteness, GitHubAnalysisConfidence,
        GitHubAnalysisFocus, GitHubAnalysisMode, GitHubAnalysisReadiness, GitHubAnalysisReport,
        GitHubAnalysisRepositoryIdentity, GitHubAnalysisRun, GitHubAnalysisSource,
        GitHubAnalysisSourceType, GitHubAnalysisStage, GitHubAnalysisStatus, GitHubAnalysisWarning,
        GitHubArchitectureModule, GitHubDependencySummary, GitHubEntrypoint, GitHubLicenseSummary,
        GitHubRecentActivity, GitHubRecentActivityKind, GitHubRepositoryKind,
        GitHubReproductionAssessment, GitHubSectionAvailability, ReproductionDifficulty,
        SourcedConclusion,
    },
    ports::{
        redact_external_text, ApiKeyStore, GitHubAnalysisPage, GitHubAnalysisRepository,
        GitHubProjectSource, GitHubProjectSourceError, GitHubRateLimit, GitHubTreeEntry,
        RepositoryError,
    },
};

use super::{
    memory_content::{strip_prompt_wrappers, truncate_unicode},
    MemoryContentPolicy, MemoryQualitySeverity,
};

pub const ANALYZER_REVISION: &str = "github-rules-v2";
const MAX_TREE_ENTRIES: usize = 20_000;
const MAX_FILES: usize = 64;
const MAX_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SOURCE_CHARS: usize = 8_000;

#[derive(Debug, Error)]
pub enum GitHubAnalysisError {
    #[error("create a Workspace before analyzing GitHub projects")]
    WorkspaceNotFound,
    #[error("GitHub analysis run not found")]
    RunNotFound,
    #[error("GitHub project analysis currently supports public repositories only")]
    PublicOnly,
    #[error("another GitHub analysis is already running")]
    AlreadyRunning,
    #[error("custom analysis question must contain at most 2,000 characters")]
    InvalidPrompt,
    #[error("GitHub repository URL is invalid")]
    InvalidRepositoryUrl,
    #[error("GitHub analysis version conflict")]
    VersionConflict,
    #[error("GitHub analysis source failed: {0}")]
    Source(#[from] GitHubProjectSourceError),
    #[error("GitHub credential store is unavailable: {0}")]
    SecretStore(String),
    #[error("stored GitHub analysis is invalid: {0}")]
    CorruptData(String),
    #[error("ForgeOS storage is unavailable: {0}")]
    Storage(String),
    #[error("GitHub analysis validation failed: {0}")]
    Validation(String),
}

#[derive(Clone, Debug)]
pub struct StartGitHubAnalysisInput {
    pub workspace_id: String,
    pub repository_url: String,
    pub focus_type: String,
    pub custom_prompt: Option<String>,
    pub analysis_mode: String,
}

#[derive(Clone, Debug)]
pub struct StartGitHubAnalysisResult {
    pub analysis_run_id: String,
    pub reused_existing_run: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GitHubAnalysisStatusView {
    pub run: GitHubAnalysisRun,
    pub stale: bool,
}

#[derive(Clone, Debug)]
pub struct GitHubAnalysisReportView {
    pub status: GitHubAnalysisStatusView,
    pub report: Option<GitHubAnalysisReport>,
    pub content_markdown: Option<String>,
    pub content_hash: Option<String>,
    pub sources: Vec<GitHubAnalysisSource>,
}

#[derive(Clone, Debug)]
pub struct GitHubAnalysisListView {
    pub items: Vec<GitHubAnalysisStatusView>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAnalysisProgress {
    pub analysis_run_id: String,
    pub stage: String,
    pub message: String,
    pub tree_entries_seen: i64,
    pub files_read: i64,
    pub files_skipped: i64,
    pub api_requests_used: i64,
    pub api_requests_remaining: Option<i64>,
    pub ai_invoked: bool,
}

#[derive(Clone)]
pub struct GitHubAnalysisKernel {
    repository: Arc<dyn GitHubAnalysisRepository>,
    source: Arc<dyn GitHubProjectSource>,
    token_store: Arc<dyn ApiKeyStore>,
}

impl GitHubAnalysisKernel {
    pub fn new(
        repository: Arc<dyn GitHubAnalysisRepository>,
        source: Arc<dyn GitHubProjectSource>,
        token_store: Arc<dyn ApiKeyStore>,
    ) -> Self {
        Self {
            repository,
            source,
            token_store,
        }
    }

    pub async fn recover_interrupted_runs(&self) -> Result<u64, GitHubAnalysisError> {
        self.repository
            .recover_interrupted_analyses(Utc::now())
            .await
            .map_err(map_store)
    }

    pub async fn start(
        &self,
        input: StartGitHubAnalysisInput,
    ) -> Result<StartGitHubAnalysisResult, GitHubAnalysisError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_store)?
            .ok_or(GitHubAnalysisError::WorkspaceNotFound)?;
        if workspace.id != input.workspace_id {
            return Err(GitHubAnalysisError::WorkspaceNotFound);
        }
        let focus = GitHubAnalysisFocus::from_str(&input.focus_type)
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        let requested_mode = GitHubAnalysisMode::from_str(&input.analysis_mode)
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        let custom_prompt = input
            .custom_prompt
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if custom_prompt
            .as_ref()
            .is_some_and(|value| value.chars().count() > 2_000)
            || (focus == GitHubAnalysisFocus::Custom && custom_prompt.is_none())
        {
            return Err(GitHubAnalysisError::InvalidPrompt);
        }
        let (owner, name) = parse_github_repository_url(&input.repository_url)
            .map_err(|_| GitHubAnalysisError::InvalidRepositoryUrl)?;
        let token = self.github_token().await?;
        let metadata = self
            .source
            .get_repository(&owner, &name, token.as_deref())
            .await?;
        if metadata.private {
            return Err(GitHubAnalysisError::PublicOnly);
        }
        let head = self
            .source
            .get_default_head(
                &metadata.owner,
                &metadata.name,
                &metadata.default_branch,
                token.as_deref(),
            )
            .await?;
        let effective_mode = GitHubAnalysisMode::Rules;
        let request_hash = request_hash(
            &workspace.id,
            &metadata.repository_id,
            &head.sha,
            focus,
            custom_prompt.as_deref(),
            effective_mode,
        );
        if let Some(existing) = self
            .repository
            .get_analysis_run_by_hash(&workspace.id, &request_hash)
            .await
            .map_err(map_store)?
        {
            return Ok(StartGitHubAnalysisResult {
                analysis_run_id: existing.id,
                reused_existing_run: true,
                warnings: (requested_mode == GitHubAnalysisMode::Ai)
                    .then(|| "github_analysis_ai_unavailable".into())
                    .into_iter()
                    .collect(),
            });
        }
        let mut run = GitHubAnalysisRun::create(
            Uuid::now_v7().to_string(),
            workspace.id,
            metadata.repository_id.clone(),
            metadata.owner.clone(),
            metadata.name.clone(),
            metadata.url.clone(),
            metadata.default_branch.clone(),
            head.sha,
            focus,
            custom_prompt,
            effective_mode,
            ANALYZER_REVISION.into(),
            request_hash,
            Utc::now(),
        )
        .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        run.api_requests_remaining = head.rate_limit.remaining.or(metadata.rate_limit.remaining);
        run.api_rate_limit_reset_at = parse_rate_reset(
            head.rate_limit
                .reset_at
                .as_deref()
                .or(metadata.rate_limit.reset_at.as_deref()),
        );
        match self.repository.insert_analysis_run(&run).await {
            Ok(()) => {}
            Err(RepositoryError::Conflict) => {
                if let Some(existing) = self
                    .repository
                    .get_analysis_run_by_hash(&run.workspace_id, &run.request_hash)
                    .await
                    .map_err(map_store)?
                {
                    return Ok(StartGitHubAnalysisResult {
                        analysis_run_id: existing.id,
                        reused_existing_run: true,
                        warnings: vec![],
                    });
                }
                return Err(GitHubAnalysisError::AlreadyRunning);
            }
            Err(error) => return Err(map_store(error)),
        }
        Ok(StartGitHubAnalysisResult {
            analysis_run_id: run.id,
            reused_existing_run: false,
            warnings: (requested_mode == GitHubAnalysisMode::Ai)
                .then(|| "github_analysis_ai_unavailable".into())
                .into_iter()
                .collect(),
        })
    }

    pub async fn execute<F>(
        &self,
        workspace_id: &str,
        run_id: &str,
        progress: F,
    ) -> Result<GitHubAnalysisStatusView, GitHubAnalysisError>
    where
        F: Fn(GitHubAnalysisProgress) + Send + Sync,
    {
        let mut run = self.require_run(workspace_id, run_id).await?;
        if !matches!(
            run.status,
            GitHubAnalysisStatus::Queued
                | GitHubAnalysisStatus::Collecting
                | GitHubAnalysisStatus::Analyzing
        ) {
            return self.status(workspace_id, run_id).await;
        }
        let result = self.collect_and_analyze(&mut run, &progress).await;
        if let Err(error) = result {
            let expected = run.version;
            let (code, readiness) = error_code(&error);
            run.fail(code.into(), error.to_string(), readiness, Utc::now());
            self.repository
                .update_analysis_run(&run, expected)
                .await
                .map_err(map_store)?;
            progress(progress_from_run(&run, "failed"));
        }
        self.status(workspace_id, run_id).await
    }

    pub async fn status(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<GitHubAnalysisStatusView, GitHubAnalysisError> {
        let run = self.require_run(workspace_id, run_id).await?;
        let stale = self
            .repository
            .has_newer_repository_head(
                workspace_id,
                &run.github_repository_id,
                &run.source_commit_sha,
                run.created_at,
            )
            .await
            .map_err(map_store)?;
        Ok(GitHubAnalysisStatusView { run, stale })
    }

    pub async fn report(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<GitHubAnalysisReportView, GitHubAnalysisError> {
        let status = self.status(workspace_id, run_id).await?;
        let report = status
            .run
            .report_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|error| GitHubAnalysisError::CorruptData(error.to_string()))?;
        let sources = self
            .repository
            .list_analysis_sources(run_id)
            .await
            .map_err(map_store)?;
        Ok(GitHubAnalysisReportView {
            content_markdown: status.run.report_markdown.clone(),
            content_hash: status.run.content_hash.clone(),
            status,
            report,
            sources,
        })
    }

    pub async fn list(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<GitHubAnalysisListView, GitHubAnalysisError> {
        self.require_workspace(workspace_id).await?;
        let GitHubAnalysisPage { items, next_cursor } = self
            .repository
            .list_analysis_runs(workspace_id, cursor, limit)
            .await
            .map_err(map_store)?;
        let mut views = Vec::with_capacity(items.len());
        for run in items {
            let stale = self
                .repository
                .has_newer_repository_head(
                    workspace_id,
                    &run.github_repository_id,
                    &run.source_commit_sha,
                    run.created_at,
                )
                .await
                .map_err(map_store)?;
            views.push(GitHubAnalysisStatusView { run, stale });
        }
        Ok(GitHubAnalysisListView {
            items: views,
            next_cursor,
        })
    }

    pub async fn reanalyze(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<StartGitHubAnalysisResult, GitHubAnalysisError> {
        let run = self.require_run(workspace_id, run_id).await?;
        self.start(StartGitHubAnalysisInput {
            workspace_id: workspace_id.into(),
            repository_url: run.repository_url,
            focus_type: run.focus_type.as_str().into(),
            custom_prompt: run.custom_prompt,
            analysis_mode: "rules".into(),
        })
        .await
    }

    pub async fn archive(
        &self,
        workspace_id: &str,
        run_id: &str,
        expected_version: i64,
    ) -> Result<GitHubAnalysisStatusView, GitHubAnalysisError> {
        let mut run = self.require_run(workspace_id, run_id).await?;
        if run.version != expected_version {
            return Err(GitHubAnalysisError::VersionConflict);
        }
        run.archive(Utc::now())
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        self.repository
            .update_analysis_run(&run, expected_version)
            .await
            .map_err(map_store)?;
        self.status(workspace_id, run_id).await
    }

    async fn collect_and_analyze<F>(
        &self,
        run: &mut GitHubAnalysisRun,
        progress: &F,
    ) -> Result<(), GitHubAnalysisError>
    where
        F: Fn(GitHubAnalysisProgress) + Send + Sync,
    {
        let token = self.github_token().await?;
        self.advance(
            run,
            GitHubAnalysisStatus::Collecting,
            GitHubAnalysisStage::CollectingTree,
            progress,
        )
        .await?;
        let tree = self
            .source
            .get_tree(
                &run.owner,
                &run.name,
                &run.source_commit_sha,
                token.as_deref(),
            )
            .await?;
        run.api_requests_used += 1;
        apply_rate(run, &tree.rate_limit);
        let original_count = tree.entries.len();
        run.tree_entries_seen = original_count.min(MAX_TREE_ENTRIES) as i64;
        run.files_skipped += original_count.saturating_sub(MAX_TREE_ENTRIES) as i64;
        let entries = &tree.entries[..original_count.min(MAX_TREE_ENTRIES)];
        let mut warnings = Vec::new();
        if tree.truncated {
            warnings.push(GitHubAnalysisWarning::TreeTruncated);
        }
        if original_count > MAX_TREE_ENTRIES {
            warnings.push(GitHubAnalysisWarning::BudgetExceeded);
        }
        let dynamic_limit = file_budget(run.api_requests_remaining, token.is_some());
        let candidates = select_sources(entries, dynamic_limit);
        run.files_skipped += entries
            .iter()
            .filter(|entry| entry.kind == "blob")
            .count()
            .saturating_sub(candidates.len()) as i64;
        progress(progress_from_run(run, "tree collected"));

        self.advance(
            run,
            GitHubAnalysisStatus::Collecting,
            GitHubAnalysisStage::ReadingSources,
            progress,
        )
        .await?;
        let mut sources = vec![metadata_source(run), tree_source(run, entries, &candidates)];
        let mut total_bytes = sources
            .iter()
            .map(|value| value.excerpt.len())
            .sum::<usize>();
        let mut blocked = false;
        for candidate in candidates {
            if total_bytes >= MAX_SOURCE_BYTES {
                warnings.push(GitHubAnalysisWarning::BudgetExceeded);
                break;
            }
            match self
                .source
                .get_text_source(
                    &run.owner,
                    &run.name,
                    &candidate.path,
                    &run.source_commit_sha,
                    token.as_deref(),
                )
                .await
            {
                Ok(content) => {
                    run.api_requests_used += 1;
                    apply_rate(run, &content.rate_limit);
                    let (excerpt, unsafe_content, prompt_removed) =
                        GitHubAnalysisContentPolicy::sanitize(&content.content);
                    blocked |= unsafe_content;
                    if prompt_removed {
                        warnings.push(GitHubAnalysisWarning::PromptSanitized);
                    }
                    if total_bytes + excerpt.len() > MAX_SOURCE_BYTES {
                        warnings.push(GitHubAnalysisWarning::BudgetExceeded);
                        break;
                    }
                    total_bytes += excerpt.len();
                    run.files_read += 1;
                    sources.push(make_source(
                        &run.id,
                        candidate.source_type,
                        candidate.path,
                        Some(candidate.sha),
                        excerpt,
                        sources.len() as i64,
                    )?);
                }
                Err(GitHubProjectSourceError::SourceTooLarge) => {
                    run.files_skipped += 1;
                    warnings.push(GitHubAnalysisWarning::BudgetExceeded);
                }
                Err(_) => {
                    run.files_skipped += 1;
                    warnings.push(GitHubAnalysisWarning::SourceUnavailable);
                }
            }
            run.source_bytes = total_bytes as i64;
            progress(progress_from_run(run, "reading selected sources"));
        }

        if run
            .api_requests_remaining
            .is_some_and(|remaining| remaining < 3)
        {
            warnings.push(GitHubAnalysisWarning::BudgetExceeded);
        } else {
            match self
                .source
                .get_supplemental_sources(
                    &run.owner,
                    &run.name,
                    &run.source_commit_sha,
                    token.as_deref(),
                )
                .await
            {
                Ok(supplemental) => {
                    run.api_requests_used += supplemental.requests_used;
                    apply_rate(run, &supplemental.rate_limit);
                    for commit in supplemental.commits {
                        let excerpt = serde_json::json!({
                            "sha": commit.sha.clone(),
                            "subject": truncate_unicode(
                                &redact_external_text(&commit.subject),
                                500,
                            ),
                            "authoredAt": commit.authored_at,
                        })
                        .to_string();
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Commit,
                            commit.sha.clone(),
                            Some(commit.sha),
                            excerpt,
                            sources.len() as i64,
                        )?);
                    }
                    if let Some(release) = supplemental.release {
                        let excerpt = serde_json::json!({
                            "tagName": release.tag_name.clone(),
                            "name": truncate_unicode(
                                &redact_external_text(&release.name),
                                500,
                            ),
                            "publishedAt": release.published_at,
                        })
                        .to_string();
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Release,
                            release.tag_name,
                            None,
                            excerpt,
                            sources.len() as i64,
                        )?);
                    }
                    if let Some(sbom) = supplemental.sbom {
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Sbom,
                            "sbom".into(),
                            None,
                            summarize_sbom_excerpt(&sbom),
                            sources.len() as i64,
                        )?);
                    } else {
                        warnings.push(GitHubAnalysisWarning::SbomMissing);
                    }
                }
                Err(_) => warnings.push(GitHubAnalysisWarning::SourceUnavailable),
            }
        }
        if !sources
            .iter()
            .any(|value| value.source_type == GitHubAnalysisSourceType::Readme)
        {
            warnings.push(GitHubAnalysisWarning::ReadmeMissing);
        }
        if !sources
            .iter()
            .any(|value| value.source_type == GitHubAnalysisSourceType::License)
        {
            warnings.push(GitHubAnalysisWarning::LicenseMissing);
        }
        if sources.len() <= 2 {
            warnings.push(GitHubAnalysisWarning::NoAnalyzableText);
        }
        dedupe_warnings(&mut warnings);
        self.repository
            .replace_analysis_sources(&run.id, &sources)
            .await
            .map_err(map_store)?;

        self.advance(
            run,
            GitHubAnalysisStatus::Analyzing,
            GitHubAnalysisStage::RulesAnalysis,
            progress,
        )
        .await?;
        let report = GitHubRulesAnalyzer::analyze(run, &sources, warnings, blocked)?;
        validate_report_sources(&report, &sources)?;
        let markdown = GitHubAnalysisRenderer::render(&report);
        let hash = sha256(&markdown);
        self.advance(
            run,
            GitHubAnalysisStatus::Analyzing,
            GitHubAnalysisStage::Persisting,
            progress,
        )
        .await?;
        let status = match report.completeness.readiness {
            GitHubAnalysisReadiness::Ready | GitHubAnalysisReadiness::Warning => {
                GitHubAnalysisStatus::Ready
            }
            GitHubAnalysisReadiness::Incomplete | GitHubAnalysisReadiness::Blocked => {
                GitHubAnalysisStatus::Partial
            }
        };
        let expected = run.version;
        run.finish(&report, markdown, hash, status, Utc::now())
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        self.repository
            .update_analysis_run(run, expected)
            .await
            .map_err(map_store)?;
        progress(progress_from_run(run, "analysis completed"));
        Ok(())
    }

    async fn advance<F>(
        &self,
        run: &mut GitHubAnalysisRun,
        status: GitHubAnalysisStatus,
        stage: GitHubAnalysisStage,
        progress: &F,
    ) -> Result<(), GitHubAnalysisError>
    where
        F: Fn(GitHubAnalysisProgress) + Send + Sync,
    {
        let expected = run.version;
        run.set_progress(status, stage, Utc::now())
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
        self.repository
            .update_analysis_run(run, expected)
            .await
            .map_err(map_store)?;
        progress(progress_from_run(run, stage.as_str()));
        Ok(())
    }

    async fn github_token(&self) -> Result<Option<String>, GitHubAnalysisError> {
        let Some(id) = self
            .repository
            .get_github_connector_id()
            .await
            .map_err(map_store)?
        else {
            return Ok(None);
        };
        self.token_store
            .get(&id)
            .await
            .map_err(|error| GitHubAnalysisError::SecretStore(error.to_string()))
    }
    async fn require_workspace(&self, workspace_id: &str) -> Result<(), GitHubAnalysisError> {
        let workspace = self
            .repository
            .get_current_workspace()
            .await
            .map_err(map_store)?
            .ok_or(GitHubAnalysisError::WorkspaceNotFound)?;
        if workspace.id != workspace_id {
            return Err(GitHubAnalysisError::WorkspaceNotFound);
        }
        Ok(())
    }
    async fn require_run(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<GitHubAnalysisRun, GitHubAnalysisError> {
        self.require_workspace(workspace_id).await?;
        self.repository
            .get_analysis_run(workspace_id, run_id)
            .await
            .map_err(map_store)?
            .ok_or(GitHubAnalysisError::RunNotFound)
    }
}

pub struct GitHubAnalysisContentPolicy;
impl GitHubAnalysisContentPolicy {
    pub fn sanitize(value: &str) -> (String, bool, bool) {
        let checked = MemoryContentPolicy::sanitize_fact(value, MAX_SOURCE_CHARS, "github_source");
        let blocked = checked
            .issues
            .iter()
            .any(|issue| issue.severity == MemoryQualitySeverity::Blocking);
        let (without_wrapper, wrapper_removed) = strip_prompt_wrappers(value);
        let redacted = redact_external_text(&without_wrapper);
        let cleaned = redacted
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        (
            truncate_unicode(&cleaned, MAX_SOURCE_CHARS),
            blocked,
            wrapper_removed,
        )
    }
}

#[derive(Clone)]
struct SourceCandidate {
    path: String,
    sha: String,
    source_type: GitHubAnalysisSourceType,
    score: i32,
}

fn select_sources(entries: &[GitHubTreeEntry], limit: usize) -> Vec<SourceCandidate> {
    let mut candidates = entries
        .iter()
        .filter(|entry| entry.kind == "blob" && is_safe_text_candidate(entry))
        .filter_map(|entry| {
            classify_source(entry).map(|(source_type, score)| SourceCandidate {
                path: entry.path.replace('\\', "/"),
                sha: entry.sha.clone(),
                source_type,
                score,
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right.score.cmp(&left.score).then_with(|| {
            left.path
                .to_ascii_lowercase()
                .cmp(&right.path.to_ascii_lowercase())
        })
    });
    let mut selected = Vec::new();
    let mut selected_paths = HashSet::new();

    for candidate in candidates.iter().filter(|candidate| {
        !candidate.path.contains('/')
            && matches!(
                candidate.source_type,
                GitHubAnalysisSourceType::Readme
                    | GitHubAnalysisSourceType::License
                    | GitHubAnalysisSourceType::Manifest
            )
    }) {
        push_candidate(&mut selected, &mut selected_paths, candidate.clone(), limit);
    }

    let mut manifest_roots = HashSet::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.source_type == GitHubAnalysisSourceType::Manifest)
    {
        if selected.len() >= limit {
            break;
        }
        let root = candidate
            .path
            .split('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !root.is_empty() && manifest_roots.insert(root) {
            push_candidate(&mut selected, &mut selected_paths, candidate.clone(), limit);
        }
    }

    for candidate in candidates {
        if selected.len() >= limit {
            break;
        }
        let type_count = selected
            .iter()
            .filter(|selected| selected.source_type == candidate.source_type)
            .count();
        if type_count >= source_type_limit(candidate.source_type) {
            continue;
        }
        if candidate.source_type == GitHubAnalysisSourceType::Manifest {
            let root = candidate
                .path
                .split('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !root.is_empty() && manifest_roots.insert(root) {
                push_candidate(&mut selected, &mut selected_paths, candidate, limit);
                continue;
            }
        }
        push_candidate(&mut selected, &mut selected_paths, candidate, limit);
    }
    selected
}

fn push_candidate(
    selected: &mut Vec<SourceCandidate>,
    selected_paths: &mut HashSet<String>,
    candidate: SourceCandidate,
    limit: usize,
) {
    if selected.len() < limit && selected_paths.insert(candidate.path.clone()) {
        selected.push(candidate);
    }
}

fn source_type_limit(source_type: GitHubAnalysisSourceType) -> usize {
    match source_type {
        GitHubAnalysisSourceType::Readme => 8,
        GitHubAnalysisSourceType::License => 4,
        GitHubAnalysisSourceType::Manifest => 20,
        GitHubAnalysisSourceType::Documentation => 8,
        GitHubAnalysisSourceType::Workflow => 6,
        GitHubAnalysisSourceType::TestConfig => 6,
        GitHubAnalysisSourceType::BuildConfig => 6,
        GitHubAnalysisSourceType::Entrypoint => 6,
        _ => usize::MAX,
    }
}

fn is_safe_text_candidate(entry: &GitHubTreeEntry) -> bool {
    if entry.size.is_some_and(|size| size > 128 * 1024) {
        return false;
    }
    let lower = entry.path.to_ascii_lowercase();
    let parts = lower.split('/').collect::<Vec<_>>();
    if parts.iter().any(|part| {
        matches!(
            *part,
            ".git" | "node_modules" | "vendor" | "target" | "dist" | "build" | ".venv" | "coverage"
        )
    }) {
        return false;
    }
    if lower.ends_with(".min.js")
        || lower.ends_with(".min.css")
        || lower.ends_with(".map")
        || lower.contains(".env")
        || lower.contains("secret")
        || lower.contains("credential")
    {
        return false;
    }
    let excluded = [
        ".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".pdf", ".zip", ".7z", ".tar", ".gz",
        ".mp4", ".mp3", ".woff", ".ttf", ".onnx", ".pt", ".bin", ".exe", ".dll", ".lock",
    ];
    !excluded.iter().any(|extension| lower.ends_with(extension))
}

fn classify_source(entry: &GitHubTreeEntry) -> Option<(GitHubAnalysisSourceType, i32)> {
    let path = entry.path.replace('\\', "/");
    let lower = path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    let depth = lower.matches('/').count() as i32;
    if file.starts_with("readme") {
        return Some((GitHubAnalysisSourceType::Readme, 1000 - depth * 10));
    }
    if file.starts_with("license") || file == "copying" {
        return Some((GitHubAnalysisSourceType::License, 960 - depth * 10));
    }
    if is_manifest(file) {
        return Some((GitHubAnalysisSourceType::Manifest, 920 - depth * 12));
    }
    if lower == ".github/workflows" || lower.starts_with(".github/workflows/") {
        return Some((GitHubAnalysisSourceType::Workflow, 760 - depth));
    }
    if lower.starts_with("docs/") || file.ends_with(".md") || file.ends_with(".rst") {
        return Some((GitHubAnalysisSourceType::Documentation, 720 - depth));
    }
    if file.contains("test")
        || file.contains("spec")
        || matches!(file, "pytest.ini" | "vitest.config.ts" | "jest.config.js")
    {
        return Some((GitHubAnalysisSourceType::TestConfig, 680 - depth));
    }
    if matches!(
        file,
        "dockerfile"
            | "makefile"
            | "cmakelists.txt"
            | "build.rs"
            | "vite.config.ts"
            | "tauri.conf.json"
    ) {
        return Some((GitHubAnalysisSourceType::BuildConfig, 660 - depth));
    }
    if is_entrypoint(&lower, file) {
        return Some((GitHubAnalysisSourceType::Entrypoint, 620 - depth));
    }
    None
}

fn is_manifest(file: &str) -> bool {
    matches!(
        file,
        "package.json"
            | "cargo.toml"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "composer.json"
            | "gemfile"
            | "pnpm-workspace.yaml"
            | "lerna.json"
            | "settings.gradle"
            | "settings.gradle.kts"
    )
}
fn is_entrypoint(path: &str, file: &str) -> bool {
    matches!(
        file,
        "main.rs"
            | "main.py"
            | "app.py"
            | "index.ts"
            | "index.tsx"
            | "index.js"
            | "main.ts"
            | "main.tsx"
            | "lib.rs"
    ) || path.ends_with("/main.go")
}

fn metadata_source(run: &GitHubAnalysisRun) -> GitHubAnalysisSource {
    let value = serde_json::json!({"owner": run.owner, "name": run.name, "url": run.repository_url, "defaultBranch": run.default_branch, "head": run.source_commit_sha}).to_string();
    make_source(
        &run.id,
        GitHubAnalysisSourceType::Metadata,
        String::new(),
        Some(run.source_commit_sha.clone()),
        value,
        0,
    )
    .expect("metadata source is valid")
}
fn tree_source(
    run: &GitHubAnalysisRun,
    entries: &[GitHubTreeEntry],
    selected: &[SourceCandidate],
) -> GitHubAnalysisSource {
    let selected_paths = selected
        .iter()
        .map(|value| value.path.as_str())
        .collect::<BTreeSet<_>>();
    let excerpt = entries
        .iter()
        .take(MAX_TREE_ENTRIES)
        .map(|entry| {
            format!(
                "{} {}{}",
                entry.kind,
                entry.path.replace('\\', "/"),
                if selected_paths.contains(entry.path.replace('\\', "/").as_str()) {
                    " [selected]"
                } else {
                    ""
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    make_source(
        &run.id,
        GitHubAnalysisSourceType::Tree,
        String::new(),
        Some(run.source_commit_sha.clone()),
        truncate_unicode(&excerpt, MAX_SOURCE_CHARS),
        1,
    )
    .expect("tree source is valid")
}
fn make_source(
    run_id: &str,
    source_type: GitHubAnalysisSourceType,
    path: String,
    sha: Option<String>,
    excerpt: String,
    order: i64,
) -> Result<GitHubAnalysisSource, GitHubAnalysisError> {
    let source = GitHubAnalysisSource {
        id: Uuid::now_v7().to_string(),
        analysis_run_id: run_id.into(),
        source_type,
        source_path: path.replace('\\', "/"),
        source_sha: sha,
        content_hash: sha256(&excerpt),
        excerpt,
        source_order: order,
    };
    source
        .validate()
        .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
    Ok(source)
}

struct GitHubRulesAnalyzer;
impl GitHubRulesAnalyzer {
    fn analyze(
        run: &GitHubAnalysisRun,
        sources: &[GitHubAnalysisSource],
        mut warnings: Vec<GitHubAnalysisWarning>,
        blocked: bool,
    ) -> Result<GitHubAnalysisReport, GitHubAnalysisError> {
        let by_type = |kind| {
            sources
                .iter()
                .filter(move |source| source.source_type == kind)
                .collect::<Vec<_>>()
        };
        let metadata = sources
            .iter()
            .find(|source| source.source_type == GitHubAnalysisSourceType::Metadata)
            .ok_or_else(|| GitHubAnalysisError::Validation("metadata source missing".into()))?;
        let readme = by_type(GitHubAnalysisSourceType::Readme).into_iter().next();
        let manifests = by_type(GitHubAnalysisSourceType::Manifest);
        let tree = by_type(GitHubAnalysisSourceType::Tree).into_iter().next();
        let repository_kind = classify_repository(run, readme, &manifests, tree.as_ref().copied());
        let kind = repository_kind.value;
        let source_like = matches!(
            kind,
            GitHubRepositoryKind::SourceApplication
                | GitHubRepositoryKind::Library
                | GitHubRepositoryKind::Monorepo
        );
        let overview = repository_overview(readme, &repository_kind);
        let capabilities = capability_conclusions(readme, tree.as_ref().copied(), kind);
        let mut product_tech_stack = if source_like {
            detect_tech_stack(&manifests, tree.as_ref().copied())
        } else {
            vec![]
        };
        sort_and_dedupe_conclusions(&mut product_tech_stack);
        let mut repository_tooling_stack = detect_repository_tooling(
            &manifests,
            &by_type(GitHubAnalysisSourceType::Workflow),
            &by_type(GitHubAnalysisSourceType::BuildConfig),
        );
        sort_and_dedupe_conclusions(&mut repository_tooling_stack);
        let architecture = if source_like {
            architecture_conclusions(tree.as_ref().copied())
        } else {
            vec![]
        };
        let entrypoints: Vec<SourcedConclusion<GitHubEntrypoint>> = if source_like {
            by_type(GitHubAnalysisSourceType::Entrypoint)
                .into_iter()
                .take(10)
                .map(|source| {
                    sourced(
                        GitHubEntrypoint {
                            path: source.source_path.clone(),
                            purpose: entrypoint_purpose(&source.source_path),
                        },
                        GitHubAnalysisConfidence::High,
                        source,
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let build_and_run = if source_like {
            build_rules(&manifests, &by_type(GitHubAnalysisSourceType::BuildConfig))
        } else {
            vec![]
        };
        let test_strategy = if source_like {
            test_rules(
                &manifests,
                &by_type(GitHubAnalysisSourceType::TestConfig),
                &by_type(GitHubAnalysisSourceType::Workflow),
            )
        } else {
            vec![]
        };
        let dependencies = dependency_summary(
            &manifests,
            by_type(GitHubAnalysisSourceType::Sbom).first().copied(),
        );
        let license_source = by_type(GitHubAnalysisSourceType::License)
            .into_iter()
            .next();
        let license = license_source.map(|source| {
            sourced(
                GitHubLicenseSummary {
                    identifier: detect_license(&source.excerpt),
                    reuse_guidance:
                        "静态分析判断：二开前请核对仓库中的完整 License 文本和依赖许可证。".into(),
                },
                GitHubAnalysisConfidence::High,
                source,
            )
        });
        let risks = if source_like {
            risk_rules(&manifests, tree.as_ref().copied())
        } else {
            vec![]
        };
        let recommended_reading_order = reading_order(
            readme,
            &manifests,
            &by_type(GitHubAnalysisSourceType::Entrypoint),
            tree.as_ref().copied(),
        );
        let reproduction_assessment = reproduction(
            build_and_run.is_empty(),
            &manifests,
            metadata,
            kind,
            &repository_kind,
        );
        let extension_points = if source_like {
            extension_rules(&architecture, &entrypoints)
        } else {
            vec![]
        };
        let recent_activity = recent_activity(sources);
        let user_question_answer = run
            .custom_prompt
            .as_deref()
            .and_then(|question| answer_question(question, sources));
        if blocked {
            warnings.push(GitHubAnalysisWarning::PromptSanitized);
        }
        dedupe_warnings(&mut warnings);
        let has_coverage_warning = warnings.iter().any(|warning| {
            matches!(
                warning,
                GitHubAnalysisWarning::TreeTruncated
                    | GitHubAnalysisWarning::BudgetExceeded
                    | GitHubAnalysisWarning::SourceUnavailable
                    | GitHubAnalysisWarning::NoAnalyzableText
            )
        }) || (warnings.contains(&GitHubAnalysisWarning::ReadmeMissing)
            && overview.is_none()
            && manifests.is_empty()
            && entrypoints.is_empty());
        let readiness = if blocked {
            GitHubAnalysisReadiness::Blocked
        } else if has_coverage_warning {
            GitHubAnalysisReadiness::Incomplete
        } else if warnings.is_empty() {
            GitHubAnalysisReadiness::Ready
        } else {
            GitHubAnalysisReadiness::Warning
        };
        let section_availability = section_availability(
            kind,
            &warnings,
            overview.is_some(),
            !capabilities.is_empty(),
            !product_tech_stack.is_empty(),
            !repository_tooling_stack.is_empty(),
            !architecture.is_empty(),
            !entrypoints.is_empty(),
            !build_and_run.is_empty(),
            !test_strategy.is_empty(),
            dependencies.is_some(),
            license.is_some(),
            !recent_activity.is_empty(),
            !risks.is_empty(),
            !extension_points.is_empty(),
            !recommended_reading_order.is_empty(),
            true,
            run.custom_prompt
                .as_deref()
                .map(|_| user_question_answer.is_some()),
        );
        Ok(GitHubAnalysisReport {
            schema_version: Some("github-report-v2".into()),
            repository: GitHubAnalysisRepositoryIdentity {
                owner: run.owner.clone(),
                name: run.name.clone(),
                url: run.repository_url.clone(),
                default_branch: run.default_branch.clone(),
                head_commit_sha: run.source_commit_sha.clone(),
            },
            repository_kind: Some(repository_kind),
            overview,
            capabilities,
            tech_stack: product_tech_stack.clone(),
            product_tech_stack,
            repository_tooling_stack,
            architecture,
            entrypoints,
            build_and_run,
            test_strategy,
            dependencies,
            license,
            risks,
            recommended_reading_order,
            reproduction_assessment,
            extension_points,
            recent_activity,
            section_availability,
            user_question_answer,
            completeness: GitHubAnalysisCompleteness {
                readiness,
                files_selected: run.files_read as usize,
                files_skipped: run.files_skipped as usize,
                tree_entries_seen: run.tree_entries_seen as usize,
                source_bytes: run.source_bytes as usize,
            },
            quality_warnings: warnings,
        })
    }
}

fn sourced<T>(
    value: T,
    confidence: GitHubAnalysisConfidence,
    source: &GitHubAnalysisSource,
) -> SourcedConclusion<T> {
    SourcedConclusion {
        value,
        confidence,
        source_ids: vec![source.id.clone()],
    }
}

fn classify_repository(
    run: &GitHubAnalysisRun,
    readme: Option<&GitHubAnalysisSource>,
    manifests: &[&GitHubAnalysisSource],
    tree: Option<&GitHubAnalysisSource>,
) -> SourcedConclusion<GitHubRepositoryKind> {
    let fallback = readme
        .or(tree)
        .expect("metadata and tree sources are always available");
    let corpus = format!(
        "{}\n{}\n{}",
        run.name.to_ascii_lowercase(),
        readme
            .map_or("", |source| source.excerpt.as_str())
            .to_ascii_lowercase(),
        tree.map_or("", |source| source.excerpt.as_str())
            .to_ascii_lowercase(),
    );
    let top_level_manifests = manifests
        .iter()
        .filter_map(|source| source.source_path.split_once('/').map(|(root, _)| root))
        .collect::<BTreeSet<_>>();
    let workspace_manifest = manifests.iter().copied().find(|source| {
        let path = source.source_path.to_ascii_lowercase();
        let excerpt = source.excerpt.to_ascii_lowercase();
        excerpt.contains("[workspace]")
            || excerpt.contains("\"workspaces\"")
            || path.ends_with("pnpm-workspace.yaml")
            || path.ends_with("lerna.json")
            || path.ends_with("settings.gradle")
            || path.ends_with("settings.gradle.kts")
    });
    let (value, confidence, source) = if let Some(workspace) = workspace_manifest {
        (
            GitHubRepositoryKind::Monorepo,
            GitHubAnalysisConfidence::High,
            workspace,
        )
    } else if top_level_manifests.len() >= 2 || corpus.contains("workspace") && manifests.len() >= 3
    {
        (
            GitHubRepositoryKind::Monorepo,
            GitHubAnalysisConfidence::High,
            tree.unwrap_or(fallback),
        )
    } else if run.name.to_ascii_lowercase().contains("releases")
        || corpus.contains("community-plugins.json")
        || corpus.contains("community-css-themes.json")
        || corpus.contains("release registry")
        || corpus.contains("community plugin list")
    {
        (
            GitHubRepositoryKind::ReleaseRegistry,
            GitHubAnalysisConfidence::High,
            tree.unwrap_or(fallback),
        )
    } else if manifests.is_empty()
        && (run.name.to_ascii_lowercase().contains("docs")
            || corpus.contains("documentation repository")
            || corpus.contains("documentation site"))
    {
        (
            GitHubRepositoryKind::Documentation,
            GitHubAnalysisConfidence::Medium,
            fallback,
        )
    } else if corpus.contains("starter template")
        || corpus.contains("project template")
        || run.name.to_ascii_lowercase().contains("template")
    {
        (
            GitHubRepositoryKind::Template,
            GitHubAnalysisConfidence::Medium,
            fallback,
        )
    } else if !manifests.is_empty()
        && (corpus.contains(" library")
            || corpus.contains(" sdk")
            || corpus.contains(" crate")
            || corpus.contains("package for"))
    {
        (
            GitHubRepositoryKind::Library,
            GitHubAnalysisConfidence::Medium,
            readme.unwrap_or(fallback),
        )
    } else if !manifests.is_empty() || corpus.contains("src/main.") || corpus.contains("src-tauri")
    {
        (
            GitHubRepositoryKind::SourceApplication,
            GitHubAnalysisConfidence::Medium,
            readme.unwrap_or(fallback),
        )
    } else {
        (
            GitHubRepositoryKind::Unknown,
            GitHubAnalysisConfidence::Low,
            fallback,
        )
    };
    sourced(value, confidence, source)
}

fn repository_overview(
    readme: Option<&GitHubAnalysisSource>,
    repository_kind: &SourcedConclusion<GitHubRepositoryKind>,
) -> Option<SourcedConclusion<String>> {
    let paragraph = readme
        .map(|source| first_meaningful_paragraph(&source.excerpt))
        .filter(|value| !value.is_empty());
    let value = match repository_kind.value {
        GitHubRepositoryKind::ReleaseRegistry => format!(
            "该仓库主要保存发布、版本或社区目录元数据，不是产品应用源码仓库。{}",
            paragraph
                .map(|value| format!(" {value}"))
                .unwrap_or_default()
        ),
        _ => paragraph?,
    };
    Some(SourcedConclusion {
        value: truncate_unicode(&value, 800),
        confidence: repository_kind.confidence,
        source_ids: repository_kind.source_ids.clone(),
    })
}

fn capability_conclusions(
    readme: Option<&GitHubAnalysisSource>,
    tree: Option<&GitHubAnalysisSource>,
    kind: GitHubRepositoryKind,
) -> Vec<SourcedConclusion<String>> {
    if kind == GitHubRepositoryKind::ReleaseRegistry {
        let Some(source) = tree.or(readme) else {
            return vec![];
        };
        let corpus = source.excerpt.to_ascii_lowercase();
        let mut values = Vec::new();
        if corpus.contains("community-plugins") {
            values.push("维护社区插件目录和插件发布元数据。".to_owned());
        }
        if corpus.contains("community-css-themes") {
            values.push("维护社区主题目录和主题发布元数据。".to_owned());
        }
        if corpus.contains("versions.json") || corpus.contains("releases") {
            values.push("维护可公开读取的版本与发布信息。".to_owned());
        }
        if values.is_empty() {
            values.push("为外部工具提供发布或社区资源索引。".to_owned());
        }
        return values
            .into_iter()
            .map(|value| sourced(value, GitHubAnalysisConfidence::High, source))
            .collect();
    }
    readme.map(extract_capabilities).unwrap_or_default()
}

fn sort_and_dedupe_conclusions(values: &mut Vec<SourcedConclusion<String>>) {
    values.sort_by(|left, right| left.value.cmp(&right.value));
    values.dedup_by(|left, right| left.value.eq_ignore_ascii_case(&right.value));
}
fn first_meaningful_paragraph(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("!["))
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(800)
        .collect()
}
fn extract_capabilities(source: &GitHubAnalysisSource) -> Vec<SourcedConclusion<String>> {
    let mut seen = BTreeSet::new();
    source
        .excerpt
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- ") || line.starts_with("* "))
        .map(|line| {
            line.trim_start_matches(['-', '*', ' '])
                .trim()
                .replace("**", "")
                .replace('`', "")
        })
        .filter(|line| {
            let normalized = line.to_ascii_lowercase();
            line.chars().count() >= 12
                && line.chars().count() <= 240
                && !line.ends_with(':')
                && !line.ends_with('：')
                && !line.starts_with("http")
                && !normalized.contains("badge")
                && seen.insert(normalized)
        })
        .take(6)
        .map(|value| sourced(value, GitHubAnalysisConfidence::Medium, source))
        .collect()
}

fn detect_tech_stack(
    manifests: &[&GitHubAnalysisSource],
    tree: Option<&GitHubAnalysisSource>,
) -> Vec<SourcedConclusion<String>> {
    let mut values = Vec::new();
    for source in manifests {
        let file = source.source_path.to_ascii_lowercase();
        for (needle, name) in [
            ("package.json", "Node.js / JavaScript"),
            ("cargo.toml", "Rust"),
            ("pyproject.toml", "Python"),
            ("requirements.txt", "Python"),
            ("go.mod", "Go"),
            ("pom.xml", "Java / Maven"),
            ("build.gradle", "Java / Gradle"),
            ("composer.json", "PHP"),
            ("gemfile", "Ruby"),
        ] {
            if file.ends_with(needle) {
                values.push(sourced(name.into(), GitHubAnalysisConfidence::High, source));
            }
        }
        for (needle, name) in [
            ("react", "React"),
            ("vite", "Vite"),
            ("tauri", "Tauri"),
            ("typescript", "TypeScript"),
            ("fastapi", "FastAPI"),
            ("django", "Django"),
            ("tokio", "Tokio"),
        ] {
            if source.excerpt.to_ascii_lowercase().contains(needle) {
                values.push(sourced(name.into(), GitHubAnalysisConfidence::High, source));
            }
        }
    }
    if values.is_empty() {
        if let Some(source) = tree {
            for (extension, name) in [
                (".rs", "Rust"),
                (".py", "Python"),
                (".ts", "TypeScript"),
                (".tsx", "TypeScript / React"),
                (".go", "Go"),
                (".java", "Java"),
            ] {
                if source.excerpt.contains(extension) {
                    values.push(sourced(
                        name.into(),
                        GitHubAnalysisConfidence::Medium,
                        source,
                    ));
                }
            }
        }
    }
    values
}

fn detect_repository_tooling(
    manifests: &[&GitHubAnalysisSource],
    workflows: &[&GitHubAnalysisSource],
    build_configs: &[&GitHubAnalysisSource],
) -> Vec<SourcedConclusion<String>> {
    let mut output = Vec::new();
    for source in manifests {
        let lower = source.source_path.to_ascii_lowercase();
        if lower.ends_with("package.json") {
            output.push(sourced(
                "Node.js / npm-compatible tooling".into(),
                GitHubAnalysisConfidence::High,
                source,
            ));
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&source.excerpt) {
                let scripts = value
                    .get("scripts")
                    .and_then(|value| value.as_object())
                    .into_iter()
                    .flatten()
                    .map(|(name, _)| name.to_ascii_lowercase())
                    .collect::<BTreeSet<_>>();
                if scripts
                    .iter()
                    .any(|name| name.contains("lint") || name.contains("format"))
                {
                    output.push(sourced(
                        "Lint / formatting automation".into(),
                        GitHubAnalysisConfidence::High,
                        source,
                    ));
                }
                if scripts
                    .iter()
                    .any(|name| name.contains("release") || name.contains("publish"))
                {
                    output.push(sourced(
                        "Release automation".into(),
                        GitHubAnalysisConfidence::High,
                        source,
                    ));
                }
            }
        }
    }
    for source in workflows {
        output.push(sourced(
            "GitHub Actions".into(),
            GitHubAnalysisConfidence::High,
            source,
        ));
    }
    for source in build_configs {
        output.push(sourced(
            format!("Build configuration: {}", source.source_path),
            GitHubAnalysisConfidence::Medium,
            source,
        ));
    }
    output
}

fn architecture_conclusions(
    tree: Option<&GitHubAnalysisSource>,
) -> Vec<SourcedConclusion<GitHubArchitectureModule>> {
    let Some(source) = tree else {
        return vec![];
    };
    let mut roots = BTreeSet::new();
    for line in source.excerpt.lines() {
        let path = line.split_whitespace().nth(1).unwrap_or_default();
        if let Some((root, _)) = path.split_once('/') {
            if !root.starts_with('.') {
                roots.insert(root.to_owned());
            }
        }
    }
    roots
        .into_iter()
        .take(10)
        .map(|root| {
            sourced(
                GitHubArchitectureModule {
                    name: root.clone(),
                    responsibility: format!("静态目录判断：{root} 是一个主要模块或内容分区。"),
                    key_paths: vec![root],
                },
                GitHubAnalysisConfidence::Low,
                source,
            )
        })
        .collect()
}
fn entrypoint_purpose(path: &str) -> String {
    if path.contains("src-tauri") {
        "桌面后端入口"
    } else if path.ends_with("main.rs") || path.ends_with("main.go") || path.ends_with("main.py") {
        "应用启动入口"
    } else if path.ends_with("lib.rs") {
        "库入口"
    } else {
        "前端或模块入口"
    }
    .into()
}

fn build_rules(
    manifests: &[&GitHubAnalysisSource],
    configs: &[&GitHubAnalysisSource],
) -> Vec<SourcedConclusion<String>> {
    let mut output = Vec::new();
    for source in manifests {
        let lower = source.source_path.to_ascii_lowercase();
        if lower.ends_with("package.json") {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&source.excerpt) {
                if let Some(scripts) = value.get("scripts").and_then(|v| v.as_object()) {
                    let names = scripts.keys().take(8).cloned().collect::<Vec<_>>();
                    if !names.is_empty() {
                        output.push(sourced(
                            format!("静态分析判断：可用 npm scripts：{}。", names.join(", ")),
                            GitHubAnalysisConfidence::Medium,
                            source,
                        ));
                    }
                }
            }
        } else if lower.ends_with("cargo.toml") {
            output.push(sourced(
                "静态分析判断：使用 Cargo 构建 Rust workspace/crate。".into(),
                GitHubAnalysisConfidence::Medium,
                source,
            ));
        } else if lower.ends_with("pyproject.toml") || lower.ends_with("requirements.txt") {
            output.push(sourced(
                "静态分析判断：需要 Python 环境并按 Manifest 安装依赖。".into(),
                GitHubAnalysisConfidence::Medium,
                source,
            ));
        } else if lower.ends_with("go.mod") {
            output.push(sourced(
                "静态分析判断：使用 Go Modules 构建。".into(),
                GitHubAnalysisConfidence::Medium,
                source,
            ));
        }
    }
    for source in configs.iter().take(3) {
        output.push(sourced(
            format!("静态分析判断：构建配置见 `{}`。", source.source_path),
            GitHubAnalysisConfidence::Medium,
            source,
        ));
    }
    output
}
fn test_rules(
    manifests: &[&GitHubAnalysisSource],
    configs: &[&GitHubAnalysisSource],
    workflows: &[&GitHubAnalysisSource],
) -> Vec<SourcedConclusion<String>> {
    let mut output = Vec::new();
    for source in configs.iter().chain(workflows.iter()).take(5) {
        output.push(sourced(
            format!(
                "静态分析判断：测试或 CI 配置见 `{}`，ForgeOS 未实际执行。",
                source.source_path
            ),
            GitHubAnalysisConfidence::Medium,
            source,
        ));
    }
    for source in manifests {
        if source.excerpt.to_ascii_lowercase().contains("test") {
            output.push(sourced(
                format!(
                    "静态分析判断：`{}` 声明了测试相关依赖或命令。",
                    source.source_path
                ),
                GitHubAnalysisConfidence::Medium,
                source,
            ));
        }
    }
    output.truncate(8);
    output
}
fn dependency_summary(
    manifests: &[&GitHubAnalysisSource],
    sbom: Option<&GitHubAnalysisSource>,
) -> Option<SourcedConclusion<GitHubDependencySummary>> {
    if let Some(source) = sbom {
        let value = serde_json::from_str::<serde_json::Value>(&source.excerpt).unwrap_or_default();
        let package_count = value
            .get("packageCount")
            .and_then(|value| value.as_u64())
            .map(|value| value as usize);
        let ecosystems = value
            .get("ecosystems")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
            .filter_map(|value| value.as_str())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        return Some(sourced(
            GitHubDependencySummary {
                package_managers: if ecosystems.is_empty() {
                    vec!["SBOM".into()]
                } else {
                    ecosystems
                },
                direct_dependency_count: package_count,
                sbom_available: true,
                summary: package_count.map_or_else(
                    || "仓库提供已有 SBOM；报告仅保存受限生态摘要。".into(),
                    |count| format!("GitHub SBOM 记录了 {count} 个包；报告不保存完整依赖清单。"),
                ),
            },
            GitHubAnalysisConfidence::High,
            source,
        ));
    }
    let source = *manifests.first()?;
    let mut managers = manifests
        .iter()
        .map(|value| manifest_manager(&value.source_path))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    managers.retain(|value| !value.is_empty());
    let count = if source
        .source_path
        .to_ascii_lowercase()
        .ends_with("package.json")
    {
        serde_json::from_str::<serde_json::Value>(&source.excerpt)
            .ok()
            .map(|value| {
                value
                    .get("dependencies")
                    .and_then(|v| v.as_object())
                    .map_or(0, |v| v.len())
                    + value
                        .get("devDependencies")
                        .and_then(|v| v.as_object())
                        .map_or(0, |v| v.len())
            })
    } else {
        None
    };
    Some(sourced(
        GitHubDependencySummary {
            package_managers: managers,
            direct_dependency_count: count,
            sbom_available: false,
            summary: "未找到现有 SBOM；依赖摘要来自 Manifest 静态分析。".into(),
        },
        GitHubAnalysisConfidence::Medium,
        source,
    ))
}
fn manifest_manager(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with("package.json") {
        "npm-compatible"
    } else if lower.ends_with("cargo.toml") {
        "Cargo"
    } else if lower.ends_with("go.mod") {
        "Go Modules"
    } else if lower.ends_with("pom.xml") {
        "Maven"
    } else if lower.contains("gradle") {
        "Gradle"
    } else if lower.ends_with("pyproject.toml") || lower.ends_with("requirements.txt") {
        "Python"
    } else {
        ""
    }
}
fn detect_license(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    [
        ("mit license", "MIT"),
        ("apache license", "Apache-2.0"),
        ("gnu general public license", "GPL"),
        ("bsd license", "BSD"),
    ]
    .into_iter()
    .find_map(|(needle, name)| lower.contains(needle).then(|| name.into()))
}
fn risk_rules(
    manifests: &[&GitHubAnalysisSource],
    tree: Option<&GitHubAnalysisSource>,
) -> Vec<SourcedConclusion<String>> {
    let mut output = Vec::new();
    if manifests.is_empty() {
        if let Some(source) = tree {
            output.push(sourced(
                "未识别到常见依赖 Manifest，复现步骤可能需要人工确认。".into(),
                GitHubAnalysisConfidence::Medium,
                source,
            ));
        }
    }
    if manifests.len() > 4 {
        output.push(sourced(
            "检测到多个 Manifest，仓库可能是 Monorepo；应按模块分别核对环境。".into(),
            GitHubAnalysisConfidence::Medium,
            manifests[0],
        ));
    }
    output
}
fn reading_order(
    readme: Option<&GitHubAnalysisSource>,
    manifests: &[&GitHubAnalysisSource],
    entrypoints: &[&GitHubAnalysisSource],
    tree: Option<&GitHubAnalysisSource>,
) -> Vec<SourcedConclusion<String>> {
    let mut output = Vec::new();
    if let Some(source) = readme {
        output.push(sourced(
            format!("先阅读 `{}` 了解项目定位与使用边界。", source.source_path),
            GitHubAnalysisConfidence::High,
            source,
        ));
    }
    for source in manifests.iter().take(3) {
        output.push(sourced(
            format!("再检查 `{}` 理解依赖、脚本和模块边界。", source.source_path),
            GitHubAnalysisConfidence::High,
            source,
        ));
    }
    for source in entrypoints.iter().take(2) {
        output.push(sourced(
            format!("随后阅读入口 `{}` 跟踪启动流程。", source.source_path),
            GitHubAnalysisConfidence::High,
            source,
        ));
    }
    if output.is_empty() {
        if let Some(source) = tree {
            output.push(sourced(
                "从仓库根目录结构开始，优先确认 Manifest 和入口文件。".into(),
                GitHubAnalysisConfidence::Low,
                source,
            ));
        }
    }
    output
}
fn reproduction(
    no_build_rule: bool,
    manifests: &[&GitHubAnalysisSource],
    source: &GitHubAnalysisSource,
    kind: GitHubRepositoryKind,
    repository_kind: &SourcedConclusion<GitHubRepositoryKind>,
) -> SourcedConclusion<GitHubReproductionAssessment> {
    let not_product_source = matches!(
        kind,
        GitHubRepositoryKind::ReleaseRegistry | GitHubRepositoryKind::Documentation
    );
    let difficulty = if not_product_source || manifests.is_empty() {
        ReproductionDifficulty::Unknown
    } else if manifests.len() > 4 || no_build_rule {
        ReproductionDifficulty::High
    } else {
        ReproductionDifficulty::Medium
    };
    SourcedConclusion {
        value: GitHubReproductionAssessment {
            difficulty,
            prerequisites: manifests
                .iter()
                .map(|value| manifest_manager(&value.source_path).to_owned())
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            caveats: if not_product_source {
                vec![
                    "该仓库不是产品源码仓库，因此无法据此复现产品应用。".into(),
                    "仅基于静态文件判断，ForgeOS 未克隆、安装依赖或执行测试。".into(),
                ]
            } else {
                vec!["仅基于静态文件判断，ForgeOS 未克隆、安装依赖或执行测试。".into()]
            },
        },
        confidence: if not_product_source {
            repository_kind.confidence
        } else {
            GitHubAnalysisConfidence::Medium
        },
        source_ids: if not_product_source {
            repository_kind.source_ids.clone()
        } else {
            vec![source.id.clone()]
        },
    }
}
fn extension_rules(
    architecture: &[SourcedConclusion<GitHubArchitectureModule>],
    entrypoints: &[SourcedConclusion<GitHubEntrypoint>],
) -> Vec<SourcedConclusion<String>> {
    architecture
        .iter()
        .take(3)
        .map(|value| SourcedConclusion {
            value: format!(
                "可从 `{}` 模块开始评估二开边界；先确认其公开接口和测试。",
                value.value.name
            ),
            confidence: GitHubAnalysisConfidence::Low,
            source_ids: value.source_ids.clone(),
        })
        .chain(entrypoints.iter().take(2).map(|value| SourcedConclusion {
            value: format!("入口 `{}` 是追踪运行流程的二开切入点。", value.value.path),
            confidence: GitHubAnalysisConfidence::Medium,
            source_ids: value.source_ids.clone(),
        }))
        .collect()
}
fn answer_question(
    question: &str,
    sources: &[GitHubAnalysisSource],
) -> Option<SourcedConclusion<String>> {
    let keywords = question
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|value| value.chars().count() >= 2)
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let source = sources
        .iter()
        .filter(|source| {
            !matches!(
                source.source_type,
                GitHubAnalysisSourceType::Metadata | GitHubAnalysisSourceType::Tree
            )
        })
        .max_by_key(|source| {
            keywords
                .iter()
                .filter(|keyword| {
                    source
                        .excerpt
                        .to_ascii_lowercase()
                        .contains(keyword.as_str())
                })
                .count()
        })?;
    let matches = keywords
        .iter()
        .filter(|keyword| {
            source
                .excerpt
                .to_ascii_lowercase()
                .contains(keyword.as_str())
        })
        .count();
    (matches > 0).then(|| {
        sourced(
            format!(
                "静态来源匹配：{}",
                first_meaningful_paragraph(&source.excerpt)
            ),
            GitHubAnalysisConfidence::Low,
            source,
        )
    })
}

fn recent_activity(sources: &[GitHubAnalysisSource]) -> Vec<GitHubRecentActivity> {
    let mut output = Vec::new();
    for source in sources
        .iter()
        .filter(|source| source.source_type == GitHubAnalysisSourceType::Commit)
        .take(5)
    {
        let value = serde_json::from_str::<serde_json::Value>(&source.excerpt).ok();
        let identifier = value
            .as_ref()
            .and_then(|value| value.get("sha"))
            .and_then(|value| value.as_str())
            .unwrap_or(&source.source_path)
            .to_owned();
        let title = value
            .as_ref()
            .and_then(|value| value.get("subject"))
            .and_then(|value| value.as_str())
            .unwrap_or(&source.excerpt)
            .to_owned();
        let occurred_at = value
            .as_ref()
            .and_then(|value| value.get("authoredAt"))
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        output.push(GitHubRecentActivity {
            kind: GitHubRecentActivityKind::Commit,
            identifier,
            title,
            occurred_at,
            source_ids: vec![source.id.clone()],
        });
    }
    if let Some(source) = sources
        .iter()
        .find(|source| source.source_type == GitHubAnalysisSourceType::Release)
    {
        let value = serde_json::from_str::<serde_json::Value>(&source.excerpt).ok();
        let identifier = value
            .as_ref()
            .and_then(|value| value.get("tagName"))
            .and_then(|value| value.as_str())
            .unwrap_or(&source.source_path)
            .to_owned();
        let title = value
            .as_ref()
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .unwrap_or(&identifier)
            .to_owned();
        let occurred_at = value
            .as_ref()
            .and_then(|value| value.get("publishedAt"))
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        output.push(GitHubRecentActivity {
            kind: GitHubRecentActivityKind::Release,
            identifier,
            title,
            occurred_at,
            source_ids: vec![source.id.clone()],
        });
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn section_availability(
    kind: GitHubRepositoryKind,
    warnings: &[GitHubAnalysisWarning],
    overview: bool,
    capabilities: bool,
    product_tech: bool,
    tooling: bool,
    architecture: bool,
    entrypoints: bool,
    build_and_run: bool,
    tests: bool,
    dependencies: bool,
    license: bool,
    recent: bool,
    risks: bool,
    extension_points: bool,
    recommended_reading: bool,
    reproduction: bool,
    user_question: Option<bool>,
) -> BTreeMap<String, GitHubSectionAvailability> {
    let budget_limited = warnings.iter().any(|warning| {
        matches!(
            warning,
            GitHubAnalysisWarning::TreeTruncated
                | GitHubAnalysisWarning::BudgetExceeded
                | GitHubAnalysisWarning::SourceUnavailable
        )
    });
    let missing = |available: bool| {
        if available {
            GitHubSectionAvailability::Available
        } else if budget_limited {
            GitHubSectionAvailability::BudgetLimited
        } else {
            GitHubSectionAvailability::SourceNotPresent
        }
    };
    let source_not_applicable = matches!(
        kind,
        GitHubRepositoryKind::ReleaseRegistry | GitHubRepositoryKind::Documentation
    );
    let source_section = |available: bool| {
        if available {
            GitHubSectionAvailability::Available
        } else if source_not_applicable {
            GitHubSectionAvailability::NotApplicable
        } else if budget_limited {
            GitHubSectionAvailability::BudgetLimited
        } else {
            GitHubSectionAvailability::StaticAnalysisInsufficient
        }
    };
    let mut availability = BTreeMap::from([
        ("overview".into(), missing(overview)),
        ("capabilities".into(), missing(capabilities)),
        ("product_tech_stack".into(), source_section(product_tech)),
        ("repository_tooling_stack".into(), missing(tooling)),
        ("architecture".into(), source_section(architecture)),
        ("entrypoints".into(), source_section(entrypoints)),
        ("build_and_run".into(), source_section(build_and_run)),
        ("test_strategy".into(), source_section(tests)),
        ("dependencies".into(), missing(dependencies)),
        ("license".into(), missing(license)),
        ("recent_activity".into(), missing(recent)),
        ("risks".into(), source_section(risks)),
        ("extension_points".into(), source_section(extension_points)),
        (
            "recommended_reading_order".into(),
            missing(recommended_reading),
        ),
        ("reproduction_assessment".into(), missing(reproduction)),
    ]);
    availability.insert(
        "user_question_answer".into(),
        match user_question {
            None => GitHubSectionAvailability::NotApplicable,
            Some(answered) => missing(answered),
        },
    );
    availability
}

fn summarize_sbom_excerpt(raw: &str) -> String {
    let value = serde_json::from_str::<serde_json::Value>(raw).unwrap_or_default();
    let packages = value
        .get("packages")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut ecosystems = BTreeSet::new();
    for package in &packages {
        if let Some(purl) = package.get("purl").and_then(|value| value.as_str()) {
            if let Some(kind) = purl
                .strip_prefix("pkg:")
                .and_then(|value| value.split('/').next())
            {
                ecosystems.insert(kind.to_owned());
            }
        }
    }
    serde_json::json!({
        "packageCount": packages.len(),
        "ecosystems": ecosystems,
    })
    .to_string()
}

fn validate_report_sources(
    report: &GitHubAnalysisReport,
    sources: &[GitHubAnalysisSource],
) -> Result<(), GitHubAnalysisError> {
    let valid = sources
        .iter()
        .map(|source| source.id.as_str())
        .collect::<HashSet<_>>();
    let json = serde_json::to_value(report)
        .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
    fn walk(value: &serde_json::Value, valid: &HashSet<&str>) -> bool {
        match value {
            serde_json::Value::Object(map) => {
                if let Some(ids) = map.get("sourceIds").and_then(|value| value.as_array()) {
                    if ids.is_empty()
                        || ids
                            .iter()
                            .any(|id| !id.as_str().is_some_and(|id| valid.contains(id)))
                    {
                        return false;
                    }
                }
                map.values().all(|value| walk(value, valid))
            }
            serde_json::Value::Array(items) => items.iter().all(|value| walk(value, valid)),
            _ => true,
        }
    }
    if !walk(&json, &valid) {
        return Err(GitHubAnalysisError::Validation(
            "report contains invalid source references".into(),
        ));
    }
    Ok(())
}

struct GitHubAnalysisRenderer;
impl GitHubAnalysisRenderer {
    fn render(report: &GitHubAnalysisReport) -> String {
        let mut out = vec![
            format!(
                "# GitHub Project Analysis: {}/{}",
                report.repository.owner, report.repository.name
            ),
            String::new(),
            format!("- Repository: {}", report.repository.url),
            format!("- Default branch: {}", report.repository.default_branch),
            format!("- HEAD: {}", report.repository.head_commit_sha),
            String::new(),
        ];
        if let Some(kind) = &report.repository_kind {
            out.push("## Repository kind and boundary".into());
            out.push(format!("- Kind: {}", kind.value.as_str()));
            if kind.value == GitHubRepositoryKind::ReleaseRegistry {
                out.push("- This repository stores release or community registry metadata; it is not the product application source repository.".into());
            }
            render_citations(&mut out, kind);
            out.push(String::new());
        }
        render_one(&mut out, "Project overview", report.overview.as_ref());
        render_many(&mut out, "Capabilities", &report.capabilities);
        if report.schema_version.as_deref() == Some("github-report-v2") {
            render_many_with_availability(
                &mut out,
                "Product technology stack",
                &report.product_tech_stack,
                availability(report, "product_tech_stack"),
            );
            render_many_with_availability(
                &mut out,
                "Repository maintenance tooling",
                &report.repository_tooling_stack,
                availability(report, "repository_tooling_stack"),
            );
        } else {
            render_many(&mut out, "Tech stack", &report.tech_stack);
        }
        out.push("## Architecture".into());
        for item in &report.architecture {
            out.push(format!(
                "- **{}** — {} ({})",
                item.value.name,
                item.value.responsibility,
                item.value.key_paths.join(", ")
            ));
            render_citations(&mut out, item);
        }
        if report.architecture.is_empty() {
            out.push(availability_message(availability(report, "architecture")).into());
        }
        out.push(String::new());
        out.push("## Entrypoints".into());
        for item in &report.entrypoints {
            out.push(format!("- `{}` — {}", item.value.path, item.value.purpose));
            render_citations(&mut out, item);
        }
        if report.entrypoints.is_empty() {
            out.push(availability_message(availability(report, "entrypoints")).into());
        }
        out.push(String::new());
        render_many(
            &mut out,
            "Build and run (static analysis)",
            &report.build_and_run,
        );
        render_many(
            &mut out,
            "Test strategy (static analysis)",
            &report.test_strategy,
        );
        out.push("## Dependencies and SBOM".into());
        if let Some(value) = &report.dependencies {
            out.push(format!(
                "- Package managers: {}",
                value.value.package_managers.join(", ")
            ));
            out.push(format!("- {}", value.value.summary));
            out.push(format!("- Existing SBOM: {}", value.value.sbom_available));
            render_citations(&mut out, value);
        } else {
            out.push(availability_message(availability(report, "dependencies")).into());
        }
        out.push(String::new());
        out.push("## License and reuse".into());
        if let Some(value) = &report.license {
            out.push(format!(
                "- Identifier: {}",
                value.value.identifier.as_deref().unwrap_or("Unknown")
            ));
            out.push(format!("- {}", value.value.reuse_guidance));
            render_citations(&mut out, value);
        } else {
            out.push(availability_message(availability(report, "license")).into());
        }
        out.push(String::new());
        out.push("## Recent commits and release".into());
        if report.recent_activity.is_empty() {
            out.push(availability_message(availability(report, "recent_activity")).into());
        }
        for item in &report.recent_activity {
            out.push(format!(
                "- {} `{}` — {}{}",
                item.kind.as_str(),
                item.identifier,
                item.title,
                item.occurred_at
                    .as_deref()
                    .map(|value| format!(" ({value})"))
                    .unwrap_or_default()
            ));
            out.push(format!("  - Sources: {}", item.source_ids.join(", ")));
        }
        out.push(String::new());
        render_many(&mut out, "Risks", &report.risks);
        render_many(
            &mut out,
            "Recommended reading order",
            &report.recommended_reading_order,
        );
        render_many(&mut out, "Extension points", &report.extension_points);
        if let Some(value) = &report.user_question_answer {
            render_one(&mut out, "Answer to custom question", Some(value));
        }
        out.push("## Reproduction assessment".into());
        out.push(format!(
            "- Difficulty: {:?}",
            report.reproduction_assessment.value.difficulty
        ));
        for caveat in &report.reproduction_assessment.value.caveats {
            out.push(format!("- {caveat}"));
        }
        render_citations(&mut out, &report.reproduction_assessment);
        out.push(String::new());
        out.push("## Completeness".into());
        out.push(format!(
            "- Readiness: {}; files selected: {}; files skipped: {}; tree entries: {}; source bytes: {}",
            report.completeness.readiness.as_str(),
            report.completeness.files_selected,
            report.completeness.files_skipped,
            report.completeness.tree_entries_seen,
            report.completeness.source_bytes
        ));
        for warning in &report.quality_warnings {
            out.push(format!("- Warning: {}", warning.as_str()));
        }
        out.join("\n").trim().to_owned() + "\n"
    }
}
fn render_one(out: &mut Vec<String>, title: &str, value: Option<&SourcedConclusion<String>>) {
    out.push(format!("## {title}"));
    if let Some(value) = value {
        out.push(value.value.clone());
        render_citations(out, value);
    } else {
        out.push("No source-backed conclusion available.".into());
    }
    out.push(String::new());
}
fn render_many(out: &mut Vec<String>, title: &str, values: &[SourcedConclusion<String>]) {
    out.push(format!("## {title}"));
    if values.is_empty() {
        out.push("No source-backed conclusion available.".into());
    }
    for value in values {
        out.push(format!("- {}", value.value));
        render_citations(out, value);
    }
    out.push(String::new());
}

fn render_many_with_availability(
    out: &mut Vec<String>,
    title: &str,
    values: &[SourcedConclusion<String>],
    availability: GitHubSectionAvailability,
) {
    out.push(format!("## {title}"));
    if values.is_empty() {
        out.push(availability_message(availability).into());
    }
    for value in values {
        out.push(format!("- {}", value.value));
        render_citations(out, value);
    }
    out.push(String::new());
}

fn availability(report: &GitHubAnalysisReport, section: &str) -> GitHubSectionAvailability {
    report
        .section_availability
        .get(section)
        .copied()
        .unwrap_or(GitHubSectionAvailability::SourceNotPresent)
}

fn availability_message(value: GitHubSectionAvailability) -> &'static str {
    match value {
        GitHubSectionAvailability::Available => "Source-backed content is available.",
        GitHubSectionAvailability::SourceNotPresent => {
            "No reliable source for this section was found in the repository."
        }
        GitHubSectionAvailability::NotApplicable => {
            "This section is not applicable to the detected repository kind."
        }
        GitHubSectionAvailability::BudgetLimited => {
            "The collection budget or unavailable sources prevented a complete conclusion."
        }
        GitHubSectionAvailability::StaticAnalysisInsufficient => {
            "Static analysis did not identify a reliable conclusion."
        }
    }
}
fn render_citations<T>(out: &mut Vec<String>, value: &SourcedConclusion<T>) {
    out.push(format!(
        "  - Confidence: {}; Sources: {}",
        value.confidence.as_str(),
        value.source_ids.join(", ")
    ));
}

fn request_hash(
    workspace_id: &str,
    repository_id: &str,
    head: &str,
    focus: GitHubAnalysisFocus,
    prompt: Option<&str>,
    mode: GitHubAnalysisMode,
) -> String {
    sha256(&format!(
        "{workspace_id}\n{repository_id}\n{head}\n{}\n{}\n{}\n{ANALYZER_REVISION}",
        focus.as_str(),
        prompt
            .unwrap_or_default()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
        mode.as_str()
    ))
}
fn sha256(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn file_budget(remaining: Option<i64>, authenticated: bool) -> usize {
    if authenticated {
        MAX_FILES
    } else {
        remaining
            .map(|value| value.saturating_sub(6).clamp(0, MAX_FILES as i64) as usize)
            .unwrap_or(24)
    }
}
fn apply_rate(run: &mut GitHubAnalysisRun, rate: &GitHubRateLimit) {
    run.api_requests_remaining = rate.remaining.or(run.api_requests_remaining);
    run.api_rate_limit_reset_at =
        parse_rate_reset(rate.reset_at.as_deref()).or(run.api_rate_limit_reset_at);
}
fn parse_rate_reset(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
}
fn dedupe_warnings(values: &mut Vec<GitHubAnalysisWarning>) {
    values.sort_by_key(|value| value.as_str());
    values.dedup();
}
fn progress_from_run(run: &GitHubAnalysisRun, message: &str) -> GitHubAnalysisProgress {
    GitHubAnalysisProgress {
        analysis_run_id: run.id.clone(),
        stage: run
            .stage
            .map(|value| value.as_str())
            .unwrap_or("completed")
            .into(),
        message: message.into(),
        tree_entries_seen: run.tree_entries_seen,
        files_read: run.files_read,
        files_skipped: run.files_skipped,
        api_requests_used: run.api_requests_used,
        api_requests_remaining: run.api_requests_remaining,
        ai_invoked: run.ai_invoked,
    }
}
fn map_store(error: RepositoryError) -> GitHubAnalysisError {
    match error {
        RepositoryError::Conflict => GitHubAnalysisError::AlreadyRunning,
        RepositoryError::OptimisticConflict => GitHubAnalysisError::VersionConflict,
        RepositoryError::CorruptData(value) => GitHubAnalysisError::CorruptData(value),
        other => GitHubAnalysisError::Storage(other.to_string()),
    }
}
fn error_code(error: &GitHubAnalysisError) -> (&'static str, GitHubAnalysisReadiness) {
    match error {
        GitHubAnalysisError::Source(GitHubProjectSourceError::RateLimited { .. }) => (
            "github_analysis_rate_limited",
            GitHubAnalysisReadiness::Incomplete,
        ),
        GitHubAnalysisError::Source(GitHubProjectSourceError::NotFound) => (
            "github_analysis_not_found",
            GitHubAnalysisReadiness::Incomplete,
        ),
        GitHubAnalysisError::PublicOnly => (
            "github_analysis_public_only",
            GitHubAnalysisReadiness::Blocked,
        ),
        GitHubAnalysisError::InvalidPrompt => (
            "github_analysis_prompt_rejected",
            GitHubAnalysisReadiness::Blocked,
        ),
        GitHubAnalysisError::Validation(_) => {
            ("github_analysis_blocked", GitHubAnalysisReadiness::Blocked)
        }
        _ => (
            "github_analysis_source_unavailable",
            GitHubAnalysisReadiness::Incomplete,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_run(name: &str) -> GitHubAnalysisRun {
        GitHubAnalysisRun::create(
            format!("run-{name}"),
            "workspace".into(),
            format!("repository-{name}"),
            "owner".into(),
            name.into(),
            format!("https://github.com/owner/{name}"),
            "main".into(),
            "0123456789abcdef".into(),
            GitHubAnalysisFocus::Overview,
            None,
            GitHubAnalysisMode::Rules,
            ANALYZER_REVISION.into(),
            "c".repeat(64),
            Utc::now(),
        )
        .expect("run")
    }

    fn test_source(
        run: &GitHubAnalysisRun,
        kind: GitHubAnalysisSourceType,
        path: &str,
        excerpt: &str,
        order: i64,
    ) -> GitHubAnalysisSource {
        make_source(
            &run.id,
            kind,
            path.into(),
            Some(format!("sha-{order}")),
            excerpt.into(),
            order,
        )
        .expect("source")
    }

    #[test]
    fn selector_is_deterministic_and_excludes_generated_or_secret_files() {
        let entries = vec![
            GitHubTreeEntry {
                path: "src/main.rs".into(),
                kind: "blob".into(),
                sha: "a".into(),
                size: Some(10),
            },
            GitHubTreeEntry {
                path: "package.json".into(),
                kind: "blob".into(),
                sha: "b".into(),
                size: Some(10),
            },
            GitHubTreeEntry {
                path: "README.md".into(),
                kind: "blob".into(),
                sha: "c".into(),
                size: Some(10),
            },
            GitHubTreeEntry {
                path: ".env".into(),
                kind: "blob".into(),
                sha: "d".into(),
                size: Some(10),
            },
            GitHubTreeEntry {
                path: "dist/app.min.js".into(),
                kind: "blob".into(),
                sha: "e".into(),
                size: Some(10),
            },
        ];
        let selected = select_sources(&entries, 64);
        assert_eq!(
            selected
                .iter()
                .map(|value| value.path.as_str())
                .collect::<Vec<_>>(),
            vec!["README.md", "package.json", "src/main.rs"]
        );
    }

    #[test]
    fn selector_reserves_budget_for_root_workspace_manifest() {
        let mut entries = (0..70)
            .map(|index| GitHubTreeEntry {
                path: format!("examples/example-{index}/README.md"),
                kind: "blob".into(),
                sha: format!("readme-{index}"),
                size: Some(10),
            })
            .collect::<Vec<_>>();
        entries.push(GitHubTreeEntry {
            path: "Cargo.toml".into(),
            kind: "blob".into(),
            sha: "workspace".into(),
            size: Some(10),
        });
        let selected = select_sources(&entries, 64);
        assert!(selected.iter().any(|value| value.path == "Cargo.toml"));
        assert!(
            selected
                .iter()
                .filter(|value| value.source_type == GitHubAnalysisSourceType::Readme)
                .count()
                <= 8
        );
    }

    #[test]
    fn content_policy_redacts_secret_and_reports_blocking_content() {
        let (value, blocked, _) =
            GitHubAnalysisContentPolicy::sanitize("API_KEY=ghp_abcdefghijklmnopqrstuvwxyz123456");
        assert!(blocked);
        assert!(!value.contains("ghp_"));
    }

    #[test]
    fn request_fingerprint_is_stable_for_prompt_whitespace() {
        assert_eq!(
            request_hash(
                "w",
                "r",
                "h",
                GitHubAnalysisFocus::Custom,
                Some("how   build"),
                GitHubAnalysisMode::Rules
            ),
            request_hash(
                "w",
                "r",
                "h",
                GitHubAnalysisFocus::Custom,
                Some("how build"),
                GitHubAnalysisMode::Rules
            )
        );
    }

    #[test]
    fn low_anonymous_rate_limit_does_not_spend_reserved_requests_on_files() {
        assert_eq!(file_budget(Some(6), false), 0);
        assert_eq!(file_budget(Some(10), false), 4);
        assert_eq!(file_budget(Some(10), true), MAX_FILES);
    }

    #[test]
    fn rules_report_has_valid_sources_and_stable_complete_markdown() {
        let now = Utc::now();
        let mut run = GitHubAnalysisRun::create(
            "run".into(),
            "workspace".into(),
            "repository".into(),
            "owner".into(),
            "sample".into(),
            "https://github.com/owner/sample".into(),
            "main".into(),
            "0123456789abcdef".into(),
            GitHubAnalysisFocus::Overview,
            None,
            GitHubAnalysisMode::Rules,
            ANALYZER_REVISION.into(),
            "c".repeat(64),
            now,
        )
        .expect("run");
        run.files_read = 2;
        let sources = vec![
            metadata_source(&run),
            make_source(
                &run.id,
                GitHubAnalysisSourceType::Readme,
                "README.md".into(),
                Some("readme-sha".into()),
                "Sample project\n\n- Analyze repositories safely".into(),
                1,
            )
            .expect("readme"),
            make_source(
                &run.id,
                GitHubAnalysisSourceType::Manifest,
                "package.json".into(),
                Some("manifest-sha".into()),
                r#"{"scripts":{"build":"vite build","test":"vitest"},"dependencies":{"react":"latest"}}"#.into(),
                2,
            )
            .expect("manifest"),
            make_source(
                &run.id,
                GitHubAnalysisSourceType::License,
                "LICENSE".into(),
                Some("license-sha".into()),
                "MIT License".into(),
                3,
            )
            .expect("license"),
        ];
        let report = GitHubRulesAnalyzer::analyze(
            &run,
            &sources,
            vec![GitHubAnalysisWarning::SbomMissing],
            false,
        )
        .expect("report");
        validate_report_sources(&report, &sources).expect("source references");
        let first = GitHubAnalysisRenderer::render(&report);
        let second = GitHubAnalysisRenderer::render(&report);
        assert_eq!(first, second);
        assert!(first.contains("## Dependencies and SBOM"));
        assert!(first.contains("## License and reuse"));
        assert!(first.contains("Warning: sbom_missing"));
    }

    #[test]
    fn repository_classifier_distinguishes_repository_kinds() {
        let cases = [
            (
                "obsidian-releases",
                "Community plugin list",
                "blob community-plugins.json",
                vec![],
                GitHubRepositoryKind::ReleaseRegistry,
            ),
            (
                "app",
                "Desktop application",
                "blob src/main.ts",
                vec![("package.json", r#"{"dependencies":{"react":"1"}}"#)],
                GitHubRepositoryKind::SourceApplication,
            ),
            (
                "sdk",
                "A Rust library and SDK",
                "blob src/lib.rs",
                vec![("Cargo.toml", "[package]\nname='sdk'")],
                GitHubRepositoryKind::Library,
            ),
            (
                "workspace",
                "Workspace project",
                "blob crates/a/Cargo.toml\nblob apps/b/package.json",
                vec![
                    ("crates/a/Cargo.toml", "[package]"),
                    ("apps/b/package.json", "{}"),
                ],
                GitHubRepositoryKind::Monorepo,
            ),
            (
                "cargo-workspace",
                "Rust application workspace",
                "blob crates/a/Cargo.toml",
                vec![("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]")],
                GitHubRepositoryKind::Monorepo,
            ),
            (
                "product-docs",
                "Documentation repository",
                "blob guide/getting-started.md",
                vec![],
                GitHubRepositoryKind::Documentation,
            ),
            (
                "starter-template",
                "Starter template for desktop apps",
                "blob README.md",
                vec![],
                GitHubRepositoryKind::Template,
            ),
        ];
        for (name, readme, tree, manifests, expected) in cases {
            let run = test_run(name);
            let readme_source = test_source(
                &run,
                GitHubAnalysisSourceType::Readme,
                "README.md",
                readme,
                1,
            );
            let tree_source = test_source(&run, GitHubAnalysisSourceType::Tree, "", tree, 2);
            let manifest_sources = manifests
                .into_iter()
                .enumerate()
                .map(|(index, (path, excerpt))| {
                    test_source(
                        &run,
                        GitHubAnalysisSourceType::Manifest,
                        path,
                        excerpt,
                        index as i64 + 3,
                    )
                })
                .collect::<Vec<_>>();
            let manifest_refs = manifest_sources.iter().collect::<Vec<_>>();
            assert_eq!(
                classify_repository(
                    &run,
                    Some(&readme_source),
                    &manifest_refs,
                    Some(&tree_source)
                )
                .value,
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn advisory_license_and_sbom_warnings_keep_report_ready_with_warning() {
        let run = test_run("source-app");
        let sources = vec![
            metadata_source(&run),
            test_source(
                &run,
                GitHubAnalysisSourceType::Tree,
                "",
                "blob src/main.ts",
                1,
            ),
            test_source(
                &run,
                GitHubAnalysisSourceType::Readme,
                "README.md",
                "Source application\n\n- Provides a reliable desktop workflow.",
                2,
            ),
            test_source(
                &run,
                GitHubAnalysisSourceType::Manifest,
                "package.json",
                r#"{"dependencies":{"react":"1"}}"#,
                3,
            ),
        ];
        let report = GitHubRulesAnalyzer::analyze(
            &run,
            &sources,
            vec![
                GitHubAnalysisWarning::LicenseMissing,
                GitHubAnalysisWarning::SbomMissing,
            ],
            false,
        )
        .expect("report");
        assert_eq!(
            report.completeness.readiness,
            GitHubAnalysisReadiness::Warning
        );
        assert_eq!(
            report.repository_kind.as_ref().map(|value| value.value),
            Some(GitHubRepositoryKind::SourceApplication)
        );
    }

    #[test]
    fn release_registry_does_not_report_maintenance_manifest_as_product_stack() {
        let run = test_run("obsidian-releases");
        let sources = vec![
            metadata_source(&run),
            test_source(
                &run,
                GitHubAnalysisSourceType::Tree,
                "",
                "blob community-plugins.json\nblob package.json",
                1,
            ),
            test_source(
                &run,
                GitHubAnalysisSourceType::Readme,
                "README.md",
                "Community plugin list",
                2,
            ),
            test_source(
                &run,
                GitHubAnalysisSourceType::Manifest,
                "package.json",
                r#"{"scripts":{"format":"prettier --write ."},"devDependencies":{"prettier":"1"}}"#,
                3,
            ),
        ];
        let report = GitHubRulesAnalyzer::analyze(&run, &sources, vec![], false).expect("report");
        assert!(report.product_tech_stack.is_empty());
        assert!(report
            .repository_tooling_stack
            .iter()
            .any(|value| value.value.contains("Node.js")));
        assert!(report.architecture.is_empty());
        assert_eq!(
            report.section_availability.get("architecture"),
            Some(&GitHubSectionAvailability::NotApplicable)
        );
    }

    #[test]
    fn recent_activity_uses_structured_commit_and_release_sources() {
        let run = test_run("activity");
        let sources = vec![
            test_source(
                &run,
                GitHubAnalysisSourceType::Commit,
                "abc123",
                r#"{"sha":"abc123","subject":"Improve report","authoredAt":"2026-07-22T00:00:00Z"}"#,
                1,
            ),
            test_source(
                &run,
                GitHubAnalysisSourceType::Release,
                "v0.2.0",
                r#"{"tagName":"v0.2.0","name":"Preview","publishedAt":"2026-07-22T01:00:00Z"}"#,
                2,
            ),
        ];
        let activity = recent_activity(&sources);
        assert_eq!(activity.len(), 2);
        assert_eq!(activity[0].identifier, "abc123");
        assert_eq!(activity[1].kind, GitHubRecentActivityKind::Release);
        assert!(activity.iter().all(|item| item.source_ids.len() == 1));
    }

    #[test]
    fn v1_report_json_remains_backward_compatible() {
        let value = r#"{"repository":{"owner":"o","name":"n","url":"https://github.com/o/n","defaultBranch":"main","headCommitSha":"abc"},"overview":null,"capabilities":[],"techStack":[],"architecture":[],"entrypoints":[],"buildAndRun":[],"testStrategy":[],"dependencies":null,"license":null,"risks":[],"recommendedReadingOrder":[],"reproductionAssessment":{"value":{"difficulty":"unknown","prerequisites":[],"caveats":[]},"confidence":"low","sourceIds":["source"]},"extensionPoints":[],"userQuestionAnswer":null,"completeness":{"readiness":"warning","filesSelected":0,"filesSkipped":0,"treeEntriesSeen":0,"sourceBytes":0},"qualityWarnings":["license_missing"]}"#;
        let report: GitHubAnalysisReport = serde_json::from_str(value).expect("v1 report");
        assert!(report.schema_version.is_none());
        assert!(report.product_tech_stack.is_empty());
        assert!(report.recent_activity.is_empty());
    }
}
