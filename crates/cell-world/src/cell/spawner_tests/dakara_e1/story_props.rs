//! DK-41 production seed guards: the story props of missions 1649-1653
//! (spawns 8016-8021) and the clones 462 (Ring Control) and 463 (Power Supply).

use super::*;

/// The plaza point every connected Dakara_E1 spawn must reach on foot.
const PLAZA: [f32; 3] = [100.0, -17.4, 230.0];

/// `(spawn id, world, template id, tag, position, heading, plaza-connected)`.
/// A-05 sits on the tent table and B-06 on its pedestal, both off-mesh,
/// so neither gets a mesh check. A-12 and A-13 keep the ledger's placeholder
/// heading 0.0.
const STORY_PROPS: [(i32, i32, i32, &str, [f32; 3], f32, bool); 6] = [
    (
        8016,
        61,
        453,
        "Dakara_E1_CommandTerminal",
        [137.1, -21.38, 274.2],
        4.1891,
        false,
    ),
    (
        8017,
        62,
        454,
        "Dakara_E1_StoryRm_MohkatanTerminal",
        [62.5, 0.02, 40.5],
        1.8158,
        false,
    ),
    (
        8018,
        61,
        455,
        "Dakara_E1_RingControl_XMinus",
        [-139.33, -16.17, 48.95],
        0.0,
        true,
    ),
    (
        8019,
        61,
        462,
        "Dakara_E1_RingControl_XPlus",
        [358.03, -16.02, 102.07],
        0.0,
        true,
    ),
    (
        8020,
        61,
        456,
        "Dakara_E1_PowerSupply1",
        [56.5, -12.36, -73.7],
        1.4724,
        true,
    ),
    (
        8021,
        61,
        463,
        "Dakara_E1_PowerSupply2",
        [202.2, -12.36, -59.4],
        4.6152,
        true,
    ),
];

/// `(source template, clone template)`: each clone must wear its source's look.
const CLONES: [(i32, i32); 2] = [(455, 462), (456, 463)];

#[tokio::test]
async fn live_db_dakara_story_props_match_the_placement_ledger() {
    let pool = require_db_or_skip!();
    let exterior = travel::mesh("dakara_e1.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 8016 AND 8021",
    )
    .fetch_one(&pool)
    .await
    .expect("DK-41 spawn count");
    assert_eq!(count, 6);

    for (spawn_id, world, template_id, tag, expected, heading, connected) in STORY_PROPS {
        let row: (i32, i32, String, bool, Option<i32>, f32, f32, f32, f32) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, is_stationary, respawn_secs, x, y, z, heading \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("DK-41 spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.3, row.4),
            (world, template_id, tag, true, Some(30))
        );
        let actual = [row.5, row.6, row.7];
        assert_eq!(
            actual, expected,
            "spawn {spawn_id} moved: update the ledger"
        );
        assert_eq!(row.8, heading, "spawn {spawn_id} turned: update the ledger");
        if connected {
            travel::standable_and_connected(&exterior, actual, PLAZA, tag);
        }
    }
}

#[tokio::test]
async fn live_db_prop_clones_wear_their_sources_look() {
    let pool = require_db_or_skip!();

    type Look = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    for (source, clone) in CLONES {
        let mut looks: Vec<Look> = Vec::new();
        for template_id in [source, clone] {
            let look: Look = sqlx::query_as(
                "SELECT static_mesh::text, body_set::text, name_id::text, \
                        flags::text, interaction_type::text, class::text \
                 FROM resources.entity_templates WHERE template_id = $1",
            )
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .unwrap_or_else(|e| panic!("DK-41 template {template_id}: {e}"));
            assert_eq!(
                look.4.as_deref(),
                Some("0"),
                "template {template_id} must keep interaction_type 0: its bind is per player"
            );

            let spawns: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM resources.spawnlist WHERE template_id = $1",
            )
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .expect("DK-41 template spawn count");
            assert_eq!(
                spawns, 1,
                "template {template_id} must be the template of exactly one story-prop spawn"
            );
            looks.push(look);
        }
        assert_eq!(
            looks[0], looks[1],
            "clone {clone} must wear {source}'s look and name"
        );
    }
}
