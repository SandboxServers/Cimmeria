//! `REQUEST_ACTIVE_SLOT_CHANGE` ability-side tests: auto-cycle stash
//! clearing + `BSF_AUTO_CYCLING` broadcast on swap, weapon-granted
//! ability revoke/grant on equip/unequip, and the `onKnownAbilitiesUpdate`
//! broadcast (including the same-slot no-op short-circuit). The wire
//! translation + holster choreography live in `active_slot_change.rs`.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::super::dispatch::dispatch;
use super::super::REQUEST_ACTIVE_SLOT_CHANGE;
use super::make_test_space_mgr;

/// Weapon swap clears auto-cycle stash + last-fired stash, and
/// broadcasts the `BSF_AUTO_CYCLING` clear to the client.
///
/// Bug shape: `setAutoCycle`'s immediate-fire path uses
/// `last_fired_ability_id` to pick what to fire, and the auto-cycle
/// tick uses `auto_cycle_ability_id`. Both are stashed at fire time
/// against the THEN-active weapon's `items_event_sets`. Swapping
/// weapons without clearing makes the next auto-cycle press fire
/// the wrong ability against the new weapon — either silently rejects
/// in `use_ability` (the new weapon doesn't grant the stashed id) or
/// misfires.
#[tokio::test]
async fn slot_change_clears_auto_cycle_stash_and_broadcasts() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    use crate::cell::content::build_engine;
    use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        // Skip the swap choreography (covered elsewhere) — this test
        // is about the auto-cycle state mutation on the immediate swap.
        e.pending_slot_swap_at = Some(std::time::Instant::now());

        // Seed slot 0 (current active, SMG-shaped) and slot 1 (target).
        for slot_id in 0..2 {
            e.bandolier_items.insert(
                slot_id,
                BandolierItem {
                    instance_id: 0,
                    item_id: 100 + slot_id,
                    clip_size: 30,
                    default_ammo_type: 1,
                    current_ammo: 30,
                    cur_ammo_type: 1,
                },
            );
        }
        e.active_bandolier_slot = 0;

        // Pre-arm auto-cycle on slot 0's weapon — the exact state
        // lomiada1's session reached before the bad swap.
        e.abilities.auto_cycle = true;
        e.abilities.auto_cycle_ability_id = Some(559); // SMG Auto Attack
        e.abilities.last_fired_ability_id = Some(559);
        e.state_field |= BSF_AUTO_CYCLING;
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    // Wire-slot 2 = server slot 1.
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes()); // bandolier bag
    args.extend_from_slice(&2i32.to_le_bytes());

    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(e.active_bandolier_slot, 1, "fixture: swap must land");
    assert!(
        !e.abilities.auto_cycle,
        "weapon swap must clear auto_cycle flag",
    );
    assert!(
        e.abilities.auto_cycle_ability_id.is_none(),
        "weapon swap must clear auto_cycle_ability_id — otherwise next \
         setAutoCycle press fires the OLD weapon's stale ability",
    );
    assert!(
        e.abilities.last_fired_ability_id.is_none(),
        "weapon swap must clear last_fired_ability_id — otherwise \
         setAutoCycle's immediate-fire path picks the OLD weapon's \
         ability and use_ability rejects with 'not granted by active \
         weapon' (live observation: lomiada1 ability 559 after swap)",
    );
    assert_eq!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "weapon swap must clear BSF_AUTO_CYCLING — the cycling icon on \
         the UI must reflect that the loop stopped",
    );

    // Wire-side: onStateFieldUpdate must fire so the client UI's
    // cycling icon clears. Without the broadcast, the UI shows the
    // loop as still armed even though the server cleared it.
    let mut saw_state_field_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == ON_STATE_FIELD_UPDATE {
                let state = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    state & BSF_AUTO_CYCLING,
                    0,
                    "broadcast must carry the cleared state",
                );
                saw_state_field_update = true;
            }
        }
    }
    assert!(
        saw_state_field_update,
        "weapon swap that ended an auto-cycle must broadcast \
         onStateFieldUpdate so the client UI clears the cycling icon",
    );
}

