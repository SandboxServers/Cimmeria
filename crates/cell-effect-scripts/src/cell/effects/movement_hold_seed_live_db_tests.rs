//! Live-DB guards on the Prison Boot rows (MG-63b): effect 1939 is the
//! movement-only hold, effect 1942 its release, and 1942 carries the
//! `ReleasesEffect` NVP that names 1939 (`effect_nvps.sql`, id 25500).
//!
//! They load the real seed through the cell's own loader. A dropped
//! `script_name`, a dropped NVP or a renamed script fails here.

use super::registry::lookup;
use crate::cell::spawner::load_effect_defs;
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn prison_boot_effects_carry_their_script_and_release_live_db() {
    let pool = require_db_or_skip!();
    let defs = load_effect_defs(&pool).await.expect("load_effect_defs");

    let boot = defs.get(&1939).expect("effect 1939 is seeded");
    assert_eq!(boot.script_name.as_deref(), Some("MovementHold"));

    let release = defs.get(&1942).expect("effect 1942 is seeded");
    assert_eq!(release.script_name.as_deref(), Some("MovementRelease"));
    assert_eq!(
        release.params.get("ReleasesEffect").map(String::as_str),
        Some("1939")
    );

    assert!(lookup("MovementHold").is_some());
    assert!(lookup("MovementRelease").is_some());
}
