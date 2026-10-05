//! Live-DB guards for Debug Area packet DA-03 (world 1300): the faction yard
//! (Z4), the AI behaviour slope (Z5) and the enemy gallery (Z7), templates
//! 1330-1369 and spawns 13200-13599 (`docs/content/debug-area.md`).
//!
//! The geometry (navmesh, occluder, aggro and assist reach) is guarded on the
//! real mesh by `cimmeria-cell`'s `service::tests::npc_ai::debug_area`. These
//! guards check what the loader hands the cell:
//!
//! * every DA-03 row is in world 1300 and inside its zone, and loads with the
//!   behaviour its station is for (patrol route, wander radius, leash, assist
//!   and aggro radii, factions, the NEUTRAL pins);
//! * the gallery holds every template hostile to players (the factions row 3
//!   of the client's reaction table marks HOSTILE, faction 10 today) exactly
//!   once, passive, so a hostile template added later fails here until it is
//!   placed in the gallery or added to [`GALLERY_EXCLUSIONS`] with its reason;
//! * the point-set footers leave both sequences past DA-03's block.
//!
//! Each was proven to fail with the DA-03 seed rows removed or a gallery row
//! dropped.
mod live_db {
    use std::collections::{BTreeMap, BTreeSet};

    use cimmeria_entity::cell_entity::MobAggression;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const WORLD: &str = "DebugArea";
    const WORLD_ID: i32 = 1300;
    const SPAWNS: std::ops::RangeInclusive<i32> = 13200..=13599;
    const TEMPLATES: std::ops::RangeInclusive<i32> = 1330..=1369;
    const HOSTILE_FACTION: i32 = 10;

    /// Hostile templates the gallery leaves out, and why. Read the PR that
    /// adds a hostile template before adding it here: the gallery exists so a
    /// tester can find every enemy in one place.
    const GALLERY_EXCLUSIONS: [(i32, &str); 2] = [
        (140, "NPC Child 1: a child, not an enemy"),
        (141, "NPC Child 2: a child, not an enemy"),
    ];

    /// The Debug Area's own station templates (DA-02..DA-04 blocks). They are
    /// placed at their stations, not in the gallery.
    const DEBUG_AREA_TEMPLATES: std::ops::RangeInclusive<i32> = 1300..=1399;

    /// The gallery respawn: short, so the line refills while a tester works
    /// along it.
    const MAX_GALLERY_RESPAWN_SECS: u32 = 30;

    async fn da03_spawns(pool: &sqlx::PgPool) -> Vec<SpawnRecord> {
        load_spawns_from_db(pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| SPAWNS.contains(&r.spawn_id))
            .collect()
    }

    fn tag(r: &SpawnRecord) -> &str {
        r.tag.as_deref().unwrap_or("")
    }

