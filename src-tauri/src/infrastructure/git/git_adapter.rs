use std::{io, process::Stdio, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::{process::Command, time::timeout};

use crate::{
    domain::{validate_commit_sha, DiffSummary},
    ports::{GitAdapter, GitAdapterError, GitCommit, GitCommitHistory, GitInspection},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const LOG_FORMAT: &str = "%H%x1f%s%x1f%an%x1f%ae%x1f%aI%x1e";

#[derive(Default)]
pub struct GitCliAdapter;

#[async_trait]
impl GitAdapter for GitCliAdapter {
    async fn inspect(&self, path: &str) -> Result<GitInspection, GitAdapterError> {
        let inside = text(
            self.run_required(path, &["rev-parse", "--is-inside-work-tree"])
                .await?,
        )?;
        if inside.trim() != "true" {
            return Err(GitAdapterError::NotRepository);
        }

        let root_path = text(
            self.run_required(path, &["rev-parse", "--show-toplevel"])
                .await?,
        )?
        .trim()
        .to_owned();
        let head_commit = text(
            self.run_required(&root_path, &["rev-parse", "--verify", "HEAD"])
                .await
                .map_err(|error| match error {
                    GitAdapterError::CommandFailed(_) => GitAdapterError::HeadUnavailable,
                    other => other,
                })?,
        )?;
        let head_commit = validate_commit_sha(head_commit.trim())
            .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;

        let branch = match self
            .run_optional(&root_path, &["symbolic-ref", "--quiet", "--short", "HEAD"])
            .await?
        {
            Some(output) => text(output)?.trim().to_owned(),
            None => "HEAD (detached)".to_owned(),
        };
        let remote_url = self
            .run_optional(&root_path, &["remote", "get-url", "origin"])
            .await?
            .map(text)
            .transpose()?
            .map(|value| redact_remote_url(value.trim()))
            .filter(|value| !value.is_empty());
        let default_branch = match self
            .run_optional(
                &root_path,
                &[
                    "symbolic-ref",
                    "--quiet",
                    "--short",
                    "refs/remotes/origin/HEAD",
                ],
            )
            .await?
        {
            Some(output) => {
                let value = text(output)?;
                value
                    .trim()
                    .strip_prefix("origin/")
                    .unwrap_or(value.trim())
                    .to_owned()
            }
            None if branch != "HEAD (detached)" => branch.clone(),
            None => "HEAD".to_owned(),
        };

        let status = self
            .run_required(
                &root_path,
                &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
            )
            .await?;
        let (is_dirty, mut diff_summary) = parse_status(&status)?;
        let numstat = text(
            self.run_required(&root_path, &["diff", "--numstat", "HEAD", "--"])
                .await?,
        )?;
        let (insertions, deletions) = parse_numstat(&numstat)?;
        diff_summary.insertions = insertions;
        diff_summary.deletions = deletions;

        let log = text(
            self.run_required(
                &root_path,
                &[
                    "log",
                    "-n",
                    "10",
                    "--date=iso-strict",
                    &format!("--format={LOG_FORMAT}"),
                ],
            )
            .await?,
        )?;
        let recent_commits = parse_commits(&log)?;
        let upstream_branch = self
            .run_optional(
                &root_path,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    "@{upstream}",
                ],
            )
            .await?
            .map(text)
            .transpose()?
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let (ahead_count, behind_count) = if upstream_branch.is_some() {
            match self
                .run_optional(
                    &root_path,
                    &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
                )
                .await?
            {
                Some(output) => {
                    let (ahead, behind) = parse_ahead_behind(&text(output)?)?;
                    (Some(ahead), Some(behind))
                }
                None => (None, None),
            }
        } else {
            (None, None)
        };

        Ok(GitInspection {
            root_path,
            remote_url,
            default_branch,
            branch,
            head_commit,
            is_dirty,
            diff_summary,
            upstream_branch,
            ahead_count,
            behind_count,
            checked_at: Utc::now(),
            recent_commits,
        })
    }

    async fn get_commit(&self, path: &str, sha: &str) -> Result<GitCommit, GitAdapterError> {
        let sha = validate_commit_sha(sha)
            .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
        let output = text(
            self.run_required(
                path,
                &[
                    "show",
                    "-s",
                    "--date=iso-strict",
                    &format!("--format={LOG_FORMAT}"),
                    &sha,
                    "--",
                ],
            )
            .await?,
        )?;
        parse_commits(&output)?
            .into_iter()
            .next()
            .ok_or_else(|| GitAdapterError::InvalidOutput("commit record is empty".into()))
    }

    async fn list_commit_history(
        &self,
        path: &str,
        previous_head: Option<&str>,
        since: DateTime<Utc>,
        limit: usize,
    ) -> Result<GitCommitHistory, GitAdapterError> {
        let head = text(
            self.run_required(path, &["rev-parse", "--verify", "HEAD"])
                .await?,
        )?;
        let head_commit = validate_commit_sha(head.trim())
            .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
        let previous_head = previous_head
            .map(validate_commit_sha)
            .transpose()
            .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
        if previous_head.as_deref() == Some(head_commit.as_str()) {
            return Ok(GitCommitHistory {
                head_commit,
                commits: vec![],
                full_rescan: false,
            });
        }

        let incremental = if let Some(previous) = previous_head.as_deref() {
            self.run_optional(path, &["merge-base", "--is-ancestor", previous, "HEAD"])
                .await?
                .is_some()
        } else {
            false
        };
        let limit = limit.clamp(1, 2_000).to_string();
        let format = format!("--format={LOG_FORMAT}");
        let output = if incremental {
            let range = format!("{}..HEAD", previous_head.as_deref().unwrap_or_default());
            self.run_required(
                path,
                &["log", "-n", &limit, "--date=iso-strict", &format, &range],
            )
            .await?
        } else {
            let since = format!("--since={}", since.to_rfc3339());
            self.run_required(
                path,
                &[
                    "log",
                    "-n",
                    &limit,
                    "--date=iso-strict",
                    &since,
                    &format,
                    "HEAD",
                ],
            )
            .await?
        };
        Ok(GitCommitHistory {
            head_commit,
            commits: parse_commits(&text(output)?)?,
            full_rescan: !incremental,
        })
    }
}

impl GitCliAdapter {
    async fn run_required(&self, path: &str, args: &[&str]) -> Result<Vec<u8>, GitAdapterError> {
        self.run(path, args)
            .await?
            .ok_or_else(|| GitAdapterError::CommandFailed("command returned no result".into()))
    }

    async fn run_optional(
        &self,
        path: &str,
        args: &[&str],
    ) -> Result<Option<Vec<u8>>, GitAdapterError> {
        self.run_with_optional_failure(path, args, true).await
    }

    async fn run(&self, path: &str, args: &[&str]) -> Result<Option<Vec<u8>>, GitAdapterError> {
        self.run_with_optional_failure(path, args, false).await
    }

    async fn run_with_optional_failure(
        &self,
        path: &str,
        args: &[&str],
        optional_failure: bool,
    ) -> Result<Option<Vec<u8>>, GitAdapterError> {
        let mut command = Command::new("git");
        command
            .arg("--no-optional-locks")
            .args(["-c", "core.quotepath=false", "-C", path])
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_window(&mut command);

        let output = timeout(COMMAND_TIMEOUT, command.output())
            .await
            .map_err(|_| GitAdapterError::Timeout)?
            .map_err(map_spawn_error)?;
        if output.status.success() {
            return Ok(Some(output.stdout));
        }
        if optional_failure {
            return Ok(None);
        }
        let message = String::from_utf8_lossy(&output.stderr);
        let message = message.trim();
        if args.starts_with(&["rev-parse", "--is-inside-work-tree"])
            || message.contains("not a git repository")
        {
            return Err(GitAdapterError::NotRepository);
        }
        Err(GitAdapterError::CommandFailed(
            message.chars().take(1_000).collect(),
        ))
    }
}

fn parse_status(bytes: &[u8]) -> Result<(bool, DiffSummary), GitAdapterError> {
    let mut summary = DiffSummary::default();
    let records = bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .collect::<Vec<_>>();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        if record.len() < 3 {
            return Err(GitAdapterError::InvalidOutput(
                "porcelain status record is short".into(),
            ));
        }
        let x = record[0] as char;
        let y = record[1] as char;
        summary.changed_files += 1;
        if x == '?' && y == '?' {
            summary.untracked_files += 1;
        } else {
            if x != ' ' {
                summary.staged_files += 1;
            }
            if y != ' ' {
                summary.unstaged_files += 1;
            }
        }
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            index += 1;
            if index >= records.len() {
                return Err(GitAdapterError::InvalidOutput(
                    "rename status lacks source path".into(),
                ));
            }
        }
        index += 1;
    }
    Ok((!records.is_empty(), summary))
}

