//! UB-40 production seed guards for the Beta E1 life NPCs: the SGC personnel
//! packing up the base (template 2308, spawns 23160-23168) and the Nine Peaks
//! camp's non-combatant figures (template 2309, spawns 23170-23172), all on
//! world 23. The positions are the sealed `LIFE_*` anchors of the placement
//! seal (UB-00b, with its addenda). The templates are not interactive and no
//! chain or bind names them, so a click on one does nothing.

use super::nav::{mesh, standable_and_connected, standable_and_walkable, ARRIVAL};
use super::*;

/// The Nine Peaks ring's SW end (`SW_RING` in the seal; spawn 23115's point).
const SW_RING: [f32; 3] = [-791.600, -51.430, -92.600];

/// Spawn id, template, tag and sealed anchor of each life spawn (`LIFE_*` in
/// placement-seal.md, at the snapped y). Template 2309 rows are the camp.
const LIFE: [(i32, i32, &str, [f32; 3]); 12] = [
    (
        23160,
        2308,
        "BetaE1_Life_Gate_1",
        [626.000, -60.830, 399.000],
    ),
    (
        23161,
        2308,
        "BetaE1_Life_Gate_2",
        [642.500, -60.830, 374.500],
    ),
    (
        23162,
        2308,
        "BetaE1_Life_Cmd_1",
        [1000.000, 13.570, 500.000],
    ),
    (
        23163,
        2308,
        "BetaE1_Life_Cmd_2",
        [1000.000, 13.570, 497.000],
    ),
    (23164, 2308, "BetaE1_Life_Hut_1", [1_033.7, 7.370, 361.349]),
    (23165, 2308, "BetaE1_Life_Hut_2", [1_033.7, 7.370, 424.563]),
    (23166, 2308, "BetaE1_Life_QM_1", [1002.433, 13.440, 281.388]),
    (23167, 2308, "BetaE1_Life_QM_2", [999.113, 13.570, 273.732]),
    (23168, 2308, "BetaE1_Life_Sci_1", [622.600, 20.640, 777.000]),
    (
        23170,
        2309,
        "BetaE1_Life_Camp_1",
        [-557.100, 6.770, 145.500],
    ),
    (
        23171,
        2309,
        "BetaE1_Life_Camp_2",
        [-562.100, 6.770, 150.500],
    ),
    (
        23172,
        2309,
        "BetaE1_Life_Camp_3",
        [-555.100, 6.770, 142.500],
    ),
];

/// Hop to the Science front. A direct leg from `ARRIVAL` to `LIFE_SCI_1` is
/// `Partial` and ends 65 m short (the corridor cap, placement seal UB-00b-3);
/// `ARRIVAL -> S1 -> anchor` is `Ok` on both legs.
const SCIENCE_HOP: [f32; 3] = [675.400, 15.770, 806.600];

/// The authored hops from `ARRIVAL` to a life spawn; empty means one direct
/// leg (every other main-valley row, probed Ok from ARRIVAL).
fn hops(spawn_id: i32) -> &'static [[f32; 3]] {
    match spawn_id {
        23168 => &[SCIENCE_HOP],
        _ => &[],
    }
}

/// Region 23031, the Asgard fire site (`asgard.rs`): centre and radius.
const FIRE_SITE_CENTRE: [f32; 3] = [1_022.3, 14.50, 271.2];
const FIRE_SITE_RADIUS: f32 = 12.0;

/// Clearance the quartermaster rows keep outside the fire site, in metres.
const FIRE_CLEARANCE: f32 = 4.0;

/// Text of a seed file under `db/resources/`, for the footer guard below.
fn seed_text(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../db/resources")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Revert: move one row 5 m into a wall, or delete a row, and this fails
/// naming the spawn. The main-valley rows walk from `ARRIVAL` in legs; the camp
/// rows must reach `SW_RING` (nav 70 is cut off from `ARRIVAL`, seal Q1).
#[tokio::test]
async fn live_db_beta_life_spawns_sit_on_their_sealed_anchors() {
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 23160 AND 23172",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-40 spawn count");
    assert_eq!(count, 12);

    for (spawn_id, template_id, tag, anchor) in LIFE {
        let row: (i32, i32, String, f32, f32, f32, bool, Option<i32>) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("life spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.6, row.7),
            (23, template_id, tag, true, Some(30)),
            "spawn {spawn_id}"
        );
        let actual = [row.3, row.4, row.5];
        for axis in 0..3 {
            assert!(
                (actual[axis] - anchor[axis]).abs() <= 0.05,
                "spawn {spawn_id} moved: update the placement seal"
            );
        }
        if template_id == 2309 {
            standable_and_connected(&exterior, actual, SW_RING, tag);
        } else {
            standable_and_walkable(&exterior, actual, ARRIVAL, hops(spawn_id), tag);
        }
    }
}

