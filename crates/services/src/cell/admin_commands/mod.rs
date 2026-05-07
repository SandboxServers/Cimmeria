//! In-chat admin command dispatcher.
//!
//! The original Stargate Worlds server uses `.` (period) prefix on
//! say/emote/yell messages as the GM-command sigil — see
//! [`python/cell/SGWPlayer.py:1836`]:
//!
//! ```python
//! if len(message) > 0 and message[0] == '.':
//!     Command.exec(self, message[1:])
//!     return
//! ```
//!
//! `/`-prefixed commands don't work for new server-side commands: the
//! game client has its own hardcoded slash-command parser and rejects
//! any leading-`/` text not on its built-in list with "invalid command"
//! BEFORE it ever reaches the server. The `.` prefix bypasses that —
//! the client treats it as ordinary chat and forwards it through the
//! normal `sendPlayerCommunication` path.
//!
//! This module is the dispatcher entry point. It owns its own
//! command-table (separate from the broader `cimmeria-commands`
//! registry, which is designed for stateless string-result handlers
//! and can't reach `space_mgr` or the cell→base channel without an
//! invasive signature change). When that registry grows real cell
//! integration, this dispatcher can delegate to it; for now it's a
//! lightweight cell-local table.
//!
//! Currently registered:
//!
//! | Command           | Effect                                                |
//! |-------------------|-------------------------------------------------------|
//! | `.world <name> [x y z]` (alias `.goto`) | Cross-world teleport via `GateTravel` |

use tokio::sync::mpsc;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

/// Dispatch a `.command` from a player's chat message body. The leading
/// `.` is already stripped by the caller. Returns `true` if the body
/// matched a known command and was handled (caller must NOT broadcast
/// the original message), `false` if no command matched (caller falls
/// through to ordinary broadcast).
pub(super) async fn dispatch(
    entity_id: u32,
    body: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mut parts = body.split_whitespace();
    let command = match parts.next() {
        Some(c) => c.to_ascii_lowercase(),
        // Body was just whitespace after the dot — treat as no-command.
        None => return false,
    };
    let args: Vec<&str> = parts.collect();

    match command.as_str() {
        "world" | "goto" => {
            cmd_world(entity_id, &args, tx, space_mgr).await;
            true
        }
        _ => {
            tracing::info!(
                entity_id,
                command,
                args = ?args,
                "admin: unknown command"
            );
            // Returning `true` consumes the message so it doesn't broadcast
            // as chat. A typo on `.world` shouldn't leak the GM's intent
            // to nearby players.
            true
        }
    }
}

/// `.world <name> [x y z]` — teleport the caller to the named world.
///
/// Mirrors the cross-world fork of
/// [`super::cell_methods::player::combat::handle_respawn`]:
///   1. Flush dirty bandolier ammo so the new world load reads fresh state.
///   2. **`space_mgr.destroy_entity(entity_id)`** — load-bearing. Without
///      this, the cell still has the player in the old space when base
///      receives `GateTravel`, and the subsequent
///      `BaseToCellMsg::CreateEntity` collides with the existing entity
///      id — symptom: the player ends up half-torn or the transition
///      silently no-ops.
///   3. Send `CellToBaseMsg::GateTravel`, which replays the world-entry
///      flow (RESET_ENTITIES → create-player → enter-world).
///
/// If x/y/z are omitted, uses [`default_arrival_position`] for known
/// worlds, or `(0, 0, 0)` for unknown worlds. Explicit coords override.
async fn cmd_world(
    entity_id: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if args.is_empty() {
        tracing::info!(
            entity_id,
            "admin: .world usage: .world <name> [x y z] (e.g., .world Castle)"
        );
        return;
    }

    let world_name = args[0];

    // Parse optional coords. Partial-coord input (e.g., only x given)
    // intentionally falls back to defaults — partial parsing would land
    // the player somewhere they didn't intend.
    let position = match (args.get(1), args.get(2), args.get(3)) {
        (Some(x), Some(y), Some(z)) => match (x.parse(), y.parse(), z.parse()) {
            (Ok(x), Ok(y), Ok(z)) => [x, y, z],
            _ => {
                tracing::warn!(
                    entity_id,
                    world = world_name,
                    args = ?args,
                    "admin: .world failed to parse x/y/z; using world default"
                );
                default_arrival_position(world_name)
            }
        },
        _ => default_arrival_position(world_name),
    };

    // Step 1: flush dirty bandolier ammo so the new world load reads
    // a coherent snapshot rather than reconstructing from a partial
    // dirty set. Mirrors the respawn cross-world fork.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            super::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
    } else {
        tracing::warn!(
            entity_id,
            world = world_name,
            "admin: .world has no source entity in cell; sending GateTravel anyway (best effort)"
        );
    }

    // Step 2: tear down the cell-side entity in the current space so
    // base's CreateEntity reply doesn't collide with a still-present
    // entity id. This is the load-bearing step.
    space_mgr.destroy_entity(entity_id);

    tracing::info!(
        entity_id,
        world = world_name,
        position = ?position,
        "admin: .world dispatching GateTravel"
    );

    // Step 3: dispatch the cross-world replay. Rotation reset to zero —
    // entity is destroyed at this point. World-entry re-anchors the
    // player anyway.
    if let Err(e) = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: world_name.to_string(),
            position,
            rotation: [0.0; 3],
        })
        .await
    {
        tracing::error!(
            entity_id,
            world = world_name,
            error = %e,
            "admin: .world GateTravel send failed"
        );
    }
}

