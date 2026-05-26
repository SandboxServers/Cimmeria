use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

/// Deferred holster after combat ends.
///
/// `exit_player_combat` stamps `combat_exit_at = Some(now)` instead of
/// flipping `weapon_holstered` immediately so chaining mobs (kill A,
/// aggro B 50ms later) doesn't visibly flicker the model. This tick
/// fires the actual holster once
/// [`crate::cell::combat::OOC_HOLSTER_DELAY`] has elapsed.
///
/// Cancellation: `enter_player_combat` clears `combat_exit_at` whenever
/// it runs (re-aggro inside the grace window). Players the timer wakes
/// up to find back in combat are skipped — their `combat_exit_at` will
/// already be cleared.
///
/// Cadence: every 100ms AoI tick. The cost is a single `Instant::now()`
/// plus a filtered pass over `all_player_entity_ids()`; rebroadcasts
/// only fire on transitioning players, so this is essentially free in
/// steady state.
/// Phase 2 delay between firing the `Item_Unequip` animation and
/// broadcasting the mesh-removal `BeingAppearance`. The constant
/// is *intentionally shorter than the visible animation length* —
/// it sets when the cell sends `RefreshAppearance(holstered=true)`,
/// not when the client renders the mesh removal.
///
/// Timing target: the mesh-removal `BeingAppearance` should *arrive
/// on the client* at the instant the holster animation visually
/// completes. Without an early send the client renders the
/// animation's final frame (pistol at thigh), then the blend tree
/// returns to the idle-armed pose (weapon snaps back to the hand
/// socket) for a few frames before the mesh-removal packet lands —
/// a visible flicker.
///
/// 850ms was the first attempt; user playtest confirmed the flicker
/// remained, which means the underlying `Item_Unequip` animation is
/// shorter than ~850ms. 600ms is the next data point — the visible
/// pistol-to-thigh motion seems to be in the ~500-700ms range based
/// on the "few frames" of flicker description.
///
/// Trade-off at lower values: the mesh removes earlier in the
/// animation. At 600ms with a ~700ms animation, the pistol vanishes
/// during the last ~100ms of the holster motion — barely
/// perceptible because the pistol is already settled at the thigh
/// socket at that point. Strictly less visible than the flicker.
///
/// Empirically tuned against the `KIS-abilities_human.KIS-handling`
/// Unequip branch — adjust upward if the flicker returns, downward
/// if the pistol vanishes too early.
pub(crate) const HOLSTER_ANIMATION_DURATION: std::time::Duration =
    std::time::Duration::from_millis(600);

#[tracing::instrument(
    name = "holster.timer_tick",
    level = "debug",
    skip_all,
    fields(phase1_count = tracing::field::Empty, phase2_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn holster_timer_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    // ── Phase 1: OOC grace elapsed → fire holster animation ──────────
    //
    // Players whose `combat_exit_at` has aged past `OOC_HOLSTER_DELAY`
    // get the `Item_Unequip` animation fired and a Phase 2 stamp set
    // for `HOLSTER_ANIMATION_DURATION` later. The weapon mesh stays
    // attached during the animation; Phase 2 removes it.
    let phase1: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr.get_entity(eid).is_some_and(|e| {
                e.combat_exit_at.is_some_and(|t| {
                    now.duration_since(t) >= crate::cell::combat::OOC_HOLSTER_DELAY
                })
            })
        })
        .collect();
    tracing::Span::current().record("phase1_count", phase1.len());
    for entity_id in phase1 {
        // Per-weapon duration: longarms (P90, AR, LMG) take longer
        // to stow than sidearms (pistol). Look up the active
        // weapon's class to pick the right Phase 2 stamp.
        let active_item_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.bandolier_items.get(&e.active_bandolier_slot))
            .map(|item| item.item_id);
        let duration = active_item_id
            .and_then(|id| space_mgr.item_defs.get(&id))
            .map(|d| d.holster_animation_duration)
            .unwrap_or(HOLSTER_ANIMATION_DURATION);
        // Transition stamps: clear combat_exit_at (Phase 1 fired) and
        // schedule Phase 2. Do this BEFORE the animation dispatch so
        // any racing tick doesn't re-fire Phase 1.
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.combat_exit_at = None;
            e.holster_animation_complete_at = Some(now + duration);
        }
        tracing::info!(
            entity_id,
            active_item_id,
            duration_ms = duration.as_millis() as u64,
            "holster_timer_tick: phase 1 — playing Item_Unequip; appearance deferred"
        );
        // Fire `Item_Unequip` (event 4001) — the bandolier-take-off
        // animation. Same `KIS-abilities_human.KIS-handling` kismet
        // script as `Item_Equip` / `Item_Reload`, but its Unequip
        // branch has the hand-authored "put weapon away" motion.
        // Used in python's `onItemUnequipped` for bandolier removal;
        // we reuse it as the OOC re-holster animation. (UE3 Matinee
        // supports `bReversePlayback` for true reverse playback at
        // `ghidra://SGW.exe@0x01893ef0`, but that flag is set at
        // design time in the kismet editor and isn't reachable
        // through the `playSequence` runtime API.)
        super::super::super::cell_methods::player::world::fire_item_sequence(
            entity_id,
            super::super::super::spawner::EVENT_ITEM_UNEQUIP,
            tx,
            space_mgr,
        )
        .await;
    }

    // ── Phase 2: animation done → remove the mesh ────────────────────
    //
    // Players whose `holster_animation_complete_at` has elapsed get
    // `weapon_holstered = true` flipped + a `RefreshAppearance`
    // dispatched so the wire `ComponentList` finally drops the weapon
    // visual. The split exists so the animation has time to play with
    // the weapon mesh attached — without it, the mesh snaps away
    // while/before the animation runs and the visible result is
    // "weapon vanishes mid-motion."
    let phase2: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .is_some_and(|e| e.holster_animation_complete_at.is_some_and(|t| now >= t))
        })
        .collect();
    tracing::Span::current().record("phase2_count", phase2.len());
    for entity_id in phase2 {
        let should_rebroadcast = match space_mgr.get_entity_mut(entity_id) {
            Some(e) => {
                e.holster_animation_complete_at = None;
                e.sync_holster_to_combat(false)
            }
            None => false,
        };
        if !should_rebroadcast {
            continue;
        }
        tracing::info!(
            entity_id,
            "holster_timer_tick: phase 2 — animation done, removing weapon mesh"
        );
        super::super::super::abilities::request_appearance_refresh(entity_id, tx, space_mgr).await;
    }
}

