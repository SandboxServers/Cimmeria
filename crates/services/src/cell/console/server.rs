//! Server / maintenance console commands (category G): `.save`,
//! `.reloadmap`, `.reloadres`, `.removerespawner`, `.loglevel`, `.logclient`.
//!
//! These depend on server-lifecycle infrastructure that the Rust server either
//! handles differently from the legacy Python (incremental persistence instead
//! of an explicit `persist()`, startup-time resource loading instead of a
//! runtime `DefMgr.load()`, `RUST_LOG`/env tracing config instead of a runtime
//! log-level map). Where the mechanism genuinely differs, the command reports
//! the real path rather than pretending to act — surfacing a no-op as success
//! would be worse than an honest pointer.
//!
//! Legacy reference: `deprecated/python/cell/commands/Player.py` (`save`,
//! `reloadMap`, `removeRespawner`), `Resource.py` (`reloadResources`),
//! `ConsoleCommands.py` (`setLogLevel`, `setClientLogging`).

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    args: &[&str],
    _target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "save" => {
            send_gm_feedback(
                caller_id,
                "save: player state persists incrementally on each mutation \
                 (mission/inventory/options writes). No explicit full-save step needed.",
                tx,
            )
            .await;
        }
        "reloadmap" => {
            send_gm_feedback(
                caller_id,
                "reloadmap: not wired — relog or use gate travel to reload the map.",
                tx,
            )
            .await;
        }
        "reloadres" => {
            send_gm_feedback(
                caller_id,
                "reloadres: resource caches load at server startup; runtime reload is not wired \
                 (restart to pick up seed edits).",
                tx,
            )
            .await;
        }
        "removerespawner" => {
            // Validate the arg so the command at least parses like the others.
            if super::parse_i32(caller_id, args, 0, "respawnerId", tx)
                .await
                .is_none()
            {
                return;
            }
            let _ = space_mgr; // no per-player respawner mutation path yet
            send_gm_feedback(
                caller_id,
                "removerespawner: per-player respawner removal is not wired (respawners are \
                 server-defined; edit db/resources/Worlds/Seed/respawners.sql).",
                tx,
            )
            .await;
        }
        "loglevel" => {
            send_gm_feedback(
                caller_id,
                "loglevel: the Rust server's log level is set via RUST_LOG / env at startup; \
                 runtime reload is not wired.",
                tx,
            )
            .await;
        }
        "logclient" => {
            send_gm_feedback(
                caller_id,
                "logclient: server→client log forwarding is not wired; use the SigNoz/OTLP \
                 pipeline for server logs.",
                tx,
            )
            .await;
        }
        _ => {}
    }
}
