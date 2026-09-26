//! `onAbilityTreeInfo` on the wire, built from a fixture catalog, and the
//! guard that no hand-copied ability tree comes back into `world_data`.

use super::super::*;
use super::{sample_player_load_data, sample_world_entry, walk_entity_method_records};
use crate::ability_tree::{tree_info, AbilityTreeCatalog, TreeNode};

/// The whole `onAbilityTreeInfo` record (marker to last arg byte) in a
/// `mapLoaded` body.
fn tree_info_record(body: &[u8]) -> &[u8] {
    let &(_, start) = walk_entity_method_records(body)
        .iter()
        .find(|(idx, _)| *idx == method_idx::ON_ABILITY_TREE_INFO)
        .expect("mapLoaded must carry onAbilityTreeInfo");
    let word_len = u16::from_le_bytes([body[start + 1], body[start + 2]]) as usize;
    &body[start..start + 3 + word_len]
}

/// Byte-exact `onAbilityTreeInfo(ARRAY<ARRAY<INT32>>)` from a catalog.
/// Nodes are inserted out of branch order and out of id order: the payload
/// groups by branch and keeps catalog order inside each branch, with the
/// empty middle branch still on the wire as a zero count.
#[test]
fn on_ability_tree_info_bytes_from_fixture_catalog() {
    const ARCH: i32 = 5;
    let catalog = AbilityTreeCatalog::from_nodes([
        TreeNode::with_defaults(ARCH, 0, 30, 1, vec![]),
        TreeNode::with_defaults(ARCH, 2, 20, 1, vec![]),
        TreeNode::with_defaults(ARCH, 0, 10, 1, vec![]),
    ]);
    let mut data = sample_player_load_data();
    data.archetype = ARCH;
    data.ability_tree = tree_info(&catalog, ARCH, data.player_id);

    let body = build_map_loaded_body(42, &data, &sample_world_entry());

    #[rustfmt::skip]
    let expected: &[u8] = &[
        0xBD, 0x21, 0x00,       // extended marker, payload 33 = eid 4 + sub 1 + args 28
        0x2A, 0x00, 0x00, 0x00, // entity id 42
        0x50,                   // sub-index: 141 - idbase 61
        0x03, 0x00, 0x00, 0x00, // outer count: 3 branches
        0x02, 0x00, 0x00, 0x00, // branch 0: 2 ids
        0x1E, 0x00, 0x00, 0x00, //   30
        0x0A, 0x00, 0x00, 0x00, //   10
        0x00, 0x00, 0x00, 0x00, // branch 1: 0 ids
        0x01, 0x00, 0x00, 0x00, // branch 2: 1 id
        0x14, 0x00, 0x00, 0x00, //   20
    ];
    assert_eq!(tree_info_record(&body), expected);
}

/// An archetype with no catalog rows still sends the three-branch shape,
/// all empty: never a fabricated tree.
#[test]
fn on_ability_tree_info_for_archetype_without_rows_is_three_empty_branches() {
    let catalog = AbilityTreeCatalog::from_nodes([TreeNode::with_defaults(1, 0, 30, 1, vec![])]);
    let mut data = sample_player_load_data();
    data.archetype = 8;
    data.ability_tree = tree_info(&catalog, 8, data.player_id);

    let body = build_map_loaded_body(42, &data, &sample_world_entry());

    #[rustfmt::skip]
    let expected: &[u8] = &[
        0xBD, 0x15, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x50,
        0x03, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(tree_info_record(&body), expected);
}

/// The smallest integer an ability id takes and the largest, for the
/// source scan below. Real ids are 3 or 4 digits.
const ID_RANGE: std::ops::RangeInclusive<i64> = 100..=9999;
/// A run this long of id-shaped integer literals on one line is a
/// hand-copied table. The retired Soldier/Commando arrays had 11 to 16 per
/// line.
const MAX_ID_RUN: usize = 6;

/// Longest run of consecutive comma-separated integer literals in
/// [`ID_RANGE`] on one line, ignoring a trailing `//` comment.
fn longest_id_run(line: &str) -> usize {
    let code = line.split("//").next().unwrap_or("");
    let mut best = 0;
    let mut run = 0;
    for token in code.split(',') {
        let token = token
            .trim()
            .trim_start_matches("vec!")
            .trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace());
        if token.parse::<i64>().is_ok_and(|n| ID_RANGE.contains(&n)) {
            run += 1;
            best = best.max(run);
        } else {
            run = 0;
        }
    }
    best
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read world_data dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// AT-02 regression guard: `onAbilityTreeInfo` comes from the catalog
/// only. A hand-copied ability-id array anywhere under
/// `mercury/world_data/` (source or tests) is the drift AT-02 removed:
/// the Rust Commando copy started at 700 while the seed started at 597.
#[test]
fn no_ability_id_array_literal_in_world_data() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mercury/world_data");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    assert!(
        files.len() > 5,
        "scan must see the world_data tree, saw {files:?}"
    );

    let offenders: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("read source");
            text.lines()
                .enumerate()
                .filter(|(_, line)| longest_id_run(line) >= MAX_ID_RUN)
                .map(|(n, line)| format!("{}:{}: {}", path.display(), n + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "ability-id array literals in mercury/world_data/; build trees from \
         crate::ability_tree::AbilityTreeCatalog instead:\n{}",
        offenders.join("\n")
    );
}

/// The scanner itself: it must flag the retired table's shape and pass
/// ordinary code.
#[test]
fn longest_id_run_flags_table_rows_only() {
    let row = ["597", "603", "604", "610", "611", "616", "617"].join(", ");
    assert_eq!(longest_id_run(&format!("    vec![{row},")), 7);
    assert_eq!(longest_id_run("    0x03, 0x00, 0x00, 0x00, // count"), 0);
    assert_eq!(longest_id_run("foo(1, 2, 3, 4, 5, 6, 7)"), 0);
}
