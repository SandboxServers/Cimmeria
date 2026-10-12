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
use crate::supervisor::world::hover::HoverRequest;
use crate::supervisor::world::io::LiveWorld;
use crate::supervisor::world::movement::MoveRequest;
use crate::supervisor::world::{camera, click, find, hover, movement, WorldError};

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
        description = "Click an entity (entity_id or name) or a world point in the 3D view like a player: turn the camera onto it when off screen, put the cursor on it, check the client's mouse-over (another entity in the way fails as occluded unless force), click with real buttons (default right = interact), report the target and the windows opened or closed. `expect` (target, window, any, nothing) decides pass/fail. Reports the straight-line distance to the entity and whether it is inside the server's 5 m interact range (result.range); a right-click that opens nothing from farther away says so. Reports native_level."
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
        description = "Walk to a point (server metres by default, or space: client), an entity (entity_id or name; the goal follows it), through optional waypoints, with the movement keys: W held, turning with the client's bound turn keys (TurnLeft/TurnRight, Q/E by default), closed loop on the player's actor position every 100 ms. Learns the turn rate and key sign from the pawn's yaw, fails as did-not-turn when the keys do nothing, jumps/strafes out of snags, fails on a teleport, a snag it cannot clear, or the timeout (default 60 s) with the distance left. Always releases the keys."
    )]
    async fn client_move_to(
        &self,
        Parameters(a): Parameters<MoveRequest>,
    ) -> Result<CallToolResult, McpError> {
        let sup = self.supervisor.clone();
        let mut guard = HeldKeysGuard::new(move || async move {
            let mut io = LiveWorld::new(&sup);
            movement::release_stray(&mut io).await;
        });
        let mut io = LiveWorld::new(&self.supervisor);
        let r = movement::run(&mut io, a).await;
        // Finished (and released by `run`): nothing left for the guard to do.
        guard.disarm();
        world_result(r)
    }

    #[tool(
        description = "Camera control with real input: raw mouse-look (yaw_counts, pitch_counts), zoom (zoom_notches of the wheel, or zoom to a distance), and/or face an entity or point (turns until it projects near the screen centre, closed loop on worldToPixel), then absolute pitch_deg (readout degrees) and yaw_offset_deg (camera world yaw, -180..180, after any face), each closed loop. zoom is the same as zoom_to and is applied before the face. Reports the camera actor's pose before and after. Reports pitch_clamped (with a warning) when the camera ends at the +-78.75 deg pitch limit; a face that starts there brings it back to 10 deg first and says so under face.pitch_clamped."
    )]
    async fn client_camera(
        &self,
        Parameters(a): Parameters<CameraRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(camera::run(&mut io, a).await)
    }

    #[tool(
        description = "Hover a target or a world point with the cursor, without clicking, and score it: whether the client's mouse-over (and no HUD window) is right at the aim pixel and at 8 points around it, and how far the pixel is from the avatar and the screen edge. For calibrating spec poses."
    )]
    async fn client_hover_probe(
        &self,
        Parameters(a): Parameters<HoverRequest>,
    ) -> Result<CallToolResult, McpError> {
        let mut io = LiveWorld::new(&self.supervisor);
        world_result(hover::run(&mut io, a).await)
    }
}

/// Releases the walk's keys if the tool future is dropped mid-walk (the
/// client cancelled the call): movement::run releases them on every exit it
/// gets to, but a dropped future gets to none. A held W or turn key would
/// wreck every later lab step. Covers the client's bound turn keys too.
struct HeldKeysGuard<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    release: Option<F>,
}

impl<F, Fut> HeldKeysGuard<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    /// A guard that runs `release` (on the runtime) if dropped armed.
    fn new(release: F) -> Self {
        Self {
            release: Some(release),
        }
    }

    /// The walk finished and released its own keys.
    fn disarm(&mut self) {
        self.release = None;
    }
}

impl<F, Fut> Drop for HeldKeysGuard<F, Fut>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    fn drop(&mut self) {
        let Some(release) = self.release.take() else {
            return;
        };
        let Ok(rt) = tokio::runtime::Handle::try_current() else {
            return;
        };
        rt.spawn(release());
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
            "client_hover_probe",
        ] {
            let t = s
                .tool_router
                .get(name)
                .unwrap_or_else(|| panic!("{name} is not routed"));
            assert!(t.description.as_deref().unwrap_or_default().len() > 40);
        }
    }

    /// A dropped (cancelled) tool future must run the key release; a
    /// disarmed guard (the walk released its own keys) must not.
    #[tokio::test]
    async fn held_keys_guard_releases_on_a_dropped_future_only() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::time::Duration;

        let released = Arc::new(AtomicU32::new(0));
        let make = |counter: Arc<AtomicU32>| {
            super::HeldKeysGuard::new(move || async move {
                counter.fetch_add(1, Ordering::SeqCst);
            })
        };

        // Cancelled mid-walk: the timeout drops the future holding the guard.
        let g = make(released.clone());
        let walk = async move {
            let _g = g;
            tokio::time::sleep(Duration::from_secs(3600)).await;
        };
        assert!(tokio::time::timeout(Duration::from_millis(20), walk)
            .await
            .is_err());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(released.load(Ordering::SeqCst), 1, "keys not released");

        // Finished walk: disarmed, nothing more to release.
        let mut g = make(released.clone());
        g.disarm();
        drop(g);
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(released.load(Ordering::SeqCst), 1, "released after disarm");
    }
}
