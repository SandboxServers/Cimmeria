//! The three travel commands whose destination is resolved from a name:
//! `.goto <player>`, `.summon <player>` and `.gotolocation <world> x y z`.
//!
//! All three end in [`super::move_subject`], which picks between the
//! same-space snap and P45's cross-space transfer. What differs is only how
//! each one derives `(world, instance, position)`:
//!
//! | Command | Subject moved | Destination |
//! |---|---|---|
//! | `.goto` | `target or player` | the named player's exact space + position |
//! | `.summon` | the named player | `target or player`'s space + position |
//! | `.gotolocation` | `target or player` | the named world, explicit coordinates |
//!
//! Legacy: `deprecated/python/cell/commands/Player.py:298-365`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use super::{move_subject, SpaceManager};
use crate::cell::console::parse_f32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::PlayerNameLookup;

/// Legacy `Player.py:308`/`:331` — the name is not a live player on this
/// CellApp at all.
const NOT_AVAILABLE: &str = "Player is not available on this CellApp";

/// Legacy `Player.py:313`/`:336` — the name resolved, but the player is not
/// in a reachable space.
const NOT_REACHABLE: &str = "Player is not on any reachable space";

/// `.goto <name>` — move the selected target (or the caller) to a named
/// online player's exact position **and exact instance**.
///
/// Legacy `goto` (`Player.py:298-318`): `entity = target or player`, then
/// `entity.teleportTo(destination.position, 0.0, destination.space.worldName)`.
/// Legacy passed only a *world name*, which on an instanced world is
/// ambiguous; P44's lookup hands back the destination player's real
/// `space_id`, and that is used verbatim — re-resolving the world name would
/// go through the first/default-instance rule and drop the GM into a
/// different copy of the map.
pub(super) async fn goto(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Arity 1 is guaranteed by the registry; the guard is here so a direct
    // `exec` call in a test can't index out of bounds.
    let Some(name) = args.first().copied() else {
        return;
    };

    // Legacy checks the name before it ever touches `entity`, so a bad name
    // reports the name error even when the selection is also broken.
    let Some((dest_entity, dest_space_id)) =
        resolve_named_player(caller_id, name, tx, space_mgr).await
    else {
        return;
    };

    // `Found` implies membership in the space's `players` set, so both of
    // these resolve; the guard maps the impossible case onto legacy's
    // not-reachable wording rather than inventing a new string.
    let Some((position, world_name)) = space_mgr
        .get_entity(dest_entity)
        .map(|e| [e.position.x, e.position.y, e.position.z])
        .zip(
            space_mgr
                .world_name_for_space(dest_space_id)
                .map(str::to_owned),
        )
    else {
        let caller = space_mgr.player_identity(caller_id);
        tracing::warn!(
            caller_id,
            account_id = caller.account_id,
            player_id = caller.player_id,
            dest_entity,
            dest_space_id,
            "goto: resolved player has no entity or no live space"
        );
        send_gm_feedback(caller_id, NOT_REACHABLE, tx).await;
        return;
    };

    move_subject(
        "goto",
        caller_id,
        target.unwrap_or(caller_id),
        &world_name,
        Some(dest_space_id),
        position,
        &format!("Teleporting to player <{name}>"),
        tx,
        space_mgr,
    )
    .await;
}

/// `.summon <name>` — move a named online player to the selected target's (or
/// the caller's) position and instance.
///
/// Legacy `summon` (`Player.py:321-341`) is `goto` with the roles swapped:
/// the *named* player is the one moved, and `entity = target or player` is
/// only the destination anchor. The anchor may therefore be an NPC — D15's
/// players-only restriction applies to the entity being *moved*, which here
/// is always the named player.
pub(super) async fn summon(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(name) = args.first().copied() else {
        return;
    };

    let Some((victim, _victim_space)) = resolve_named_player(caller_id, name, tx, space_mgr).await
    else {
        return;
    };

    let anchor = target.unwrap_or(caller_id);
    let Some(anchor_space_id) = space_mgr.get_entity_space_id(anchor) else {
        send_gm_feedback(caller_id, "summon: entity not found", tx).await;
        return;
    };
    let Some((position, world_name)) = space_mgr
        .get_entity(anchor)
        .map(|e| [e.position.x, e.position.y, e.position.z])
        .zip(
            space_mgr
                .world_name_for_space(anchor_space_id)
                .map(str::to_owned),
        )
    else {
        send_gm_feedback(caller_id, "summon: entity not found", tx).await;
        return;
    };

    move_subject(
        "summon",
        caller_id,
        victim,
        &world_name,
        Some(anchor_space_id),
        position,
        &format!("Summoning player <{name}>"),
        tx,
        space_mgr,
    )
    .await;
}

