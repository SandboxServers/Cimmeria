//! UO-14: Omega's ambient population (templates 63860-63861, spawns
//! 63880-63959; CIMMERIA_AUTHORING, D-UO12). The guards keep it inert,
//! inside the D-UO26 cap, and clear of every service and travel point.
//! The nav check is UO-02's `live_db_every_omega_spawn_stands_on_its_mesh`.

use crate::test_support::require_db_or_skip;

const AMBIENT_TEMPLATES: std::ops::RangeInclusive<i32> = 63_860..=63_899;
const AMBIENT_SPAWNS: std::ops::RangeInclusive<i32> = 63_880..=63_959;
/// Omega's id block. A spawn in it that is not ambient is a cast, service or
/// travel NPC, whatever its interaction_type (the cast templates use 0).
const OMEGA_BLOCK: std::ops::RangeInclusive<i32> = 63_800..=63_999;
/// D-UO26: the ambient cap. Change only with an owner ruling.
const AMBIENT_CAP: usize = 24;
const HOSTILE_FACTION: i32 = 10;
const ALLOWED_MONIKERS: [i32; 2] = [8_360, 26_723];
/// (label, world, x, z): points an ambient NPC keeps 3 m from.
const FIXED_POINTS: [(&str, i32, f32, f32); 5] = [
    ("respawner 180 (hub.respawner)", 18, 34.70, -138.00),
    ("the C4 arrival point (hub.arrival)", 18, 34.70, -142.00),
    ("Modi (spawn 78)", 18, 39.544, -129.92),
    ("respawner 26", 80, 10.0, 9.987),
    ("pad 17", 80, 6.16, 10.08),
];
const MIN_SERVICE_CLEARANCE: f32 = 3.0;
const MIN_AMBIENT_SPACING: f32 = 2.5;

fn xz(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    ((ax - bx).powi(2) + (az - bz).powi(2)).sqrt()
}

/// The two ambient templates are inert: a shipped moniker, a non-hostile
/// faction, no interaction, no sets, no item or ability list, and no chain
/// that keys on or acts on an ambient NPC.
#[tokio::test]
async fn live_db_omega_ambient_templates_are_inert() {
    let pool = require_db_or_skip!();
    #[allow(clippy::type_complexity)]
    let rows: Vec<(
        i32,
        String,
        Option<i32>,
        i64,
        Vec<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
        Option<i32>,
    )> = sqlx::query_as(
        "SELECT template_id, class, faction, interaction_type, static_interaction_sets, interaction_set_id, buy_item_list, sell_item_list, repair_item_list, recharge_item_list, trainer_ability_list_id, ability_set_id, loot_table_id, speaker_id, name_id FROM resources.entity_templates WHERE template_id BETWEEN 63860 AND 63899 ORDER BY template_id",
    )
    .fetch_all(&pool)
    .await
    .expect("entity_templates query");
    let ids: Vec<i32> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [63_860, 63_861], "exactly the two ambient templates");
    for r in &rows {
        let id = r.0;
        assert_eq!(r.1, "mob", "template {id} class");
        assert_ne!(r.2, Some(HOSTILE_FACTION), "template {id} is hostile");
        assert_eq!(r.3, 0, "template {id} interaction_type");
        assert!(r.4.is_empty(), "template {id} has static interaction sets");
        for (label, column) in [
            ("interaction_set_id", r.5),
            ("buy_item_list", r.6),
            ("sell_item_list", r.7),
            ("repair_item_list", r.8),
            ("recharge_item_list", r.9),
            ("trainer_ability_list_id", r.10),
            ("ability_set_id", r.11),
            ("loot_table_id", r.12),
            ("speaker_id", r.13),
        ] {
            assert!(column.is_none(), "template {id} sets {label}");
        }
        assert!(
            r.14.is_some_and(|n| ALLOWED_MONIKERS.contains(&n)),
            "template {id} name_id {:?} is not a shipped ambient moniker",
            r.14
        );
    }
    let (actions,): (i64,) = sqlx::query_as(
        r#"SELECT count(*) FROM resources.content_actions WHERE params::text ~ '"slot":\s*638[6-9][0-9]'"#,
    )
    .fetch_one(&pool)
    .await
    .expect("content_actions query");
    assert_eq!(actions, 0, "a chain action names an ambient slot");
    let (triggers,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.content_triggers WHERE event_key = 'UnionOmega_Ambient'",
    )
    .fetch_one(&pool)
    .await
    .expect("content_triggers query");
    assert_eq!(triggers, 0, "a chain trigger keys on an ambient NPC");
}

