//! Guards on the seeded template rows themselves: an ability set is present and
//! is not the pistol fallback, a respawn delay is set, no loot table is
//! attached, the allocated names are in place, and every row reaches the client
//! with something to render and something to be called.

use super::*;

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

    // Aggregated, not a bare LEFT JOIN. H11 wrote this as a plain join and
    // relied on `PRIMARY KEY (ability_set_id)` to guarantee one row per
    // template — and said so, predicting that widening the key would break
    // the count for a reason having nothing to do with the seed rows. Packet
    // H09 widened it, so the join now fans out to one row per *ability*.
    // `array_agg` restores one row per template and, as a bonus, mirrors
    // exactly what `load_spawns_from_db` does, so this guard now checks the
    // shape the runtime actually consumes.
    let rows: Vec<(i32, String, Option<i32>, Vec<i32>)> = sqlx::query_as(
        "SELECT t.template_id, t.template_name, t.ability_set_id, \
                COALESCE( \
                  (SELECT array_agg(asa.ability_id ORDER BY asa.ability_id) \
                   FROM resources.ability_set_abilities asa \
                   WHERE asa.ability_set_id = t.ability_set_id), \
                  ARRAY[]::int[] \
                ) AS ability_ids \
         FROM resources.entity_templates t \
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

    for (template_id, template_name, ability_set_id, ability_ids) in &rows {
        assert!(
            ability_set_id.is_some(),
            "template {template_id} ({template_name}) has a NULL ability_set_id — it will \
             fall back to NPC_DEFAULT_ABILITY ({NPC_DEFAULT_ABILITY}, Pistol Shot). That is \
             defect H-B8, the whole point of this packet."
        );
        assert!(
            !ability_ids.is_empty(),
            "template {template_id} ({template_name}) points at ability set \
             {ability_set_id:?}, but that set has no row in ability_set_abilities — the \
             aggregate yields an empty ability bucket and the spawn path silently falls \
             back to NPC_DEFAULT_ABILITY"
        );
        assert!(
            !ability_ids.contains(&NPC_DEFAULT_ABILITY),
            "template {template_id} ({template_name}) resolves to the pistol fallback \
             ability {NPC_DEFAULT_ABILITY} through an explicit ability set — that defeats \
             the H-B8 fix. Bucket: {ability_ids:?}"
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

    let expected_roster =
        (MOB_TEMPLATES.len() + PROP_TEMPLATES.len() + PREEXISTING_HARSET_TEMPLATES.len()) as i64;

    let (roster_size, new_block, with_loot): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*), \
                count(*) FILTER (WHERE template_id BETWEEN $1 AND $2), \
                count(*) FILTER (WHERE loot_table_id IS NOT NULL) \
         FROM resources.entity_templates \
         WHERE template_id BETWEEN $1 AND $2 OR template_id = ANY($3)",
    )
    .bind(BLOCK_MIN)
    .bind(BLOCK_MAX)
    .bind(PREEXISTING_HARSET_TEMPLATES.as_slice())
    .fetch_one(&pool)
    .await
    .expect("query must succeed");

    // Assert the new block first so a revert reports "the H11 rows are
    // gone" rather than the less specific roster-size mismatch.
    let expected_block = (MOB_TEMPLATES.len() + PROP_TEMPLATES.len()) as i64;
    assert_eq!(
        new_block, expected_block,
        "{BLOCK_MIN}-{BLOCK_MAX} holds {new_block} templates, expected {expected_block} — \
         the H11 seed rows are missing, so the loot sweep below would pass vacuously"
    );
    assert_eq!(
        roster_size, expected_roster,
        "Harset template roster is {roster_size} rows, expected {expected_roster} — one of \
         the ten pre-existing Harset templates was deleted"
    );
    assert_eq!(
        with_loot, 0,
        "{with_loot} Harset template(s) carry a loot_table_id; spec L-01 says the zone has \
         no random loot and none was recovered from the 2009 data"
    );
}

