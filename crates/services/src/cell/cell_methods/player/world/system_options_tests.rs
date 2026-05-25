//! Regression guards for the `updateSystemOptions` handler and the two
//! reload triggers it gates: auto-reload (clip → 0) and reload-on-activate
//! (bandolier slot switch). Pins the wire-format parse + the
//! option-aware behaviour of both seams.
//!
//! Companion to [`super::tests`] which covers the other player-world
//! cell methods. These tests sit in their own file because the parent
//! test file already pushes the 700-line file cap.

use super::*;
use crate::cell::cell_methods::player::constants::UPDATE_SYSTEM_OPTIONS;
use crate::mercury::write_wstring;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;

const ABILITY_RIFLE_FIRE: i32 = 100;

fn make_player_with_ability() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.ability_defs.insert(
        ABILITY_RIFLE_FIRE,
        AbilityDef {
            ability_id: ABILITY_RIFLE_FIRE,
            is_ranged: true,
            min_range: 0,
            name: "RifleFire".into(),
            warmup: 0.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    // Reload ability used by handle_reload's looked-up def.
    mgr.ability_defs.insert(
        ABILITY_RELOAD_WEAPON,
        AbilityDef {
            ability_id: ABILITY_RELOAD_WEAPON,
            is_ranged: false,
            min_range: 0,
            name: "Reload".into(),
            warmup: 1.0,
            cooldown: 0.5,
            flags: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
        p.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 55,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 30,
                cur_ammo_type: 1,
            },
        );
        p.active_bandolier_slot = 0;
        p.weapon_holstered = false; // drawn — skip Phase A draw window
    }
    mgr.connect_entity(1);
    mgr
}

fn build_options_payload(pairs: &[(&str, &str)]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    for (name, value) in pairs {
        write_wstring(&mut buf, name);
        write_wstring(&mut buf, value);
    }
    buf
}

// ── parse_name_value_pairs ────────────────────────────────────────────────

#[test]
fn parse_name_value_pairs_decodes_two_options() {
    let buf = build_options_payload(&[("autoReload", "1"), ("reloadOnActivate", "0")]);
    let pairs = parse_name_value_pairs(&buf).unwrap();
    assert_eq!(
        pairs,
        vec![
            ("autoReload".into(), "1".into()),
            ("reloadOnActivate".into(), "0".into()),
        ]
    );
}

#[test]
fn parse_name_value_pairs_decodes_empty_array() {
    let buf = build_options_payload(&[]);
    let pairs = parse_name_value_pairs(&buf).unwrap();
    assert!(pairs.is_empty());
}

#[test]
fn parse_name_value_pairs_rejects_truncated_count() {
    // 3 bytes — not enough for the u32 count prefix.
    let err = parse_name_value_pairs(&[0, 0, 0]).unwrap_err();
    assert!(err.contains("count"), "got {err:?}");
}

#[test]
fn parse_name_value_pairs_rejects_implausibly_large_count() {
    // count = 0x7FFFFFFF would otherwise cause Vec::with_capacity to OOM.
    let mut buf = Vec::new();
    buf.extend_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
    let err = parse_name_value_pairs(&buf).unwrap_err();
    assert!(err.contains("implausible"), "got {err:?}");
}

// ── UPDATE_SYSTEM_OPTIONS dispatch ────────────────────────────────────────

#[tokio::test]
async fn update_system_options_applies_both_known_options() {
    let mut mgr = make_player_with_ability();
    // Flip from defaults: autoReload=true → false, reloadOnActivate=false → true.
    let payload = build_options_payload(&[("autoReload", "0"), ("reloadOnActivate", "1")]);
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    let handled = dispatch(1, UPDATE_SYSTEM_OPTIONS, &payload, &tx, &mut mgr, &engine).await;
    assert!(handled);

    let opts = &mgr.get_entity(1).unwrap().system_options;
    assert!(!opts.auto_reload);
    assert!(opts.reload_on_activate);
}

