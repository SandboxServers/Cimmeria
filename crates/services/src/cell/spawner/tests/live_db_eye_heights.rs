//! Live-DB guards for `resources.body_sets.eye_height` (NA31).
//!
//! The values were measured from each body set's reference skeletal mesh in
//! the cooked client (`docs/reverse-engineering/findings/being-eye-heights.md`).
//! The loader reads them into `SpaceManager::body_set_eye_heights`, and line
//! of sight casts between those eyes.

use std::collections::HashSet;

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{load_body_set_eye_heights, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// The playable and main NPC body sets load their measured eye heights, and
/// a body set with no reference mesh stays out (the 1.5 m default).
#[tokio::test]
async fn seeded_eye_heights_load_by_body_set() {
    let pool = require_db_or_skip!();
    let m = load_body_set_eye_heights(&pool)
        .await
        .expect("load_body_set_eye_heights");
    for (bs, want) in [
        ("BS_HumanMale.BS_HumanMale", 1.81),
        ("BS_HumanFemale.BS_HumanFemale", 1.71),
        ("BS_JaffaMale.BS_JaffaMale", 2.12),
        ("BS_JaffaFemale.BS_JaffaFemale", 2.02),
        ("BS_GoauldMale.BS_GoauldMale", 1.99),
        ("BS_GoauldFemale.BS_GoauldFemale", 1.81),
        ("BS_Asgard.BS_Asgard", 1.25),
        ("MOB_AMBRat.BS_MOB_Rat", 0.15),
    ] {
        let got = m.get(bs).copied();
        assert!(
            got.is_some_and(|h| (h - want).abs() < 1e-4),
            "{bs}: {got:?}, want {want}"
        );
    }
    assert!(!m.contains_key("GLB_Components.WorldObject_Small"));
}

/// Every body set a seeded spawn uses that has a reference mesh has an eye
/// height. `BS_JaffaMale` (50 templates) had no `body_sets` row at all
/// until NA31; a new template on an unmeasured body set fails here.
#[tokio::test]
async fn every_spawned_being_body_set_has_an_eye_height() {
    let pool = require_db_or_skip!();
    let m = load_body_set_eye_heights(&pool).await.expect("load");
    let spawns = load_spawns_from_db(&pool).await.expect("spawns");
    let missing: HashSet<&str> = spawns
        .iter()
        .map(|s| s.body_set.as_str())
        // Props and terminals have no skeleton to measure.
        .filter(|b| !b.starts_with("GLB_Components."))
        .filter(|b| !m.contains_key(*b))
        .collect();
    assert!(
        missing.is_empty(),
        "body sets with no eye height: {missing:?}"
    );
}

/// The loaded table drives `eye_height_of` for a spawned NPC.
#[tokio::test]
async fn a_seeded_jaffa_looks_from_its_measured_eye() {
    let pool = require_db_or_skip!();
    let mut mgr = SpaceManager::new(1);
    mgr.body_set_eye_heights = load_body_set_eye_heights(&pool).await.expect("load");
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    mgr.spawn_npc(7, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(7).unwrap();
    e.body_set = Some("BS_JaffaMale.BS_JaffaMale".into());
    let e = mgr.get_entity(7).unwrap();
    assert!((mgr.eye_height_of(e) - 2.12).abs() < 1e-4);
}
