//! Guards on the two ability sets H11 allocates and H09 fills out, on the
//! composite primary key that lets a set hold more than one ability, and on the
//! join that carries them from the template through `load_spawns_from_db` to
//! the spawn record the AI tick actually reads.

use super::*;
use crate::cell::service::npc_ai::choose_npc_ability;

/// The two Harset ability sets hold exactly the abilities they were
/// allocated, in ascending-id order.
///
/// **Updated by packet H09**, which is the packet that made a multi-row set
/// possible at all. H11 wrote this test to assert exactly one row per set and
/// documented that as a schema constraint — `ability_set_abilities` then had
/// `PRIMARY KEY (ability_set_id)`, so a second row was a duplicate key that
/// aborted the whole seed load. H09 widened the key to `(ability_set_id,
/// ability_id)`, so the one-row assertion is gone and the expected membership
/// is the ranged/melee auto-attack pair `items_event_sets` binds to each
/// weapon.
///
/// It also asserts the property that *chose* these abilities: a non-NULL
/// `event_set_id`. `cell/abilities/use_ability/handle.rs` gates the whole
/// Ability_Begin/Ability_End `onSequence` broadcast on that field, so an
/// ability without one deals damage and plays no animation. That is why 594
/// Strike, 1482 Ground Blast and 1768 Double Blast — the three the H09 packet
/// text named — are all absent, along with 540 Staff Strike and 479 Staff
/// Blast. Neither packet owns the `abilities` seed, so the assertion is
/// deliberately narrow: it guards the one column the membership choice rests
/// on, nothing more.
#[tokio::test]
async fn harset_ability_sets_resolve_to_abilities_that_can_animate() {
    let pool = require_db_or_skip!();

    for (set_id, expected) in HARSET_ABILITY_SETS {
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
            expected.to_vec(),
            "ability set {set_id} must hold exactly {expected:?}; a single-element result \
             here means the H09 second rows are missing (or the composite primary key was \
             reverted, which would have failed the seed load first)"
        );

        for ability_id in expected {
            let event_set_id: Option<i32> = sqlx::query_scalar(
                "SELECT event_set_id FROM resources.abilities WHERE ability_id = $1",
            )
            .bind(ability_id)
            .fetch_one(&pool)
            .await
            .expect("the ability a Harset set points at must exist");

            assert!(
                event_set_id.is_some(),
                "ability {ability_id} (set {set_id}) has a NULL event_set_id — the \
                 Ability_Begin/Ability_End onSequence broadcast is skipped entirely and \
                 the NPC deals damage with no attack animation"
            );
        }
    }
}

