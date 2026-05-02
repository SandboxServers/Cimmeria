//! Runtime glue: dispatches [`super::ring_transport::Effect`] values into
//! actual wire calls + state-machine cross-link work, and runs the per-tick
//! deadline scan.
//!
//! Kept separate from [`crate::cell::ring_transport`] so the FSM stays unit-
//! testable without a `tokio::mpsc` runtime / live `SpaceManager`.

use std::time::Instant;

use cimmeria_common::Vector3;
use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use super::content;
use super::messages::CellToBaseMsg;
use super::ring_transport::{
    build_on_ring_transporter_list, Effect, RegionEvent, RingRegion, State,
};
use super::space_manager::SpaceManager;

/// `BSF_MovementLock` — bit 6 of `state_field`. See
/// `crates/entity/src/cell_entity.rs` for the full bit layout.
pub const BSF_MOVEMENT_LOCK: u32 = 1 << 6;

/// `onStateFieldUpdate` (SGWBeing interface, flat index 19).
const METHOD_ON_STATE_FIELD_UPDATE: u16 = 19;
/// `onSequence` (SGWSpawnableEntity own, flat index 1).
const METHOD_ON_SEQUENCE: u16 = 1;
/// `onVisible` (SGWSpawnableEntity own, flat index 8) — alias for
/// `crate::mercury::method_idx::ON_VISIBLE`.
const METHOD_ON_VISIBLE: u16 = crate::mercury::method_idx::ON_VISIBLE;
/// `onRingTransporterList` (SGWPlayer own, flat index 133).
pub const METHOD_ON_RING_TRANSPORTER_LIST: u16 = 133;

/// KISMET_VIEW_EventInvoker viewType passed to `onSequence`. Same value used
/// elsewhere for region-driven kismet — the camera follows the triggering
/// player.
const KISMET_VIEW_EVENT_INVOKER: u8 = 3;

/// Dispatch a single [`Effect`] to the wire / world.
///
/// Returns a `Pin<Box<...>>` future to break the recursion cycle that
/// `Effect::TeleportPlayer` introduces: it triggers `mark_player_loaded` →
/// `try_advance_after_load` → more `dispatch_effects`. Without box-pinning,
/// rustc rejects the infinitely sized `Future`.
pub fn dispatch_effect<'a>(
    effect: Effect,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
    engine: &'a ChainEngine,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(dispatch_effect_inner(effect, tx, space_mgr, engine))
}

async fn dispatch_effect_inner(
    effect: Effect,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match effect {
        Effect::PlaySequence { entity_id, event_set_id, region_event } => {
            send_play_sequence(entity_id, event_set_id, region_event, tx, space_mgr).await;
        }
        Effect::OnTeleportOut { entity_id, region_id, destination_id } => {
            // No client wire call: this is a script-level event hook. The
            // current content engine has no `teleport_out` trigger registered,
            // so we just log. Add a TriggerType / loader entry if/when content
            // authoring needs it.
            tracing::debug!(entity_id, region_id, destination_id, "ring: onTeleportOut event (no chain trigger registered)");
        }
        Effect::LockMovement { entity_id } => {
            update_state_flag(entity_id, BSF_MOVEMENT_LOCK, true, tx, space_mgr).await;
        }
        Effect::UnlockMovement { entity_id } => {
            update_state_flag(entity_id, BSF_MOVEMENT_LOCK, false, tx, space_mgr).await;
        }
        Effect::HidePlayer { entity_id } => {
            send_visible(entity_id, false, tx).await;
        }
        Effect::ShowPlayer { entity_id } => {
            send_visible(entity_id, true, tx).await;
        }
        Effect::TeleportPlayer { entity_id, position, world_name, destination_region_id } => {
            same_world_teleport(entity_id, position, &world_name, tx, space_mgr).await;
            // Mark the player as "loaded" on the destination ring immediately —
            // Castle_CellBlock is non-instanced and same-world, so we don't
            // wait for a real `mapLoaded` round trip. Cross-world rings need a
            // different hook (BaseToCell `PlayerLoaded` after world re-entry);
            // see TeleportCrossWorld branch.
            mark_player_loaded(destination_region_id, entity_id, tx, space_mgr, engine).await;
        }
        Effect::TeleportCrossWorld { entity_id, position, world_name, destination_region_id } => {
            // TODO: cross-world ring travel needs a CellToBase message akin to
            // GateTravel that re-creates the player on the destination world,
            // and a callback that drives `RingTransporter::player_loaded` on
            // the destination. None of the live region pairs in the seed data
            // cross worlds (Castle 1↔2 is same-world), so leaving this as a
            // warn rather than a half-implementation.
            let _ = destination_region_id;
            tracing::warn!(
                entity_id, ?position, %world_name,
                "ring transport: cross-world teleport not implemented yet — player will not travel"
            );
        }
        Effect::FireTeleportIn { entity_id, region_id } => {
            let player_id = space_mgr.get_entity(entity_id)
                .and_then(|e| e.player_id).unwrap_or(0);
            content::fire_teleport_in(entity_id, player_id, region_id, engine, tx, space_mgr).await;
        }
        Effect::SendDestinationList { entity_id, source_region_id, destinations } => {
            send_destination_list(entity_id, source_region_id, &destinations, tx, space_mgr).await;
        }
    }
}

