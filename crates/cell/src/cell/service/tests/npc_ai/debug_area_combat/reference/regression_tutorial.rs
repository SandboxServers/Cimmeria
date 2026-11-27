//! NC-23: the Castle_CellBlock combat profiles (world 12) that must not change
//! while the NPC combat reference is reworked. The NID Guard (24) and the PRU
//! (4) keep their explicit sets, the set-less mobs keep the Pistol Shot
//! placeholder under `Legacy`, and the PRU's scripted activation still loads.
//! These are pins: a failure is a finding, not a fix
//! (`docs/analysis/npc-combat-reference/work-packets-w1-w2.md` NC-23).

mod live_db {
    use cimmeria_cell_world::cell::combat::combat_profile::{
        resolve_npc_combat_profile, CombatProfileSource, PlaceholderPolicy,
    };
    use sqlx::PgPool;

    use crate::cell::content::load_single_chain_for_test;
    use crate::cell::spawner::{load_spawns_from_db, SpawnRecord};
    use crate::test_support::require_db_or_skip;

    /// The CellBlock world id in `resources.worlds`.
    const CELLBLOCK_WORLD_ID: i32 = 12;
    /// The PRU's scripted-activation tag (`castle_cellblock_chains.sql`).
    const PRU_TAG: &str = "ArmYourself_PrisonerRetrievalUnit";
    /// The chains that drive the PRU: 1032 (vial: aggro and threat), 1033 and
    /// 1131 (guard killed), 1037 (zone-load re-arm).
    const PRU_CHAINS: [i32; 4] = [1032, 1033, 1037, 1131];

    /// The world name of world 12, read from its spawn rows, and the spawns in
    /// that world. The name is never hardcoded.
    async fn cellblock_rows(pool: &PgPool) -> (String, Vec<SpawnRecord>) {
        let names: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT w.world FROM resources.spawnlist s \
             JOIN resources.worlds w ON s.world_id = w.world_id \
             WHERE s.world_id = $1",
        )
        .bind(CELLBLOCK_WORLD_ID)
        .fetch_all(pool)
        .await
        .expect("world 12 spawn rows load");
        assert_eq!(
            names.len(),
            1,
            "world 12 spawn rows name one world: {names:?}"
        );
        let world = names[0].clone();
        let rows = load_spawns_from_db(pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| r.world_name == world)
            .collect();
        (world, rows)
    }

    /// Every CellBlock NID Guard (24) and PRU (4) spawn loads with its seeded
    /// set, [559] and [221], and resolves `ExplicitSet` under both placeholder
    /// policies. Revert proof: dropping 559 from template 24 or 221 from
    /// template 4 fails here.
    #[tokio::test]
    async fn live_db_regression_cellblock_profiles_unchanged() {
        let pool = require_db_or_skip!();
        let (world, rows) = cellblock_rows(&pool).await;
        for (template_id, abilities) in [(24, vec![559]), (4, vec![221])] {
            let spawns: Vec<&SpawnRecord> = rows
                .iter()
                .filter(|r| r.template_id == template_id)
                .collect();
            assert!(
                !spawns.is_empty(),
                "{world} has spawns of template {template_id}"
            );
            for r in spawns {
                assert_eq!(
                    r.ability_ids, abilities,
                    "spawn {} ({}) ability set",
                    r.spawn_id, r.template_name
                );
                for policy in [
                    PlaceholderPolicy::Legacy,
                    PlaceholderPolicy::RequireWeaponEvidence,
                ] {
                    let p =
                        resolve_npc_combat_profile(&r.ability_ids, r.components.as_deref(), policy);
                    assert_eq!(
                        p.source,
                        CombatProfileSource::ExplicitSet,
                        "spawn {} under {policy:?}",
                        r.spawn_id
                    );
                    assert_eq!(
                        p.abilities, abilities,
                        "spawn {} under {policy:?}",
                        r.spawn_id
                    );
                }
            }
        }
    }

    /// Every world-12 spawn whose template has no ability set resolves
    /// `LegacyPlaceholder` [592] under `Legacy`. Its verdict under
    /// `RequireWeaponEvidence` is printed per template so NC-30 sees the
    /// impact. Revert proof: a `Legacy` arm that stops returning [592] fails
    /// here by spawn and template.
    #[tokio::test]
    async fn live_db_regression_tutorial_no_set_mobs_keep_firing_under_legacy() {
        let pool = require_db_or_skip!();
        let (world, rows) = cellblock_rows(&pool).await;
        for r in rows.iter().filter(|r| r.ability_ids.is_empty()) {
            let components = r.components.as_deref();
            let legacy = resolve_npc_combat_profile(&[], components, PlaceholderPolicy::Legacy);
            assert_eq!(
                legacy.source,
                CombatProfileSource::LegacyPlaceholder,
                "spawn {} (template {}, {}) in {world} under Legacy",
                r.spawn_id,
                r.template_id,
                r.template_name
            );
            assert_eq!(legacy.abilities, vec![592], "spawn {}", r.spawn_id);
            let strict = resolve_npc_combat_profile(
                &[],
                components,
                PlaceholderPolicy::RequireWeaponEvidence,
            );
            eprintln!(
                "NC-23 {world} template {} ({}) spawn {}: RequireWeaponEvidence -> {:?} {:?}, evidence {:?}",
                r.template_id,
                r.template_name,
                r.spawn_id,
                strict.source,
                strict.abilities,
                strict.weapon_evidence
            );
        }
    }

    /// The CellBlock PRU spawn (template 4, tagged `PRU_TAG`) is in world 12,
    /// and the chains that drive its scripted activation still load and name
    /// it. Revert proof: deleting the spawn row, or any chain row that names the
    /// PRU, fails here.
    #[tokio::test]
    async fn live_db_regression_pru_cellblock_activation_rows_intact() {
        let pool = require_db_or_skip!();
        let (world, rows) = cellblock_rows(&pool).await;
        let pru = rows
            .iter()
            .find(|r| r.tag.as_deref() == Some(PRU_TAG))
            .unwrap_or_else(|| panic!("{world} has no {PRU_TAG} spawn"));
        assert_eq!(pru.template_id, 4, "the CellBlock PRU is template 4");
        for id in PRU_CHAINS {
            let chain = load_single_chain_for_test(&pool, id)
                .await
                .expect("chain query must succeed")
                .unwrap_or_else(|| panic!("chain {id} loads"));
            assert!(
                format!("{chain:?}").contains(PRU_TAG),
                "chain {id} names {PRU_TAG}"
            );
        }
    }
}
