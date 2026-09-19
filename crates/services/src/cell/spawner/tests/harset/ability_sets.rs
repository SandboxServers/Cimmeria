//! Guards on the two ability sets H11 allocates, and on the join that carries
//! them from the template through `load_spawns_from_db` to the spawn record the
//! AI tick actually reads.

use super::*;

/// The two new ability sets exist and hold exactly the ability they were
/// allocated for.
///
/// One ability per set is a schema constraint, not a style choice:
/// `ability_set_abilities` has `PRIMARY KEY (ability_set_id)`, so a second
/// row for the same set is a duplicate key that aborts the whole seed load.
/// This test pins the shape so a future packet that tries to add variety
/// discovers the constraint here rather than in a failed DB reload.
///
/// It also asserts the property that *chose* these two abilities: a
/// non-NULL `event_set_id`. `cell/abilities/use_ability/handle.rs` gates the
/// whole Ability_Begin/Ability_End `onSequence` broadcast on that field, so
/// an ability without one deals damage and plays no animation. That is why
/// 594 Strike, 540 Staff Strike, 479 Staff Blast, 1482 Ground Blast and 1768
/// Double Blast were all rejected. This packet does not own the `abilities`
/// seed, so the assertion is deliberately narrow — it guards the one column
/// the ability-set choice rests on, nothing more.
#[tokio::test]
async fn harset_ability_sets_resolve_to_an_ability_that_can_animate() {
    let pool = require_db_or_skip!();

    for (set_id, expected_ability) in [(4_i32, STAFF_AUTO_ATTACK), (5, RIBBON_AUTO_ATTACK)] {
        let abilities: Vec<i32> = sqlx::query_scalar(
            "SELECT ability_id FROM resources.ability_set_abilities \
             WHERE ability_set_id = $1 ORDER BY ability_id",
        )
        .bind(set_id)
        .fetch_all(&pool)
        .await
        .expect("query must succeed");

        assert_eq!(
            abilities,
            vec![expected_ability],
            "ability set {set_id} must hold exactly [{expected_ability}]"
        );

        let event_set_id: Option<i32> = sqlx::query_scalar(
            "SELECT event_set_id FROM resources.abilities WHERE ability_id = $1",
        )
        .bind(expected_ability)
        .fetch_one(&pool)
        .await
        .expect("the ability an H11 set points at must exist");

        assert!(
            event_set_id.is_some(),
            "ability {expected_ability} (set {set_id}) has a NULL event_set_id — the \
             Ability_Begin/Ability_End onSequence broadcast is skipped entirely and the \
             NPC deals damage with no attack animation"
        );
    }
}

/// **Loader round-trip.** The template block has no spawn rows yet (H12/H13/
/// H14 own `spawnlist.sql`), so the assertions above can only see the table.
/// This one pins the whole join: a spawn row pointed at template 200 must
/// surface from `load_spawns_from_db` carrying the staff ability and the
/// respawn delay, which is what the runtime actually consumes.
///
/// Insert → load → delete by exact id → assert, so a failing assertion
/// cannot leave a live spawn registered in the shared test DB (the same
/// shape `chain_replay_tests/grant_xp.rs` uses for sentinel chains).
#[tokio::test]
async fn harset_templates_resolve_abilities_and_respawn_through_the_spawn_loader() {
    let pool = require_db_or_skip!();

    // Defensive: a previous aborted run may have leaked the sentinel.
    sqlx::query("DELETE FROM resources.spawnlist WHERE spawn_id = $1")
        .bind(SENTINEL_SPAWN_ID)
        .execute(&pool)
        .await
        .expect("sentinel pre-clean must succeed");

    sqlx::query(
        "INSERT INTO resources.spawnlist \
             (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) \
         VALUES ($1, 0, 0, 0, 0, $2, 200, 'H11_LoaderProbe', NULL)",
    )
    .bind(SENTINEL_SPAWN_ID)
    .bind(HARSET_WORLD_ID)
    .execute(&pool)
    .await
    .expect("sentinel spawn insert must succeed");

    let loaded = load_spawns_from_db(&pool).await;

    sqlx::query("DELETE FROM resources.spawnlist WHERE spawn_id = $1")
        .bind(SENTINEL_SPAWN_ID)
        .execute(&pool)
        .await
        .expect("sentinel cleanup must succeed");

    let records = loaded.expect("load_spawns_from_db must succeed");
    let probe = records
        .iter()
        .find(|r| r.spawn_id == SENTINEL_SPAWN_ID)
        .expect("sentinel spawn must surface from the loader");

    assert_eq!(probe.template_name, "Mala'c");
    assert_eq!(probe.world_name, "Harset");
    assert_eq!(
        probe.ability_ids,
        vec![STAFF_AUTO_ATTACK],
        "the template's ability set must reach the spawn record; an empty bucket here is \
         exactly the state that makes `choose_npc_ability` fall back to \
         NPC_DEFAULT_ABILITY ({NPC_DEFAULT_ABILITY})"
    );
    assert_eq!(
        probe.respawn_secs,
        Some(RESPAWN_DEFAULT as u32),
        "the spawn row leaves respawn_secs NULL, so COALESCE must pick up the template \
         default"
    );
    assert!(
        probe.loot_table_id.is_none(),
        "spec L-01: no Harset template carries a loot table"
    );
}
