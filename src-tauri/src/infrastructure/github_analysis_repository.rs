use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::FromRow;

use crate::{
    domain::{
        GitHubAnalysisFocus, GitHubAnalysisMode, GitHubAnalysisReadiness, GitHubAnalysisRun,
        GitHubAnalysisSource, GitHubAnalysisSourceType, GitHubAnalysisStage, GitHubAnalysisStatus,
        Workspace,
    },
    ports::{GitHubAnalysisPage, GitHubAnalysisRepository, RepositoryError, WorkspaceRepository},
};

use super::repositories::SqliteProjectRepository;

const RUN_COLUMNS: &str = "id, workspace_id, github_repository_id, owner, name, repository_url, default_branch, source_commit_sha, focus_type, custom_prompt, status, stage, analysis_mode, readiness, analyzer_revision, request_hash, report_json, report_markdown, content_hash, tree_entries_seen, files_read, files_skipped, source_bytes, api_requests_used, api_requests_remaining, api_rate_limit_reset_at, ai_invoked, error_code, error_summary, started_at, completed_at, created_at, updated_at, version";

#[derive(FromRow)]
struct RunRow {
    id: String,
    workspace_id: String,
    github_repository_id: String,
    owner: String,
    name: String,
    repository_url: String,
    default_branch: String,
    source_commit_sha: String,
    focus_type: String,
    custom_prompt: Option<String>,
    status: String,
    stage: Option<String>,
    analysis_mode: String,
    readiness: String,
    analyzer_revision: String,
    request_hash: String,
    report_json: Option<String>,
    report_markdown: Option<String>,
    content_hash: Option<String>,
    tree_entries_seen: i64,
    files_read: i64,
    files_skipped: i64,
    source_bytes: i64,
    api_requests_used: i64,
    api_requests_remaining: Option<i64>,
    api_rate_limit_reset_at: Option<String>,
    ai_invoked: i64,
    error_code: Option<String>,
    error_summary: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl RunRow {
    fn into_domain(self) -> Result<GitHubAnalysisRun, RepositoryError> {
        Ok(GitHubAnalysisRun {
            id: self.id,
            workspace_id: self.workspace_id,
            github_repository_id: self.github_repository_id,
            owner: self.owner,
            name: self.name,
            repository_url: self.repository_url,
            default_branch: self.default_branch,
            source_commit_sha: self.source_commit_sha,
            focus_type: GitHubAnalysisFocus::from_str(&self.focus_type).map_err(corrupt)?,
            custom_prompt: self.custom_prompt,
            status: GitHubAnalysisStatus::from_str(&self.status).map_err(corrupt)?,
            stage: self
                .stage
                .map(|value| GitHubAnalysisStage::from_str(&value).map_err(corrupt))
                .transpose()?,
            analysis_mode: GitHubAnalysisMode::from_str(&self.analysis_mode).map_err(corrupt)?,
            readiness: GitHubAnalysisReadiness::from_str(&self.readiness).map_err(corrupt)?,
            analyzer_revision: self.analyzer_revision,
            request_hash: self.request_hash,
            report_json: self.report_json,
            report_markdown: self.report_markdown,
            content_hash: self.content_hash,
            tree_entries_seen: self.tree_entries_seen,
            files_read: self.files_read,
            files_skipped: self.files_skipped,
            source_bytes: self.source_bytes,
            api_requests_used: self.api_requests_used,
            api_requests_remaining: self.api_requests_remaining,
            api_rate_limit_reset_at: optional_time(
                "github_analysis_runs.api_rate_limit_reset_at",
                self.api_rate_limit_reset_at,
            )?,
            ai_invoked: self.ai_invoked != 0,
            error_code: self.error_code,
            error_summary: self.error_summary,
            started_at: optional_time("github_analysis_runs.started_at", self.started_at)?,
            completed_at: optional_time("github_analysis_runs.completed_at", self.completed_at)?,
            created_at: parse_time("github_analysis_runs.created_at", &self.created_at)?,
            updated_at: parse_time("github_analysis_runs.updated_at", &self.updated_at)?,
            version: self.version,
        })
    }
}

#[derive(FromRow)]
struct SourceRow {
    id: String,
    analysis_run_id: String,
    source_type: String,
    source_path: String,
    source_sha: Option<String>,
    content_hash: String,
    excerpt: String,
    source_order: i64,
}

impl SourceRow {
    fn into_domain(self) -> Result<GitHubAnalysisSource, RepositoryError> {
        let source = GitHubAnalysisSource {
            id: self.id,
            analysis_run_id: self.analysis_run_id,
            source_type: GitHubAnalysisSourceType::from_str(&self.source_type).map_err(corrupt)?,
            source_path: self.source_path,
            source_sha: self.source_sha,
            content_hash: self.content_hash,
            excerpt: self.excerpt,
            source_order: self.source_order,
        };
        source.validate().map_err(corrupt)?;
        Ok(source)
    }
}

#[async_trait]
impl GitHubAnalysisRepository for SqliteProjectRepository {
    async fn get_current_workspace(&self) -> Result<Option<Workspace>, RepositoryError> {
        WorkspaceRepository::get_current_workspace(self).await
    }

