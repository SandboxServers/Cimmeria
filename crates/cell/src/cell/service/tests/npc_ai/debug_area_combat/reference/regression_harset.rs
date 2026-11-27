//! NC-24: regression pins for the Harset Jaffa (templates 159 and 160, spawned
//! in worlds 57 and 8) and the Jaffa pets (ability sets 351 and 352). The
//! pets gain the Staff Melee AA only through NC-12's side effect, so that is
//! pinned here on purpose. Pet selection at 2 m is NC-20's test and is not
//! repeated.

mod live_db {
    use crate::cell::spawner::SpawnRecord;
    use crate::cell::spawner::{load_ability_sets, load_effect_defs, load_spawns_from_db};
    use crate::test_support::require_db_or_skip;

    /// The Harset Jaffa templates, and the two worlds that spawn them.
    const HARSET_TEMPLATES: [i32; 2] = [159, 160];
    const HARSET_WORLDS: [i32; 2] = [57, 8];

    /// Every Harset Jaffa spawn in worlds 57 and 8 loads ability set 4 as
    /// [584, 710], faction 1 and the staff component `WP-Jaffa.WP_Staff_Plasma_4A`
    /// (D-NC3, NC-22: RECONSTRUCTED, not recovered original behaviour). Revert
    /// proof: changing set 4 or a faction, or dropping the staff component,
    /// fails here by name; dropping the spawn rows of either world fails the
    /// coverage check.
    #[tokio::test]
    async fn live_db_regression_harset_jaffa_profiles() {
        let pool = require_db_or_skip!();
        let sets = load_ability_sets(&pool).await.expect("ability sets load");
        assert_eq!(
            sets.get(&4).map(Vec::as_slice),
            Some(&[584, 710][..]),
            "ability set 4 is the Harset staff pair"
        );

        let spawns: Vec<(i32, i32, i32)> = sqlx::query_as(
            "SELECT spawn_id, world_id, template_id FROM resources.spawnlist \
             WHERE world_id = ANY($1) AND template_id = ANY($2) ORDER BY spawn_id",
        )
        .bind(HARSET_WORLDS.to_vec())
        .bind(HARSET_TEMPLATES.to_vec())
        .fetch_all(&pool)
        .await
        .expect("harset spawn query");
        for world in HARSET_WORLDS {
            assert!(
                spawns.iter().any(|(_, w, _)| *w == world),
                "world {world} spawns a Harset Jaffa"
            );
        }

        let spawn_ids: Vec<i32> = spawns.iter().map(|(id, _, _)| *id).collect();
        let rows: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| spawn_ids.contains(&r.spawn_id))
            .collect();
        assert_eq!(rows.len(), spawns.len(), "every Harset spawn loads");
        for r in &rows {
            let name = &r.template_name;
            assert!(
                HARSET_TEMPLATES.contains(&r.template_id),
                "{name} is a Harset template"
            );
            assert_eq!(r.ability_ids, vec![584, 710], "{name} loads set 4");
            assert_eq!(r.faction, Some(1), "{name} is faction 1");
            assert!(
                r.components
                    .iter()
                    .flatten()
                    .any(|c| c == "WP-Jaffa.WP_Staff_Plasma_4A"),
                "{name} carries the staff component"
            );
        }
    }

    /// The Jaffa pet sets 351 and 352 carry the Staff Melee AA (710) and its
    /// partner 584, and effect 736 carries its damage params (NC-12): the
    /// intended side effect, pinned. Revert proof: dropping 710 from either
    /// pet set, or putting `NULL` back as effect 736's script, fails here.
    #[tokio::test]
    async fn live_db_regression_jaffa_pet_sets_carry_staff() {
        let pool = require_db_or_skip!();
        let sets = load_ability_sets(&pool).await.expect("ability sets load");
        for set in [351, 352] {
            let abilities = sets.get(&set).map(Vec::as_slice).unwrap_or_default();
            assert!(
                abilities.contains(&584) && abilities.contains(&710),
                "pet set {set} carries the staff: {abilities:?}"
            );
        }

        let effects = load_effect_defs(&pool).await.expect("effect defs load");
        let def = &effects[&736];
        assert_eq!(
            def.script_name.as_deref(),
            Some("MeleePhysicalDamage"),
            "effect 736 is bound to the melee damage script"
        );
        assert_eq!(
            def.params.get("HealthDamage").map(String::as_str),
            Some("10"),
            "effect 736 HealthDamage"
        );
        assert_eq!(
            def.params.get("FocusDamage").map(String::as_str),
            Some("100"),
            "effect 736 FocusDamage"
        );
    }
}