/// Companion: same-slot "swap" (replayed packet / no-op) must NOT
/// clear auto-cycle. The player hasn't expressed intent to change
/// weapons, so the loop should continue.
#[tokio::test]
async fn same_slot_change_preserves_auto_cycle_stash() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.pending_slot_swap_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: 100,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
        e.abilities.auto_cycle = true;
        e.abilities.auto_cycle_ability_id = Some(559);
        e.abilities.last_fired_ability_id = Some(559);
        e.state_field |= BSF_AUTO_CYCLING;
    }
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(16);
    let engine = build_engine(None).await;

    // Wire-slot 1 = server slot 0 = current slot. No-op swap.
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&1i32.to_le_bytes());

    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.abilities.auto_cycle,
        "same-slot must NOT clear auto_cycle"
    );
    assert_eq!(
        e.abilities.auto_cycle_ability_id,
        Some(559),
        "same-slot must preserve auto_cycle_ability_id",
    );
    assert_eq!(
        e.abilities.last_fired_ability_id,
        Some(559),
        "same-slot must preserve last_fired_ability_id",
    );
    assert_ne!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "same-slot must NOT clear BSF_AUTO_CYCLING",
    );
}
#[tokio::test]
async fn slot_change_to_p90_revokes_pistol_abilities_grants_smg_and_broadcasts() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    const PISTOL_ITEM_ID: i32 = 55;
    const P90_ITEM_ID: i32 = 21;
    const STRIKE: i32 = 594; // archetype starter — must survive
    const PISTOL_RANGED: i32 = 579;
    const PISTOL_MELEE: i32 = 708;
    const P90_RANGED: i32 = 559;
    const P90_MELEE: i32 = 595;
    const EVENT_RANGED: i32 = 7;
    const EVENT_MELEE: i32 = 6;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    // Seed items_event_sets — the lookup the handler will perform.
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_RANGED), PISTOL_RANGED);
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_MELEE), PISTOL_MELEE);
    mgr.item_event_set_abilities
        .insert((P90_ITEM_ID, EVENT_RANGED), P90_RANGED);
    mgr.item_event_set_abilities
        .insert((P90_ITEM_ID, EVENT_MELEE), P90_MELEE);

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: PISTOL_ITEM_ID,
                clip_size: 12,
                default_ammo_type: 1,
                current_ammo: 12,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_items.insert(
            1,
            BandolierItem {
                instance_id: 0,
                item_id: P90_ITEM_ID,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Pre-seed the known set as if the player already equipped
        // the pistol earlier (archetype Strike + the pistol's two
        // bindings). The bandolier handler must revoke the two
        // pistol bindings and keep Strike.
        e.abilities.add_ability(STRIKE);
        // Tag the pistol bindings as weapon-granted by running the
        // helper once — gives us a clean pre-state without poking
        // private fields.
        let pistol_set: std::collections::HashSet<i32> =
            [PISTOL_RANGED, PISTOL_MELEE].into_iter().collect();
        e.abilities.swap_weapon_granted_abilities(pistol_set);
        // Skip the choreography branch — we're testing the swap
        // outcome, not the animation gating.
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    let engine = build_engine(None).await;

    // Wire packet: bag_id = 3, wire_slot = 2 (server slot 1 — the P90).
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes());

    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    // Known-set post-conditions.
    let entity = mgr.get_entity(1).unwrap();
    assert!(
        entity.abilities.has_ability(STRIKE),
        "archetype Strike must SURVIVE the swap — only weapon-granted bindings get revoked"
    );
    assert!(
        !entity.abilities.has_ability(PISTOL_RANGED),
        "pistol's RANGED binding {PISTOL_RANGED} must be revoked when its weapon is unequipped"
    );
    assert!(
        !entity.abilities.has_ability(PISTOL_MELEE),
        "pistol's MELEE binding {PISTOL_MELEE} must be revoked too"
    );
    assert!(
        entity.abilities.has_ability(P90_RANGED),
        "P90's RANGED binding {P90_RANGED} must be granted on equip"
    );
    assert!(
        entity.abilities.has_ability(P90_MELEE),
        "P90's MELEE binding {P90_MELEE} must be granted too"
    );

    // Broadcast post-condition: at least one onKnownAbilitiesUpdate
    // packet must have gone out carrying the post-swap known set.
    let mut saw_known_abilities_update: Option<Vec<i32>> = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } = msg
        {
            if method_index == crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE {
                // Decode the wire payload: u32 count + N × i32 ability_id.
                let count = u32::from_le_bytes([args[0], args[1], args[2], args[3]]) as usize;
                let mut ids = Vec::with_capacity(count);
                for i in 0..count {
                    let off = 4 + i * 4;
                    ids.push(i32::from_le_bytes([
                        args[off],
                        args[off + 1],
                        args[off + 2],
                        args[off + 3],
                    ]));
                }
                saw_known_abilities_update = Some(ids);
            }
        }
    }
    let broadcast = saw_known_abilities_update.expect(
        "weapon swap must broadcast onKnownAbilitiesUpdate so the client \
         hotbar/abilities-window re-renders",
    );
    let mut sorted = broadcast.clone();
    sorted.sort();
    assert!(
        sorted.contains(&STRIKE) && sorted.contains(&P90_RANGED) && sorted.contains(&P90_MELEE),
        "broadcast must carry post-swap known set (Strike + P90 bindings); got {sorted:?}"
    );
    assert!(
        !sorted.contains(&PISTOL_RANGED) && !sorted.contains(&PISTOL_MELEE),
        "broadcast must NOT carry the revoked pistol bindings; got {sorted:?}"
    );
}

