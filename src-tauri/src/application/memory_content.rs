use std::collections::HashSet;

use crate::ports::redact_external_text;

const REQUEST_MARKERS: [&str; 2] = ["my request for codex", "please implement this plan"];
const FILES_MARKER: &str = "files mentioned by the user";
const PLACEHOLDER_MARKERS: [&str; 8] = [
    "example",
    "placeholder",
    "your-",
    "your_",
    "<token>",
    "[redacted]",
    "xxxx",
    "sk-your",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryQualityCode {
    PromptWrapperRemoved,
    PromptWrapperUnresolved,
    ReadmeOverload,
    SuspectedSecret,
    ConfirmedStateSuperseded,
    MissingIdentity,
    DuplicateHistoryFields,
}

impl MemoryQualityCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PromptWrapperRemoved => "prompt_wrapper_removed",
            Self::PromptWrapperUnresolved => "prompt_wrapper_unresolved",
            Self::ReadmeOverload => "readme_overload",
            Self::SuspectedSecret => "suspected_secret",
            Self::ConfirmedStateSuperseded => "confirmed_state_superseded",
            Self::MissingIdentity => "missing_identity",
            Self::DuplicateHistoryFields => "duplicate_history_fields",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryQualitySeverity {
    Warning,
    Blocking,
}

impl MemoryQualitySeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Blocking => "blocking",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryQualityIssue {
    pub code: MemoryQualityCode,
    pub severity: MemoryQualitySeverity,
    pub section: String,
}

impl MemoryQualityIssue {
    pub fn warning(code: MemoryQualityCode, section: impl Into<String>) -> Self {
        Self {
            code,
            severity: MemoryQualitySeverity::Warning,
            section: section.into(),
        }
    }

    pub fn blocking(code: MemoryQualityCode, section: impl Into<String>) -> Self {
        Self {
            code,
            severity: MemoryQualitySeverity::Blocking,
            section: section.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SanitizedMemoryText {
    pub value: String,
    pub issues: Vec<MemoryQualityIssue>,
}

pub struct MemoryContentPolicy;

impl MemoryContentPolicy {
    pub fn sanitize_fact(value: &str, max_chars: usize, section: &str) -> SanitizedMemoryText {
        let (without_wrapper, wrapper_removed) = strip_prompt_wrappers(value);
        let suspected_secret = contains_suspected_secret(&without_wrapper);
        let redacted = redact_external_text(&without_wrapper);
        let cleaned = collapse_markdown_text(&redacted);
        let value = truncate_unicode(&cleaned, max_chars);
        let mut issues = Vec::new();
        if wrapper_removed {
            issues.push(MemoryQualityIssue::warning(
                MemoryQualityCode::PromptWrapperRemoved,
                section,
            ));
        }
        if contains_prompt_wrapper(&value) {
            issues.push(MemoryQualityIssue::blocking(
                MemoryQualityCode::PromptWrapperUnresolved,
                section,
            ));
        }
        if suspected_secret {
            issues.push(MemoryQualityIssue::blocking(
                MemoryQualityCode::SuspectedSecret,
                section,
            ));
        }
        SanitizedMemoryText { value, issues }
    }

    pub fn sanitize_scope(value: &str, section: &str) -> SanitizedMemoryText {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        let readme_like = looks_like_readme(&normalized);
        let suspected_secret = contains_suspected_secret(&normalized);
        let mut kept = Vec::new();
        let mut current_heading = String::new();
        let mut keep_section = true;
        let mut in_fence = false;

        for raw in normalized.lines() {
            let line = raw.trim();
            if line.starts_with("```") || line.starts_with("~~~") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence || line.is_empty() {
                continue;
            }
            if let Some(heading) = markdown_heading(line) {
                current_heading = heading.to_ascii_lowercase();
                keep_section = scope_heading_allowed(&current_heading);
                continue;
            }
            if !keep_section || looks_like_command_or_configuration(line) {
                continue;
            }
            kept.push(line);
        }

        if kept.is_empty() && !current_heading.is_empty() {
            kept = normalized
                .lines()
                .map(str::trim)
                .filter(|line| {
                    !line.is_empty()
                        && !line.starts_with('#')
                        && !line.starts_with("```")
                        && !looks_like_command_or_configuration(line)
                })
                .take(12)
                .collect();
        }

        let redacted = redact_external_text(&kept.join("\n"));
        let value = truncate_unicode(&collapse_markdown_text(&redacted), 1_200);
        let mut issues = Vec::new();
        if readme_like {
            issues.push(MemoryQualityIssue::warning(
                MemoryQualityCode::ReadmeOverload,
                section,
            ));
        }
        if suspected_secret {
            issues.push(MemoryQualityIssue::blocking(
                MemoryQualityCode::SuspectedSecret,
                section,
            ));
        }
        SanitizedMemoryText { value, issues }
    }

    pub fn sanitize_list(
        values: &[String],
        max_items: usize,
        max_chars: usize,
        section: &str,
    ) -> (Vec<String>, Vec<MemoryQualityIssue>) {
        let mut output = Vec::new();
        let mut fingerprints = HashSet::new();
        let mut issues = Vec::new();
        for value in values {
            let sanitized = Self::sanitize_fact(value, max_chars, section);
            issues.extend(sanitized.issues);
            if sanitized.value.is_empty() {
                continue;
            }
            let fingerprint = semantic_fingerprint(&sanitized.value);
            if fingerprint.is_empty() || !fingerprints.insert(fingerprint) {
                continue;
            }
            output.push(sanitized.value);
            if output.len() == max_items {
                break;
            }
        }
        (output, dedupe_issues(issues))
    }
}

pub fn strip_prompt_wrappers(value: &str) -> (String, bool) {
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let lower = normalized.to_ascii_lowercase();
    let mut start = None;
    for marker in REQUEST_MARKERS {
        if let Some(index) = lower.find(marker) {
            let after = index + marker.len();
            start = Some(after);
            break;
        }
    }
    let wrapper_present = start.is_some() || lower.contains(FILES_MARKER);
    let candidate = start.map_or(normalized.as_str(), |index| &normalized[index..]);
    let cleaned = candidate
        .lines()
        .filter_map(|line| {
            let mut line = line.trim();
            if line.is_empty() || line.eq_ignore_ascii_case("[local path redacted]") {
                return None;
            }
            line = line.trim_start_matches('#').trim();
            line = line.trim_start_matches(':').trim();
            (!line.eq_ignore_ascii_case("summary")).then_some(line)
        })
        .collect::<Vec<_>>()
        .join("\n");
    (cleaned, wrapper_present)
}

pub fn semantic_fingerprint(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn semantically_duplicate(left: &str, right: &str) -> bool {
    let left = semantic_fingerprint(left);
    let right = semantic_fingerprint(right);
    if left.is_empty() || right.is_empty() {
        return false;
    }
    if left == right {
        return true;
    }
    let (shorter, longer) = if left.chars().count() <= right.chars().count() {
        (&left, &right)
    } else {
        (&right, &left)
    };
    let short_len = shorter.chars().count();
    let long_len = longer.chars().count();
    short_len * 100 >= long_len * 80 && longer.contains(shorter)
}

pub fn truncate_unicode(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.trim().to_owned();
    }
    let mut output = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    output.push('…');
    output.trim().to_owned()
}

pub fn dedupe_issues(values: Vec<MemoryQualityIssue>) -> Vec<MemoryQualityIssue> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| {
            seen.insert((
                value.code.as_str(),
                value.severity.as_str(),
                value.section.clone(),
            ))
        })
        .collect()
}

fn contains_prompt_wrapper(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains(FILES_MARKER) || REQUEST_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn contains_suspected_secret(value: &str) -> bool {
    value.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        let token_prefix = ["ghp_", "github_pat_", "xoxb-", "xoxp-", "sk-"]
            .iter()
            .any(|prefix| {
                lower
                    .split(|character: char| {
                        character.is_whitespace() || matches!(character, '"' | '\'' | '`')
                    })
                    .map(normalize_secret_candidate)
                    .any(|token| token.starts_with(prefix) && !is_placeholder_secret(token))
            });
        let assignment = [
            "api_key",
            "apikey",
            "authorization",
            "access_token",
            "auth_token",
            "password",
            "secret",
        ]
        .iter()
        .any(|key| {
            lower.find(key).is_some_and(|index| {
                let suffix = lower[index + key.len()..].trim_start();
                let Some(value) = suffix
                    .strip_prefix('=')
                    .or_else(|| suffix.strip_prefix(':'))
                else {
                    return false;
                };
                let value = value.trim_start();
                let value = value.strip_prefix("bearer ").unwrap_or(value);
                let candidate =
                    normalize_secret_candidate(value.split_whitespace().next().unwrap_or_default());
                looks_like_assigned_secret(candidate)
            })
        });
        let bearer = lower.match_indices("bearer ").any(|(index, marker)| {
            let candidate = normalize_secret_candidate(
                lower[index + marker.len()..]
                    .split_whitespace()
                    .next()
                    .unwrap_or_default(),
            );
            looks_like_assigned_secret(candidate)
        });
        token_prefix || assignment || bearer
    })
}

fn normalize_secret_candidate(value: &str) -> &str {
    value.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
    })
}

