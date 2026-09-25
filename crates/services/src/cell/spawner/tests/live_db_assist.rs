//! Live-DB guards for the NA14 column `entity_templates.assist_radius`
//! (D-NA04, D-NA09). Nullable and deliberately not COALESCEd: NULL means the
//! server default (`combat::DEFAULT_ASSIST_RADIUS`, 10 u).

use crate::cell::spawner::{load_spawn_templates, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// Sentinel ids, unique to this file. Fit in `i32`.
const SENTINEL_TEMPLATE_ID: i32 = 0x7000_1414;
const SENTINEL_SPAWN_ID: i32 = 0x7000_1414;

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

async fn insert_template(pool: &sqlx::PgPool, assist_radius: Option<f32>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO resources.entity_templates \
             (template_id, template_name, class, body_set, flags, interaction_type, \
              level, alignment, faction, static_interaction_sets, has_dynamic_properties, \
              assist_radius) \
         VALUES ($1, 'NA14 Assist Probe', 'mob', 'GLB_Components.WorldObject_Small', \
                 0, 0, 5, 0, 10, ARRAY[]::integer[], false, $2)",
    )
    .bind(SENTINEL_TEMPLATE_ID)
    .bind(assist_radius)
    .execute(pool)
    .await
    .map(|_| ())
}

async fn insert_spawn(pool: &sqlx::PgPool) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO resources.spawnlist \
             (spawn_id, x, y, z, heading, world_id, template_id, tag) \
         VALUES ($1, 1, 2, 3, 0, 12, $2, 'NA14_Probe')",
    )
    .bind(SENTINEL_SPAWN_ID)
    .bind(SENTINEL_TEMPLATE_ID)
    .execute(pool)
    .await
    .map(|_| ())
}

/// A template's assist radius loads through both loaders and reaches the
/// spawned NPC; NULL stays `None` (the 10 u default). Fails if the column
/// is dropped from either SELECT, mis-typed, or not copied onto the entity.
#[tokio::test]
async fn assist_radius_reaches_the_spawned_npc() {
    let pool = require_db_or_skip!();
    cleanup(&pool).await;
    insert_template(&pool, Some(6.5)).await.expect("template");
    insert_spawn(&pool).await.expect("spawn");

    let spawns = load_spawns_from_db(&pool).await;
    let templates = load_spawn_templates(&pool).await;
    cleanup(&pool).await;

    let mut record = spawns
        .expect("load_spawns_from_db")
        .into_iter()
        .find(|s| s.spawn_id == SENTINEL_SPAWN_ID)
        .expect("the sentinel spawn loads");
    assert_eq!(record.assist_radius, Some(6.5));
    assert_eq!(
        record.aggro_radius, None,
        "the two radii are separate columns"
    );

    let proto = templates
        .expect("load_spawn_templates")
        .remove(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template loads");
    assert_eq!(proto.assist_radius, Some(6.5), "template loader reads it");

    let mut mgr = crate::test_support::make_space_manager();
    record.world_name = "Agnos".to_string();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record)
        .expect("sentinel NPC must spawn");
    let npc = mgr.get_entity(npc_id).unwrap();
    assert_eq!(npc.aggro.assist_radius_override, Some(6.5));
    assert_eq!(crate::cell::combat::assist_radius(npc), 6.5);

    // NULL: the default applies.
    insert_template(&pool, None).await.expect("template");
    let proto = load_spawn_templates(&pool)
        .await
        .expect("load_spawn_templates")
        .remove(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template loads");
    cleanup(&pool).await;
    assert_eq!(proto.assist_radius, None);
}

/// The CHECK rejects a non-positive radius.
#[tokio::test]
async fn assist_radius_check_rejects_non_positive_values() {
    let pool = require_db_or_skip!();
    for bad in [0.0f32, -3.0] {
        cleanup(&pool).await;
        let r = insert_template(&pool, Some(bad)).await;
        cleanup(&pool).await;
        let err = r.expect_err("assist_radius <= 0 must violate the CHECK");
        assert!(
            err.to_string()
                .contains("entity_templates_assist_radius_positive"),
            "wrong error for {bad}: {err}"
        );
    }
}

/// No seeded template sets an assist radius yet: every seeded NPC uses the
/// 10 u default. Update this test when a template tunes it.
#[tokio::test]
async fn no_seeded_template_sets_an_assist_radius_yet() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool).await.expect("load spawns");
    let radii: Vec<_> = spawns
        .iter()
        .filter(|s| s.assist_radius.is_some())
        .map(|s| (s.spawn_id, s.assist_radius))
        .collect();
    assert!(radii.is_empty(), "{radii:?}");
}
