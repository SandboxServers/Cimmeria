//! UO-05: the Command Center's cast and placements (world 80). Spawns
//! 63800-63803 place the cast and templates 63800-63803 are clones of the stock
//! rows 51, 59, 30 and 64 (F7, D-UO5). The offer maps of dialog set 63800
//! (D-UO6) are checked here; nothing binds them until UO-07..09.

use std::f32::consts::TAU;

use crate::test_support::require_db_or_skip;

/// Pad 17 (x, z), the Command Center's shipped pad.
const PAD_17: (f32, f32) = (6.16, 10.08);
/// Respawner 26 (x, z).
const RESPAWNER_26: (f32, f32) = (10.0, 9.987);

fn planar_distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Each clone copies its stock row's behaviour: same body, components, name
/// and speaker, and no interaction (the prompt comes from the bound map).
#[tokio::test]
async fn live_db_the_cast_are_clones_of_the_stock_four() {
    let pool = require_db_or_skip!();
    for (clone, stock) in [
        (63_800_i32, 51_i32),
        (63_801, 59),
        (63_802, 30),
        (63_803, 64),
    ] {
        let (matches,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM resources.entity_templates c \
             JOIN resources.entity_templates s ON s.template_id = $2 \
             WHERE c.template_id = $1 AND c.body_set = s.body_set \
               AND c.components = s.components AND c.name_id = s.name_id \
               AND c.speaker_id IS NOT DISTINCT FROM s.speaker_id \
               AND c.interaction_type = 0 \
               AND c.static_interaction_sets = s.static_interaction_sets \
               AND c.template_name LIKE 'Union Omega - %'",
        )
        .bind(clone)
        .bind(stock)
        .fetch_one(&pool)
        .await
        .expect("clone query");
        assert_eq!(
            matches, 1,
            "template {clone} is not a clone of stock template {stock}"
        );
    }
}

/// The four cast stand in world 80, one spawn per clone, clear of pad 17 and
/// respawner 26. Mesh and spacing are UO-02's `live_db_every_omega_spawn_*`.
#[tokio::test]
async fn live_db_the_cast_stand_in_the_command_center() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, i32, f32, f32, f32, bool)> = sqlx::query_as(
        "SELECT spawn_id, world_id, template_id, x, z, heading, is_stationary \
         FROM resources.spawnlist WHERE spawn_id BETWEEN 63800 AND 63803 \
         ORDER BY spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("cast spawn query");
    assert_eq!(rows.len(), 4, "the cast has four spawns");
    for &(spawn, world, template, x, z, heading, stationary) in &rows {
        assert_eq!(world, 80, "spawn {spawn} is not in world 80");
        assert_eq!(template, spawn, "spawn {spawn} places template {template}");
        assert!(stationary, "spawn {spawn} is not stationary");
        assert!(
            (0.0..TAU).contains(&heading),
            "spawn {spawn} heading {heading} is outside [0, TAU)"
        );
        assert!(
            planar_distance((x, z), PAD_17) >= 3.0,
            "spawn {spawn} stands within 3.0 m of pad 17"
        );
        assert!(
            planar_distance((x, z), RESPAWNER_26) >= 1.5,
            "spawn {spawn} stands within 1.5 m of respawner 26"
        );
    }
    let (placed,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.spawnlist WHERE template_id BETWEEN 63800 AND 63803",
    )
    .fetch_one(&pool)
    .await
    .expect("cast placement count");
    assert_eq!(placed, 4, "each cast clone is placed exactly once");
}

/// The offer set holds the two shipped dialogs, each as a "!" delivery and a
/// "?" re-offer, all at level 1.
#[tokio::test]
async fn live_db_the_offer_maps_open_the_shipped_dialogs() {
    let pool = require_db_or_skip!();
    let maps: Vec<(i32, i32, i32, String, i64, i32)> = sqlx::query_as(
        "SELECT dialog_set_map_id, dialog_set_id, dialog_id, topic_text, \
                interaction_flags, min_level \
         FROM resources.dialog_set_maps \
         WHERE dialog_set_map_id BETWEEN 63800 AND 63803 ORDER BY 1",
    )
    .fetch_all(&pool)
    .await
    .expect("offer map query");
    let expected: [(i32, i32, i32, &str, i64, i32); 4] = [
        (63_800, 63_800, 4641, "The SGU", 16_777_216, 1),
        (63_801, 63_800, 5025, "The SGU", 16_777_216, 1),
        (63_802, 63_800, 4641, "The SGU", 8_388_608, 1),
        (63_803, 63_800, 5025, "The SGU", 8_388_608, 1),
    ];
    assert_eq!(maps.len(), expected.len(), "the offer set has four maps");
    for (got, want) in maps.iter().zip(expected) {
        assert_eq!(
            (got.0, got.1, got.2, got.3.as_str(), got.4, got.5),
            want,
            "offer map {} differs from its shipped row",
            got.0
        );
    }
    let (name,): (Option<String>,) =
        sqlx::query_as("SELECT name FROM resources.dialog_sets WHERE dialog_set_id = 63800")
            .fetch_one(&pool)
            .await
            .expect("offer set query");
    assert_eq!(name.as_deref(), Some("Omega Command Center offers"));
}
