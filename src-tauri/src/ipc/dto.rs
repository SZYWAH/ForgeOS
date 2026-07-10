use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::{Project, ProjectSnapshot, Workspace};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub timezone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub name: String,
    pub timezone: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Workspace> for WorkspaceDto {
    fn from(workspace: Workspace) -> Self {
        Self {
            id: workspace.id,
            name: workspace.name,
            timezone: workspace.timezone,
            created_at: timestamp(workspace.created_at),
            updated_at: timestamp(workspace.updated_at),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub target_user: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub non_goals: String,
    #[serde(default)]
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    pub slug: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub vision: String,
    #[serde(default)]
    pub target_user: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub non_goals: String,
    #[serde(default)]
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDto {
    pub id: String,
    pub workspace_id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub vision: String,
    pub target_user: String,
    pub scope: String,
    pub non_goals: String,
    pub tech_stack: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
}

impl From<Project> for ProjectDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            workspace_id: project.workspace_id,
            slug: project.slug,
            name: project.name,
            description: project.description,
            vision: project.vision,
            target_user: project.target_user,
            scope: project.scope,
            non_goals: project.non_goals,
            tech_stack: project.tech_stack,
            status: project.status.as_str().to_owned(),
            lifecycle_stage: project.lifecycle_stage.as_str().to_owned(),
            priority: project.priority,
            created_at: timestamp(project.created_at),
            updated_at: timestamp(project.updated_at),
            version: project.version,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummaryDto {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub lifecycle_stage: String,
    pub priority: i32,
    pub updated_at: String,
    pub version: i64,
}

impl From<Project> for ProjectSummaryDto {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            slug: project.slug,
            name: project.name,
            description: project.description,
            status: project.status.as_str().to_owned(),
            lifecycle_stage: project.lifecycle_stage.as_str().to_owned(),
            priority: project.priority,
            updated_at: timestamp(project.updated_at),
            version: project.version,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSnapshotRequest {
    pub current_goal: String,
    pub status_summary: String,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_steps: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshotRequest {
    pub current_goal: String,
    pub status_summary: String,
    #[serde(default)]
    pub achievements: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub next_steps: Vec<String>,
    pub expected_version: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshotDto {
    pub id: String,
    pub project_id: String,
    pub version: i64,
    pub current_goal: String,
    pub status_summary: String,
    pub achievements: Vec<String>,
    pub blockers: Vec<String>,
    pub next_steps: Vec<String>,
    pub confirmed_at: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProjectSnapshot> for ProjectSnapshotDto {
    fn from(snapshot: ProjectSnapshot) -> Self {
        Self {
            id: snapshot.id().to_owned(),
            project_id: snapshot.project_id().to_owned(),
            version: snapshot.version(),
            current_goal: snapshot.current_goal().to_owned(),
            status_summary: snapshot.status_summary().to_owned(),
            achievements: snapshot.achievements().to_vec(),
            blockers: snapshot.blockers().to_vec(),
            next_steps: snapshot.next_steps().to_vec(),
            confirmed_at: timestamp(snapshot.confirmed_at()),
            created_at: timestamp(snapshot.created_at()),
            updated_at: timestamp(snapshot.updated_at()),
        }
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