/// Hardcoded arrival positions for known worlds. Picked to land the
/// player at a sensible content entry point — for Castle, near
/// `Castle_Coppleman` (mission 701 NPC). Unknown worlds return
/// `(0, 0, 0)`; the caller should provide explicit coords or accept
/// wherever spawn 0 puts them.
fn default_arrival_position(world_name: &str) -> [f32; 3] {
    match world_name {
        // Near Castle_Coppleman (spawn_id 87) at (352.69, 70.27, 952.32).
        // Pulled back a few units so the player faces him on arrival.
        "Castle" => [350.0, 70.27, 945.0],
        // Castle_CellBlock starting cell — same default as new-character
        // spawn for testing the cellblock arc.
        "Castle_CellBlock" => [-322.5, 73.47, -209.83],
        // SGC briefing room — Gen. Hammond's table.
        "SGC_W1" => [200.0, 1.31, 43.6],
        _ => [0.0, 0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_mgr() -> SpaceManager {
        SpaceManager::new(12)
    }

    /// `.world Castle` dispatches a `GateTravel` to the cell→base
    /// channel and returns `true` (consumed; do not broadcast).
    #[tokio::test]
    async fn dot_world_dispatches_gate_travel_and_consumes_message() {
        let mut mgr = empty_mgr();
        let (tx, mut rx) = mpsc::channel(16);

        let consumed = dispatch(1, "world Castle", &tx, &mut mgr).await;
        assert!(consumed, ".world must be consumed (return true)");

        let msg = rx.try_recv().expect(".world must enqueue a CellToBaseMsg");
        match msg {
            CellToBaseMsg::GateTravel {
                entity_id,
                target_world_name,
                ..
            } => {
                assert_eq!(entity_id, 1);
                assert_eq!(target_world_name, "Castle");
            }
            other => panic!("expected GateTravel, got {other:?}"),
        }
    }

    /// Explicit coords after the world name override the default.
    /// Pin so a future regression on the parser doesn't silently fall
    /// back without complaint.
    #[tokio::test]
    async fn dot_world_with_explicit_coords_uses_them() {
        let mut mgr = empty_mgr();
        let (tx, mut rx) = mpsc::channel(16);

        let consumed = dispatch(1, "world Castle 100 50 200", &tx, &mut mgr).await;
        assert!(consumed);

        match rx.try_recv().expect("must dispatch GateTravel") {
            CellToBaseMsg::GateTravel { position, .. } => {
                assert_eq!(position, [100.0, 50.0, 200.0]);
            }
            other => panic!("expected GateTravel, got {other:?}"),
        }
    }

    /// Unknown world with no coords falls back to (0, 0, 0). Documents
    /// the typo-safety behavior.
    #[tokio::test]
    async fn dot_world_unknown_world_falls_back_to_origin() {
        let mut mgr = empty_mgr();
        let (tx, mut rx) = mpsc::channel(16);

        let consumed = dispatch(1, "world UnknownWorldXYZ", &tx, &mut mgr).await;
        assert!(consumed);

        match rx.try_recv().expect("must dispatch GateTravel") {
            CellToBaseMsg::GateTravel {
                target_world_name,
                position,
                ..
            } => {
                assert_eq!(target_world_name, "UnknownWorldXYZ");
                assert_eq!(position, [0.0, 0.0, 0.0]);
            }
            other => panic!("expected GateTravel, got {other:?}"),
        }
    }

    /// `.goto` is an alias for `.world` (matches python's command
    /// vocabulary).
    #[tokio::test]
    async fn dot_goto_is_alias_for_world() {
        let mut mgr = empty_mgr();
        let (tx, mut rx) = mpsc::channel(16);

        let consumed = dispatch(1, "goto Castle", &tx, &mut mgr).await;
        assert!(consumed);

        match rx.try_recv().expect("must dispatch GateTravel") {
            CellToBaseMsg::GateTravel {
                target_world_name, ..
            } => {
                assert_eq!(target_world_name, "Castle");
            }
            other => panic!("expected GateTravel, got {other:?}"),
        }
    }

    /// Unknown commands are still consumed (return `true`) — the
    /// trailing `.typo` shouldn't leak to nearby players as ordinary
    /// chat. They just no-op with a log line.
    #[tokio::test]
    async fn unknown_command_is_consumed_but_no_op() {
        let mut mgr = empty_mgr();
        let (tx, mut rx) = mpsc::channel(16);

        let consumed = dispatch(1, "completelybogus", &tx, &mut mgr).await;
        assert!(
            consumed,
            "unknown commands must be consumed so the leading-dot text doesn't leak as chat"
        );
        assert!(
            rx.try_recv().is_err(),
            "unknown commands must not produce side-effect messages"
        );
    }

    /// Whitespace-only body (e.g., user typed just `.`) returns
    /// `false` so the caller can broadcast the original chat. We
    /// don't want every accidental dot to be silently swallowed.
    #[tokio::test]
    async fn empty_body_does_not_consume() {
        let mut mgr = empty_mgr();
        let (tx, _rx) = mpsc::channel(16);

        let consumed = dispatch(1, "   ", &tx, &mut mgr).await;
        assert!(
            !consumed,
            "whitespace-only body must fall through to broadcast"
        );
    }
}
