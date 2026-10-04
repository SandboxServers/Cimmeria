//! The committed `009-starter-hotbar` zip against its sources in the repo.
//!
//! The patched `ActionProfileDefault1.lua` is the stock file with
//! `data/client-patches/009-starter-hotbar/StarterHotbar.lua` appended byte
//! for byte. CI has no stock client, so these tests pin what can be checked
//! without one: the op's stock source and result hashes, and that the delta
//! builds a file exactly as long as the stock file plus the committed hook.
//! An edit to the hook without a rebuilt zip fails here; a rebuilt zip must
//! ship under a new patch id once 009 is published (append-only).

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::recipe::{Recipe, RECIPE_NAME};
use crate::sha256_hex;

const TARGET: &str = "Working/SGWGame/Content/UI/Core/ActionButtons/ActionProfileDefault1.lua";
/// The stock 2009 file (3470 bytes), as the cabinets ship it.
const STOCK_SHA256: &str = "a09eb055d5d806018c4307e1371e5851480a51b06db380b0afc8755c8c151387";
const STOCK_LEN: u64 = 3470;
/// Stock bytes followed by `StarterHotbar.lua`.
const RESULT_SHA256: &str = "9139475b0884be6e3af926548dff0853c9d6dadbb2db43807dfdd6b083e83912";
/// `StarterHotbar.lua` as built into the committed zip.
const HOOK_SHA256: &str = "f3eeecf2579b5806cc666d133db58f6f3c3be49160f5cb318c2392571f7a53f3";

fn patch_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/client-patches")
}

fn committed() -> (Recipe, Vec<u8>) {
    let file = std::fs::File::open(patch_dir().join("009-starter-hotbar.zip")).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut read = |name: &str| {
        let mut bytes = Vec::new();
        zip.by_name(name).unwrap().read_to_end(&mut bytes).unwrap();
        bytes
    };
    let recipe = Recipe::parse(&read(RECIPE_NAME)).unwrap();
    let delta = read(&recipe.ops[0].delta);
    (recipe, delta)
}

/// One op, on the stock-spelled file, pinned to the stock source.
#[test]
fn committed_009_patches_only_the_stock_default_profile_script() {
    let (recipe, _) = committed();
    assert_eq!(recipe.ops.len(), 1, "{recipe:?}");
    let op = &recipe.ops[0];
    assert_eq!(op.target, TARGET);
    assert_eq!(op.sources.len(), 1);
    assert_eq!(op.sources[0].path, TARGET);
    assert_eq!(op.sources[0].sha256, STOCK_SHA256);
    assert_eq!(op.result_sha256, RESULT_SHA256);
}

/// The delta's target is the stock file plus the hook in the repo, and
/// the hook is stored the way the client's UI scripts are: ASCII, CRLF.
#[test]
fn committed_009_appends_the_committed_hook() {
    let hook = std::fs::read(patch_dir().join("009-starter-hotbar/StarterHotbar.lua")).unwrap();
    assert!(hook.is_ascii(), "the hook must be ASCII");
    assert!(
        hook.windows(2).filter(|w| w == b"\r\n").count()
            == hook.iter().filter(|&&b| b == b'\n').count(),
        "the hook must use CRLF line endings (.gitattributes keeps them)"
    );

    let (_, delta) = committed();
    let target_len = qbsdiff::Bspatch::new(&delta).unwrap().hint_target_size();
    assert_eq!(
        target_len,
        STOCK_LEN + hook.len() as u64,
        "StarterHotbar.lua changed without a rebuilt 009 zip"
    );

    // Catches an edit that keeps the length.
    assert_eq!(
        sha256_hex(&hook),
        HOOK_SHA256,
        "StarterHotbar.lua changed without a rebuilt 009 zip"
    );
}
