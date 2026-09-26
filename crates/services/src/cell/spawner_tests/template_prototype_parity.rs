//! Live-DB guard that the two `entity_templates` readers never drift.
//!
//! `resources.entity_templates` is read from exactly two places: the cell's
//! startup template cache ([`load_spawn_templates`], which feeds the
//! `spawn_entity` content action) and the base-side GM `.spawn` handler
//! ([`crate::base::gm_spawn::load_spawn_record_for_template`]). Before the
//! PR #662 review each carried its own byte-identical copy of a 25-column
//! SELECT *and* of the row → [`SpawnRecord`] field mapping, so adding a
//! column meant editing two files and forgetting one produced a GM-spawned
//! mob that was armed, paced or factioned differently from a
//! content-spawned one — with nothing failing.
//!
//! Both now share `entity_template_select!` and `build_prototype`. This
//! test is what makes re-forking them fail loudly: it compares the *whole*
//! record rather than a field list, so a mapping that diverges on a column
//! this file has never heard of still trips it.

use crate::cell::spawner::{load_spawn_templates, SpawnRecord};
use crate::test_support::require_db_or_skip;

/// Normalise the spawn-instance fields — the legitimately different half.
/// A GM spawn carries the command's world/position/heading and is forced
/// one-shot; the cached prototype carries placeholders. Everything the
/// *template* decides must survive untouched, and that is what the caller
/// compares.
fn clear_spawn_instance_fields(record: &mut SpawnRecord) {
    record.world_name = String::new();
    record.x = 0.0;
    record.y = 0.0;
    record.z = 0.0;
    record.heading = 0.0;
    record.tag = None;
    record.spawn_id = -1;
    record.respawn_secs = None;
}

/// Every template-derived field of a GM-spawned record must equal the
/// cached prototype's for the same `template_id`.
///
/// Run against a template with a patrol path where the seed has one: the
/// patrol lookup is the one part of the mapping the two callers still feed
/// differently (the cache resolves every path in one query, the GM handler
/// resolves a single id), so a record with an empty path on both sides
/// would pass vacuously.
#[tokio::test]
async fn gm_spawn_record_matches_the_cached_template_prototype() {
    let pool = require_db_or_skip!();

    let cache = load_spawn_templates(&pool)
        .await
        .expect("load_spawn_templates must succeed against the seeded DB");
    assert!(
        !cache.is_empty(),
        "seeded resources.entity_templates must have rows"
    );

    // Prefer a template with a resolved patrol path so the patrol half of
    // the mapping is genuinely exercised; fall back to the lowest id if the
    // seed has none.
    let template_id = cache
        .iter()
        .filter(|(_, r)| !r.patrol_path.is_empty())
        .map(|(id, _)| *id)
        .min()
        .or_else(|| cache.keys().copied().min())
        .expect("cache is non-empty");

    let mut from_cache = cache
        .get(&template_id)
        .expect("template id came from this map")
        .clone();

    let mut from_gm = crate::base::gm_spawn::load_spawn_record_for_template(
        &pool,
        template_id,
        // Deliberately non-placeholder values: if the GM path ever stopped
        // overwriting the prototype's spawn-instance fields, the
        // normalisation below would hide it — so the *other* GM live-DB
        // test (`gm_spawn_resolves_real_template_and_replies`) pins those,
        // and this one pins only the template half.
        "Castle",
        [10.0, 20.0, 30.0],
        1.25,
    )
    .await
    .expect("the GM template query must succeed")
    .expect("the template id came from the cache, so the row exists");

    clear_spawn_instance_fields(&mut from_cache);
    clear_spawn_instance_fields(&mut from_gm);

    // Whole-record Debug comparison rather than a field list: a new column
    // mapped on only one side has to fail here without anyone remembering
    // to extend this test. `SpawnRecord` is not `PartialEq`, and making it
    // so for one test would be the bigger change.
    assert_eq!(
        format!("{from_gm:?}"),
        format!("{from_cache:?}"),
        "the GM spawn handler and the cell's startup template cache must \
         map template {template_id} identically — they share \
         `build_prototype` precisely so this can never diverge",
    );
}
