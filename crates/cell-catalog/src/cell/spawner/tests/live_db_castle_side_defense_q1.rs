//! Live-DB guard for the Castle `defense_q1` authored mission (CS-04b1s): the
//! seeded mission 10310 loads through the spawner's mission loaders with its
//! sealed step, objective and flag shape, and the sentry giver (template 419,
//! spawn 13808) loads at its sealed transform in World 8.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The sealed transform of the `defense_q1` sentry, spawn 13808 in World 8.
    const SENTRY_POS: [f32; 3] = [518.0, 28.18, 640.0];
    const SENTRY_HEADING: f32 = 5.119;

    #[tokio::test]
    async fn live_db_castle_side_defense_q1_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10310)
            .expect("mission 10310 loads with its first step");
        assert_eq!(entry.step_id, 10354, "step A is the first step by index");
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 0);
        assert!(!entry.can_repeat_on_fail);
        let first: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(first, vec![10474]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let second: Vec<i32> = steps
            .get(&10355)
            .expect("step B 10355 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(second, vec![10475]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10354), Some(&10310));
        assert_eq!(owners.get(&10355), Some(&10310));
        assert!(steps[&10355].iter().all(|o| !o.is_hidden && !o.is_optional));
    }

    #[tokio::test]
    async fn live_db_castle_side_defense_q1_sentry_loads() {
        let pool = require_db_or_skip!();
        let spawns = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let sentry = spawns
            .iter()
            .find(|r| r.spawn_id == 13808)
            .expect("spawn 13808 must load");
        assert_eq!(sentry.world_name, "Castle", "spawn 13808 is in World 8");
        assert_eq!(sentry.template_id, 419, "spawn 13808 uses template 419");
        for (got, want) in [sentry.x, sentry.y, sentry.z].iter().zip(SENTRY_POS) {
            assert!((got - want).abs() < 1e-3, "spawn 13808 at {got} != {want}");
        }
        assert!(
            (sentry.heading - SENTRY_HEADING).abs() < 1e-3,
            "spawn 13808 heading {} != {SENTRY_HEADING}",
            sentry.heading
        );
        assert!(sentry.is_stationary, "the sentry stands on post");
        assert_eq!(sentry.tag.as_deref(), Some("Castle_DefenseQ1_Sentry"));
        let on_419 = spawns.iter().filter(|r| r.template_id == 419).count();
        assert_eq!(on_419, 1, "template 419 has exactly one spawn");

        let templates: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM resources.entity_templates WHERE template_id = 419",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(templates, 1, "template 419 is seeded exactly once");
        // Template 419 is template 174 with only its id and name changed.
        let (same,): (bool,) = sqlx::query_as(
            "SELECT (to_jsonb(a) - 'template_id' - 'template_name') \
                 = (to_jsonb(b) - 'template_id' - 'template_name') \
             FROM resources.entity_templates a, resources.entity_templates b \
             WHERE a.template_id = 419 AND b.template_id = 174",
        )
        .fetch_one(&pool)
        .await
        .expect("template 419 loads beside template 174");
        assert!(same, "template 419 differs from 174 beyond id and name");
    }
}
