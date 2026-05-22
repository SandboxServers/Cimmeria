use super::*;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;

fn make_mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}

#[tokio::test]
async fn dispatch_returns_false_for_unknown_method() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
    assert!(!handled);
}

/// SET_AUTO_CYCLE flips entity.abilities.auto_cycle. When
/// disabled, must clear auto_cycle_ability_id too — otherwise a
/// stale ability id would re-trigger on the next enable cycle.
#[tokio::test]
async fn set_auto_cycle_disable_clears_ability_id() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.auto_cycle = true;
        e.abilities.auto_cycle_ability_id = Some(597);
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);

    // args = [0] → enabled = false
    let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.abilities.auto_cycle);
    assert!(
        e.abilities.auto_cycle_ability_id.is_none(),
        "disable must also clear auto_cycle_ability_id"
    );
}

/// SET_AUTO_CYCLE enable doesn't touch auto_cycle_ability_id —
/// that's set elsewhere. Pin so a refactor that conflates the
/// two doesn't leak.
#[tokio::test]
async fn set_auto_cycle_enable_only_sets_flag() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    assert!(handled);
    let e = mgr.get_entity(1).unwrap();
    assert!(e.abilities.auto_cycle);
    // auto_cycle_ability_id stays None (was never set)
    assert!(e.abilities.auto_cycle_ability_id.is_none());
}

/// TRIGGER_REGION with a negative region_id must be rejected by
/// the explicit `u32::try_from` guard, NOT by accidentally
/// missing a sign-extended u32 lookup. Pre-seed a real region at
/// the sign-extended id (`-5i32 as u32 == 0xFFFFFFFB`); if the
/// regression resurfaces (the cast slips through), the lookup
/// would match the planted region and fire content events.
/// With the negative-id guard in place the planted region must
/// stay invisible.
#[tokio::test]
async fn trigger_region_with_negative_id_rejects_via_explicit_guard() {
    use crate::cell::space_manager::RegionData;
    let mut mgr = make_mgr_with_player();
    // Plant a region at the sign-extended id of -5. If a regression
    // reintroduces the `region_id as u32` cast, get_region(0xFFFFFFFB)
    // would match this row and fire ring_transport / fire_enter_region.
    let trap_id: u32 = (-5i32) as u32;
    mgr.regions.insert(
        trap_id,
        RegionData {
            runtime_id: trap_id,
            db_set_id: 9999,
            tag: "trap".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 0,
            points: vec![],
        },
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    // Layout: i32 region_id + u8 b_entering + 3 × f32 position.
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&(-5i32).to_le_bytes());
    args.push(1);
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());

    let handled = dispatch(1, TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    assert!(
        handled,
        "TRIGGER_REGION must claim the method even when region_id is bogus"
    );
    // The planted trap region MUST NOT match. No fire_*_region
    // cascade, no ring_transport message.
    assert!(
        rx.try_recv().is_err(),
        "negative region_id must be rejected by u32::try_from before lookup, \
         so the trap region at 0xFFFFFFFB can't fire"
    );
}

/// `handle_reload` is a no-op when the active slot is at full
/// clip and no reload is in flight. Pin so a refactor that
/// always starts a reload (and therefore wastes ammo on every
/// keypress) gets caught.
#[tokio::test]
async fn handle_reload_no_op_when_already_full() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30, // full
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    let (tx, mut rx) = mpsc::channel(8);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "no reload should be queued when full"
    );
    assert!(rx.try_recv().is_err(), "no packets should be emitted");
}

/// `handle_reload` from an empty magazine pins the slot id at
/// the time of issue. If the player swaps mid-reload, the
/// completion tick must refill THIS slot, not whatever slot is
/// active when the deadline elapses.
#[tokio::test]
async fn handle_reload_pins_reload_slot_id_to_current_active_slot() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B slot
        // pin, not the Phase A defer path.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            2,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 2;
    }
    // Seed the reload AbilityDef so warmup/cooldown/event_set are read.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    let (tx, _rx) = mpsc::channel(16);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "reload must arm the deadline"
    );
    assert_eq!(
        e.reload_slot_id,
        Some(2),
        "reload_slot_id must capture the active slot at issue time, not be re-read at completion"
    );
}