fn parse_numstat(output: &str) -> Result<(i64, i64), GitAdapterError> {
    let mut insertions = 0_i64;
    let mut deletions = 0_i64;
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let mut fields = line.splitn(3, '\t');
        let added = fields.next().unwrap_or_default();
        let removed = fields.next().unwrap_or_default();
        if fields.next().is_none() {
            return Err(GitAdapterError::InvalidOutput(
                "numstat record is incomplete".into(),
            ));
        }
        if added != "-" {
            insertions += added
                .parse::<i64>()
                .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
        }
        if removed != "-" {
            deletions += removed
                .parse::<i64>()
                .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
        }
    }
    Ok((insertions, deletions))
}

fn parse_ahead_behind(output: &str) -> Result<(i64, i64), GitAdapterError> {
    let fields = output.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 {
        return Err(GitAdapterError::InvalidOutput(
            "upstream count record is invalid".into(),
        ));
    }
    let ahead = fields[0]
        .parse::<i64>()
        .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
    let behind = fields[1]
        .parse::<i64>()
        .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?;
    if ahead < 0 || behind < 0 {
        return Err(GitAdapterError::InvalidOutput(
            "upstream counts cannot be negative".into(),
        ));
    }
    Ok((ahead, behind))
}

fn parse_commits(output: &str) -> Result<Vec<GitCommit>, GitAdapterError> {
    output
        .split('\x1e')
        .map(str::trim)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let fields = record.split('\x1f').collect::<Vec<_>>();
            if fields.len() != 5 {
                return Err(GitAdapterError::InvalidOutput(
                    "commit field count is invalid".into(),
                ));
            }
            Ok(GitCommit {
                sha: validate_commit_sha(fields[0])
                    .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?,
                message: fields[1].trim().to_owned(),
                author_name: fields[2].trim().to_owned(),
                author_email: fields[3].trim().to_owned(),
                authored_at: DateTime::parse_from_rfc3339(fields[4].trim())
                    .map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))?
                    .with_timezone(&Utc),
            })
        })
        .collect()
}

