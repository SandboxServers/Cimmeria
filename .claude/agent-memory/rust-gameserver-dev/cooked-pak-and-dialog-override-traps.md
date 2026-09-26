---
name: cooked-pak-and-dialog-override-traps
description: data/cache/*.pak IS in git (several docs claim otherwise, so tests avoid it needlessly); a fail-closed dialog patcher plus a seed-only linter diverge silently; content-engine cannot see services
metadata:
  type: project
---

Three traps found while filling the Castle dialog patch table (packet DU-02b, 2026-09-21).

**`data/cache/*.pak` is committed to git.** All twenty cooked archives, including
`CookedDataDialogs.pak` (2.7 MB), are tracked — check with `git ls-files data/cache/`.
`crates/resources/src/base/resources/tests/committed_paks.rs` has been reading that
directory in tests for a long time.

**Why:** several docs assert the opposite — the DU-01 acceptance line in
`docs/analysis/dialog-ui-redesign/work-packets.md` ("The PAK is not in git, so no test
may read `data/cache/`") and the `dialog_button_linter.rs` module doc. Both are wrong,
and believing them costs you the only test that can see the real cooked data.

**How to apply:** when a test needs to prove something about shipped cooked content
(a screen id exists, a button's `ButtonType`/`ButtonID`/`Text`, an entry's shape), read
the archive. `ResourceCache::load_pak` is private to the `resources` module, so from
elsewhere in `cimmeria-services` open it with `zip::ZipArchive` directly; entries are
named `_<id>`. Verify the claim in the doc before repeating it.

**A fail-closed patcher plus a seed-only linter diverge in silence.**
`apply_dialog_patch` refuses an `OnlyOn` plan whose `screen_id` is absent from the cooked
entry (`PatchError::ScreenMissing`) and the caller keeps the canonical bytes. Correct for
an unattended server, invisible to everyone else: the seed still says the dialog is fixed,
the DU-L linter reads only the seed and agrees, and the player still gets the old broken
layout. There is no red test anywhere once the dialog leaves the linter's allowlist.

**How to apply:** any patch table that transforms shipped data needs a guard that applies
every row to the **real** input, not just one asserting the patch and its parallel seed
record agree with each other. Two records agreeing is not evidence either matches the
third. See `crates/resources/src/base/dialog_overrides/patch_seed_agreement_castle.rs`,
whose three tests are deliberately independent: agreement, final-screen rule, and
applies-to-cooked-entry each fail on a different mistake.

**`cimmeria-content-engine` does not depend on `cimmeria-services`** — the arrow points
the other way (`crates/services/Cargo.toml`). A test that has to see both a Rust table in
`services` and a seed file therefore cannot live in the content-engine test crate, however
much it looks like linter work. It goes in `services` and reads the seed with
`env!("CARGO_MANIFEST_DIR")`, two hops up to the workspace root.

Related: [[content-chain-authoring-traps]], [[dialog-set-bind-routing-and-edges]].
