//! Live-DB guard for the Castle `defense_q3` authored mission (CS-04b3s): the
//! seeded mission 10312 loads through the spawner's mission loaders with its
//! sealed step and four required objectives, and its giver (template 425,
//! spawn 14096) loads at the sealed transform in World 8.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_castle_side_defense_q3_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10312)
            .expect("mission 10312 loads with its first step");
        assert_eq!(
            entry.step_id, 10366,
            "the sealed step is the first by index"
        );
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 0);
        assert!(!entry.can_repeat_on_fail);
        let objectives: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(objectives, vec![10486, 10487, 10488, 10489]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let seeded: Vec<i32> = steps
            .get(&10366)
            .expect("step 10366 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(seeded, vec![10486, 10487, 10488, 10489]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10366), Some(&10312));
    }

    #[tokio::test]
    async fn live_db_castle_side_defense_q3_giver_loads() {
        let pool = require_db_or_skip!();
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let giver = spawns
            .iter()
            .find(|r| r.spawn_id == 14096)
            .expect("spawn 14096 must load");
        assert_eq!(giver.world_name, "Castle", "spawn 14096 is in World 8");
        assert_eq!(giver.template_id, 425, "spawn 14096 uses template 425");
        assert_eq!(
            [giver.x, giver.y, giver.z],
            [790.0, 59.7, 530.0],
            "spawn 14096 sits at the sealed transform"
        );
        assert!(
            (giver.heading - 5.3559).abs() < 1e-3,
            "spawn 14096 heading {}",
            giver.heading
        );
        assert_eq!(giver.tag.as_deref(), Some("Castle_DefenseQ3_Giver"));
        assert!(giver.is_stationary, "the giver stands on post");
        assert_eq!(giver.faction, Some(3), "the giver is a friendly");
        let on_425 = spawns.iter().filter(|r| r.template_id == 425).count();
        assert_eq!(on_425, 1, "template 425 has exactly one spawn");
    }

    /// `cimmeria_entity`'s aggro default, mirrored as `live_db_castle_hub`
    /// does (cell-world depends on this crate).
    const DEFAULT_AGGRO_RADIUS: f32 = 18.0;
    /// Horizontal slack between a hostile's reach and the giver (the seal's 5 m).
    const GIVER_CLEARANCE: f32 = 5.0;
    /// The giver's sealed position (spawn 14096).
    const GIVER_POS: [f32; 3] = [790.0, 59.7, 530.0];

    /// Every point a hostile can stand on while idle: its spawn, plus its
    /// patrol loop sampled every tenth of a leg.
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

    /// No idle hostile in World 8 reaches the giver (CS-04b3c2, seal c2 test 7):
    /// each faction-10 spawn's aggro radius (default 18 m), wander radius and
    /// the 5 m clearance stay short of spawn 14096 from every idle position.
    /// Revert proof: move spawn 14096 to (955, 25, 475), Bravo officer 1's spot.
    #[tokio::test]
    async fn castle_side_defense_q3_giver_clearance() {
        let pool = require_db_or_skip!();
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let giver = spawns
            .iter()
            .find(|r| r.spawn_id == 14096)
            .expect("spawn 14096 must load");
        let giver_pos = [giver.x, giver.y, giver.z];
        assert_eq!(
            giver_pos, GIVER_POS,
            "the giver sits at the sealed position"
        );

        let mut checked = 0;
        for h in spawns
            .iter()
            .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
        {
            checked += 1;
            let reach =
                h.aggro_radius.unwrap_or(DEFAULT_AGGRO_RADIUS) + h.wander_radius + GIVER_CLEARANCE;
            for p in idle_positions(h) {
                let d = ((p[0] - giver_pos[0]).powi(2) + (p[2] - giver_pos[2]).powi(2)).sqrt();
                assert!(
                    d > reach,
                    "hostile spawn {} ({:?}) reaches the giver: {d:.2} u <= reach {reach:.2} u",
                    h.spawn_id,
                    h.tag
                );
            }
        }
        assert!(
            checked > 0,
            "no faction-10 World 8 spawn loaded; the probe is broken"
        );
    }
}
