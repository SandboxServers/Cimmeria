//! Stargate travel handler for the CellService.
//!
//! Two entry points, matching the two halves of the 2009 flow in
//! `deprecated/python/cell/SGWPlayer.py:2046-2129`:
//!
//! 1. [`handle_dial_gate`] — `onDialGate`. Validates the destination,
//!    arms a 4-second dial (`beginDialing`), and returns. It does NOT
//!    travel: the gate has to open first.
//! 2. [`handle_stargate_region_entered`] — the player walks into the
//!    gate's `REGION_FLAG_Stargate` volume (`GenericRegion.py:174-176`
//!    dispatches this instead of a generic region event). If the dial is
//!    open, `stargatePassed` emits `Stargate_CrossGate` and then travels.
//!
//! Stargate destinations are loaded from `resources.stargates` at startup
//! and cached in `SpaceManager.stargates`; the gate regions come from
//! `resources.point_sets` (`type = 'AreaSet'`, `flags & 2`).
//!
//! **Fallback.** Only twelve of the seeded stargates' worlds have a gate
//! region (`Castle.Stargate`, `Harset.Stargate`, …). On a world without
//! one there is no way to reach the crossing, so a dial there travels
//! immediately, as this handler did before CA10. That keeps every
//! currently-working route working; it is a deliberate divergence from
//! the Python, which would simply have left the player standing there.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// `pub(crate)` so the base-side fan-out byte test can drive the real
// emitter rather than hand-building `WitnessEntityMethod` messages.
pub(crate) mod sequences;
mod tick;

pub(crate) use sequences::world_has_stargate_region;
pub use tick::gate_dial_tick;

use sequences::{origin_gate_event_set, send_gate_sequence, EVENT_STARGATE_CROSS_GATE};

// ── Dial ─────────────────────────────────────────────────────────────────────