    async fn get_github_connector_id(&self) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar("SELECT id FROM github_connectors LIMIT 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)
    }

    async fn insert_analysis_run(&self, run: &GitHubAnalysisRun) -> Result<(), RepositoryError> {
        sqlx::query("INSERT INTO github_analysis_runs (id, workspace_id, github_repository_id, owner, name, repository_url, default_branch, source_commit_sha, focus_type, custom_prompt, status, stage, analysis_mode, readiness, analyzer_revision, request_hash, report_json, report_markdown, content_hash, tree_entries_seen, files_read, files_skipped, source_bytes, api_requests_used, api_requests_remaining, api_rate_limit_reset_at, ai_invoked, error_code, error_summary, started_at, completed_at, created_at, updated_at, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&run.id).bind(&run.workspace_id).bind(&run.github_repository_id)
            .bind(&run.owner).bind(&run.name).bind(&run.repository_url)
            .bind(&run.default_branch).bind(&run.source_commit_sha)
            .bind(run.focus_type.as_str()).bind(&run.custom_prompt)
            .bind(run.status.as_str()).bind(run.stage.map(|value| value.as_str()))
            .bind(run.analysis_mode.as_str()).bind(run.readiness.as_str())
            .bind(&run.analyzer_revision).bind(&run.request_hash)
            .bind(&run.report_json).bind(&run.report_markdown).bind(&run.content_hash)
            .bind(run.tree_entries_seen).bind(run.files_read).bind(run.files_skipped)
            .bind(run.source_bytes).bind(run.api_requests_used).bind(run.api_requests_remaining)
            .bind(run.api_rate_limit_reset_at.map(timestamp)).bind(i64::from(run.ai_invoked))
            .bind(&run.error_code).bind(&run.error_summary).bind(run.started_at.map(timestamp))
            .bind(run.completed_at.map(timestamp)).bind(timestamp(run.created_at))
            .bind(timestamp(run.updated_at)).bind(run.version)
            .execute(&self.pool).await.map_err(storage)?;
        Ok(())
    }

    async fn get_analysis_run(
        &self,
        workspace_id: &str,
        run_id: &str,
    ) -> Result<Option<GitHubAnalysisRun>, RepositoryError> {
        sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {RUN_COLUMNS} FROM github_analysis_runs WHERE workspace_id = ? AND id = ?"
        ))
        .bind(workspace_id)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .map(RunRow::into_domain)
        .transpose()
    }

