//! `.dummy`: a lab target with a million Health that never fights back,
//! gone after ten minutes or with its owner, clearable only by its owner.
//! (That it never attacks is pinned against the real AI tick in
//! `cimmeria-cell`'s `npc_ai::lab_dummy`.)

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::MobAggression;
use cimmeria_entity::stats::{DEFENSE, HEALTH};
use tokio::sync::mpsc;
use tracing::Level;

use super::{console, lines, world, EntityCount, CALLER};
use crate::cell::console::abilities::dummy::{DEFAULT_DUMMY_TEMPLATE, DUMMY_TAG};
use crate::cell::console::abilities::{despawn_lab_dummies_of, lab_dummy_tick};
use crate::cell::space_manager::{
    LabDummy, SpaceManager, LAB_DUMMY_HEALTH, LAB_DUMMY_MAX_PER_OWNER,
};
use crate::cell::spawner::SpawnRecord;
use crate::test_support::LogCapture;

const WITNESS: u32 = 2;

pub(super) fn template(id: i32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: None,
        template_id: id,
        template_name: "SGC Jaffa".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_JaffaMale.BS_JaffaMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(10),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: Vec::new(),
        has_dynamic_properties: false,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![],
        respawn_secs: Some(30),
        patrol_path: Vec::new(),
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: None,
        vault_scope: cimmeria_entity::cell_entity::VaultScope::Personal,
        training_dummy: false,
    }
}

pub(super) fn dummy_world() -> SpaceManager {
    let (mut mgr, _npc) = world(2);
    mgr.spawn_templates
        .insert(DEFAULT_DUMMY_TEMPLATE, template(DEFAULT_DUMMY_TEMPLATE));
    mgr
}

#[tokio::test]
async fn ab_l2_dummy_places_a_million_health_target_that_holds_still() {
    let mut mgr = dummy_world();
    let logs = LogCapture::install();

    let msgs = console(&mut mgr, None, ".dummy").await;

    let ids = mgr.lab_dummies_of(CALLER);
    assert_eq!(ids.len(), 1);
    let d = mgr.get_entity(ids[0]).unwrap();
    let mark = d.extensions.get::<LabDummy>().expect("marked");
    assert_eq!(mark.owner_id, CALLER);
    assert_eq!(mark.disposition, MobAggression::Hostile);
    assert!(mark.expires_at > Instant::now() + Duration::from_secs(590));
    let h = d.stats.get(HEALTH).unwrap();
    assert_eq!((h.cur, h.max), (LAB_DUMMY_HEALTH, LAB_DUMMY_HEALTH));
    assert_eq!(d.aggro.override_level, Some(MobAggression::Hostile));
    assert_eq!(d.respawn_secs, None, "never respawns");
    assert!(d.is_stationary);
    assert_eq!(d.tag.as_deref(), Some(DUMMY_TAG));
    // The caller stands at (11, 0, 10) facing yaw 0: 3 m along +Z.
    assert!((d.position.x - 11.0).abs() < 1e-4 && (d.position.z - 13.0).abs() < 1e-4);

    let defense = d.stats.get(DEFENSE).map_or(0, |s| s.cur);
    let out = lines(&msgs);
    assert_eq!(out.len(), 1);
    assert!(
        out[0].starts_with(&format!("dummy [{}] placed: hostile", ids[0]))
            && out[0].contains("Health 1000000")
            && out[0].contains(&format!("Defense {defense}"))
            && out[0].contains("Accuracy ")
            && out[0].contains("never attacks"),
        "{out:?}"
    );
    let row = logs
        .find_message(Level::INFO, "GM placed a lab dummy")
        .expect("one abilities.gm row");
    assert!(row.has_field("event", "lab_dummy_spawned"));
    assert!(row.has_field("player_id", "71"));
    assert!(row.has_field("dummy_id", &ids[0].to_string()));
}

#[tokio::test]
async fn ab_l2_dummy_friendly_and_a_named_template() {
    let mut mgr = dummy_world();
    mgr.spawn_templates.insert(24, template(24));

    console(&mut mgr, None, ".dummy friendly 24").await;

    let id = mgr.lab_dummies_of(CALLER)[0];
    let d = mgr.get_entity(id).unwrap();
    assert_eq!(d.aggro.override_level, Some(MobAggression::Friendly));
    assert_eq!(d.template_id, Some(24));
}