/// The ambient population is one bounded, stationary, tagged set inside the
/// block, within the D-UO26 cap, and no other template takes an ambient id.
#[tokio::test]
async fn live_db_omega_ambient_stays_inside_its_cap() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, Option<String>, bool)> = sqlx::query_as(
        "SELECT s.spawn_id, s.world_id, s.tag, s.is_stationary FROM resources.spawnlist s WHERE s.template_id BETWEEN 63860 AND 63899 ORDER BY s.spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist query");
    assert!(
        (1..=AMBIENT_CAP).contains(&rows.len()),
        "{} ambient spawns, cap {AMBIENT_CAP}",
        rows.len()
    );
    for (id, world, tag, stationary) in &rows {
        assert!(
            AMBIENT_SPAWNS.contains(id),
            "spawn {id} is outside 63880-63959"
        );
        assert!(
            *world == 18 || *world == 80,
            "spawn {id} stands in world {world}"
        );
        assert!(
            tag.as_deref()
                .is_some_and(|t| t.starts_with("UnionOmega_Ambient_")),
            "spawn {id} tag {tag:?}"
        );
        assert!(stationary, "spawn {id} is not stationary");
    }
    let block: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT spawn_id, template_id FROM resources.spawnlist WHERE spawn_id BETWEEN 63880 AND 63959 ORDER BY spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawn block query");
    let stray: Vec<(i32, i32)> = block
        .into_iter()
        .filter(|(_, template)| !AMBIENT_TEMPLATES.contains(template))
        .collect();
    assert!(
        stray.is_empty(),
        "ambient spawn ids on other templates: {stray:?}"
    );
}

/// Every ambient NPC keeps 3 m (x/z) from each fixed point and each service
/// or travel spawn in its world, and 2.5 m from every other ambient NPC.
#[tokio::test]
async fn live_db_omega_ambient_keeps_clear_of_services_and_travel() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, f32, f32, i64)> = sqlx::query_as(
        "SELECT s.spawn_id, s.world_id, s.x, s.z, t.interaction_type FROM resources.spawnlist s JOIN resources.entity_templates t ON t.template_id = s.template_id WHERE s.world_id IN (18, 80)",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist query");
    let ambient: Vec<&(i32, i32, f32, f32, i64)> = rows
        .iter()
        .filter(|r| AMBIENT_SPAWNS.contains(&r.0))
        .collect();
    assert!(!ambient.is_empty(), "no ambient spawns to check");
    for a in &ambient {
        let (id, world, x, z, _) = **a;
        for (label, fw, fx, fz) in FIXED_POINTS {
            if fw == world {
                let d = xz(x, z, fx, fz);
                assert!(
                    d >= MIN_SERVICE_CLEARANCE,
                    "ambient {id} stands {d:.2} m from {label}"
                );
            }
        }
        let is_service = |o: &&(i32, i32, f32, f32, i64)| {
            o.1 == world
                && (o.4 != 0 || (OMEGA_BLOCK.contains(&o.0) && !AMBIENT_SPAWNS.contains(&o.0)))
        };
        for o in rows.iter().filter(is_service) {
            let d = xz(x, z, o.2, o.3);
            assert!(
                d >= MIN_SERVICE_CLEARANCE,
                "ambient {id} stands {d:.2} m from service or travel spawn {}",
                o.0
            );
        }
        for o in ambient.iter().filter(|o| o.0 != id && o.1 == world) {
            let d = xz(x, z, o.2, o.3);
            assert!(
                d >= MIN_AMBIENT_SPACING,
                "ambient {id} stands {d:.2} m from ambient {}",
                o.0
            );
        }
    }
}
