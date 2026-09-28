//! Surfaces F and G: base-entity method indices.

use super::source::{block_consts, file_consts, Const};
use super::{assert_no_mismatches, flatten, mismatches, Section};

/// F: SGWPlayer exposed BaseMethods, sent as `0xC0 | idx`.
const BASE_DISPATCH: &str = "base/src/base/dispatch/mod.rs";
/// Base-method constants that live in `cimmeria-wire` and are re-exported
/// into `sgw_player_base` by path (no literal there).
const WIRE_BASE_FILES: &[&str] = &["wire/src/base/organization.rs", "wire/src/base/duel.rs"];
const BASE_ALIASES: &[(&str, &str)] = &[
    ("CHAT_SET_AFK", "chatSetAFKMessage"),
    ("CHAT_SET_DND", "chatSetDNDMessage"),
];
const BASE_METHOD_BASE: u32 = 0xC0;

/// G: Account ClientMethods, sent as `0x80 + idx`.
const BASEMSG_FILES: &[&str] = &[
    "wire/src/mercury/mod.rs",
    "base-session/src/base/cooked_sync/tests/mod.rs",
    "base/src/base/connect_loop/encrypted/cache_routing_tests.rs",
];
const ACCOUNT_METHOD_BASE: u32 = 0x80;

#[test]
fn sgw_player_base_constants_match_the_flattened_exposed_base_methods() {
    let base = flatten("SGWPlayer", Section::Base);
    let mut consts = block_consts(BASE_DISPATCH, "mod sgw_player_base", &["u8"]);
    assert!(
        consts.len() >= 18,
        "sgw_player_base yielded {} consts; the scan is broken",
        consts.len()
    );
    for file in WIRE_BASE_FILES {
        let in_range: Vec<Const> = file_consts(file, &["u8"])
            .into_iter()
            .filter(|c| c.value >= BASE_METHOD_BASE)
            .collect();
        assert!(!in_range.is_empty(), "{file} yielded no base-method consts");
        consts.extend(in_range);
    }
    assert_no_mismatches(
        "SGWPlayer base methods",
        mismatches(&consts, &base, BASE_METHOD_BASE, BASE_ALIASES, ""),
    );
}

#[test]
fn basemsg_constants_match_the_flattened_account_client_methods() {
    let account = flatten("Account", Section::Client);
    let mut consts = Vec::new();
    for file in BASEMSG_FILES {
        let found: Vec<Const> = file_consts(file, &["u8"])
            .into_iter()
            .filter(|c| c.name.starts_with("BASEMSG_ON_"))
            .collect();
        assert!(!found.is_empty(), "{file} yielded no BASEMSG_ON_* consts");
        consts.extend(found);
    }
    assert!(
        consts.len() >= 5,
        "found only {} BASEMSG_ON_* consts",
        consts.len()
    );
    assert_no_mismatches(
        "BASEMSG_ON_* (Account)",
        mismatches(&consts, &account, ACCOUNT_METHOD_BASE, &[], "BASEMSG_"),
    );
}