/// Promote queued bandolier slot swap: finalize the slot change
/// (active slot update, `Item_Equip` for the new weapon) after the
/// `Item_Unequip` animation for the old weapon has played out.
///
/// `handle_request_active_slot_change` detects "OOC + both slots
/// hold weapons + different slots," fires `Item_Unequip` immediately,
/// stashes `(pending_slot_swap_at, pending_slot_swap_target)`, and
/// returns. This tick fires once `HOLSTER_ANIMATION_DURATION` has
/// elapsed — it re-invokes the handler with the target slot. The
/// pending state is kept set during the re-invocation so the
/// choreography branch short-circuits; the handler clears it on
/// the way through.
///
/// Cadence: every 100ms AoI tick. Cost is one filter pass; the
/// inner re-invocation only fires on transition.
#[tracing::instrument(
    name = "bandolier.pending_slot_swap_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn pending_slot_swap_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();
    let ready: Vec<(u32, i32)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            let at = e.pending_slot_swap_at?;
            if now < at {
                return None;
            }
            let target = e.pending_slot_swap_target?;
            Some((eid, target))
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    for (entity_id, target_slot) in ready {
        tracing::info!(
            entity_id,
            target_slot,
            "pending_slot_swap_tick: holster animation done, finalizing swap"
        );
        // Build wire args matching `requestActiveSlotChange`:
        // bag_id (i32) + wire_slot_id (i32, 1-indexed). The wire
        // decoder in `handle_request_active_slot_change` subtracts 1.
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes());
        args.extend_from_slice(&(target_slot + 1).to_le_bytes());
        super::super::super::cell_methods::inventory::handle_request_active_slot_change(
            entity_id, &args, tx, space_mgr,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;

    fn make_holster_test_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        let p = mgr.get_entity_mut(1).unwrap();
        p.is_player = true;
        p.player_id = Some(100);
        p.weapon_visual = Some("BS_Gun.Pistol".into());
        p.weapon_holstered = false;
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
        mgr
    }

    /// The holster timer is a NO-OP when the grace window hasn't
    /// elapsed yet — drawing weapons must not flicker holstered just
    /// because the next tick runs. Pin it with a stamp that's
    /// effectively "now."
    #[tokio::test]
    async fn holster_timer_tick_skips_players_inside_grace_window() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.combat_exit_at = Some(std::time::Instant::now());
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "no RefreshAppearance should fire inside the grace window",
        );
        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "weapon must stay drawn until the grace window elapses",
        );
        assert!(
            player.combat_exit_at.is_some(),
            "timer must remain stamped — only elapsed entries get consumed",
        );
    }

    /// Phase 1 (OOC grace elapsed): fires `Item_Unequip` animation
    /// and schedules Phase 2. The weapon mesh STAYS attached —
    /// `weapon_holstered` does NOT flip yet and no `RefreshAppearance`
    /// is dispatched. The hand-authored animation plays with the mesh
    /// visible; Phase 2 removes the mesh after the animation finishes.
    ///
    /// Bug shape this catches: a refactor collapses Phase 1 and Phase
    /// 2 back into a single tick, the mesh snaps away while the
    /// animation is still playing, and the visible result regresses
    /// to "weapon vanishes mid-motion."
    #[tokio::test]
    async fn holster_timer_tick_phase1_plays_animation_without_removing_mesh() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
        }
        mgr.sequence_map
            .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);
        let elapsed = crate::cell::combat::OOC_HOLSTER_DELAY + std::time::Duration::from_millis(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.combat_exit_at = std::time::Instant::now().checked_sub(elapsed);
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        let mut saw_unequip_sequence = false;
        let mut saw_refresh = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::RefreshAppearance { .. } => saw_refresh = true,
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE => {
                    let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    if seq_id == 1873 {
                        saw_unequip_sequence = true;
                    }
                }
                _ => {}
            }
        }
        assert!(
            saw_unequip_sequence,
            "Phase 1 must fire Item_Unequip sequence so the client plays the \
             hand-authored holster animation",
        );
        assert!(
            !saw_refresh,
            "Phase 1 must NOT dispatch RefreshAppearance — the weapon mesh \
             must stay attached during the animation. Removing it here is \
             the bug shape that drove the two-phase split.",
        );

        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "Phase 1 must NOT flip weapon_holstered yet — the mesh stays \
             attached until Phase 2 fires after HOLSTER_ANIMATION_DURATION",
        );
        assert!(
            player.combat_exit_at.is_none(),
            "Phase 1 stamp (combat_exit_at) must clear so it doesn't re-fire",
        );
        assert!(
            player.holster_animation_complete_at.is_some(),
            "Phase 2 must be scheduled via holster_animation_complete_at",
        );
    }

    /// Phase 2 (animation duration elapsed): flips `weapon_holstered`
    /// and dispatches `RefreshAppearance(holstered=true)` so the wire
    /// `ComponentList` drops the weapon visual and the client removes
    /// the mesh.
    #[tokio::test]
    async fn holster_timer_tick_phase2_removes_mesh_when_animation_done() {
        let mut mgr = make_holster_test_mgr();
        let elapsed = HOLSTER_ANIMATION_DURATION + std::time::Duration::from_millis(1);
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.combat_exit_at = None;
            p.holster_animation_complete_at = std::time::Instant::now().checked_sub(elapsed);
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        let mut saw_refresh = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::RefreshAppearance {
                holstered: true, ..
            } = msg
            {
                saw_refresh = true;
            }
        }
        assert!(
            saw_refresh,
            "Phase 2 must dispatch RefreshAppearance(holstered=true) so the \
             weapon mesh is removed from the wire ComponentList",
        );

        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.weapon_holstered,
            "Phase 2 must flip weapon_holstered=true"
        );
        assert!(
            player.holster_animation_complete_at.is_none(),
            "Phase 2 stamp must clear so subsequent ticks don't re-fire",
        );
    }

    /// Phase 2 stamp scheduled in the future is a no-op (animation
    /// still playing). Pins the boundary.
    #[tokio::test]
    async fn holster_timer_tick_phase2_skips_while_animation_in_flight() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.archetype_id = Some(1);
            p.combat_exit_at = None;
            p.holster_animation_complete_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
        }

        let (tx, mut rx) = mpsc::channel(8);
        holster_timer_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "no messages should fire while Phase 2 stamp is in the future",
        );
        let player = mgr.get_entity(1).unwrap();
        assert!(
            !player.weapon_holstered,
            "weapon must stay drawn until the animation finishes",
        );
    }

    #[tokio::test]
    async fn pending_slot_swap_tick_no_op_when_stamp_in_future() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.pending_slot_swap_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
            p.pending_slot_swap_target = Some(1);
        }

        let (tx, mut rx) = mpsc::channel(8);
        pending_slot_swap_tick(&tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "future stamp must not trigger slot swap"
        );
        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.pending_slot_swap_at.is_some(),
            "stamp must remain unconsumed"
        );
        assert_eq!(player.pending_slot_swap_target, Some(1));
    }

    #[tokio::test]
    async fn pending_slot_swap_tick_fires_when_elapsed() {
        let mut mgr = make_holster_test_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.pending_slot_swap_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            p.pending_slot_swap_target = Some(0);
        }

        let (tx, mut rx) = mpsc::channel(8);
        pending_slot_swap_tick(&tx, &mut mgr).await;

        // After the tick fires, `handle_request_active_slot_change`
        // clears the pending state (lines 209-214 in bandolier.rs).
        let player = mgr.get_entity(1).unwrap();
        assert!(
            player.pending_slot_swap_at.is_none(),
            "pending_slot_swap_at must be cleared after the swap fires"
        );
        assert!(
            player.pending_slot_swap_target.is_none(),
            "pending_slot_swap_target must be cleared after the swap fires"
        );

        // The downstream handler sends ActiveSlotUpdate to base.
        let mut got_active_slot_update = false;
        while let Ok(msg) = rx.try_recv() {
            if matches!(msg, CellToBaseMsg::ActiveSlotUpdate { .. }) {
                got_active_slot_update = true;
            }
        }
        assert!(
            got_active_slot_update,
            "pending_slot_swap_tick must dispatch ActiveSlotUpdate via \
             handle_request_active_slot_change"
        );
    }
}
