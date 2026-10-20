//! Live-DB guard for the Castle `return_q1` authored mission (CS-04c1s): the
//! seeded mission 10320 loads through the spawner's mission loaders with its
//! sealed step, objective and flag shape.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_castle_side_return_q1_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10320)
            .expect("mission 10320 loads with its first step");
        assert_eq!(entry.step_id, 10372, "step A is the only step");
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 0);
        assert!(!entry.can_repeat_on_fail);
        let first: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(first, vec![10492]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let only: Vec<i32> = steps
            .get(&10372)
            .expect("step 10372 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(only, vec![10492]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10372), Some(&10320));
        assert!(steps[&10372].iter().all(|o| !o.is_hidden && !o.is_optional));
    }
}
