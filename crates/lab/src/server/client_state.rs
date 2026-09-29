//! Client-state tools: one-shot UI read, condition wait, the entity
//! table, and region screenshots / pixel probes.

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::json;

use super::LabServer;
use crate::supervisor::screenshot::{self, Region};

/// Args for `client_ui_state`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UiStateArgs {
    /// Chat lines to return from the primary chat tab (default 10, max 150).
    #[serde(default)]
    pub chat_lines: Option<u32>,
}

/// Args for `client_wait_for`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct WaitForArgs {
    /// A Lua boolean *expression* (no `return`), e.g.
    /// `CharSelectWin ~= nil and CharSelectWin:isVisible()`. Evaluated under
    /// pcall: an error reads as false and is reported as `last_error`.
    pub lua_condition: String,
    /// Give up after this many ms (default 10000, max 600000).
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    /// Poll interval in ms (default 250, 50..=60000).
    #[serde(default)]
    pub poll_ms: Option<u64>,
}

/// Args for `client_entity_table`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct EntityTableArgs {
    /// Only read details for these entity ids (the id lists are always
    /// complete). Omit for every entity.
    #[serde(default)]
    pub ids: Option<Vec<u32>>,
    /// Call each detailed entity's `isReady()` (a journaled native call on
    /// the main thread). Default true.
    #[serde(default)]
    pub is_ready: Option<bool>,
    /// Max tree nodes visited per map (default 4096).
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

/// A rectangle in capture pixels (the whole window, frame included).
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegionArgs {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Args for `lab_pixel_probe`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PixelProbeArgs {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    /// Inclusive lower RGB bound, e.g. `[0, 150, 0]`.
    pub min: [u8; 3],
    /// Inclusive upper RGB bound, e.g. `[120, 255, 120]`.
    pub max: [u8; 3],
}

#[tool_router(router = client_state_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "One read of the visible UI: visible top-level windows, the open dialog (title, text, visible buttons), visible prompts (title, message), mission tracker lines, and the chat tail. Each section is read under its own pcall; a failed section is listed in `errors`."
    )]
    async fn client_ui_state(
        &self,
        Parameters(a): Parameters<UiStateArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(self.supervisor.ui_state(a.chat_lines).await)
    }

    #[tool(
        description = "Poll a Lua boolean expression on the client until it is true or the timeout passes. Returns met, elapsed_ms, polls and the last Lua error; a timeout is `met: false`, not an error."
    )]
    async fn client_wait_for(
        &self,
        Parameters(a): Parameters<WaitForArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(
            self.supervisor
                .wait_for(&a.lua_condition, a.timeout_ms, a.poll_ms)
                .await,
        )
    }

    #[tool(
        description = "Walk the client's BigWorld entity maps (GameEntityManager: entities, limbo, pending enter counts) with coarse memory reads (one per tree node, one per entity). Per entity: id, pointer, vtable, enter count, actor pointer (rendered = non-null), the rendered flag bit, and isReady() via its vtable. Needs the client in the world."
    )]
    async fn client_entity_table(
        &self,
        Parameters(a): Parameters<EntityTableArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.wrap(
            self.supervisor
                .entity_table(a.ids, a.is_ready.unwrap_or(true), a.max_nodes)
                .await,
        )
    }

    #[tool(
        description = "Capture a rectangle of the client window (capture pixels: the whole window, frame included, same space as lab_screenshot) and return it as a PNG image."
    )]
    async fn lab_screenshot_region(
        &self,
        Parameters(a): Parameters<RegionArgs>,
    ) -> Result<CallToolResult, McpError> {
        let region = Region {
            x: a.x,
            y: a.y,
            w: a.w,
            h: a.h,
        };
        let img = self
            .supervisor
            .capture()
            .await
            .and_then(|full| screenshot::crop(&full, region))
            .and_then(|c| screenshot::encode_png(&c).map(|png| (png, c.width, c.height)));
        match img {
            Ok((png, w, h)) => Ok(CallToolResult::success(vec![
                ContentBlock::text(format!("region {w}x{h} at ({}, {})", a.x, a.y)),
                ContentBlock::image(screenshot::png_to_base64(&png), "image/png"),
            ])),
            Err(e) => Err(McpError::internal_error(
                format!("screenshot region: {e}"),
                None,
            )),
        }
    }

    #[tool(
        description = "Count the pixels in a rectangle of the client window whose RGB lies in an inclusive [min, max] box, plus the rectangle's mean colour. Cheap on-screen checks (a nameplate colour, a HUD element) without reading the image."
    )]
    async fn lab_pixel_probe(
        &self,
        Parameters(a): Parameters<PixelProbeArgs>,
    ) -> Result<CallToolResult, McpError> {
        let region = Region {
            x: a.x,
            y: a.y,
            w: a.w,
            h: a.h,
        };
        let r = self
            .supervisor
            .capture()
            .await
            .and_then(|full| screenshot::probe(&full, region, a.min, a.max))
            .map(|p| {
                json!({
                    "matched": p.matched,
                    "total": p.total,
                    "fraction": p.matched as f64 / p.total.max(1) as f64,
                    "mean_rgb": p.mean_rgb,
                })
            });
        self.wrap(r)
    }
}
