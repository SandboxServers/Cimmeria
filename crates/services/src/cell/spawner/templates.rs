//! Startup cache of every `resources.entity_templates` row, shaped as a
//! prototype [`SpawnRecord`].
//!
//! # Why the cell needs this
//!
//! The GM `.spawn` path resolves a template by round-tripping
//! cell → base → cell (`CellToBaseMsg::GmSpawnNpc` →
//! [`crate::base::gm_spawn`] → `BaseToCellMsg::GmSpawnNpcReady`), because
//! the base owns the DB pool at request time. That shape does not work for
//! a content chain's `spawn_entity` action, for two reasons:
//!
//! 1. **Action ordering.** The executor runs a chain's actions as an ordered
//!    list against one `&mut SpaceManager` borrow, and the action right
//!    after a spawn is routinely `set_aggression` / `generate_threat` /
//!    `add_dialog_set` / `set_interaction_type` — every one of which
//!    resolves through `find_entity_by_tag`. Deferring the spawn to a
//!    message round-trip makes all of them miss, and the chain
//!    half-executes with nothing but "entity tag not found" debug lines.
//! 2. **Instance lifetime.** `GmSpawnNpcReady` carries a `space_id`
//!    captured at request time. Mission spawns target per-player instanced
//!    spaces, which can be torn down while a request is in flight; the
//!    spawn would then land on `spawn_npc_from_record_into`'s
//!    "Space {id} disappeared" error path.
//!
//! So the cell caches the templates at startup instead, the same way it
//! already caches `dialog_set_maps`, `mission_defs`, `stargates`,
//! `ability_defs`, `item_defs`, `loot_tables` and `ring_regions`. 186 rows
//! at ~380 bytes is ~70 KB.
//!
//! # What a prototype record is and is not
//!
//! The returned records carry **template-derived fields only**. The
//! spawn-instance fields are placeholders that the caller must overwrite:
//! `world_name` (empty), `x`/`y`/`z`/`heading` (0.0), `tag` (`None`), and
//! `spawn_id` (`-1`, the same non-DB sentinel the GM path uses — it lands
//! as `CellEntity.spawn_id = None` so authoring commands like `.delspawn`
//! never target a phantom `spawnlist` row).
//!
//! Two fields have **no template column at all** and are therefore not
//! "inherited" in any sense: `is_stationary` lives only on `spawnlist`, and
//! aggression is a pure runtime `CellEntity` field. Both default off here.
//!
//! Like every other `SpaceManager` cache, this is a startup snapshot — an
//! `entity_templates` edit needs a server restart to take effect.

use std::collections::HashMap;

use sqlx::PgPool;

use super::npcs::SpawnRecord;

