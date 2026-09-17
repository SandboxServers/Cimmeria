//! Player-administration travel console commands: `.gotoxyz`, `.goto`,
//! `.summon`, `.gotolocation`.
//!
//! All four share legacy's `entity = target or player` precedence
//! (`Target::None` + in-handler `target.unwrap_or(caller_id)`) and all four
//! feed back to the *caller*, never the moved entity (D03).
//!
//! # The two move mechanisms
//!
//! **Same space** — the P26 `.gotoxyz` snap: `update_entity_position` (spatial
//! grid + `cell_entity.position`), then `note_authorized_teleport` (reseeds
//! the movement-validator clock so the snap doesn't trip speed-hack
//! detection — called unconditionally for players and NPCs, matching
//! `gmSummon`'s precedent; `move_clock` is only ever consulted against
//! *client* movement packets, which NPCs never send), then `TeleportPlayer`
//! (`BASEMSG_FORCED_POSITION`) for a player subject only. An NPC has no
//! client to push that to, but witnesses still see it move through the normal
//! AoI broadcast once the grid updates. See
//! [`snap_in_current_space`].
//!
//! **Cross space / cross world** — P45's
//! [`crate::cell::space_transfer::transfer_player_to_space`], which validates
//! the whole destination before any teardown and then drives the
//! teardown → `pending_world_entry` → re-enter handshake. D15 restricts that
//! leg to *player* subjects, because the handshake needs a client; the
//! primitive enforces it.
//!
//! [`move_subject`] picks between them, so the cheap in-place snap is used
//! whenever the destination resolves to the subject's own space and a full
//! loading screen is only paid for when the space actually changes. That
//! also means an **NPC subject still works for a same-space move** — D15
//! restricts only the cross-space legs, and legacy's `entity.teleportTo(...)`
//! never cared what kind of entity it was.
//!
//! # Legacy sources
//!
//! - `.gotoxyz` — `deprecated/python/cell/commands/Player.py:368-382`.
//! - `.goto` / `.summon` — `Player.py:298-341` (`PlayersByName` exact-match
//!   lookup, resolved here by P44's
//!   [`crate::cell::space_manager::SpaceManager::find_online_player_by_name`]).
//! - `.gotolocation` — `Player.py:344-365`.
//!
//! Legacy's own GM strings are reproduced verbatim where they exist; the
//! places this module deviates are called out at their call sites.
//!
//! # Layout
//!
//! This file holds the two shared mechanisms ([`snap_in_current_space`],
//! [`move_subject`]), the dispatch routing, and `.gotoxyz` — the one command
//! that is *only* ever a same-space snap. [`named_destination`] holds the
//! three commands whose destination is resolved from a **name** (a player's
//! for `.goto`/`.summon`, a world's for `.gotolocation`) and which therefore
//! have to cope with the cross-space leg.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::space_transfer::{
    transfer_player_to_space, TransferDestination, TransferOutcome, TransferRejected,
};

mod named_destination;

/// `.gotoxyz <x> <y> <z>` — teleport the selected target (or the caller, if
/// nothing is selected) to the given coordinates within its current space.
///
/// `Target::None` with in-handler `target.unwrap_or(caller_id)` resolution —
/// unlike most typed dot commands, legacy `gotoXYZ` makes the target
/// optional with a caller fallback rather than requiring a selection.
pub(super) async fn goto_xyz(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // `parse_f32` already rejects non-finite (NaN/inf) values — matches the
    // native `gmGotoXYZ`'s separate finite check via the console's shared
    // parse layer instead of duplicating it here.
    let Some(x) = super::parse_f32(caller_id, args, 0, "x", tx).await else {
        return;
    };
    let Some(y) = super::parse_f32(caller_id, args, 1, "y", tx).await else {
        return;
    };
    let Some(z) = super::parse_f32(caller_id, args, 2, "z", tx).await else {
        return;
    };
    let position = [x, y, z];

    let entity = target.unwrap_or(caller_id);
    if snap_in_current_space("gotoxyz", caller_id, entity, position, tx, space_mgr).await {
        send_gm_feedback(
            caller_id,
            &format!("gotoxyz: moved entity {entity} to ({x}, {y}, {z})"),
            tx,
        )
        .await;
    }
}

