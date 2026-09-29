//! Live-DB guards for the Castle standoff (NPC-vs-NPC, #1009, D-CP11): the eight
//! `Castle_Standoff_*` rows and their templates 187-189
//! (`docs/analysis/castle-population/README.md`, K20).
//!
//! Before #1009 these rows stood out of every hostile's reach on purpose,
//! because NPCs did not fight. Now they are meant to fight, and the seed
//! mistakes that would silently stop that are exactly the ones nothing logs:
//!
//! * a standoff row left on faction 1 (World Object), which is FRIENDLY in every
//!   row of the reaction table and so never fights anything;
//! * a standoff row moved out of reach of every hostile, so the scan never finds
//!   one;
//! * a standoff template that re-levels or re-factions a shared template (160 is
//!   Harset's and the Checkpoint Alpha story actors' too).
//!
//! What the rows do at runtime, on the real navmesh and occluder, is pinned by
//! `cimmeria-cell`'s `service::tests::npc_ai::castle_standoff`.

mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const STANDOFF_PREFIX: &str = "Castle_Standoff_";
    /// Template for each standoff row, by tag.
    const ROWS: [(&str, i32); 8] = [
        ("Castle_Standoff_SgtStanton", 188),
        ("Castle_Standoff_Courtyard_Soldier1", 187),
        ("Castle_Standoff_Courtyard_Soldier2", 187),
        ("Castle_Standoff_Courtyard_Soldier3", 187),
        ("Castle_Standoff_Courtyard_Soldier4", 187),
        ("Castle_Standoff_Alpha_Jaffa1", 189),
        ("Castle_Standoff_Alpha_Jaffa2", 189),
        ("Castle_Standoff_Alpha_Jaffa3", 189),
    ];
    /// Praxis. `faction_reaction::reaction(3, 10)` and `(10, 3)` are HOSTILE;
    /// `(3, 3)` is FRIENDLY, so the players (who react as 3) see a friend.
    const STANDOFF_FACTION: i32 = 3;
    const HOSTILE_FACTION: i32 = 10;
    /// `cimmeria_cell_world::cell::combat::aggression::{DEFAULT_AGGRO_RADIUS,
    /// AGGRO_VERTICAL_BAND}`, mirrored because cell-world depends on this crate.
    const DEFAULT_AGGRO_RADIUS: f32 = 18.0;
    const AGGRO_VERTICAL_BAND: f32 = 4.0;

    /// **Standoff guard.** Exactly the eight rows, each on its standoff template,
    /// faction 3, a 30 u aggro radius, no non-hostile override (which would
    /// disarm it against NPCs too), a 120 s respawn, and at least one faction-10
    /// hostile inside its aggro radius and height band, so its scan has
    /// something to engage.
    ///
    /// Mutation proof: set a row's template back to 174 (faction 1), or move a
    /// courtyard row back to the barricade at (507.5, 28.18, 640), and this fails
    /// naming the row.
    #[tokio::test]
    async fn castle_population_live_db_standoff_rows_have_a_hostile_in_reach() {
        let pool = require_db_or_skip!();
        let spawns: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| r.world_name == "Castle")
            .collect();
        let standoff: Vec<&SpawnRecord> = spawns
            .iter()
            .filter(|r| {
                r.tag
                    .as_deref()
                    .is_some_and(|t| t.starts_with(STANDOFF_PREFIX))
            })
            .collect();
        assert_eq!(standoff.len(), ROWS.len(), "the eight standoff rows");
        let hostiles: Vec<&SpawnRecord> = spawns
            .iter()
            .filter(|r| r.faction == Some(HOSTILE_FACTION))
            .collect();

        for (tag, template_id) in ROWS {
            let r = standoff
                .iter()
                .find(|r| r.tag.as_deref() == Some(tag))
                .unwrap_or_else(|| panic!("standoff row {tag} is missing"));
            assert_eq!(r.template_id, template_id, "{tag} template");
            assert_eq!(
                r.faction,
                Some(STANDOFF_FACTION),
                "{tag} must be faction 3: faction 1 never fights"
            );
            assert_eq!(r.aggro_radius, Some(30.0), "{tag} aggro radius");
            assert!(
                r.aggression_override.is_none(),
                "{tag} carries an aggression override ({:?}); a non-hostile one \
                 disarms it against NPCs too",
                r.aggression_override
            );
            assert_eq!(r.respawn_secs, Some(120), "{tag} respawn");
            let radius = r.aggro_radius.unwrap_or(DEFAULT_AGGRO_RADIUS);
            let in_reach: Vec<(&str, f32)> = hostiles
                .iter()
                .filter(|h| (h.y - r.y).abs() <= AGGRO_VERTICAL_BAND)
                .map(|h| {
                    let d = ((h.x - r.x).powi(2) + (h.z - r.z).powi(2)).sqrt();
                    (h.tag.as_deref().unwrap_or("?"), d)
                })
                .filter(|&(_, d)| d <= radius)
                .collect();
            assert!(
                !in_reach.is_empty(),
                "{tag} at ({}, {}, {}) has no faction-10 hostile within its {radius} u \
                 aggro radius and 4 u of height: it would never fight",
                r.x,
                r.y,
                r.z
            );
        }
    }

    /// **Shared-template guard.** The standoff templates are new rows; the
    /// templates they were cloned from keep faction 1 and their level, because
    /// 160 also stands at Checkpoint Alpha as a story actor and in Harset, and
    /// 174 / 178 are the other Castle friendlies.
    #[tokio::test]
    async fn castle_population_live_db_standoff_templates_leave_their_sources_alone() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool)
            .await
            .expect("load_spawn_templates must succeed");
        for (id, level) in [(160, 50), (174, 4), (178, 50)] {
            let t = templates
                .get(&id)
                .unwrap_or_else(|| panic!("template {id}"));
            assert_eq!(t.faction, Some(1), "template {id} faction changed");
            assert_eq!(t.level, Some(level), "template {id} level changed");
        }
        for id in [187, 188, 189] {
            let t = templates
                .get(&id)
                .unwrap_or_else(|| panic!("standoff template {id} is missing"));
            assert_eq!(t.faction, Some(STANDOFF_FACTION), "template {id} faction");
            assert_eq!(t.level, Some(4), "template {id} level");
            assert_eq!(t.class.as_str(), "mob", "template {id} class");
        }
    }
}
