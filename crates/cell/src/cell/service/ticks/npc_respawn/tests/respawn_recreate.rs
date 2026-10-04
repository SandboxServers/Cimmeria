//! A respawned NPC is re-created on every witness's client, so the client
//! drops the death pose (2026-09-28 colo playtest: guards respawned lying
//! on the floor, alive, and could not be auto-targeted).
//!
//! The death burst ends with `onSequence` Entity_Death. No property delta
//! takes a client pawn out of that pose, so the respawn tick must send
//! each witness `LeftAoI` and then the AoI-enter introduction, carrying
//! the respawned state. These tests pin that fan-out at the cell -> base
//! boundary: per-witness cardinality, order, and the introduced state.

use super::super::*;
use crate::cell::combat::BSF_DEAD;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

const NPC: u32 = 50;
const NEAR: u32 = 1;
const ALSO_NEAR: u32 = 2;
const FAR: u32 = 3;
const SPAWN: [f32; 3] = [3.0, 0.0, 0.0];
/// A content interaction bit the NPC had before it died.
const PRE_DEATH_FLAGS: i64 = 1 << 5;

/// Two players in view of a one-shottable NPC and one player out of view.
fn world() -> SpaceManager {
    use cimmeria_entity::abilities::{AbilityDef, EffectDef};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    for (id, pos) in [
        (NEAR, [0.0, 0.0, 0.0]),
        (ALSO_NEAR, [0.0, 0.0, 10.0]),
        (FAR, [1500.0, 0.0, 1500.0]),
    ] {
        mgr.create_entity(id, "Castle", pos, [0.0; 3]).unwrap();
        let p = mgr.get_entity_mut(id).unwrap();
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        p.abilities.add_ability(7);
    }
    mgr.spawn_npc(NPC, "Castle", SPAWN, [0.0, 1.57, 0.0])
        .unwrap();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.faction = crate::cell::combat::HOSTILE_FACTION;
    npc.respawn_secs = Some(3);
    npc.original_interaction_type_flags = PRE_DEATH_FLAGS;
    npc.interaction_type_flags = PRE_DEATH_FLAGS;
    let hp = npc.stats.get_mut(HEALTH).unwrap();
    hp.update(0, 1, 100);
    hp.clear_dirty();

    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        100,
        EffectDef {
            effect_id: 100,
            ability_id: 7,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        7,
        AbilityDef {
            ability_id: 7,
            name: "test".to_string(),
            cooldown: 0.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: false,
            min_range: 0.0,
            max_range: 30.0,
            target_type_id: 0,
            effect_ids: vec![100],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
            type_id: Default::default(),
        },
    );
    for id in [NEAR, ALSO_NEAR, FAR] {
        mgr.connect_entity(id);
    }
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// Kill the NPC through the real ability path, then move its respawn
/// deadline into the past and run the tick. Returns what the tick sent.
async fn kill_and_respawn(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(512);
    let _ = crate::cell::abilities::handle_use_ability(NEAR, 7, NPC as i32, &tx, mgr).await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Dead,
        "fixture: the kill must land"
    );
    let _death_burst = drain(&mut rx);

    mgr.get_entity_mut(NPC).unwrap().respawn_at =
        Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
    npc_respawn_tick(&tx, mgr).await;
    drain(&mut rx)
}

/// What the tick sent about the NPC to one witness, as short tags in order.
fn tags_for(msgs: &[CellToBaseMsg], witness: u32) -> Vec<&'static str> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::LeftAoI {
                witness_id,
                entity_id: NPC,
            } if *witness_id == witness => Some("leave"),
            CellToBaseMsg::EnteredAoI {
                witness_id,
                entity_id: NPC,
                ..
            } if *witness_id == witness => Some("enter"),
            CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id: NPC,
                ..
            } if *witness_id == witness => Some("moved"),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id: NPC,
                method_index,
                ..
            } if *witness_id == witness => match *method_index {
                method_idx::INTERACTION_TYPE => Some("int"),
                method_idx::ON_STATE_FIELD_UPDATE => Some("state"),
                method_idx::ON_STAT_UPDATE => Some("stat"),
                _ => Some("other_method"),
            },
            _ => None,
        })
        .collect()
}

