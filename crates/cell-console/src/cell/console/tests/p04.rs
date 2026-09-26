//! Packet P04 regression suite: `.listabilities` (new) plus the CellApp-wide
//! scope fix for `.players`.
//!
//! Filter prefix: `legacy_p04_`.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::exec;
use crate::cell::space_manager::SpaceManager;

fn ability_def(id: i32, name: &str) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: name.to_string(),
        cooldown: 0.0,
        warmup: 0.0,
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
    }
}

/// Create a fresh player entity in `setup()`'s "Agnos" space with no
/// abilities pre-granted — unlike `setup()`'s own NPC target, which goes
/// through `SpaceManager::spawn_npc` and so always carries the
/// `NPC_DEFAULT_ABILITY` (592, Pistol Shot) seen in
/// `space_manager/tests/npc_spawn.rs`. Using a plain `create_entity` player
/// here keeps this suite's ability lists exact and free of that unrelated
/// baseline grant.
fn bare_player(mgr: &mut SpaceManager, entity_id: u32) {
    mgr.create_entity(entity_id, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(entity_id);
}

/// `.listabilities` on a target with two known abilities — one resolvable
/// against the ability catalog, one not — must print the header line, the
/// resolved name for the known id, and the `<unknown ability N>` fallback
/// for the unresolvable one, in ascending-id order.
#[tokio::test]
async fn legacy_p04_listabilities_resolves_known_and_falls_back_unknown() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    bare_player(&mut mgr, target);
    mgr.ability_defs
        .insert(101, ability_def(101, "Stunning Blow"));
    // 999 is deliberately absent from ability_defs.
    if let Some(e) = mgr.get_entity_mut(target) {
        e.abilities.add_ability(999);
        e.abilities.add_ability(101);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec(
        "listabilities",
        gm,
        &[],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    assert_eq!(
        lines,
        vec![
            format!("Ability list of entity {target}:"),
            "    Stunning Blow".to_string(),
            "    <unknown ability 999>".to_string(),
        ],
        "known id must resolve by name, unknown id must retain a fallback line, sorted by id"
    );
}

/// A target with no known abilities still gets the header line and no rows —
/// mirrors legacy's empty-loop behavior rather than a special "none" message.
#[tokio::test]
async fn legacy_p04_listabilities_no_abilities_prints_header_only() {
    let (mut mgr, gm, _npc) = setup();
    let target = 2u32;
    bare_player(&mut mgr, target);
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec(
        "listabilities",
        gm,
        &[],
        Some(target),
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    assert_eq!(lines, vec![format!("Ability list of entity {target}:")]);
}

/// `.listabilities` on a non-player target must be rejected by the shared
/// `Target::Player` guard before the handler ever runs — same contract as
/// every other typed command.
#[tokio::test]
async fn legacy_p04_listabilities_rejects_non_player_target() {
    let (mut mgr, gm, _npc) = setup(); // setup's npc has is_player = false
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::console::handle_console_command(gm, ".listabilities", &tx, &mut mgr, &engine)
        .await;

    let mut saw_rejection = false;
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            if text.contains("expected a player as a target") {
                saw_rejection = true;
            }
        }
    }
    assert!(
        saw_rejection,
        "a non-player target must be rejected before list_abilities runs"
    );
}

/// Build a second loaded space ("Landing") with its own connected player,
/// alongside `setup()`'s default "Agnos" space and its GM. Returns
/// `(mgr, gm, other_world_player_id)`.
fn setup_two_spaces() -> (SpaceManager, u32, u32) {
    let (mut mgr, gm, _npc) = setup();

    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Landing" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Landing" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    let other = 2u32;
    mgr.create_entity(other, "Landing", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(other);
    if let Some(e) = mgr.get_entity_mut(other) {
        e.character_name = Some("Teal'c".to_string());
    }
    if let Some(e) = mgr.get_entity_mut(gm) {
        e.character_name = Some("Daniel".to_string());
    }

    (mgr, gm, other)
}

/// `.players` must list players from every loaded space on this service, not
/// just the caller's own — the exact bug this packet fixes. Two spaces, one
/// player each; both rows must appear with their correct world names.
#[tokio::test]
async fn legacy_p04_players_spans_every_loaded_space() {
    let (mut mgr, gm, _other) = setup_two_spaces();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec("players", gm, &[], None, &tx, &mut mgr, &engine).await;

    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    assert_eq!(
        lines,
        vec![
            "Players online on this CellApp:".to_string(),
            " - Daniel (Agnos)".to_string(),
            " - Teal'c (Landing)".to_string(),
        ],
        "both spaces' players must appear, sorted by entity id, with their own world: {lines:?}"
    );
}

/// A player with no cached `character_name` falls back to a stable
/// `"player {id}"` label instead of silently omitting the row.
#[tokio::test]
async fn legacy_p04_players_falls_back_to_id_label_when_name_missing() {
    let (mut mgr, gm, _npc) = setup();
    // setup()'s gm has no character_name set by default.
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec("players", gm, &[], None, &tx, &mut mgr, &engine).await;

    let mut lines = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            lines.push(text);
        }
    }
    assert_eq!(
        lines,
        vec![
            "Players online on this CellApp:".to_string(),
            format!(" - player {gm} (Agnos)"),
        ]
    );
}

/// `.players` performs no mutation — a caller-only read that must not touch
/// any entity or authoring state.
#[tokio::test]
async fn legacy_p04_players_is_caller_only_and_non_mutating() {
    let (mut mgr, gm, other) = setup_two_spaces();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);
    exec("players", gm, &[], None, &tx, &mut mgr, &engine).await;

    // Every feedback message must route to the caller, never the other
    // player, and no authoring SQL should be recorded.
    while let Ok(msg) = rx.try_recv() {
        if let crate::cell::messages::CellToBaseMsg::EntityMethodCall { entity_id, .. } = msg {
            assert_eq!(
                entity_id, gm,
                "players output must be delivered only to the caller"
            );
        }
    }
    assert!(mgr.authoring_changes.get(&gm).is_none_or(Vec::is_empty));
    assert!(
        mgr.get_entity(other).is_some(),
        "players must not mutate entities"
    );
}
