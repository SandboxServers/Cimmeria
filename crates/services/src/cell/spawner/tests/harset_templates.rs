//! Live-DB regression guards for the Harset entity templates seeded by packet
//! H11 (`docs/analysis/harset-rebuild/work-packets.md`).
//!
//! These are seed-data guards, not fixture tests: the rows under test are the
//! production seed in `db/resources/Entities/Seed/entity_templates.sql` and
//! `db/resources/Abilities/Seed/ability_set*.sql`. Every test here fails if the
//! H11 rows are removed — the counts and the per-id assertions are the point.
//!
//! The three defects they pin, all from
//! `docs/analysis/harset-rebuild/audit.md`:
//!
//! - **H-B7** — `respawn_secs` was NULL on all 153 templates and all 167 spawn
//!   rows, so `mark_npc_dead` never stamped `respawn_at` and clearing the Harset
//!   plaza emptied it until server restart.
//! - **H-B8** — `ability_set_id` was set on 3 of 153 templates; every other NPC
//!   fell back to `NPC_DEFAULT_ABILITY` (592, Pistol Shot), so Jaffa with staff
//!   models fired a Tau'ri pistol. Template 163 (Petbe) additionally shipped
//!   with NULL faction, level and alignment.
//! - **spec L-01** — Harset has no random loot; `loot_table_id` must stay NULL
//!   across the whole zone roster.

mod harset_templates {
    use crate::cell::combat::NPC_DEFAULT_ABILITY;
    use crate::cell::spawner::load_spawns_from_db;
    use crate::test_support::require_db_or_skip;

    /// Template-id block the packet ledger reserves for Harset.
    const BLOCK_MIN: i32 = 200;
    const BLOCK_MAX: i32 = 299;

    /// The documented per-template respawn default for this packet (D-H17).
    const RESPAWN_DEFAULT: i32 = 300;

    /// Ability 584 "Staff Auto Attack" — the single member of ability set 4.
    const STAFF_AUTO_ATTACK: i32 = 584;
    /// Ability 712 "Ribbon Device Auto Attack" — the single member of set 5.
    const RIBBON_AUTO_ATTACK: i32 = 712;

    /// Every `class = 'mob'` template H11 seeds, as `(template_id, name)`.
    const MOB_TEMPLATES: [(i32, &str); 24] = [
        (200, "Mala'c"),
        (201, "Lo'rak"),
        (202, "Bra'hin"),
        (203, "Ra's Jaffa Infiltrator"),
        (204, "Ra's Former Jaffa"),
        (205, "Suspicious Jaffa"),
        (206, "Angry Jaffa"),
        (207, "Jaffa Volunteer"),
        (208, "Free Jaffa Attacker"),
        (209, "Anat's Royal Guard"),
        (210, "Haughty Goa'uld"),
        (211, "Ashrak Assassin"),
        (212, "Hansen"),
        (213, "Jacobs"),
        (214, "Blackstock"),
        (215, "Opheltes"),
        (216, "Lance Corporal Grogan"),
        (217, "Dawson"),
        (218, "NID Operative"),
        (219, "Storage Lo'taur"),
        (220, "Lethander's Contact"),
        (221, "Petbe (hostile)"),
        (222, "Lance Corporal Grogan (hostile)"),
        (223, "Dawson (hostile)"),
    ];

    /// Every prop template H11 seeds. Props are `class = 'being'`: they never
    /// AI-tick (`all_npc_entity_ids` admits only class_id `0x04`) so they carry
    /// no ability set and no respawn.
    const PROP_TEMPLATES: [(i32, &str); 9] = [
        (240, "Harset Camera Location"),
        (241, "Replitech Crate"),
        (242, "Harset Storage Crate"),
        (243, "Harset Shield Tower"),
        (244, "Petbe's Quarters Search Object"),
        (245, "Anat's Symbiote Tank"),
        (246, "Strange Beacon Technology"),
        (247, "Devlin's Device"),
        (248, "Harset Monitoring Device Anchor"),
    ];