/// Apply a list of effects sequentially.
pub async fn dispatch_effects(
    effects: Vec<Effect>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    for effect in effects {
        dispatch_effect(effect, tx, space_mgr, engine).await;
    }
}

/// Per-tick deadline scan with an explicit engine handle. Used by the cell
/// loop's tick scheduler.
pub async fn run_tick_with_engine(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = Instant::now();
    let ready = space_mgr.ring_transporters.ready_regions(now);
    if ready.is_empty() {
        return;
    }

    for (region_id, deadline) in ready {
        // Resolve any cross-region lookups (warmup needs the destination's
        // position) BEFORE taking a `&mut` to the source transporter.
        let (destination_for_warmup, warmup_num_players, warmup_dst_id): (Option<RingRegion>, u32, i32) = if deadline.is_warmup() {
            let (dst_id, num_players) = space_mgr.ring_transporters.get(region_id)
                .map(|t| (t.remote_region_id.unwrap_or(0), t.send_players.len() as u32))
                .unwrap_or((0, 0));
            (space_mgr.ring_regions.get(&dst_id).cloned(), num_players, dst_id)
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
                        continue;
                    }
                }
            } else if deadline.is_remote_warmup() {
                t.remote_warmup_timer_expired(now)
            } else if deadline.is_cooldown() {
                t.cooldown_timer_expired()
            } else {
                continue;
            }
        } else {
            continue;
        };

        // For warmup we have to update the destination's `num_remote_players`
        // BEFORE dispatching the TeleportPlayer effects — same-world teleports
        // synchronously call `mark_player_loaded`, and that won't fire
        // `all_players_loaded` until the count is set. The Python original
        // does this in the opposite order (teleport then count update) because
        // its `playerLoaded` callback is genuinely async (waits for the
        // client's `mapLoaded`). We collapse the timing into one tick.
        if deadline.is_warmup() {
            advance_destination_after_warmup(warmup_dst_id, warmup_num_players, tx, space_mgr, engine).await;
        }

        dispatch_effects(effects, tx, space_mgr, engine).await;
    }
}

/// After the source ring's warmup expires, push the destination ring through
/// RecvWarmup → RemoteLoadWait → (eventually) RemoteWarmup. This is the
/// cross-link work that the Python `__warmupTimerExpired` does inline.
///
/// `dst_id` and `num_players` are captured by the caller BEFORE
/// `warmup_timer_expired` runs — that call clears `send_players` and resets
/// the source to `Idle` so the next trip can start cleanly.
async fn advance_destination_after_warmup(
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

/// Mark a player as loaded on the destination ring and, if the destination is
/// ready, fire its `all_players_loaded` transition.
async fn mark_player_loaded(
    dst_region_id: i32,
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    if let Some(dst) = space_mgr.ring_transporters.get_mut(dst_region_id) {
        // `player_loaded` is idempotent on the same eid.
        let _ = dst.player_loaded(entity_id);
    }
    // Python clears `destinationRingId` here once the destination ring has
    // taken responsibility for the player.
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.destination_ring_id = None;
    }
    try_advance_after_load(dst_region_id, tx, space_mgr, engine).await;
}