/// Reload-start must fire the `Item_Reload` (event 4002) sequence from
/// the player's archetype-keyed "Item handling" event set so the
/// client plays the visible reload animation. Mirrors
/// `python/cell/SGWBeing.py:863-874` (`getItemSequence(Item_Reload)` +
/// `playSequence`). Previously this site looked up the *reload
/// ability's* `event_set_id`, which is NULL in the seed — the lookup
/// short-circuited and no animation ever played in production.
///
/// Bug shape this catches: a refactor that goes back to keying off
/// `ability_defs[596].event_set_id` reintroduces the dead path.
#[tokio::test]
async fn handle_reload_sends_item_reload_sequence() {
    use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
    use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ITEM_RELOAD};

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Soldier archetype → event set 804 (the human "Item handling"
        // set per `archetype_item_event_set`).
        e.archetype_id = Some(1);
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B byte
        // layout of Item_Reload's ON_SEQUENCE.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the sequence_map so the (804, Item_Reload) lookup finds a
    // sentinel sequence id we can recognise on the wire.
    const HUMAN_ITEM_EVENT_SET: i32 = 804;
    const ITEM_RELOAD_SEQ_ID: i32 = 1874;
    mgr.sequence_map.insert(
        (HUMAN_ITEM_EVENT_SET, EVENT_ITEM_RELOAD),
        ITEM_RELOAD_SEQ_ID,
    );
    // Seed a decoy (ability's event set → Ability_Begin) — the
    // regression we're guarding against would have sent THIS one.
    const DECOY_EVENT_SET: i32 = 7777;
    const DECOY_BEGIN_SEQ_ID: i32 = 9001;
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: Some(DECOY_EVENT_SET),
            velocity: 0.0,
        },
    );
    mgr.sequence_map
        .insert((DECOY_EVENT_SET, EVENT_ABILITY_BEGIN), DECOY_BEGIN_SEQ_ID);

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire path):
    //   sequence_id   i32 LE  @ 0..4
    //   source_id     i32 LE  @ 4..8
    //   target_id     i32 LE  @ 8..12
    //   primary       u8      @ 12
    //   impact_time   f32 LE  @ 13..17
    //   nvp_count     u32 LE  @ 17..21
    //   view_type     u8      @ 21
    //   instance_id   i32 LE  @ 22..26
    let mut item_reload_count = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_SEQUENCE {
                assert_eq!(
                    args.len(),
                    26,
                    "ON_SEQUENCE payload must be exactly 26 bytes — any drift \
                     in the serializer would silently corrupt the kismet event \
                     frame on the wire"
                );
                let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_ne!(
                    seq_id, DECOY_BEGIN_SEQ_ID,
                    "reload-start must NOT fire the reload ability's Ability_Begin \
                     (that's the dead pre-fix path)",
                );
                if seq_id != ITEM_RELOAD_SEQ_ID {
                    continue;
                }
                let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let primary = args[12];
                let impact_time = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);
                let nvp_count = u32::from_le_bytes([args[17], args[18], args[19], args[20]]);
                let view_type = args[21];
                let instance_id = i32::from_le_bytes([args[22], args[23], args[24], args[25]]);
                assert_eq!(
                    source_id, 1,
                    "source = entity_id (player firing the reload)"
                );
                assert_eq!(target_id, 1, "reload targets self");
                assert_eq!(primary, 1, "primary target flag set");
                assert_eq!(impact_time, 0.0, "no projectile impact time for reload");
                assert_eq!(nvp_count, 0, "no name-value pairs in payload");
                assert_eq!(
                    view_type, 0,
                    "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's \
                     fire path so reload-begin animates consistently with weapon \
                     fire animations"
                );
                assert_eq!(instance_id, 0, "no effect instance for the reload sequence");
                item_reload_count += 1;
            }
        }
    }
    assert_eq!(
        item_reload_count, 1,
        "reload-start must send exactly one onSequence with the Item_Reload \
         sequence id; without it the client plays no visible reload animation. \
         Got {item_reload_count}.",
    );
}

/// Reload-while-holstered Phase A: a player who's OOC and
/// holstered presses reload. The handler defers the actual reload
/// to give the draw animation time to play. Phase A must:
///   1. Flip `weapon_holstered` to false.
///   2. Stamp `combat_exit_at` so the OOC re-holster timer fires
///      AFTER the eventual Phase B reload completes.
///   3. Set `pending_reload_at = now + UNHOLSTER_DRAW_DURATION` so
///      `pending_reload_tick` can promote Phase A → Phase B.
///   4. Dispatch `RefreshAppearance` (mesh attaches at hand socket).
///   5. NOT start the reload-completion timer or fire `Item_Reload`
///      yet — those land in Phase B.
///
/// Bug shape this catches (the playtest report that drove the fix):
/// firing `Item_Reload` and the appearance change in the same tick
/// makes the weapon "teleport into the hand + reload anim plays on
/// empty space", and the player has to press reload twice.
#[tokio::test]
async fn reload_while_holstered_phase_a_defers_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = true; // OOC + holstered
        e.combat_exit_at = None;
        e.pending_reload_at = None;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "Phase A must draw the weapon");
    assert!(
        e.combat_exit_at.is_some(),
        "Phase A must stamp combat_exit_at so OOC re-holster fires AFTER \
         the eventual reload completes",
    );
    assert!(
        e.pending_reload_at.is_some(),
        "Phase A must set pending_reload_at so the deferred-reload tick \
         can promote to Phase B once the draw window elapses",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "Phase A must NOT start the reload-completion timer — the actual \
         reload hasn't started yet, only the draw. Firing the reload here \
         is the bug shape we're explicitly avoiding (user playtest: \
         'weapon teleports into my hand and I still need to hit reload again')",
    );

    let mut saw_refresh = false;
    while let Ok(msg) = rx.try_recv() {
        if matches!(
            msg,
            CellToBaseMsg::RefreshAppearance {
                holstered: false,
                ..
            }
        ) {
            saw_refresh = true;
            break;
        }
    }
    assert!(
        saw_refresh,
        "Phase A must dispatch RefreshAppearance(holstered=false) so the \
         client attaches the weapon mesh at the hand socket before the \
         draw animation triggers",
    );
}