    async fn get_analysis_run_by_hash(
        &self,
        workspace_id: &str,
        request_hash: &str,
    ) -> Result<Option<GitHubAnalysisRun>, RepositoryError> {
        sqlx::query_as::<_, RunRow>(&format!("SELECT {RUN_COLUMNS} FROM github_analysis_runs WHERE workspace_id = ? AND request_hash = ?"))
            .bind(workspace_id).bind(request_hash).fetch_optional(&self.pool).await.map_err(storage)?
            .map(RunRow::into_domain).transpose()
    }

    async fn list_analysis_runs(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<GitHubAnalysisPage, RepositoryError> {
        let limit = limit.clamp(1, 50);
        let rows = if let Some(cursor) = cursor {
            let (created_at, id) = decode_cursor(cursor)?;
            sqlx::query_as::<_, RunRow>(&format!("SELECT {RUN_COLUMNS} FROM github_analysis_runs WHERE workspace_id = ? AND (created_at < ? OR (created_at = ? AND id < ?)) ORDER BY created_at DESC, id DESC LIMIT ?"))
                .bind(workspace_id).bind(&created_at).bind(&created_at).bind(id).bind((limit + 1) as i64)
                .fetch_all(&self.pool).await.map_err(storage)?
        } else {
            sqlx::query_as::<_, RunRow>(&format!("SELECT {RUN_COLUMNS} FROM github_analysis_runs WHERE workspace_id = ? ORDER BY created_at DESC, id DESC LIMIT ?"))
                .bind(workspace_id).bind((limit + 1) as i64).fetch_all(&self.pool).await.map_err(storage)?
        };
        let mut items = rows
            .into_iter()
            .map(RunRow::into_domain)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if items.len() > limit {
            items.truncate(limit);
            items
                .last()
                .map(|item| encode_cursor(item.created_at, &item.id))
        } else {
            None
        };
        Ok(GitHubAnalysisPage { items, next_cursor })
    }

    async fn update_analysis_run(
        &self,
        run: &GitHubAnalysisRun,
        expected_version: i64,
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query("UPDATE github_analysis_runs SET status = ?, stage = ?, readiness = ?, report_json = ?, report_markdown = ?, content_hash = ?, tree_entries_seen = ?, files_read = ?, files_skipped = ?, source_bytes = ?, api_requests_used = ?, api_requests_remaining = ?, api_rate_limit_reset_at = ?, ai_invoked = ?, error_code = ?, error_summary = ?, started_at = ?, completed_at = ?, updated_at = ?, version = ? WHERE id = ? AND workspace_id = ? AND version = ?")
            .bind(run.status.as_str()).bind(run.stage.map(|value| value.as_str())).bind(run.readiness.as_str())
            .bind(&run.report_json).bind(&run.report_markdown).bind(&run.content_hash)
            .bind(run.tree_entries_seen).bind(run.files_read).bind(run.files_skipped).bind(run.source_bytes)
            .bind(run.api_requests_used).bind(run.api_requests_remaining).bind(run.api_rate_limit_reset_at.map(timestamp))
            .bind(i64::from(run.ai_invoked)).bind(&run.error_code).bind(&run.error_summary)
            .bind(run.started_at.map(timestamp)).bind(run.completed_at.map(timestamp))
            .bind(timestamp(run.updated_at)).bind(run.version).bind(&run.id).bind(&run.workspace_id).bind(expected_version)
            .execute(&self.pool).await.map_err(storage)?;
        if result.rows_affected() != 1 {
            return Err(RepositoryError::OptimisticConflict);
        }
        Ok(())
    }

    async fn replace_analysis_sources(
        &self,
        run_id: &str,
        sources: &[GitHubAnalysisSource],
    ) -> Result<(), RepositoryError> {
        let mut transaction = self.pool.begin().await.map_err(storage)?;
        sqlx::query("DELETE FROM github_analysis_sources WHERE analysis_run_id = ?")
            .bind(run_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
        for source in sources {
            source.validate().map_err(corrupt)?;
            sqlx::query("INSERT INTO github_analysis_sources (id, analysis_run_id, source_type, source_path, source_sha, content_hash, excerpt, source_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(&source.id).bind(&source.analysis_run_id).bind(source.source_type.as_str())
                .bind(&source.source_path).bind(&source.source_sha).bind(&source.content_hash)
                .bind(&source.excerpt).bind(source.source_order).execute(&mut *transaction).await.map_err(storage)?;
        }
        transaction.commit().await.map_err(storage)?;
        Ok(())
    }

    async fn list_analysis_sources(
        &self,
        run_id: &str,
    ) -> Result<Vec<GitHubAnalysisSource>, RepositoryError> {
        sqlx::query_as::<_, SourceRow>("SELECT id, analysis_run_id, source_type, source_path, source_sha, content_hash, excerpt, source_order FROM github_analysis_sources WHERE analysis_run_id = ? ORDER BY source_order, id")
            .bind(run_id).fetch_all(&self.pool).await.map_err(storage)?
            .into_iter().map(SourceRow::into_domain).collect()
    }

    async fn recover_interrupted_analyses(
        &self,
        now: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query("UPDATE github_analysis_runs SET status = 'failed', stage = NULL, readiness = 'incomplete', error_code = 'github_analysis_interrupted', error_summary = 'ForgeOS closed before analysis completed', completed_at = ?, updated_at = ?, version = version + 1 WHERE status IN ('queued', 'collecting', 'analyzing')")
            .bind(timestamp(now)).bind(timestamp(now)).execute(&self.pool).await.map_err(storage)?;
        Ok(result.rows_affected())
    }

    async fn has_newer_repository_head(
        &self,
        workspace_id: &str,
        repository_id: &str,
        head_sha: &str,
        created_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM github_analysis_runs WHERE workspace_id = ? AND github_repository_id = ? AND source_commit_sha <> ? AND created_at > ? AND status IN ('ready', 'partial')")
            .bind(workspace_id).bind(repository_id).bind(head_sha).bind(timestamp(created_at)).fetch_one(&self.pool).await.map_err(storage)?;
        Ok(count > 0)
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
fn parse_time(field: &str, value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| RepositoryError::CorruptData(format!("{field}: {error}")))
}
fn optional_time(
    field: &str,
    value: Option<String>,
) -> Result<Option<DateTime<Utc>>, RepositoryError> {
    value.map(|value| parse_time(field, &value)).transpose()
}
fn corrupt(error: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::CorruptData(error.to_string())
}
fn storage(error: sqlx::Error) -> RepositoryError {
    match &error {
        sqlx::Error::Database(value) if value.is_unique_violation() => RepositoryError::Conflict,
        _ => RepositoryError::Storage(error.to_string()),
    }
}
fn encode_cursor(created_at: DateTime<Utc>, id: &str) -> String {
    format!("{}\0{id}", timestamp(created_at))
        .bytes()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn decode_cursor(value: &str) -> Result<(String, String), RepositoryError> {
    if value.len() % 2 != 0 {
        return Err(RepositoryError::CorruptData(
            "invalid analysis cursor".into(),
        ));
    }
    let bytes = (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| RepositoryError::CorruptData("invalid analysis cursor".into()))?;
    let decoded = String::from_utf8(bytes)
        .map_err(|_| RepositoryError::CorruptData("invalid analysis cursor".into()))?;
    let (created_at, id) = decoded
        .split_once('\0')
        .ok_or_else(|| RepositoryError::CorruptData("invalid analysis cursor".into()))?;
    parse_time("analysis cursor", created_at)?;
    Ok((created_at.to_owned(), id.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_round_trips_unicode_safe_identity() {
        let now = Utc::now();
        let encoded = encode_cursor(now, "分析-1");
        let (_, id) = decode_cursor(&encoded).expect("cursor");
        assert_eq!(id, "分析-1");
    }
}