    /// The zone a tag belongs to, as an XZ box (and, for the gallery, the two
    /// terrace rows and the terrace height).
    fn in_zone(r: &SpawnRecord) -> Result<(), String> {
        let t = tag(r);
        let inside = |x0: f32, x1: f32, z0: f32, z1: f32| {
            (x0..=x1).contains(&r.x) && (z0..=z1).contains(&r.z)
        };
        let ok = if t.starts_with("DebugArea_Yard_") {
            // Z4 (416, -5.7, -786): the rows sit at z -806..-794, south of
            // the plan's centre, where the navmesh and the terrain agree.
            inside(390.0, 445.0, -810.0, -780.0)
        } else if t.starts_with("DebugArea_Slope_") {
            // Z5: the slope x 20-35, the wanderer at x 74, the leash station
            // at x 60 and the assist trio on the flat at x 116.
            inside(15.0, 125.0, -770.0, -640.0)
        } else if t.starts_with("DebugArea_Gallery_") {
            // Z7: the terrace rows z -592 / -612, y 23.06.
            (r.z == -592.0 || r.z == -612.0)
                && ((55.0..=171.0).contains(&r.x) || (305.0..=465.0).contains(&r.x))
                && (r.y - 23.06).abs() <= 0.3
        } else {
            return Err(format!(
                "spawn {} has a tag outside DA-03: {t:?}",
                r.spawn_id
            ));
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "{t} (spawn {}) at ({}, {}, {}) is outside its zone",
                r.spawn_id, r.x, r.y, r.z
            ))
        }
    }

    /// Every row of the DA-03 spawn block is in world 1300, carries a
    /// `DebugArea_` tag (D-DA6) and stands inside its zone, and every new
    /// template is in the 1330-1369 block.
    #[tokio::test]
    async fn debug_area_npcs_live_db_rows_are_world_1300_inside_their_zones() {
        let pool = require_db_or_skip!();
        let rows: Vec<(i32, i32, Option<String>)> = sqlx::query_as(
            "SELECT spawn_id, world_id, tag FROM resources.spawnlist \
             WHERE spawn_id BETWEEN 13200 AND 13599 ORDER BY spawn_id",
        )
        .fetch_all(&pool)
        .await
        .expect("spawnlist query must succeed");
        assert_eq!(rows.len(), 116, "DA-03 seeds 116 spawns");
        for (id, world, t) in &rows {
            assert_eq!(*world, WORLD_ID, "spawn {id} world");
            assert!(
                t.as_deref().is_some_and(|t| t.starts_with("DebugArea_")),
                "spawn {id} tag {t:?}"
            );
        }

        let spawns = da03_spawns(&pool).await;
        assert_eq!(spawns.len(), rows.len(), "every row loads");
        let errors: Vec<String> = spawns
            .iter()
            .filter(|r| r.world_name != WORLD)
            .map(|r| format!("spawn {} loads into {}", r.spawn_id, r.world_name))
            .chain(spawns.iter().filter_map(|r| in_zone(r).err()))
            .collect();
        assert!(errors.is_empty(), "{errors:#?}");

        let tags: BTreeSet<&str> = spawns.iter().map(tag).collect();
        assert_eq!(tags.len(), spawns.len(), "tags are unique");

        for r in spawns.iter().filter(|r| r.template_id >= 1300) {
            assert!(
                TEMPLATES.contains(&r.template_id),
                "{} uses template {} outside DA-03's block",
                tag(r),
                r.template_id
            );
        }
    }

    /// The factions players react to as HOSTILE: the cells of row 3 (Praxis,
    /// what every player reacts as) of `FACTION_REACTION_TABLE` that hold 1,
    /// read from the client's `entities/defs/enumerations.xml`, the table the
    /// AI's `is_hostile_to_players` uses (`cell/combat/faction_reaction.rs`,
    /// pinned to the same XML there). Derived, not hard-coded, so the gallery
    /// guard follows the AI.
    fn player_hostile_factions() -> Vec<i32> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../entities/defs/enumerations.xml");
        let xml = std::fs::read_to_string(&path).expect("read enumerations.xml");
        let start = xml
            .find("<FACTION_REACTION_TABLE>")
            .expect("FACTION_REACTION_TABLE in enumerations.xml");
        let end = start
            + xml[start..]
                .find("</FACTION_REACTION_TABLE>")
                .expect("closing tag");
        let row3 = xml[start..end]
            .split("<Data>")
            .nth(1 + 3)
            .expect("row 3 (Praxis)");
        let row3 = &row3[..row3.find("</Data>").expect("</Data>")];
        let hostile: Vec<i32> = row3
            .split(',')
            .enumerate()
            .filter(|(_, v)| v.trim() == "1")
            .map(|(i, _)| i as i32)
            .collect();
        assert!(
            hostile.contains(&HOSTILE_FACTION),
            "row 3 must be hostile to 10"
        );
        hostile
    }

    /// The gallery holds every template hostile to players (any faction the
    /// reaction table makes HOSTILE to them, not only 10) exactly once, tagged
    /// `DebugArea_Gallery_<template_id>`, passive (NEUTRAL override, D-DA9),
    /// mobile apart from the plant and the beacon, and on a short respawn. A
    /// hostile template the gallery does not place and
    /// [`GALLERY_EXCLUSIONS`] does not name fails here.
    #[tokio::test]
    async fn debug_area_npcs_live_db_gallery_covers_every_hostile_template() {
        let pool = require_db_or_skip!();
        let factions = player_hostile_factions();
        let hostile: Vec<(i32, String)> = sqlx::query_as(
            "SELECT template_id, template_name FROM resources.entity_templates \
             WHERE faction = ANY($1) AND class = 'mob' AND template_id < 1879048192 \
             ORDER BY template_id",
        )
        .bind(&factions)
        .fetch_all(&pool)
        .await
        .expect("entity_templates query must succeed");
        assert!(hostile.len() >= 101, "hostile templates: {}", hostile.len());

        let spawns = da03_spawns(&pool).await;
        let gallery: Vec<&SpawnRecord> = spawns
            .iter()
            .filter(|r| tag(r).starts_with("DebugArea_Gallery_"))
            .collect();
        let mut placed: BTreeMap<i32, Vec<&SpawnRecord>> = BTreeMap::new();
        for r in &gallery {
            placed.entry(r.template_id).or_default().push(r);
        }

        let excluded: BTreeMap<i32, &str> = GALLERY_EXCLUSIONS.into_iter().collect();
        let mut missing = Vec::new();
        for (id, name) in &hostile {
            let in_gallery = placed.get(id).map_or(0, Vec::len);
            if excluded.contains_key(id) || DEBUG_AREA_TEMPLATES.contains(id) {
                assert_eq!(in_gallery, 0, "excluded template {id} ({name}) is placed");
            } else if in_gallery != 1 {
                missing.push(format!("{id} {name}: placed {in_gallery} times"));
            }
        }
        assert!(
            missing.is_empty(),
            "hostile templates not in the gallery exactly once (place them in \
             spawnlist_debug_area_npcs.sql or add them to GALLERY_EXCLUSIONS): {missing:#?}"
        );

        for r in &gallery {
            let t = tag(r);
            assert_eq!(t, format!("DebugArea_Gallery_{}", r.template_id), "tag");
            assert!(
                r.faction.is_some_and(|f| factions.contains(&f)),
                "{t}: faction {:?} is not hostile to players",
                r.faction
            );
            assert_eq!(
                r.aggression_override,
                Some(MobAggression::Neutral),
                "{t}: the gallery is passive (D-DA9)"
            );
            assert!(
                r.respawn_secs
                    .is_some_and(|s| s <= MAX_GALLERY_RESPAWN_SECS),
                "{t}: respawn {:?}",
                r.respawn_secs
            );
            assert!(
                r.patrol_path.is_empty() && r.wander_radius == 0.0,
                "{t} stays put"
            );
        }
        let pinned: BTreeSet<i32> = gallery
            .iter()
            .filter(|r| r.is_stationary)
            .map(|r| r.template_id)
            .collect();
        assert_eq!(pinned, BTreeSet::from([77, 80]), "the beacon and the plant");
    }

    /// Each station loads with the behaviour it exists to show.
    #[tokio::test]
    async fn debug_area_npcs_live_db_stations_load_with_their_behaviour() {
        let pool = require_db_or_skip!();
        let spawns = da03_spawns(&pool).await;
        let one = |t: &str| -> &SpawnRecord {
            let m: Vec<&SpawnRecord> = spawns.iter().filter(|r| tag(r) == t).collect();
            assert_eq!(m.len(), 1, "exactly one {t}");
            m[0]
        };
        let prefixed = |p: &str| -> Vec<&SpawnRecord> {
            spawns.iter().filter(|r| tag(r).starts_with(p)).collect()
        };

        // Z4: friendly and neutral rows that can never fight, a pinned
        // neutral Jaffa that can be shot, and a hostile pen.
        let friendly = prefixed("DebugArea_Yard_Friendly_");
        assert_eq!(friendly.len(), 4, "four friendlies");
        let neutral = prefixed("DebugArea_Yard_Neutral_");
        assert_eq!(neutral.len(), 2, "two faction-7 neutrals");
        for r in friendly {
            assert!(
                matches!(r.faction, Some(1) | Some(9)),
                "{}: {:?}",
                tag(r),
                r.faction
            );
            assert_eq!(r.aggression_override, None);
        }
        for r in neutral {
            assert_eq!(r.faction, Some(7), "{}: Neutral_Ambient", tag(r));
            assert_eq!(r.aggression_override, None);
        }
        let pinned = prefixed("DebugArea_Yard_NeutralPinned_");
        assert_eq!(pinned.len(), 2);
        for r in pinned {
            assert_eq!(r.faction, Some(HOSTILE_FACTION), "{}: damageable", tag(r));
            assert_eq!(r.aggression_override, Some(MobAggression::Neutral));
            assert!(r.respawn_secs.is_some(), "{} respawns", tag(r));
        }
        let pen = prefixed("DebugArea_Yard_Hostile_");
        assert_eq!(pen.len(), 3);
        for r in pen {
            assert_eq!(r.faction, Some(HOSTILE_FACTION), "{}", tag(r));
            assert_eq!(r.aggression_override, None, "{}: hostile on sight", tag(r));
            assert!(r.respawn_secs.is_some(), "{} respawns", tag(r));
        }

        // Z5: the patrol route is the two-point 'Patrol' set 13200 in world 1300.
        let patrol = one("DebugArea_Slope_Patrol");
        assert_eq!(patrol.patrol_path.len(), 2, "A <-> B");
        assert!(patrol.patrol_point_delay_secs > 0.0);
        let set: (String, String, i32) = sqlx::query_as(
            "SELECT p.type, p.shape, p.world_id FROM resources.point_sets p \
             JOIN resources.spawnlist s ON s.patrol_path_id = p.set_id \
             WHERE s.spawn_id = $1",
        )
        .bind(patrol.spawn_id)
        .fetch_one(&pool)
        .await
        .expect("the patroller names a point set");
        assert_eq!(set, ("Patrol".into(), "Path".into(), WORLD_ID));

        let wander = one("DebugArea_Slope_Wander");
        assert_eq!(wander.wander_radius, 8.0);
        assert!(wander.patrol_path.is_empty(), "patrol beats wander");
        assert!(wander.wander_min_dwell_secs <= wander.wander_max_dwell_secs);

        let leash = one("DebugArea_Slope_Leash");
        assert_eq!(leash.leash_distance, Some(15.0));
        assert_eq!(leash.aggro_radius, Some(12.0));

        let trio = prefixed("DebugArea_Slope_Assist_");
        assert_eq!(trio.len(), 3);
        for r in &trio {
            assert_eq!(r.assist_radius, Some(10.0), "{}", tag(r));
            assert_eq!(r.aggro_radius, Some(6.0), "{}", tag(r));
        }

        for r in prefixed("DebugArea_Slope_") {
            assert_eq!(r.faction, Some(HOSTILE_FACTION), "{}", tag(r));
            assert_eq!(r.aggression_override, None, "{}: hostile", tag(r));
            assert!(r.respawn_secs.is_some(), "{} respawns", tag(r));
        }
    }

    /// The point-set footers in `point_sets_debug_area_npcs.sql`: after a load
    /// the set and point sequences hand out ids past every seeded row and past
    /// DA-03's reserved block (sets 13200-13209, points 13200-13299), so the
    /// console's `.path_add` never takes a seeded or reserved id. The base
    /// file's own footer is a fixed 2122 / 2551, which loads first; reverting
    /// this file's footer leaves the sequences there and fails this guard.
    #[tokio::test]
    async fn debug_area_npcs_live_db_point_set_sequences_pass_the_block() {
        let pool = require_db_or_skip!();
        for (seq, table, column, floor) in [
            ("point_sets_set_id_seq", "point_sets", "set_id", 13209_i64),
            (
                "point_set_points_point_id_seq",
                "point_set_points",
                "point_id",
                13299,
            ),
        ] {
            let sql = format!(
                "SELECT CASE WHEN s.is_called THEN s.last_value + 1 ELSE s.last_value END, \
                        (SELECT MAX({column}) FROM resources.{table} \
                         WHERE {column} < 1879048192)::int8 \
                 FROM resources.{seq} s"
            );
            let (next, max): (i64, Option<i64>) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("{seq} query must succeed: {e}"));
            assert!(
                next > max.unwrap_or(0),
                "{seq} would hand out {next}, but {table} holds {column} {max:?}"
            );
            assert!(
                next > floor,
                "{seq} would hand out {next}, inside DA-03's block up to {floor}"
            );
        }
    }
}
