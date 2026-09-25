//! Live-DB guards for `entity_templates.use_cover` and the Cover Stance
//! effect rows (NA22).
//!
//! The column is nullable: NULL means the runtime default rule (a hostile
//! NPC takes cover), so the loaders read it without a COALESCE. The seed
//! sets it on the Cellblock and Castle combat templates only. The last test
//! runs the whole chain on seeded data: the real spawn row, the real
//! world-12 cover rows and world ids, and the spawn hold.

use std::collections::HashMap;

use cimmeria_common::EntityId;

use crate::cell::cover::{self, CoverSlotKey, COVER_STANCE_EFFECT, COVER_STANCE_REMOVE_EFFECT};
use crate::cell::space_manager::{resolve_use_cover, SpaceManager};
use crate::cell::spawner::{
    load_effect_defs, load_spawn_templates, load_spawns_from_db, load_world_rows,
};
use crate::test_support::require_db_or_skip;

/// Templates the seed opts in, and the two drones it opts out.
const COVER_TEMPLATES: [i32; 7] = [15, 24, 146, 148, 169, 170, 171];
const NO_COVER_TEMPLATES: [i32; 2] = [4, 145];

/// The seeded values reach both loaders (seeded spawns and the template
/// cache that content and GM spawns use), and an unset template stays NULL.
/// Fails if the column is dropped from either SELECT or COALESCEd.
#[tokio::test]
async fn seeded_use_cover_values_reach_both_loaders() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db");
    let templates = load_spawn_templates(&pool)
        .await
        .expect("load_spawn_templates");

    for (tids, want) in [
        (&COVER_TEMPLATES[..], Some(true)),
        (&NO_COVER_TEMPLATES[..], Some(false)),
    ] {
        for &tid in tids {
            let rows: Vec<_> = spawns.iter().filter(|s| s.template_id == tid).collect();
            assert!(!rows.is_empty(), "template {tid} is spawned somewhere");
            assert!(
                rows.iter().all(|s| s.use_cover == want),
                "template {tid}: every spawn must load use_cover = {want:?}"
            );
            assert_eq!(templates[&tid].use_cover, want, "template cache, {tid}");
        }
    }
    // Prisoner 329: a non-combat template the seed leaves NULL.
    assert_eq!(templates[&17].use_cover, None);
}

/// The resolved rule on real rows: the ranged guards take cover, the
/// stationary PRU and a friendly NPC do not.
#[tokio::test]
async fn seeded_spawns_resolve_use_cover_as_designed() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db");
    let by_tag = |tag: &str| {
        spawns
            .iter()
            .find(|s| s.tag.as_deref() == Some(tag))
            .unwrap_or_else(|| panic!("seeded spawn {tag}"))
    };
    assert!(resolve_use_cover(by_tag("MessHall_Guard1")));
    assert!(resolve_use_cover(by_tag("Castle_nidGuard3")));
    assert!(!resolve_use_cover(by_tag(
        "ArmYourself_PrisonerRetrievalUnit"
    )));
    assert!(!resolve_use_cover(by_tag("Castle_PRU1")));
}

/// Effects 4565 and 1742 name the Cover Stance scripts. Without them the
/// stance is tracked but no stat changes (`cover.stance event=effect_missing`).
#[tokio::test]
async fn cover_stance_effect_rows_name_their_scripts() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");
    for (id, script) in [
        (COVER_STANCE_EFFECT, "CoverStance"),
        (COVER_STANCE_REMOVE_EFFECT, "RemoveCoverStance"),
    ] {
        let def = defs
            .get(&id)
            .unwrap_or_else(|| panic!("effect {id} seeded"));
        assert_eq!(def.ability_id, cover::COVER_STANCE_ABILITY);
        assert_eq!(def.script_name.as_deref(), Some(script), "effect {id}");
        assert!(
            crate::cell::effects::registry::lookup(script).is_some(),
            "{script} must be registered"
        );
    }
}

/// End to end on seeded data: `MessHall_Guard1` spawns holding the cover
/// marker it is authored at (1200046/0), with cover and world ids loaded the
/// way `start()` loads them.
#[tokio::test]
async fn mess_hall_guard_spawns_holding_its_seeded_cover_slot() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db");
    let record = spawns
        .iter()
        .find(|s| s.tag.as_deref() == Some("MessHall_Guard1"))
        .expect("MessHall_Guard1 is seeded")
        .clone();

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let rows: HashMap<_, _> = load_world_rows(&pool).await.expect("load_world_rows");
    mgr.stamp_world_rows(&rows);
    mgr.cover = cover::Cover::from_loaded(
        cover::load_cover_sets(&pool).await.expect("cover sets"),
        cover::load_cover_nodes(&pool).await.expect("cover nodes"),
    );

    mgr.spawn_npc_from_record(200, &record).unwrap();
    let held = mgr
        .cover
        .reservations
        .lock()
        .unwrap()
        .slot_for_entity(EntityId(200));
    assert_eq!(held, Some(CoverSlotKey::new(1200046, 0)));
}
