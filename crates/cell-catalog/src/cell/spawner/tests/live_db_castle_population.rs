//! Live-DB guards for the Castle (World 8) population pass: templates 174-186,
//! friendly spawns 189-212, hostile spawns 247-282 and patrol sets 2086-2093
//! (`docs/analysis/castle-population/README.md`).
//!
//! A sibling of `live_db_castle_seed.rs`, split because that file guards CA05's
//! story actors and this one guards ambient content with different invariants.
//! Every row here loads cleanly whatever its coordinates, so the loaders cannot
//! catch any of the mistakes below; nothing at runtime logs them either:
//!
//! * a hostile placed so its aggro radius covers a respawner, the Armory ring
//!   pad or a mission actor, so a player who respawns or stops to talk is shot
//!   (and a guard that respawns every 120 s re-pulls a player mid-dialog);
//! * a friendly standing inside a hostile's aggro radius. NPCs do not fight each
//!   other yet, so it would stand idle beside a guard that is shooting the player;
//! * a friendly that can be attacked or turns hostile, or one whose `name_id`
//!   resolves to an empty string and renders with no name (the Castle's own
//!   Ogilvie moniker 8895 is exactly that);
//! * a new guard template outside levels 2-4, or an edit that leaks into the
//!   old level-1 templates 145/146/148 other Castle rows still use.
//!
//! Navmesh placement is guarded without a database by
//! `crates/entity/tests/castle_navmesh.rs`, which checks every World 8 spawn and
//! every Castle patrol leg against `data/spaces/castle.nav`.
mod live_db {
    use std::collections::{BTreeSet, HashMap};

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The population's spawn id blocks.
    const FRIENDLY_SPAWNS: std::ops::RangeInclusive<i32> = 189..=212;
    const HOSTILE_SPAWNS: std::ops::RangeInclusive<i32> = 247..=282;
    const FRIENDLY_TEMPLATES: [i32; 8] = [160, 174, 175, 176, 177, 178, 179, 180];
    const HOSTILE_TEMPLATES: [i32; 7] = [145, 181, 182, 183, 184, 185, 186];

    /// `(template_id, level, aggro_radius)` for the new guard templates: 181-183
    /// clone 148 (inside), 184-186 clone 146 (outside).
    const NEW_GUARDS: [(i32, i32, f32); 6] = [
        (181, 2, 15.0),
        (182, 3, 15.0),
        (183, 4, 15.0),
        (184, 2, 20.0),
        (185, 3, 20.0),
        (186, 4, 20.0),
    ];
    const NEW_GUARD_ASSIST_RADIUS: f32 = 12.0;

    /// `cimmeria_cell_world::cell::combat::aggression::{DEFAULT_AGGRO_RADIUS,
    /// AGGRO_VERTICAL_BAND}`, mirrored because cell-world depends on this crate.
    /// An NPC engages a player within this horizontal radius and this height band.
    const DEFAULT_AGGRO_RADIUS: f32 = 18.0;
    const AGGRO_VERTICAL_BAND: f32 = 4.0;

    /// Slack for where a player stands relative to the point being protected:
    /// the actor's own position is not where the player talks to it from.
    const PLAYER_STANDOFF: f32 = 3.0;
    /// How far outside a hostile's aggro radius a population friendly must stand.
    const FRIENDLY_CLEARANCE: f32 = 5.0;
    /// `Castle.ArmoryRingDropZone` is a cylinder of this radius.
    const RING_PAD_RADIUS: f32 = 3.5;

    /// Spawn tags of the actors a player must stand next to on the mission path.
    const MISSION_ACTORS: [&str; 9] = [
        "Castle_SgtGerschon",
        "Castle_Coppleman",
        "Castle_Zuritska_Cell",
        "Castle_Zuritska_Comms",
        "Castle_CommsTerminal",
        "Castle_AccessPanel",
        "Castle_ColMarsh",
        "Castle_Mohkatan",
        "Castle_DHD",
    ];

