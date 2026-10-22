# Wave 8b stage 1 review: LA-03, LA-09, LA-12

Reviewed: la-03 @ 012585f27, la-09 @ 83d2a49cd, la-12 @ c4b64a6cb, base c4126f495. Read-only; no builds run. Pairwise `git merge-tree --write-tree` of the three branches: no conflicts (la-09 and la-12 both add one `mod` line to `adoption/mod.rs`, with an unchanged line between).

## Verdict

fix

LA-03: fix (N1 blocks the merge). LA-09: fix (N2, N3, N4). LA-12: fix (N5, N6, both low; the code itself conforms to the spec).

## Fix notes

N1 [LA-03, high]. crates/launcher/desktop/engine/src/storage/adoption/reference.rs:110-114 (and four test literals) — `InstalledState` gained `incomplete_patches`, but LA-03 only patched the struct literals inside `launcher-core`. `cimmeria_launcher_engine::state` is `pub use cimmeria_launcher_core::state`, so five literals elsewhere no longer compile (E0063): the production `reference::ledger` and four test fixtures. The core-only lane (366 passed) cannot see it. After LA-03 merges, every later engine lane (LA-09, LA-12, LA-04's engine clippy) fails. — add `..Default::default()` as the last field of each literal. Five sites, nothing else: (1) engine `storage/adoption/reference.rs`, in `ledger()`: after `applied_patches: manifest.patches.iter().map(|p| p.state_key()).collect(),` (line 113) add the line `..Default::default()`; (2) engine `storage/client_changes/tests.rs`, the literal at :59-61 (`seed_sha256`, `seed_adopted: false`, `applied_patches: ...`): add `..Default::default()` after the `applied_patches` expression, before the closing `}`; (3) engine `storage/install_recovery/tests.rs:52-56`: add `..Default::default()` after `seed_adopted: false,` (line 55); (4) shell `crates/launcher/desktop/shell/src/host/runtime_setup/tests.rs:128-132`: add `..Default::default()` after `seed_adopted: false,` (line 131); (5) shell `crates/launcher/desktop/shell/src/host/install/tests.rs:393-397`: add `..Default::default()` after `seed_adopted: false,` (line 396). Do not touch the `LegacyLedger` literals (`golden_tests.rs:117`, `migration/model.rs`); they are a different type. Add a commit on `launcher-telemetry/la-03-incomplete-ledger`; say in the commit body that the files are outside the packet list because the field addition breaks them. — verify: `Set-Location <la-03>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-launcher-engine --all-targets -- -D warnings` and the same for `-p cimmeria-launcher-desktop` both pass; `git grep -n -A1 "seed_adopted: false" -- crates/launcher/desktop` shows each `InstalledState` literal followed by `..Default::default()`.

N2 [LA-09, medium]. crates/launcher/desktop/engine/src/storage/adoption/light_plan.rs:68-73 — a `PatchSummaryError` is mapped to `artifact_hash_mismatch`. That is the wrong reason: `fetch_some` has already verified every blob's size and sha256 against the signed manifest, so a summary failure means the verified zip is unreadable, has an invalid recipe or names an unsafe path. The player and the frontend text table (`adoption-reasons.ts:31`, "A signed file did not match its download.") would blame the download. The existing replay uses the right codes for the same situation (`replay.rs:252` `patch_layout_unsupported`, `replay.rs:295` `patch_archive_unsupported`, entry `patch:<id>`); both are in `REJECTION_CHECKS` and in the frontend table, so no new code and no TypeScript change is needed. Do not invent a new code: the closed list is shared with the shell and the frontend. — replace the `map_err` closure at :68-73 with:

```rust
.map_err(|error| {
    let label = format!("patch:{}", patch.id);
    match error {
        patch_summary::PatchSummaryError::Path(_) => rejection::reject(
            Error::UnsupportedLayout,
            Rejection::new("patch_layout_unsupported").entry(label),
        ),
        patch_summary::PatchSummaryError::Io(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("patch_archive_unsupported").entry(label).found("io"),
        ),
        patch_summary::PatchSummaryError::Zip(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("patch_archive_unsupported").entry(label).found("zip"),
        ),
        patch_summary::PatchSummaryError::Recipe(_) => rejection::reject(
            Error::InvalidArtifact,
            Rejection::new("patch_archive_unsupported").entry(label).found("recipe"),
        ),
    }
})?
```

(`Error::UnsupportedLayout` matches `replay.rs:251`.) Never put the error's `Display` text in `found`; it can carry a path. Add the test from N4(b). — verify: that test asserts `Err(Error::InvalidArtifact)` and, from `rejection::scope(|| plan(..))`, `recorded.check == "patch_archive_unsupported"` and `entry == Some("patch:overlay")`; with the old code it sees `artifact_hash_mismatch` and fails.

N3 [LA-09, medium]. light_plan.rs:78-79 — `download_bytes` sums every patch, even blobs already in the store. The packet says "sum of the manifest sizes of the blobs `fetch_some` had to download, not cached ones; if `fetch_some` does not say, use every patch". `fetch_some` does not say, but `artifacts::held(state_root, release, cancel)` (artifacts.rs:103) is the existing call that does ("read before a fetch, so the review counts only what must download"). LA-11 shows `download_bytes` to the player in `LightSummary`; after a dismissed review (blobs kept in the store) it would again promise the full size. — before the `fetch_some` call (line 43) add `let held = artifacts::held(cache, release, cancel)?;`. Replace lines 78-79 with:

```rust
let download_bytes = manifest
    .patches
    .iter()
    .filter(|patch| !held.contains(&patch.sha256))
    .map(|patch| patch.size)
    .sum();
```

and delete the comment on line 78. `held` holds the `patch.sha256` strings exactly as the manifest spells them (artifacts.rs:117-120). — verify: new test `a_second_plan_over_the_same_store_downloads_nothing`: call `run(..)` twice with the same `state` dir and capture the second call's `adoption_light_planned` (capture-test standard, as `the_planned_event_carries_the_four_counts`); assert its `download_bytes == "0"`, and that the first call's `LightPlan.download_bytes` equals `patch0.len() + patch1.len()` taken from the `blobs` vector `release()` returns, not from the manifest. With the old code the second call reports the full sum.

N4 [LA-09, low]. crates/launcher/desktop/engine/src/storage/adoption/light_plan_tests.rs — three gaps where the change can be reverted and every test still passes. (a) The cooked-data fallback in `have()` (light_plan.rs:104, the `or_else(|| folder.get(&stock_spelling(path)?...))`) is reached only through the `stock_spelling` unit test, never through `plan`; delete the `or_else` and all tests pass. (b) The refusal path (N2) has no test. (c) The event test pins `event` but not `target`. — add: (a) test `a_placed_cooked_file_held_under_the_stock_name_counts_as_current`: refactor `release()` into `release_of(patch0: Vec<u8>, patch1: Vec<u8>)` (keep `release()` calling it with the two current zips); build a release whose patch 0 zip holds `("Working/SGWGame/SourceCache.en-us/Cooked.pak", b"cooked")` and whose patch 1 is the existing `zip_of(&[(EXTRA, EXTRA_OUT)])`; the folder holds `("Working/SGWGame/Cache.en-US/Cooked.pak", b"cooked")` and `(EXTRA, EXTRA_OUT)`; assert `planned.pending.is_empty()`. (b) test `a_patch_that_is_not_a_zip_is_refused_as_an_unsupported_archive`: `release_of(b"not a zip".to_vec(), <the current patch1 zip>)`, serve both blobs, run `rejection::scope(|| tokio::task::block_in_place(|| plan(..)))`, assert `Err(Error::InvalidArtifact)` and `recorded.check == "patch_archive_unsupported"`. (c) in `the_planned_event_carries_the_four_counts` add `assert_eq!(event.target, "adoption");` after the `.expect(..)` call. — verify: delete the `or_else` and (a) fails; revert N2 and (b) fails; change the macro's `target:` and (c) fails.

N5 [LA-12, low]. crates/launcher/desktop/engine/src/storage/adoption/index_compare.rs:146-166 — the D-LA17 caps of `sample` (10 paths, 600 characters) have no test. Raise `SAMPLE_PATHS`, drop the `.take(SAMPLE_PATHS)` or raise `SAMPLE_CHARS`, and all seven tests still pass, so one log field could carry a whole folder. — add in `mod tests` the unit test `a_sample_holds_at_most_ten_paths_and_600_characters_of_one_class` calling `sample(..)` directly: (1) 12 entries `(format!("p{i:02}"), FileClass::Changed)` plus 3 entries `("x".to_string(), FileClass::Extra)` mixed in: `sample(&files, FileClass::Changed)` split on `,` is exactly `p00..p09` (10 items, no `x`); (2) 10 entries `("a".repeat(100), FileClass::Missing)`: `sample(&files, FileClass::Missing).chars().count() <= 600` and it holds exactly 5 paths (5*100 + 4 commas = 504; a sixth would make 605); (3) `sample(&[], FileClass::Changed) == ""`. — verify: each assertion fails when its cap is removed.

N6 [LA-12, low]. index_compare.rs:299-306, test `the_compared_event_names_only_index_paths` — it pins `event`, `state`, two counts and the R5 absence, but not `target` (R-LA 6) and not the second sample. — after `assert_eq!(rows.len(), 1);` add `assert_eq!(rows[0].target, "adoption");`. After the `changed_sample` assertion add `assert!(!rows[0].field("missing_sample").unwrap().is_empty());` (the fixture has no `Working/Binaries/SGW.exe`, so the Missing sample is not empty). — verify: changing the macro's `target:` or sampling the wrong class into `missing_sample` fails the test.

N7 [LA-03, low]. crates/launcher-core/src/unpack/zip.rs, test `tolerant_unpack_returns_the_skipped_op_and_strict_still_fails`, last line `assert!(unpack_with(&zip_path, &dest, &sink, &ApplyOptions::default()).is_err());` — `is_err()` also passes when strict fails for an unrelated reason (a broken fixture, a missing delta). The R-LA 10 point is that strict still fails with the source mismatch it fails with today. — replace that line with `assert!(matches!(unpack_with(&zip_path, &dest, &sink, &ApplyOptions::default()), Err(super::super::UnpackError::PatchsetHash { kind: "source_mismatch", .. })));`. If the path does not resolve, import `UnpackError` from `super::super` in the test. — verify: the test still passes; making the strict call tolerate ops turns it red.

## Not fixed

- LA-12 `inventory.rs` is 502 lines (430 plus net 77 against the packet's 45; soft cap 500, hard cap 700). Judgment: accept, no split. The spec's own prescribed code (`sized` with a nested `add`, `scan_with_progress`, the two required `// LA-11 wires this` comments, and the `walk` call that rustfmt expands from 1 to 9 lines) comes to about 75 lines, so 45 was a spec estimate error, not padding. The file is 2 lines over the soft cap, nothing else is planned for it (LA-11 only calls `scan_with_progress`), and a split into `inventory/progress.rs` would add a `pub(super) use` and churn for 2 lines. Ledger: note "inventory.rs 502 lines, accepted at 8b1".
- LA-12 R-LA 10, `scan` byte-identical: verified from the diff. `scan` is `scan_inner(root, cancel, &mut |_| {})`; the body is unchanged apart from the extra `on_file` parameter, the rustfmt-expanded `walk(..)` calls, and `out.insert(name, hash(&path, cancel)?)` becoming `let entry = hash(..)?; let size = entry.size; out.insert(name, entry); on_file(size);` (same order: hash, then insert; same errors). No existing `inventory` or `verification_tests` test body is edited (only `seed_bytes` and `origin` become `pub(super)`). Coder's run: 766 passed, 0 failed. `sized` matches the spec step by step.
- `nextest` fails on `cimmeria-launcher-engine` ("creating test list failed": `harness = false` binaries print PASS lines). Pre-existing and documented in `.claude/agent-memory/rust-gameserver-dev/desktop-engine-tests-need-cargo-test.md`; LA-09 and LA-12 used `cargo test` through the lane, the documented substitute. Whether to fix the harness output or add a nextest filter is an ops/owner call outside these packets; until then R-LA 2 should say `cargo test` for the engine crate. LA-12 passed `--target-dir target/desktop` as that note says; ops should confirm it is the worktree-local directory (maintainer memory: no shared target dir on C:).
- LA-09 deviation 4 (5 lines in `adoption/mod.rs`, unconditional `#[allow(dead_code)]` on `mod light_plan;`) is acceptable. Add "remove LA-09's `#[allow(dead_code)]` on `mod light_plan;`" to the LA-10/LA-11 brief next to LA-12's two attributes.
- Hand-off for LA-10 (drift risk, not a fix): `light_plan::stock_spelling` (`pub(super)`) is the reverse of `install_layout::bundled_cooked_alias`. LA-10's replay needs the same reverse ("using `bundled_cooked_alias` for the stock spelling"). The LA-10 brief must say to call `light_plan::stock_spelling` and not write a second one. Its constants repeat `bundled_cooked_alias`'s (`install_layout.rs:88-89`); if a third caller appears, move it beside `bundled_cooked_alias` in `launcher-core`.
- The SHA-256-to-lower-hex fold is written inline again in `light_plan.rs:93` and `index_compare.rs:104`, and about nine adoption files have their own `hex()`. It is the module's existing pattern; not worth a shared helper in this wave.
- LA-03 edits existing core test literals (`install_tests.rs` x2, `state.rs` tests x4, `zip.rs` two `extract(..)` call sites) only to add `..Default::default()` or the new argument. No assertion changes, so R-LA 10's "never edit an existing test to make it pass" is not breached. Its R-LA 10 guard `ledger_without_incomplete_patches_roundtrips_byte_identically` is real (the coder showed it fails without `skip_serializing_if`).
- Capture-test standard and R5: LA-09's and LA-12's event tests follow `commands.rs:133-150` (module-local `SERIAL`, `_second` Dispatch held, `rebuild_interest_cache()` first in the closure, no await while the guard is held). LA-09's event carries counts only; LA-12's samples take only `Changed` and `Missing`, never `Extra`, and the test proves it with `my_screenshot_Steve`. Not findings.
- The lane log location (`logs\<worktree>\`) could not be checked: no `logs` directory exists in the three worktrees or in the main checkout. Ops should confirm when merging.