//! Player-administration travel console commands: `.gotoxyz`, `.goto`,
//! `.summon`, `.gotolocation`.
//!
//! All four share legacy's `entity = target or player` precedence
//! (`Target::None` + in-handler `target.unwrap_or(caller_id)`) and all four
//! feed back to the *caller*, never the moved entity (D03).
//!
//! # The two move mechanisms
//!
//! **Same space** — the P26 `.gotoxyz` snap:
//! `update_position_preserving_facing` (spatial grid +
//! `cell_entity.position`), then `note_authorized_teleport` (reseeds
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
    transfer_player_to_loaded_space, transfer_player_to_space, TransferDestination,
    TransferOutcome, TransferRejected,
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
/// grid write is authoritative the instant the position write runs, but
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
    // Subject identity, read off the borrow we already hold. `.summon` /
    // `.goto <player>` move somebody *else*, so the moved player is recorded
    // separately from the GM who issued the command.
    let subject = e.identity();

    // `account_id`/`player_id` name the CALLER on every console log, so one
    // SigNoz filter (`account_id = N`) returns everything that account did —
    // the subject is carried under its own prefixed keys.
    let caller = space_mgr.player_identity(caller_id);
    tracing::info!(
        caller_id,
        account_id = caller.account_id,
        player_id = caller.player_id,
        entity,
        subject_player_id = subject.player_id,
        ?position,
        space_id,
        is_player,
        command = cmd,
        "GM console travel: moving entity within its space"
    );

    // Keep the spatial grid consistent first (writes cell_entity.position and
    // the AoI-relevant grid index — witnesses pick this up on the next AoI
    // tick regardless of player/NPC), then send the authoritative snap for a
    // player target only. Position-only: a travel command changes where the
    // subject is, never which way it is looking.
    space_mgr.update_position_preserving_facing(entity, position, [0.0; 3]);
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
                account_id = caller.account_id,
                player_id = caller.player_id,
                entity,
                subject_player_id = subject.player_id,
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
    // `u32::try_from` only catches the negative half. Zero is neither a
    // negative nor a space id — `allocate_space_id` starts at
    // `(cell_id << 16) | 0` with `cell_id >= 1` — so without this arm a
    // `.gotospace 0` reports "not loaded" as if the GM had named a plausible
    // instance that happened to be gone.
    let space_id = match u32::try_from(raw) {
        Ok(id) if id != 0 => id,
        _ => {
            send_gm_feedback(caller_id, "gotospace: spaceId must be positive", tx).await;
            return;
        }
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
        TravelDestination::Loaded {
            world_name: &world_name,
            space_id,
        },
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

/// How a travel command names its destination — which decides whether the
/// world-name table is consulted at all.
#[derive(Clone, Copy)]
pub(super) enum TravelDestination<'a> {
    /// A world name that came off a chat line, plus the exact instance the
    /// command resolved for itself (`.goto`/`.summon` via P44) or `None` for
    /// D15's default-instance rule (`.gotolocation`). Canonicalised against
    /// `spaces.xml`; a world the table doesn't declare is refused.
    Named {
        world_name: &'a str,
        space_id: Option<u32>,
    },
    /// One instance the command has already confirmed is loaded, named by id
    /// (`.gotospace`). `world_name` was read *off* that instance, so the
    /// table is skipped entirely — otherwise the escape hatch would still
    /// dead-end on `UnknownWorld` for a live instance whose world the table
    /// doesn't declare, which is exactly what it exists to reach.
    Loaded { world_name: &'a str, space_id: u32 },
}

impl<'a> TravelDestination<'a> {
    /// World the destination is in, for the GM-facing log line. Derived for
    /// [`Self::Loaded`], typed-then-canonicalised for [`Self::Named`].
    fn world_name(&self) -> &'a str {
        match *self {
            Self::Named { world_name, .. } | Self::Loaded { world_name, .. } => world_name,
        }
    }

    /// The exact destination instance, when one is known.
    fn space_id(&self) -> Option<u32> {
        match *self {
            Self::Named { space_id, .. } => space_id,
            Self::Loaded { space_id, .. } => Some(space_id),
        }
    }
}

/// Move `subject` to `position` at `dest`, choosing the cheap in-place snap
/// when the destination resolves to the subject's own space and the full
/// cross-space transfer otherwise. `success` is the GM-facing line sent only
/// when the move actually happened.
async fn move_subject(
    cmd: &str,
    caller_id: u32,
    subject: u32,
    dest: TravelDestination<'_>,
    position: [f32; 3],
    success: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let world_name = dest.world_name();
    let dest_space_id = dest.space_id();
    let Some(origin_space_id) = space_mgr.get_entity_space_id(subject) else {
        send_gm_feedback(caller_id, &format!("{cmd}: entity not found"), tx).await;
        return;
    };
    // Same space: no teardown, no loading screen, and NPC subjects keep
    // working (D15 restricts only the cross-space legs). Checked before the
    // facing/identity lookups below because `snap_in_current_space` resolves
    // all three off its own borrow — doing them here first would be three
    // wasted lookups on the common case.
    if dest_space_id == Some(origin_space_id) {
        if snap_in_current_space(cmd, caller_id, subject, position, tx, space_mgr).await {
            send_gm_feedback(caller_id, success, tx).await;
        }
        return;
    }

    // Carried into the cross-space destination below so a transfer doesn't
    // zero the subject's facing at arrival — `TransferDestination::in_world`/
    // `in_instance` default `rotation` to `[0.0; 3]`, and that value flows
    // through `GateTravel` into the destination entity's `direction`
    // unconditionally. (The same-space leg has no such parameter to fill: it
    // goes through `update_position_preserving_facing`, which never writes
    // `direction` at all.)
    let facing = space_mgr
        .get_entity(subject)
        .map(|e| [e.direction.x, e.direction.y, e.direction.z])
        .unwrap_or([0.0; 3]);
    // Both identities are snapshotted BEFORE the transfer: a cross-space
    // transfer tears the subject's entity down and rebuilds it in the
    // destination, so resolving afterwards can race the teardown and report
    // UNKNOWN for the very command that caused it.
    let caller = space_mgr.player_identity(caller_id);
    let subject_id = space_mgr.player_identity(subject);

    // `.gotospace` holds a space id it has already confirmed is loaded, so it
    // takes the by-id entry point and never touches the world-name table —
    // the whole promise of the command is that a live instance is reachable
    // whether or not `spaces.xml` declares its world. Every *named*
    // destination still goes through the canonicalising lookup.
    let outcome = match dest {
        TravelDestination::Loaded { space_id, .. } => {
            transfer_player_to_loaded_space(subject, space_id, position, facing, tx, space_mgr)
                .await
        }
        TravelDestination::Named {
            world_name,
            space_id,
        } => {
            let mut dest = match space_id {
                Some(space_id) => TransferDestination::in_instance(world_name, space_id, position),
                None => TransferDestination::in_world(world_name, position),
            };
            dest.rotation = facing;
            transfer_player_to_space(subject, &dest, tx, space_mgr).await
        }
    };

    // Exhaustive by design: `SameSpace` performs no position move at all, so
    // an `is_ok()` adapter would silently report a teleport that never
    // happened (P45 handoff).
    match outcome {
        Ok(TransferOutcome::Transferred { space_id }) => {
            tracing::info!(
                caller_id,
                account_id = caller.account_id,
                player_id = caller.player_id,
                subject,
                subject_player_id = subject_id.player_id,
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
