//! AB-T5: a player's death writes one `abilities.snapshot` row of what the
//! player died with, before the clear-on-death strip; an NPC's writes none.

use std::time::Instant;

use cimmeria_entity::abilities::EF_CLEAR_ON_DEATH;
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ACCURACY;
use tokio::sync::mpsc;
use tracing::Level;

use super::resolve_death;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

const KILLER: u32 = 1;
const VICTIM: u32 = 2;

fn world(victim_is_player: bool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(KILLER, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(VICTIM, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let v = mgr.get_entity_mut(VICTIM).unwrap();
    v.is_player = victim_is_player;
    v.player_id = victim_is_player.then_some(200);
    v.apply_timed_effect(
        TimedEffectSpec {
            cast_id: Some(5),
            effect_id: 700,
            ability_id: 637,
            invoker_id: VICTIM,
            effect_flags: EF_CLEAR_ON_DEATH,
            moniker_ids: vec![],
            stats: vec![(ACCURACY, 200)],
            absorb: Vec::new(),
            state_flags: 0,
            duration_secs: Some(15.0),
            stacking: TimedStacking::PerSource,
            invoker_identity: Default::default(),
        },
        Instant::now(),
    )
    .expect("buff applies");
    mgr
}

/// Revert proof: drop the hook from `resolve_death` and no row is found; move
/// it after `clear_stat_buffs_on_death` and the row's ledger is empty.
#[tokio::test]
async fn ab_t5_player_death_snapshots_the_state_before_the_strip() {
    let mut mgr = world(true);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(resolve_death(VICTIM, KILLER, Some(592), false, false, &tx, &mut mgr).await);

    let row = logs
        .find_message(Level::INFO, "ability state snapshot")
        .expect("one snapshot row on a player's death");
    assert!(row.has_field("trigger", "death"));
    assert!(row.has_field("entity_id", &VICTIM.to_string()));
    assert!(row.has_field("ledger", "1"), "taken before the strip");
    assert!(
        row.fields["snapshot"].contains("\"effect_id\":700"),
        "{}",
        row.fields["snapshot"]
    );
    assert!(
        mgr.get_entity(VICTIM)
            .unwrap()
            .stat_buffs
            .entries
            .is_empty(),
        "fixture: the clear-on-death strip did run after it"
    );
}

#[tokio::test]
async fn ab_t5_npc_death_writes_no_snapshot() {
    let mut mgr = world(false);
    let (tx, _rx) = mpsc::channel(256);
    let logs = LogCapture::install();

    assert!(resolve_death(VICTIM, KILLER, Some(592), true, false, &tx, &mut mgr).await);

    assert!(logs
        .find_message(Level::INFO, "ability state snapshot")
        .is_none());
}
