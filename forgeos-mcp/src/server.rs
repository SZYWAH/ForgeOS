use std::sync::Arc;

use forgeos_lib::application::{McpQueryError, McpQueryKernel};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router, Json, ServerHandler,
};

use crate::{
    protocol::{
        ContextPackResult, CurrentSnapshotResult, EvidenceListResult, EvidenceResult,
        ProjectIdentityResult, RecentSessionResult, RecentSessionsResult,
    },
    tools::{
        context::BuildContextPackParams, evidence::ListEvidenceParams, project::ProjectScope,
        session::RecentSessionsParams, snapshot::CurrentSnapshotParams,
    },
};

#[derive(Clone)]
pub struct ForgeOsMcpServer {
    kernel: Arc<McpQueryKernel>,
    tool_router: ToolRouter<Self>,
}

#[tool_router(router = tool_router)]
impl ForgeOsMcpServer {
    pub fn new(kernel: Arc<McpQueryKernel>) -> Self {
        Self {
            kernel,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Read the confirmed identity and lifecycle state of one ForgeOS Project.")]
    async fn get_project_identity(
        &self,
        Parameters(ProjectScope { project_id }): Parameters<ProjectScope>,
    ) -> Result<Json<ProjectIdentityResult>, String> {
        require_project_id(&project_id)?;
        self.kernel
            .get_project_identity(&project_id)
            .await
            .map(ProjectIdentityResult::from)
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(description = "Read the current confirmed Snapshot for one ForgeOS Project.")]
    async fn get_current_snapshot(
        &self,
        Parameters(CurrentSnapshotParams { project_id }): Parameters<CurrentSnapshotParams>,
    ) -> Result<Json<CurrentSnapshotResult>, String> {
        require_project_id(&project_id)?;
        self.kernel
            .get_current_snapshot(&project_id)
            .await
            .map(|snapshot| CurrentSnapshotResult::from_optional(project_id, snapshot))
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Read up to five most recent Development Sessions for one ForgeOS Project."
    )]
    async fn get_recent_sessions(
        &self,
        Parameters(RecentSessionsParams { project_id }): Parameters<RecentSessionsParams>,
    ) -> Result<Json<RecentSessionsResult>, String> {
        require_project_id(&project_id)?;
        self.kernel
            .get_recent_sessions(&project_id)
            .await
            .map(|sessions| RecentSessionsResult {
                project_id,
                sessions: sessions
                    .into_iter()
                    .map(RecentSessionResult::from)
                    .collect(),
            })
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Read manual and Git Evidence summaries for one ForgeOS Project. Source paths are omitted."
    )]
    async fn list_evidence(
        &self,
        Parameters(ListEvidenceParams { project_id }): Parameters<ListEvidenceParams>,
    ) -> Result<Json<EvidenceListResult>, String> {
        require_project_id(&project_id)?;
        self.kernel
            .list_evidence(&project_id)
            .await
            .map(|evidence| EvidenceListResult {
                project_id,
                evidence: evidence.into_iter().map(EvidenceResult::from).collect(),
            })
            .map(Json)
            .map_err(safe_error)
    }

    #[tool(
        description = "Build the deterministic ForgeOS Context Pack for one Project in memory without modifying the database."
    )]
    async fn build_context_pack(
        &self,
        Parameters(BuildContextPackParams {
            project_id,
            template_type,
        }): Parameters<BuildContextPackParams>,
    ) -> Result<Json<ContextPackResult>, String> {
        require_project_id(&project_id)?;
        self.kernel
            .build_context_pack(&project_id, &template_type)
            .await
            .map(ContextPackResult::from)
            .map(Json)
            .map_err(safe_error)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ForgeOsMcpServer {}

fn require_project_id(project_id: &str) -> Result<(), String> {
    if project_id.trim().is_empty() {
        Err("project_id is required".into())
    } else {
        Ok(())
    }
}

fn safe_error(error: McpQueryError) -> String {
    match error {
        McpQueryError::ProjectNotFound => error.to_string(),
        McpQueryError::WorkspaceNotFound => error.to_string(),
        McpQueryError::Validation(_) => error.to_string(),
        McpQueryError::Render(_) | McpQueryError::CorruptData(_) | McpQueryError::Storage(_) => {
            "ForgeOS could not complete the read-only query".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_surface_is_exactly_the_five_read_only_tools() {
        let names = ForgeOsMcpServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            [
                "build_context_pack",
                "get_current_snapshot",
                "get_project_identity",
                "get_recent_sessions",
                "list_evidence",
            ]
        );
        assert!(names
            .iter()
            .all(|name| !["create", "update", "delete", "accept"]
                .iter()
                .any(|verb| name.contains(verb))));
    }
}