/// Move `entity` inside the space it is already in, authoritatively.
///
/// Returns `true` when the move is complete *and* reportable. `false` means
/// either the entity was gone (a GM-facing line was already sent) or the base
/// channel closed before the player's forced-position push — the cell-side
/// grid write is authoritative the instant `update_entity_position` runs, but
/// the caller must not claim a snap that never reached the client.
///
/// `cmd` only prefixes the not-found line so each command keeps its own
/// wording.
async fn snap_in_current_space(
    cmd: &str,
    caller_id: u32,
    entity: u32,
    position: [f32; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(e) = space_mgr.get_entity(entity) else {
        send_gm_feedback(caller_id, &format!("{cmd}: entity not found"), tx).await;
        return false;
    };
    let space_id = e.space_id.0 as u32;
    let prev_pos = [e.position.x, e.position.y, e.position.z];
    let is_player = e.is_player;
    // Captured *before* the grid write: `update_entity_position` overwrites
    // `direction` unconditionally from its `[i8; 3]` parameter, so the
    // `[0, 0, 0]` below would otherwise silently zero the entity's facing on
    // every travel command that reaches this shared mechanism (`.gotoxyz`,
    // `.goto`, `.summon`, `.gotolocation`'s same-space leg) — the same bug
    // P18's `.location` found and fixed for its own call site. Restored
    // immediately after (see `console/placement.rs::location` for the
    // identical pattern).
    let facing = e.direction;

    tracing::info!(
        caller_id,
        entity,
        ?position,
        space_id,
        is_player,
        command = cmd,
        "GM console travel: moving entity within its space"
    );

    // Keep the spatial grid consistent first (writes cell_entity.position and
    // the AoI-relevant grid index — witnesses pick this up on the next AoI
    // tick regardless of player/NPC), then send the authoritative snap for a
    // player target only.
    space_mgr.update_entity_position(entity, position, [0, 0, 0], [0.0; 3]);
    if let Some(e) = space_mgr.get_entity_mut(entity) {
        e.direction = facing;
    }
    space_mgr.note_authorized_teleport(entity);

    if is_player {
        if let Err(err) = tx
            .send(CellToBaseMsg::TeleportPlayer {
                entity_id: entity,
                space_id,
                position,
                prev_pos,
            })
            .await
        {
            tracing::warn!(
                caller_id,
                entity,
                command = cmd,
                error = %err,
                "console travel: base channel closed, snap not sent"
            );
            return false; // don't claim a snap that never sent.
        }
    }

    true
}

/// `.gotospace <spaceId> <x> <y> <z>` — teleport the selected target (or the
/// caller) to explicit coordinates in one exact **loaded space instance**,
/// named by id.
///
/// **Deliberate deviation — legacy has no equivalent.** Every other travel
/// command resolves its destination through the world-name table, which means
/// a GM can only reach a world `spaces.xml` declares, spelled the way
/// `spaces.xml` spells it, and on an instanced world only the instance D15's
/// default rule happens to pick. This is the escape hatch: the space id is
/// the runtime identity of a loaded instance, so the world name is derived
/// *from* it rather than looked up, and any live instance is reachable —
/// including a second copy of an instanced world with nobody in it to
/// `.goto`.
///
/// Authority is the same server-side `access_level` gate every `.`-command
/// runs behind (see [`crate::cell::console`]); nothing here is asserted by
/// the client. Coordinates are **not** navmesh-checked — placing a GM off the
/// walkable mesh is the point — but they are finite-checked by `parse_f32`
/// and the destination instance must actually be loaded.
pub(super) async fn goto_space(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(raw) = super::parse_i32(caller_id, args, 0, "spaceId", tx).await else {
        return;
    };
    let Ok(space_id) = u32::try_from(raw) else {
        send_gm_feedback(caller_id, "gotospace: spaceId must be positive", tx).await;
        return;
    };
    let Some(x) = super::parse_f32(caller_id, args, 1, "x", tx).await else {
        return;
    };
    let Some(y) = super::parse_f32(caller_id, args, 2, "y", tx).await else {
        return;
    };
    let Some(z) = super::parse_f32(caller_id, args, 3, "z", tx).await else {
        return;
    };

    // The world name is derived from the instance, never typed — so this
    // command cannot dead-end on an unknown-world rejection.
    let Some(world_name) = space_mgr.world_name_for_space(space_id).map(str::to_owned) else {
        send_gm_feedback(
            caller_id,
            &format!("gotospace: space {space_id} is not loaded"),
            tx,
        )
        .await;
        return;
    };

    let subject = target.unwrap_or(caller_id);
    move_subject(
        "gotospace",
        caller_id,
        subject,
        &world_name,
        Some(space_id),
        [x, y, z],
        &format!("Moving entity {subject} to space {space_id} ({x}, {y}, {z})"),
        tx,
        space_mgr,
    )
    .await;
}

/// Route `.goto` / `.summon` / `.gotolocation` (P46) and `.gotospace` to
/// their handlers.
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "goto" => named_destination::goto(caller_id, target, args, tx, space_mgr).await,
        "summon" => named_destination::summon(caller_id, target, args, tx, space_mgr).await,
        "gotolocation" => {
            named_destination::goto_location(caller_id, target, args, tx, space_mgr).await
        }
        "gotospace" => goto_space(caller_id, target, args, tx, space_mgr).await,
        _ => {}
    }
}