/// Each witness gets exactly `LeftAoI` then `EnteredAoI` for the NPC, and
/// nothing else about it: no delta reaches the pawn the leave destroys.
/// A player out of view gets nothing.
///
/// Fails on the pre-fix tick, which sent `EntityMoved`, `InteractionType`,
/// `onStateFieldUpdate` and `onStatUpdate` to the existing (dead-posed) pawn.
#[tokio::test]
async fn respawn_recreates_the_npc_on_every_witness() {
    let mut mgr = world();
    let mut witnesses = mgr.get_witnesses_of(NPC);
    witnesses.sort_unstable();
    assert_eq!(witnesses, vec![NEAR, ALSO_NEAR], "fixture: two witnesses");

    let msgs = kill_and_respawn(&mut mgr).await;

    for w in [NEAR, ALSO_NEAR] {
        assert_eq!(
            tags_for(&msgs, w),
            vec!["leave", "enter"],
            "witness {w} must get LeftAoI then EnteredAoI for the respawned NPC, nothing else"
        );
    }
    assert!(
        tags_for(&msgs, FAR).is_empty(),
        "a player out of view must get nothing about the NPC"
    );
}

/// The introduction carries the respawned state: the spawn position and
/// facing, no BSF_DEAD, full HEALTH, and the pre-death interaction type
/// without INT_NormalLoot. The client builds the pawn from these, so any
/// stale field here brings the corpse back.
#[tokio::test]
async fn respawn_introduction_carries_the_alive_state() {
    let mut mgr = world();
    let msgs = kill_and_respawn(&mut mgr).await;

    let enter = msgs
        .iter()
        .find_map(|m| match m {
            CellToBaseMsg::EnteredAoI {
                witness_id: NEAR,
                entity_id: NPC,
                position,
                direction,
                npc_data,
                ..
            } => Some((*position, *direction, npc_data.clone())),
            _ => None,
        })
        .expect("the respawn must introduce the NPC to the witness");
    let (position, direction, npc_data) = enter;
    assert_eq!(position, SPAWN, "introduced at the spawn point");
    assert!(
        (direction[1] - 1.57).abs() < 1e-4,
        "introduced facing the spawn heading, got {direction:?}"
    );
    let data = npc_data.expect("an NPC introduction carries NpcAoIData");
    assert_eq!(data.state_field & BSF_DEAD, 0, "introduced alive");
    assert_eq!(
        data.interaction_type, PRE_DEATH_FLAGS,
        "introduced with the pre-death interaction type (no INT_NormalLoot)"
    );
    let vitals = data.vitals.expect("live vitals in the introduction");
    assert_eq!(vitals.health[1], vitals.health[2], "introduced at full HP");
}

/// The re-create leaves AoI bookkeeping alone: the NPC stays in both
/// witness sets, and the next AoI tick neither re-introduces it nor
/// sends a leave (which would destroy the pawn just rebuilt).
#[tokio::test]
async fn respawn_recreate_leaves_the_witness_sets_consistent() {
    let mut mgr = world();
    let _ = kill_and_respawn(&mut mgr).await;

    let mut witnesses = mgr.get_witnesses_of(NPC);
    witnesses.sort_unstable();
    assert_eq!(witnesses, vec![NEAR, ALSO_NEAR], "witness sets unchanged");

    let next = mgr.compute_aoi_changes();
    let churn: Vec<&CellToBaseMsg> = next
        .iter()
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::LeftAoI { entity_id: NPC, .. }
                    | CellToBaseMsg::EnteredAoI { entity_id: NPC, .. }
            )
        })
        .collect();
    assert!(
        churn.is_empty(),
        "the next AoI tick must not re-enter or leave the NPC: {churn:?}"
    );
}
