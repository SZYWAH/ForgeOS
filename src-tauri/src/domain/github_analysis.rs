use std::{collections::BTreeMap, fmt, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::DomainError;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $value),+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    _ => Err(DomainError::InvalidGitHubAnalysis(value.to_owned())),
                }
            }
        }
    };
}

string_enum!(GitHubAnalysisStatus {
    Queued => "queued",
    Collecting => "collecting",
    Analyzing => "analyzing",
    Ready => "ready",
    Partial => "partial",
    Failed => "failed",
    Archived => "archived",
});

string_enum!(GitHubAnalysisStage {
    Resolving => "resolving",
    CollectingTree => "collecting_tree",
    ReadingSources => "reading_sources",
    RulesAnalysis => "rules_analysis",
    AiAnalysis => "ai_analysis",
    Persisting => "persisting",
});

string_enum!(GitHubAnalysisFocus {
    Overview => "overview",
    Architecture => "architecture",
    Learning => "learning",
    Reproduction => "reproduction",
    SecondaryDevelopment => "secondary_development",
    Custom => "custom",
});

string_enum!(GitHubAnalysisMode { Rules => "rules", Ai => "ai" });

string_enum!(GitHubAnalysisReadiness {
    Ready => "ready",
    Warning => "warning",
    Incomplete => "incomplete",
    Blocked => "blocked",
});

string_enum!(GitHubAnalysisSourceType {
    Metadata => "metadata",
    Tree => "tree",
    Readme => "readme",
    Documentation => "documentation",
    Manifest => "manifest",
    License => "license",
    Workflow => "workflow",
    TestConfig => "test_config",
    BuildConfig => "build_config",
    Release => "release",
    Commit => "commit",
    Sbom => "sbom",
    Entrypoint => "entrypoint",
});

string_enum!(GitHubAnalysisConfidence { High => "high", Medium => "medium", Low => "low" });

string_enum!(GitHubRepositoryKind {
    SourceApplication => "source_application",
    Library => "library",
    Monorepo => "monorepo",
    ReleaseRegistry => "release_registry",
    Documentation => "documentation",
    Template => "template",
    Unknown => "unknown",
});

string_enum!(GitHubSectionAvailability {
    Available => "available",
    SourceNotPresent => "source_not_present",
    NotApplicable => "not_applicable",
    BudgetLimited => "budget_limited",
    StaticAnalysisInsufficient => "static_analysis_insufficient",
});

string_enum!(GitHubRecentActivityKind { Commit => "commit", Release => "release" });

