//! Live-DB guards for the NA13 columns: `entity_templates.aggro_radius` and
//! `spawnlist.aggression_override` (D-NA01, D-NA01a, D-NA09).
//!
//! Both are nullable and deliberately not COALESCEd: NULL radius means the
//! server default (`combat::DEFAULT_AGGRO_RADIUS`), and a NULL override
//! means the faction reaction decides. The seed pins NEUTRAL on the two
//! chain-armed Cellblock spawns and nowhere else.

use cimmeria_entity::cell_entity::MobAggression;

use crate::cell::spawner::{load_spawn_templates, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// Sentinel ids, unique to this file. Fit in `i32`.
const SENTINEL_TEMPLATE_ID: i32 = 0x7000_1313;
const SENTINEL_SPAWN_ID: i32 = 0x7000_1313;

async fn cleanup(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM resources.spawnlist WHERE spawn_id = $1")
        .bind(SENTINEL_SPAWN_ID)
        .execute(pool)
        .await
        .expect("sentinel spawn cleanup must succeed");
    sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
        .bind(SENTINEL_TEMPLATE_ID)
        .execute(pool)
        .await
        .expect("sentinel template cleanup must succeed");
}

async fn insert_template(pool: &sqlx::PgPool, aggro_radius: Option<f32>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO resources.entity_templates \
             (template_id, template_name, class, body_set, flags, interaction_type, \
              level, alignment, faction, static_interaction_sets, has_dynamic_properties, \
              aggro_radius) \
         VALUES ($1, 'NA13 Aggro Probe', 'mob', 'GLB_Components.WorldObject_Small', \
                 0, 0, 5, 0, 10, ARRAY[]::integer[], false, $2)",
    )
    .bind(SENTINEL_TEMPLATE_ID)
    .bind(aggro_radius)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn insert_spawn(pool: &sqlx::PgPool, override_level: Option<i16>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO resources.spawnlist \
             (spawn_id, x, y, z, heading, world_id, template_id, tag, aggression_override) \
         VALUES ($1, 1, 2, 3, 0, 12, $2, 'NA13_Probe', $3)",
    )
    .bind(SENTINEL_SPAWN_ID)
    .bind(SENTINEL_TEMPLATE_ID)
    .bind(override_level)
    .execute(pool)
    .await
    .map(|_| ())
}

/// A template radius and a spawn override both load, and both reach the
/// spawned NPC. Fails if either column is dropped from its SELECT, mis-typed,
/// or not copied onto the entity.
#[tokio::test]
async fn aggro_columns_reach_the_spawned_npc() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_template(&pool, Some(30.0)).await.expect("template");
    insert_spawn(&pool, Some(1)).await.expect("spawn");

    let spawns = load_spawns_from_db(&pool).await;
    let templates = load_spawn_templates(&pool).await;
    cleanup(&pool).await;

    let mut record = spawns
        .expect("load_spawns_from_db")
        .into_iter()
        .find(|s| s.spawn_id == SENTINEL_SPAWN_ID)
        .expect("the sentinel spawn loads");
    assert_eq!(record.aggro_radius, Some(30.0));
    assert_eq!(record.aggression_override, Some(MobAggression::Hostile));

    let proto = templates
        .expect("load_spawn_templates")
        .remove(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template loads");
    assert_eq!(
        proto.aggro_radius,
        Some(30.0),
        "template loader reads the radius"
    );
    assert_eq!(
        proto.aggression_override, None,
        "the override is a placement property, never on a template"
    );

    let mut mgr = crate::test_support::make_space_manager();
    record.world_name = "Agnos".to_string();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record)
        .expect("sentinel NPC must spawn");
    let npc = mgr.get_entity(npc_id).unwrap();
    assert_eq!(npc.aggro.radius_override, Some(30.0));
    assert_eq!(npc.aggro.override_level, Some(MobAggression::Hostile));
}

/// The CHECKs reject a non-positive radius and an override outside 1-5.
#[tokio::test]
async fn aggro_column_checks_reject_bad_values() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    let zero = insert_template(&pool, Some(0.0)).await;
    cleanup(&pool).await;
    let err = zero.expect_err("aggro_radius = 0 must violate the CHECK");
    assert!(
        err.to_string()
            .contains("entity_templates_aggro_radius_positive"),
        "wrong error: {err}"
    );

    insert_template(&pool, None).await.expect("template");
    for bad in [0i16, 6] {
        let r = insert_spawn(&pool, Some(bad)).await;
        let err = r.expect_err("an override outside 1-5 must violate the CHECK");
        assert!(
            err.to_string()
                .contains("spawnlist_aggression_override_level"),
            "wrong error for {bad}: {err}"
        );
    }
    cleanup(&pool).await;
}

/// D-NA01a seed pin: spawns 20 and 10 carry NEUTRAL, and no other seeded
/// spawn carries an override. No seeded template sets a radius yet.
#[tokio::test]
async fn seed_overrides_only_the_chain_armed_spawns() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool).await.expect("load spawns");
    let mut overridden: Vec<_> = spawns
        .iter()
        .filter_map(|s| s.aggression_override.map(|l| (s.spawn_id, l)))
        .collect();
    overridden.sort_by_key(|(id, _)| *id);
    assert_eq!(
        overridden,
        vec![(10, MobAggression::Neutral), (20, MobAggression::Neutral)],
        "only ArmYourself_PrisonerRetrievalUnit (10) and ArmYourself_NIDGuard (20)"
    );
    let radii: Vec<_> = spawns
        .iter()
        .filter(|s| s.aggro_radius.is_some())
        .map(|s| (s.spawn_id, s.aggro_radius))
        .collect();
    assert!(
        radii.is_empty(),
        "no seeded template sets aggro_radius yet; update this test when one does: {radii:?}"
    );
}
