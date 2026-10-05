//! Live-DB guards for Z10, the Debug Area's Visual NPC Lineup (DA-10, world
//! 1300): templates 1410-1589 and spawns 13870-14079, one passive display
//! actor per distinct character look in `entity_templates`
//! (`docs/content/debug-area.md#visual-npc-lineup`), and the six Lineup
//! attendants (templates 1590-1595, spawns 14090-14095) that switch its
//! groups.
//!
//! The geometry (navmesh, occluder, walkways, reach, the aggro scan) is
//! guarded on the real mesh by `cimmeria-cell`'s
//! `service::tests::npc_ai::debug_area::lineup`. These guards check the seed
//! and what the loader hands the cell:
//!
//! * every character look outside the block has an actor in it, and every
//!   character body set no template uses has a dressed one, so a template
//!   added later with a new look fails here until it gets a lineup row (the
//!   guard counts looks, not templates);
//! * the block holds each look once, 161 actors, each with one spawn;
//! * every actor is a display copy only: no event set, ability set, loot,
//!   dialog, interactions, patrol, wander or radius overrides, and nothing
//!   anywhere in `resources` points at it;
//! * every nameplate and tag names its source template and body set;
//! * every row loads into world 1300 as a stationary, friendly (faction 1)
//!   mob, and the footers leave both sequences past the block;
//! * each attendant's click chain switches a lineup group that exists.
//!
//! The groups themselves (one per actor, size bound, off at boot) are
//! guarded where they are loaded, in `cimmeria-cell-world`'s
//! `spawner_tests::live_db_lineup_sets`.
//!
//! Each was proven to fail with a lineup template row deleted from the seed.
mod live_db {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const WORLD: &str = "DebugArea";
    /// The actors. The attendants follow them in DA-10's id blocks
    /// (templates 1410-1599, spawns 13870-14099).
    const TEMPLATES: (i32, i32) = (1410, 1589);
    const SPAWNS: (i32, i32) = (13870, 14079);
    const ATTENDANT_TEMPLATES: (i32, i32) = (1590, 1595);
    const ATTENDANT_SPAWNS: (i32, i32) = (14090, 14095);
    /// DA-10's reserved blocks, which the sequence footers clear.
    const BLOCK_END: (i32, i32) = (1599, 14099);
    /// 155 looks and 6 template-less body sets (owner-approved, 2026-10-05).
    /// A template with a new look raises it: add its actor and bump this.
    const ACTORS: i64 = 161;
    /// World Object: friendly to players, hostile to nobody.
    const FRIENDLY_FACTION: i32 = 1;
    /// Tag prefix (D-DA6); the rest is the source template id, or
    /// `NoTemplate_<body set>`.
    const TAG: &str = "DebugArea_VisualLineup_";

    /// `$query` behind a `look` CTE: each template's look, comparable across
    /// rows: body set, components as a sorted set, the two colours, skin tint
    /// and static mesh with NULL and '' the same (the client draws both as no
    /// static mesh; that merges Nerus, template 53, and template 166). Props
    /// (`GLB_Components.*`) and deployables and mines (`WP-Human.*`) are not
    /// characters.
    macro_rules! with_looks {
        ($query:literal) => {
            concat!(
                "WITH look AS ( \
                   SELECT template_id, template_name, body_set, \
                          (SELECT array_agg(c ORDER BY c) FROM unnest(components) c) AS comps, \
                          primary_color_id, secondary_color_id, skin_tint, \n                          coalesce(static_mesh, '') AS mesh \
                   FROM resources.entity_templates \
                   WHERE body_set NOT LIKE 'GLB\\_Components.%' \
                     AND body_set NOT LIKE 'WP-Human.%') ",
                $query
            )
        };
    }

