use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::make_test_space_mgr;

/// REQUEST_ACTIVE_SLOT_CHANGE must reject any wire slot that maps outside
/// the bandolier's server-side range (`0..bag_max_slots(3)` = `0..4`)
/// before mutating active_bandolier_slot or sending ActiveSlotUpdate. A
/// forged value would otherwise leave the entity in an impossible state.
///
/// Wire ↔ server translation is `server = wire - 1`, so the rejected wire
/// values are: 0 (→ server -1), 5+ (→ server 4+), and any negative wire
/// value (which the legacy client never sends but a forged packet might).
#[tokio::test]
async fn request_active_slot_change_rejects_out_of_range_slot() {
    use crate::cell::content::build_engine;

    // Wire values that translate to invalid server slots:
    //   0  → server -1 (below range)
    //   5  → server  4 (above the 4-slot bandolier)
    //   99 → server 98
    //   -1 → server -2 (already below the floor)
    //   i32::MAX → server i32::MAX-1 (above range)
    //   i32::MIN → server i32::MIN (saturating_sub guards against debug-overflow panic)
    for bad_wire_slot in [0i32, 5, 99, -1, i32::MAX, i32::MIN] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.active_bandolier_slot = 0;
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes()); // bag_id = 3
        args.extend_from_slice(&bad_wire_slot.to_le_bytes());

        let handled = dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;
        assert!(
            handled,
            "handler claims the index even when slot is invalid (bad_wire_slot={bad_wire_slot})"
        );
        assert!(
            rx.try_recv().is_err(),
            "no messages emitted for bad_wire_slot={bad_wire_slot}"
        );
        assert_eq!(
            mgr.get_entity(1).unwrap().active_bandolier_slot,
            0,
            "active_bandolier_slot must not change for bad_wire_slot={bad_wire_slot}"
        );
    }
}

/// Wire slot IDs are 1-indexed by client convention (`Bag.py:369`,
/// `SGWPlayer.py:2192`). The wire decoder must subtract 1 before any
/// mutation so the cell's `active_bandolier_slot` and the
/// `ActiveSlotUpdate` message both carry the 0-indexed server slot.
///
/// Pins the translation: wire 1..=4 ↔ server 0..=3 across the entire
/// 4-slot bandolier range. A regression here was the original cause of
/// "switching between slots does not work at all" — the server treated
/// wire 1 as server slot 1 (the second weapon, not the first), so
/// keypress 1 either no-op'd or jumped to the wrong weapon.
#[tokio::test]
async fn request_active_slot_change_translates_wire_to_server_slot() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    for (wire_slot, expected_server_slot) in [(1i32, 0), (2, 1), (3, 2), (4, 3)] {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            // Pre-stamp `pending_slot_swap_at` so the handler treats
            // this as the re-entry-from-tick path, skipping the
            // choreography branch and running the immediate swap.
            // This test verifies wire-to-server slot translation,
            // not the choreography timing.
            e.pending_slot_swap_at = Some(std::time::Instant::now());
            // Seed all 4 slots so the swap doesn't bail on "empty target".
            for slot_id in 0..4 {
                e.bandolier_items.insert(
                    slot_id,
                    BandolierItem {
                        item_id: 100 + slot_id,
                        clip_size: 30,
                        default_ammo_type: 1,
                        current_ammo: 30,
                        cur_ammo_type: 1,
                    },
                );
            }
            // Start at a slot that's distinct from every test case so each
            // iteration genuinely exercises the swap path.
            e.active_bandolier_slot = if expected_server_slot == 0 { 3 } else { 0 };
        }
        mgr.connect_entity(1);

        let (tx, mut rx) = mpsc::channel(8);
        let engine = build_engine(None).await;

        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&3i32.to_le_bytes());
        args.extend_from_slice(&wire_slot.to_le_bytes());

        dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().active_bandolier_slot,
            expected_server_slot,
            "wire slot {wire_slot} must land in cell as server slot {expected_server_slot}"
        );

        // Confirm two pieces of state both reflect the new server slot:
        //   1. `ActiveSlotUpdate` to base: the persisted `bandolier_slot`
        //      column must store the server-side index, not the wire-side
        //      one (would cause the appearance query to filter the wrong
        //      bandolier row and render the player without a weapon).
        //   2. `onActiveSlotUpdate` (method 70) to the client: the
        //      bandolier UI indicator must learn the new slot, otherwise
        //      the LUA `getActiveSlotForContainer(...) ~= N` guard turns
        //      subsequent keypresses for the slot it thinks is selected
        //      into client-side no-ops. That's the bug behind "switching
        //      back to the first bandolier slot does not give me my
        //      weapon back" during play-testing.
        let mut saw_active_slot_update = false;
        let mut saw_client_indicator = false;
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::ActiveSlotUpdate { slot_id, .. } => {
                    assert_eq!(
                        slot_id, expected_server_slot,
                        "ActiveSlotUpdate must carry the server slot, not the wire slot"
                    );
                    saw_active_slot_update = true;
                }
                CellToBaseMsg::EntityMethodCall {
                    method_index, args, ..
                } if method_index
                    == crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE =>
                {
                    let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    let wire_slot_field = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    assert_eq!(bag_id, 3, "bandolier bag id on the wire indicator");
                    assert_eq!(
                        wire_slot_field,
                        expected_server_slot + 1,
                        "client indicator must carry wire slot (= server slot + 1)"
                    );
                    saw_client_indicator = true;
                }
                _ => {}
            }
        }
        assert!(
            saw_active_slot_update,
            "expected an ActiveSlotUpdate message for wire_slot={wire_slot}"
        );
        assert!(
            saw_client_indicator,
            "expected an onActiveSlotUpdate (client UI indicator) for wire_slot={wire_slot}"
        );
    }
}

