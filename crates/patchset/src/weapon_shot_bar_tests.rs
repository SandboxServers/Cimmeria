//! The committed `015-weapon-shot-bar` zip against its sources in the repo.
//!
//! 015 writes the same file as `009-starter-hotbar`,
//! `ActionProfileDefault1.lua`, and must leave 009's code as it is. The
//! patched file is 009's output with
//! `data/client-patches/015-weapon-shot-bar/WeaponShotBar.lua` appended byte
//! for byte.
//!
//! CI has no stock client, so the delta is decoded against two synthetic
//! sources. Both are 009's output with the stock part (the first 3470 bytes)
//! replaced, by `0x00` in one and `0x01` in the other, and followed by the
//! real 009 hook, which is in the repo. A byte the delta takes from 009's
//! hook or from its own extra block comes out the same from both; a byte it
//! derives from the stock part differs. That pins, without the stock file:
//!
//! - the first 3470 output bytes are the source copied unchanged, so the
//!   delta never alters a stock byte and stores none;
//! - every other byte that is not derived from a stock byte is 009's hook
//!   byte or the 015 block's byte at that offset, and those are nearly all.
//!
//! bsdiff codes a few bytes as a difference from a stock byte (a comment
//! rule that looks like one in the stock file), and this test cannot check
//! those. `real_client_weapon_shot_bar`, ignored, applies the zip to a real
//! client's file and compares every byte.
//!
//! An edit to the block without a rebuilt zip fails here. Once 015 is
//! published, a rebuilt zip must ship under a new patch id (append-only).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::apply::bspatch;
use crate::debug_area_rings_tests::{entry, recipe};

const ID: &str = "015-weapon-shot-bar";
const STARTER_HOTBAR: &str = "009-starter-hotbar";
const TARGET: &str = "Working/SGWGame/Content/UI/Core/ActionButtons/ActionProfileDefault1.lua";
/// The stock 2009 file, as the cabinets ship it.
const STOCK_LEN: usize = 3470;
/// 009's output followed by `WeaponShotBar.lua`.
const RESULT_SHA256: &str = "b3202d575a514f97f434e485e30d6084837da479d44aef5c149e6080fb5c4d6a";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Vec<u8> {
    std::fs::read(repo().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn read_abs(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn block() -> Vec<u8> {
    read("data/client-patches/015-weapon-shot-bar/WeaponShotBar.lua")
}

fn starter_hook() -> Vec<u8> {
    read("data/client-patches/009-starter-hotbar/StarterHotbar.lua")
}

/// 015 starts from 009's output and nothing else: a client without 009 is
/// refused, and a client with it gets 009's bytes back unchanged.
#[test]
fn committed_015_starts_from_009_output_only() {
    let (r015, z015) = recipe(ID);
    let (r009, _) = recipe(STARTER_HOTBAR);
    assert_eq!(r015.ops.len(), 1, "{r015:?}");
    let op = &r015.ops[0];
    assert_eq!(op.target, TARGET);
    assert_eq!(op.sources.len(), 1, "{op:?}");
    let source = &op.sources[0];
    assert_eq!(source.path, TARGET);
    assert_eq!(
        source.sha256, r009.ops[0].result_sha256,
        "015 must start from the exact file 009 writes"
    );
    assert_eq!(source.output_of.as_deref(), Some(STARTER_HOTBAR));
    assert!(op.alternatives.is_empty(), "{op:?}");
    assert_eq!(op.result_sha256, RESULT_SHA256);
    // The recipe and the one delta: no whole files.
    assert_eq!(z015.len(), 2);
}

/// The block is stored the way the client's UI scripts are: ASCII, CRLF.
#[test]
fn weapon_shot_bar_block_is_ascii_crlf() {
    let block = block();
    assert!(block.is_ascii(), "the block must be ASCII");
    let lf = block.iter().filter(|&&b| b == b'\n').count();
    let crlf = block.windows(2).filter(|w| w == b"\r\n").count();
    assert_eq!(
        crlf, lf,
        "the block must use CRLF line endings (.gitattributes keeps them)"
    );
}

/// Decode the delta: 009's output comes back unchanged, and everything after
/// it is the committed block.
#[test]
fn committed_015_delta_keeps_009_and_appends_exactly_the_committed_block() {
    let block = block();
    let hook = starter_hook();
    let (r015, mut z015) = recipe(ID);
    let delta = entry(&mut z015, &r015.ops[0].delta);

    let source = |stock_byte: u8| {
        let mut s = vec![stock_byte; STOCK_LEN];
        s.extend_from_slice(&hook);
        s
    };
    let zeros = bspatch(&source(0), &delta).unwrap();
    let ones = bspatch(&source(1), &delta).unwrap();

    // What the patched file must hold after the stock part.
    let mut ours = hook.clone();
    ours.extend_from_slice(&block);
    assert_eq!(
        zeros.len(),
        STOCK_LEN + ours.len(),
        "the delta builds a file of another length: WeaponShotBar.lua changed without a rebuilt 015 zip"
    );

    // The stock part is the source, copied unchanged.
    for i in 0..STOCK_LEN {
        assert!(
            zeros[i] == 0 && ones[i] == 1,
            "output byte {i} is not a plain copy of the stock byte"
        );
    }

    // Every byte that does not depend on a stock byte is 009's hook byte or
    // the block's byte at that offset.
    let mut pinned = 0usize;
    for (k, &want) in ours.iter().enumerate() {
        let i = STOCK_LEN + k;
        if zeros[i] == ones[i] {
            let part = if k < hook.len() {
                "009's hook"
            } else {
                "the 015 block"
            };
            assert_eq!(
                zeros[i], want,
                "output byte {i} ({part}) differs from the committed source: \
                 WeaponShotBar.lua changed without a rebuilt 015 zip, or the delta changes 009's code"
            );
            pinned += 1;
        }
    }
    // The rest is derived from stock bytes by bsdiff, which this test cannot
    // check without the stock file; keep that share small.
    assert!(
        pinned * 100 >= ours.len() * 97,
        "only {pinned} of {} bytes after the stock part are pinned",
        ours.len()
    );
}

// Needs the client's own file: ActionProfileDefault1.lua with 009 applied (a
// launcher-installed client has it). Set SGW_009_DEFAULT_PROFILE to that
// file. The zip must rebuild it into the same bytes followed by the block,
// say "already current" the second time, and refuse the stock file.
//   cargo test -p cimmeria-patchset real_client_weapon_shot_bar -- --ignored --nocapture
#[test]
#[ignore = "needs a client's ActionProfileDefault1.lua with 009 applied; see the comment"]
fn real_client_weapon_shot_bar() {
    let with_009 = read_abs(&std::env::var("SGW_009_DEFAULT_PROFILE").unwrap());
    let zip = repo().join(format!("data/client-patches/{ID}.zip"));
    let block = block();
    let hook = starter_hook();
    assert!(
        with_009.ends_with(&hook),
        "the file does not end with 009's hook"
    );
    assert_eq!(with_009.len(), STOCK_LEN + hook.len());

    let tree = tempfile::tempdir().unwrap();
    let to = tree.path().join(TARGET);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::write(&to, &with_009).unwrap();
    let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![TARGET.to_string()]);
    let rebuilt = std::fs::read(&to).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), RESULT_SHA256);
    assert_eq!(
        &rebuilt[..with_009.len()],
        &with_009[..],
        "009's output changed"
    );
    assert_eq!(
        &rebuilt[with_009.len()..],
        &block[..],
        "the appended bytes are not the block"
    );
    let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![TARGET.to_string()]);

    // A client without 009 is not a source of this patch.
    std::fs::write(&to, &with_009[..STOCK_LEN]).unwrap();
    assert!(crate::apply(&zip, tree.path(), &mut |_| {}).is_err());
    assert_eq!(
        std::fs::read(&to).unwrap(),
        &with_009[..STOCK_LEN],
        "the stock file was touched"
    );
}

