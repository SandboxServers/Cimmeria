//! Method-index conformance: every hand-written wire method index in the
//! workspace, checked against `entities/defs/`, the files the client parses
//! (#801).
//!
//! A flat method index comes from BigWorld flattening an entity's parent
//! chain plus its `<Implements>` interfaces ([`flatten`]). Adding a method
//! to an interface, reordering methods or toggling `<Exposed/>` shifts every
//! index after it, while the Rust constants stay put. These tests make that
//! drift fail CI in either direction: a `.def` edit, or a constant edit.
//!
//! | Surface | Index space | Test |
//! |---|---|---|
//! | A `mercury::method_idx` | SGWPlayer (and SGWMob 27/28) ClientMethods | [`client_methods`] |
//! | B `cell/client_methods/**` | SGWPlayer ClientMethods, 0..157 once each; SGWPet in `pet.rs` | [`client_methods`] |
//! | C `cell-console` local copies | SGWPlayer ClientMethods | [`client_methods`] |
//! | D `cell_method_name` (every `CM_*`) | SGWPlayer exposed CellMethods | [`cell_methods`] |
//! | E `cell-console` `gm/mod.rs` | SGWGmPlayer exposed CellMethods (109+) | [`cell_methods`] |
//! | F `base` `sgw_player_base`, `wire::base::*` | SGWPlayer exposed BaseMethods (`0xC0 + idx`) | [`base_methods`] |
//! | G `BASEMSG_ON_*` | Account ClientMethods (`0x80 + idx`) | [`base_methods`] |
//! | I `crate::names` tables (`wire-log` names through them) | every section of every client-visible entity type | [`names_codegen`] |
//!
//! A constant matches when its name, uppercased with `_` removed, equals
//! the def method at its index, or when it is in that surface's short alias
//! list. The anchor tests below pin the flattener itself to counts and
//! boundary names taken from the dispatch tables, so a flattener bug cannot
//! make the surface tests agree with it by accident.

mod base_methods;
mod cell_methods;
mod client_methods;
pub(crate) mod flatten;
mod names_codegen;
mod source;

use flatten::{flatten, Section};
use source::{normalize, Const};

/// Check each const's index (`value - offset`) against `table`. Returns one
/// message per mismatch.
fn mismatches(
    consts: &[Const],
    table: &[String],
    offset: u32,
    aliases: &[(&str, &str)],
    strip_prefix: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    // Offset-encoded surfaces (0x80 / 0xC0 + idx) read better in hex.
    let show = |v: u32| {
        if offset == 0 {
            v.to_string()
        } else {
            format!("{v:#04X}")
        }
    };
    for c in consts {
        let alias = aliases.iter().find(|(n, _)| *n == c.name).map(|(_, d)| *d);
        let implied = alias.map_or_else(
            || normalize(c.name.strip_prefix(strip_prefix).unwrap_or(&c.name)),
            normalize,
        );
        let implied_at = table.iter().position(|m| normalize(m) == implied);
        let describe_implied = match implied_at {
            Some(i) => format!("{} is at {i}", table[i]),
            None => "no def method has that name".to_string(),
        };
        let Some(idx) = c.value.checked_sub(offset).map(|i| i as usize) else {
            out.push(format!(
                "{} = {:#X} ({}:{}) is below the index base {offset:#X}",
                c.name, c.value, c.file, c.line
            ));
            continue;
        };
        match table.get(idx) {
            Some(actual) if normalize(actual) == implied => {}
            Some(actual) => out.push(format!(
                "{} = {} ({}:{}) but flattened[{idx}] = {actual}; {describe_implied}",
                c.name,
                show(c.value),
                c.file,
                c.line
            )),
            None => out.push(format!(
                "{} = {} ({}:{}) is past the end of the {}-method table; {describe_implied}",
                c.name,
                show(c.value),
                c.file,
                c.line,
                table.len()
            )),
        }
    }
    out
}

/// Panic with every mismatch, one per line.
fn assert_no_mismatches(surface: &str, found: Vec<String>) {
    assert!(
        found.is_empty(),
        "{surface}: method-index constants disagree with entities/defs:\n  {}",
        found.join("\n  ")
    );
}

fn at(table: &[String], idx: usize) -> &str {
    table.get(idx).map_or("<past the end>", String::as_str)
}

/// Counts from the dispatch tables: `client-method-dispatch-table.md` (157),
/// `cell-method-dispatch-table.md` (109, GM 226),
/// `sgwplayer-base-method-dispatch-table.md` (30), and Account's 6.
#[test]
fn flattener_reproduces_dispatch_table_counts() {
    assert_eq!(flatten("SGWPlayer", Section::Client).len(), 157);
    assert_eq!(flatten("SGWPlayer", Section::Cell).len(), 109);
    assert_eq!(flatten("SGWGmPlayer", Section::Cell).len(), 226);
    assert_eq!(flatten("SGWPlayer", Section::Base).len(), 30);
    assert_eq!(flatten("Account", Section::Client).len(), 6);
    assert_eq!(flatten("SGWMob", Section::Client).len(), 29);
    assert_eq!(flatten("SGWPet", Section::Client).len(), 32);
}

/// Boundary names that catch interfaces-after-own-methods (11/12, 26/27,
/// 97/98), a `GamePawn` parent treated as an error or an entity (Account
/// 0/2), and the SGWBeing entity/interface mix-up (12 and 26).
#[test]
fn flattener_orders_interfaces_before_own_methods_at_each_level() {
    let client = flatten("SGWPlayer", Section::Client);
    for (idx, name) in [
        (11, "onBeingNameIDUpdate"),
        (12, "onTimerUpdate"),
        (26, "BeingAppearance"),
        (27, "onSystemCommunication"),
        (97, "onCookedDataError"),
        (98, "onBeginAidWait"),
        (156, "onCancelMovie"),
    ] {
        assert_eq!(at(&client, idx), name, "SGWPlayer client {idx}");
    }

    let cell = flatten("SGWPlayer", Section::Cell);
    assert_eq!(at(&cell, 0), "setTargetID");
    assert_eq!(at(&cell, 108), "cancelMovie");

    let gm = flatten("SGWGmPlayer", Section::Cell);
    assert_eq!(
        gm[..109],
        cell[..],
        "SGWGmPlayer starts with SGWPlayer's 109"
    );
    assert_eq!(at(&gm, 109), "gmMissionAssign");
    assert_eq!(at(&gm, 225), "changeCoverStanceWeight");

    let base = flatten("SGWPlayer", Section::Base);
    for (idx, name) in [
        (0, "chatJoin"),
        (21, "elementDataRequest"),
        (22, "logOff"),
        (29, "perfStats"),
    ] {
        assert_eq!(at(&base, idx), name, "SGWPlayer base {idx}");
    }

    let account = flatten("Account", Section::Client);
    assert_eq!(at(&account, 0), "onVersionInfo");
    assert_eq!(at(&account, 2), "onCharacterList");

    let mob = flatten("SGWMob", Section::Client);
    assert_eq!(mob[..27], client[..27], "SGWMob shares SGWPlayer's 0-26");
    assert_eq!(at(&mob, 27), "onAggressionOverrideUpdate");
}