fn is_placeholder_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    PLACEHOLDER_MARKERS
        .iter()
        .any(|marker| lower.contains(marker))
        || matches!(
            lower.as_str(),
            "token" | "tokens" | "value" | "key" | "secret" | "credential" | "credentials"
        )
}

fn looks_like_assigned_secret(value: &str) -> bool {
    !value.is_empty()
        && value.len() >= 8
        && !is_placeholder_secret(value)
        && value
            .chars()
            .any(|character| character.is_ascii_alphanumeric())
}

fn collapse_markdown_text(value: &str) -> String {
    value
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let line = line
                .trim_start_matches('#')
                .trim_start_matches(|character| matches!(character, '-' | '*' | '+'))
                .trim();
            (!line.is_empty()).then_some(line.replace(['`', '*'], ""))
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn looks_like_readme(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.chars().count() > 1_200
        || value.matches("```").count() >= 2
        || [
            "## install",
            "## usage",
            "## development",
            "## test",
            "pnpm ",
            "npm ",
            "cargo ",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
}

fn markdown_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let without = trimmed.trim_start_matches('#');
    (without.len() != trimmed.len()).then(|| without.trim())
}

fn scope_heading_allowed(heading: &str) -> bool {
    let excluded = [
        "install",
        "usage",
        "development",
        "getting started",
        "test",
        "build",
        "configuration",
        "environment",
        "backup",
        "安装",
        "运行",
        "开发",
        "测试",
        "构建",
        "配置",
        "环境变量",
        "备份",
    ];
    !excluded.iter().any(|value| heading.contains(value))
}

fn looks_like_command_or_configuration(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("npm ")
        || lower.starts_with("pnpm ")
        || lower.starts_with("cargo ")
        || lower.starts_with("git ")
        || lower.starts_with("docker ")
        || lower.starts_with("vite_")
        || lower.starts_with("export ")
        || lower.starts_with("set ")
        || lower.contains("api_key=")
        || lower.contains("password=")
}

#[cfg(test)]
mod tests {
    use super::{
        semantically_duplicate, MemoryContentPolicy, MemoryQualityCode, MemoryQualitySeverity,
    };

    #[test]
    fn strips_codex_file_wrapper_and_keeps_request() {
        let value = "# Files mentioned by the user:\n\n## image.png: [local path redacted]\n\n## My request for Codex:\n解决安装失败";
        let cleaned = MemoryContentPolicy::sanitize_fact(value, 200, "goal");
        assert_eq!(cleaned.value, "解决安装失败");
        assert!(cleaned
            .issues
            .iter()
            .any(|issue| issue.code == MemoryQualityCode::PromptWrapperRemoved));
    }

    #[test]
    fn semantic_comparison_ignores_markdown_and_spacing() {
        assert!(semantically_duplicate("**RC10 已发布**", "RC10已发布"));
        assert!(semantically_duplicate(
            "发布 RC10 并完成安装验证",
            "发布 RC10，并完成安装验证与回归"
        ));
        assert!(!semantically_duplicate("实现发布", "验证发布"));
    }

    #[test]
    fn list_cleanup_is_unicode_safe_deduplicated_and_bounded() {
        let values = vec![
            "**完成中文界面调整**".to_owned(),
            "完成中文界面调整".to_owned(),
            "验证安装流程".to_owned(),
            "记录下一步".to_owned(),
        ];
        let (cleaned, _) = MemoryContentPolicy::sanitize_list(&values, 3, 6, "next_steps");

        assert_eq!(cleaned.len(), 3);
        assert_eq!(cleaned[0], "完成中文界…");
        assert!(cleaned.iter().all(|value| value.chars().count() <= 6));
    }

    #[test]
    fn scope_keeps_product_summary_and_removes_commands() {
        let scope = "# Daymark\n本地优先的个人知识库。\n## 功能\n保存每日记录。\n## 安装\n```bash\npnpm install\n```\nVITE_API_KEY=sk-your-key";
        let cleaned = MemoryContentPolicy::sanitize_scope(scope, "scope");
        assert!(cleaned.value.contains("本地优先"));
        assert!(cleaned.value.contains("保存每日记录"));
        assert!(!cleaned.value.contains("pnpm"));
        assert!(!cleaned.value.contains("VITE_API_KEY"));
    }

    #[test]
    fn real_secret_is_blocking_but_placeholder_is_not() {
        let real = MemoryContentPolicy::sanitize_fact(
            "Authorization: Bearer private-production-token",
            200,
            "scope",
        );
        assert!(real.issues.iter().any(|issue| {
            issue.code == MemoryQualityCode::SuspectedSecret
                && issue.severity == MemoryQualitySeverity::Blocking
        }));
        assert!(!real.value.contains("private-production-token"));
        let example =
            MemoryContentPolicy::sanitize_fact("VITE_API_KEY=sk-your-example-key", 200, "scope");
        assert!(!example
            .issues
            .iter()
            .any(|issue| issue.code == MemoryQualityCode::SuspectedSecret));

        let documentation = MemoryContentPolicy::sanitize_fact(
            "ForgeOS 不会输出 API Key、Bearer token 或 Credential。",
            200,
            "scope",
        );
        assert!(!documentation
            .issues
            .iter()
            .any(|issue| issue.code == MemoryQualityCode::SuspectedSecret));
    }
}