/// `(ability_id, event_id)` of every weapon binding in the seed.
fn seeded_bindings() -> Vec<(u32, u32)> {
    let sql = String::from_utf8(read("db/resources/Items/Seed/items_event_sets.sql")).unwrap();
    let rows: Vec<(u32, u32)> = sql
        .lines()
        .filter_map(|line| {
            let values = line
                .strip_prefix("INSERT INTO items_event_sets ")?
                .split_once("VALUES (")?
                .1
                .strip_suffix(");")?;
            let fields: Vec<u32> = values
                .split(',')
                .map(|f| f.trim().parse().unwrap())
                .collect();
            assert_eq!(fields.len(), 4, "{line}");
            Some((fields[2], fields[3]))
        })
        .collect();
    assert!(rows.len() > 2000, "only {} seed rows parsed", rows.len());
    rows
}

/// The ids in the block's `WeaponShots = { ... }` table.
fn block_weapon_shots() -> BTreeSet<u32> {
    let block = String::from_utf8(block()).unwrap();
    let list = block
        .split_once("WeaponShots = {")
        .expect("WeaponShots table")
        .1
        .split_once('}')
        .unwrap()
        .0;
    list.split(',')
        .map(|id| id.trim().parse().unwrap())
        .collect()
}

/// The block's list of weapon shots is the seed's: every ability some item
/// binds as its ranged basic attack (event 7), except one that items also
/// bind as a melee attack (event 6; Blade Melee AA, on two blades). A new
/// weapon family in the seed fails here until the block names its shot,
/// which needs a new patch id.
#[test]
fn weapon_shots_are_the_seeded_ranged_basic_attacks() {
    const MELEE: u32 = 6;
    const RANGED: u32 = 7;
    let bindings = seeded_bindings();
    let melee: BTreeSet<u32> = bindings
        .iter()
        .filter(|(_, event)| *event == MELEE)
        .map(|(ability, _)| *ability)
        .collect();
    let ranged: BTreeSet<u32> = bindings
        .iter()
        .filter(|(ability, event)| *event == RANGED && !melee.contains(ability))
        .map(|(ability, _)| *ability)
        .collect();
    assert_eq!(block_weapon_shots(), ranged);
}
