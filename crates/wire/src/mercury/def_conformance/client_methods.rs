//! Surfaces A, B and C: ClientMethod indices (server to client).

use std::collections::BTreeMap;

use super::source::{block_consts, crate_file, file_consts, rust_files_under};
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

/// C: the console tree. No file in it may declare a local `u16` copy of a
/// ClientMethod index; it imports them from the tables (#171).
const CONSOLE_DIR: &str = "cell-console/src/cell/console";
/// Files in C that hold `u16` ids which are not ClientMethod indices: the
/// inbound GM method ids (client to server) and the test pinning their range.
const NOT_CLIENT_METHODS: &[&str] = &[
    "cell-console/src/cell/console/gm/mod.rs",
    "cell-console/src/cell/console/gm/tests/mod.rs",
];
/// Files C must reach; a walker that stops short would pass on nothing.
const MUST_SCAN: &[&str] = &[
    "cell-console/src/cell/console/net.rs",
    "cell-console/src/cell/console/chat/mod.rs",
    "cell-console/src/cell/console/tests/p38.rs",
    "cell-console/src/cell/console/tests/ss_u2_duel.rs",
];

/// Every `.rs` file under `crates/<dir>`, test directories included, sorted.
fn all_rust_files_under(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_string()];
    while let Some(rel) = stack.pop() {
        let entries = std::fs::read_dir(crate_file(&rel))
            .unwrap_or_else(|e| panic!("crates/{rel} is gone ({e})"));
        for entry in entries.flatten() {
            let child = format!("{rel}/{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                stack.push(child);
            } else if child.ends_with(".rs") {
                out.push(child);
            }
        }
    }
    out.sort();
    out
}

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
    let files = all_rust_files_under(CONSOLE_DIR);
    for must in MUST_SCAN {
        assert!(
            files.iter().any(|f| f == must),
            "the console scan did not reach {must}"
        );
    }
    for file in files
        .iter()
        .filter(|f| !NOT_CLIENT_METHODS.contains(&f.as_str()))
    {
        let consts = file_consts(file, &["u16"]);
        assert!(
            consts.is_empty(),
            "{file} declares local ClientMethod copies {:?}: import them from \
             cell::client_methods instead",
            consts.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    }
}