/// Check if the destination has all players loaded; if so, trigger the
/// `all_players_loaded` transition. Handles both the "real" path (sequence +
/// timer) and the fast path (no players → skip straight to cooldown).
async fn try_advance_after_load(
    dst_region_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = Instant::now();
    let (do_fire, do_fast_path) = match space_mgr.ring_transporters.get(dst_region_id) {
        Some(t) => {
            let ready = t.state == State::RemoteLoadWait
                && t.players_loaded.len() as u32 == t.num_remote_players;
            let empty = ready && t.players_loaded.is_empty();
            (ready, empty)
        }
        None => (false, false),
    };
    if !do_fire {
        return;
    }
    let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
        Some(t) => t.all_players_loaded(now),
        None => return,
    };
    dispatch_effects(effects, tx, space_mgr, engine).await;

    // Python: if no one loaded, immediately call __remoteWarmupTimerExpired,
    // and from there __cooldownTimerExpired (since players_loaded is empty
    // there too).
    if do_fast_path {
        let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
            Some(t) => t.remote_warmup_timer_expired(now),
            None => return,
        };
        dispatch_effects(effects, tx, space_mgr, engine).await;
        let effects = match space_mgr.ring_transporters.get_mut(dst_region_id) {
            Some(t) => t.cooldown_timer_expired(),
            None => return,
        };
        dispatch_effects(effects, tx, space_mgr, engine).await;
    }
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
        tracing::warn!(region_id, entity_id, "TriggerTransporter: no transporter loaded for region");
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
    if let Some(src) = space_mgr.ring_transporters.get(source_region_id) {
        if let Err(e) = src.validate_destination(destination_region_id) {
            tracing::warn!(
                source_region_id, destination_region_id, entity_id, error = %e,
                "selectDestination: rejected"
            );
            return;
        }
    } else {
        tracing::warn!(source_region_id, "selectDestination: source transporter not loaded");
        return;
    }
    if !space_mgr.ring_transporters.regions.contains_key(&destination_region_id) {
        tracing::warn!(destination_region_id, "selectDestination: destination transporter not loaded");
        return;
    }

    if let Some(src) = space_mgr.ring_transporters.get_mut(source_region_id) {
        src.enter_send_wait(destination_region_id);
    }
    {
        let dst_state = space_mgr.ring_transporters.get(destination_region_id).map(|d| d.state);
        if dst_state != Some(State::Idle) {
            tracing::warn!(
                destination_region_id, ?dst_state,
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

    let auto_start = space_mgr.ring_transporters.get(source_region_id)
        .map_or(false, |t| t.should_auto_start());
    if auto_start {
        kick_off_warmup(source_region_id, destination_region_id, tx, space_mgr, engine).await;
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
    let region_id = match space_mgr.ring_point_set_to_region.get(&point_set_id).copied() {
        Some(id) => id,
        None => return,
    };

    if let Some(t) = space_mgr.ring_transporters.get_mut(region_id) {
        t.region_triggered(entering, entity_id);
    }

    let auto_start = space_mgr.ring_transporters.get(region_id)
        .map_or(false, |t| t.should_auto_start());
    if auto_start {
        let dst_id = space_mgr.ring_transporters.get(region_id)
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

// ── Wire helpers ─────────────────────────────────────────────────────────────

/// Build an `onSequence` payload (matches the layout in
/// `cell/content/executor.rs::PlaySequence` and `cell_methods/player/world.rs`).
fn build_on_sequence_args(seq_id: i32, entity_id: u32) -> Vec<u8> {
    let mut args = Vec::with_capacity(26);
    args.extend_from_slice(&seq_id.to_le_bytes());                  // KismetEventSetSeqID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes());      // SourceID
    args.extend_from_slice(&(entity_id as i32).to_le_bytes());      // TargetID
    args.push(1);                                                   // PrimaryTarget = true
    args.extend_from_slice(&0.0f32.to_le_bytes());                  // ImpactTime
    args.extend_from_slice(&0u32.to_le_bytes());                    // NameValuePairs count = 0
    args.push(KISMET_VIEW_EVENT_INVOKER);                           // ViewType
    args.extend_from_slice(&0i32.to_le_bytes());                    // InstanceId
    args
}

async fn send_play_sequence(
    entity_id: u32,
    event_set_id: i32,
    region_event: RegionEvent,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let event_id = region_event.event_id();
    let seq_id = match space_mgr.sequence_map.get(&(event_set_id, event_id)) {
        Some(&id) => id,
        None => {
            tracing::warn!(
                event_set_id, event_id,
                "ring sequence not in event_sets_sequences map — kismet sequence will not play"
            );
            return;
        }
    };
    let args = build_on_sequence_args(seq_id, entity_id);
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: METHOD_ON_SEQUENCE,
        args,
    }).await;
}

async fn update_state_flag(
    entity_id: u32,
    flag: u32,
    set: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let new_state = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            if set { e.state_field |= flag; } else { e.state_field &= !flag; }
            e.state_field
        }
        None => return,
    };
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: METHOD_ON_STATE_FIELD_UPDATE,
        args: new_state.to_le_bytes().to_vec(),
    }).await;
}