    /// Every character look in `entity_templates` outside the lineup block
    /// has an actor in it, and every character body set that no template
    /// outside the block uses has one dressed actor. This is the guard that
    /// fails when a template with a new look is added without a lineup row.
    /// Revert proof: delete any lineup template row from the seed and this
    /// names every template with that look.
    #[tokio::test]
    async fn debug_area_lineup_live_db_every_look_has_an_actor() {
        let pool = require_db_or_skip!();
        let missing: Vec<(i32, String)> = sqlx::query_as(with_looks!(
            "SELECT s.template_id, s.template_name FROM look s \
             WHERE s.template_id NOT BETWEEN $1 AND $2 \
               AND NOT EXISTS ( \
                 SELECT 1 FROM look c WHERE c.template_id BETWEEN $1 AND $2 \
                   AND (c.body_set, c.comps, c.primary_color_id, c.secondary_color_id, \
                        c.skin_tint, c.mesh) IS NOT DISTINCT FROM \
                       (s.body_set, s.comps, s.primary_color_id, s.secondary_color_id, \
                        s.skin_tint, s.mesh)) \
             ORDER BY s.template_id"
        ))
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_all(&pool)
        .await
        .expect("look query must succeed");
        assert!(
            missing.is_empty(),
            "templates whose look has no lineup actor (add one to \
             entity_templates_debug_area_lineup.sql and a spawn to \
             spawnlist_debug_area_lineup.sql): {missing:?}"
        );

        let bare: Vec<(String,)> = sqlx::query_as(
            "SELECT b.body_set FROM resources.body_sets b \
             WHERE b.body_set NOT LIKE 'GLB\\_Components.%' \
               AND b.body_set NOT LIKE 'WP-Human.%' \
               AND NOT EXISTS (SELECT 1 FROM resources.entity_templates t \
                               WHERE t.body_set = b.body_set) \
             ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("body set query must succeed");
        assert!(
            bare.is_empty(),
            "character body sets with no template and no lineup actor: {bare:?}"
        );
        let dressed: Vec<(String, i64)> = sqlx::query_as(
            "SELECT t.body_set, count(*) FROM resources.entity_templates t \
             WHERE t.template_id BETWEEN $1 AND $2 \
               AND NOT EXISTS (SELECT 1 FROM resources.entity_templates o \
                               WHERE o.body_set = t.body_set \
                                 AND o.template_id NOT BETWEEN $1 AND $2) \
             GROUP BY 1 ORDER BY 1",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_all(&pool)
        .await
        .expect("dressed body set query must succeed");
        for (body_set, n) in &dressed {
            assert_eq!(*n, 1, "{body_set}: one default-dressed actor");
        }
        assert_eq!(
            dressed.len(),
            6,
            "the six body sets no template uses are dressed: {dressed:?}"
        );
    }

    /// The block holds each look once, [`ACTORS`] in all, and each actor has
    /// exactly one world-1300 spawn in the block (and nothing else spawns
    /// there). Revert proof: drop a spawn row, or duplicate a template row
    /// under a new id, and this fails.
    #[tokio::test]
    async fn debug_area_lineup_live_db_one_actor_and_one_spawn_per_look() {
        let pool = require_db_or_skip!();
        let (actors, looks): (i64, i64) = sqlx::query_as(with_looks!(
            "SELECT count(*), count(DISTINCT (body_set, comps, primary_color_id, \
                    secondary_color_id, skin_tint, mesh)) \
             FROM look WHERE template_id BETWEEN $1 AND $2"
        ))
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_one(&pool)
        .await
        .expect("actor count must succeed");
        assert_eq!(actors, ACTORS, "155 looks and 6 template-less body sets");
        assert_eq!(actors, looks, "two lineup actors share a look");

        let spawns: Vec<(i32, i32, i32)> = sqlx::query_as(
            "SELECT t.template_id, count(s.spawn_id)::int, \
                    count(*) FILTER (WHERE s.world_id = 1300 \
                                       AND s.spawn_id BETWEEN $3 AND $4)::int \
             FROM resources.entity_templates t \
             LEFT JOIN resources.spawnlist s ON s.template_id = t.template_id \
             WHERE t.template_id BETWEEN $1 AND $2 \
             GROUP BY 1 ORDER BY 1",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .bind(SPAWNS.0)
        .bind(SPAWNS.1)
        .fetch_all(&pool)
        .await
        .expect("spawn query must succeed");
        assert_eq!(spawns.len() as i64, actors);
        for (t, all, in_block) in &spawns {
            assert!(
                *all == 1 && *in_block == 1,
                "template {t}: {all} spawns, {in_block} in the lineup block"
            );
        }
        let (strays,): (i64,) = sqlx::query_as(
            "SELECT count(*) FROM resources.spawnlist \
             WHERE spawn_id BETWEEN $1 AND $2 \
               AND template_id NOT BETWEEN $3 AND $4",
        )
        .bind(SPAWNS.0)
        .bind(SPAWNS.1)
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_one(&pool)
        .await
        .expect("stray query must succeed");
        assert_eq!(strays, 0, "the lineup block spawns only lineup actors");
    }

    /// Every actor is a display copy only (owner requirement 4): faction 1,
    /// class mob, no flags, and every behaviour column empty: no event set,
    /// ability set, ammo, loot, vendor lists, trainer list, dialog speaker,
    /// interaction type or sets, weapon, patrol, wander, follow, speed,
    /// leash, aggro or assist radius, cover use, training-dummy mark or
    /// respawn. Faction 1 is what makes it non-combat: players may damage
    /// only faction 10, and no faction is hostile to 1. Revert proof: give a
    /// clone event set 570 or ability set 2 and this names it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_actors_are_display_copies_only() {
        let pool = require_db_or_skip!();
        let bad: Vec<(i32, String)> = sqlx::query_as(
            "SELECT template_id, template_name FROM resources.entity_templates \
             WHERE template_id BETWEEN $1 AND $2 AND NOT ( \
                   faction = $3 AND class = 'mob' AND flags = 0 \
               AND interaction_type = 0 AND event_set_id IS NULL \
               AND ability_set_id IS NULL AND ammo_type IS NULL \
               AND loot_table_id IS NULL AND buy_item_list IS NULL \
               AND sell_item_list IS NULL AND repair_item_list IS NULL \
               AND recharge_item_list IS NULL AND trainer_ability_list_id IS NULL \
               AND speaker_id IS NULL AND interaction_set_id IS NULL \
               AND cardinality(static_interaction_sets) = 0 \
               AND weapon_item_id IS NULL AND patrol_path_id IS NULL \
               AND wander_radius IS NULL AND follow_min_distance IS NULL \
               AND follow_max_distance IS NULL AND move_speed IS NULL \
               AND leash_distance IS NULL AND aggro_radius IS NULL \
               AND assist_radius IS NULL AND use_cover IS NULL \
               AND respawn_secs IS NULL AND NOT training_dummy \
               AND coalesce(cardinality(components), 0) > 0) \
             ORDER BY 1",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .bind(FRIENDLY_FACTION)
        .fetch_all(&pool)
        .await
        .expect("template query must succeed");
        assert!(
            bad.is_empty(),
            "lineup actors with behaviour attached: {bad:?}"
        );

        // Nothing else in `resources` (content chains, mission steps,
        // dialogs, loot, spawn sets, ...) names a lineup template or spawn:
        // every integer column called `template_id` or `spawn_id` outside
        // the two seed tables, counted in the block.
        let refs: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT c.table_name::text, c.column_name::text, \
                    (xpath('/row/n/text()', query_to_xml(format( \
                      'SELECT count(*) AS n FROM resources.%I WHERE %I BETWEEN %s AND %s', \
                      c.table_name, c.column_name, \
                      CASE WHEN c.column_name LIKE '%spawn%' THEN $3 ELSE $1 END, \
                      CASE WHEN c.column_name LIKE '%spawn%' THEN $4 ELSE $2 END), \
                    false, true, '')))[1]::text::bigint \
             FROM information_schema.columns c \
             JOIN information_schema.tables t \
               ON t.table_schema = c.table_schema AND t.table_name = c.table_name \
             WHERE c.table_schema = 'resources' AND t.table_type = 'BASE TABLE' \
               AND c.column_name IN ('template_id', 'spawn_id', 'entity_template_id') \
               AND c.data_type IN ('integer', 'bigint') \
               AND c.table_name NOT IN ('entity_templates', 'spawnlist') \
             ORDER BY 1, 2",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .bind(SPAWNS.0)
        .bind(SPAWNS.1)
        .fetch_all(&pool)
        .await
        .expect("reference scan must succeed");
        let pointing: Vec<_> = refs.iter().filter(|(_, _, n)| *n > 0).collect();
        assert!(
            pointing.is_empty(),
            "seed rows that hook a lineup actor into content: {pointing:?}"
        );

        // Content chains, dialogs and the rest name NPCs by tag (trigger
        // keys, action target keys, JSON params): no text or JSON column
        // outside `spawnlist` mentions a lineup tag.
        let tagged: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT c.table_name::text, c.column_name::text, \
                    (xpath('/row/n/text()', query_to_xml(format( \
                      'SELECT count(*) AS n FROM resources.%I WHERE %I::text LIKE %L', \
                      c.table_name, c.column_name, '%DebugArea\\_VisualLineup\\_%'), \
                    false, true, '')))[1]::text::bigint \
             FROM information_schema.columns c \
             JOIN information_schema.tables t \
               ON t.table_schema = c.table_schema AND t.table_name = c.table_name \
             WHERE c.table_schema = 'resources' AND t.table_type = 'BASE TABLE' \
               AND c.data_type IN ('text', 'character varying', 'json', 'jsonb') \
               AND c.table_name <> 'spawnlist' \
             ORDER BY 1, 2",
        )
        .fetch_all(&pool)
        .await
        .expect("tag scan must succeed");
        assert!(tagged.len() > 50, "the tag scan covers the text columns");
        let hooked: Vec<_> = tagged.iter().filter(|(_, _, n)| *n > 0).collect();
        assert!(
            hooked.is_empty(),
            "seed rows that name a lineup tag: {hooked:?}"
        );
    }

    /// Every actor's nameplate and tag trace it (owner requirement 3): the
    /// tag is `DebugArea_VisualLineup_<source id>` and the `display_name`
    /// ends `#<source id> <body set>`, where the source is a template outside
    /// the block with the same look; a template-less actor is tagged
    /// `..._NoTemplate_<body set>` and reads `(no template) <body set>`.
    /// Every nameplate fits in 40 characters. Revert proof: point a tag at
    /// another template, or clear a `display_name`, and this names it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_nameplates_and_tags_name_the_source() {
        let pool = require_db_or_skip!();
        let rows: Vec<(i32, String, String, Option<String>, Option<i32>)> =
            sqlx::query_as(with_looks!(
                "SELECT c.template_id, c.body_set, s.tag, t.display_name, \
                        (SELECT min(o.template_id) FROM look o \
                          WHERE o.template_id NOT BETWEEN $1 AND $2 \
                            AND (o.body_set, o.comps, o.primary_color_id, \
                                 o.secondary_color_id, o.skin_tint, o.mesh) \
                                IS NOT DISTINCT FROM \
                                (c.body_set, c.comps, c.primary_color_id, \
                                 c.secondary_color_id, c.skin_tint, c.mesh)) \
                 FROM look c \
                 JOIN resources.entity_templates t ON t.template_id = c.template_id \
                 JOIN resources.spawnlist s ON s.template_id = c.template_id \
                 WHERE c.template_id BETWEEN $1 AND $2 ORDER BY 1"
            ))
            .bind(TEMPLATES.0)
            .bind(TEMPLATES.1)
            .fetch_all(&pool)
            .await
            .expect("nameplate query must succeed");
        assert_eq!(rows.len() as i64, ACTORS);
        let mut errors = Vec::new();
        let mut plates = BTreeMap::new();
        for (id, body_set, tag, plate, source) in &rows {
            let short = body_set.rsplit('.').next().unwrap_or(body_set);
            let plate = plate.as_deref().unwrap_or("");
            let (want_tag, want_tail) = match source {
                Some(src) => (format!("{TAG}{src}"), format!(" #{src} {short}")),
                None => (
                    format!("{TAG}NoTemplate_{short}"),
                    format!("(no template) {short}"),
                ),
            };
            if *tag != want_tag {
                errors.push(format!("{id}: tag {tag}, want {want_tag}"));
            }
            if !plate.ends_with(&want_tail) || plate.len() > 40 {
                errors.push(format!("{id}: nameplate {plate:?}, want ...{want_tail:?}"));
            }
            if let Some(other) = plates.insert(plate.to_string(), *id) {
                errors.push(format!("{id} and {other} share the nameplate {plate:?}"));
            }
        }
        assert!(errors.is_empty(), "{errors:#?}");
    }

    /// Every lineup row loads into world 1300 as what it is meant to be: a
    /// stationary, friendly mob that stands still and never fights, carrying
    /// its nameplate, with a unique tag. Revert proof: clear `is_stationary`
    /// or drop `display_name` from the loader query, and this names it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_rows_load_friendly_and_stationary() {
        let pool = require_db_or_skip!();
        let rows: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| (SPAWNS.0..=SPAWNS.1).contains(&r.spawn_id))
            .collect();
        assert_eq!(rows.len() as i64, ACTORS, "every lineup row loads");
        let mut errors = Vec::new();
        let mut tags = BTreeSet::new();
        for r in &rows {
            let t = r.tag.as_deref().unwrap_or("");
            let mut why = Vec::new();
            if r.world_name != WORLD {
                why.push(format!("world {}", r.world_name));
            }
            if !t.starts_with(TAG) || !tags.insert(t.to_string()) {
                why.push("tag not a unique DebugArea_VisualLineup_*".into());
            }
            if r.faction != Some(FRIENDLY_FACTION) || r.class != "mob" {
                why.push(format!("faction {:?} class {}", r.faction, r.class));
            }
            if !r.is_stationary || r.training_dummy {
                why.push("not stationary, or a training dummy".into());
            }
            if r.display_name.as_deref().is_none_or(str::is_empty) {
                why.push("no nameplate".into());
            }
            if r.event_set_id.is_some() || !r.ability_ids.is_empty() || r.loot_table_id.is_some() {
                why.push("has an event set, abilities or loot".into());
            }
            if r.interaction_type != 0
                || !r.static_interaction_sets.is_empty()
                || r.speaker_id.is_some()
            {
                why.push("has interactions".into());
            }
            if !r.patrol_path.is_empty()
                || r.wander_radius != 0.0
                || r.aggression_override.is_some()
                || r.respawn_secs.is_some()
            {
                why.push("patrols, wanders, is pinned or respawns".into());
            }
            if !why.is_empty() {
                errors.push(format!("{t} (spawn {}): {}", r.spawn_id, why.join(", ")));
            }
        }
        assert!(errors.is_empty(), "{errors:#?}");
    }

    /// The footers leave both sequences past the block, so a default-id
    /// insert (`.savespawn`, a GM template) never lands in it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_sequences_clear_the_block() {
        let pool = require_db_or_skip!();
        let (templates,): (i64,) =
            sqlx::query_as("SELECT last_value FROM resources.entity_templates_template_id_seq")
                .fetch_one(&pool)
                .await
                .expect("template sequence");
        let (spawns,): (i64,) =
            sqlx::query_as("SELECT last_value FROM resources.spawnlist_spawn_id_seq")
                .fetch_one(&pool)
                .await
                .expect("spawn sequence");
        assert!(templates >= BLOCK_END.0 as i64, "template seq {templates}");
        assert!(spawns >= BLOCK_END.1 as i64, "spawn seq {spawns}");
    }

    /// Each Lineup attendant is a clickable world-1300 NPC whose tag fires
    /// exactly one chain, and that chain's one `spawn_set` action names a
    /// lineup group that exists (or clears the lineup's kind in world 1300).
    /// Each label reads `Show <group> (<its actor count>)`, or `Clear lineup`. Revert proof: point an attendant's
    /// `set_id` at 1399, or drop its trigger row, and this names it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_attendants_switch_existing_groups() {
        let pool = require_db_or_skip!();
        let rows: Vec<(
            i32,
            i32,
            String,
            Option<String>,
            i64,
            Option<i64>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT s.spawn_id, t.interaction_type::int, s.tag, t.display_name, \
                        (SELECT count(*) FROM resources.content_triggers tr \
                          WHERE tr.event_type = 'interact_tag' AND tr.event_key = s.tag), \
                        (SELECT count(*) FROM resources.content_triggers tr \
                           JOIN resources.content_actions a ON a.chain_id = tr.chain_id \
                           LEFT JOIN resources.spawn_sets ss \
                             ON ss.set_id = (a.params->>'set_id')::int \
                          WHERE tr.event_type = 'interact_tag' AND tr.event_key = s.tag \
                            AND a.action_type = 'spawn_set' \
                            AND ((a.params->>'op' = 'show' AND ss.type = 'visual_lineup' \
                                  AND ss.world_id = 1300) \
                              OR (a.params->>'op' = 'clear' \
                                  AND a.params->>'kind' = 'visual_lineup' \
                                  AND (a.params->>'world_id')::int = 1300))), \
                        (SELECT count(m.spawn_id)::text FROM resources.content_triggers tr \
                           JOIN resources.content_actions a ON a.chain_id = tr.chain_id \
                           JOIN resources.spawn_sets ss \
                             ON ss.set_id = (a.params->>'set_id')::int \
                           JOIN resources.spawnlist m \
                             ON m.set_name = ss.name AND m.world_id = ss.world_id \
                          WHERE tr.event_key = s.tag AND a.params->>'op' = 'show') \
                 FROM resources.spawnlist s \
                 JOIN resources.entity_templates t ON t.template_id = s.template_id \
                 WHERE s.spawn_id BETWEEN $1 AND $2 AND s.world_id = 1300 \
                   AND s.template_id BETWEEN $3 AND $4 AND s.set_name IS NULL \
                 ORDER BY 1",
        )
        .bind(ATTENDANT_SPAWNS.0)
        .bind(ATTENDANT_SPAWNS.1)
        .bind(ATTENDANT_TEMPLATES.0)
        .bind(ATTENDANT_TEMPLATES.1)
        .fetch_all(&pool)
        .await
        .expect("attendant query must succeed");
        assert_eq!(rows.len(), 6, "six attendants: {rows:#?}");
        let mut errors = Vec::new();
        for (spawn, interaction, tag, label, triggers, good, members) in &rows {
            let label = label.as_deref().unwrap_or("");
            if *interaction == 0 {
                errors.push(format!("{spawn}: no click cursor"));
            }
            if *triggers != 1 || *good != Some(1) {
                errors.push(format!(
                    "{tag}: {triggers} trigger(s), {good:?} valid spawn_set"
                ));
            }
            // A label longer than this overlaps its neighbour's, 2 m away.
            let fits = match members.as_deref().filter(|n| *n != "0") {
                Some(n) => label.starts_with("Show ") && label.ends_with(&format!(" ({n})")),
                None => label == "Clear lineup",
            };
            if !fits || label.len() > 24 {
                errors.push(format!("{spawn}: label {label:?} for {members:?} actors"));
            }
        }
        assert!(errors.is_empty(), "{errors:#?}");
    }
}