/// Move `subject` to `position` in `world_name`, choosing the cheap in-place
/// snap when the destination resolves to the subject's own space and the full
/// cross-space transfer otherwise. `success` is the GM-facing line sent only
/// when the move actually happened.
///
/// `dest_space_id` is the **exact** instance when the caller knows it
/// (`.goto`/`.summon` via P44, `.gotolocation` into the subject's own world);
/// `None` hands instance selection to P45's D15 default.
async fn move_subject(
    cmd: &str,
    caller_id: u32,
    subject: u32,
    world_name: &str,
    dest_space_id: Option<u32>,
    position: [f32; 3],
    success: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(origin_space_id) = space_mgr.get_entity_space_id(subject) else {
        send_gm_feedback(caller_id, &format!("{cmd}: entity not found"), tx).await;
        return;
    };
    // Carried into the cross-space destination below so a transfer doesn't
    // zero the subject's facing at arrival — `TransferDestination::in_world`/
    // `in_instance` default `rotation` to `[0.0; 3]`, and that value flows
    // through `GateTravel` into the destination entity's `direction`
    // unconditionally. Same root cause as the same-space fix above and
    // P18's `.location`.
    let facing = space_mgr
        .get_entity(subject)
        .map(|e| [e.direction.x, e.direction.y, e.direction.z])
        .unwrap_or([0.0; 3]);

    // Same space: no teardown, no loading screen, and NPC subjects keep
    // working (D15 restricts only the cross-space legs).
    if dest_space_id == Some(origin_space_id) {
        if snap_in_current_space(cmd, caller_id, subject, position, tx, space_mgr).await {
            send_gm_feedback(caller_id, success, tx).await;
        }
        return;
    }

    let mut dest = match dest_space_id {
        Some(space_id) => TransferDestination::in_instance(world_name, space_id, position),
        None => TransferDestination::in_world(world_name, position),
    };
    dest.rotation = facing;

    // Exhaustive by design: `SameSpace` performs no position move at all, so
    // an `is_ok()` adapter would silently report a teleport that never
    // happened (P45 handoff).
    match transfer_player_to_space(subject, &dest, tx, space_mgr).await {
        Ok(TransferOutcome::Transferred { space_id }) => {
            tracing::info!(
                caller_id,
                subject,
                origin_space_id,
                destination_space_id = ?space_id,
                world = world_name,
                command = cmd,
                "GM console travel: cross-space transfer enqueued"
            );
            send_gm_feedback(caller_id, success, tx).await;
        }
        // Pre-empted by the `dest_space_id == origin` check above for every
        // current call shape, but the primitive owns the authoritative
        // resolution — honour its answer instead of assuming it can't happen.
        Ok(TransferOutcome::SameSpace { .. }) => {
            if snap_in_current_space(cmd, caller_id, subject, position, tx, space_mgr).await {
                send_gm_feedback(caller_id, success, tx).await;
            }
        }
        Err(rejected) => {
            let text = match &rejected {
                // Legacy's own wording, unprefixed — `Player.py:361`.
                TransferRejected::UnknownWorld(_) => rejected.describe(),
                other => format!("{cmd}: {}", other.describe()),
            };
            send_gm_feedback(caller_id, &text, tx).await;
        }
    }
}
