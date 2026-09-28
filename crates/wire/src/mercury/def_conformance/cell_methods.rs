//! Surfaces D and E: exposed CellMethod indices (client to cell).

use super::source::file_consts;
use super::{assert_no_mismatches, flatten, mismatches, Section};
use crate::cell::dispatch::names::cell_method_name;

/// E: the GM tail, numbered from SGWGmPlayer's flattened exposed methods.
const GM_CONSTS: &str = "cell-console/src/cell/console/gm/mod.rs";
const GM_ALIASES: &[(&str, &str)] = &[
    ("GM_PHYSICS", "onPhysics"),
    ("GM_SEND_GM_SHOUT", "sendGMShout"),
];

/// D: `cell_method_name` maps every `CM_*` constant to its def name, so
/// checking it at every index checks every constant through the arms that
/// already exist, with no source scan.
#[test]
fn cell_method_names_are_the_flattened_exposed_cell_methods() {
    let cell = flatten("SGWPlayer", Section::Cell);
    let wrong: Vec<String> = cell
        .iter()
        .enumerate()
        .filter(|(i, name)| cell_method_name(*i as u16) != name.as_str())
        .map(|(i, name)| {
            format!(
                "{i}: def {name}, cell_method_name {}",
                cell_method_name(i as u16)
            )
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "CM_* constants disagree with entities/defs:\n  {}",
        wrong.join("\n  ")
    );
    assert_eq!(cell_method_name(cell.len() as u16), "unknown");
}

#[test]
fn gm_constants_match_the_flattened_sgwgmplayer_cell_methods() {
    let player = flatten("SGWPlayer", Section::Cell);
    let gm = flatten("SGWGmPlayer", Section::Cell);
    let consts = file_consts(GM_CONSTS, &["u16"]);
    assert!(
        consts.len() >= 39,
        "{GM_CONSTS} yielded {} consts; the scan is broken",
        consts.len()
    );
    let below: Vec<_> = consts
        .iter()
        .filter(|c| (c.value as usize) < player.len())
        .map(|c| format!("{} = {}", c.name, c.value))
        .collect();
    assert!(
        below.is_empty(),
        "GM consts below the GM tail ({}): {below:?}",
        player.len()
    );
    assert_no_mismatches(
        "cell-console gm/mod.rs",
        mismatches(&consts, &gm, 0, GM_ALIASES, ""),
    );
}
