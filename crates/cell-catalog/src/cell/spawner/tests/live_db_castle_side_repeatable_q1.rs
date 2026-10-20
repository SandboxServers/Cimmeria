//! Live-DB guard for the Castle `repeatable_q1` authored mission (CS-05b1s): the
//! seeded mission 10330 loads through the spawner's mission loaders with its
//! sealed steps, objectives and repeat cap, and its hub giver (template 424,
//! spawn 13809) loads at the sealed transform.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_castle_side_repeatable_q1_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10330)
            .expect("mission 10330 loads with its first step");
        assert_eq!(entry.step_id, 10384, "step A is the first step by index");
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 2, "the sealed cap is NumRepeats 2");
        assert!(!entry.can_repeat_on_fail);
        let first: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(first, vec![10504]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let second: Vec<i32> = steps
            .get(&10385)
            .expect("step B 10385 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(second, vec![10505]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10384), Some(&10330));
        assert_eq!(owners.get(&10385), Some(&10330));
        assert!(steps[&10385].iter().all(|o| !o.is_hidden && !o.is_optional));

        // Frozen (e): template 424 is the non-vendor hub giver, spawn 13809 its
        // only spawn, at the sealed navmesh transform with the chain tag.
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let giver = spawns
            .iter()
            .find(|r| r.spawn_id == 13809)
            .expect("spawn 13809 must load");
        assert_eq!(giver.world_name, "Castle", "spawn 13809 is in World 8");
        assert_eq!(giver.template_id, 424, "spawn 13809 uses template 424");
        assert_eq!(
            [giver.x, giver.y, giver.z],
            [440.0, 70.22, 984.0],
            "spawn 13809 sits at the sealed transform"
        );
        assert!(
            (giver.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
            "spawn 13809 heading {}",
            giver.heading
        );
        assert_eq!(giver.tag.as_deref(), Some("Castle_HubSupplyOfficer"));
        assert!(giver.is_stationary, "the supply officer never walks");
        assert_eq!(giver.interaction_type, 0, "template 424 is not a vendor");
        assert_eq!(giver.faction, Some(1), "the supply officer is a friendly");
        let on_424 = spawns.iter().filter(|r| r.template_id == 424).count();
        assert_eq!(on_424, 1, "template 424 has exactly one spawn");
    }
}
