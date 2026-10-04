//! The committed `009-starter-hotbar` zip against its sources in the repo.
//!
//! The patched `ActionProfileDefault1.lua` is the stock file with
//! `data/client-patches/009-starter-hotbar/StarterHotbar.lua` appended byte
//! for byte. CI has no stock client, so the delta is decoded against two
//! synthetic sources of the stock length, all `0x00` and all `0x01`. A byte
//! the delta adds from its extra block comes out the same from both; a byte
//! it derives from the source comes out as `diff + 0` and `diff + 1`. That
//! pins, without the stock file:
//!
//! - the first 3470 output bytes are the source copied unchanged (diff 0), so
//!   the delta never alters a stock byte and stores none;
//! - every extra-block byte equals the hook byte at the same offset, and
//!   extra-block bytes cover nearly all of the hook.
//!
//! A hook edit without a rebuilt zip fails here. Once 009 is published, a
//! rebuilt zip must ship under a new patch id (append-only).

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::apply::bspatch;
use crate::recipe::{Recipe, RECIPE_NAME};

const TARGET: &str = "Working/SGWGame/Content/UI/Core/ActionButtons/ActionProfileDefault1.lua";
/// The stock 2009 file (3470 bytes), as the cabinets ship it.
const STOCK_SHA256: &str = "a09eb055d5d806018c4307e1371e5851480a51b06db380b0afc8755c8c151387";
const STOCK_LEN: usize = 3470;
/// Stock bytes followed by `StarterHotbar.lua`.
const RESULT_SHA256: &str = "159bf4e8424586abccd593bea8fbfece5b389d8ace0ceada550a550df2e53414";

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

fn hook() -> Vec<u8> {
    std::fs::read(patch_dir().join("009-starter-hotbar/StarterHotbar.lua")).unwrap()
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

/// The hook is stored the way the client's UI scripts are: ASCII, CRLF.
#[test]
fn starter_hotbar_hook_is_ascii_crlf() {
    let hook = hook();
    assert!(hook.is_ascii(), "the hook must be ASCII");
    let lf = hook.iter().filter(|&&b| b == b'\n').count();
    let crlf = hook.windows(2).filter(|w| w == b"\r\n").count();
    assert_eq!(
        crlf, lf,
        "the hook must use CRLF line endings (.gitattributes keeps them)"
    );
}

/// Decode the delta and compare the bytes it adds against the hook source.
#[test]
fn committed_009_delta_adds_exactly_the_committed_hook() {
    let hook = hook();
    let (_, delta) = committed();
    let zeros = bspatch(&vec![0u8; STOCK_LEN], &delta).unwrap();
    let ones = bspatch(&vec![1u8; STOCK_LEN], &delta).unwrap();

    assert_eq!(
        zeros.len(),
        STOCK_LEN + hook.len(),
        "the delta builds a file of another length: StarterHotbar.lua changed without a rebuilt 009 zip"
    );

    // The stock part is the source, copied unchanged.
    for i in 0..STOCK_LEN {
        assert!(
            zeros[i] == 0 && ones[i] == 1,
            "output byte {i} is not a plain copy of the stock byte"
        );
    }

    // Every byte the delta stores is the hook's byte at that offset.
    let mut from_extra = 0usize;
    for (k, &want) in hook.iter().enumerate() {
        let i = STOCK_LEN + k;
        if zeros[i] == ones[i] {
            assert_eq!(
                zeros[i], want,
                "hook byte {k} differs from the delta: StarterHotbar.lua changed without a rebuilt 009 zip"
            );
            from_extra += 1;
        }
    }
    // The rest of the hook is derived from stock bytes by bsdiff, which this
    // test cannot check without the stock file; keep that share small.
    assert!(
        from_extra * 10 >= hook.len() * 9,
        "only {from_extra} of {} hook bytes are pinned",
        hook.len()
    );
}
