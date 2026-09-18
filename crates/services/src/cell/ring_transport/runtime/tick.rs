//! The 100ms ring deadline scan.
//!
//! Every FSM timer — the four Python delays and the bounded stall aborts H02
//! added — is polled from here, so a transition and the wire effects it
//! produces always move together within one tick.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::super::dispatch::{dispatch_effects, dispatch_release_effects, try_advance_after_load};
use super::super::regions::RingRegion;
use super::super::transporter::{AbortReason, Effect, State};
use crate::cell::arrival::resolve_arrival;
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
///
/// Also reconciles any entity teardown that came through the synchronous
/// `SpaceManager::destroy_entity` path — see
/// [`super::transporter::RingTransporterManager::note_player_gone`] for why
/// that work is deferred to here instead of being applied inline.
pub async fn run_tick_with_engine(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    /// Hard ceiling on transitions per region per tick. The FSM has 4
    /// deadline-driven transitions in a single trip (hide, warmup,
    /// remote_warmup, cooldown) plus the bounded stall abort, which is
    /// terminal; 8 leaves headroom without letting a bug loop forever.
    const MAX_PER_REGION: usize = 8;

    reconcile_pending(tx, space_mgr, engine).await;

    let now = space_mgr.ring_transporters.now();
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

/// Apply the work queued by the synchronous paths that cannot dispatch
/// effects themselves: entities destroyed via `SpaceManager::destroy_entity`,
/// and load-readiness re-checks left by [`forget_player`].
async fn reconcile_pending(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let gone = space_mgr.ring_transporters.take_pending_player_gone();
    for entity_id in gone {
        let emptied = space_mgr.ring_transporters.forget_source_side(entity_id);
        for region_id in emptied {
            let effects = space_mgr.ring_transporters.abort_pair(
                region_id,
                AbortReason::PlayerGone,
                Some(entity_id),
            );
            dispatch_release_effects(effects, tx, space_mgr).await;
        }
    }
    for region_id in space_mgr.ring_transporters.take_pending_load_recheck() {
        try_advance_after_load(region_id, tx, space_mgr, engine).await;
    }
}

/// Apply a single elapsed deadline transition for one region. Factored out
/// of `run_tick_with_engine` so the scanner can re-poll for additional
/// deadlines on the same region within one tick.
async fn run_one_deadline(
    region_id: i32,
    deadline: super::super::transporter::RawDeadline,
    now: std::time::Instant,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // The bounded stall abort is terminal for the trip: no other transition
    // runs for this region this tick, so handle it before the borrow dance
    // below.
    if deadline.is_stall() {
        let effects = space_mgr
            .ring_transporters
            .abort_pair(region_id, AbortReason::Timeout, None);
        dispatch_release_effects(effects, tx, space_mgr).await;
        return;
    }

    // Resolve any cross-region lookups (warmup needs the destination's
    // position) BEFORE taking a `&mut` to the source transporter.
    let (destination_for_warmup, warmup_players, warmup_dst_id): (
        Option<RingRegion>,
        Vec<u32>,
        i32,
    ) = if deadline.is_warmup() {
        let (dst_id, players) = space_mgr
            .ring_transporters
            .get(region_id)
            .map(|t| (t.remote_region_id.unwrap_or(0), t.send_players.clone()))
            .unwrap_or((0, Vec::new()));
        (
            space_mgr.ring_regions.get(&dst_id).cloned(),
            players,
            dst_id,
        )
    } else {
        (None, Vec::new(), 0)
    };

    // `ring_regions` and `ring_transporters` are separate maps and can
    // disagree. Before H02 this branch returned WITHOUT clearing
    // `warmup_at`, so it re-fired every tick forever while the source sat in
    // `SendWarmup` with every passenger locked and hidden — the same
    // unbounded-state defect class as H-B3, one state the audit did not
    // list. Abort the trip instead so the passengers are released.
    if deadline.is_warmup() && destination_for_warmup.is_none() {
        tracing::error!(
            region_id,
            destination_region_id = warmup_dst_id,
            reason = AbortReason::DestinationRegionMissing.as_str(),
            "ring warmup: destination region not loaded — aborting the trip and releasing \
             passengers rather than leaving them locked and hidden in SendWarmup"
        );
        let effects = space_mgr.ring_transporters.abort_pair(
            region_id,
            AbortReason::DestinationRegionMissing,
            None,
        );
        dispatch_release_effects(effects, tx, space_mgr).await;
        return;
    }

    // H01 arrival contract: the destination pad's own row coordinate is the
    // only place a ring arrival is chosen, so validate it here against the
    // destination world's navmesh (respawner fallback on a miss, never the
    // raw input) before the transporter copies it into the teleport effects.
    // Ring rows carry no yaw; the transporter does not use one either.
    let warmup_arrival: Option<[f32; 3]> = destination_for_warmup.as_ref().map(|dst| {
        resolve_arrival(space_mgr, &dst.world_name, [dst.x, dst.y, dst.z], 0.0).position
    });

    let effects: Vec<Effect> = if let Some(t) = space_mgr.ring_transporters.get_mut(region_id) {
        if deadline.is_hide() {
            t.hide_timer_expired()
        } else if deadline.is_warmup() {
            match (destination_for_warmup.as_ref(), warmup_arrival) {
                (Some(dst), Some(arrival)) => t.warmup_timer_expired(arrival, &dst.world_name),
                // Unreachable: the `is_none()` guard above returned already.
                _ => return,
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

    // For warmup we have to update the destination's expected-passenger set
    // BEFORE dispatching the TeleportPlayer effects — same-world teleports
    // synchronously call `mark_player_loaded`, and that won't fire
    // `all_players_loaded` until the expectation is set. The Python original
    // does this in the opposite order (teleport then count update) because
    // its `playerLoaded` callback is genuinely async (waits for the
    // client's `mapLoaded`). We collapse the timing into one tick.
    if deadline.is_warmup() {
        advance_destination_after_warmup(warmup_dst_id, warmup_players, tx, space_mgr, engine)
            .await;
    }

    dispatch_effects(effects, tx, space_mgr, engine).await;
}

/// After the source ring's warmup expires, push the destination ring through
/// RecvWarmup → RemoteLoadWait → (eventually) RemoteWarmup. This is the
/// cross-link work that the Python `__warmupTimerExpired` does inline.
///
/// `dst_id` and `players` are captured by the caller BEFORE
/// `warmup_timer_expired` runs — that call takes `send_players` and resets
/// the source to `Idle` so the next trip can start cleanly.
pub(in crate::cell::ring_transport) async fn advance_destination_after_warmup(
    dst_id: i32,
    players: Vec<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = space_mgr.ring_transporters.now();
    if let Some(dst) = space_mgr.ring_transporters.get_mut(dst_id) {
        // Python order: remoteCountUpdate, then remoteTransport. The order
        // matters because an empty expectation fast-paths into
        // __allPlayersLoaded.
        dst.remote_expect(players);
        if dst.state == State::RecvWarmup {
            dst.remote_transport(now);
        }
    }
    // Same-world teleports were already marked-loaded synchronously by
    // dispatch_effects → mark_player_loaded. If `players_loaded` already
    // satisfies the expectation we need to fire `all_players_loaded` now.
    try_advance_after_load(dst_id, tx, space_mgr, engine).await;
}
