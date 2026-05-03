//! Public ring-transport entry points: `interact()`, `selectDestination()`,
//! region-trigger crossings, and the per-tick deadline scan.
//!
//! All of these turn external events (chain action, cell-method call,
//! point-set crossing, tick) into FSM transitions on a [`super::transporter::RingTransporter`]
//! plus an [`Effect`] dispatch via [`super::dispatch`].

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::dispatch::{dispatch_effect, dispatch_effects, try_advance_after_load};
use super::regions::RingRegion;
use super::transporter::{Effect, State};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Per-tick deadline scan with an explicit engine handle. Used by the cell
/// loop's tick scheduler.
///
/// Drains every elapsed deadline at `now` before returning. A single tick
/// can span multiple deadlines on the same region — the documented timings
/// (3.5s hide, 4.0s warmup, 3.0s remote-warmup, 2.5s cooldown) are tighter
/// than worst-case tick lag, so re-scanning until quiescent preserves the
/// timeline under jitter. Bounded per region by `MAX_PER_REGION` to keep a
/// hypothetical FSM bug from spinning.
pub async fn run_tick_with_engine(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    /// Hard ceiling on transitions per region per tick. The FSM has 4
    /// deadline-driven transitions in a single trip (hide, warmup,
    /// remote_warmup, cooldown); 8 leaves headroom without letting a bug
    /// loop forever.
    const MAX_PER_REGION: usize = 8;
    let now = Instant::now();
    let mut iterations: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
    loop {
        let ready = space_mgr.ring_transporters.ready_regions(now);
        if ready.is_empty() {
            break;
        }
        let mut made_progress = false;
        for (region_id, deadline) in ready {
            let count = iterations.entry(region_id).or_insert(0);
            if *count >= MAX_PER_REGION {
                tracing::error!(
                    region_id,
                    max = MAX_PER_REGION,
                    "ring tick: hit per-region transition cap — possible FSM loop"
                );
                continue;
            }
            *count += 1;
            run_one_deadline(region_id, deadline, now, tx, space_mgr, engine).await;
            made_progress = true;
        }
        if !made_progress {
            break;
        }
    }
}

/// Apply a single elapsed deadline transition for one region. Factored out
/// of `run_tick_with_engine` so the scanner can re-poll for additional
/// deadlines on the same region within one tick.
async fn run_one_deadline(
    region_id: i32,
    deadline: super::transporter::RawDeadline,
    now: Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Resolve any cross-region lookups (warmup needs the destination's
    // position) BEFORE taking a `&mut` to the source transporter.
    let (destination_for_warmup, warmup_num_players, warmup_dst_id): (
        Option<RingRegion>,
        u32,
        i32,
    ) = if deadline.is_warmup() {
        let (dst_id, num_players) = space_mgr
            .ring_transporters
            .get(region_id)
            .map(|t| (t.remote_region_id.unwrap_or(0), t.send_players.len() as u32))
            .unwrap_or((0, 0));
        (
            space_mgr.ring_regions.get(&dst_id).cloned(),
            num_players,
            dst_id,
        )
    } else {
        (None, 0, 0)
    };

    let effects: Vec<Effect> = if let Some(t) = space_mgr.ring_transporters.get_mut(region_id) {
        if deadline.is_hide() {
            t.hide_timer_expired()
        } else if deadline.is_warmup() {
            match destination_for_warmup.as_ref() {
                Some(dst) => t.warmup_timer_expired([dst.x, dst.y, dst.z], &dst.world_name),
                None => {
                    tracing::error!(region_id, "ring warmup: destination region not loaded");
                    return;
                }
            }
        } else if deadline.is_remote_warmup() {
            t.remote_warmup_timer_expired(now)
        } else if deadline.is_cooldown() {
            t.cooldown_timer_expired()
        } else {
            return;
        }
    } else {
        return;
    };

    // For warmup we have to update the destination's `num_remote_players`
    // BEFORE dispatching the TeleportPlayer effects — same-world teleports
    // synchronously call `mark_player_loaded`, and that won't fire
    // `all_players_loaded` until the count is set. The Python original
    // does this in the opposite order (teleport then count update) because
    // its `playerLoaded` callback is genuinely async (waits for the
    // client's `mapLoaded`). We collapse the timing into one tick.
    if deadline.is_warmup() {
        advance_destination_after_warmup(warmup_dst_id, warmup_num_players, tx, space_mgr, engine)
            .await;
    }

    dispatch_effects(effects, tx, space_mgr, engine).await;
}

/// After the source ring's warmup expires, push the destination ring through
/// RecvWarmup → RemoteLoadWait → (eventually) RemoteWarmup. This is the
/// cross-link work that the Python `__warmupTimerExpired` does inline.
///
/// `dst_id` and `num_players` are captured by the caller BEFORE
/// `warmup_timer_expired` runs — that call clears `send_players` and resets
/// the source to `Idle` so the next trip can start cleanly.
pub(super) async fn advance_destination_after_warmup(
    dst_id: i32,
    num_players: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if let Some(dst) = space_mgr.ring_transporters.get_mut(dst_id) {
        // Python order: remoteCountUpdate, then remoteTransport. The order
        // matters because remoteCountUpdate(0) fast-paths into __allPlayersLoaded.
        dst.remote_count_update(num_players);
        if dst.state == State::RecvWarmup {
            dst.remote_transport();
        }
    }
    // Same-world teleports were already marked-loaded synchronously by
    // dispatch_effects → mark_player_loaded. If `players_loaded` already
    // satisfies the count we need to fire `all_players_loaded` now.
    try_advance_after_load(dst_id, tx, space_mgr, engine).await;
}

/// `interact()` entry point — called by the `TriggerTransporter` action
/// executor. Sets `ringSourceId` on the player and sends the destination list.
pub async fn handle_interact(
    region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if space_mgr.ring_transporters.get(region_id).is_none() {
        tracing::warn!(
            region_id,
            entity_id,
            "TriggerTransporter: no transporter loaded for region"
        );
        return;
    }

    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.ring_source_id = Some(region_id);
    }

    let effect = match space_mgr.ring_transporters.get(region_id) {
        Some(t) => t.interact(entity_id),
        None => return,
    };
    dispatch_effect(effect, tx, space_mgr, engine).await;
}