#[tokio::test]
async fn update_system_options_ignores_unknown_keys_without_clobbering_known_ones() {
    let mut mgr = make_player_with_ability();
    // One unknown + one known. The known one must still apply.
    let payload = build_options_payload(&[
        ("notARealOption", "42"),
        ("reloadOnActivate", "1"),
        ("alsoNotReal", "potato"),
    ]);
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let handled = dispatch(1, UPDATE_SYSTEM_OPTIONS, &payload, &tx, &mut mgr, &engine).await;
    assert!(handled);

    let opts = &mgr.get_entity(1).unwrap().system_options;
    // Default for auto_reload is true (defaultBool='true' in XML); not
    // present in payload, so should stay true.
    assert!(
        opts.auto_reload,
        "unknown key must not corrupt other options"
    );
    assert!(
        opts.reload_on_activate,
        "known key must apply alongside unknowns"
    );
}

#[tokio::test]
async fn update_system_options_with_garbage_payload_leaves_options_unchanged() {
    let mut mgr = make_player_with_ability();
    // Set non-default values so we can detect any mutation.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = false;
        p.system_options.reload_on_activate = true;
    }
    // count=2 but only enough bytes for an empty array — name parse fails.
    let mut payload = Vec::new();
    payload.extend_from_slice(&2u32.to_le_bytes());
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let handled = dispatch(1, UPDATE_SYSTEM_OPTIONS, &payload, &tx, &mut mgr, &engine).await;
    assert!(handled, "dispatch returns true even on parse failure");

    let opts = &mgr.get_entity(1).unwrap().system_options;
    // Both stay at the test-set non-default values — failed parse must
    // be all-or-nothing, not partial.
    assert!(!opts.auto_reload);
    assert!(opts.reload_on_activate);
}

// ── maybe_trigger_reload_on_activate ──────────────────────────────────────

#[tokio::test]
async fn reload_on_activate_triggers_when_option_set_and_clip_partial() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.reload_on_activate = true;
        // Partial clip — needs reload.
        p.set_slot_ammo(0, 10);
    }
    let (tx, _rx) = mpsc::channel(32);
    maybe_trigger_reload_on_activate(1, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "reload-on-activate with option=true and partial clip must start a reload"
    );
}

#[tokio::test]
async fn reload_on_activate_skips_when_option_unset() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.reload_on_activate = false; // XML default
        p.set_slot_ammo(0, 10); // partial clip
    }
    let (tx, _rx) = mpsc::channel(32);
    maybe_trigger_reload_on_activate(1, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "option off must not reload even with a partial clip"
    );
}

#[tokio::test]
async fn reload_on_activate_skips_when_clip_full() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.reload_on_activate = true;
        // already at 30/30
    }
    let (tx, _rx) = mpsc::channel(32);
    maybe_trigger_reload_on_activate(1, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "full clip + option on → no-op (handle_reload's own guard)"
    );
}

#[tokio::test]
async fn reload_on_activate_skips_when_reload_already_in_flight() {
    let mut mgr = make_player_with_ability();
    let now = std::time::Instant::now();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.reload_on_activate = true;
        p.set_slot_ammo(0, 10);
        // Simulate an in-flight reload already running.
        p.reload_complete_at = Some(now + std::time::Duration::from_secs(2));
    }
    let (tx, _rx) = mpsc::channel(32);
    maybe_trigger_reload_on_activate(1, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    // The stamped deadline should still be the test's value — the
    // helper must not stomp on an in-flight reload.
    assert_eq!(
        e.reload_complete_at,
        Some(now + std::time::Duration::from_secs(2)),
        "in-flight reload must not be restarted by activate"
    );
}