async fn send_visible(
    entity_id: u32,
    visible: bool,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let byte: u8 = if visible { 1 } else { 0 };
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: METHOD_ON_VISIBLE,
        args: vec![byte],
    }).await;
}

async fn same_world_teleport(
    entity_id: u32,
    position: [f32; 3],
    _world_name: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Server-side: keep the spatial grid + entity position consistent so AoI
    // ticks broadcast the new position to other witnesses (build_avatar_update).
    space_mgr.update_entity_position(entity_id, position, [0, 0, 0], [0.0; 3]);
    // SpaceId.0 is i32 (matches DB type); the wire packet is u32, and space
    // ids are always non-negative so this is a width-only conversion.
    let space_id = space_mgr.get_entity(entity_id)
        .map(|e| e.space_id.0 as u32)
        .unwrap_or(0);
    if let Some(e) = space_mgr.get_entity_mut(entity_id) {
        e.position = Vector3::new(position[0], position[1], position[2]);
    }

    // Client-side: hand off to the base. The base sends `forcedPosition` (0x31)
    // to authoritatively snap the player's own avatar, then `onPlayerTeleport`
    // (method 116) for streaming-load coordination. The bare 116-only path the
    // previous version used does NOT move the avatar — see SGWPlayer.def comment
    // and the `handle_teleport_player` handler.
    let _ = tx.send(CellToBaseMsg::TeleportPlayer { entity_id, space_id, position }).await;
}