/// **H09 key guard.** The primary key is `(ability_set_id, ability_id)`: it
/// accepts two different abilities on one set and rejects a repeat of the
/// same pair.
///
/// The accept half is the shipped seed — sets 4 and 5 each hold two rows, and
/// under the old single-column key `db/database.sql` could not have loaded at
/// all. The reject half is probed live, because the realistic future
/// regression is not "someone re-narrows the key" (that breaks the seed load
/// and every live-DB test at once, loudly) but "someone adds a surrogate id
/// column and drops the composite uniqueness", which is silent: duplicate
/// rows would fan out every loader's `array_agg` and hand the NPC the same
/// ability twice.
///
/// The probe asserts SQLSTATE `23505` on the named constraint rather than a
/// bare `is_err()` — `is_err()` would also pass on a `23503` FK violation,
/// which would make this test pass for a reason that has nothing to do with
/// the key.
///
/// Nothing is cleaned up because nothing is written: the duplicate INSERT
/// fails, and each `sqlx` statement against `&PgPool` runs in its own
/// implicit transaction, so the rejection poisons no later statement.
#[tokio::test]
async fn ability_set_primary_key_is_the_composite_pair() {
    let pool = require_db_or_skip!();

    let constraint: Option<String> = sqlx::query_scalar(
        "SELECT pg_get_constraintdef(oid) FROM pg_constraint \
         WHERE conrelid = 'resources.ability_set_abilities'::regclass AND contype = 'p'",
    )
    .fetch_optional(&pool)
    .await
    .expect("query must succeed");

    assert_eq!(
        constraint.as_deref(),
        Some("PRIMARY KEY (ability_set_id, ability_id)"),
        "ability_set_abilities must be keyed on the pair; on the single column every NPC \
         is capped at one ability, which is the defect H09 exists to fix"
    );

    // Accept: two different abilities on one set. Read back through the same
    // aggregate the three production loaders use.
    let set_four: Vec<i32> = sqlx::query_scalar(
        "SELECT ability_id FROM resources.ability_set_abilities \
         WHERE ability_set_id = 4 ORDER BY ability_id",
    )
    .fetch_all(&pool)
    .await
    .expect("query must succeed");
    assert_eq!(
        set_four,
        vec![STAFF_AUTO_ATTACK, STAFF_MELEE_AA],
        "set 4 must hold two distinct abilities — that is the accept half of the key"
    );

    // Reject: the same pair twice.
    let err = sqlx::query(
        "INSERT INTO resources.ability_set_abilities (ability_set_id, ability_id) \
         VALUES (4, $1)",
    )
    .bind(STAFF_MELEE_AA)
    .execute(&pool)
    .await
    .expect_err("re-inserting (4, 710) must violate the primary key");

    let db_err = err
        .as_database_error()
        .expect("a duplicate key is a database error, not a client/protocol error");
    assert_eq!(
        db_err.code().as_deref(),
        Some("23505"),
        "expected unique_violation, got {db_err:?}"
    );
    assert_eq!(
        db_err.constraint(),
        Some("ability_set_abilities_pkey"),
        "the violation must come from the primary key itself, not some other unique index"
    );

    // And the rejection wrote nothing.
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.ability_set_abilities WHERE ability_set_id = 4",
    )
    .fetch_one(&pool)
    .await
    .expect("query must succeed");
    assert_eq!(after, 2, "the rejected INSERT must not have added a row");
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
        vec![STAFF_AUTO_ATTACK, STAFF_MELEE_AA],
        "the template's ability set must reach the spawn record *in full*; an empty bucket \
         here is exactly the state that makes `choose_npc_ability` fall back to \
         NPC_DEFAULT_ABILITY ({NPC_DEFAULT_ABILITY}), and a one-element bucket means the \
         loader's correlated `array_agg` is only returning the first row — the H09 second \
         rows would be seeded but unreachable at runtime"
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

