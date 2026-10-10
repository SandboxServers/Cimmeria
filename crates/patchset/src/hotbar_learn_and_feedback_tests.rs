//! The committed `016-hotbar-learn-and-feedback` zip against its sources in
//! the repo.
//!
//! 016 writes the same file as `009-starter-hotbar` and `015-weapon-shot-bar`,
//! `ActionProfileDefault1.lua`, and must leave both blocks as they are. The
//! patched file is 015's output with
//! `data/client-patches/016-hotbar-learn-and-feedback/HotbarLearnAndFeedback.lua`
//! appended byte for byte.
//!
//! As in `weapon_shot_bar_tests`, CI has no stock client, so the delta is
//! decoded against 015's output with the stock part (the first 3470 bytes)
//! replaced by `0x00` in one source and `0x01` in the other, followed by the
//! real 009 and 015 blocks. A byte that comes out the same from both is not
//! derived from a stock byte and must be the committed byte at that offset.
//! `real_client_hotbar_learn_and_feedback`, ignored, applies the zip to a
//! real client's file and compares every byte.
//!
//! An edit to the block without a rebuilt zip fails here. Once 016 is
//! published, a rebuilt zip must ship under a new patch id (append-only).

use std::path::{Path, PathBuf};

use crate::apply::bspatch;
use crate::debug_area_rings_tests::{entry, recipe};