/// `selectDestination()` — called by the `setRingTransporterDestination`
/// inbound cell method handler.
pub async fn handle_select_destination(
    source_region_id: i32,
    destination_region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let src_world = match space_mgr.ring_transporters.get(source_region_id) {
        Some(src) => {
            if let Err(e) = src.validate_destination(destination_region_id) {
                tracing::warn!(
                    source_region_id, destination_region_id, entity_id, error = %e,
                    "selectDestination: rejected"
                );
                return;
            }
            src.world_name.clone()
        }
        None => {
            tracing::warn!(
                source_region_id,
                "selectDestination: source transporter not loaded"
            );
            return;
        }
    };
    let dst_world = match space_mgr.ring_transporters.get(destination_region_id) {
        Some(dst) => dst.world_name.clone(),
        None => {
            tracing::warn!(
                destination_region_id,
                "selectDestination: destination transporter not loaded"
            );
            return;
        }
    };
    // Reject before any state transitions — running the FSM half-way for a
    // teleport we can't fulfill leaves both rings stuck mid-cycle.
    if src_world != dst_world {
        tracing::warn!(
            source_region_id, destination_region_id, entity_id,
            %src_world, %dst_world,
            "selectDestination: cross-world ring travel not yet supported"
        );
        return;
    }

    if let Some(src) = space_mgr.ring_transporters.get_mut(source_region_id) {
        src.enter_send_wait(destination_region_id);
    }
    {
        let dst_state = space_mgr
            .ring_transporters
            .get(destination_region_id)
            .map(|d| d.state);
        if dst_state != Some(State::Idle) {
            tracing::warn!(
                destination_region_id,
                ?dst_state,
                "selectDestination: destination busy — aborting"
            );
            // Reset the source we just nudged into SendWait.
            if let Some(src) = space_mgr.ring_transporters.get_mut(source_region_id) {
                src.state = State::Idle;
                src.remote_region_id = None;
            }
            return;
        }
        if let Some(dst) = space_mgr.ring_transporters.get_mut(destination_region_id) {
            dst.remote_wait(source_region_id);
        }
    }

    // Python: clear `ringSourceId` and remember `destinationRingId` so the
    // destination's `playerLoaded` callback can route the player. The dest id
    // is cleared in `mark_player_loaded` once the destination ring picks the
    // player up (matching the Python `playerLoaded` lifecycle).
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.ring_source_id = None;
        player.destination_ring_id = Some(destination_region_id);
    }

    let auto_start = space_mgr
        .ring_transporters
        .get(source_region_id)
        .is_some_and(|t| t.should_auto_start());
    if auto_start {
        kick_off_warmup(
            source_region_id,
            destination_region_id,
            tx,
            space_mgr,
            engine,
        )
        .await;
    }
}

/// Hook called from the existing region-trigger path when a player crosses a
/// generic region boundary. If the region's `point_set_id` matches a known
/// ring pad, forward the enter/exit to the FSM.
pub async fn handle_region_trigger(
    point_set_id: i32,
    entering: bool,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let region_id = match space_mgr
        .ring_point_set_to_region
        .get(&point_set_id)
        .copied()
    {
        Some(id) => id,
        None => return,
    };

    if let Some(t) = space_mgr.ring_transporters.get_mut(region_id) {
        t.region_triggered(entering, entity_id);
    }

    let auto_start = space_mgr
        .ring_transporters
        .get(region_id)
        .is_some_and(|t| t.should_auto_start());
    if auto_start {
        let dst_id = space_mgr
            .ring_transporters
            .get(region_id)
            .and_then(|t| t.remote_region_id);
        if let Some(dst_id) = dst_id {
            kick_off_warmup(region_id, dst_id, tx, space_mgr, engine).await;
        }
    }
}

/// Drive the source from SendWait → SendWarmup, the destination from RecvWait
/// → RecvWarmup, and dispatch the source's start-up effects (PlaySequence,
/// onTeleportOut, LockMovement). Shared between the destination-selection
/// path and the player-walks-on-pad path.
async fn kick_off_warmup(
    source_region_id: i32,
    destination_region_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let effects = match space_mgr.ring_transporters.get_mut(source_region_id) {
        Some(t) => t.start_sending(Instant::now()),
        None => return,
    };
    if let Some(dst) = space_mgr.ring_transporters.get_mut(destination_region_id) {
        if dst.state == State::RecvWait {
            dst.remote_send();
        }
    }
    dispatch_effects(effects, tx, space_mgr, engine).await;
}