#[tokio::test]
async fn ab_l2_dummy_refuses_bad_input_visibly_and_spawns_nothing() {
    let mut mgr = dummy_world();
    let before = mgr.entity_count();
    for bad in [
        ".dummy angry",
        ".dummy hostile x",
        ".dummy hostile 999",
        ".dummy 34",
    ] {
        let out = lines(&console(&mut mgr, None, bad).await);
        assert_eq!(out.len(), 1, "{bad}: one visible line");
        assert!(out[0].starts_with(".dummy"), "{bad}: {out:?}");
    }
    assert_eq!(mgr.entity_count(), before);
}

#[tokio::test]
async fn ab_l2_dummy_caps_how_many_one_gm_may_have() {
    let mut mgr = dummy_world();
    for _ in 0..LAB_DUMMY_MAX_PER_OWNER {
        console(&mut mgr, None, ".dummy").await;
    }
    let out = lines(&console(&mut mgr, None, ".dummy").await);
    assert!(out[0].contains("already have"), "{out:?}");
    assert_eq!(mgr.lab_dummies_of(CALLER).len(), LAB_DUMMY_MAX_PER_OWNER);
}

/// Colo rule: `.dummy clear` removes the caller's dummies and leaves
/// another GM's standing.
#[tokio::test]
async fn ab_l2_dummy_clear_removes_only_the_callers_own() {
    let mut mgr = dummy_world();
    console(&mut mgr, None, ".dummy").await;
    console(&mut mgr, None, ".dummy").await;
    let theirs = mgr.lab_dummies_of(CALLER)[0];
    mgr.get_entity_mut(theirs)
        .unwrap()
        .extensions
        .get_mut::<LabDummy>()
        .unwrap()
        .owner_id = WITNESS;
    let logs = LogCapture::install();

    let out = lines(&console(&mut mgr, None, ".dummy clear").await);

    assert_eq!(
        out,
        vec!["dummy clear: 1 of your dummies removed".to_string()]
    );
    assert!(mgr.lab_dummies_of(CALLER).is_empty());
    assert_eq!(
        mgr.lab_dummies_of(WITNESS),
        vec![theirs],
        "not the caller's"
    );
    let row = logs
        .find_message(Level::INFO, "lab dummy despawned")
        .unwrap();
    assert!(row.has_field("reason", "cleared"));
}

#[tokio::test]
async fn ab_l2_dummy_expires_after_its_lifetime_and_tells_its_owner() {
    let mut mgr = dummy_world();
    console(&mut mgr, None, ".dummy").await;
    console(&mut mgr, None, ".dummy").await;
    let ids = mgr.lab_dummies_of(CALLER);
    mgr.get_entity_mut(ids[0])
        .unwrap()
        .extensions
        .get_mut::<LabDummy>()
        .unwrap()
        .expires_at = Instant::now() - Duration::from_secs(1);
    let (tx, mut rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    lab_dummy_tick(&tx, &mut mgr).await;

    assert_eq!(
        mgr.lab_dummies_of(CALLER),
        vec![ids[1]],
        "only the lapsed one"
    );
    assert!(mgr.get_entity(ids[0]).is_none());
    let msgs: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        lines(&msgs).contains(&format!(
            "dummy [{}] despawned: its 10 minutes are up",
            ids[0]
        )),
        "{:?}",
        lines(&msgs)
    );
    assert!(logs
        .find_message(Level::INFO, "lab dummy despawned")
        .unwrap()
        .has_field("reason", "expired"));
}

#[tokio::test]
async fn ab_l2_dummy_goes_with_its_owners_logout() {
    let mut mgr = dummy_world();
    console(&mut mgr, None, ".dummy").await;
    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();

    despawn_lab_dummies_of(CALLER, &tx, &mut mgr).await;

    assert!(mgr.lab_dummies_of(CALLER).is_empty());
    assert!(logs
        .find_message(Level::INFO, "lab dummy despawned")
        .unwrap()
        .has_field("reason", "owner_logout"));
}