const ID: &str = "016-hotbar-learn-and-feedback";
const WEAPON_SHOT_BAR: &str = "015-weapon-shot-bar";
const TARGET: &str = "Working/SGWGame/Content/UI/Core/ActionButtons/ActionProfileDefault1.lua";
/// The stock 2009 file, as the cabinets ship it.
const STOCK_LEN: usize = 3470;
/// 015's output followed by `HotbarLearnAndFeedback.lua`.
const RESULT_SHA256: &str = "8d3a2190dde2a1ac6029908a8eae8fb668c68395293b608e64713c056da3a627";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Vec<u8> {
    std::fs::read(repo().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn block() -> Vec<u8> {
    read("data/client-patches/016-hotbar-learn-and-feedback/HotbarLearnAndFeedback.lua")
}

/// 009's hook followed by 015's block: what 015 leaves after the stock part.
fn earlier_blocks() -> Vec<u8> {
    let mut b = read("data/client-patches/009-starter-hotbar/StarterHotbar.lua");
    b.extend_from_slice(&read(
        "data/client-patches/015-weapon-shot-bar/WeaponShotBar.lua",
    ));
    b
}

/// 016 starts from 015's output and nothing else: a client without 015 is
/// refused, and a client with it gets 009's and 015's bytes back unchanged.
#[test]
fn committed_016_starts_from_015_output_only() {
    let (r016, z016) = recipe(ID);
    let (r015, _) = recipe(WEAPON_SHOT_BAR);
    assert_eq!(r016.ops.len(), 1, "{r016:?}");
    let op = &r016.ops[0];
    assert_eq!(op.target, TARGET);
    assert_eq!(op.sources.len(), 1, "{op:?}");
    let source = &op.sources[0];
    assert_eq!(source.path, TARGET);
    assert_eq!(
        source.sha256, r015.ops[0].result_sha256,
        "016 must start from the exact file 015 writes"
    );
    assert_eq!(source.output_of.as_deref(), Some(WEAPON_SHOT_BAR));
    assert!(op.alternatives.is_empty(), "{op:?}");
    assert_eq!(op.result_sha256, RESULT_SHA256);
    // The recipe and the one delta: no whole files.
    assert_eq!(z016.len(), 2);
}

/// The block is stored the way the client's UI scripts are: ASCII, CRLF.
#[test]
fn hotbar_learn_and_feedback_block_is_ascii_crlf() {
    let block = block();
    assert!(block.is_ascii(), "the block must be ASCII");
    let lf = block.iter().filter(|&&b| b == b'\n').count();
    let crlf = block.windows(2).filter(|w| w == b"\r\n").count();
    assert_eq!(
        crlf, lf,
        "the block must use CRLF line endings (.gitattributes keeps them)"
    );
}

/// Decode the delta: 015's output comes back unchanged, and everything after
/// it is the committed block.
#[test]
fn committed_016_delta_keeps_015_and_appends_exactly_the_committed_block() {
    let block = block();
    let earlier = earlier_blocks();
    let (r016, mut z016) = recipe(ID);
    let delta = entry(&mut z016, &r016.ops[0].delta);

    let source = |stock_byte: u8| {
        let mut s = vec![stock_byte; STOCK_LEN];
        s.extend_from_slice(&earlier);
        s
    };
    let zeros = bspatch(&source(0), &delta).unwrap();
    let ones = bspatch(&source(1), &delta).unwrap();

    let mut ours = earlier.clone();
    ours.extend_from_slice(&block);
    assert_eq!(
        zeros.len(),
        STOCK_LEN + ours.len(),
        "the delta builds a file of another length: HotbarLearnAndFeedback.lua changed without a rebuilt 016 zip"
    );

    for i in 0..STOCK_LEN {
        assert!(
            zeros[i] == 0 && ones[i] == 1,
            "output byte {i} is not a plain copy of the stock byte"
        );
    }

    let mut pinned = 0usize;
    for (k, &want) in ours.iter().enumerate() {
        let i = STOCK_LEN + k;
        if zeros[i] == ones[i] {
            let part = if k < earlier.len() {
                "009's or 015's block"
            } else {
                "the 016 block"
            };
            assert_eq!(
                zeros[i], want,
                "output byte {i} ({part}) differs from the committed source: \
                 HotbarLearnAndFeedback.lua changed without a rebuilt 016 zip, or the delta changes an earlier block"
            );
            pinned += 1;
        }
    }
    assert!(
        pinned * 100 >= ours.len() * 97,
        "only {pinned} of {} bytes after the stock part are pinned",
        ours.len()
    );
}

/// The no-shot line is the server's right-click line, word for word, so the
/// player reads one message for the same state whichever way they fire.
#[test]
fn no_shot_line_is_the_servers_right_click_line() {
    let server = String::from_utf8(read(
        "crates/cell-methods/src/cell/cell_methods/player/interaction/hostile_attack.rs",
    ))
    .unwrap();
    let text = server
        .split_once("NO_RANGED_ATTACK_TEXT: &str = \"")
        .expect("NO_RANGED_ATTACK_TEXT in hostile_attack.rs")
        .1
        .split_once('"')
        .unwrap()
        .0;
    let block = String::from_utf8(block()).unwrap();
    assert!(
        block.contains(&format!("Text = '{text}',")),
        "the block's NoShotFeedback.Text is not the server's {text:?}"
    );
}

// Needs the client's own file: ActionProfileDefault1.lua with 009 and 015
// applied (a launcher-installed client has it). Set SGW_015_DEFAULT_PROFILE to
// that file. The zip must rebuild it into the same bytes followed by the
// block, say "already current" the second time, and refuse a file with 009
// only.
//   cargo test -p cimmeria-patchset real_client_hotbar_learn_and_feedback -- --ignored --nocapture
#[test]
#[ignore = "needs a client's ActionProfileDefault1.lua with 009 and 015 applied; see the comment"]
fn real_client_hotbar_learn_and_feedback() {
    let path = std::env::var("SGW_015_DEFAULT_PROFILE").unwrap();
    let with_015 = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let zip = repo().join(format!("data/client-patches/{ID}.zip"));
    let block = block();
    let earlier = earlier_blocks();
    assert!(
        with_015.ends_with(&earlier),
        "the file does not end with 009's and 015's blocks"
    );
    assert_eq!(with_015.len(), STOCK_LEN + earlier.len());

    let tree = tempfile::tempdir().unwrap();
    let to = tree.path().join(TARGET);
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::write(&to, &with_015).unwrap();
    let report = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(report.rebuilt, vec![TARGET.to_string()]);
    let rebuilt = std::fs::read(&to).unwrap();
    assert_eq!(crate::sha256_hex(&rebuilt), RESULT_SHA256);
    assert_eq!(
        &rebuilt[..with_015.len()],
        &with_015[..],
        "015's output changed"
    );
    assert_eq!(
        &rebuilt[with_015.len()..],
        &block[..],
        "the appended bytes are not the block"
    );
    let again = crate::apply(&zip, tree.path(), &mut |_| {}).unwrap();
    assert_eq!(again.already_current, vec![TARGET.to_string()]);

    // A client with 009 only is not a source of this patch.
    let hook_len = read("data/client-patches/009-starter-hotbar/StarterHotbar.lua").len();
    let with_009 = &with_015[..STOCK_LEN + hook_len];
    std::fs::write(&to, with_009).unwrap();
    assert!(crate::apply(&zip, tree.path(), &mut |_| {}).is_err());
    assert_eq!(
        std::fs::read(&to).unwrap(),
        with_009,
        "the 009-only file was touched"
    );
}