    /// Caged prisoners are the documented exemption from the friendly clearance
    /// rule: they stand behind cell doors in Romney's corridor.
    const CAGED_PREFIX: &str = "Castle_Pop_Prisoner";

    async fn castle_spawns(pool: &sqlx::PgPool) -> Vec<SpawnRecord> {
        load_spawns_from_db(pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| r.world_name == "Castle")
            .collect()
    }

    fn is_hostile(r: &SpawnRecord) -> bool {
        r.faction == Some(10)
    }

    fn aggro_radius(r: &SpawnRecord) -> f32 {
        r.aggro_radius.unwrap_or(DEFAULT_AGGRO_RADIUS)
    }

    /// Every point a hostile can stand on while idle: its spawn, plus its patrol
    /// loop sampled every tenth of a leg.
    fn idle_positions(r: &SpawnRecord) -> Vec<[f32; 3]> {
        let mut out = vec![[r.x, r.y, r.z]];
        let path = &r.patrol_path;
        for i in 0..path.len() {
            let (a, b) = (path[i], path[(i + 1) % path.len()]);
            for k in 0..=10 {
                let t = k as f32 / 10.0;
                out.push([
                    a.x + (b.x - a.x) * t,
                    a.y + (b.y - a.y) * t,
                    a.z + (b.z - a.z) * t,
                ]);
            }
        }
        out
    }

    /// `Some(horizontal distance)` when `p` is inside the NPC's height band.
    fn engage_distance(from: [f32; 3], p: [f32; 3]) -> Option<f32> {
        ((from[1] - p[1]).abs() <= AGGRO_VERTICAL_BAND)
            .then(|| ((from[0] - p[0]).powi(2) + (from[2] - p[2]).powi(2)).sqrt())
    }

    /// **Population guard**: the population is exactly the two spawn blocks, all
    /// in World 8, on the templates the ledger assigns, with unique tags; hostiles
    /// respawn on the zone's 120 s, every patrol starts at its own spawn, and each
    /// of the six new guard templates is placed at least once.
    ///
    /// Counts are pinned (24 + 36) so a dropped or duplicated row fails here
    /// rather than silently thinning a zone. A patrol whose first waypoint is not
    /// the spawn walks there first on every respawn, which reads as a guard
    /// sprinting across the room the moment it appears.
    #[tokio::test]
    async fn castle_population_live_db_rows_are_world8_blocks_with_their_templates() {
        let pool = require_db_or_skip!();
        let all = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        let friendly: Vec<&SpawnRecord> = all
            .iter()
            .filter(|r| FRIENDLY_SPAWNS.contains(&r.spawn_id))
            .collect();
        let hostile: Vec<&SpawnRecord> = all
            .iter()
            .filter(|r| HOSTILE_SPAWNS.contains(&r.spawn_id))
            .collect();
        assert_eq!(
            friendly.len(),
            24,
            "friendly block 189-212 must hold 24 rows"
        );
        assert_eq!(hostile.len(), 36, "hostile block 247-282 must hold 36 rows");

        let mut tags = BTreeSet::new();
        for r in all.iter().filter(|r| r.world_name == "Castle") {
            if let Some(tag) = &r.tag {
                assert!(
                    tags.insert(tag.clone()),
                    "World 8 tag {tag} is used by more than one spawn row; \
                     interact_tag and entity_death chains would bind either"
                );
            }
        }

        for r in friendly.iter().chain(hostile.iter()) {
            assert_eq!(
                r.world_name, "Castle",
                "spawn {} must be in World 8",
                r.spawn_id
            );
            assert!(
                r.tag.as_deref().is_some_and(|t| t.starts_with("Castle_")),
                "spawn {} needs a Castle_ tag, got {:?}",
                r.spawn_id,
                r.tag
            );
        }
        for r in &friendly {
            assert!(
                FRIENDLY_TEMPLATES.contains(&r.template_id),
                "friendly spawn {} ({:?}) uses template {}, not a population friendly",
                r.spawn_id,
                r.tag,
                r.template_id
            );
            assert_eq!(
                r.respawn_secs, None,
                "friendly spawn {} cannot die",
                r.spawn_id
            );
        }
        for r in &hostile {
            assert!(
                HOSTILE_TEMPLATES.contains(&r.template_id),
                "hostile spawn {} ({:?}) uses template {}, not a population hostile",
                r.spawn_id,
                r.tag,
                r.template_id
            );
            assert_eq!(
                r.respawn_secs,
                Some(120),
                "hostile spawn {} ({:?}) must respawn on the zone-wide 120 s",
                r.spawn_id,
                r.tag
            );
            if let Some(first) = r.patrol_path.first() {
                let off = ((first.x - r.x).powi(2) + (first.z - r.z).powi(2)).sqrt();
                assert!(
                    off < 0.5 && r.patrol_path.len() >= 2,
                    "patrolling spawn {} ({:?}) must start on its first waypoint and \
                     have at least two; first waypoint is {off:.1} u away, {} points",
                    r.spawn_id,
                    r.tag,
                    r.patrol_path.len()
                );
            }
        }
        let patrols = hostile.iter().filter(|r| !r.patrol_path.is_empty()).count();
        assert_eq!(
            patrols, 8,
            "the population authors eight patrol loops (2086-2093)"
        );
        for (id, level, _) in NEW_GUARDS {
            assert!(
                hostile.iter().any(|r| r.template_id == id),
                "guard template {id} (level {level}) is placed nowhere; every level of \
                 both variants is meant to appear in the mix"
            );
        }
    }

