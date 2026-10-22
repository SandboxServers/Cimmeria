# Wave 8b4 review: LA-11b (Light places the bundled cooked-data tier)

Worktree `.claude/worktrees/la-11b`, commit e3978189c (base 571e3f932). Spec: work-packets-light-adoption.md (R-LA, LA-11b) and wave8b3-review.md "Not fixed" 1. Read-only; no build was run (findings are from the code).

## Verdict

fix (one high note, two small ones)

## Fix notes

N1. HIGH. `crates/launcher/desktop/engine/src/storage/adoption/in_place_replace.rs:51-55` (`place_cooked_tier`) -- a folder that already holds both tiers and gets a stored move fails to publish with `Corrupt`. The stored-move rows that `light.rs` still builds when `place` is false (`light.rs` the `for (from, to) in moves` loop, `source_path: Some(from)`, `path: to`, `NeedsPatching`) look exactly like a rename row to `cooked_tier` (alias source, path differs). So `cooked_tier` returns `Some`, the stock dir exists, the placed dir exists and is not empty, `remove_dir` fails, and the packet's own `Corrupt` branch aborts the publish at the first line of `move_files`. Concrete input: `light_tests::a_stock_named_cooked_file_is_stored_as_a_move`'s fixture (Cache.en-US/Cooked.pak, SourceCache.en-us/Other.pak, a patch shipping SourceCache.en-us/Cooked.pak) previews fine and then fails on confirm. The spec says both-tiers behaves "exactly as today"; nothing in the diff publishes this case, so the break is untested. The spec's `Corrupt` rule is the cause (the architect text needs the same amendment in LA-11b Shape 2). Change: replace
```
    if plain_dir(&placed)? && std::fs::remove_dir(&placed).is_err() {
        tracing::error!(target: "adoption", "light adoption: both cooked-data tiers are in the folder");
        return Err(StorageError::Corrupt.into());
    }
```
with
```
    if plain_dir(&placed)? && std::fs::remove_dir(&placed).is_err() {
        // Both tiers hold files (a stored-move row looks like a rename row): nothing to place.
        tracing::debug!(target: "adoption", "light adoption: both cooked-data tiers hold files; the directory stays as it is");
        return Ok(());
    }
```
Also amend the LA-11b Shape 2 text in work-packets-light-adoption.md (`place_cooked_tier`, second bullet: "log ... and return `Err(StorageError::Corrupt.into())`" becomes "log at debug and return `Ok(())`", CRLF). Safe failure is kept: if a placed file really appeared after the review in a place plan, `move_files` finds the file in neither place and returns `Corrupt` (in_place.rs:441-444). `restore_cooked_tier` already treats (true, true) as a warn-only no-op. verify: N2's test fails before the edit (confirm returns `Corrupt`) and passes after.

N2. MEDIUM. `crates/launcher/desktop/engine/src/storage/adoption/light_tests.rs`, end of `a_stock_named_cooked_file_is_stored_as_a_move` (after the last `assert!(preview.replacements()...)`) -- the coder edited this test to keep a both-tiers fixture, but it only previews, so it never exercises the publish of that fixture (the packet said existing tests pass unedited; the edit is accepted, see Not fixed 1, only if it pins publish). Change: append, after the last assert, `commit(preview).unwrap();` then
```
    let game = lt.game();
    assert_eq!(std::fs::read(inventory::native(&game, PLACED)).unwrap(), b"cooked");
    assert_eq!(std::fs::read(inventory::native(&game, "Working/SGWGame/SourceCache.en-us/Other.pak")).unwrap(), b"other");
    assert_eq!(std::fs::read(inventory::native(&lt.folder, STOCK)).unwrap(), b"cooked");
```
(`commit` is already imported at line 4; the `preview` borrow ends at the last assert; the folder keeps the stock original, as the comment in `light.rs` says. If `content_valid(&game, &lt.verified())` holds for this release, add it too.) verify: apply N2 without N1: the test fails with `Corrupt`; with N1: passes. Also remove nothing else.

N3. LOW. `in_place_replace.rs:14-27` (`cooked_tier`) -- the `FileCheck::Light` guard is the only thing that keeps full-check and copy adoption (R-LA 10) from a directory rename, and no test fails if it is removed: the full path's rows never have an alias source with a differing path (verification.rs:393-436 folds case only), so no existing test reaches the guard. Change: make `cooked_tier` `pub(super)` and add to `light_cooked_tests.rs` one test: build the stock-tier Light preview (`stock_tier(&[]).await.preview().unwrap()`), clone its plan (grep how `commit` in `in_place_tests.rs` obtains the `Plan` from a `Preview`), assert `cooked_tier(&plan)` is `Some(("Working/SGWGame/Cache.en-US".into(), "Working/SGWGame/SourceCache.en-us".into()))`, then set `plan.provenance.check = FileCheck::Full` and assert `cooked_tier(&plan).is_none()`. verify: delete the `matches!(.., FileCheck::Light)` early return in `cooked_tier`; the test must fail.

## Not fixed

1. Edited existing test (`light_tests::a_stock_named_cooked_file_is_stored_as_a_move`, plus `pub(super)` on `Light` and helpers). The packet said to return `blocked` if an existing light test fails. The failure is real and caused by the packet's intended behaviour (that fixture is the stock-only split-tree case), the coder flagged it, and the edit keeps the stored-move path alive with a both-tiers fixture. Accepted once N2 makes it publish; the orchestrator should record the deviation in the ledger. No golden, downloaded, full-check or in_place test was touched.
2. Both-tiers folder: the stock-only PAKs stay under `Cache.en-US` in `game/`, so the tier is not "whole under SourceCache.en-us" there. This is the spec's decided rule (same as `place_bundled_cooked_data`, which no-ops when `SourceCache.en-us` exists) and a patch-produced `SourceCache.en-us` file in such a folder lands as a new file with `source_path = None`. Accepted by design; a future change would need a per-file rename and is out of this packet.
3. Checked and fine: placed-name slices (`[..16]`, `[28..]`, `[..27]`) match `bundled_cooked_alias` (STOCK is 27 chars plus `/`); a patched PAK's old stock copy is dropped by the `replaced` skip and its replacement row carries the stock `source_path`, so `replace_file` sets the original aside from `from/<placed path>` after the rename; recovery from `BeforePromotion` and `Moved` and abandon both reach `place_cooked_tier`/`restore_cooked_tier` through `move_files` and are idempotent (stock absent means return); empty leftover placed dir is removed then renamed; a Full plan returns `None` so full in-place, copy and downloaded installs, `golden_tests` PLAN_BYTES/PLAN_DIGEST and the ledger bytes are untouched (no plan or ledger type changed); file sizes by line count: in_place.rs 692 (cap 692), publication.rs 699 (untouched), light.rs 364, in_place_replace.rs 369, light_cooked_tests.rs 153, light_tests.rs 472, all under 700 and the split is clean (new code in the named file only); the new `mod` line is cfg-gated like its neighbours; migration.md has one added sentence and is all CRLF (455 CRLF, 0 LF); new log lines carry no path, URL or user field (R5), static messages, debug/warn/error only; no debug leftovers or unrelated edits. Revert-sensitivity: removing the two Shape 3 calls fails the placed tree, abandon, recovery and empty-dir tests (B stays under `Cache.en-US`; the stock dir is never restored); the review-rows and both-tiers tests pin Shape 1 (a wrong `place` makes the both-tiers publish fail). Not confirmed by running them.