/// Companion guard: when the player swaps INTO an empty bandolier
/// slot (no weapon bound), every previously weapon-granted ability
/// must be revoked and the broadcast still fires so the client
/// sees the empty-handed known set.
///
/// Reverting the unequip path (e.g., guarding the broadcast on
/// `item_id.is_some()`) trips this guard.
#[tokio::test]
async fn slot_change_to_empty_slot_revokes_weapon_abilities_and_broadcasts() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    const PISTOL_ITEM_ID: i32 = 55;
    const STRIKE: i32 = 594;
    const PISTOL_RANGED: i32 = 579;
    const PISTOL_MELEE: i32 = 708;
    const EVENT_RANGED: i32 = 7;
    const EVENT_MELEE: i32 = 6;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_RANGED), PISTOL_RANGED);
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_MELEE), PISTOL_MELEE);

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: PISTOL_ITEM_ID,
                clip_size: 12,
                default_ammo_type: 1,
                current_ammo: 12,
                cur_ammo_type: 1,
            },
        );
        // Slot 1 left empty deliberately — the unequip target.
        e.active_bandolier_slot = 0;
        e.abilities.add_ability(STRIKE);
        e.abilities
            .swap_weapon_granted_abilities([PISTOL_RANGED, PISTOL_MELEE].into_iter().collect());
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    let engine = build_engine(None).await;

    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&2i32.to_le_bytes()); // wire slot 2 = server slot 1 (empty)

    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).unwrap();
    assert!(entity.abilities.has_ability(STRIKE));
    assert!(!entity.abilities.has_ability(PISTOL_RANGED));
    assert!(!entity.abilities.has_ability(PISTOL_MELEE));

    let mut saw_broadcast = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            ..
        } = msg
        {
            if method_index == crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE {
                saw_broadcast = true;
            }
        }
    }
    assert!(
        saw_broadcast,
        "unequip into an empty slot must still broadcast onKnownAbilitiesUpdate \
         so the client sees the empty-handed known set"
    );
}

/// **Same-slot no-op (PR #494 review G10).** When the player
/// "swaps" to the slot they're already on (e.g., replayed client
/// packet, CEGUI fires the keybinding 3-4× per physical press),
/// the weapon-equip ability swap MUST NOT broadcast
/// `onKnownAbilitiesUpdate`. The diff is empty (same weapon,
/// same bindings); a forced broadcast would flood the wire on
/// every duplicate keypress.
///
/// Reverting the empty-diff short-circuit in
/// `swap_weapon_granted_abilities_for_slot` trips this guard by
/// firing `onKnownAbilitiesUpdate` for the no-op case.
#[tokio::test]
async fn same_slot_swap_does_not_broadcast_known_abilities_update() {
    use crate::cell::content::build_engine;
    use cimmeria_entity::cell_entity::BandolierItem;

    const PISTOL_ITEM_ID: i32 = 55;
    const PISTOL_RANGED: i32 = 579;
    const PISTOL_MELEE: i32 = 708;
    const EVENT_RANGED: i32 = 7;
    const EVENT_MELEE: i32 = 6;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_RANGED), PISTOL_RANGED);
    mgr.item_event_set_abilities
        .insert((PISTOL_ITEM_ID, EVENT_MELEE), PISTOL_MELEE);

    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.archetype_id = Some(1);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 0,
                item_id: PISTOL_ITEM_ID,
                clip_size: 12,
                default_ammo_type: 1,
                current_ammo: 12,
                cur_ammo_type: 1,
            },
        );
        e.active_bandolier_slot = 0;
        // Pre-seed pistol's bindings as already weapon-granted —
        // the prior equip already populated the tracked set.
        e.abilities
            .swap_weapon_granted_abilities([PISTOL_RANGED, PISTOL_MELEE].into_iter().collect());
        e.pending_slot_swap_at = Some(std::time::Instant::now());
    }
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    let engine = build_engine(None).await;

    // Wire slot 1 = server slot 0 — the slot we're already on.
    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&3i32.to_le_bytes());
    args.extend_from_slice(&1i32.to_le_bytes());

    dispatch(1, REQUEST_ACTIVE_SLOT_CHANGE, &args, &tx, &mut mgr, &engine).await;

    // Known set must be unchanged.
    let entity = mgr.get_entity(1).unwrap();
    assert!(entity.abilities.has_ability(PISTOL_RANGED));
    assert!(entity.abilities.has_ability(PISTOL_MELEE));

    // No onKnownAbilitiesUpdate broadcast should have fired. Other
    // wire packets from the slot-change handler (active-slot
    // indicator, etc.) are out of scope here — we're pinning the
    // ability-swap broadcast specifically.
    let mut saw_known_abilities_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            ..
        } = msg
        {
            if method_index == crate::cell::client_methods::player::ON_KNOWN_ABILITIES_UPDATE {
                saw_known_abilities_update = true;
            }
        }
    }
    assert!(
        !saw_known_abilities_update,
        "same-slot 'swap' (weapon binding unchanged) must NOT broadcast \
         onKnownAbilitiesUpdate — the diff is empty and the bulk update \
         would flood the wire on every duplicate keypress"
    );
}