/// Revert: set 2308's or 2309's `interaction_type` to 1073741824, or give
/// either a `speaker_id`, and this fails naming the template.
#[tokio::test]
async fn live_db_beta_life_templates_are_not_interactive() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i64, i64, Option<i32>, Option<i32>)> = sqlx::query_as(
        "SELECT template_id, interaction_type, flags, speaker_id, name_id \
         FROM resources.entity_templates WHERE template_id IN (2308, 2309) \
         ORDER BY template_id",
    )
    .fetch_all(&pool)
    .await
    .expect("UB-40 templates");
    assert_eq!(
        rows,
        vec![
            (2308, 0, 0, None, Some(7450)),
            (2309, 0, 0, None, Some(7409))
        ],
        "life templates must be background figures with no dialog"
    );
}

/// Revert: add a scratch chain with `interact_tag 'BetaE1_Life_Gate_1'`, or a
/// `add_dialog_set` with `slot` 2308, and this fails.
#[tokio::test]
async fn no_chain_or_bind_names_a_life_npc() {
    let pool = require_db_or_skip!();
    let (triggers,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.content_triggers \
         WHERE left(event_key, 12) = 'BetaE1_Life_'",
    )
    .fetch_one(&pool)
    .await
    .expect("trigger keys");
    assert_eq!(triggers, 0, "a chain trigger names a life NPC");

    let (binds,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.content_actions \
         WHERE action_type = 'add_dialog_set' AND params->>'slot' IN ('2308', '2309')",
    )
    .fetch_one(&pool)
    .await
    .expect("dialog binds");
    assert_eq!(binds, 0, "a dialog set binds slot 2308 or 2309");
}

/// Revert: put `BetaE1_Life_QM_1` on the fire centre, and this fails.
#[tokio::test]
async fn life_npcs_stay_clear_of_the_fire_site() {
    let pool = require_db_or_skip!();
    let rows: Vec<(String, f32, f32)> = sqlx::query_as(
        "SELECT tag::text, x, z FROM resources.spawnlist \
         WHERE spawn_id BETWEEN 23160 AND 23172",
    )
    .fetch_all(&pool)
    .await
    .expect("life rows");
    let qm: Vec<_> = rows
        .iter()
        .filter(|(tag, _, _)| tag.starts_with("BetaE1_Life_QM_"))
        .collect();
    assert_eq!(qm.len(), 2, "both quartermaster rows must exist");
    for (tag, x, z) in qm {
        let dx = x - FIRE_SITE_CENTRE[0];
        let dz = z - FIRE_SITE_CENTRE[2];
        let gap = (dx * dx + dz * dz).sqrt() - FIRE_SITE_RADIUS;
        assert!(
            gap >= FIRE_CLEARANCE,
            "{tag} is {gap:.2} m outside the fire site, needs {FIRE_CLEARANCE} m"
        );
    }
}

/// Revert: give a non-life row one of the life tags, and this fails.
#[tokio::test]
async fn life_tags_are_unique_across_worlds() {
    let pool = require_db_or_skip!();
    let tags: Vec<String> = LIFE.iter().map(|row| row.2.to_string()).collect();
    let (rows,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM resources.spawnlist WHERE tag::text = ANY($1)")
            .bind(&tags)
            .fetch_one(&pool)
            .await
            .expect("tag count");
    assert_eq!(
        rows,
        LIFE.len() as i64,
        "a life tag is used by another spawnlist row"
    );
}

/// The footer guard: the life spawn file must carry its own sequence footer.
/// The DB-level sequence checks cannot tell whose footer ran, because later
/// Beta files and the Debug Area footers re-set the same sequence.
///
/// Revert: delete the `setval` line from `spawnlist_beta_e1_life.sql`, and this
/// fails naming the file.
#[test]
fn beta_life_seed_file_carries_its_own_sequence_footer() {
    let footer = "SELECT pg_catalog.setval('spawnlist_spawn_id_seq', GREATEST((SELECT MAX(spawn_id) FROM spawnlist), (SELECT last_value FROM spawnlist_spawn_id_seq), 23199), true);";
    let file = "Worlds/Seed/spawnlist_beta_e1_life.sql";
    assert!(
        seed_text(file).lines().any(|line| line.trim() == footer),
        "{file} is missing its sequence footer: {footer}"
    );
}