/// Every H11 template exists under the exact name the packet allocated.
///
/// Name *uniqueness* is not asserted as a data query. `entity_templates`
/// carries `entity_templates_template_name_key UNIQUE (template_name)`, so
/// `count(*) == count(DISTINCT template_name)` is a tautology — it asserts
/// the database enforced its own index. The constraint's continued existence
/// is asserted directly instead, which is the thing that can actually
/// regress (a migration dropping it).
///
/// The per-id loop is the revert guard: it names the first missing template
/// rather than reporting a count mismatch.
#[tokio::test]
async fn harset_template_names_match_the_allocation() {
    let pool = require_db_or_skip!();

    let ids: Vec<i32> = MOB_TEMPLATES
        .iter()
        .chain(PROP_TEMPLATES.iter())
        .map(|(id, _)| *id)
        .collect();

    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT template_id, template_name FROM resources.entity_templates \
         WHERE template_id = ANY($1)",
    )
    .bind(&ids)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    for (template_id, expected_name) in MOB_TEMPLATES.iter().chain(PROP_TEMPLATES.iter()) {
        let actual = rows
            .iter()
            .find(|(id, _)| id == template_id)
            .map(|(_, name)| name.as_str());
        assert_eq!(
            actual,
            Some(*expected_name),
            "template {template_id} should be '{expected_name}'"
        );
    }

    let unique_constraints: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_constraint \
         WHERE conname = 'entity_templates_template_name_key' AND contype = 'u'",
    )
    .fetch_one(&pool)
    .await
    .expect("query must succeed");

    assert_eq!(
        unique_constraints, 1,
        "entity_templates_template_name_key is gone — nothing stops a future packet \
         seeding a duplicate template_name, which the loader would surface as two \
         templates the content engine cannot tell apart"
    );
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
///
/// The predicate below mirrors that branch **exactly**, including the
/// `body_set` half. `body_set` is `NOT NULL` but an empty string is legal
/// and already present in the seed (template 167 ships `body_set = ''`), so
/// a row with components, an empty body_set and no static_mesh would fall
/// through both branches while a components-only check waved it past.
#[tokio::test]
async fn every_harset_template_has_a_renderable_appearance() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, String, String, Option<String>, Option<Vec<String>>)> = sqlx::query_as(
        "SELECT template_id, template_name, body_set, static_mesh, components \
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

    for (template_id, template_name, body_set, static_mesh, components) in &rows {
        let takes_being_branch =
            !body_set.is_empty() && components.as_ref().is_some_and(|c| !c.is_empty());
        let takes_mesh_branch = static_mesh.as_ref().is_some_and(|m| !m.is_empty());
        assert!(
            takes_being_branch || takes_mesh_branch,
            "template {template_id} ({template_name}) matches neither appearance branch \
             (body_set {body_set:?}, components {components:?}, static_mesh \
             {static_mesh:?}) — append_appearance logs \
             aoi.cascade_appearance_missing and the entity is invisible to every witness \
             for the life of the space"
        );
    }
}

/// **Unnamed-entity guard.** Every H11 template whose `name_id` is set must
/// point at a `texts` row with non-empty text.
///
/// `onNameIdUpdate` is only sent when `name_id` is `Some` and non-zero, and
/// the client resolves it against the text table — so a moniker whose `text`
/// column is empty renders as a blank name over the NPC's head. Three Harset
/// monikers are in exactly that state (7582 NID Operative, 7585 Opheltes,
/// 7587 Ra's Jaffa), which is why those three templates point at substitute
/// monikers from other zones instead. This guard stops a future packet
/// "correcting" them back to the empty Harset rows.
///
/// A NULL `name_id` is allowed and not flagged: four rows (207 Jaffa
/// Volunteer, 220 Lethander's Contact, 243 Shield Tower, 244 Petbe's
/// Quarters) have no moniker anywhere in the 2009 data.
#[tokio::test]
async fn harset_templates_with_a_name_id_resolve_to_non_empty_text() {
    let pool = require_db_or_skip!();

    let rows: Vec<(i32, String, i32, Option<String>)> = sqlx::query_as(
        "SELECT t.template_id, t.template_name, t.name_id, x.text \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id BETWEEN $1 AND $2 AND t.name_id IS NOT NULL \
         ORDER BY t.template_id",
    )
    .bind(BLOCK_MIN)
    .bind(BLOCK_MAX)
    .fetch_all(&pool)
    .await
    .expect("query must succeed");

    assert!(
        !rows.is_empty(),
        "no H11 template in {BLOCK_MIN}-{BLOCK_MAX} has a name_id — the seed rows are \
         missing, so the sweep below would pass vacuously"
    );

    for (template_id, template_name, name_id, text) in &rows {
        let text = text.as_deref().unwrap_or_else(|| {
            panic!(
                "template {template_id} ({template_name}) points at moniker {name_id}, \
                 which has no row in resources.texts"
            )
        });
        assert!(
            !text.is_empty(),
            "template {template_id} ({template_name}) points at moniker {name_id}, whose \
             text is empty — the NPC renders with a blank name over its head"
        );
    }
}
