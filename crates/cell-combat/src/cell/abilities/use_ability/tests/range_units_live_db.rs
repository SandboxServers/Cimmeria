//! #919 end to end: the seeded 1652 Jaffa: Double Blast row, loaded by
//! `load_ability_defs`, gates `handle_use_ability` at 30 m, not 3000 m.
//!
//! The unit guard in `range_units.rs` builds the def by hand; this one takes
//! the loader's output, so it also fails if the loader stops converting the
//! UE3-unit column to metres.

use super::*;
use crate::cell::spawner::load_ability_defs;
use crate::test_support::require_db_or_skip;

const JAFFA_DOUBLE_BLAST: i32 = 1652;

#[tokio::test]
async fn seeded_double_blast_range_is_30_metres() {
    let pool = require_db_or_skip!();
    let defs = load_ability_defs(&pool).await.expect("ability defs load");
    let def = defs
        .get(&JAFFA_DOUBLE_BLAST)
        .expect("1652 Jaffa: Double Blast must be seeded");
    // The range is the subject; the shared no-op effect keeps the AB-12
    // no-mechanic refusal (which runs before the range check) out of it.
    let mut def = def.clone();
    def.effect_ids = vec![FIXTURE_EFFECT];

    assert!(
        !fire_at_hostile(&def, 29.0).await,
        "1652 (MaxRange 3000 UE3 units = 30 m) must reach a target at 29 m"
    );
    assert!(
        fire_at_hostile(&def, 3000.0).await,
        "1652 must refuse a target 3000 m away; a raw 3000 would accept it"
    );
}