    /// Harset templates that predate this packet: Ba'al, Anat, Lethander,
    /// CaptCoppleman, Nerus, Moh'Katan, the two Praxis guard rows, Petbe and the
    /// mission-742 merchant basket. Folded into the loot sweep so spec L-01 is
    /// asserted across the whole zone roster, not just the new rows.
    const PREEXISTING_HARSET_TEMPLATES: [i32; 10] = [42, 43, 46, 48, 53, 54, 159, 160, 163, 164];

    /// Sentinel spawn id for the loader round-trip. Sits well clear of every
    /// `0x7000_xxxx` base already in use by `crates/services` (highest today is
    /// `0x7000_5000`). Deleted by exact id before any assertion runs.
    const SENTINEL_SPAWN_ID: i32 = 0x7000_6100;

    /// World 57 = Harset, confirmed against `resources.worlds`.
    const HARSET_WORLD_ID: i32 = 57;

    /// **H-B8 guard.** Every H11 mob template must resolve to a real ability
    /// set, and that set must not be the pistol fallback.
    ///
    /// Reverting the seed fails this on the count assertion; pointing a row at
    /// an ability set with no `ability_set_abilities` row fails it on the
    /// per-row check (that shape is invisible to a FK — the set would exist,
    /// the join would return nothing, and `load_spawns_from_db`'s
    /// `COALESCE(..., ARRAY[]::int[])` would hand the spawn path an empty bucket
    /// that falls straight back to `NPC_DEFAULT_ABILITY`).
    #[tokio::test]
    async fn every_harset_mob_template_carries_a_non_default_ability_set() {
        let pool = require_db_or_skip!();

        let rows: Vec<(i32, String, Option<i32>, Option<i32>)> = sqlx::query_as(
            "SELECT t.template_id, t.template_name, t.ability_set_id, asa.ability_id \
             FROM resources.entity_templates t \
             LEFT JOIN resources.ability_set_abilities asa \
                    ON asa.ability_set_id = t.ability_set_id \
             WHERE t.template_id BETWEEN $1 AND $2 AND t.class = 'mob' \
             ORDER BY t.template_id",
        )
        .bind(BLOCK_MIN)
        .bind(BLOCK_MAX)
        .fetch_all(&pool)
        .await
        .expect("query must succeed");

        assert_eq!(
            rows.len(),
            MOB_TEMPLATES.len(),
            "expected exactly {} Harset mob templates in {BLOCK_MIN}-{BLOCK_MAX}, found {}; \
             the H11 seed rows are missing or a row changed class",
            MOB_TEMPLATES.len(),
            rows.len()
        );

        for (template_id, template_name, ability_set_id, ability_id) in &rows {
            assert!(
                ability_set_id.is_some(),
                "template {template_id} ({template_name}) has a NULL ability_set_id — it will \
                 fall back to NPC_DEFAULT_ABILITY ({NPC_DEFAULT_ABILITY}, Pistol Shot). That is \
                 defect H-B8, the whole point of this packet."
            );
            let ability_id = ability_id.unwrap_or_else(|| {
                panic!(
                    "template {template_id} ({template_name}) points at ability set {:?}, but \
                     that set has no row in ability_set_abilities — the join yields an empty \
                     ability bucket and the spawn path silently falls back to \
                     NPC_DEFAULT_ABILITY",
                    ability_set_id
                )
            });
            assert_ne!(
                ability_id, NPC_DEFAULT_ABILITY,
                "template {template_id} ({template_name}) resolves to the pistol fallback \
                 ability {NPC_DEFAULT_ABILITY} through an explicit ability set — that defeats \
                 the H-B8 fix"
            );
        }
    }

