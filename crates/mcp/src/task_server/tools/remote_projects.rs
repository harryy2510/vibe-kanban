use api_types::ListProjectsResponse;
use rmcp::{
    ErrorData, handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool,
    tool_router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::McpServer;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpListProjectsRequest {
    #[schemars(description = "The ID of the organization to list projects from")]
    organization_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ProjectSummary {
    #[schemars(description = "The unique identifier of the project")]
    id: String,
    #[schemars(description = "The name of the project")]
    name: String,
    #[schemars(description = "When the project was created")]
    created_at: String,
    #[schemars(description = "When the project was last updated")]
    updated_at: String,
}

impl ProjectSummary {
    fn from_remote_project(project: api_types::Project) -> Self {
        Self {
            id: project.id.to_string(),
            name: project.name,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct McpListProjectsResponse {
    projects: Vec<ProjectSummary>,
    count: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct McpGetProjectRequest {
    #[schemars(description = "The ID of the project to retrieve")]
    project_id: Uuid,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct ProjectDetails {
    #[schemars(description = "The unique identifier of the project")]
    id: String,
    #[schemars(description = "The organization this project belongs to")]
    organization_id: String,
    #[schemars(description = "The name of the project")]
    name: String,
    #[schemars(description = "The project color")]
    color: String,
    #[schemars(description = "Sort order within the organization")]
    sort_order: i32,
    #[schemars(description = "When the project was created")]
    created_at: String,
    #[schemars(description = "When the project was last updated")]
    updated_at: String,
}

#[tool_router(router = remote_projects_tools_router, vis = "pub")]
impl McpServer {
    #[tool(description = "List all the available projects")]
    async fn list_projects(
        &self,
        Parameters(McpListProjectsRequest { organization_id }): Parameters<McpListProjectsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!(
            "/api/remote/projects?organization_id={}",
            organization_id
        ));
        let response: ListProjectsResponse = match self.send_json(self.client.get(&url)).await {
            Ok(r) => r,
            Err(e) => return Ok(Self::tool_error(e)),
        };

        let project_summaries: Vec<ProjectSummary> = response
            .projects
            .into_iter()
            .map(ProjectSummary::from_remote_project)
            .collect();

        McpServer::success(&McpListProjectsResponse {
            count: project_summaries.len(),
            projects: project_summaries,
        })
    }

    #[tool(
        description = "Get detailed information about a single project including its color, sort order, and organization."
    )]
    async fn get_project(
        &self,
        Parameters(McpGetProjectRequest { project_id }): Parameters<McpGetProjectRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let url = self.url(&format!("/api/remote/projects/{}", project_id));
        let project: api_types::Project = match self.send_json(self.client.get(&url)).await {
            Ok(p) => p,
            Err(e) => return Ok(Self::tool_error(e)),
        };

        Self::success(&ProjectDetails {
            id: project.id.to_string(),
            organization_id: project.organization_id.to_string(),
            name: project.name,
            color: project.color,
            sort_order: project.sort_order,
            created_at: project.created_at.to_rfc3339(),
            updated_at: project.updated_at.to_rfc3339(),
        })
    }
}