    /// **Population guard**: no population friendly can be attacked or turn on the
    /// player, and every one renders a real name.
    ///
    /// Faction 1 is what makes a friendly safe twice over: the damage gates refuse
    /// any target that is not faction 10, and the Praxis row of the faction
    /// reaction table reads faction 1 as FRIENDLY. A hostile aggression override
    /// would bypass the second. The name check reads `resources.texts.text`, not
    /// just the id: Ogilvie's own Castle moniker (8895) exists and is empty, so an
    /// id-only check would pass a nameless Ogilvie.
    #[tokio::test]
    async fn castle_population_live_db_friendlies_are_safe_and_named() {
        let pool = require_db_or_skip!();
        let spawns = castle_spawns(&pool).await;
        for r in spawns
            .iter()
            .filter(|r| FRIENDLY_SPAWNS.contains(&r.spawn_id))
        {
            assert_eq!(
                r.faction,
                Some(1),
                "friendly {:?} (spawn {}) must be faction 1; faction 10 makes it a \
                 target and other factions may read hostile to players",
                r.tag,
                r.spawn_id
            );
            assert!(
                !r.aggression_override.is_some_and(|a| a.is_hostile()),
                "friendly {:?} carries a hostile aggression override",
                r.tag
            );
            let name_id = r
                .name_id
                .unwrap_or_else(|| panic!("friendly {:?} has no name_id", r.tag));
            let text: Option<(String,)> =
                sqlx::query_as("SELECT text FROM resources.texts WHERE moniker_id = $1")
                    .bind(name_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("resources.texts probe must succeed");
            assert!(
                text.as_ref().is_some_and(|(t,)| !t.trim().is_empty()),
                "friendly {:?} name_id {name_id} resolves to {text:?}; an empty or \
                 missing moniker renders the NPC with no name",
                r.tag
            );
        }
    }

    /// **Population guard**: the new guard templates are levels 2-4 with the
    /// ledger's radii and the SMG kit, and the old level-1 templates they were
    /// cloned from are untouched.
    ///
    /// `level` sets max HP (200 + 50 * level) and kill XP (10 * level) and nothing
    /// else, so a level typo is a silent balance change. The old templates are
    /// pinned column by column because the easy mistake is editing 146/148 in
    /// place instead of adding a row, which re-levels every pre-existing Castle
    /// guard at once.
    #[tokio::test]
    async fn castle_population_live_db_guard_templates_are_levels_2_to_4_and_old_ones_unchanged() {
        let pool = require_db_or_skip!();
        let templates: HashMap<i32, SpawnRecord> = load_spawn_templates(&pool)
            .await
            .expect("load_spawn_templates must succeed");
        let get = |id: i32| {
            templates
                .get(&id)
                .unwrap_or_else(|| panic!("template {id} must load"))
        };

        let smg_kit = &get(148).ability_ids;
        assert!(!smg_kit.is_empty(), "template 148 must carry ability set 3");
        for (id, level, aggro) in NEW_GUARDS {
            let t = get(id);
            assert_eq!(t.level, Some(level), "template {id} must be level {level}");
            assert_eq!(
                t.faction,
                Some(10),
                "template {id} must be hostile (faction 10)"
            );
            assert_eq!(t.aggro_radius, Some(aggro), "template {id} aggro_radius");
            assert_eq!(
                t.assist_radius,
                Some(NEW_GUARD_ASSIST_RADIUS),
                "template {id} assist_radius"
            );
            assert_eq!(
                &t.ability_ids, smg_kit,
                "template {id} must fire the SMG set"
            );
            assert_eq!(t.use_cover, Some(true), "template {id} takes cover");
        }

        // (template, name_id): the level-1 Castle mobs as they shipped.
        for (id, name_id) in [(145, 6968), (146, 7417), (148, 7417)] {
            let t = get(id);
            assert_eq!(t.level, Some(1), "template {id} must stay level 1");
            assert_eq!(t.faction, Some(10), "template {id} faction changed");
            assert_eq!(t.name_id, Some(name_id), "template {id} name_id changed");
            assert_eq!(t.aggro_radius, None, "template {id} aggro_radius changed");
            assert_eq!(t.assist_radius, None, "template {id} assist_radius changed");
            assert_eq!(
                t.respawn_secs, None,
                "template {id} template-level respawn changed"
            );
            assert_eq!(t.loot_table_id, None, "template {id} loot table changed");
        }
    }

    /// **Population guard**: no World 8 hostile can aggro a player who is standing
    /// on a respawner, on the Armory ring pad, or next to a mission actor.
    ///
    /// Checked for every faction-10 row, not only the population's, because the
    /// failure is the same whoever authored it; population rows must also clear
    /// [`PLAYER_STANDOFF`]. Patrols are checked along the whole loop. A breach
    /// is a player shot on arrival or on respawn, or pulled while in a dialog,
    /// and with a 120 s respawn it recurs every two minutes.
    #[tokio::test]
    async fn castle_population_live_db_hostile_aggro_clears_respawners_ring_pad_and_actors() {
        let pool = require_db_or_skip!();
        let spawns = castle_spawns(&pool).await;
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");

        // (what, position, player slack, whether pre-population rows get the slack)
        let mut protected: Vec<(String, [f32; 3], f32, bool)> = Vec::new();
        let respawners: Vec<(String, f32, f32, f32)> = sqlx::query_as(
            "SELECT name, pos_x, pos_y, pos_z FROM resources.respawners WHERE world_id = 8",
        )
        .fetch_all(&pool)
        .await
        .expect("respawners query must succeed");
        assert_eq!(respawners.len(), 4, "World 8 has four respawners");
        for (name, x, y, z) in respawners {
            protected.push((
                format!("respawner '{name}'"),
                [x, y, z],
                PLAYER_STANDOFF,
                false,
            ));
        }
        let pad = regions
            .iter()
            .find(|r| r.name == "Castle.ArmoryRingDropZone")
            .expect("the Armory ring pad point set must load");
        let n = pad.points.len() as f32;
        let centre = pad.points.iter().fold([0.0; 3], |acc, p| {
            [acc[0] + p[0] / n, acc[1] + p[1] / n, acc[2] + p[2] / n]
        });
        // The pad volume sits on the floor; test at floor height (the lowest
        // point). An arriving player can land anywhere on it, so its radius
        // applies to every hostile, old or new.
        let floor = pad.points.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        protected.push((
            "the Armory ring pad".into(),
            [centre[0], floor, centre[2]],
            RING_PAD_RADIUS,
            true,
        ));
        for tag in MISSION_ACTORS {
            let a = spawns
                .iter()
                .find(|r| r.tag.as_deref() == Some(tag))
                .unwrap_or_else(|| panic!("mission actor {tag} must have a spawn row"));
            protected.push((tag.to_string(), [a.x, a.y, a.z], PLAYER_STANDOFF, false));
        }

        let hostiles: Vec<&SpawnRecord> = spawns.iter().filter(|r| is_hostile(r)).collect();
        assert!(
            hostiles
                .iter()
                .any(|r| HOSTILE_SPAWNS.contains(&r.spawn_id)),
            "no population hostile loaded; the probe itself is broken"
        );
        for h in hostiles {
            let population = HOSTILE_SPAWNS.contains(&h.spawn_id);
            for (what, at, slack, for_all) in &protected {
                // Pre-population rows are held to the runtime truth only; the
                // tightest is Castle_PRU1 at 18.03 u from the Op-Core Triage
                // respawner, inside the player slack but outside its radius.
                let slack = if population || *for_all { *slack } else { 0.0 };
                let need = aggro_radius(h) + slack;
                for p in idle_positions(h) {
                    if let Some(d) = engage_distance(p, *at) {
                        assert!(
                            d > need,
                            "hostile {:?} (spawn {}, aggro {:.0} u) comes within {d:.1} u \
                             of {what} at ({:.1}, {:.1}, {:.1}) while idle or patrolling \
                             at ({:.1}, {:.1}, {:.1}); it must stay beyond {need:.1} u",
                            h.tag,
                            h.spawn_id,
                            aggro_radius(h),
                            at[0],
                            at[1],
                            at[2],
                            p[0],
                            p[1],
                            p[2]
                        );
                    }
                }
            }
        }
    }

    /// **Population guard**: every population friendly except the caged
    /// prisoners stands at least [`FRIENDLY_CLEARANCE`] outside the aggro radius
    /// of every World 8 hostile, patrol loops included.
    ///
    /// NPCs do not fight each other yet. The `Castle_Standoff_*` rows face hostile
    /// ground from behind cover and read as a standoff only while they stay out of
    /// range; inside it they would stand idle next to a guard that is shooting the
    /// player. When NPC-vs-NPC combat lands, moving them into range is a
    /// deliberate edit that updates this guard.
    #[tokio::test]
    async fn castle_population_live_db_friendlies_stand_outside_every_hostile_aggro_radius() {
        let pool = require_db_or_skip!();
        let spawns = castle_spawns(&pool).await;
        let hostiles: Vec<&SpawnRecord> = spawns.iter().filter(|r| is_hostile(r)).collect();
        let friendlies: Vec<&SpawnRecord> = spawns
            .iter()
            .filter(|r| FRIENDLY_SPAWNS.contains(&r.spawn_id))
            .filter(|r| {
                !r.tag
                    .as_deref()
                    .is_some_and(|t| t.starts_with(CAGED_PREFIX))
            })
            .collect();
        assert!(
            friendlies
                .iter()
                .filter(|r| r
                    .tag
                    .as_deref()
                    .is_some_and(|t| t.starts_with("Castle_Standoff_")))
                .count()
                >= 8,
            "expected the eight Castle_Standoff_ rows (courtyard and Checkpoint Alpha)"
        );
        for f in friendlies {
            for h in &hostiles {
                let need = aggro_radius(h) + FRIENDLY_CLEARANCE;
                for p in idle_positions(h) {
                    if let Some(d) = engage_distance(p, [f.x, f.y, f.z]) {
                        assert!(
                            d >= need,
                            "friendly {:?} (spawn {}) is {d:.1} u from hostile {:?} \
                             (spawn {}, aggro {:.0} u); friendlies must stand {need:.1} u \
                             or more away until NPCs can fight each other",
                            f.tag,
                            f.spawn_id,
                            h.tag,
                            h.spawn_id,
                            aggro_radius(h)
                        );
                    }
                }
            }
        }
    }
}
