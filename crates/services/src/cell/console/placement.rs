//! Selected-entity read/set position + orientation console commands
//! (category J): `.location`, `.rotation` (P18).
//!
//! Distinct from [`super::travel`]'s `.gotoxyz`/`.goto`/`.summon`/
//! `.gotolocation`: those move an entity to a *destination* (another player,
//! a named world) and always mutate. `.location`/`.rotation` are dual-mode —
//! zero args reports the target's current placement, a complete tuple sets
//! it.
//!
//! `.lookat` (the third command P18's ledger entry originally listed) is
//! already implemented in [`super::entity`] (P13/P14-era work) — heading-only
//! "face the caller" rotation. Not this file's concern.
//!
//! Legacy reference: `deprecated/python/cell/commands/Entity.py:101-134`.
//! Both legacy bodies have the same shape — `if z is not None: target.<field>
//! = Atrea.Vector3(x, y, z)`, then unconditionally feed back the current
//! value. The `z is not None` gate means legacy silently *ignored* a 1- or
//! 2-argument invocation and reported as if nothing had been asked for; per
//! D02 ("correct legacy bugs rather than reproducing them") a partial tuple
//! is rejected with an explicit error here instead. Zero args and a complete
//! three-tuple remain the only accepted shapes, matching the registry's
//! `min = 0, max = 3`.
//!
//! ## Orientation representation
//!
//! `CellEntity::direction` is `[pitch, yaw, roll]` in **radians** — not a
//! Cartesian facing vector. The outbound AoI packers read it that way
//! unconditionally for both players and NPCs
//! (`crate::mercury::aoi::{create, update}` call `pack_angle(direction[1])`
//! for yaw, `[0]` for pitch, `[2]` for roll), and the NPC movement tick
//! writes the same convention directly
//! (`cell::service::ticks::npc_movement` sets `direction = (0, yaw, 0)`).
//! Legacy agreed: `SGWPlayer.connected` builds `Vector3(0, heading, 0)`,
//! `SGWSpawnableEntity.lookAt` writes only `rot.y`, and the player row
//! persists `heading = rot.y`. So legacy `.rotation x y z` maps 1:1 onto
//! `direction.x/.y/.z` with no vector math — there is no Euler
//! representation gap to escalate.
//!
//! ## What reaches the client
//!
//! A `direction` write needs no explicit fan-out: the AoI tick emits
//! [`CellToBaseMsg::EntityMoved`] for every entity in a witness's AoI each
//! pass, and that message already carries `direction` alongside `position`
//! (`cell::space_manager::aoi`). Witnesses therefore pick up both a
//! `.location` and a `.rotation` change on the next tick.
//!
//! The moved entity's *own* client is a different matter. A position change
//! on a player target still needs the authoritative
//! `BASEMSG_FORCED_POSITION` snap ([`CellToBaseMsg::TeleportPlayer`]),
//! exactly as `.gotoxyz` does. An orientation change has no equivalent:
//! `compose_forced_position_body` carries entity/space/vehicle/position/
//! prev-position and no angles at all, so there is no wire mechanism to
//! snap a player's own camera yaw. `.rotation` on a player target is
//! therefore server-side + witness-visible only, and the target's next
//! `avatarUpdateExplicit` overwrites it — the same thing legacy did (it
//! wrote `target.rotation` and pushed nothing). The realistic use of
//! `.rotation` is posing an NPC, which works fully.

use cimmeria_common::Vector3;
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Route `.location` / `.rotation` (P18) to their handlers.
pub(super) async fn dispatch(
    name: &str,
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    match name {
        "location" => location(caller_id, target, args, tx, space_mgr).await,
        "rotation" => rotation(caller_id, target, args, tx, space_mgr).await,
        _ => {}
    }
}

/// Result of the shared "zero args or a complete triple" argument gate.
enum Triple {
    /// No args — report the current value, mutate nothing.
    Report,
    /// A complete, finite three-tuple — set it, then report.
    Set([f32; 3]),
    /// Rejected: a partial (1- or 2-arg) tuple, or a malformed/non-finite
    /// component. Feedback has already gone to the caller and **no mutation
    /// has happened** — callers must return immediately.
    Rejected,
}

