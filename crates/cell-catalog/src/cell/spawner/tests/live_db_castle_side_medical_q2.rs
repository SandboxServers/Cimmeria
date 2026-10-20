//! Live-DB guard for the Castle `medical_q2` authored mission (CS-04a2s): the
//! seeded mission 10301 loads through the spawner's mission loaders with its
//! sealed step, objective and flag shape, and the prisoner 417 at spawn 13806
//! is seeded as the sealed template and spawn.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn live_db_castle_side_medical_q2_seed_loads() {
        let pool = require_db_or_skip!();
        let defs = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        let entry = defs
            .get(&10301)
            .expect("mission 10301 loads with its first step");
        assert_eq!(entry.step_id, 10342, "step A is the first step by index");
        assert!(!entry.is_hidden);
        assert_eq!(entry.num_repeats, 0);
        assert!(!entry.can_repeat_on_fail);
        let first: Vec<i32> = entry.objectives.iter().map(|o| o.objective_id).collect();
        assert_eq!(first, vec![10462]);
        assert!(entry
            .objectives
            .iter()
            .all(|o| !o.is_hidden && !o.is_optional));

        let steps = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        let second: Vec<i32> = steps
            .get(&10343)
            .expect("step B 10343 loads its objectives")
            .iter()
            .map(|o| o.objective_id)
            .collect();
        assert_eq!(second, vec![10463]);
        let owners = load_step_missions(&pool)
            .await
            .expect("load_step_missions must succeed");
        assert_eq!(owners.get(&10342), Some(&10301));
        assert_eq!(owners.get(&10343), Some(&10301));
        assert!(steps[&10343].iter().all(|o| !o.is_hidden && !o.is_optional));
    }

    #[tokio::test]
    async fn live_db_castle_side_medical_q2_prisoner_seeded() {
        let pool = require_db_or_skip!();
        // Spawn 13806 is world 8, template 417, tag Castle_Pop_Prisoner5.
        let (world, template, tag, x, y, z, heading): (i32, i32, String, f32, f32, f32, f32) =
            sqlx::query_as(
                "SELECT world_id, template_id, tag, x, y, z, heading \
                 FROM resources.spawnlist WHERE spawn_id = 13806",
            )
            .fetch_one(&pool)
            .await
            .expect("spawn 13806 is seeded");
        assert_eq!((world, template), (8, 417));
        assert_eq!(tag, "Castle_Pop_Prisoner5");
        // Frozen (e): the sealed navmesh transform (component 250, h 0.00 m).
        assert_eq!([x, y, z], [270.0, 66.98, 1050.0], "spawn 13806 transform");
        assert!(
            (heading - std::f32::consts::PI).abs() < 1e-4,
            "spawn 13806 heading {heading}"
        );
        // Template 417 is template 176 with only its id and name changed.
        let (same,): (bool,) = sqlx::query_as(
            "SELECT (to_jsonb(a) - 'template_id' - 'template_name') \
                 = (to_jsonb(b) - 'template_id' - 'template_name') \
             FROM resources.entity_templates a, resources.entity_templates b \
             WHERE a.template_id = 417 AND b.template_id = 176",
        )
        .fetch_one(&pool)
        .await
        .expect("template 417 loads beside template 176");
        assert!(same, "template 417 differs from 176 beyond id and name");
        // 417 has exactly one spawn, and the tag is unique across spawns.
        let (spawns,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM resources.spawnlist WHERE template_id = 417")
                .fetch_one(&pool)
                .await
                .expect("count spawns of template 417");
        assert_eq!(spawns, 1);
        let (tags,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM resources.spawnlist WHERE tag = 'Castle_Pop_Prisoner5'",
        )
        .fetch_one(&pool)
        .await
        .expect("count Castle_Pop_Prisoner5 tags");
        assert_eq!(tags, 1);
    }
}
