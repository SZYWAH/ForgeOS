use std::{
    collections::{BTreeSet, HashSet},
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
        GitHubReproductionAssessment, ReproductionDifficulty, SourcedConclusion,
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

pub const ANALYZER_REVISION: &str = "github-rules-v1";
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
            .map_err(|error| GitHubAnalysisError::Validation(error.to_string()))?;
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
                    for (sha, message) in supplemental.commits {
                        let excerpt =
                            truncate_unicode(&redact_external_text(&message), MAX_SOURCE_CHARS);
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Commit,
                            sha.clone(),
                            Some(sha),
                            excerpt,
                            sources.len() as i64,
                        )?);
                    }
                    if let Some((tag, name)) = supplemental.release {
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Release,
                            tag,
                            None,
                            truncate_unicode(&redact_external_text(&name), MAX_SOURCE_CHARS),
                            sources.len() as i64,
                        )?);
                    }
                    if let Some(sbom) = supplemental.sbom {
                        sources.push(make_source(
                            &run.id,
                            GitHubAnalysisSourceType::Sbom,
                            "sbom".into(),
                            None,
                            truncate_unicode(&redact_external_text(&sbom), MAX_SOURCE_CHARS),
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
        let status = if report.completeness.readiness == GitHubAnalysisReadiness::Ready {
            GitHubAnalysisStatus::Ready
        } else {
            GitHubAnalysisStatus::Partial
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
    let mut manifest_roots = HashSet::new();
    for candidate in candidates {
        if candidate.source_type == GitHubAnalysisSourceType::Manifest {
            let root = candidate
                .path
                .split('/')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !root.is_empty() && manifest_roots.insert(root) {
                selected.push(candidate);
                continue;
            }
        }
        selected.push(candidate);
        if selected.len() >= limit {
            break;
        }
    }
    selected.truncate(limit);
    selected
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
        let mut tech = detect_tech_stack(&manifests, tree.as_ref().copied());
        tech.sort_by(|left, right| left.value.cmp(&right.value));
        tech.dedup_by(|left, right| left.value == right.value);
        let overview = readme.map(|source| {
            sourced(
                first_meaningful_paragraph(&source.excerpt),
                GitHubAnalysisConfidence::Medium,
                source,
            )
        });
        let capabilities = readme.map(extract_capabilities).unwrap_or_default();
        let architecture = architecture_conclusions(tree.as_ref().copied());
        let entrypoints: Vec<SourcedConclusion<GitHubEntrypoint>> =
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
                .collect();
        let build_and_run =
            build_rules(&manifests, &by_type(GitHubAnalysisSourceType::BuildConfig));
        let test_strategy = test_rules(
            &manifests,
            &by_type(GitHubAnalysisSourceType::TestConfig),
            &by_type(GitHubAnalysisSourceType::Workflow),
        );
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
        let risks = risk_rules(&manifests, tree.as_ref().copied());
        let recommended_reading_order = reading_order(
            readme,
            &manifests,
            &by_type(GitHubAnalysisSourceType::Entrypoint),
            tree.as_ref().copied(),
        );
        let reproduction_assessment = reproduction(build_and_run.is_empty(), &manifests, metadata);
        let extension_points = extension_rules(&architecture, &entrypoints);
        let user_question_answer = run
            .custom_prompt
            .as_deref()
            .and_then(|question| answer_question(question, sources));
        if blocked {
            warnings.push(GitHubAnalysisWarning::PromptSanitized);
        }
        dedupe_warnings(&mut warnings);
        let readiness = if blocked {
            GitHubAnalysisReadiness::Blocked
        } else if warnings.contains(&GitHubAnalysisWarning::NoAnalyzableText) {
            GitHubAnalysisReadiness::Incomplete
        } else if warnings.is_empty() {
            GitHubAnalysisReadiness::Ready
        } else {
            GitHubAnalysisReadiness::Warning
        };
        Ok(GitHubAnalysisReport {
            repository: GitHubAnalysisRepositoryIdentity {
                owner: run.owner.clone(),
                name: run.name.clone(),
                url: run.repository_url.clone(),
                default_branch: run.default_branch.clone(),
                head_commit_sha: run.source_commit_sha.clone(),
            },
            overview,
            capabilities,
            tech_stack: tech,
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
    source
        .excerpt
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("- ") || line.starts_with("* "))
        .map(|line| line.trim_start_matches(['-', '*', ' ']).to_owned())
        .filter(|line| line.chars().count() >= 8)
        .take(8)
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
        return Some(sourced(
            GitHubDependencySummary {
                package_managers: vec!["SBOM".into()],
                direct_dependency_count: None,
                sbom_available: true,
                summary: "仓库提供已有 SBOM；报告仅保存受限摘要。".into(),
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
) -> SourcedConclusion<GitHubReproductionAssessment> {
    let difficulty = if manifests.is_empty() {
        ReproductionDifficulty::Unknown
    } else if manifests.len() > 4 || no_build_rule {
        ReproductionDifficulty::High
    } else {
        ReproductionDifficulty::Medium
    };
    sourced(
        GitHubReproductionAssessment {
            difficulty,
            prerequisites: manifests
                .iter()
                .map(|value| manifest_manager(&value.source_path).to_owned())
                .filter(|value| !value.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            caveats: vec!["仅基于静态文件判断，ForgeOS 未克隆、安装依赖或执行测试。".into()],
        },
        GitHubAnalysisConfidence::Medium,
        source,
    )
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
        render_one(&mut out, "Project overview", report.overview.as_ref());
        render_many(&mut out, "Capabilities", &report.capabilities);
        render_many(&mut out, "Tech stack", &report.tech_stack);
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
        out.push(String::new());
        out.push("## Entrypoints".into());
        for item in &report.entrypoints {
            out.push(format!("- `{}` — {}", item.value.path, item.value.purpose));
            render_citations(&mut out, item);
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
            out.push("No source-backed conclusion available.".into());
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
            out.push("No source-backed conclusion available.".into());
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
}
