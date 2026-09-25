//! Live-DB guards for `entity_templates.leash_distance` (NA12, D-NA09).
//!
//! The column is nullable: NULL means "use the server default"
//! (`combat::LEASH_DISTANCE`), and the loaders deliberately do not COALESCE
//! it, so a template that sets a radius must reach the spawned NPC as
//! `leash.distance_override = Some(radius)`, and one that does not must reach
//! it as `None`. Both loaders (`load_spawns_from_db` for seeded spawns and
//! `load_spawn_templates` for content and GM spawns) are covered.

use crate::cell::spawner::{load_spawn_templates, load_spawns_from_db};
use crate::test_support::require_db_or_skip;

/// Sentinel template id, unique to this file. Fits in `i32`.
const SENTINEL_TEMPLATE_ID: i32 = 0x7000_1212;

async fn delete_sentinel(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
        .bind(SENTINEL_TEMPLATE_ID)
        .execute(pool)
        .await
        .expect("sentinel template cleanup must succeed");
}

async fn insert_sentinel(pool: &sqlx::PgPool, leash_distance: Option<f32>) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO resources.entity_templates \
             (template_id, template_name, class, body_set, flags, interaction_type, \
              level, alignment, faction, static_interaction_sets, has_dynamic_properties, \
              leash_distance) \
         VALUES ($1, 'NA12 Leash Probe', 'mob', 'GLB_Components.WorldObject_Small', \
                 0, 0, 5, 0, 10, ARRAY[]::integer[], false, $2)",
    )
    .bind(SENTINEL_TEMPLATE_ID)
    .bind(leash_distance)
    .execute(pool)
    .await
    .map(|_| ())
}

/// A template with `leash_distance = 80` loads as `Some(80.0)` and lands on
/// the spawned NPC's leash override. Fails if the column is dropped from the
/// shared template SELECT, mis-typed, or not copied onto the entity.
#[tokio::test]
async fn template_leash_distance_reaches_the_spawned_npc() {
    let pool = require_db_or_skip!();
    delete_sentinel(&pool).await;
    insert_sentinel(&pool, Some(80.0))
        .await
        .expect("sentinel template insert must succeed");

    let loaded = load_spawn_templates(&pool).await;
    delete_sentinel(&pool).await;

    let templates = loaded.expect("load_spawn_templates must succeed");
    let mut record = templates
        .get(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template must surface from the loader")
        .clone();
    assert_eq!(record.leash_distance, Some(80.0));

    let mut mgr = crate::test_support::make_space_manager();
    record.world_name = "Agnos".to_string();
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record)
        .expect("sentinel NPC must spawn");
    assert_eq!(
        mgr.get_entity(npc_id).unwrap().leash.distance_override,
        Some(80.0),
        "the template's radius must reach the live NPC"
    );
}

/// The DB rejects a non-positive radius; NULL is the "use the default"
/// spelling.
#[tokio::test]
async fn leash_distance_check_rejects_zero() {
    let pool = require_db_or_skip!();
    delete_sentinel(&pool).await;
    let zero = insert_sentinel(&pool, Some(0.0)).await;
    delete_sentinel(&pool).await;
    let err = zero.expect_err("leash_distance = 0 must violate the CHECK");
    assert!(
        err.to_string()
            .contains("entity_templates_leash_distance_positive"),
        "wrong error: {err}"
    );
}

/// The shipped seed sets no radius, so every seeded spawn loads `None` and
/// keeps the server default. Pins that the seeded-spawn loader reads the
/// column without COALESCEing it to a number (which would hide "the
/// template said nothing" from the leash logs).
#[tokio::test]
async fn seeded_spawns_load_the_default_leash() {
    let pool = require_db_or_skip!();
    let spawns = load_spawns_from_db(&pool)
        .await
        .expect("load_spawns_from_db must succeed");
    assert!(!spawns.is_empty(), "the seed has spawns");
    let set: Vec<_> = spawns
        .iter()
        .filter(|s| s.leash_distance.is_some())
        .map(|s| (s.spawn_id, s.template_id, s.leash_distance))
        .collect();
    assert!(
        set.is_empty(),
        "no seeded template sets leash_distance yet; update this test when one does: {set:?}"
    );
}