/// task #20 — weapon→weapon slot swap (both source and target
/// hold weapons, player is OOC) must defer the slot change behind an
/// `Item_Unequip` animation. The handler fires the holster sequence
/// immediately and stashes `(pending_slot_swap_at, pending_slot_swap_target)`
/// for `pending_slot_swap_tick` to finalize once
/// `HOLSTER_ANIMATION_DURATION` has elapsed.
///
/// Bug shape this catches: a refactor that drops the choreography
/// branch regresses to "F1→F2 instantly swaps with no animation" —
/// the symptom this task fixes.
#[tokio::test]
async fn weapon_to_weapon_swap_ooc_defers_via_item_unequip() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        // Both slots populated → weapon→weapon swap.
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 11,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 12,
                cur_ammo_type: 7,
            },
        );
        e.active_bandolier_slot = 0;
        // OOC (no threatened mobs) so the choreography fires.
    }
    mgr.connect_entity(1);
    // Seed the Item_Unequip sequence (archetype 1 → event_set 804 → seq 1873).
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    // Swap slot 0 → slot 1 (wire slot 2).
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.active_bandolier_slot, 0,
        "choreography must NOT change the active slot yet — \
         pending_slot_swap_tick finalizes after HOLSTER_ANIMATION_DURATION",
    );
    assert!(
        e.pending_slot_swap_at.is_some(),
        "choreography must stash pending_slot_swap_at so the tick \
         knows to fire Phase 2",
    );
    assert_eq!(
        e.pending_slot_swap_target,
        Some(1),
        "target slot must be stashed for the deferred swap",
    );

    let mut saw_unequip_sequence = false;
    let mut saw_active_slot_update = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall { method_index, .. }
                if method_index == crate::cell::client_methods::spawnable_entity::ON_SEQUENCE =>
            {
                saw_unequip_sequence = true;
            }
            CellToBaseMsg::ActiveSlotUpdate { .. } => {
                saw_active_slot_update = true;
            }
            _ => {}
        }
    }
    assert!(
        saw_unequip_sequence,
        "choreography must fire Item_Unequip immediately so the client \
         plays the holster animation for the old weapon",
    );
    assert!(
        !saw_active_slot_update,
        "ActiveSlotUpdate must NOT be sent yet — base would persist \
         the new active slot, which triggers refresh_player_appearance \
         broadcasting the new weapon mesh mid-holster-animation",
    );
}

/// Per-weapon holster duration: the slot-swap choreography's
/// `pending_slot_swap_at` stamp must use the OUTGOING weapon's
/// class-specific duration. Without this, longarms (P90, AR, LMG —
/// ~1s holster) get cut off mid-animation when the new weapon's
/// draw starts on top — the user's playtest glitch on P90→pistol.
///
/// Bug shape: refactor stamps `pending_slot_swap_at` with the
/// generic `HOLSTER_ANIMATION_DURATION` (600ms) regardless of the
/// outgoing weapon class → longarm holster anims clip mid-flight,
/// pistol draw layers on top, mesh ends up in the wrong place.
#[tokio::test]
async fn slot_swap_uses_outgoing_weapon_holster_duration() {
    use crate::cell::content::build_engine;
    use crate::cell::spawner::WeaponDef;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    // Seed item_defs so the per-weapon lookup finds a longarm.
    let longarm_duration = std::time::Duration::from_millis(1000);
    mgr.item_defs.insert(
        50,
        WeaponDef {
            clip_size: 50,
            default_ammo_type: 1,
            allowed_ammo_types: vec![],
            holster_animation_duration: longarm_duration,
        },
    );
    mgr.item_defs.insert(
        51,
        WeaponDef {
            clip_size: 8,
            default_ammo_type: 2,
            allowed_ammo_types: vec![],
            holster_animation_duration: std::time::Duration::from_millis(600),
        },
    );
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        // Slot 0 holds a longarm (item 50), slot 1 a sidearm (item 51).
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 50,
                clip_size: 50,
                default_ammo_type: 1,
                current_ammo: 50,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 51,
                clip_size: 8,
                default_ammo_type: 2,
                current_ammo: 8,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);

    let (tx, _rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    // Swap longarm (slot 0) → sidearm (slot 1, wire 2).
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());
    let before = std::time::Instant::now();
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;
    let after = std::time::Instant::now();

    let e = mgr.get_entity(1).unwrap();
    let stamp = e
        .pending_slot_swap_at
        .expect("choreography must stash pending_slot_swap_at");
    let stamp_offset = stamp - before;
    // Window: between [longarm_duration - 50ms] and [longarm_duration + elapsed-dispatch + 50ms].
    // We need a band because `Instant::now()` ticks between `before` and the
    // handler's stamp call.
    let dispatch_elapsed = after - before;
    assert!(
        stamp_offset >= longarm_duration - std::time::Duration::from_millis(50)
            && stamp_offset
                <= longarm_duration + dispatch_elapsed + std::time::Duration::from_millis(50),
        "swap from a longarm (1000ms holster) must stamp pending_slot_swap_at \
         at ~1000ms in the future, NOT the generic 600ms — got {}ms offset",
        stamp_offset.as_millis()
    );
}