/// Every column a prototype [`SpawnRecord`] needs, plus the `FROM`, with
/// the caller's trailing clause (`""` for "all templates", or a `WHERE`)
/// concatenated onto the end.
///
/// Shared with the base-side GM spawn handler
/// ([`crate::base::gm_spawn`]), which reads one template by id. Before the
/// PR #662 review the two sites carried byte-identical copies of this
/// 25-column SELECT *and* of the field mapping below — so a new
/// `entity_templates` column had to be added in two places, and a GM-
/// spawned mob could silently diverge from a content-spawned one if only
/// one copy was updated. The `WHERE` is the only thing that ever differed.
///
/// A macro rather than a `fn(&str) -> String` because sqlx 0.9 only accepts
/// `&'static str` (`SqlSafeStr`): `concat!` keeps the composed query a
/// compile-time literal, so the sharing costs nothing and the
/// dynamic-SQL-injection escape hatch (`AssertSqlSafe`) is never needed.
/// The suffix is a literal at both call sites and the `$1` it references is
/// still bound by the caller, so no user data is interpolated either way.
macro_rules! entity_template_select {
    ($tail:literal) => {
        concat!(
            "SELECT t.template_id, t.template_name, t.class, t.static_mesh, t.body_set, \
                    t.components, t.flags, t.interaction_type, t.event_set_id, t.level, \
                    t.alignment, t.faction, t.name_id, t.speaker_id, \
                    t.static_interaction_sets, t.has_dynamic_properties, \
                    t.loot_table_id, \
                    t.patrol_path_id, \
                    COALESCE(t.patrol_point_delay, 2.0) AS patrol_point_delay, \
                    COALESCE(t.wander_radius, 0.0) AS wander_radius, \
                    COALESCE(t.wander_min_dwell_secs, 3.0) AS wander_min_dwell_secs, \
                    COALESCE(t.wander_max_dwell_secs, 8.0) AS wander_max_dwell_secs, \
                    COALESCE(t.follow_min_distance, 2.0) AS follow_min_distance, \
                    COALESCE(t.follow_max_distance, 5.0) AS follow_max_distance, \
                    COALESCE(t.move_speed, 0.6) AS move_speed, \
                    t.respawn_secs, \
                    COALESCE( \
                      (SELECT array_agg(asa.ability_id ORDER BY asa.ability_id) \
                       FROM resources.ability_set_abilities asa \
                       WHERE asa.ability_set_id = t.ability_set_id), \
                      ARRAY[]::int[] \
                    ) AS ability_ids \
             FROM resources.entity_templates t",
            $tail,
        )
    };
}

pub(crate) use entity_template_select;

/// Load every `resources.entity_templates` row into a prototype
/// [`SpawnRecord`], keyed by `template_id`.
///
/// Column → field mapping mirrors `load_spawns_from_db` exactly, and is
/// literally shared with `base::gm_spawn::load_spawn_record_for_template`
/// (see [`build_prototype`]), so a content-spawned mob is armed, paced and
/// configured identically to a seeded or GM-spawned one.
///
/// A row that fails to decode (a NULL in a non-`Option` column) is skipped
/// with a `warn!` rather than failing the whole load: one malformed
/// template must not cost the cell every other one.
///
/// That branch is **unreachable against the current schema** — every column
/// read without an `Option` here (`template_name`, `class`, `body_set`,
/// `flags`, `interaction_type`, `static_interaction_sets`,
/// `has_dynamic_properties`) is `NOT NULL` in
/// `db/resources/Entities/Tables/entity_templates.sql`, so it is defence
/// against a future schema relaxation rather than against today's seed. It
/// is deliberately untested for that reason; the base-side GM loader — which
/// since the PR #662 review shares [`build_prototype`] with this one — keeps
/// an equivalent guard with a live-DB test that drops the constraint to
/// reach it (`base::gm_spawn::tests::gm_spawn_malformed_template_drops_gracefully`).
pub async fn load_spawn_templates(pool: &PgPool) -> Result<HashMap<i32, SpawnRecord>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(entity_template_select!(""))
        .fetch_all(pool)
        .await?;

    // Resolve every referenced patrol path in one query rather than one per
    // template — same helper the spawnlist loader uses.
    let path_ids: Vec<i32> = rows
        .iter()
        .filter_map(|r| r.try_get::<Option<i32>, _>("patrol_path_id").ok().flatten())
        .collect();
    let patrol_paths = if path_ids.is_empty() {
        HashMap::new()
    } else {
        super::load_patrol_points(pool, &path_ids).await?
    };

    let mut out = HashMap::with_capacity(rows.len());
    let mut skipped = 0usize;
    for row in &rows {
        // `try_get` + `?` inside a closure so a decode failure on one row
        // skips that row instead of aborting the load.
        let template_id: i32 = match row.try_get("template_id") {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "entity_templates: row has no decodable template_id; skipped");
                skipped += 1;
                continue;
            }
        };
        let record = match build_prototype(row, &patrol_paths) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    template_id,
                    error = %e,
                    "entity_templates: template row failed to decode (NULL in a \
                     non-nullable column?); spawn_entity cannot use this template"
                );
                skipped += 1;
                continue;
            }
        };
        out.insert(template_id, record);
    }
    if skipped > 0 {
        tracing::warn!(
            skipped,
            loaded = out.len(),
            "entity_templates: some template rows were skipped; \
             spawn_entity on those template ids will refuse"
        );
    }
    Ok(out)
}

