//! Surfaces A, B, C and H: ClientMethod indices (server to client).

use std::collections::BTreeMap;

use super::source::{block, block_consts, file_consts, read, rust_files_under};
use super::{assert_no_mismatches, flatten, mismatches, Section};

/// A: `mercury::method_idx`. `pub use` re-exports carry no literal and are
/// checked where they are declared (B).
const METHOD_IDX: &str = "wire/src/mercury/mod.rs";
/// SGWMob's own methods in `method_idx` (27/28 on a player are Communicator).
const MOB_ONLY: &[&str] = &[
    "ON_AGGRESSION_OVERRIDE_UPDATE",
    "ON_AGGRESSION_OVERRIDE_CLEARED",
];
const METHOD_IDX_ALIASES: &[(&str, &str)] =
    &[("CLEAR_HINTED_REGIONS", "clearClientHintedGenericRegions")];

/// B: the per-interface tables, the authoritative ClientMethod constants.
const CLIENT_METHODS_DIR: &str = "wire/src/cell/client_methods";
/// SGWPet's own three methods; every other file numbers SGWPlayer.
const PET_FILE: &str = "wire/src/cell/client_methods/pet.rs";
/// `u16` consts in B that are enum values, not method indices.
const NOT_METHOD_INDICES: &[&str] = &["CONDITION_FEEDBACK_INVALID_ENTITY"];

/// C: local copies outside the tables (#171 removes the console ones).
const LOCAL_COPIES: &[&str] = &[
    "cell-console/src/cell/console/net.rs",
    "cell-console/src/cell/console/tests/p38.rs",
    "cell-console/src/cell/console/tests/ss_u2_duel.rs",
];
const LOCAL_COPY_ALIASES: &[(&str, &str)] = &[
    // The PvP flag rides onEntityProperty(GENERICPROPERTY_PvPFlag, ...).
    ("PVP_FLAG", "onEntityProperty"),
    ("DUEL_CLEAR", "onDuelEntitiesClear"),
];

/// H: the wire-log name table (157 literal arms, sourced from the doc).
const OUTBOUND_NAMES: &str = "wire-log/src/wire_log/client_names.rs";

#[test]
fn method_idx_constants_match_the_flattened_client_methods() {
    let player = flatten("SGWPlayer", Section::Client);
    let mob = flatten("SGWMob", Section::Client);
    let consts = block_consts(METHOD_IDX, "pub mod method_idx", &["u16"]);
    assert!(
        consts.len() >= 60,
        "method_idx yielded {} consts; the scan is broken",
        consts.len()
    );
    let (mob_consts, player_consts): (Vec<_>, Vec<_>) = consts
        .into_iter()
        .partition(|c| MOB_ONLY.contains(&c.name.as_str()));
    assert_eq!(mob_consts.len(), MOB_ONLY.len(), "SGWMob consts not found");
    let mut found = mismatches(&player_consts, &player, 0, METHOD_IDX_ALIASES, "");
    found.extend(mismatches(&mob_consts, &mob, 0, &[], ""));
    assert_no_mismatches("mercury::method_idx", found);
}

#[test]
fn client_method_tables_cover_every_sgwplayer_index_exactly_once() {
    let player = flatten("SGWPlayer", Section::Client);
    let pet = flatten("SGWPet", Section::Client);
    let files = rust_files_under(CLIENT_METHODS_DIR);
    assert!(files.len() >= 15, "found only {} files", files.len());

    let (mut player_consts, mut pet_consts) = (Vec::new(), Vec::new());
    for file in &files {
        let consts = file_consts(file, &["u16"])
            .into_iter()
            .filter(|c| !NOT_METHOD_INDICES.contains(&c.name.as_str()));
        if file == PET_FILE {
            pet_consts.extend(consts);
        } else {
            player_consts.extend(consts);
        }
    }
    assert_eq!(pet_consts.len(), 3, "SGWPet's three consts: {pet_consts:?}");

    let mut found = mismatches(&player_consts, &player, 0, &[], "");
    found.extend(mismatches(&pet_consts, &pet, 0, &[], ""));
    assert_no_mismatches("cell::client_methods", found);

    // Complete and unique: the tables are the one place every index lives.
    let mut by_index: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for c in &player_consts {
        by_index
            .entry(c.value)
            .or_default()
            .push(format!("{} ({})", c.name, c.file));
    }
    let dups: Vec<_> = by_index.iter().filter(|(_, v)| v.len() > 1).collect();
    assert!(dups.is_empty(), "indices declared twice: {dups:?}");
    let missing: Vec<usize> = (0..player.len())
        .filter(|i| !by_index.contains_key(&(*i as u32)))
        .collect();
    assert!(
        missing.is_empty(),
        "no constant for SGWPlayer client methods {:?}",
        missing
            .iter()
            .map(|i| format!("{i} {}", player[*i]))
            .collect::<Vec<_>>()
    );
    assert_eq!(player_consts.len(), player.len());
}

#[test]
fn local_client_method_copies_match_the_flattened_client_methods() {
    let player = flatten("SGWPlayer", Section::Client);
    let mut found = Vec::new();
    for file in LOCAL_COPIES {
        let consts = file_consts(file, &["u16"]);
        assert!(!consts.is_empty(), "{file} yielded no consts");
        found.extend(mismatches(&consts, &player, 0, LOCAL_COPY_ALIASES, ""));
    }
    assert_no_mismatches("local ClientMethod copies", found);
}

#[test]
fn outbound_method_names_are_the_flattened_client_methods() {
    let player = flatten("SGWPlayer", Section::Client);
    let text = read(OUTBOUND_NAMES);
    let body = block(&text, "pub fn outbound_method_name");
    let mut arms = Vec::new();
    let mut has_unknown = false;
    for line in body.lines() {
        let Some((pattern, name)) = line.trim().split_once("=>") else {
            continue;
        };
        let name = name.trim().trim_end_matches(',').trim_matches('"');
        match pattern.trim() {
            "_" => has_unknown = name == "unknown",
            idx => arms.push((
                idx.parse::<usize>()
                    .unwrap_or_else(|_| panic!("arm `{}` is not a literal", line.trim())),
                name.to_string(),
            )),
        }
    }
    assert!(
        has_unknown,
        "outbound_method_name has no `_ => \"unknown\"` arm"
    );
    let expected: Vec<(usize, String)> = player.iter().cloned().enumerate().collect();
    let wrong: Vec<String> = expected
        .iter()
        .zip(arms.iter().map(Some).chain(std::iter::repeat(None)))
        .filter(|(e, a)| Some(*e) != *a)
        .map(|((i, want), got)| format!("{i}: def {want}, table {got:?}"))
        .collect();
    assert!(
        wrong.is_empty() && arms.len() == player.len(),
        "{OUTBOUND_NAMES} disagrees with entities/defs ({} arms, {} methods):\n  {}",
        arms.len(),
        player.len(),
        wrong.join("\n  ")
    );
}
