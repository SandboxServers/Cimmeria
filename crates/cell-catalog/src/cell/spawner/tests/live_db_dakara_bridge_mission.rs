//! DK-10fs: the Dakara bridge mission 63100 (D-DK1 F). Its seed rows must
//! load into the accept and advance caches, because `accept_mission 63100`
//! (DK-10fc's chain 8075, wave 3) reads exactly these caches.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_dakara_bridge_mission_63100_loads_one_step_one_objective() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&63100)
            .expect("mission 63100 must be in the accept cache");
        assert_eq!(entry.step_id, 63101);
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 1);
        assert!(entry.can_repeat_on_fail);
        assert_eq!(entry.objectives.len(), 1);
        let objective = &entry.objectives[0];
        assert_eq!(objective.objective_id, 63102);
        assert!(!objective.is_hidden);
        assert!(!objective.is_optional);

        let objectives = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let step = objectives
            .get(&63101)
            .expect("step 63101 must be in the advance cache");
        assert_eq!(step.len(), 1);
        assert_eq!(step[0].objective_id, 63102);
        assert!(!step[0].is_hidden);
        assert!(!step[0].is_optional);
    }
}
