use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildContextPackParams {
    /// ForgeOS Project id. Sources from other Projects are never included.
    pub project_id: String,
    /// One of: planning, coding, debugging, review, handoff.
    pub template_type: String,
}