/// **H09 acceptance.** A three-row ability set survives the whole chain —
/// `ability_set_abilities` → `load_spawn_templates`'s correlated `array_agg`
/// → `SpawnRecord.ability_ids` → `spawn_npc_from_record` → the NPC's ability
/// bucket → `choose_npc_ability` — and the selector can reach *every* member,
/// not just the first.
///
/// Why a sentinel set rather than the shipped set 4: padding a production set
/// to three rows would put a third ability on every live Harset Jaffa and
/// change their attack rate a second time. The sentinel exercises the code
/// path without touching content.
///
/// **The three rows are inserted out of ascending order (712, 584, 710) on
/// purpose.** Inserting them ascending would let this test pass even if
/// someone deleted the `sort_unstable()` in `choose_npc_ability`, because
/// physical row order would already be the answer. With the insert order
/// scrambled, the assertion that the loader returns `[584, 710, 712]` and
/// that the selector walks them in that order is a real guard on the sort.
///
/// The ability ids are real (584/710/712) rather than invented sentinels
/// because `ability_set_abilities_ability_id_fkey` is `ON DELETE RESTRICT` —
/// unlike `SENTINEL_SPAWN_ID`, an ability id cannot be conjured.
///
/// Insert → load → delete → assert, so a failing assertion cannot leave the
/// sentinel set or template registered in the shared test DB. Cleanup runs
/// child-before-parent (`ability_set_abilities`, then the template that
/// references the set, then `ability_sets`) because both FKs are RESTRICT.
#[tokio::test]
async fn multi_ability_set_reaches_the_chooser_and_every_member_is_selectable() {
    let pool = require_db_or_skip!();

    clean_sentinel_ability_set(&pool).await;

    sqlx::query("INSERT INTO resources.ability_sets (ability_set_id, description) VALUES ($1, $2)")
        .bind(SENTINEL_ABILITY_SET_ID)
        .bind("H09 multi-ability chooser probe")
        .execute(&pool)
        .await
        .expect("sentinel ability set insert must succeed");

    // Deliberately NOT ascending — see the doc comment.
    for ability_id in [RIBBON_AUTO_ATTACK, STAFF_AUTO_ATTACK, STAFF_MELEE_AA] {
        sqlx::query(
            "INSERT INTO resources.ability_set_abilities (ability_set_id, ability_id) \
             VALUES ($1, $2)",
        )
        .bind(SENTINEL_ABILITY_SET_ID)
        .bind(ability_id)
        .execute(&pool)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "inserting ability {ability_id} into the sentinel set must succeed — a \
                 duplicate-key error here means the primary key is still the single \
                 column `(ability_set_id)` and H09's widen did not reach this database: {e}"
            )
        });
    }

    sqlx::query(
        "INSERT INTO resources.entity_templates \
             (template_id, template_name, class, body_set, static_mesh, flags, \
              interaction_type, level, alignment, faction, static_interaction_sets, \
              has_dynamic_properties, ability_set_id) \
         VALUES ($1, 'H09 Multi-Ability Probe', 'mob', 'GLB_Components.WorldObject_Small', \
                 'Props.TestMesh', 0, 0, 5, 0, 10, ARRAY[]::integer[], false, $2)",
    )
    .bind(SENTINEL_TEMPLATE_ID)
    .bind(SENTINEL_ABILITY_SET_ID)
    .execute(&pool)
    .await
    .expect("sentinel template insert must succeed");

    let loaded = load_spawn_templates(&pool).await;

    clean_sentinel_ability_set(&pool).await;

    let templates = loaded.expect("load_spawn_templates must succeed");
    let prototype = templates
        .get(&SENTINEL_TEMPLATE_ID)
        .expect("the sentinel template must surface from the loader");

    assert_eq!(
        prototype.ability_ids,
        SENTINEL_SET_ABILITIES.to_vec(),
        "a three-row ability set must load as three abilities in ascending id order; \
         one element means the loader's `array_agg` is collapsing the set"
    );

    // ── Loader output → live NPC → selector ──────────────────────────────
    let mut mgr = crate::test_support::make_space_manager();
    let mut record = prototype.clone();
    // `load_spawn_templates` returns prototypes with the spawn-instance
    // fields blank; the caller supplies placement. Agnos is the startup
    // space `make_space_manager` creates.
    record.world_name = "Agnos".to_string();

    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc_id, &record)
        .expect("sentinel NPC must spawn");

    let npc = mgr.get_entity(npc_id).expect("spawned NPC must exist");
    assert_eq!(
        npc.abilities.known_count(),
        SENTINEL_SET_ABILITIES.len(),
        "all three abilities must land in the bucket; the spawn path adds \
         NPC_DEFAULT_ABILITY ({NPC_DEFAULT_ABILITY}) only when `ability_ids` is empty, so a \
         count of 1 here would mean the set collapsed somewhere upstream"
    );
    for ability_id in SENTINEL_SET_ABILITIES {
        assert!(
            npc.abilities.has_ability(ability_id),
            "ability {ability_id} must be in the spawned NPC's bucket"
        );
    }

    // Walk the bucket by stacking cooldowns: each pass must yield the next
    // ability up, which is only true if the selector iterates the whole set.
    for (n, expected) in SENTINEL_SET_ABILITIES.iter().enumerate() {
        let chosen = choose_npc_ability(npc_id, &mgr);
        assert_eq!(
            chosen,
            Some(*expected),
            "with the {n} lowest-id abilities cooling, the selector must pick {expected}; \
             a selector that only ever reads the first member of the set would return \
             {} every time",
            SENTINEL_SET_ABILITIES[0]
        );
        // Cool the one just chosen so the next pass has to move on.
        mgr.get_entity_mut(npc_id)
            .expect("NPC must still exist")
            .abilities
            .start_ability_cooldown(*expected, std::time::Duration::from_secs(60));
    }

    // Every member cooling → hold fire rather than re-firing a cooling ability.
    assert_eq!(
        choose_npc_ability(npc_id, &mgr),
        None,
        "with all three cooling the selector must hold fire; a `Some` here means a cooling \
         ability leaked back into the usable bucket"
    );
}