/// Phase A → Phase B promotion: once the draw window has
/// elapsed, calling `handle_reload` again (as the
/// `pending_reload_tick` does) finds `pending_reload_at` set,
/// clears it, and runs the normal Phase B reload start
/// (`reload_complete_at` armed, `Item_Reload` sequence fired).
///
/// Bug shape this catches: a refactor that forgets to clear
/// `pending_reload_at` in Phase B leaves the tick re-firing
/// `handle_reload` every 100ms forever.
#[tokio::test]
async fn reload_phase_a_to_phase_b_clears_pending_and_starts_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        // Already drawn by Phase A; `pending_reload_at` is what the
        // promotion key reads.
        e.weapon_holstered = false;
        e.combat_exit_at = Some(std::time::Instant::now());
        e.pending_reload_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.pending_reload_at.is_none(),
        "Phase B must clear pending_reload_at so the tick doesn't re-fire \
         handle_reload every 100ms forever",
    );
    assert!(
        e.reload_complete_at.is_some(),
        "Phase B must start the reload (set reload_complete_at) so the \
         completion tick can promote the ammo refill",
    );
}

/// Reload-while-in-OOC-grace (weapon already drawn): the timer
/// must be RE-STAMPED so it doesn't fire `OOC_HOLSTER_DELAY`
/// seconds after combat ended — which could land mid-reload and
/// holster the weapon while the animation is still playing.
#[tokio::test]
async fn reload_during_ooc_grace_resets_holster_timer() {
    let mut mgr = make_mgr_with_player();
    let stale_stamp = std::time::Instant::now() - std::time::Duration::from_secs(8);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // OOC but still drawn
        e.combat_exit_at = Some(stale_stamp);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "already-drawn weapon stays drawn");
    let new_stamp = e.combat_exit_at.expect("timer must remain armed");
    assert!(
        new_stamp > stale_stamp,
        "timer must be re-stamped to current time so the existing \
         OOC_HOLSTER_DELAY countdown doesn't expire mid-reload",
    );
}

/// Second reload press during the Phase A draw window must be
/// silently ignored. Without this gate, the second press falls
/// through to Phase B, clears `pending_reload_at` early, and
/// starts the reload cooldown immediately — defeating the draw
/// animation timing.
///
/// Bug shape: refactor drops the `now < pending_reload_at` check
/// at the top of Phase B; a player mashing R during the draw
/// window triggers Phase B prematurely and the reload anim
/// chains in mid-draw (the symptom that drove the original
/// two-phase split).
#[tokio::test]
async fn reload_second_press_during_draw_window_is_ignored() {
    let mut mgr = make_mgr_with_player();
    let future = std::time::Instant::now() + std::time::Duration::from_millis(800);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // weapon drawn (Phase A finished its draw)
        e.combat_exit_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Phase A already fired — Phase B is queued for the future.
        e.pending_reload_at = Some(future);
    }
    // No reload ability def needed — the gate fires before any
    // ability lookup.

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.pending_reload_at,
        Some(future),
        "second press must NOT clear pending_reload_at — the \
         tick still owns the Phase B promotion at the right time",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "second press must NOT start the reload cooldown — Phase B \
         would otherwise fire mid-draw and chain the reload \
         animation before the unholster motion finishes",
    );
}

/// Reload-in-isolation regression: reloading without any aggro must
/// NOT flip BSF_InCombat on the player. The previous bug: the
/// reload handler set the bit raw, but reload doesn't generate
/// threat on anything — so no NPC death would ever clear the bit,
/// stranding the player in the in-combat HUD/cursor forever (and
/// blocking the out-of-combat regen tick, which gates on
/// `threatened_mobs.is_empty()`).
///
#[tokio::test]
async fn reload_in_isolation_does_not_flip_bsf_in_combat() {
    use crate::cell::combat::BSF_IN_COMBAT;

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the reload AbilityDef so the warmup path runs.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let s = mgr.get_entity(1).unwrap().state_field;
    assert_eq!(
        s & BSF_IN_COMBAT,
        0,
        "reload MUST NOT flip BSF_InCombat — reload-without-aggro had no \
         NPC-death clear path and the bit would strand forever"
    );
    assert!(
        mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
        "reload must leave threatened_mobs empty — the source of truth \
         for the in-combat state"
    );
}
