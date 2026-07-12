use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectScope {
    /// ForgeOS Project id. Every request is explicitly scoped to one Project.
    pub project_id: String,
}