fn text(bytes: Vec<u8>) -> Result<String, GitAdapterError> {
    String::from_utf8(bytes).map_err(|error| GitAdapterError::InvalidOutput(error.to_string()))
}

fn redact_remote_url(value: &str) -> String {
    let Some((scheme, rest)) = value.split_once("://") else {
        return value.to_owned();
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let suffix = &rest[authority_end..];
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://{host}{suffix}"),
        None => value.to_owned(),
    }
}

fn map_spawn_error(error: io::Error) -> GitAdapterError {
    if error.kind() == io::ErrorKind::NotFound {
        GitAdapterError::GitUnavailable(error.to_string())
    } else {
        GitAdapterError::CommandFailed(error.to_string())
    }
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.as_std_mut().creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_numstat_are_machine_parsed() {
        let (dirty, summary) = parse_status(b"M  staged.rs\0 M unstaged.rs\0?? new.rs\0").unwrap();
        assert!(dirty);
        assert_eq!(summary.changed_files, 3);
        assert_eq!(summary.staged_files, 1);
        assert_eq!(summary.unstaged_files, 1);
        assert_eq!(summary.untracked_files, 1);
        assert_eq!(
            parse_numstat("12\t3\tsrc/main.rs\n-\t-\timage.bin\n").unwrap(),
            (12, 3)
        );
        assert_eq!(parse_ahead_behind("2\t4\n").unwrap(), (2, 4));
    }

    #[test]
    fn remote_credentials_are_not_persisted() {
        assert_eq!(
            redact_remote_url("https://token@example.com/org/repo.git"),
            "https://example.com/org/repo.git"
        );
    }

    #[test]
    fn adapter_source_contains_no_git_mutation_subcommands() {
        let source = include_str!("git_adapter.rs");
        for command in [
            "\"commit\"",
            "\"push\"",
            "\"pull\"",
            "\"fetch\"",
            "\"merge\"",
            "\"checkout\"",
            "\"reset\"",
            "\"stash\"",
        ] {
            assert!(
                !source.contains(command),
                "forbidden Git command literal: {command}"
            );
        }
    }

    #[tokio::test]
    async fn inspects_forgeos_checkout_with_read_only_commands() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .display()
            .to_string();
        let inspection = GitCliAdapter.inspect(&root).await.unwrap();
        assert_eq!(inspection.head_commit.len(), 40);
        assert!(!inspection.recent_commits.is_empty());
    }
}