/// Weapon→empty slot must NOT trigger the weapon→weapon choreography —
/// that path is only for swap-between-occupied-slots. Swapping to an
/// empty slot is a "no active weapon" scenario; the existing draw_intent
/// logic (which only fires Item_Equip when the new slot has a weapon)
/// handles it correctly. A regression that triggers choreography here
/// would fire Item_Unequip and then defer indefinitely (no new weapon
/// to equip in Phase 2 means the "active slot has weapon" condition
/// for the OOC re-holster timer never re-fires either).
#[tokio::test]
async fn weapon_to_empty_slot_skips_choreography() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        // Slot 1 intentionally empty.
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.active_bandolier_slot, 1,
        "swap to empty slot must complete immediately (no choreography \
         queue) — empty target means there's nothing to Item_Equip",
    );
    assert!(
        e.pending_slot_swap_at.is_none(),
        "empty target must NOT queue a choreography — the weapon→empty \
         case is handled by the normal active-slot flow",
    );
}

/// In-combat weapon→weapon swap MUST still trigger the choreography
/// (and the ability lockout that comes with it). Without the
/// in-combat penalty, players could free-swap weapons mid-fight to
/// teleport between e.g. a long-range rifle and a melee weapon with
/// no animation cost — defeating the bandolier as a real loadout
/// choice. The penalty is the animation duration plus the weapon-
/// attack rejection while `pending_slot_swap_at` is set.
///
/// Bug shape: a refactor that re-adds the
/// `threatened_mobs.is_empty()` gate to `needs_holster_first`
/// regresses to "in-combat swaps are instant" — the user's
/// playtest call-out from the original review.
#[tokio::test]
async fn weapon_to_weapon_swap_in_combat_still_triggers_choreography() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        // In-combat: aggro on a mob.
        e.threatened_mobs.insert(9999);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                item_id: 11,
                clip_size: 12,
                default_ammo_type: 7,
                current_ammo: 12,
                cur_ammo_type: 7,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);
    mgr.sequence_map
        .insert((804, crate::cell::spawner::EVENT_ITEM_UNEQUIP), 1873);

    let (tx, _rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());
    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.active_bandolier_slot, 0,
        "in-combat swap must defer the active-slot change behind the \
         choreography — Phase 2 (via pending_slot_swap_tick) flips \
         the slot once the holster animation has played out",
    );
    assert!(
        e.pending_slot_swap_at.is_some(),
        "in-combat swap must queue the choreography — the penalty is \
         what makes weapon swaps a real loadout choice rather than \
         a free teleport",
    );
    assert_eq!(
        e.pending_slot_swap_target,
        Some(1),
        "target slot must be stashed for the deferred swap",
    );
}

/// While a bandolier slot swap is in progress (pending_slot_swap_at
/// armed and not yet elapsed), weapon attacks must be silently
/// rejected. The player's hands are physically holstering and
/// drawing; firing through the window would defeat the animation
/// penalty.
///
/// Non-weapon abilities (heals, buffs) still pass — the lockout is
/// about the FIRE pose, not all abilities.
#[tokio::test]
async fn weapon_attack_blocked_while_slot_swap_in_progress() {
    use crate::cell::abilities::handle_use_ability;
    use cimmeria_entity::abilities::AbilityDef;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.weapon_holstered = false;
        // Slot swap in progress — armed 100ms in the future.
        e.pending_slot_swap_at =
            Some(std::time::Instant::now() + std::time::Duration::from_millis(500));
        e.pending_slot_swap_target = Some(1);
        e.abilities.add_ability(7);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.connect_entity(1);
    // Weapon attack: required_ammo=1.
    mgr.ability_defs.insert(
        7,
        AbilityDef {
            ability_id: 7,
            name: "test_fire".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(16);
    let committed = handle_use_ability(1, 7, 0, &tx, &mut mgr).await;
    assert!(
        !committed,
        "weapon attack must be rejected while a slot swap is in \
         progress — the animation penalty is what makes swaps a real \
         loadout choice",
    );
    assert_eq!(
        mgr.get_entity(1).unwrap().bandolier_items[&0].current_ammo,
        30,
        "rejection must NOT consume ammo",
    );
}