#[tokio::test]
async fn reload_on_activate_skips_for_npc() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = false; // pretend it's an NPC
        p.system_options.reload_on_activate = true;
        p.set_slot_ammo(0, 10);
    }
    let (tx, _rx) = mpsc::channel(32);
    maybe_trigger_reload_on_activate(1, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "non-player must never trigger reload-on-activate"
    );
}

// ── maybe_trigger_auto_reload (direct unit tests) ─────────────────────────
//
// The fire path (`handle_use_ability`) calls this helper at both
// exit-success points. These tests pin the gate decisions directly
// rather than driving through the full fire pipeline — the integration
// path is already covered by the existing use_ability tests.
//
// Path under test: `use_ability::maybe_trigger_auto_reload`. The helper
// is private but exposed in tests via the `super::*` import (test_helpers
// for `use_ability` live in its own module; we replicate the gates here
// to pin the option's behavioural contract).

#[tokio::test]
async fn auto_reload_triggers_when_clip_drains_to_zero_with_option_on() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = true;
        p.set_slot_ammo(0, 0); // simulate clip just emptied by fire
    }
    let (tx, _rx) = mpsc::channel(32);
    // Call via the use_ability re-export to exercise the actual
    // production helper, not a re-derived copy.
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, true, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "ammo=0 + autoReload=on must start a reload"
    );
}

#[tokio::test]
async fn auto_reload_skips_when_option_off() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = false; // user turned it off
        p.set_slot_ammo(0, 0);
    }
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, true, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "user turned the option off — must not auto-reload even at ammo=0"
    );
}

#[tokio::test]
async fn auto_reload_skips_when_ammo_remaining() {
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = true;
        p.set_slot_ammo(0, 1); // one bullet left — don't reload yet
    }
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, true, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "ammo>0 must not auto-reload (Phase A of handle_reload would no-op anyway, but skipping saves the call)"
    );
}

#[tokio::test]
async fn auto_reload_skips_when_reload_already_in_flight() {
    let mut mgr = make_player_with_ability();
    let now = std::time::Instant::now();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = true;
        p.set_slot_ammo(0, 0);
        p.reload_complete_at = Some(now + std::time::Duration::from_secs(2));
    }
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, true, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    // Stamp untouched — in-flight reload must not be restarted.
    assert_eq!(
        e.reload_complete_at,
        Some(now + std::time::Duration::from_secs(2)),
        "auto-reload during in-flight reload must not restart it"
    );
}

#[tokio::test]
async fn auto_reload_skips_when_ammo_was_not_consumed() {
    // Self-buff / no-target casts hit `maybe_trigger_auto_reload` with
    // `ammo_was_consumed = false` because their gate inside
    // `handle_use_ability` cleared the flag. Make sure that bypass works
    // even when the player is technically at ammo=0 from some other
    // path — the helper trusts the consumption signal as the gate.
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = true;
        p.set_slot_ammo(0, 0);
    }
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, false, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "ammo_was_consumed=false must short-circuit before the gate check"
    );
}

#[tokio::test]
async fn auto_reload_skips_for_melee_weapon_with_zero_clip_size() {
    // A melee weapon reports `active_clip_size() == 0` AND
    // `active_ammo() == 0` — without the clip-size gate the helper
    // would queue a no-op reload that handle_reload's "already at max
    // ammo" branch catches. The gate skips the wasted call.
    let mut mgr = make_player_with_ability();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.system_options.auto_reload = true;
        // Replace the bandolier slot with a melee weapon (clip_size=0).
        p.bandolier_items.insert(
            0,
            cimmeria_entity::cell_entity::BandolierItem {
                item_id: 99,
                clip_size: 0,
                default_ammo_type: 0,
                current_ammo: 0,
                cur_ammo_type: 0,
            },
        );
    }
    let (tx, _rx) = mpsc::channel(32);
    crate::cell::abilities::maybe_trigger_auto_reload_for_test(1, true, 100, &tx, &mut mgr).await;
    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "melee weapon (clip_size=0) must not trigger auto-reload"
    );
}
