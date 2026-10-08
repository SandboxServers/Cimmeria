//! Live-DB guards: the seed's start profiles are what the fixture and the
//! CharDef identity table say.

use super::super::chardef::chardef_lookup;
use super::fixture;
use super::*;
use crate::test_support::require_db_or_skip;

/// **Guard.** The seed's profiles, kits and debug kit are exactly the
/// fixture the non-DB tests use. A seed change without the fixture (or the
/// reverse) fails here, so a unit test can never pass on a profile the
/// seed no longer has. It also pins the CS-02 rows themselves: the Free
/// Jaffa start on Dakara_E1, the legacy kit only on the holding states, no
/// kit on a canonical human.
#[tokio::test]
async fn fixture_is_the_seed_live_db() {
    let pool = require_db_or_skip!();
    let mut conn = pool.acquire().await.expect("connection");
    let loaded = load_all(&mut conn).await.expect("start profiles load");
    let want = fixture::seeded();
    for p in want.profiles() {
        assert_eq!(
            loaded.by_char_def(p.char_def_id),
            Some(p),
            "char_def {}: db/resources/Archetypes/Seed/char_creation*.sql and \
             start_profiles/fixture.rs disagree",
            p.char_def_id
        );
    }
    assert_eq!(loaded.profiles().len(), want.profiles().len());
    assert_eq!(loaded.debug_kit(), want.debug_kit());
}

/// **Guard.** The identity columns of `resources.char_creation` agree with
/// the CharDef table creation writes from (`chardef_lookup`). The profile's
/// alignment and archetype pick a player's home, so a seed row naming
/// another archetype would send that class to someone else's start.
#[tokio::test]
async fn chardef_identity_matches_the_start_profile_rows_live_db() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, i32, i32, String)> = sqlx::query_as(
        "SELECT char_def_id, \
                (array_position(enum_range(NULL::resources.\"EAlignment\"), alignment) - 1)::int, \
                (array_position(enum_range(NULL::resources.\"EArchetype\"), archetype) - 1)::int, \
                array_position(enum_range(NULL::resources.\"EGender\"), gender)::int, \
                body_set::text \
           FROM resources.char_creation ORDER BY char_def_id",
    )
    .fetch_all(&pool)
    .await
    .expect("read char_creation identity");
    assert_eq!(rows.len(), 23);
    for (id, alignment, archetype, gender, body_set) in rows {
        let c = chardef_lookup(id).unwrap_or_else(|| panic!("char_def {id} is a client id"));
        assert_eq!(
            (alignment, archetype, gender, body_set.as_str()),
            (c.alignment, c.archetype, c.gender, c.bodyset),
            "char_def {id}"
        );
    }
}

/// **Guard (L3 input).** Every profile's start world is a `resources.worlds`
/// row. Creation also refuses a world with no loaded cell space; that half
/// is the cell's and is tested in `cimmeria-base`.
#[tokio::test]
async fn every_start_world_is_a_known_world_live_db() {
    let pool = require_db_or_skip!();
    let missing: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT cc.starting_world::text FROM resources.char_creation cc \
          WHERE NOT EXISTS (SELECT 1 FROM resources.worlds w WHERE w.world = cc.starting_world)",
    )
    .fetch_all(&pool)
    .await
    .expect("read start worlds");
    assert_eq!(missing, Vec::<String>::new());
}
