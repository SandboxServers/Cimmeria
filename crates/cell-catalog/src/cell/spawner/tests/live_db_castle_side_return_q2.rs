//! Live-DB guard for the Castle `return_q2` authored mission (CS-04c2s): the
//! seeded mission 10321 loads through the spawner's mission loaders with its
//! sealed step, objective and flag shape.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_castle_side_return_q2_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10321)
            .expect("mission 10321 loads with its first step");
        assert_eq!(entry.step_id, 10378, "step A is the first step by index");
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 0);
        assert!(!entry.can_repeat_on_fail);
        let first: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(first, vec![10498]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let second: Vec<i32> = steps
            .get(&10379)
            .expect("step B 10379 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(second, vec![10499]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10378), Some(&10321));
        assert_eq!(owners.get(&10379), Some(&10321));
        assert!(steps[&10379].iter().all(|o| !o.is_hidden && !o.is_optional));
    }
}