    /// **H-B7 guard.** Every H11 mob template carries a respawn delay, so a
    /// killed Harset NPC repopulates instead of leaving the zone permanently
    /// empty.
    ///
    /// The `>= 3` floor mirrors the `entity_templates_respawn_secs_min_3` CHECK;
    /// asserting it here catches a future migration that relaxes the constraint
    /// as well as a seed edit that drops to 0 (which
    /// `normalize_respawn_secs` would downgrade back to `None`).
    #[tokio::test]
    async fn every_harset_mob_template_carries_a_respawn_delay() {
        let pool = require_db_or_skip!();

        let rows: Vec<(i32, String, Option<i32>)> = sqlx::query_as(
            "SELECT template_id, template_name, respawn_secs \
             FROM resources.entity_templates \
             WHERE template_id BETWEEN $1 AND $2 AND class = 'mob' \
             ORDER BY template_id",
        )
        .bind(BLOCK_MIN)
        .bind(BLOCK_MAX)
        .fetch_all(&pool)
        .await
        .expect("query must succeed");

        assert_eq!(
            rows.len(),
            MOB_TEMPLATES.len(),
            "expected exactly {} Harset mob templates, found {}",
            MOB_TEMPLATES.len(),
            rows.len()
        );

        for (template_id, template_name, respawn_secs) in &rows {
            let secs = respawn_secs.unwrap_or_else(|| {
                panic!(
                    "template {template_id} ({template_name}) has a NULL respawn_secs — with a \
                     NULL on the spawn row too, `mark_npc_dead` never stamps `respawn_at` and \
                     the mob is one-shot (defect H-B7)"
                )
            });
            assert!(
                secs >= 3,
                "template {template_id} ({template_name}) has respawn_secs = {secs}; values \
                 below 3 are rejected by the entity_templates_respawn_secs_min_3 CHECK and \
                 downgraded to None by normalize_respawn_secs"
            );
        }
    }

    /// **Spec L-01.** Harset has no random loot. Asserted across the whole zone
    /// roster — the new 200-299 block plus the ten templates that predate it —
    /// so a future packet cannot attach a loot table to Ba'al or the plaza
    /// guards without tripping this.
    ///
    /// The row-count assertion is what stops this going vacuous: without it, a
    /// revert that deletes every H11 row would leave the sweep scanning ten rows
    /// and still passing.
    #[tokio::test]
    async fn no_harset_template_carries_a_loot_table() {
        let pool = require_db_or_skip!();

        let expected_roster = (MOB_TEMPLATES.len()
            + PROP_TEMPLATES.len()
            + PREEXISTING_HARSET_TEMPLATES.len()) as i64;

        let (roster_size, with_loot): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(*) FILTER (WHERE loot_table_id IS NOT NULL) \
             FROM resources.entity_templates \
             WHERE template_id BETWEEN $1 AND $2 OR template_id = ANY($3)",
        )
        .bind(BLOCK_MIN)
        .bind(BLOCK_MAX)
        .bind(PREEXISTING_HARSET_TEMPLATES.as_slice())
        .fetch_one(&pool)
        .await
        .expect("query must succeed");

