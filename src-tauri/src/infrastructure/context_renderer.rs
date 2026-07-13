use std::fmt::Write;

use crate::{
    domain::EvidenceType,
    ports::{
        redact_external_text, redact_git_evidence_description, ContextRenderError,
        ContextRenderInput, ContextRenderer,
    },
};

#[derive(Default)]
pub struct MarkdownContextRenderer;

impl ContextRenderer for MarkdownContextRenderer {
    fn render(&self, input: ContextRenderInput<'_>) -> Result<String, ContextRenderError> {
        let mut output = String::new();
        writeln!(output, "# ForgeOS Context Pack").map_err(render_error)?;
        writeln!(output).map_err(render_error)?;
        writeln!(output, "Template: {}", input.template_type.as_str()).map_err(render_error)?;
        writeln!(output, "Purpose: {}", input.template_type.purpose()).map_err(render_error)?;
        writeln!(output).map_err(render_error)?;

        writeln!(output, "## Project Overview").map_err(render_error)?;
        field(&mut output, "Name", &input.project.name)?;
        field(&mut output, "Description", &input.project.description)?;
        field(&mut output, "Vision", &input.project.vision)?;
        field(&mut output, "Target User", &input.project.target_user)?;
        field(&mut output, "Scope", &input.project.scope)?;
        field(&mut output, "Tech Stack", &input.project.tech_stack)?;
        field(
            &mut output,
            "Lifecycle Stage",
            input.project.lifecycle_stage.as_str(),
        )?;
        field(&mut output, "Status", input.project.status.as_str())?;

        writeln!(output, "## Current Snapshot").map_err(render_error)?;
        if let Some(snapshot) = input.snapshot {
            field(&mut output, "Current Goal", snapshot.current_goal())?;
            field(&mut output, "Status Summary", snapshot.status_summary())?;
            list(&mut output, "Achievements", snapshot.achievements())?;
            list(&mut output, "Blockers", snapshot.blockers())?;
            list(&mut output, "Next Steps", snapshot.next_steps())?;
        } else {
            writeln!(output, "Not available.").map_err(render_error)?;
            writeln!(output).map_err(render_error)?;
        }

        writeln!(output, "## Recent Development Session").map_err(render_error)?;
        if let Some(session) = input.recent_session {
            field(&mut output, "Goal", session.goal())?;
            field(&mut output, "Outcome", session.outcome())?;
            field(&mut output, "Validation", session.validation())?;
            field(&mut output, "Handoff", session.handoff())?;
        } else {
            writeln!(output, "Not available.").map_err(render_error)?;
            writeln!(output).map_err(render_error)?;
        }

        writeln!(output, "## Evidence").map_err(render_error)?;
        let mut evidence = input
            .evidences
            .iter()
            .filter(|evidence| !is_local_git_evidence(evidence.evidence_type()))
            .peekable();
        if evidence.peek().is_none() {
            writeln!(output, "Not available.").map_err(render_error)?;
        } else {
            for evidence in evidence {
                writeln!(
                    output,
                    "### {}",
                    value(&redact_external_text(evidence.title()))
                )
                .map_err(render_error)?;
                field(&mut output, "Type", evidence.evidence_type().as_str())?;
                field(&mut output, "Description", evidence.description())?;
                field(
                    &mut output,
                    "Verification Status",
                    evidence.verification_status().as_str(),
                )?;
            }
        }

        writeln!(output).map_err(render_error)?;
        writeln!(output, "## Recent Git Activity").map_err(render_error)?;
        let mut git_activity = input
            .evidences
            .iter()
            .filter(|evidence| is_local_git_evidence(evidence.evidence_type()))
            .peekable();
        if git_activity.peek().is_none() {
            writeln!(output, "Not available.").map_err(render_error)?;
        } else {
            for evidence in git_activity {
                writeln!(
                    output,
                    "### {}",
                    value(&redact_external_text(evidence.title()))
                )
                .map_err(render_error)?;
                field(&mut output, "Type", evidence.evidence_type().as_str())?;
                field(
                    &mut output,
                    "Description",
                    &redact_git_evidence_description(evidence.description()),
                )?;
                field(
                    &mut output,
                    "Verification Status",
                    evidence.verification_status().as_str(),
                )?;
            }
        }

        Ok(output)
    }
}

fn is_local_git_evidence(evidence_type: EvidenceType) -> bool {
    matches!(
        evidence_type,
        EvidenceType::GitCommit | EvidenceType::GitStatus
    )
}

fn field(output: &mut String, label: &str, content: &str) -> Result<(), ContextRenderError> {
    writeln!(output, "{label}:").map_err(render_error)?;
    writeln!(output, "{}", value(&redact_external_text(content))).map_err(render_error)?;
    writeln!(output).map_err(render_error)
}

fn list(output: &mut String, label: &str, items: &[String]) -> Result<(), ContextRenderError> {
    writeln!(output, "{label}:").map_err(render_error)?;
    if items.is_empty() {
        writeln!(output, "- None recorded").map_err(render_error)?;
    } else {
        for item in items {
            writeln!(output, "- {}", value(&redact_external_text(item))).map_err(render_error)?;
        }
    }
    writeln!(output).map_err(render_error)
}

fn value(content: &str) -> &str {
    let content = content.trim();
    if content.is_empty() {
        "Not recorded"
    } else {
        content
    }
}

fn render_error(error: std::fmt::Error) -> ContextRenderError {
    ContextRenderError::Render(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        ContextTemplateType, LifecycleStage, Project, ProjectDetails, ProjectStatus,
    };
    use chrono::Utc;

    #[test]
    fn rendering_is_stable_and_has_fixed_section_order() {
        let project = Project::new(
            "project-1".into(),
            "workspace-1".into(),
            ProjectDetails {
                slug: "forgeos".into(),
                name: "ForgeOS".into(),
                description: "Engineering memory".into(),
                vision: "Reliable context".into(),
                target_user: "Independent developer".into(),
                scope: "Local-first".into(),
                non_goals: "Task manager".into(),
                tech_stack: "Tauri + Rust".into(),
                status: ProjectStatus::Active,
                lifecycle_stage: LifecycleStage::Implementation,
                priority: 100,
            },
            Utc::now(),
        )
        .unwrap();
        let renderer = MarkdownContextRenderer;
        let render = || {
            renderer
                .render(ContextRenderInput {
                    template_type: ContextTemplateType::Planning,
                    project: &project,
                    snapshot: None,
                    recent_session: None,
                    evidences: &[],
                })
                .unwrap()
        };
        let first = render();
        assert_eq!(first, render());
        let project_at = first.find("## Project Overview").unwrap();
        let snapshot_at = first.find("## Current Snapshot").unwrap();
        let session_at = first.find("## Recent Development Session").unwrap();
        let evidence_at = first.find("## Evidence").unwrap();
        let git_at = first.find("## Recent Git Activity").unwrap();
        assert!(
            project_at < snapshot_at
                && snapshot_at < session_at
                && session_at < evidence_at
                && evidence_at < git_at
        );
        assert!(first.contains("Lifecycle Stage:\nimplementation"));
        assert!(first.contains("Status:\nactive"));
    }
}
