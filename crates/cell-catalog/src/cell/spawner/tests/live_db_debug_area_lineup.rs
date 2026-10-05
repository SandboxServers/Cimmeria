//! Live-DB guards for the Debug Area NPC lineup (DA-10, world 1300):
//! templates 1410-1599 and spawns 13870-14099, one friendly clone of every
//! distinct character look in `entity_templates`
//! (`docs/content/debug-area.md#npc-lineup`).
//!
//! The geometry (navmesh, occluder, walkways, reach) is guarded on the real
//! mesh by `cimmeria-cell`'s `service::tests::npc_ai::debug_area::lineup`.
//! These guards check the seed and what the loader hands the cell:
//!
//! * every character look outside the block has a clone in it, and every
//!   character body set no template uses has a dressed one, so a template
//!   added later with a new look fails here until it gets a lineup row;
//! * the block holds each look once, and each clone has exactly one spawn;
//! * every row loads into world 1300 as a stationary, friendly (faction 1)
//!   mob with no ability set, loot, interactions, patrol or override;
//! * the footers leave both sequences past the block.
//!
//! Each was proven to fail with a lineup template row deleted from the seed.
mod live_db {
    use std::collections::BTreeSet;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    const WORLD: &str = "DebugArea";
    const TEMPLATES: (i32, i32) = (1410, 1599);
    const SPAWNS: (i32, i32) = (13870, 14099);
    /// World Object: friendly to players, hostile to nobody.
    const FRIENDLY_FACTION: i32 = 1;
    /// The default sequence set every clone carries (`entity_templates`
    /// event set 570, "Players default event set").
    const DEFAULT_EVENT_SET: i32 = 570;

    /// `$query` behind a `look` CTE: each template's look, comparable across
    /// rows: body set, components in name order, the two colours, skin tint
    /// and static mesh ('' and NULL alike). Props (`GLB_Components.*`) and
    /// deployables and mines (`WP-Human.*`) are not characters.
    macro_rules! with_looks {
        ($query:literal) => {
            concat!(
                "WITH look AS ( \
                   SELECT template_id, template_name, body_set, \
                          (SELECT array_agg(c ORDER BY c) FROM unnest(components) c) AS comps, \
                          primary_color_id, secondary_color_id, skin_tint, \
                          coalesce(static_mesh, '') AS mesh \
                   FROM resources.entity_templates \
                   WHERE body_set NOT LIKE 'GLB\\_Components.%' \
                     AND body_set NOT LIKE 'WP-Human.%') ",
                $query
            )
        };
    }

    /// Every character look in `entity_templates` outside the lineup block
    /// has a clone in it, and every character body set that no template
    /// outside the block uses has a dressed clone. This is the guard that
    /// fails when a template with a new look is added without a lineup row.
    /// Revert proof: delete any lineup template row from the seed and this
    /// names its source.
    #[tokio::test]
    async fn debug_area_lineup_live_db_every_look_has_a_clone() {
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
            "templates whose look has no lineup clone (add one to \
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
            "character body sets with no template and no lineup clone: {bare:?}"
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
            assert_eq!(*n, 1, "{body_set}: one default-dressed clone");
        }
        assert_eq!(
            dressed.len(),
            6,
            "the six body sets no template uses are dressed: {dressed:?}"
        );
    }

    /// The block holds each look once, every clone is a `DebugArea Lineup - `
    /// template with a non-empty component list, and each has exactly one
    /// world-1300 spawn in the block (and nothing else spawns there).
    /// Revert proof: drop a spawn row, or duplicate a template row under a
    /// new id, and this fails.
    #[tokio::test]
    async fn debug_area_lineup_live_db_one_clone_and_one_spawn_per_look() {
        let pool = require_db_or_skip!();
        let (clones, looks): (i64, i64) = sqlx::query_as(with_looks!(
            "SELECT count(*), count(DISTINCT (body_set, comps, primary_color_id, \
                    secondary_color_id, skin_tint, mesh)) \
             FROM look WHERE template_id BETWEEN $1 AND $2"
        ))
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_one(&pool)
        .await
        .expect("clone count must succeed");
        assert!(clones >= 161, "155 looks and 6 bare body sets: {clones}");
        assert_eq!(clones, looks, "two lineup clones share a look");

        let bad_names: Vec<(i32, String)> = sqlx::query_as(
            "SELECT template_id, template_name FROM resources.entity_templates \
             WHERE template_id BETWEEN $1 AND $2 \
               AND (template_name NOT LIKE 'DebugArea Lineup - %' \
                    OR coalesce(cardinality(components), 0) = 0) \
             ORDER BY 1",
        )
        .bind(TEMPLATES.0)
        .bind(TEMPLATES.1)
        .fetch_all(&pool)
        .await
        .expect("template query must succeed");
        assert!(bad_names.is_empty(), "{bad_names:?}");

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
        assert_eq!(spawns.len() as i64, clones);
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
        assert_eq!(strays, 0, "the lineup block spawns only lineup clones");
    }

    /// Every lineup row loads into world 1300 as what it is meant to be: a
    /// stationary, friendly mob that stands still and never fights, with a
    /// unique `DebugArea_Lineup_` tag. Revert proof: give a clone faction 10,
    /// an ability set or a loot table, or clear `is_stationary`, and this
    /// names it.
    #[tokio::test]
    async fn debug_area_lineup_live_db_rows_load_friendly_and_stationary() {
        let pool = require_db_or_skip!();
        let rows: Vec<SpawnRecord> = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed")
            .into_iter()
            .filter(|r| (SPAWNS.0..=SPAWNS.1).contains(&r.spawn_id))
            .collect();
        assert!(rows.len() >= 161, "every lineup row loads: {}", rows.len());
        let mut errors = Vec::new();
        let mut tags = BTreeSet::new();
        for r in &rows {
            let t = r.tag.as_deref().unwrap_or("");
            let mut why = Vec::new();
            if r.world_name != WORLD {
                why.push(format!("world {}", r.world_name));
            }
            if !t.starts_with("DebugArea_Lineup_") || !tags.insert(t.to_string()) {
                why.push("tag not a unique DebugArea_Lineup_*".into());
            }
            if !(TEMPLATES.0..=TEMPLATES.1).contains(&r.template_id) {
                why.push(format!("template {} outside the block", r.template_id));
            }
            if r.faction != Some(FRIENDLY_FACTION) || r.class != "mob" {
                why.push(format!("faction {:?} class {}", r.faction, r.class));
            }
            if !r.is_stationary {
                why.push("not stationary".into());
            }
            if r.event_set_id != Some(DEFAULT_EVENT_SET) {
                why.push(format!("event set {:?}", r.event_set_id));
            }
            if !r.ability_ids.is_empty() || r.loot_table_id.is_some() {
                why.push("has abilities or loot".into());
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
        assert!(templates >= TEMPLATES.1 as i64, "template seq {templates}");
        assert!(spawns >= SPAWNS.1 as i64, "spawn seq {spawns}");
    }
}