/// Argument gate shared by both commands. Only `0` and `3` are valid arities
/// (the registry's `max = 3` already rejects `4+` upstream), so this is the
/// single place the "malformed partial tuple causes no mutation" guarantee
/// lives.
async fn parse_triple(
    caller_id: u32,
    name: &str,
    args: &[&str],
    labels: [&str; 3],
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Triple {
    match args.len() {
        0 => Triple::Report,
        3 => {
            let mut out = [0.0f32; 3];
            for (i, label) in labels.iter().enumerate() {
                // `parse_f32` rejects non-finite (NaN/inf) values as well as
                // non-numeric ones, and feeds back its own error line.
                let Some(v) = super::parse_f32(caller_id, args, i, label, tx).await else {
                    return Triple::Rejected;
                };
                out[i] = v;
            }
            Triple::Set(out)
        }
        n => {
            send_gm_feedback(
                caller_id,
                &format!(
                    "{name}: expected no arguments (report) or all three ({} {} {}); \
                     got {n} -- nothing changed",
                    labels[0], labels[1], labels[2]
                ),
                tx,
            )
            .await;
            Triple::Rejected
        }
    }
}

/// `.location` (report) / `.location <x> <y> <z>` (set) — the selected
/// target's position.
///
/// The set path reuses the same snap abstraction the native
/// `gmGotoXYZ`/`gmSummon` handlers and `.gotoxyz` use:
/// `update_position_preserving_facing` (writes `cell_entity.position` **and**
/// the AoI spatial grid, leaving `direction` alone) then
/// `note_authorized_teleport` (reseeds the movement validator's clock so the
/// jump isn't scored as a speed-hack), then `TeleportPlayer` for a player
/// target only.
async fn location(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let set_to = match parse_triple(caller_id, "location", args, ["x", "y", "z"], tx).await {
        Triple::Report => None,
        Triple::Set(p) => Some(p),
        Triple::Rejected => return,
    };

    let Some(e) = space_mgr.get_entity(target) else {
        send_gm_feedback(
            caller_id,
            &format!("location: entity {target} not found"),
            tx,
        )
        .await;
        return;
    };

    if let Some(position) = set_to {
        let space_id = e.space_id.0 as u32;
        let prev_pos = [e.position.x, e.position.y, e.position.z];
        let is_player = e.is_player;

        tracing::info!(
            caller_id,
            target,
            ?position,
            space_id,
            is_player,
            "GM .location: setting entity position"
        );

        // Position-only write: legacy `location` assigns `target.position` and
        // nothing else, so the facing must survive the move.
        space_mgr.update_position_preserving_facing(target, position, [0.0; 3]);
        space_mgr.note_authorized_teleport(target);

        if is_player {
            if let Err(err) = tx
                .send(CellToBaseMsg::TeleportPlayer {
                    entity_id: target,
                    space_id,
                    position,
                    prev_pos,
                })
                .await
            {
                // The cell-side grid write is already authoritative; only the
                // client snap was lost. Don't report a placement we can't
                // vouch for on the client.
                tracing::warn!(
                    caller_id,
                    target,
                    error = %err,
                    "location: base channel closed, snap not sent"
                );
                return;
            }
        }
    }

    // Re-read rather than echoing the requested tuple: the report has to be
    // the entity's *actual* final position, so a silently-dropped write shows
    // up as a stale readout instead of a confirmation of something that
    // didn't happen.
    let Some(p) = space_mgr.get_entity(target).map(|e| e.position) else {
        return;
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "Position of entity {target} is: ({:.3}, {:.3}, {:.3})",
            p.x, p.y, p.z
        ),
        tx,
    )
    .await;
}

/// `.rotation` (report) / `.rotation <pitch> <yaw> <roll>` (set) — the
/// selected target's orientation, in radians, written straight to
/// `direction.x/.y/.z` (see the module doc for why no vector math is
/// involved).
///
/// Unlike [`location`] this makes no spatial-grid write, so there is no
/// `note_authorized_teleport` reseed — the movement validator only ever
/// scores *position* deltas, and turning in place moves nothing. Witnesses
/// see the new facing on the next AoI tick via `EntityMoved.direction`.
async fn rotation(
    caller_id: u32,
    target: u32,
    args: &[&str],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let set_to = match parse_triple(caller_id, "rotation", args, ["pitch", "yaw", "roll"], tx).await
    {
        Triple::Report => None,
        Triple::Set(d) => Some(d),
        Triple::Rejected => return,
    };

    if space_mgr.get_entity(target).is_none() {
        send_gm_feedback(
            caller_id,
            &format!("rotation: entity {target} not found"),
            tx,
        )
        .await;
        return;
    }

    if let Some([pitch, yaw, roll]) = set_to {
        tracing::info!(
            caller_id,
            target,
            pitch,
            yaw,
            roll,
            "GM .rotation: setting entity orientation"
        );
        if let Some(e) = space_mgr.get_entity_mut(target) {
            e.direction = Vector3::new(pitch, yaw, roll);
        }
    }

    let Some(d) = space_mgr.get_entity(target).map(|e| e.direction) else {
        return;
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "Rotation of entity {target} is: (pitch {:.3}, yaw {:.3}, roll {:.3}) rad; \
             heading {:.1} deg",
            d.x,
            d.y,
            d.z,
            d.y.to_degrees()
        ),
        tx,
    )
    .await;
}
