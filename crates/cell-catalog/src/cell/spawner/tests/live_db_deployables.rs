//! Live-DB guards for the deployables seed (deployables Phase 0,
//! `docs/analysis/deployables/`): `resources.deployables`, template 400 and
//! the two effects of 1012 "Deployable: Microwave Emitter".
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a deployables row naming a template that is not a deployable (a mob,
//!   a looted template, one some other campaign owns, or one placed in
//!   `spawnlist`, which would stand an ownerless emitter in the world);
//! * 1012 losing the cooked-data numbers the design rests on: 30 pulses of
//!   1 s from 5065, the "Medium" (8 m) radius, 100 Focus and the
//!   Focus-first script on 5066;
//! * template 400 naming a body set or component the seed does not know.
//!
//! Each was proven to fail with its seed row reverted (ledger
//! `docs/analysis/deployables/README.md`).
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// The deployable templates the campaign owns.
    const DEPLOYABLE_TEMPLATES: std::ops::RangeInclusive<i32> = 400..=409;

    const MICROWAVE_EMITTER: i32 = 1012;
    const MICROWAVE_TEMPLATE: i32 = 400;
    const PULSER: i32 = 5065;
    const DAMAGE: i32 = 5066;

    /// Every `deployables` row names a deployable template (400-409, class
    /// `being`, no loot table) and two loaded effects, and 1012's row is the
    /// Phase 0 binding. Read through the startup loaders, so a broken loader
    /// query fails here too.
    #[tokio::test]
    async fn every_live_db_deployable_names_a_deployable_template_and_its_effects() {
        let pool = require_db_or_skip!();
        let catalog = load_deployables(&pool).await.expect("deployables load");
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        let effects = load_effect_defs(&pool).await.expect("effects load");

        assert_eq!(
            catalog.deployable_for(MICROWAVE_EMITTER),
            Some(DeployableSpec {
                ability_id: MICROWAVE_EMITTER,
                template_id: MICROWAVE_TEMPLATE,
                lifetime_effect_id: PULSER,
                pulse_effect_id: DAMAGE,
                max_active: 1,
            }),
            "1012 must place template 400, timed by 5065 and pulsing 5066, one at a time"
        );

        let rows: Vec<(i32, i32, i32, i32)> = sqlx::query_as(
            "SELECT ability_id, template_id, lifetime_effect_id, pulse_effect_id \
             FROM resources.deployables",
        )
        .fetch_all(&pool)
        .await
        .expect("deployables query");
        assert_eq!(rows.len(), catalog.len(), "the loader keeps every row");

        let mut bad = Vec::new();
        for (ability_id, template_id, lifetime, pulse) in rows {
            if !DEPLOYABLE_TEMPLATES.contains(&template_id) {
                bad.push(format!(
                    "{ability_id}: template {template_id} is outside 400-409"
                ));
            }
            match templates.get(&template_id) {
                None => bad.push(format!("{ability_id}: template {template_id} did not load")),
                Some(t) => {
                    if t.class != "being" {
                        bad.push(format!("{ability_id}: template class {:?}", t.class));
                    }
                    if t.loot_table_id.is_some() {
                        bad.push(format!("{ability_id}: template has a loot table"));
                    }
                }
            }
            match effects.get(&lifetime) {
                Some(e) if e.pulse_count >= 1 && e.pulse_duration > 0.0 => {}
                other => bad.push(format!(
                    "{ability_id}: lifetime effect {lifetime} does not pulse: {other:?}"
                )),
            }
            if !effects.contains_key(&pulse) {
                bad.push(format!("{ability_id}: pulse effect {pulse} did not load"));
            }
        }
        assert!(
            bad.is_empty(),
            "deployables rows that name a non-deployable template or a bad effect:\n{}",
            bad.join("\n")
        );
    }

    /// No deployable template is placed in `spawnlist`: only a cast places
    /// one, with an owner.
    #[tokio::test]
    async fn deployable_templates_are_never_placed_in_spawnlist_live_db() {
        let pool = require_db_or_skip!();
        let placed: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT spawn_id, template_id FROM resources.spawnlist \
             WHERE template_id BETWEEN 400 AND 409",
        )
        .fetch_all(&pool)
        .await
        .expect("spawnlist query");
        assert!(
            placed.is_empty(),
            "deployable templates placed in spawnlist: {placed:?}"
        );
    }

    /// The numbers the Microwave Emitter is built from, as the cell loads
    /// them: a 5 m range (1012 `MaxRange` 500 UE3 units), 30 pulses of 1 s
    /// (5065, "30 pulses x1 Second duration"), a 10 m radius (5066
    /// "Medium", no `Radius` NVP), 100 Focus damage and the Focus-first
    /// `RangedPhysicalDamage` script (5066 "Secondary -100F").
    #[tokio::test]
    async fn microwave_emitter_effects_carry_the_cooked_numbers_live_db() {
        let pool = require_db_or_skip!();
        let effects = load_effect_defs(&pool).await.expect("effects load");
        let abilities = load_ability_defs(&pool).await.expect("abilities load");

        let emitter = abilities.get(&MICROWAVE_EMITTER).expect("1012 loads");
        assert_eq!(emitter.max_range, 5.0, "1012: 500 UE3 units = 5 m");
        assert_eq!(emitter.target_type_id, 3, "1012 is ground-targeted");

        let pulser = effects.get(&PULSER).expect("5065 loads");
        assert_eq!(pulser.pulse_count, 30, "5065: 30 pulses");
        assert!(
            (pulser.pulse_duration - 1.0).abs() < 1e-6,
            "5065: 1 s apart, got {}",
            pulser.pulse_duration
        );

        let damage = effects.get(&DAMAGE).expect("5066 loads");
        assert_eq!(damage.tcm_param1, "Medium", "5066: Medium radius tier");
        assert_eq!(
            cimmeria_entity::abilities::ae_radius_metres(&damage.tcm_param1),
            Some(10.0),
            "the client's Medium AE radius"
        );
        assert_eq!(damage.param_f32("Radius"), 0.0, "5066 has no Radius NVP");
        assert_eq!(damage.param_i32("FocusDamage"), 100, "5066: -100F");
        assert_eq!(damage.param_i32("HealthDamage"), 0, "5066: no -H term");
        assert_eq!(
            damage.script_name.as_deref(),
            Some("RangedPhysicalDamage"),
            "5066 runs the Focus-first damage script"
        );
        assert_eq!(damage.pulse_count, 1, "5066 is one hit per pulse");
    }

    /// Template 400 wears a body set and a component the seed knows, and the
    /// component is authored for that body set. A typo here leaves the
    /// emitter invisible in the client.
    #[tokio::test]
    async fn microwave_emitter_body_is_a_seeded_deployable_body_live_db() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        let t = templates
            .get(&MICROWAVE_TEMPLATE)
            .expect("template 400 loads");
        assert_eq!(t.body_set, "WP-Human.BS_DeployableLow");
        let components = t.components.clone().unwrap_or_default();
        assert_eq!(components, vec!["WP-Human.Dp_Standard100".to_string()]);

        let (mesh,): (String,) =
            sqlx::query_as("SELECT ref_skeletal_mesh FROM resources.body_sets WHERE body_set = $1")
                .bind(&t.body_set)
                .fetch_one(&pool)
                .await
                .expect("the body set is seeded");
        assert_eq!(mesh, "DP-Base100");

        let (sets,): (Vec<String>,) = sqlx::query_as(
            "SELECT body_sets FROM resources.body_components WHERE component_name = $1",
        )
        .bind(&components[0])
        .fetch_one(&pool)
        .await
        .expect("the component is seeded");
        assert!(
            sets.iter().any(|s| s == "BS_DeployableLow"),
            "Dp_Standard100 must be authored for BS_DeployableLow, got {sets:?}"
        );
    }
}