string_enum!(GitHubAnalysisWarning {
    TreeTruncated => "tree_truncated",
    BudgetExceeded => "budget_exceeded",
    SourceUnavailable => "source_unavailable",
    ReadmeMissing => "readme_missing",
    LicenseMissing => "license_missing",
    SbomMissing => "sbom_missing",
    NoAnalyzableText => "no_analyzable_text",
    PromptSanitized => "prompt_sanitized",
    AiUnavailable => "github_analysis_ai_unavailable",
    AiFallback => "github_analysis_ai_fallback",
    AiInvalidResponse => "github_analysis_ai_invalid_response",
    AiSourceInvalid => "github_analysis_ai_source_invalid",
    AiProviderUnavailable => "github_analysis_ai_provider_unavailable",
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcedConclusion<T> {
    pub value: T,
    pub confidence: GitHubAnalysisConfidence,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAnalysisRepositoryIdentity {
    pub owner: String,
    pub name: String,
    pub url: String,
    pub default_branch: String,
    pub head_commit_sha: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubArchitectureModule {
    pub name: String,
    pub responsibility: String,
    pub key_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubEntrypoint {
    pub path: String,
    pub purpose: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubDependencySummary {
    pub package_managers: Vec<String>,
    pub direct_dependency_count: Option<usize>,
    pub sbom_available: bool,
    pub summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubLicenseSummary {
    pub identifier: Option<String>,
    pub reuse_guidance: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReproductionDifficulty {
    Low,
    Medium,
    High,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubReproductionAssessment {
    pub difficulty: ReproductionDifficulty,
    pub prerequisites: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAnalysisCompleteness {
    pub readiness: GitHubAnalysisReadiness,
    pub files_selected: usize,
    pub files_skipped: usize,
    pub tree_entries_seen: usize,
    pub source_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRecentActivity {
    pub kind: GitHubRecentActivityKind,
    pub identifier: String,
    pub title: String,
    pub occurred_at: Option<String>,
    pub source_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubAnalysisReport {
    #[serde(default)]
    pub schema_version: Option<String>,
    pub repository: GitHubAnalysisRepositoryIdentity,
    #[serde(default)]
    pub repository_kind: Option<SourcedConclusion<GitHubRepositoryKind>>,
    pub overview: Option<SourcedConclusion<String>>,
    pub capabilities: Vec<SourcedConclusion<String>>,
    pub tech_stack: Vec<SourcedConclusion<String>>,
    #[serde(default)]
    pub product_tech_stack: Vec<SourcedConclusion<String>>,
    #[serde(default)]
    pub repository_tooling_stack: Vec<SourcedConclusion<String>>,
    pub architecture: Vec<SourcedConclusion<GitHubArchitectureModule>>,
    pub entrypoints: Vec<SourcedConclusion<GitHubEntrypoint>>,
    pub build_and_run: Vec<SourcedConclusion<String>>,
    pub test_strategy: Vec<SourcedConclusion<String>>,
    pub dependencies: Option<SourcedConclusion<GitHubDependencySummary>>,
    pub license: Option<SourcedConclusion<GitHubLicenseSummary>>,
    pub risks: Vec<SourcedConclusion<String>>,
    pub recommended_reading_order: Vec<SourcedConclusion<String>>,
    pub reproduction_assessment: SourcedConclusion<GitHubReproductionAssessment>,
    pub extension_points: Vec<SourcedConclusion<String>>,
    #[serde(default)]
    pub recent_activity: Vec<GitHubRecentActivity>,
    #[serde(default)]
    pub section_availability: BTreeMap<String, GitHubSectionAvailability>,
    pub user_question_answer: Option<SourcedConclusion<String>>,
    pub completeness: GitHubAnalysisCompleteness,
    pub quality_warnings: Vec<GitHubAnalysisWarning>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubAnalysisSource {
    pub id: String,
    pub analysis_run_id: String,
    pub source_type: GitHubAnalysisSourceType,
    pub source_path: String,
    pub source_sha: Option<String>,
    pub content_hash: String,
    pub excerpt: String,
    pub source_order: i64,
}

impl GitHubAnalysisSource {
    pub fn validate(&self) -> Result<(), DomainError> {
        let path = self.source_path.replace('\\', "/");
        if self.id.trim().is_empty()
            || self.analysis_run_id.trim().is_empty()
            || self.content_hash.len() != 64
            || self.excerpt.chars().count() > 8_000
            || self.source_order < 0
            || path.starts_with('/')
            || path.contains(':')
            || path.split('/').any(|part| part == "..")
        {
            return Err(DomainError::InvalidGitHubAnalysis("invalid source".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitHubAnalysisRun {
    pub id: String,
    pub workspace_id: String,
    pub github_repository_id: String,
    pub owner: String,
    pub name: String,
    pub repository_url: String,
    pub default_branch: String,
    pub source_commit_sha: String,
    pub focus_type: GitHubAnalysisFocus,
    pub custom_prompt: Option<String>,
    pub status: GitHubAnalysisStatus,
    pub stage: Option<GitHubAnalysisStage>,
    pub analysis_mode: GitHubAnalysisMode,
    pub ai_provider_id: Option<String>,
    pub ai_model: Option<String>,
    pub readiness: GitHubAnalysisReadiness,
    pub analyzer_revision: String,
    pub request_hash: String,
    pub report_json: Option<String>,
    pub report_markdown: Option<String>,
    pub content_hash: Option<String>,
    pub tree_entries_seen: i64,
    pub files_read: i64,
    pub files_skipped: i64,
    pub source_bytes: i64,
    pub api_requests_used: i64,
    pub api_requests_remaining: Option<i64>,
    pub api_rate_limit_reset_at: Option<DateTime<Utc>>,
    pub ai_invoked: bool,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

impl GitHubAnalysisRun {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: String,
        workspace_id: String,
        repository_id: String,
        owner: String,
        name: String,
        repository_url: String,
        default_branch: String,
        head_sha: String,
        focus_type: GitHubAnalysisFocus,
        custom_prompt: Option<String>,
        analysis_mode: GitHubAnalysisMode,
        ai_provider_id: Option<String>,
        ai_model: Option<String>,
        analyzer_revision: String,
        request_hash: String,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if id.trim().is_empty()
            || workspace_id.trim().is_empty()
            || repository_id.trim().is_empty()
            || owner.trim().is_empty()
            || name.trim().is_empty()
            || default_branch.trim().is_empty()
            || !(7..=64).contains(&head_sha.len())
            || request_hash.len() != 64
            || custom_prompt
                .as_ref()
                .is_some_and(|value| value.chars().count() > 2_000)
            || (analysis_mode == GitHubAnalysisMode::Ai
                && (ai_provider_id.as_deref().is_none_or(str::is_empty)
                    || ai_model.as_deref().is_none_or(str::is_empty)))
            || (analysis_mode == GitHubAnalysisMode::Rules
                && (ai_provider_id.is_some() || ai_model.is_some()))
        {
            return Err(DomainError::InvalidGitHubAnalysis(
                "invalid analysis request".into(),
            ));
        }
        Ok(Self {
            id,
            workspace_id,
            github_repository_id: repository_id,
            owner,
            name,
            repository_url,
            default_branch,
            source_commit_sha: head_sha,
            focus_type,
            custom_prompt,
            status: GitHubAnalysisStatus::Queued,
            stage: Some(GitHubAnalysisStage::Resolving),
            analysis_mode,
            ai_provider_id,
            ai_model,
            readiness: GitHubAnalysisReadiness::Incomplete,
            analyzer_revision,
            request_hash,
            report_json: None,
            report_markdown: None,
            content_hash: None,
            tree_entries_seen: 0,
            files_read: 0,
            files_skipped: 0,
            source_bytes: 0,
            api_requests_used: 2,
            api_requests_remaining: None,
            api_rate_limit_reset_at: None,
            ai_invoked: false,
            error_code: None,
            error_summary: None,
            started_at: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
            version: 1,
        })
    }

    pub fn set_progress(
        &mut self,
        status: GitHubAnalysisStatus,
        stage: GitHubAnalysisStage,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let valid = matches!(
            (self.status, status),
            (
                GitHubAnalysisStatus::Queued,
                GitHubAnalysisStatus::Collecting
            ) | (
                GitHubAnalysisStatus::Collecting,
                GitHubAnalysisStatus::Collecting
            ) | (
                GitHubAnalysisStatus::Collecting,
                GitHubAnalysisStatus::Analyzing
            ) | (
                GitHubAnalysisStatus::Analyzing,
                GitHubAnalysisStatus::Analyzing
            )
        );
        if !valid {
            return Err(DomainError::InvalidGitHubAnalysis(
                "invalid status transition".into(),
            ));
        }
        self.status = status;
        self.stage = Some(stage);
        self.started_at.get_or_insert(now);
        self.bump(now);
        Ok(())
    }

    pub fn finish(
        &mut self,
        report: &GitHubAnalysisReport,
        markdown: String,
        hash: String,
        status: GitHubAnalysisStatus,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status != GitHubAnalysisStatus::Analyzing
            || !matches!(
                status,
                GitHubAnalysisStatus::Ready | GitHubAnalysisStatus::Partial
            )
            || hash.len() != 64
        {
            return Err(DomainError::InvalidGitHubAnalysis(
                "invalid completion".into(),
            ));
        }
        self.report_json = Some(serde_json::to_string(report).map_err(|_| {
            DomainError::InvalidGitHubAnalysis("report serialization failed".into())
        })?);
        self.report_markdown = Some(markdown);
        self.content_hash = Some(hash);
        self.status = status;
        self.stage = None;
        self.readiness = report.completeness.readiness;
        self.completed_at = Some(now);
        self.error_code = None;
        self.error_summary = None;
        self.bump(now);
        Ok(())
    }

    pub fn fail(
        &mut self,
        code: String,
        summary: String,
        readiness: GitHubAnalysisReadiness,
        now: DateTime<Utc>,
    ) {
        self.status = GitHubAnalysisStatus::Failed;
        self.stage = None;
        self.readiness = readiness;
        self.error_code = Some(code);
        self.error_summary = Some(summary.chars().take(500).collect());
        self.completed_at = Some(now);
        self.bump(now);
    }

    pub fn archive(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if !matches!(
            self.status,
            GitHubAnalysisStatus::Ready
                | GitHubAnalysisStatus::Partial
                | GitHubAnalysisStatus::Failed
        ) {
            return Err(DomainError::InvalidGitHubAnalysis(
                "running analysis cannot be archived".into(),
            ));
        }
        self.status = GitHubAnalysisStatus::Archived;
        self.stage = None;
        self.bump(now);
        Ok(())
    }

    fn bump(&mut self, now: DateTime<Utc>) {
        self.updated_at = now.max(self.updated_at);
        self.version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> GitHubAnalysisRun {
        GitHubAnalysisRun::create(
            "run-1".into(),
            "workspace-1".into(),
            "repository-1".into(),
            "owner".into(),
            "repository".into(),
            "https://github.com/owner/repository".into(),
            "main".into(),
            "0123456789abcdef".into(),
            GitHubAnalysisFocus::Overview,
            None,
            GitHubAnalysisMode::Rules,
            None,
            None,
            "github-rules-v1".into(),
            "a".repeat(64),
            Utc::now(),
        )
        .expect("valid run")
    }

    #[test]
    fn analysis_lifecycle_rejects_skips_and_preserves_terminal_history() {
        let mut run = run();
        let now = Utc::now();
        assert!(run
            .set_progress(
                GitHubAnalysisStatus::Analyzing,
                GitHubAnalysisStage::RulesAnalysis,
                now,
            )
            .is_err());
        run.set_progress(
            GitHubAnalysisStatus::Collecting,
            GitHubAnalysisStage::CollectingTree,
            now,
        )
        .expect("collecting");
        assert!(run.archive(now).is_err());
        run.fail(
            "github_analysis_source_unavailable".into(),
            "read failed".into(),
            GitHubAnalysisReadiness::Incomplete,
            now,
        );
        run.archive(now).expect("terminal run can be archived");
        assert_eq!(run.status, GitHubAnalysisStatus::Archived);
    }

    #[test]
    fn ai_analysis_requires_provider_and_model_while_rules_forbid_them() {
        let create = |mode, provider, model| {
            GitHubAnalysisRun::create(
                "run-ai".into(),
                "workspace".into(),
                "repository".into(),
                "owner".into(),
                "sample".into(),
                "https://github.com/owner/sample".into(),
                "main".into(),
                "0123456789abcdef".into(),
                GitHubAnalysisFocus::Overview,
                None,
                mode,
                provider,
                model,
                "revision".into(),
                "b".repeat(64),
                Utc::now(),
            )
        };
        assert!(create(GitHubAnalysisMode::Ai, None, None).is_err());
        assert!(create(
            GitHubAnalysisMode::Ai,
            Some("provider-1".into()),
            Some("model-1".into())
        )
        .is_ok());
        assert!(create(
            GitHubAnalysisMode::Rules,
            Some("provider-1".into()),
            Some("model-1".into())
        )
        .is_err());
    }

    #[test]
    fn source_requires_a_safe_relative_path_and_bounded_excerpt() {
        let mut source = GitHubAnalysisSource {
            id: "source-1".into(),
            analysis_run_id: "run-1".into(),
            source_type: GitHubAnalysisSourceType::Readme,
            source_path: "README.md".into(),
            source_sha: Some("abc".into()),
            content_hash: "b".repeat(64),
            excerpt: "safe".into(),
            source_order: 0,
        };
        source.validate().expect("relative source");
        source.source_path = "C:/secret/README.md".into();
        assert!(source.validate().is_err());
        source.source_path = "README.md".into();
        source.excerpt = "x".repeat(8_001);
        assert!(source.validate().is_err());
    }
}
