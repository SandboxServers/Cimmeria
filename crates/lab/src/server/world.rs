//! World-interaction tools: find entities, click them in the 3D view,
//! walk, aim the camera. Orchestration lives in `supervisor::world`; every
//! result reports `native_level` (see that module).

use rmcp::{
    handler::server::wrapper::Parameters, model::*, tool, tool_router, ErrorData as McpError,
};
use serde_json::Value;

use super::LabServer;
use crate::supervisor::world::camera::CameraRequest;
use crate::supervisor::world::click::{ClickRequest, Expect};
use crate::supervisor::world::find::FindRequest;
use crate::supervisor::world::io::LiveWorld;
use crate::supervisor::world::movement::MoveRequest;
use crate::supervisor::world::{camera, click, find, movement, WorldError};

#[tool_router(router = world_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Find entities the client knows, by entity_id, name (substring), mob_id (the client's template id), hostility or distance. Per match: id, name, level, hostility, mob id, rendered (actor in the scene), targetable, position (client UE3 units and server metres), distance from the player, and the screen point from the game's own worldToPixel. Read-only (native_level: read)."
    )]
    async fn client_entity_find(
        &self,
        Parameters(a): Parameters<FindRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(find::run(&mut io, a).await)
    }

    #[tool(
        description = "Click an entity (entity_id or name) or a world point in the 3D view like a player: project it with the game's view, turn the camera with mouse-look if it is off screen, put the cursor on it and check the client's own mouse-over (another entity in the way fails as occluded unless force), click with real button messages (default right = interact), then report the target and the windows opened/closed. `expect` (target, window, any, nothing) decides pass/fail. Reports native_level."
    )]
    async fn client_world_click(
        &self,
        Parameters(a): Parameters<ClickRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(click::run(&mut io, "client_world_click", a, None).await)
    }

    #[tool(
        description = "Target an entity (entity_id or name) by left-clicking it in the 3D view (same path as client_world_click) and verify Unit.Target became that entity. With allow_fallback, a failed click falls back to the stock targetUnit() and the result says native_level: ui_lua (N3), which a UAT runner must not count as a native pass."
    )]
    async fn client_target(
        &self,
        Parameters(a): Parameters<ClickRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(click::run(&mut io, "client_target", a, Some((0, Expect::Target))).await)
    }

    #[tool(
        description = "Walk to a point (server metres by default, or space: client), an entity (entity_id or name; the goal follows it), through optional waypoints, with the movement keys: W held, turning with mouse-look, closed loop on the player's actor position every 100 ms. Learns the mouse-look gain, jumps/strafes out of snags, fails on a teleport, a snag it cannot clear, or the timeout (default 60 s) with the distance left. Always releases the keys."
    )]
    async fn client_move_to(
        &self,
        Parameters(a): Parameters<MoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(movement::run(&mut io, a).await)
    }

    #[tool(
        description = "Camera control with real input: raw mouse-look (yaw_counts, pitch_counts), zoom (zoom_notches of the wheel), and/or face an entity or point (turns until it projects near the screen centre, closed loop on worldToPixel). Reports the camera actor's pose before and after."
    )]
    async fn client_camera(
        &self,
        Parameters(a): Parameters<CameraRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(camera::run(&mut io, a).await)
    }
}

/// A world tool's JSON as text, or its failure as an MCP error whose `data`
/// names the step, the entity and the evidence.
fn world_result(r: Result<Value, WorldError>) -> Result<CallToolResult, McpError> {
    match r {
        Ok(v) => {
            let text = serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string());
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
        }
        Err(e) => Err(McpError::internal_error(e.summary(), Some(e.to_json()))),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::client::BridgeClient;
    use crate::server::LabServer;
    use crate::supervisor::{Supervisor, SupervisorConfig};

    #[test]
    fn world_tools_are_routed() {
        let config = SupervisorConfig {
            install_dir: None,
            dll_path: None,
            patches_dll: None,
            helper_path: None,
            bind: "127.0.0.1".into(),
            port: 8770,
            instance: None,
            telemetry: Default::default(),
        };
        let bridge = Arc::new(BridgeClient::new("127.0.0.1:1", ""));
        let s = LabServer::new(Arc::new(Supervisor::new(bridge, config)));
        for name in [
            "client_entity_find",
            "client_world_click",
            "client_target",
            "client_move_to",
            "client_camera",
        ] {
            let t = s
                .tool_router
                .get(name)
                .unwrap_or_else(|| panic!("{name} is not routed"));
            assert!(t.description.as_deref().unwrap_or_default().len() > 40);
        }
    }
}