/// `.gotolocation <worldName> <x> <y> <z>` — move the selected target (or the
/// caller) to explicit coordinates in a named world.
///
/// Legacy `gotoLocation` (`Player.py:344-365`) validated the world against
/// `world_info` and reported `"Unable to find world: %s"`; here that check is
/// `SpaceManager::canonical_world_name`, whose rejection carries the same
/// wording.
///
/// Instance selection is D15's first/default loaded instance — **except**
/// when the named world is the one the subject is already in, where their own
/// instance is named explicitly. `TransferDestination::in_world` resolves to
/// the *oldest* loaded instance, so a GM standing in instance C of an
/// instanced world would otherwise be yanked to instance A through a full
/// loading screen just to change coordinates.
pub(super) async fn goto_location(
    caller_id: u32,
    target: Option<u32>,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(world_name) = args.first().copied() else {
        return;
    };
    // `parse_f32` rejects NaN/inf up front; the transfer primitive re-checks
    // finiteness for the callers that don't come through this layer.
    let Some(x) = parse_f32(caller_id, args, 1, "x", tx).await else {
        return;
    };
    let Some(y) = parse_f32(caller_id, args, 2, "y", tx).await else {
        return;
    };
    let Some(z) = parse_f32(caller_id, args, 3, "z", tx).await else {
        return;
    };

    let subject = target.unwrap_or(caller_id);
    let Some(origin_space_id) = space_mgr.get_entity_space_id(subject) else {
        send_gm_feedback(caller_id, "gotolocation: entity not found", tx).await;
        return;
    };
    // Case-insensitive, matching `SpaceManager::canonical_world_name`: a GM
    // typing `castle_cellblock` while standing in `Castle_CellBlock` must get
    // the in-place snap, not a full loading screen into a different instance.
    let dest_space_id = match space_mgr.world_name_for_space(origin_space_id) {
        Some(w) if w.eq_ignore_ascii_case(world_name) => Some(origin_space_id),
        _ => None,
    };

    // Captured before the move: a cross-world transfer destroys the origin
    // entity, so the name is gone by the time the feedback line is built.
    let subject_name = subject_display_name(space_mgr, subject);

    move_subject(
        "gotolocation",
        caller_id,
        subject,
        world_name,
        dest_space_id,
        [x, y, z],
        // Legacy's `"Moving entity %s to %s (%f, %f, %f)"`. Deliberate
        // deviation: Rust's `{}` float formatting rather than C's `%f`, so
        // `10` renders as `10` and not `10.000000` — matching the `.gotoxyz`
        // line the GM sees right next to it.
        &format!("Moving entity {subject_name} to {world_name} ({x}, {y}, {z})"),
        tx,
        space_mgr,
    )
    .await;
}

/// Wording for P44's `Ambiguous` outcome.
///
/// **Deliberate deviation — legacy has no equivalent.** `PlayersByName` is a
/// dict keyed by name, so it structurally cannot hold two players with the
/// same name and never needed a message for it. Rust's lookup can observe the
/// violation, and collapsing it onto either legacy string would hide a real
/// data problem behind "no such player". See `handoffs/p44.md`.
const AMBIGUOUS_NAME_HINT: &str = "Multiple players named";

/// Resolve `name` to a live player's `(entity_id, space_id)`, feeding back
/// legacy's own error wording on every failure shape.
async fn resolve_named_player(
    caller_id: u32,
    name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Option<(u32, u32)> {
    match space_mgr.find_online_player_by_name(name) {
        PlayerNameLookup::Found {
            entity_id,
            space_id,
        } => Some((entity_id, space_id)),
        PlayerNameLookup::NotFound => {
            send_gm_feedback(caller_id, NOT_AVAILABLE, tx).await;
            None
        }
        PlayerNameLookup::InTransition { entity_id } => {
            tracing::debug!(
                caller_id,
                entity_id,
                player_name = name,
                "console travel: named player is not in a space"
            );
            send_gm_feedback(caller_id, NOT_REACHABLE, tx).await;
            None
        }
        PlayerNameLookup::Ambiguous { entity_ids } => {
            // The lookup already logged the uniqueness violation at `error`;
            // this is just the GM-facing half.
            send_gm_feedback(
                caller_id,
                &format!(
                    "{AMBIGUOUS_NAME_HINT} <{name}> -- refusing to guess ({} matches)",
                    entity_ids.len()
                ),
                tx,
            )
            .await;
            None
        }
    }
}

/// Best-effort display name for a feedback line — legacy's
/// `entity.getName()`.
fn subject_display_name(space_mgr: &SpaceManager, entity_id: u32) -> String {
    space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.character_name.clone().or_else(|| e.npc_name.clone()))
        .unwrap_or_else(|| entity_id.to_string())
}