async fn send_destination_list(
    entity_id: u32,
    source_region_id: i32,
    destinations: &[i32],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let source = match space_mgr.ring_regions.get(&source_region_id) {
        Some(r) => r,
        None => {
            tracing::warn!(source_region_id, "send_destination_list: source region not in cache");
            return;
        }
    };
    let dests: Vec<&RingRegion> = destinations.iter()
        .filter_map(|id| {
            let r = space_mgr.ring_regions.get(id);
            if r.is_none() {
                tracing::warn!(invalid_id = id, "ring destination id not in cache — skipping");
            }
            r
        })
        .collect();

    let payload = build_on_ring_transporter_list(source, &dests);
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: METHOD_ON_RING_TRANSPORTER_LIST,
        args: payload,
    }).await;
    tracing::info!(
        entity_id, source_region_id, destination_count = dests.len(),
        "Sent onRingTransporterList"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::ring_transport::RingRegion;
    use crate::cell::space_manager::SpaceManager;

    fn make_test_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    fn ring(id: i32, world: &str, dests: Vec<i32>, pos: [f32; 3]) -> RingRegion {
        RingRegion {
            region_id: id, world_id: 12, world_name: world.to_string(),
            x: pos[0], y: pos[1], z: pos[2],
            tag: format!("Ring{id}"),
            height: 1.7, radius: 3.5,
            event_set_id: 100, display_name_id: 7508,
            destination_ids: dests, point_set_id: 2000 + id,
        }
    }

    /// End-to-end same-world ring travel: interact → select destination →
    /// player walks onto pad → tick through hide / warmup / cooldown →
    /// teleport_in event eventually dispatched.
    #[tokio::test]
    async fn full_ring_cycle_dispatches_expected_messages() {
        let mut mgr = make_test_space_mgr();

        // Two rings in Castle_CellBlock; the player will travel from 1 → 2.
        let mut regions = std::collections::HashMap::new();
        regions.insert(1, ring(1, "Castle_CellBlock", vec![2], [0.0, 0.0, 0.0]));
        regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0, 20.0, 30.0]));
        mgr.ring_transporters.load(&regions);
        mgr.ring_point_set_to_region = regions.iter()
            .map(|(rid, r)| (r.point_set_id, *rid))
            .collect();
        mgr.ring_regions = regions;

        // Seed the kismet sequence map so PlaySequence resolves.
        mgr.sequence_map.insert((100, 8000), 9000);
        mgr.sequence_map.insert((100, 8001), 9001);

        // Spawn the player entity in the source space.
        mgr.create_entity(42, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3]).unwrap();
        mgr.connect_entity(42);
        if let Some(p) = mgr.get_entity_mut(42) {
            p.player_id = Some(700);
        }

        let (tx, mut rx) = mpsc::channel(64);
        let engine = ChainEngine::new();

        // 1. Player triggers the ring switch (chain action = TriggerTransporter).
        handle_interact(1, 42, &tx, &mut mgr, &engine).await;
        assert_eq!(mgr.get_entity(42).and_then(|e| e.ring_source_id), Some(1));

        // Drain: should have one EntityMethodCall for onRingTransporterList.
        let mut seen_dest_list = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == METHOD_ON_RING_TRANSPORTER_LIST {
                    seen_dest_list = true;
                }
            }
        }
        assert!(seen_dest_list, "onRingTransporterList not sent");

        // 2. Player picks destination 2.
        handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
        assert_eq!(mgr.get_entity(42).and_then(|e| e.ring_source_id), None);
        assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::SendWait);
        assert_eq!(mgr.ring_transporters.get(2).unwrap().state, State::RecvWait);

        // 3. Player walks onto the source pad's region (point_set 2001).
        handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;
        // Auto-start: source SendWait → SendWarmup, dest RecvWait → RecvWarmup.
        assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::SendWarmup);
        assert_eq!(mgr.ring_transporters.get(2).unwrap().state, State::RecvWarmup);

        // 4. Drain effects so far (should include movement lock and play_sequence).
        let mut got_lock = false;
        let mut got_sequence = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
                if method_index == METHOD_ON_STATE_FIELD_UPDATE { got_lock = true; }
                if method_index == METHOD_ON_SEQUENCE { got_sequence = true; }
            }
        }
        assert!(got_lock, "BSF_MovementLock not applied");
        assert!(got_sequence, "Region_Teleport_Out sequence not played");

        // 5. Manually fire hide-timer (purely to advance state — no externally
        //    visible side effect on the destination). Then run the warmup
        //    transition through the proper dispatch path so TeleportPlayer's
        //    effects run, including the `mark_player_loaded` cross-link which
        //    moves the destination through RemoteLoadWait → RemoteWarmup.
        let hide_effects = mgr.ring_transporters.get_mut(1).unwrap().hide_timer_expired();
        dispatch_effects(hide_effects, &tx, &mut mgr, &engine).await;

        let dst_pos = {
            let dst = mgr.ring_regions.get(&2).unwrap();
            ([dst.x, dst.y, dst.z], dst.world_name.clone())
        };
        // Capture num_players BEFORE warmup (which clears send_players as part
        // of the source's reset to Idle).
        let warmup_num_players = mgr.ring_transporters.get(1).unwrap().send_players.len() as u32;
        let warmup_effects = mgr.ring_transporters.get_mut(1).unwrap()
            .warmup_timer_expired(dst_pos.0, &dst_pos.1);
        // Same ordering as the production tick: count update before teleport
        // so `mark_player_loaded` can advance the FSM synchronously.
        advance_destination_after_warmup(2, warmup_num_players, &tx, &mut mgr, &engine).await;
        dispatch_effects(warmup_effects, &tx, &mut mgr, &engine).await;

        // After the warmup teleport step we should see a TeleportPlayer
        // message (replacing the bare onPlayerTeleport that used to be
        // emitted directly — that path didn't move the avatar).
        let mut teleport_msg = None;
        let mut remaining = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::TeleportPlayer { entity_id, space_id, position } => {
                    teleport_msg = Some((entity_id, space_id, position));
                }
                other => remaining.push(other),
            }
        }
        let (eid, space_id, pos) = teleport_msg.expect("TeleportPlayer not emitted by warmup→teleport step");
        assert_eq!(eid, 42);
        assert_ne!(space_id, 0, "TeleportPlayer must carry a non-zero space_id");
        assert!((pos[0] - 10.0).abs() < 0.001);
        assert!((pos[1] - 20.0).abs() < 0.001);
        assert!((pos[2] - 30.0).abs() < 0.001);
        // Push the unrelated drained messages back into a sink so the rest of
        // the test runs against an empty channel — we don't need them, the
        // checks below look at FSM state and `mgr.get_entity` directly.
        drop(remaining);

        // After warmup + advance, the destination has either auto-advanced
        // through RemoteLoadWait (because the same-world TeleportPlayer marked
        // the player loaded in dispatch) and is now in Cooldown, or it's still
        // in RemoteWarmup waiting for the deadline. Both are valid intermediate
        // states; let's force the cooldown to fire.
        let dst_state = mgr.ring_transporters.get(2).unwrap().state;
        if dst_state == State::RemoteWarmup {
            let effs = mgr.ring_transporters.get_mut(2).unwrap()
                .remote_warmup_timer_expired(std::time::Instant::now());
            dispatch_effects(effs, &tx, &mut mgr, &engine).await;
        }
        let cd_effects = mgr.ring_transporters.get_mut(2).unwrap().cooldown_timer_expired();
        dispatch_effects(cd_effects, &tx, &mut mgr, &engine).await;

        // 6. Verify final state.
        assert_eq!(mgr.ring_transporters.get(2).unwrap().state, State::Idle);
        let player_pos = mgr.get_entity(42).unwrap().position;
        // `update_entity_position` was called inside same_world_teleport with
        // the destination's coords; verify the player landed there.
        assert!((player_pos.x - 10.0).abs() < 0.001, "player x: {} (expected 10.0)", player_pos.x);
        assert!((player_pos.y - 20.0).abs() < 0.001, "player y: {}", player_pos.y);
        assert!((player_pos.z - 30.0).abs() < 0.001, "player z: {}", player_pos.z);

        // Movement lock should have been cleared on cooldown.
        let final_state_field = mgr.get_entity(42).unwrap().state_field;
        assert_eq!(final_state_field & BSF_MOVEMENT_LOCK, 0,
            "BSF_MovementLock not cleared at cooldown");
    }

    /// Self-as-destination is rejected (matches Python `selectDestination`).
    #[tokio::test]
    async fn select_destination_self_rejected() {
        let mut mgr = make_test_space_mgr();
        let mut regions = std::collections::HashMap::new();
        regions.insert(1, ring(1, "Castle_CellBlock", vec![1, 2], [0.0; 3]));
        regions.insert(2, ring(2, "Castle_CellBlock", vec![1], [10.0; 3]));
        mgr.ring_transporters.load(&regions);
        mgr.ring_point_set_to_region = regions.iter()
            .map(|(rid, r)| (r.point_set_id, *rid))
            .collect();
        mgr.ring_regions = regions;

        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        let engine = ChainEngine::new();

        handle_select_destination(1, 1, 1, &tx, &mut mgr, &engine).await;

        // No teleport / sequence should fire — source ring stays idle.
        assert_eq!(mgr.ring_transporters.get(1).unwrap().state, State::Idle);
        assert!(rx.try_recv().is_err());
    }
}
