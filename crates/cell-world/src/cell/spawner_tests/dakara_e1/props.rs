//! DK-09 production seed guards: the SG-18 remains (spawns 8008-8010) and
//! mission 1646's five drop locations (spawns 8011-8015), and the
//! step-4902 search regions around the remains (point sets 2128-2130).

use super::*;

/// The plaza point every connected Dakara_E1 spawn must reach on foot.
const PLAZA: [f32; 3] = [100.0, -17.4, 230.0];

/// `(spawn id, template id, tag, position, plaza-connected)`. The remains sit
/// off the plaza's component (D-DK11); the drop locations are on it (D-DK13).
const PROPS: [(i32, i32, &str, [f32; 3], bool); 8] = [
    (
        8008,
        450,
        "Dakara_E1_SG18_Louis",
        [483.9, -9.1, 460.6],
        false,
    ),
    (
        8009,
        451,
        "Dakara_E1_SG18_Nguyn",
        [503.9, -9.1, 462.0],
        false,
    ),
    (
        8010,
        452,
        "Dakara_E1_SG18_Waters",
        [440.1, -9.1, 497.8],
        false,
    ),
    (8011, 449, "Dakara_E1_Drop1", [-110.0, -16.61, 253.0], true),
    (8012, 458, "Dakara_E1_Drop2", [318.0, -17.82, 309.0], true),
    (8013, 459, "Dakara_E1_Drop3", [62.0, -16.56, 413.0], true),
    (8014, 460, "Dakara_E1_Drop4", [26.0, -17.9, 109.0], true),
    (8015, 461, "Dakara_E1_Drop5", [366.0, -16.26, -31.0], true),
];

/// `(set id, region name, remains spawn)`: each step-4902 region is named for
/// the remains it is centred on (D-DK18).
const REGIONS: [(i32, &str, i32); 3] = [
    (2128, "Dakara_E1.SG18Louis", 8008),
    (2129, "Dakara_E1.SG18Nguyn", 8009),
    (2130, "Dakara_E1.SG18Waters", 8010),
];

#[tokio::test]
async fn live_db_dakara_sg18_remains_and_drop_locations_match_the_placement_ledger() {
    let pool = require_db_or_skip!();
    let exterior = travel::mesh("dakara_e1.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 8008 AND 8015",
    )
    .fetch_one(&pool)
    .await
    .expect("DK-09 spawn count");
    assert_eq!(count, 8);

    for (spawn_id, template_id, tag, expected, connected) in PROPS {
        let row: (i32, i32, String, bool, Option<i32>, f32, f32, f32) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, is_stationary, respawn_secs, x, y, z \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("DK-09 spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.3, row.4),
            (61, template_id, tag, true, Some(30))
        );
        let actual = [row.5, row.6, row.7];
        assert_eq!(
            actual, expected,
            "spawn {spawn_id} moved: update the ledger"
        );
        if connected {
            travel::standable_and_connected(&exterior, actual, PLAZA, tag);
        } else {
            travel::standable(&exterior, actual, tag);
        }
    }
}

#[tokio::test]
async fn live_db_each_drop_location_has_its_own_template_wearing_449s_look() {
    let pool = require_db_or_skip!();

    type Look = (
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    let mut looks: Vec<Look> = Vec::new();
    for template_id in [449, 458, 459, 460, 461] {
        let look: Look = sqlx::query_as(
            "SELECT template_id, static_mesh::text, body_set::text, name_id::text, \
                    flags::text, interaction_type::text, class::text \
             FROM resources.entity_templates WHERE template_id = $1",
        )
        .bind(template_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("drop template {template_id}: {e}"));
        looks.push(look);

        let spawns: i64 =
            sqlx::query_scalar("SELECT count(*) FROM resources.spawnlist WHERE template_id = $1")
                .bind(template_id)
                .fetch_one(&pool)
                .await
                .expect("drop spawn count");
        assert_eq!(
            spawns, 1,
            "template {template_id} must be the template of exactly one drop spawn"
        );
    }

    let base = looks[0].clone();
    for look in &looks {
        assert_eq!(
            (&look.1, &look.2, &look.3, &look.4, &look.5, &look.6),
            (&base.1, &base.2, &base.3, &base.4, &base.5, &base.6),
            "template {} must wear 449's look and name",
            look.0
        );
    }
}

#[tokio::test]
async fn live_db_each_sg18_search_region_is_centred_on_its_remains() {
    let pool = require_db_or_skip!();

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.point_sets WHERE set_id BETWEEN 2128 AND 2130",
    )
    .fetch_one(&pool)
    .await
    .expect("DK-09 region count");
    assert_eq!(count, 3);

    for (set_id, name, remains) in REGIONS {
        let region: (
            String,
            String,
            i32,
            Option<f32>,
            Option<f32>,
            String,
            i32,
            f32,
            f32,
            f32,
        ) = sqlx::query_as(
            "SELECT s.name::text, s.type::text, s.world_id, s.radius, s.height, \
                    s.shape::text, s.flags, p.x, p.y, p.z \
             FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) \
             WHERE s.set_id = $1",
        )
        .bind(set_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("DK-09 region {set_id}: {e}"));
        assert_eq!(
            (
                region.0.as_str(),
                region.1.as_str(),
                region.2,
                region.5.as_str(),
                region.6
            ),
            (name, "AreaSet", 61, "Cylinder", 1)
        );
        assert_eq!((region.3, region.4), (Some(8.0), Some(10.0)));

        let spawn: (f32, f32, f32) =
            sqlx::query_as("SELECT x, y, z FROM resources.spawnlist WHERE spawn_id = $1")
                .bind(remains)
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("remains spawn {remains}: {e}"));
        let moved =
            format!("region {set_id} no longer centred on spawn {remains}: move both together");
        assert_eq!((region.7, region.9), (spawn.0, spawn.2), "{moved}");
        assert!((region.8 - (spawn.1 - 3.0)).abs() < 0.01, "{moved}");
    }
}