/// Materialize one `entity_templates` row (as selected by
/// [`entity_template_select`]) into a prototype `SpawnRecord`.
///
/// The spawn-instance fields are placeholders — `world_name` empty,
/// position/heading zero, `tag` `None`, `spawn_id = -1`. Callers that have
/// real values (the GM spawn command, the `spawn_entity` executor)
/// overwrite them; see the module header for what is and is not
/// "inherited" from a template.
///
/// `patrol_paths` is the resolved `point_set_points` lookup keyed by
/// `patrol_path_id`. A single-row caller can pass a one-entry map from
/// [`crate::cell::spawner::load_patrol_points`]; a NULL or unresolved
/// `patrol_path_id` yields an empty path either way.
pub(crate) fn build_prototype(
    row: &sqlx::postgres::PgRow,
    patrol_paths: &HashMap<i32, Vec<cimmeria_common::Vector3>>,
) -> Result<SpawnRecord, sqlx::Error> {
    use sqlx::Row;

    let patrol_path = row
        .try_get::<Option<i32>, _>("patrol_path_id")?
        .and_then(|id| patrol_paths.get(&id).cloned())
        .unwrap_or_default();

    Ok(SpawnRecord {
        // ── Spawn-instance placeholders — the caller overwrites all of
        // these. `spawn_id = -1` is the non-DB sentinel; it lands as
        // `CellEntity.spawn_id = None`.
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: None,
        // `entity_templates` has no `is_stationary` column (it lives on
        // `spawnlist`), so there is nothing to inherit — the spawn action's
        // param is the only source.
        is_stationary: false,

        // ── Template-derived ──
        template_id: row.try_get("template_id")?,
        template_name: row.try_get("template_name")?,
        class: row.try_get("class")?,
        static_mesh: row.try_get("static_mesh")?,
        body_set: row.try_get("body_set")?,
        components: row.try_get("components")?,
        flags: row.try_get("flags")?,
        interaction_type: row.try_get("interaction_type")?,
        event_set_id: row.try_get("event_set_id")?,
        level: row.try_get("level")?,
        alignment: row.try_get("alignment")?,
        faction: row.try_get("faction")?,
        name_id: row.try_get("name_id")?,
        speaker_id: row.try_get("speaker_id")?,
        static_interaction_sets: row.try_get("static_interaction_sets")?,
        has_dynamic_properties: row.try_get("has_dynamic_properties")?,
        loot_table_id: row.try_get("loot_table_id")?,
        ability_ids: row.try_get::<Vec<i32>, _>("ability_ids")?,
        // Carried verbatim off the template. The `spawn_entity` executor
        // forces this to `None` — see `executor::spawn` for why a
        // content-scoped spawn must never be handed to the respawn tick.
        respawn_secs: row
            .try_get::<Option<i32>, _>("respawn_secs")?
            .filter(|s| *s > 0)
            .map(|s| s as u32),
        patrol_path,
        patrol_point_delay_secs: row.try_get::<f32, _>("patrol_point_delay")?,
        wander_radius: row.try_get::<f32, _>("wander_radius")?,
        wander_min_dwell_secs: row.try_get::<f32, _>("wander_min_dwell_secs")?,
        wander_max_dwell_secs: row.try_get::<f32, _>("wander_max_dwell_secs")?,
        follow_min_distance: row.try_get::<f32, _>("follow_min_distance")?,
        follow_max_distance: row.try_get::<f32, _>("follow_max_distance")?,
        move_speed: row.try_get::<f32, _>("move_speed")?,
    })
}
