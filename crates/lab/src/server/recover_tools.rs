//! `lab_recover` (C11): known fixes for a stuck client, each read back.
//! The `instance` argument is added and stripped by the router
//! (`server/instances.rs`), so it is not a field here. Orchestration lives in
//! `supervisor::recover_recipes`.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};

use super::LabServer;

/// Args for `lab_recover`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RecoverArgs {
    /// `release_input`, `close_dialogs`, `recenter_camera` or `unstick`.
    pub recipe: String,
}

#[tool_router(router = recover_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Run one recovery recipe (release_input, close_dialogs, recenter_camera, unstick) and read back whether it worked. Returns evidence; a recipe that could not confirm its effect is inconclusive, never a silent success."
    )]
    async fn lab_recover(
        &self,
        Parameters(a): Parameters<RecoverArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.recover(&a.recipe).await)
    }
}
