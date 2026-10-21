//! Surfaces A, B and C: ClientMethod indices (server to client).

use std::collections::BTreeMap;

use super::source::{block_consts, file_consts, rust_files_under};
use super::{assert_no_mismatches, flatten, mismatches, Section};

/// A: `mercury::method_idx`. It holds re-exports only; each constant is
/// checked where the table declares it (B).
const METHOD_IDX: &str = "wire/src/mercury/mod.rs";

/// B: the per-interface tables, the authoritative ClientMethod constants.
const CLIENT_METHODS_DIR: &str = "wire/src/cell/client_methods";
/// SGWPet's own three methods; every other file numbers SGWPlayer.
const PET_FILE: &str = "wire/src/cell/client_methods/pet.rs";
/// SGWMob's own two methods (27/28 on a player are Communicator).
const MOB_FILE: &str = "wire/src/cell/client_methods/mob.rs";
/// `u16` consts in B that are enum values, not method indices.
const NOT_METHOD_INDICES: &[&str] = &["CONDITION_FEEDBACK_INVALID_ENTITY"];

/// C: console files that must import their ClientMethod indices from the
/// tables, not declare local `u16` copies (#171).
const LOCAL_COPIES: &[&str] = &[
    "cell-console/src/cell/console/net.rs",
    "cell-console/src/cell/console/tests/p38.rs",
    "cell-console/src/cell/console/tests/ss_u2_duel.rs",
];

#[test]
fn method_idx_declares_no_literal_constants() {
    let consts = block_consts(METHOD_IDX, "pub mod method_idx", &["u16"]);
    assert!(
        consts.is_empty(),
        "method_idx declares literals {:?}: add the constant to \
         `cell::client_methods` and re-export it from method_idx",
        consts.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
}

#[test]
fn client_method_tables_cover_every_sgwplayer_index_exactly_once() {
    let player = flatten("SGWPlayer", Section::Client);
    let pet = flatten("SGWPet", Section::Client);
    let mob = flatten("SGWMob", Section::Client);
    let files = rust_files_under(CLIENT_METHODS_DIR);
    assert!(files.len() >= 15, "found only {} files", files.len());

    let (mut player_consts, mut pet_consts, mut mob_consts) = (Vec::new(), Vec::new(), Vec::new());
    for file in &files {
        let consts = file_consts(file, &["u16"])
            .into_iter()
            .filter(|c| !NOT_METHOD_INDICES.contains(&c.name.as_str()));
        if file == PET_FILE {
            pet_consts.extend(consts);
        } else if file == MOB_FILE {
            mob_consts.extend(consts);
        } else {
            player_consts.extend(consts);
        }
    }
    assert_eq!(pet_consts.len(), 3, "SGWPet's three consts: {pet_consts:?}");
    assert_eq!(mob_consts.len(), 2, "SGWMob's two consts: {mob_consts:?}");

    let mut found = mismatches(&player_consts, &player, 0, &[], "");
    found.extend(mismatches(&pet_consts, &pet, 0, &[], ""));
    found.extend(mismatches(&mob_consts, &mob, 0, &[], ""));
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
fn console_files_keep_no_local_client_method_copies() {
    for file in LOCAL_COPIES {
        let consts = file_consts(file, &["u16"]);
        assert!(
            consts.is_empty(),
            "{file} declares local ClientMethod copies {:?}: import them from \
             cell::client_methods instead",
            consts.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }
}