/// Handle the `onDialGate` cell method call.
///
/// Validates the target stargate address and arms the dial. Four seconds
/// later [`gate_dial_tick`] fires `Stargate_MakeGate` and the gate becomes
/// passable; the actual world transition happens when the player walks
/// into the gate region.
///
/// `target_address_id == -1` is the client's cancel sentinel and drops any
/// armed dial (`SGWPlayer.cancelDialing`).
#[tracing::instrument(
    name = "gate_travel.dial",
    level = "info",
    skip_all,
    fields(entity_id, target_address_id)
)]
pub async fn handle_dial_gate(
    entity_id: u32,
    target_address_id: i32,
    _source_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // target_address_id == -1 means cancel dialing. The Python's
    // `cancelDialing` emits no sequence — not `Stargate_DestroyGate`
    // (6103), not anything — so neither do we (D-CA10).
    if target_address_id == -1 {
        match space_mgr.cancel_gate_dial(entity_id) {
            Some(prev) => tracing::debug!(
                entity_id,
                cancelled_target = prev.target_address_id,
                "onDialGate: cancel dial"
            ),
            None => tracing::debug!(entity_id, "onDialGate: cancel dial (nothing armed)"),
        }
        return;
    }

    // Every rejection below cancels the dial in flight, because
    // `SGWPlayer.onDialGate` does: each of its three reject branches
    // (`SGWPlayer.py:2050`, `:2061`, `:2067`) calls `self.cancelDialing()`
    // before warning and returning. Without it a rejected re-dial leaves
    // the PREVIOUS destination armed and, once its timer expires,
    // crossable — the player walks into the gate and is sent somewhere
    // they did not dial.

    // Look up the destination stargate from the DB cache
    let gate = match space_mgr.stargates.get(&target_address_id) {
        Some(g) => g.clone(),
        None => {
            space_mgr.cancel_gate_dial(entity_id);
            tracing::warn!(
                entity_id,
                target_address_id,
                "onDialGate: invalid stargate address — pending dial cancelled"
            );
            return;
        }
    };

    // Validate the entity exists and get its current world
    let current_world = match space_mgr.get_entity_world_name(entity_id) {
        Some(w) => w,
        None => {
            space_mgr.cancel_gate_dial(entity_id);
            tracing::warn!(entity_id, "onDialGate: entity not found");
            return;
        }
    };

    // Don't travel to the same world (Python also checks this implicitly)
    if gate.world_name == current_world {
        space_mgr.cancel_gate_dial(entity_id);
        tracing::debug!(
            entity_id, target_address_id, world = %gate.world_name,
            "onDialGate: already in destination world — pending dial cancelled"
        );
        return;
    }

    let player_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    // `dialingStargate` is the gate the player is STANDING at, not the
    // one they dialled — its event set owns the 6100/6113 sequences.
    let origin_event_set_id = origin_gate_event_set(space_mgr, &current_world);

    if !world_has_stargate_region(space_mgr, &current_world) {
        tracing::warn!(
            entity_id, target_address_id,
            from = %current_world, to = %gate.world_name,
            reason = "no_stargate_region",
            "onDialGate: origin world has no REGION_FLAG_Stargate region — \
             travelling on the dial instead of on the crossing (no gate \
             volume exists for the player to walk into)"
        );
        // The player still dials and still crosses, just in one step, so
        // both content triggers fire in the same order a real crossing
        // would produce them.
        super::content::fire_stargate_dialed(
            entity_id,
            player_id,
            &gate.world_name,
            engine,
            tx,
            space_mgr,
        )
        .await;
        super::content::fire_stargate_crossed(
            entity_id,
            player_id,
            &gate.world_name,
            engine,
            tx,
            space_mgr,
        )
        .await;
        perform_gate_travel(entity_id, target_address_id, tx, space_mgr).await;
        return;
    }

    tracing::info!(
        entity_id, target_address_id,
        from = %current_world, to = %gate.world_name,
        origin_event_set_id = ?origin_event_set_id,
        "Gate travel: dial accepted, gate opens in 4s"
    );

    space_mgr.begin_gate_dial(
        entity_id,
        target_address_id,
        gate.world_name.clone(),
        origin_event_set_id,
    );

    super::content::fire_stargate_dialed(
        entity_id,
        player_id,
        &gate.world_name,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

// ── Crossing ─────────────────────────────────────────────────────────────────

/// Everything the crossing owes the client and the content engine, in one
/// place: `onSequence(Stargate_CrossGate)` to the dialer and every witness
/// of them, then the `stargate_crossed` content trigger.
///
/// Deliberately self-contained and parameterised rather than reading the
/// dial state, so any passage path can call it: this branch's
/// [`handle_stargate_region_entered`] does today, and Harset packet H01's
/// generic `REGION_FLAG_Stargate` entry point will call the same function
/// once the two branches meet. It does NOT travel — the caller owns the
/// world transition, because it also owns deciding whether the gate was
/// crossable in the first place.
///
/// Emission order matches `SGWPlayer.stargatePassed`
/// (`SGWPlayer.py:2117-2129`): sequence, then `fire('stargate::passage')`,
/// then the move. Getting the sequence out before `RESET_ENTITIES` is the
/// load-bearing part — after the teardown the client has no entity to play
/// it on.
pub async fn on_stargate_passage(
    entity_id: u32,
    player_id: i32,
    origin_event_set_id: Option<i32>,
    destination_world: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    send_gate_sequence(
        entity_id,
        origin_event_set_id,
        EVENT_STARGATE_CROSS_GATE,
        tx,
        space_mgr,
    )
    .await;

    super::content::fire_stargate_crossed(
        entity_id,
        player_id,
        destination_world,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

/// The player entered a `REGION_FLAG_Stargate` volume.
///
/// `SGWPlayer.stargatePassed`: no-op unless a dial is armed AND the gate
/// has opened, then [`on_stargate_passage`], then the transition.
#[tracing::instrument(
    name = "gate_travel.cross",
    level = "info",
    skip_all,
    fields(entity_id)
)]
pub async fn handle_stargate_region_entered(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let Some(dial) = space_mgr.gate_dial(entity_id).cloned() else {
        tracing::debug!(
            entity_id,
            "stargate region entered with no gate dialled — no travel"
        );
        return;
    };
    if !dial.passable {
        tracing::debug!(
            entity_id,
            target_address_id = dial.target_address_id,
            "stargate region entered before the gate opened — no travel"
        );
        return;
    }

    tracing::info!(
        entity_id,
        target_address_id = dial.target_address_id,
        target_world = %dial.target_world_name,
        "Stargate passed — crossing"
    );

    let player_id = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);
    on_stargate_passage(
        entity_id,
        player_id,
        dial.origin_event_set_id,
        &dial.target_world_name,
        tx,
        space_mgr,
        engine,
    )
    .await;

    // `stargatePassed` clears `dialedAddress`/`gatePassable` before
    // `moveTo`. `destroy_entity` inside `perform_gate_travel` would scrub
    // it anyway, but clearing here keeps the "one crossing per dial"
    // invariant true even if the travel is refused below.
    space_mgr.cancel_gate_dial(entity_id);

    perform_gate_travel(entity_id, dial.target_address_id, tx, space_mgr).await;
}

// ── Transition ───────────────────────────────────────────────────────────────

/// Tear the entity out of its space and hand the world transition to the
/// BaseApp. Unchanged from the pre-CA10 tail of `handle_dial_gate`.
async fn perform_gate_travel(
    entity_id: u32,
    target_address_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(gate) = space_mgr.stargates.get(&target_address_id).cloned() else {
        tracing::warn!(
            entity_id,
            target_address_id,
            "gate travel: destination vanished from the stargate cache"
        );
        return;
    };

    // Stage D: world transition destroys the cell entity and re-creates it on
    // the destination world. Flush any pending bandolier ammo writes before
    // teardown — anything still in `bandolier_ammo_dirty` after this is lost
    // to the cross-world re-spawn.
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if let Some(player_id) = entity.player_id {
            super::cell_methods::inventory::flush_dirty_bandolier_ammo(entity, player_id, tx).await;
        }
    }

    // Tell BaseApp to perform the world transition (RESET_ENTITIES + new world
    // entry) BEFORE removing the entity locally. A closed base channel must
    // not leave the player destroyed cell-side with no transfer in flight —
    // that is an "un-spaced" player who can only recover by relogging. Same
    // ordering the native `gmGotoLocation` handler already uses.
    if let Err(e) = tx
        .send(CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name: gate.world_name.clone(),
            position: [gate.x, gate.y, gate.z],
            rotation: [0.0, 0.0, gate.yaw],
            destination_ring_id: None,
            // Stargate travel resolves the destination by world name.
            destination_space_id: None,
        })
        .await
    {
        tracing::error!(
            entity_id, world = %gate.world_name, error = %e,
            "gate travel: base channel closed — entity left in place, no transfer"
        );
        return;
    }

    // Cancel any open trade before the entity goes. `destroy_entity` doesn't
    // clean trade state, and this helper early-returns once `get_entity`
    // misses — so gating on it afterwards would be a silent no-op, leaving the
    // traveller's partner with a dangling `trade_partner_entity_id` and no
    // `onTradeResults(Cancelled)`. Both lifecycle arms call it for the same
    // reason; stargate travel is just as much a departure.
    super::cell_methods::player::trade::cancel_trade_on_disconnect(entity_id, tx, space_mgr).await;

    // Remove entity from current space (CellService side)
    space_mgr.destroy_entity(entity_id);
}

#[cfg(test)]
mod tests;
