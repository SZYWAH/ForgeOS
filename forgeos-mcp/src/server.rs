use std::sync::Arc;

use forgeos_lib::application::{ProjectMemoryError, ProjectMemoryQueryKernel};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router, Json, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::protocol::{
    AchievementResult, AiContextResult, DevelopmentHistoryItemResult, ProjectAchievementsResult,
    ProjectContextResult, ProjectHistoryResult, ProjectListItem, ProjectListResult,
};

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLocator {
    pub project_id: Option<String>,
    pub repository_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHistoryParams {
    pub project_id: Option<String>,
    pub repository_path: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone)]
pub struct ForgeOsMcpServer {
    kernel: Arc<ProjectMemoryQueryKernel>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl ForgeOsMcpServer {
    pub fn new(kernel: Arc<ProjectMemoryQueryKernel>) -> Self {
        Self {
            kernel,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List ForgeOS Projects using safe discovery fields only. Local repository paths are never returned."
    )]
    async fn list_projects(&self) -> Result<Json<ProjectListResult>, String> {
        self.kernel
            .list_project_memory_summaries()
            .await
            .map(|items| ProjectListResult {
                projects: items.into_iter().map(ProjectListItem::from).collect(),
            })
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Read one Project's identity, confirmed current state, and latest observed state. Provide exactly one of projectId or repositoryPath."
    )]
    async fn get_project_context(
        &self,
        Parameters(locator): Parameters<ProjectLocator>,
    ) -> Result<Json<ProjectContextResult>, String> {
        let project_id = self.resolve(locator).await?;
        self.kernel
            .get_project_memory_overview(&project_id)
            .await
            .map(ProjectContextResult::from)
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Read aggregated development activities for one Project. Codex turns are grouped into episodes and Git commits are attached without returning conversations, diffs, or paths."
    )]
    async fn get_project_history(
        &self,
        Parameters(params): Parameters<ProjectHistoryParams>,
    ) -> Result<Json<ProjectHistoryResult>, String> {
        let project_id = self
            .kernel
            .resolve_project_locator(
                params.project_id.as_deref(),
                params.repository_path.as_deref(),
            )
            .await
            .map_err(safe_error)?;
        self.kernel
            .list_development_history(
                &project_id,
                params.cursor.as_deref(),
                params.limit.unwrap_or(20),
            )
            .await
            .map(|page| ProjectHistoryResult {
                project_id,
                items: page
                    .items
                    .into_iter()
                    .map(DevelopmentHistoryItemResult::from)
                    .collect(),
                next_cursor: page.next_cursor,
            })
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Read confirmed, structured project achievements. Draft and archived achievements are omitted."
    )]
    async fn get_project_achievements(
        &self,
        Parameters(locator): Parameters<ProjectLocator>,
    ) -> Result<Json<ProjectAchievementsResult>, String> {
        let project_id = self.resolve(locator).await?;
        self.kernel
            .list_project_achievements(&project_id)
            .await
            .map(|items| ProjectAchievementsResult {
                project_id,
                achievements: items
                    .into_iter()
                    .filter(|item| item.status.as_str() == "confirmed")
                    .map(AchievementResult::from)
                    .collect(),
            })
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Build deterministic Markdown context for one Project from identity, confirmed state, observed activity, recent history, confirmed achievements, Git state, and next steps. This is read-only."
    )]
    async fn build_ai_context(
        &self,
        Parameters(locator): Parameters<ProjectLocator>,
    ) -> Result<Json<AiContextResult>, String> {
        let project_id = self.resolve(locator).await?;
        self.kernel
            .build_project_ai_context(&project_id)
            .await
            .map(AiContextResult::from)
            .map(Json)
            .map_err(safe_error)
    }

    async fn resolve(&self, locator: ProjectLocator) -> Result<String, String> {
        self.kernel
            .resolve_project_locator(
                locator.project_id.as_deref(),
                locator.repository_path.as_deref(),
            )
            .await
            .map_err(safe_error)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ForgeOsMcpServer {}

fn safe_error(error: ProjectMemoryError) -> String {
    match error {
        ProjectMemoryError::ProjectNotFound
        | ProjectMemoryError::WorkspaceNotFound
        | ProjectMemoryError::InvalidCursor
        | ProjectMemoryError::InvalidProjectLocator
        | ProjectMemoryError::RepositoryPathNotConnected => error.to_string(),
        ProjectMemoryError::CorruptData(_) | ProjectMemoryError::Storage(_) => {
            "ForgeOS could not complete the read-only query".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_surface_is_exactly_the_five_read_only_memory_tools() {
        let names = ForgeOsMcpServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "build_ai_context",
                "get_project_achievements",
                "get_project_context",
                "get_project_history",
                "list_projects"
            ]
        );
        assert!(names.iter().all(|name| ![
            "create", "update", "delete", "accept", "write", "execute"
        ]
        .iter()
        .any(|verb| name.contains(verb))));
    }
}