/// **H09 melee-reach acceptance, end to end against the seed.** Both Harset
/// sets, loaded out of Postgres with the real `resources.abilities` rows,
/// spawned onto a real NPC, and driven through the *production* selector — the
/// reach-filtered `choose_npc_ability_within_reach` that `npc_ai_fight` calls.
///
/// The unit guards in `cell/service/tests/npc_ai/melee_reach.rs` hand-seed the
/// four `AbilityDef`s, so they prove the gate's logic but not that the seed
/// actually carries the values the gate reads. This one closes that gap: if
/// someone flips `abilities.is_ranged` on 710 or 711, or gives one of them a
/// non-zero `max_range`, the unit guards keep passing and this one fails.
///
/// The two sets are deliberately asserted together because they sort
/// oppositely. Set 4 is `584` (ranged) then `710` (melee), so the lowest-id
/// walk would have picked correctly at range *by accident*; set 5 is `711`
/// (melee) then `712` (ranged), so only the reach filter gets it right.
/// Asserting set 5 alone would leave the accident untested, and set 4 alone
/// would pass with no filter at all.
///
/// `BEYOND_MELEE` (20 m) sits between `NPC_MELEE_RANGE` (3) and
/// `NPC_ATTACK_RANGE` (30) so the two gates disagree and the assertion can
/// tell which one ran.
#[tokio::test]
async fn seeded_harset_sets_never_select_their_melee_half_at_range() {
    use crate::cell::combat::{NPC_ATTACK_RANGE, NPC_MELEE_RANGE};
    use crate::cell::service::npc_ai::choose_npc_ability_within_reach;
    use crate::cell::spawner::load_ability_defs;

    const BEYOND_MELEE: f32 = 20.0;
    const WITHIN_MELEE: f32 = 2.0;

    let pool = require_db_or_skip!();

    let defs = load_ability_defs(&pool)
        .await
        .expect("ability defs must load");

    // Sanity-check the columns the gate reads before relying on them, so a
    // seed change surfaces here rather than as a confusing selector result.
    for (ability_id, expect_ranged) in [
        (STAFF_AUTO_ATTACK, true),
        (STAFF_MELEE_AA, false),
        (RIBBON_MELEE_AA, false),
        (RIBBON_AUTO_ATTACK, true),
    ] {
        let def = defs
            .get(&ability_id)
            .unwrap_or_else(|| panic!("ability {ability_id} must exist in resources.abilities"));
        assert_eq!(
            def.is_ranged, expect_ranged,
            "ability {ability_id} ({}) must have is_ranged = {expect_ranged}; the melee \
             reach gate keys off this column alone",
            def.name
        );
        assert_eq!(
            def.max_range, 0,
            "ability {ability_id} ({}) must keep the `0` max_range sentinel — a non-zero \
             value here overrides the server default and, on a melee ability, would put \
             the swing back out at whatever number the seed carries",
            def.name
        );
    }

    let templates = load_spawn_templates(&pool)
        .await
        .expect("spawn templates must load");

    for (set_id, members, template_id) in HARSET_SET_PROBE_TEMPLATES {
        // A real seeded template, not a hand-built record: this also pins that
        // the template still points at the set it is supposed to.
        let prototype = templates
            .get(&template_id)
            .unwrap_or_else(|| panic!("template {template_id} must surface from the loader"));
        assert_eq!(
            prototype.ability_ids,
            members.to_vec(),
            "template {template_id} must still carry ability set {set_id} in full"
        );

        let mut mgr = crate::test_support::make_space_manager();
        mgr.ability_defs = defs.clone();

        let mut record = prototype.clone();
        // `load_spawn_templates` returns prototypes with the spawn-instance
        // fields blank; the caller supplies placement. Agnos is the startup
        // space `make_space_manager` creates.
        record.world_name = "Agnos".to_string();

        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(npc_id, &record)
            .expect("probe NPC must spawn");

        let ranged_half = members
            .iter()
            .copied()
            .find(|id| defs[id].is_ranged)
            .expect("each Harset set holds one ranged auto-attack");
        let melee_half = members
            .iter()
            .copied()
            .find(|id| !defs[id].is_ranged)
            .expect("each Harset set holds one melee auto-attack");

        assert_eq!(
            choose_npc_ability_within_reach(npc_id, &mgr, BEYOND_MELEE, NPC_ATTACK_RANGE),
            Some(ranged_half),
            "set {set_id}: at {BEYOND_MELEE} m the selector must take the ranged half \
             ({ranged_half}); {melee_half} here means the melee reach gate is gone and the \
             NPC plays a weapon swing at a target {BEYOND_MELEE} m away"
        );

        // The melee half is not blacklisted, just range-gated: inside
        // NPC_MELEE_RANGE the lowest-id member wins again, which for set 5 is
        // the melee one.
        let expected_up_close = members.iter().copied().min().expect("set is non-empty");
        assert_eq!(
            choose_npc_ability_within_reach(npc_id, &mgr, WITHIN_MELEE, NPC_ATTACK_RANGE),
            Some(expected_up_close),
            "set {set_id}: inside {NPC_MELEE_RANGE} m both members are usable, so the \
             lowest id ({expected_up_close}) wins — a gate that rejected melee \
             unconditionally would return {ranged_half} here"
        );

        // With the ranged half cooling and the target out of reach, nothing is
        // usable — the selector must still hand back the melee ability so the
        // fight tick walks the NPC in. `None` would read as "all cooling".
        mgr.get_entity_mut(npc_id)
            .expect("probe NPC must exist")
            .abilities
            .start_ability_cooldown(ranged_half, std::time::Duration::from_secs(60));
        assert_eq!(
            choose_npc_ability_within_reach(npc_id, &mgr, BEYOND_MELEE, NPC_ATTACK_RANGE),
            Some(melee_half),
            "set {set_id}: with {ranged_half} cooling and nothing in reach, the selector \
             must fall back to {melee_half} so the NPC closes the distance rather than \
             freezing"
        );
    }
}

/// Delete the H09 sentinel rows by exact id, child-before-parent.
///
/// Order is load-bearing: `entity_templates_ability_set_id_fkey` and
/// `ability_set_abilities_ability_set_id_fkey` are both `ON DELETE RESTRICT`,
/// so `ability_sets` cannot go until nothing references it. Called before the
/// inserts too, so a previous aborted run cannot wedge the next one.
async fn clean_sentinel_ability_set(pool: &sqlx::PgPool) {
    sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
        .bind(SENTINEL_TEMPLATE_ID)
        .execute(pool)
        .await
        .expect("sentinel template cleanup must succeed");
    sqlx::query("DELETE FROM resources.ability_set_abilities WHERE ability_set_id = $1")
        .bind(SENTINEL_ABILITY_SET_ID)
        .execute(pool)
        .await
        .expect("sentinel ability-set-abilities cleanup must succeed");
    sqlx::query("DELETE FROM resources.ability_sets WHERE ability_set_id = $1")
        .bind(SENTINEL_ABILITY_SET_ID)
        .execute(pool)
        .await
        .expect("sentinel ability set cleanup must succeed");
}
