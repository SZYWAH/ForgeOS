use thiserror::Error;

use crate::domain::{ContextTemplateType, DevelopmentSession, Evidence, Project, ProjectSnapshot};

pub struct ContextRenderInput<'a> {
    pub template_type: ContextTemplateType,
    pub project: &'a Project,
    pub snapshot: Option<&'a ProjectSnapshot>,
    pub recent_session: Option<&'a DevelopmentSession>,
    pub evidences: &'a [Evidence],
}

#[derive(Debug, Error)]
pub enum ContextRenderError {
    #[error("Context Pack rendering failed: {0}")]
    Render(String),
}

pub trait ContextRenderer: Send + Sync {
    fn render(&self, input: ContextRenderInput<'_>) -> Result<String, ContextRenderError>;
}

/// Removes local-only details embedded in legacy Git Evidence descriptions
/// before the text crosses an external read boundary.
pub fn redact_git_evidence_description(description: &str) -> String {
    let normalized = description.replace("\r\n", "\n");
    let mut output = Vec::new();

    for line in normalized.lines() {
        if line.trim() == "Repository:" {
            break;
        }
        output.push(redact_external_text(line));
    }

    output.join("\n").trim().to_owned()
}

/// Applies a conservative safety projection to free text leaving ForgeOS.
/// URLs remain available, while common local paths, email addresses, and
/// inline secret assignments are replaced with explicit placeholders.
pub fn redact_external_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .lines()
        .map(redact_external_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_external_line(line: &str) -> String {
    let line = redact_email_tokens(line);
    let path_start = find_private_path_start(&line);
    let secret_start = find_secret_assignment_start(&line);

    match (path_start, secret_start) {
        (None, None) => line,
        (Some(start), None) => replace_sensitive_suffix(&line, start, "[local path redacted]"),
        (None, Some(start)) => replace_sensitive_suffix(&line, start, "[secret redacted]"),
        (Some(path), Some(secret)) if path <= secret => {
            replace_sensitive_suffix(&line, path, "[local path redacted]")
        }
        (Some(_), Some(secret)) => replace_sensitive_suffix(&line, secret, "[secret redacted]"),
    }
}

fn replace_sensitive_suffix(line: &str, start: usize, placeholder: &str) -> String {
    let prefix = line[..start].trim_end();
    if prefix.is_empty() {
        placeholder.into()
    } else {
        format!("{prefix} {placeholder}")
    }
}

fn redact_email_tokens(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    for part in line.split_inclusive(char::is_whitespace) {
        let content = part.trim_end_matches(char::is_whitespace);
        let whitespace = &part[content.len()..];
        if looks_like_email(content) {
            output.push_str("[email redacted]");
        } else if looks_like_secret_token(content) {
            output.push_str("[secret redacted]");
        } else {
            output.push_str(content);
        }
        output.push_str(whitespace);
    }
    output
}

fn looks_like_email(token: &str) -> bool {
    let token = token.trim_matches(|character: char| {
        matches!(character, '<' | '>' | '(' | ')' | '[' | ']' | ',' | ';')
    });
    token
        .split_once('@')
        .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
}

fn looks_like_secret_token(token: &str) -> bool {
    let token = token.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
    });
    let lower = token.to_ascii_lowercase();
    (lower.starts_with("sk-") && lower.len() >= 12)
        || lower.starts_with("ghp_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xoxb-")
        || lower.starts_with("xoxp-")
}

fn find_private_path_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let windows_path = (0..bytes.len().saturating_sub(2)).find(|index| {
        bytes[*index].is_ascii_alphabetic()
            && bytes[*index + 1] == b':'
            && matches!(bytes[*index + 2], b'/' | b'\\')
            && (*index == 0
                || line[..*index].chars().last().is_some_and(|character| {
                    !character.is_ascii_alphanumeric() && !matches!(character, '_' | '-')
                }))
    });
    let known_path = ["file:///", "/Users/", "/home/", "/root/"]
        .into_iter()
        .flat_map(|pattern| line.match_indices(pattern).map(|(index, _)| index))
        .filter(|index| {
            *index == 0
                || line[..*index].chars().last().is_some_and(|character| {
                    character.is_whitespace()
                        || matches!(character, '(' | '[' | '{' | '=' | ':' | '"' | '\'')
                })
        })
        .min();
    let unc_path = line.find("\\\\").filter(|index| {
        *index == 0
            || line[..*index]
                .chars()
                .last()
                .is_some_and(char::is_whitespace)
    });

    [windows_path, known_path, unc_path]
        .into_iter()
        .flatten()
        .min()
}

fn find_secret_assignment_start(line: &str) -> Option<usize> {
    let lower = line.to_ascii_lowercase();
    [
        "api_key",
        "apikey",
        "access_token",
        "auth_token",
        "credential",
        "password",
        "secret",
    ]
    .into_iter()
    .filter_map(|key| {
        lower.find(key).and_then(|start| {
            let suffix = lower[start + key.len()..].trim_start();
            (suffix.starts_with(':') || suffix.starts_with('=')).then_some(start)
        })
    })
    .min()
}

#[cfg(test)]
mod safety_tests {
    use super::{redact_external_text, redact_git_evidence_description};

    #[test]
    fn git_evidence_redaction_removes_local_path_and_author_email() {
        let description = "Commit:\nabc123\n\nAuthor:\nDeveloper <private@example.com>\n\nRepository:\nD:/private/repository";
        let redacted = redact_git_evidence_description(description);

        assert!(redacted.contains("Developer"));
        assert!(!redacted.contains("private@example.com"));
        assert!(!redacted.contains("D:/private/repository"));
        assert!(!redacted.contains("Repository:"));
    }

    #[test]
    fn external_text_redacts_embedded_paths_emails_and_tokens() {
        let redacted = redact_external_text(
            "Inspect D:/private/repository and private@example.com with sk-test-secret-value",
        );

        assert!(redacted.contains("Inspect"));
        assert!(!redacted.contains("D:/private/repository"));
        assert!(!redacted.contains("private@example.com"));
        assert!(!redacted.contains("sk-test-secret-value"));
    }

    #[test]
    fn external_text_preserves_http_urls() {
        let value = "Docs: https://example.com/forgeos and http://localhost:1420/help";

        assert_eq!(redact_external_text(value), value);
    }
}
