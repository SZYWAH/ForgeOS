use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    time::timeout,
};

use crate::ports::{
    redact_external_text, CodexActivityBatch, CodexActivitySource, CodexActivitySourceError,
    CodexConnectorHealth, CodexTurnActivity,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const ALLOWED_METHODS: &[&str] = &[
    "initialize",
    "thread/list",
    "thread/read",
    "thread/goal/get",
];

#[derive(Default)]
pub struct CodexAppServerAdapter;

#[async_trait]
impl CodexActivitySource for CodexAppServerAdapter {
    async fn health_check(
        &self,
        configured_executable: Option<&str>,
    ) -> Result<CodexConnectorHealth, CodexActivitySourceError> {
        let executable =
            discover_executable(configured_executable).ok_or(CodexActivitySourceError::NotFound)?;
        let mut command = Command::new(&executable);
        command
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_window(&mut command);
        let output = timeout(REQUEST_TIMEOUT, command.output())
            .await
            .map_err(|_| CodexActivitySourceError::Timeout)?
            .map_err(map_spawn_error)?;
        if !output.status.success() {
            return Err(CodexActivitySourceError::Process(redact_external_text(
                &String::from_utf8_lossy(&output.stderr),
            )));
        }
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .chars()
            .take(256)
            .collect::<String>();
        Ok(CodexConnectorHealth {
            available: true,
            executable_path: executable.to_str().map(str::to_owned),
            version: Some(version),
            message: "Codex App Server 可用".into(),
        })
    }

    async fn read_repository_activity(
        &self,
        configured_executable: Option<&str>,
        repository_path: &Path,
        updated_after: Option<DateTime<Utc>>,
        max_threads: usize,
        max_turns: usize,
        max_normalized_bytes: usize,
    ) -> Result<CodexActivityBatch, CodexActivitySourceError> {
        let executable =
            discover_executable(configured_executable).ok_or(CodexActivitySourceError::NotFound)?;
        let canonical_repository = fs::canonicalize(repository_path)
            .map_err(|error| CodexActivitySourceError::Process(error.to_string()))?;
        let mut server = AppServerSession::start(&executable).await?;
        server.request("initialize", json!({ "clientInfo": { "name": "forgeos", "version": env!("CARGO_PKG_VERSION") }, "capabilities": {} })).await?;
        server.notify("initialized", json!({})).await?;

        let list = server
            .request(
                "thread/list",
                json!({
                    "cwd": canonical_repository.to_string_lossy(),
                    "limit": max_threads.min(50),
                    "sortKey": "updated_at",
                    "sourceKinds": ["cli", "vscode"],
                    "archived": false,
                    "useStateDbOnly": true
                }),
            )
            .await?;
        let mut threads = list
            .get("data")
            .or_else(|| list.get("threads"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        threads.retain(|thread| thread_is_in_scope(thread, &canonical_repository, updated_after));
        threads.truncate(max_threads.min(50));

        let mut activities = Vec::new();
        let mut turns_seen = 0usize;
        let mut normalized_bytes = 0usize;
        let mut latest = None;
        let mut partial_reason = None;

        for thread in &threads {
            if turns_seen >= max_turns || normalized_bytes >= max_normalized_bytes {
                partial_reason = Some("Codex 首次回溯达到安全上限，已保存可用的部分结果。".into());
                break;
            }
            let Some(thread_id) = string_at(thread, &["id", "threadId"]) else {
                continue;
            };
            let source_kind = source_kind(thread);
            let thread_updated =
                datetime_at(thread, &["updatedAt", "updated_at"]).unwrap_or_else(Utc::now);
            latest = Some(latest.map_or(thread_updated, |value: DateTime<Utc>| {
                value.max(thread_updated)
            }));
            let read = server
                .request(
                    "thread/read",
                    json!({ "threadId": thread_id, "includeTurns": true }),
                )
                .await?;
            let thread_value = read.get("thread").unwrap_or(&read);
            let goal = string_at(thread_value, &["goal", "title"]).unwrap_or_default();
            let goal = if goal.trim().is_empty() {
                server
                    .request_optional("thread/goal/get", json!({ "threadId": thread_id }))
                    .await
                    .and_then(|value| string_at(&value, &["goal", "title", "text"]))
                    .unwrap_or_default()
            } else {
                goal
            };
            let turns = thread_value
                .get("turns")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for turn in turns {
                if turns_seen >= max_turns || normalized_bytes >= max_normalized_bytes {
                    partial_reason =
                        Some("Codex 首次回溯达到安全上限，已保存可用的部分结果。".into());
                    break;
                }
                turns_seen += 1;
                let Some(turn_id) = string_at(&turn, &["id", "turnId"]) else {
                    continue;
                };
                let activity = normalize_turn(
                    &thread_id,
                    &turn_id,
                    &source_kind,
                    &goal,
                    thread_updated,
                    &turn,
                    &canonical_repository,
                );
                let byte_count = serde_json::to_vec(&json!({
                    "g": activity.goal_hint, "a": activity.activity_summary,
                    "o": activity.outcome_summary, "v": activity.validation_summary,
                    "h": activity.handoff_summary, "b": activity.blockers
                }))
                .map(|bytes| bytes.len())
                .unwrap_or_default();
                if normalized_bytes.saturating_add(byte_count) > max_normalized_bytes {
                    partial_reason = Some("Codex 结构化摘要达到 2MB 安全上限。".into());
                    break;
                }
                normalized_bytes += byte_count;
                activities.push(activity);
            }
        }
        server.close().await;
        Ok(CodexActivityBatch {
            threads_scanned: threads.len(),
            turns_seen,
            activities,
            latest_thread_updated_at: latest,
            partial_reason,
        })
    }
}

struct AppServerSession {
    child: Child,
    client: JsonlClient<ChildStdin, ChildStdout>,
}

struct JsonlClient<W, R> {
    stdin: W,
    stdout: Lines<BufReader<R>>,
    next_id: u64,
    request_timeout: Duration,
}

impl AppServerSession {
    async fn start(executable: &Path) -> Result<Self, CodexActivitySourceError> {
        let mut command = Command::new(executable);
        command
            .args(["app-server", "--listen", "stdio://"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        hide_window(&mut command);
        let mut child = command.spawn().map_err(map_spawn_error)?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| CodexActivitySourceError::Process("Codex stdin unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| CodexActivitySourceError::Process("Codex stdout unavailable".into()))?;
        if let Some(stderr) = child.stderr.take() {
            drain_stderr(stderr);
        }
        Ok(Self {
            child,
            client: JsonlClient::new(stdin, stdout, REQUEST_TIMEOUT),
        })
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, CodexActivitySourceError> {
        self.client.request(method, params).await
    }

    async fn request_optional(&mut self, method: &str, params: Value) -> Option<Value> {
        self.request(method, params).await.ok()
    }
    async fn notify(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), CodexActivitySourceError> {
        self.client.notify(method, params).await
    }
    async fn close(mut self) {
        let _ = self.client.stdin.shutdown().await;
        let _ = timeout(Duration::from_secs(2), self.child.wait()).await;
        let _ = self.child.kill().await;
    }
}

impl<W, R> JsonlClient<W, R>
where
    W: AsyncWrite + Unpin,
    R: AsyncRead + Unpin,
{
    fn new(stdin: W, stdout: R, request_timeout: Duration) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout).lines(),
            next_id: 1,
            request_timeout,
        }
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, CodexActivitySourceError> {
        if !ALLOWED_METHODS.contains(&method) {
            return Err(CodexActivitySourceError::Protocol(format!(
                "method {method} is outside the read-only whitelist"
            )));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "id": id, "method": method, "params": params }))
            .await?;
        loop {
            let line = timeout(self.request_timeout, self.stdout.next_line())
                .await
                .map_err(|_| CodexActivitySourceError::Timeout)?
                .map_err(|error| CodexActivitySourceError::Protocol(error.to_string()))?
                .ok_or_else(|| {
                    CodexActivitySourceError::Process("Codex App Server closed stdout".into())
                })?;
            let message: Value = serde_json::from_str(&line).map_err(|error| {
                CodexActivitySourceError::Protocol(format!("invalid JSON response: {error}"))
            })?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(CodexActivitySourceError::Protocol(redact_external_text(
                    &error.to_string(),
                )));
            }
            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn notify(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), CodexActivitySourceError> {
        self.send(json!({ "method": method, "params": params }))
            .await
    }
    async fn send(&mut self, value: Value) -> Result<(), CodexActivitySourceError> {
        let mut bytes = serde_json::to_vec(&value)
            .map_err(|error| CodexActivitySourceError::Protocol(error.to_string()))?;
        bytes.push(b'\n');
        self.stdin
            .write_all(&bytes)
            .await
            .map_err(|error| CodexActivitySourceError::Process(error.to_string()))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| CodexActivitySourceError::Process(error.to_string()))
    }
}

fn normalize_turn(
    thread_id: &str,
    turn_id: &str,
    source_kind: &str,
    thread_goal: &str,
    updated_at: DateTime<Utc>,
    turn: &Value,
    repository: &Path,
) -> CodexTurnActivity {
    let status = normalized_status(string_at(turn, &["status"]).as_deref());
    let mut user_texts = Vec::new();
    let mut agent_texts = Vec::new();
    let mut validations = Vec::new();
    let mut handoff = Vec::new();
    let mut blockers = Vec::new();
    let items = turn
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in items {
        let item_type = string_at(&item, &["type"]).unwrap_or_default();
        match item_type.as_str() {
            "userMessage" | "user_message" => push_text(&mut user_texts, extract_text(&item)),
            "agentMessage" | "agent_message" => {
                let phase = string_at(&item, &["phase"]).unwrap_or_default();
                let value = extract_text(&item);
                if phase == "final_answer" {
                    push_text(&mut agent_texts, value);
                } else if agent_texts.is_empty() {
                    push_text(&mut agent_texts, value);
                }
            }
            "commandExecution" | "command_execution" => {
                let name = command_name(&item);
                let exit = item
                    .get("exitCode")
                    .or_else(|| item.get("exit_code"))
                    .and_then(Value::as_i64);
                let command_status = string_at(&item, &["status"]).unwrap_or_default();
                if !name.is_empty() {
                    validations.push(format!(
                        "{name}: {}",
                        exit.map(|code| format!("exit {code}"))
                            .unwrap_or_else(|| command_status.clone())
                    ));
                }
                if exit.is_some_and(|code| code != 0) || command_status == "failed" {
                    blockers.push(format!("验证命令 {name} 未通过"));
                }
            }
            "plan" | "planItem" | "plan_item" => {
                if let Some(steps) = item
                    .get("steps")
                    .or_else(|| item.get("items"))
                    .and_then(Value::as_array)
                {
                    for step in steps {
                        let step_status = string_at(step, &["status"]).unwrap_or_default();
                        if step_status != "completed" {
                            push_text(&mut handoff, extract_text(step));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if matches!(status.as_str(), "failed" | "interrupted") {
        blockers.push(format!("Codex 轮次状态为 {status}"));
    }
    let goal = user_texts
        .first()
        .cloned()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| thread_goal.to_owned());
    let activity = agent_texts.first().cloned().unwrap_or_default();
    let outcome = agent_texts
        .last()
        .filter(|value| agent_texts.len() > 1 && !same_summary(&activity, value))
        .cloned()
        .unwrap_or_default();
    CodexTurnActivity {
        thread_id: thread_id.to_owned(),
        turn_id: turn_id.to_owned(),
        source_kind: source_kind.to_owned(),
        status,
        goal_hint: limited_redacted(&goal, repository, 2_000),
        activity_summary: limited_redacted(&activity, repository, 4_000),
        outcome_summary: limited_redacted(&outcome, repository, 4_000),
        validation_summary: limited_redacted(&validations.join("\n"), repository, 4_000),
        handoff_summary: limited_redacted(&handoff.join("\n"), repository, 4_000),
        blockers: blockers
            .into_iter()
            .map(|value| limited_redacted(&value, repository, 2_000))
            .collect(),
        source_updated_at: datetime_at(
            turn,
            &["updatedAt", "updated_at", "completedAt", "completed_at"],
        )
        .unwrap_or(updated_at),
    }
}

fn same_summary(left: &str, right: &str) -> bool {
    let normalize = |value: &str| {
        value
            .chars()
            .filter(|character| character.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let left = normalize(left);
    let right = normalize(right);
    !left.is_empty() && left == right
}

fn thread_is_in_scope(
    thread: &Value,
    repository: &Path,
    updated_after: Option<DateTime<Utc>>,
) -> bool {
    if thread
        .get("parentThreadId")
        .or_else(|| thread.get("parent_thread_id"))
        .is_some_and(|value| !value.is_null())
    {
        return false;
    }
    let source = source_kind(thread).to_ascii_lowercase();
    if source.contains("subagent") || source.contains("review") || source.contains("compact") {
        return false;
    }
    let Some(cwd) = string_at(thread, &["cwd"]) else {
        return false;
    };
    if !path_equals(Path::new(&cwd), repository) {
        return false;
    }
    updated_after.is_none_or(|after| {
        datetime_at(thread, &["updatedAt", "updated_at"]).is_none_or(|value| value > after)
    })
}

fn source_kind(value: &Value) -> String {
    value
        .get("sourceKind")
        .or_else(|| value.get("source_kind"))
        .or_else(|| value.get("source"))
        .and_then(|source| {
            source.as_str().map(str::to_owned).or_else(|| {
                source
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
        })
        .unwrap_or_else(|| "interactive".into())
}
fn string_at(value: &Value, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| {
        value
            .get(*field)
            .and_then(|value| value.as_str().map(str::to_owned))
    })
}
fn datetime_at(value: &Value, fields: &[&str]) -> Option<DateTime<Utc>> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(parse_datetime))
}
fn parse_datetime(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|value| {
            DateTime::parse_from_rfc3339(value)
                .ok()
                .map(|value| value.with_timezone(&Utc))
        })
        .or_else(|| {
            value.as_i64().and_then(|value| {
                if value > 10_000_000_000 {
                    Utc.timestamp_millis_opt(value).single()
                } else {
                    Utc.timestamp_opt(value, 0).single()
                }
            })
        })
}
fn extract_text(value: &Value) -> String {
    if let Some(text) = string_at(value, &["text", "content", "message", "title"]) {
        return text;
    }
    value
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| string_at(part, &["text", "content"]))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}
fn command_name(item: &Value) -> String {
    let command = item.get("command").or_else(|| item.get("cmd"));
    let raw = command
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            command
                .and_then(Value::as_array)
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let leaf = raw.split_whitespace().next().unwrap_or_default();
    Path::new(leaf)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(leaf)
        .chars()
        .take(128)
        .collect()
}
fn push_text(output: &mut Vec<String>, value: String) {
    let value = value.trim();
    if !value.is_empty() {
        output.push(value.to_owned());
    }
}
fn normalized_status(value: Option<&str>) -> String {
    match value.unwrap_or_default() {
        "completed" | "success" => "completed",
        "failed" | "error" => "failed",
        "interrupted" | "cancelled" | "canceled" => "interrupted",
        "inProgress" | "in_progress" | "running" => "in_progress",
        _ => "unknown",
    }
    .into()
}
fn limited_redacted(value: &str, repository: &Path, max: usize) -> String {
    let repository = fs::canonicalize(repository).unwrap_or_else(|_| repository.to_path_buf());
    let mut normalized = value.to_owned();
    for path in [
        repository.to_string_lossy().to_string(),
        repository.to_string_lossy().replace('\\', "/"),
    ] {
        normalized = replace_ascii_case_insensitive(&normalized, &path, "<repo>");
    }
    redact_external_text(&normalized)
        .chars()
        .take(max)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn replace_ascii_case_insensitive(value: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return value.to_owned();
    }
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    let needle_lower = needle.to_ascii_lowercase();
    loop {
        let lower = remaining.to_ascii_lowercase();
        let Some(index) = lower.find(&needle_lower) else {
            output.push_str(remaining);
            break;
        };
        output.push_str(&remaining[..index]);
        output.push_str(replacement);
        remaining = &remaining[index + needle.len()..];
    }
    output
}
fn path_equals(left: &Path, right: &Path) -> bool {
    fs::canonicalize(left)
        .unwrap_or_else(|_| left.to_path_buf())
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(
            fs::canonicalize(right)
                .unwrap_or_else(|_| right.to_path_buf())
                .to_string_lossy()
                .replace('/', "\\")
                .trim_end_matches('\\'),
        )
}

fn discover_executable(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(path) = configured.map(PathBuf::from).filter(|path| path.is_file()) {
        return Some(path);
    }
    if command_available("codex") {
        return Some(PathBuf::from("codex"));
    }
    let mut candidates = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        let alias = local
            .join("Microsoft")
            .join("WindowsApps")
            .join("codex.exe");
        if alias.is_file() {
            candidates.push(alias);
        }
        let bundled = local.join("OpenAI").join("Codex").join("bin");
        candidates.extend(
            fs::read_dir(bundled)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("codex.exe"))
                .filter(|path| path.is_file()),
        );
    }
    if let Some(program_files) = env::var_os("ProgramFiles").map(PathBuf::from) {
        candidates.extend(
            fs::read_dir(program_files.join("WindowsApps"))
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .starts_with("openai.codex_")
                })
                .map(|entry| entry.path().join("app").join("resources").join("codex.exe"))
                .filter(|path| path.is_file()),
        );
    }
    candidates.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    candidates.pop()
}
fn command_available(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}
fn drain_stderr(stderr: ChildStderr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while lines.next_line().await.ok().flatten().is_some() {}
    });
}
fn map_spawn_error(error: io::Error) -> CodexActivitySourceError {
    if error.kind() == io::ErrorKind::NotFound {
        CodexActivitySourceError::NotFound
    } else {
        CodexActivitySourceError::Process(error.to_string())
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
    use tokio::io::{duplex, split};

    #[test]
    fn only_read_methods_are_whitelisted() {
        assert_eq!(
            ALLOWED_METHODS,
            [
                "initialize",
                "thread/list",
                "thread/read",
                "thread/goal/get"
            ]
        );
        assert!(ALLOWED_METHODS.iter().all(|method| {
            !method.starts_with("turn/")
                && !method.contains("exec")
                && !method.contains("archive")
                && !method.contains("delete")
        }));
    }

    #[tokio::test]
    async fn fake_jsonl_server_round_trips_initialize() {
        let (client_side, server_side) = duplex(4_096);
        let (client_read, client_write) = split(client_side);
        let (server_read, mut server_write) = split(server_side);
        let fake_server = tokio::spawn(async move {
            let mut lines = BufReader::new(server_read).lines();
            let request: Value =
                serde_json::from_str(&lines.next_line().await.unwrap().expect("request line"))
                    .unwrap();
            assert_eq!(request["method"], "initialize");
            server_write
                .write_all(b"{\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"fake\"}}}\n")
                .await
                .unwrap();
        });
        let mut client = JsonlClient::new(client_write, client_read, Duration::from_millis(250));

        let response = client.request("initialize", json!({})).await.unwrap();

        assert_eq!(response["serverInfo"]["name"], "fake");
        fake_server.await.unwrap();
    }

    #[tokio::test]
    async fn fake_jsonl_server_reports_timeout_and_invalid_json() {
        let (client_side, server_side) = duplex(4_096);
        let (client_read, client_write) = split(client_side);
        let mut timed_client =
            JsonlClient::new(client_write, client_read, Duration::from_millis(10));
        let _held_open = server_side;
        assert!(matches!(
            timed_client.request("thread/list", json!({})).await,
            Err(CodexActivitySourceError::Timeout)
        ));

        let (client_side, mut server_side) = duplex(4_096);
        let (client_read, client_write) = split(client_side);
        let fake_server = tokio::spawn(async move {
            let mut buffer = [0_u8; 512];
            use tokio::io::AsyncReadExt;
            let _ = server_side.read(&mut buffer).await.unwrap();
            server_side.write_all(b"not-json\n").await.unwrap();
        });
        let mut invalid_client =
            JsonlClient::new(client_write, client_read, Duration::from_millis(250));
        assert!(matches!(
            invalid_client.request("thread/read", json!({})).await,
            Err(CodexActivitySourceError::Protocol(message)) if message.contains("invalid JSON")
        ));
        fake_server.await.unwrap();
    }

    #[tokio::test]
    async fn jsonl_client_rejects_write_methods_before_sending() {
        let (client_side, _server_side) = duplex(256);
        let (client_read, client_write) = split(client_side);
        let mut client = JsonlClient::new(client_write, client_read, Duration::from_millis(10));

        assert!(matches!(
            client.request("turn/start", json!({})).await,
            Err(CodexActivitySourceError::Protocol(message)) if message.contains("read-only whitelist")
        ));
    }

    #[test]
    fn turn_normalization_omits_reasoning_and_command_output() {
        let turn = json!({ "id": "turn-1", "status": "completed", "items": [
            { "type": "userMessage", "text": "Fix the build" },
            { "type": "reasoning", "text": "private chain" },
            { "type": "commandExecution", "command": "cargo test --secret abc", "exitCode": 0, "aggregatedOutput": "SECRET" },
            { "type": "agentMessage", "phase": "final_answer", "text": "Tests pass" }
        ]});
        let activity = normalize_turn(
            "thread-1",
            "turn-1",
            "cli",
            "",
            Utc::now(),
            &turn,
            Path::new("D:/repo"),
        );
        assert_eq!(activity.goal_hint, "Fix the build");
        assert_eq!(activity.activity_summary, "Tests pass");
        assert!(activity.outcome_summary.is_empty());
        assert_eq!(activity.validation_summary, "cargo: exit 0");
        assert!(!format!("{activity:?}").contains("private chain"));
        assert!(!format!("{activity:?}").contains("SECRET"));
    }

    #[test]
    fn turn_normalization_keeps_only_a_distinct_later_outcome() {
        let turn = json!({ "id": "turn-1", "status": "completed", "items": [
            { "type": "userMessage", "text": "Release RC10" },
            { "type": "agentMessage", "text": "Preparing release files" },
            { "type": "agentMessage", "phase": "final_answer", "text": "RC10 released and checks passed" }
        ]});
        let activity = normalize_turn(
            "thread-1",
            "turn-1",
            "cli",
            "",
            Utc::now(),
            &turn,
            Path::new("D:/repo"),
        );

        assert_eq!(activity.activity_summary, "Preparing release files");
        assert_eq!(activity.outcome_summary, "RC10 released and checks passed");
    }

    #[test]
    fn repository_path_is_projected_without_storing_the_absolute_path() {
        let turn = json!({ "id": "turn-1", "status": "completed", "items": [
            { "type": "userMessage", "text": "Inspect D:/repo/src/main.rs and D:/outside/secret.txt" }
        ]});
        let activity = normalize_turn(
            "thread-1",
            "turn-1",
            "cli",
            "",
            Utc::now(),
            &turn,
            Path::new("D:/repo"),
        );

        assert!(activity.goal_hint.contains("<repo>/src/main.rs"));
        assert!(!activity.goal_hint.contains("D:/repo"));
        assert!(!activity.goal_hint.contains("D:/outside"));
    }

    #[test]
    fn thread_scope_requires_exact_repository_and_excludes_derived_tasks() {
        let updated = Utc::now();
        let root = json!({
            "id": "root",
            "cwd": "D:/work/ForgeOS",
            "sourceKind": "cli",
            "updatedAt": updated.to_rfc3339()
        });
        let subagent = json!({
            "id": "child",
            "parentThreadId": "root",
            "cwd": "D:/work/ForgeOS",
            "sourceKind": "subagent",
            "updatedAt": updated.to_rfc3339()
        });

        assert!(thread_is_in_scope(
            &root,
            Path::new("d:/WORK/forgeos"),
            None
        ));
        assert!(!thread_is_in_scope(
            &root,
            Path::new("D:/work/ForgeOS-other"),
            None
        ));
        assert!(!thread_is_in_scope(
            &subagent,
            Path::new("D:/work/ForgeOS"),
            None
        ));
    }

    #[test]
    fn content_hash_input_can_be_stable() {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"same");
        let hash: String = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            hash,
            "0967115f2813a3541eaef77de9d9d5773f1c0c04314b0bbfe4ff3b3b1c55b5d5"
        );
    }

    #[tokio::test]
    #[ignore = "requires an installed Codex App Server and local task history"]
    async fn windows_codex_app_server_readonly_smoke() {
        let adapter = CodexAppServerAdapter;
        let health = adapter
            .health_check(None)
            .await
            .expect("Codex should be discoverable");
        assert!(health.available);
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let batch = adapter
            .read_repository_activity(
                None,
                repository,
                Some(Utc::now() - chrono::Duration::days(30)),
                2,
                10,
                100_000,
            )
            .await
            .expect("read-only App Server query should succeed");
        assert!(batch.threads_scanned <= 2);
        assert!(batch.turns_seen <= 10);
    }
}