        assert_eq!(
            roster_size, expected_roster,
            "Harset template roster is {roster_size} rows, expected {expected_roster} — the \
             H11 seed rows are missing, so the loot sweep below would pass vacuously"
        );
        assert_eq!(
            with_loot, 0,
            "{with_loot} Harset template(s) carry a loot_table_id; spec L-01 says the zone has \
             no random loot and none was recovered from the 2009 data"
        );
    }

    /// Every H11 template exists under the exact name the packet allocated, and
    /// no name collides anywhere in the table.
    ///
    /// `entity_templates_template_name_key` already enforces uniqueness at the
    /// DB boundary, so the global check is belt-and-braces against a future
    /// migration dropping it. The per-id name check is the part that fails on a
    /// revert — and it also catches an id/name swap, which a bare count would
    /// not.
    #[tokio::test]
    async fn harset_template_names_are_unique_and_match_the_allocation() {
        let pool = require_db_or_skip!();

        for (template_id, expected_name) in MOB_TEMPLATES.iter().chain(PROP_TEMPLATES.iter()) {
            let name: Option<String> = sqlx::query_scalar(
                "SELECT template_name FROM resources.entity_templates WHERE template_id = $1",
            )
            .bind(template_id)
            .fetch_optional(&pool)
            .await
            .expect("query must succeed");

            assert_eq!(
                name.as_deref(),
                Some(*expected_name),
                "template {template_id} should be '{expected_name}'"
            );
        }

        let (total, distinct_names): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(DISTINCT template_name) FROM resources.entity_templates",
        )
        .fetch_one(&pool)
        .await
        .expect("query must succeed");

        assert_eq!(
            total, distinct_names,
            "entity_templates has {total} rows but only {distinct_names} distinct \
             template_name values — a Harset template collided with an existing row"
        );
    }

    /// **H-B8, second half.** Template 163 (Petbe) shipped with NULL level,
    /// alignment, faction and name_id.
    ///
    /// `faction` is pinned to 1, not 10, on purpose: the shared-hub Petbe
    /// carries mission 742's dialog binding and is the tag target of 1363
    /// Prudence, whose spec test M-10 requires that tagging him must not aggro.
    /// Faction cannot be changed at runtime — `useAbility` rejects a player
    /// ability against any target whose faction is not `HOSTILE_FACTION`, and
    /// right-click on an alive faction-10 NPC is rerouted to auto-attack — so
    /// mission 1245's hostile ambush uses template 221 instead. Flipping 163 to
    /// 10 would make Petbe unspeakable and break 742.
    #[tokio::test]
    async fn petbe_template_163_has_faction_level_and_alignment() {
        let pool = require_db_or_skip!();

        let (level, alignment, faction, name_id): (
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
        ) = sqlx::query_as(
            "SELECT level, alignment, faction, name_id \
             FROM resources.entity_templates WHERE template_id = 163",
        )
        .fetch_one(&pool)
        .await
        .expect("template 163 (Petbe) must exist");

        assert_eq!(level, Some(42), "Petbe's level (mission 1245 is L42)");
        assert_eq!(alignment, Some(0), "Petbe's alignment");
        assert_eq!(
            faction,
            Some(1),
            "Petbe must stay non-hostile — template 221 is the hostile clone for mission 1245"
        );
        assert_eq!(
            name_id,
            Some(7586),
            "Petbe needs a name_id or onNameIdUpdate is never sent and he renders unnamed"
        );

        // The hostile clone must exist and must actually be hostile, or 1245
        // has no ambush target.
        let (hostile_faction, hostile_set): (Option<i32>, Option<i32>) = sqlx::query_as(
            "SELECT faction, ability_set_id FROM resources.entity_templates WHERE template_id = 221",
        )
        .fetch_one(&pool)
        .await
        .expect("template 221 (Petbe, hostile) must exist");

        assert_eq!(hostile_faction, Some(10), "template 221 must be hostile");
        assert!(
            hostile_set.is_some(),
            "template 221 needs an ability set — it is the one Petbe that fights"
        );
    }

    /// **H-B8 for the eight plaza guards and four lieutenants.** Templates 159
    /// and 160 must resolve to the staff auto-attack, not the pistol fallback,
    /// and must carry the template-level respawn default.
    ///
    /// The respawn half is not redundant with H13's per-spawn values:
    /// `COALESCE(s.respawn_secs, t.respawn_secs)` means the template covers any
    /// spawn row that omits the column — including the rows M0's `.savespawn`
    /// authoring flow emits.
    #[tokio::test]
    async fn praxis_jaffa_guard_templates_use_the_staff_ability_set() {
        let pool = require_db_or_skip!();

        for template_id in [159_i32, 160] {
            let (ability_id, respawn_secs): (Option<i32>, Option<i32>) = sqlx::query_as(
                "SELECT asa.ability_id, t.respawn_secs \
                 FROM resources.entity_templates t \
                 LEFT JOIN resources.ability_set_abilities asa \
                        ON asa.ability_set_id = t.ability_set_id \
                 WHERE t.template_id = $1",
            )
            .bind(template_id)
            .fetch_one(&pool)
            .await
            .expect("Praxis Jaffa guard template must exist");

            assert_eq!(
                ability_id,
                Some(STAFF_AUTO_ATTACK),
                "template {template_id} must resolve to ability {STAFF_AUTO_ATTACK} (Staff Auto \
                 Attack); a NULL means it is back on NPC_DEFAULT_ABILITY \
                 ({NPC_DEFAULT_ABILITY}, Pistol Shot) and the Jaffa fire a Tau'ri pistol again"
            );
            assert_eq!(
                respawn_secs,
                Some(RESPAWN_DEFAULT),
                "template {template_id} must carry the template-level respawn default so a \
                 spawn row that omits respawn_secs still repopulates (defect H-B7)"
            );
        }
    }

    /// The two new ability sets exist and hold exactly the ability they were
    /// allocated for.
    ///
    /// One ability per set is a schema constraint, not a style choice:
    /// `ability_set_abilities` has `PRIMARY KEY (ability_set_id)`, so a second
    /// row for the same set is a duplicate key that aborts the whole seed load.
    /// This test pins the shape so a future packet that tries to add variety
    /// discovers the constraint here rather than in a failed DB reload.
    #[tokio::test]
    async fn harset_ability_sets_resolve_to_their_allocated_ability() {
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

            let description: Option<String> = sqlx::query_scalar(
                "SELECT description FROM resources.ability_sets WHERE ability_set_id = $1",
            )
            .bind(set_id)
            .fetch_optional(&pool)
            .await
            .expect("query must succeed");

            assert!(
                description.is_some(),
                "ability set {set_id} has ability_set_abilities rows but no ability_sets row — \
                 entity_templates_ability_set_id_fkey would reject every template pointing at it"
            );
        }
    }

    /// **Invisible-entity guard.** A template with no `components` must carry a
    /// non-empty `static_mesh`.
    ///
    /// `mercury::aoi::create::append_appearance` picks exactly one branch:
    /// `BeingAppearance` when body_set *and* components are non-empty,
    /// `onStaticMeshNameUpdate` when static_mesh is non-empty, otherwise it logs
    /// `aoi.cascade_appearance_missing` and the entity is **permanently**
    /// invisible to every witness — the AoI tick marks the witness set before
    /// delivery, so it never re-introduces it. A prop seeded with neither would
    /// spawn server-side, log a warn nobody reads, and be unplaceable in the M0
    /// session.
    #[tokio::test]
    async fn every_harset_template_has_a_renderable_appearance() {
        let pool = require_db_or_skip!();

        let rows: Vec<(i32, String, Option<String>, Option<Vec<String>>)> = sqlx::query_as(
            "SELECT template_id, template_name, static_mesh, components \
             FROM resources.entity_templates \
             WHERE template_id BETWEEN $1 AND $2 \
             ORDER BY template_id",
        )
        .bind(BLOCK_MIN)
        .bind(BLOCK_MAX)
        .fetch_all(&pool)
        .await
        .expect("query must succeed");

        let expected = MOB_TEMPLATES.len() + PROP_TEMPLATES.len();
        assert_eq!(
            rows.len(),
            expected,
            "expected {expected} Harset templates in {BLOCK_MIN}-{BLOCK_MAX}, found {}",
            rows.len()
        );

        for (template_id, template_name, static_mesh, components) in &rows {
            let has_components = components.as_ref().is_some_and(|c| !c.is_empty());
            let has_mesh = static_mesh.as_ref().is_some_and(|m| !m.is_empty());
            assert!(
                has_components || has_mesh,
                "template {template_id} ({template_name}) has neither components nor a \
                 static_mesh — append_appearance would take no branch and the entity would be \
                 invisible to every witness for the life of the space"
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
}
