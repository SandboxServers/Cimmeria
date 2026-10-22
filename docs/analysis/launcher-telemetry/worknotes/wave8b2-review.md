# Wave 8b stage 2 review: LA-04, LA-10

Reviewed: la-04 @ 201bbd02e, la-10 @ d228dcca9, both on base f8697d630 (LA-03, LA-09 and LA-12 merged, with the stage 1 fixes). Read-only; no builds run. `git merge-tree --write-tree 201bbd02e d228dcca9` is clean (tree 596edf735): la-04 touches only `crates/launcher-core/src/install*.rs`, la-10 only three files under `engine/src/storage/adoption/`. Neither branch changes a signature the other calls (`install_all*` keep their signatures; `apply_patch` and `verify_and_unpack` are private).

## Verdict

fix

LA-04: fix (N5, N6, both low; the code conforms and R-LA 10 holds). LA-10: fix (N1 medium, N2 and N3 medium test gaps, N4 and N7 low).

## Fix notes

N1 [LA-10, medium]. crates/launcher/desktop/engine/src/storage/adoption/light_replay.rs:156-183 (`stage`) and :124-127 — `stage` copies the folder file to scratch under the recipe's spelling (`inventory::native(scratch, path)`, `path` = the pinned path), not the folder's own spelling. `collect` then names the replacement by the scratch spelling. `patchset::apply` keeps the on-disk spelling of an existing file (`resolve_existing_case`, apply.rs doc: the published `005-login-delay` recipe names `eula.lua` while the stock file is `EULA.lua`). So a folder holding `.../EULA.lua` gives a `Replacement.path` of `.../eula.lua`. LA-11 overlays replacements on the folder scan, whose keys are case-sensitive (`BTreeMap<String, _>`), so it would carry both spellings into the digest and the staged tree. — In `stage`, keep the folder's spelling whenever the folder holds the file under the same name case-blind. Replace the lines from `let name = in_folder.get(&lower)...` to the end of the fn with:

```rust
    let direct = in_folder.get(&lower).copied();
    let name = direct.or_else(|| {
        let stock = stock_spelling(path)?;
        in_folder.get(&stock.to_ascii_lowercase()).copied()
    });
    let Some(name) = name else {
        return Ok(());
    };
    // The folder's own spelling when it holds the file; the placed cooked-data
    // spelling (`path`) only for a stock-named file that moves.
    let spelling = if direct.is_some() { name.as_str() } else { path };
    let dest = inventory::native(scratch, spelling);
    match std::fs::remove_file(&dest) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    inventory::copy(&inventory::native(folder, name), &dest, &scan[name], cancel)?;
    present.insert(lower, spelling.to_string());
    Ok(())
```

and in `run`, change `present.insert(output.path.to_ascii_lowercase(), output.path.clone());` to `present.entry(output.path.to_ascii_lowercase()).or_insert_with(|| output.path.clone());` so a staged spelling is not overwritten. Add the test `a_replacement_keeps_the_folders_spelling`: `patch_zip` with `target`/source `"working/data/overlay.dat"` (lower case), the folder holds `OVERLAY` (`Working/Data/Overlay.dat`) with the stock bytes; assert `done.replacements[0].path == OVERLAY` and `stored(&f, OVERLAY)` is the patched bytes. — verify: that test fails on the current code (the path comes back lower-case) and passes after; the other tests still pass.

N2 [LA-10, medium]. light_replay_tests.rs — D-LA9 "overlays always apply" and the spec's test 2 ("the overlay replacement exists" next to the skipped op) are not tested. Test 2 (`a_changed_source_is_skipped_and_recorded_not_refused`) uses two separate patches; the file it calls "overlay" (`OVERLAY`) is an op target, not a zip overlay. No test puts a skipped op and a non-recipe zip entry in the same patch, and no test uses `root: sgw_game`, so the `Working/SGWGame/` prefix branch at light_replay.rs:106-107 runs in no test. — add two tests. (a) `an_overlay_in_a_patch_with_a_skipped_op_still_lands`: `patch_zip("p0", &[OpFixture{ target: OVERLAY, sources: &[(OVERLAY, &big("stock overlay"))], result: &big("patched overlay") }], &[("Working/Data/Added.dat", b"added")])`; folder `(OVERLAY, b"modded by the player")`; assert `done.replacements` is exactly one entry with path `Working/Data/Added.dat` (stored bytes `added`) and `done.incomplete[0].skipped` is exactly `[SkippedTarget{ path: OVERLAY, reason: "source_mismatch" }]`. (b) `a_sgw_game_patch_records_its_skips_under_the_client_prefix`: add a `release_with_roots(zips: &[Vec<u8>], roots: &[&str])` that adds `"root": roots[i]` to each patch JSON (keep `release_of` calling it with the default root; check the serde name of the default in `core/manifest.rs` first), build the patch with the op on `"UI/Dialog.lua"` and source `"UI/Dialog.lua"`, root `sgw_game`, folder holding `Working/SGWGame/UI/Dialog.lua` = modded bytes; assert the skipped path is `"Working/SGWGame/UI/Dialog.lua"`. — verify: (a) fails if `ApplyOptions.only_targets` were set to `Some(..)` or tolerance dropped the overlay; (b) fails if the prefix literal is changed.

N3 [LA-10, medium]. light_replay_tests.rs, test `a_cooked_file_held_under_the_stock_name_is_a_move_not_a_replacement` — it covers the `collect` fallback only. Delete the `or_else(|| stock_spelling ...)` fallback in `stage` and every test still passes, yet that fallback is what lets a delta patch read a cooked PAK that the folder still holds under `Cache.en-US`. — add `a_cooked_source_held_under_the_stock_name_is_patched`: `patch_zip("p0", &[OpFixture{ target: COOKED_PLACED, sources: &[(COOKED_PLACED, &big("stock cooked"))], result: &big("patched cooked") }], &[])`; folder `[(COOKED_STOCK, &big("stock cooked"))]`; assert `f.plan.pending == vec![0]`, `done.incomplete.is_empty()`, `done.moves.is_empty()`, and `stored(&f, COOKED_PLACED) == big("patched cooked")`. — verify: with the fallback removed from `stage` the op is skipped as `source_missing` and the test fails.

N4 [LA-10, low]. light_replay.rs:17-19 — `SGW_GAME_DIR` is a second copy of the private constant at `crates/launcher-core/src/patch_summary.rs:26`; the literal `"Working/SGWGame/"` at :107 is a third. If core's value changes, the summary paths (which `stage` copies by) and the recorded skip paths diverge silently. — in patch_summary.rs make the constant `pub const SGW_GAME_DIR: &str = "Working/SGWGame";` (keep or add its doc comment). In light_replay.rs delete the local const and its doc, add `use cimmeria_launcher_core::patch_summary::SGW_GAME_DIR;`, and change :106-107 to `PatchRoot::InstallDir => (scratch.to_path_buf(), String::new()),` and `PatchRoot::SgwGame => (inventory::native(scratch, SGW_GAME_DIR), format!("{SGW_GAME_DIR}/")),`; then `format!("{prefix}{}", ...)` still works. — verify: N2(b) passes before and after; `git grep -n "Working/SGWGame" crates/launcher/desktop/engine/src/storage/adoption/light_replay.rs` shows nothing outside tests.

N5 [LA-04, low]. crates/launcher-core/src/install_tolerant.rs:84-103 (`retry_incomplete`) — the spec says "for each `IncompletePatch`, in manifest order"; the code walks the ledger order. They differ when a patch enters the manifest ahead of an incomplete one, or a retry reorders entries. A chained retry (patch 2 reads patch 1's target) then runs before patch 1. — replace the loop header and the `let Some(patch)` block (lines 88-98) with:

```rust
    for patch in &ctx.manifest.patches {
        let Some(incomplete) = state
            .incomplete_patches
            .iter()
            .find(|i| i.state_key == patch.state_key())
            .cloned()
        else {
            continue;
        };
```

(an entry whose patch left the manifest is never visited, so it is kept). Keep the body (`match retry_one(...)`), using `incomplete`. Update the doc comment to "in manifest order". — verify: new test `retry_runs_in_manifest_order`: a manifest of two patches `001`, `002` (two blobs on the server), a ledger whose `incomplete_patches` lists `002` before `001`, both applied; assert from `server.received_requests()` that `/p/001.zip` was requested before `/p/002.zip`. With the ledger-order loop the order is reversed.

N6 [LA-04, low]. install_tolerant_tests.rs — two spec rules of the retry have no test and survive being broken: "if its `PatchEntry` is missing, keep the entry" and the swallow of a failed retry (`patch_retry_failed`). — add (a) `an_entry_whose_patch_left_the_manifest_is_kept`: ledger `applied_patches: ["001","gone"]`, `incomplete_patches: [001 -> skipped_dialog(), "gone" -> skipped_dialog()]`, manifest `manifest_for(&zip)`, tolerant run; assert `Ok` and that `InstalledState::load(..).incomplete_patches` still holds both keys and `gone`'s `skipped` is unchanged. (b) `a_failed_retry_keeps_the_entry_and_the_run_succeeds`: a `MockServer::start().await` with no mounted mock (every request 404), ledger `applied_patches: ["001"]`, `incomplete_patches: [001 -> skipped_dialog()]`; the tolerant run returns `Ok(())` and the entry is unchanged. — verify: (a) fails if the missing-patch branch removes the entry; (b) fails if a non-cancel retry error is returned instead of logged.

N7 [LA-10, low]. light_replay_tests.rs:449-450 — `assert!(result.is_err(), ...)` passes for any strict failure (the same weakness stage 1 raised as N7 for LA-03). It also cannot fail when LA-10 is reverted (LA-10 does not touch `unpack`); it is only a guard on LA-03. — replace the two lines with `assert!(matches!(result, Err(crate::unpack::UnpackError::PatchsetHash { kind: "source_mismatch", .. })), "{result:?}");`. If `UnpackError` is not re-exported at `crate::unpack`, use `cimmeria_launcher_core::unpack::UnpackError`. — verify: the test passes; making the call use `tolerate_ops: true` turns it red.

## Not fixed

- Spec conformity LA-04: conforms. `InstallOptions` is passed to `install_all_into`, `apply_patch` and `verify_and_unpack`, with `only_targets` as the coder's reported deviation; the two public tolerant fns use `TOLERANT`, the existing ones `STRICT`. `retry_incomplete` is behind `if opts.tolerate_ops`; `record_skipped` is too, so the strict path never calls `record_incomplete` and its ledger bytes are unchanged. `apply_patch` is called with `STRICT, None` in the strict loop and `unpack_with(.., default)` is `unpack`. D-LA9: overlays are written (the `only_targets: None` apply writes them; test 1 asserts `hello.txt`), skipped ops are recorded per op, and the patch stays in `applied_patches` per D-LA18. R5: `patch_incomplete` carries `patch:<id>` and a count; `patch_retry_failed` carries `patch:<id>` and `InstallError::code()` (a closed list, `install_codes.rs`), no Display text.
- R-LA 10 for LA-04: the guard `strict_install_ignores_the_incomplete_list` fails when the `opts.tolerate_ops` guard at install.rs:271 is removed (the mock `.expect(0)` and the empty `received_requests`); I confirmed by reading that strict would request `/p/001.zip` in that case. `retry_clears_the_entry_once_the_source_is_restored` fails without the retry (the dialog stays modded and the entry stays). `strict_install_fails_on_the_same_input` fails if strict became tolerant.
- LA-04 edits an existing test: `install_tests.rs` `hash_mismatch_deletes_the_download` gains `InstallOptions::STRICT, None` in its `verify_and_unpack` call, and four helpers become `pub(super)`. The assertions are unchanged; the spec itself requires the new parameters, so this is signature fallout, not a test edited to pass. Accepted.
- File caps: `install.rs` is 573 lines (505 before; soft cap 500, hard 700), `install_tests.rs` 586, `install_tolerant.rs` 125, `install_tolerant_tests.rs` 233. The retry helper was moved out as the spec asks; the four `install_all_tolerant*` fns and `InstallOptions` (about 55 lines) could also move to `install_tolerant.rs`, but they need a `pub use` and the file was already over the soft cap. Accepted; ledger "install.rs 573, accepted at 8b2". LA-10: `light_replay.rs` 253, `light_replay_tests.rs` 451, `adoption/mod.rs` 469 (the packet may add call lines only; it adds one `mod` plus a comment and `#[allow(dead_code)]`, same as LA-09).
- `patch_incomplete` has no capture test (R-LA 6). `launcher-core` has no event-capture helper and no `tracing-subscriber` dev-dependency (`test_capture` lives in the engine crate), so a test needs either a dev-dependency or a hand-written `tracing::Subscriber`. That choice is outside this packet, and the spec's LA-04 test list does not ask for one. Hand-off to ops: decide whether the event is pinned through an engine-level test in LA-13.
- LA-09's `#[allow(dead_code)]` on `mod light_plan;` is not removed by LA-10 and cannot be: `plan` still has no caller outside tests until LA-11. LA-10 correctly adds the same attribute on `mod light_replay;` and rewrites the comment. Hand-off to LA-11: remove the attributes on `mod light_plan`, `mod light_replay` and LA-12's two (`scan_with_progress`/`sized`, and the file-level one in `index_compare.rs`), in the same commit that adds the callers.
- LA-10 reuses `light_plan::stock_spelling` (imported in light_replay.rs:8, used in `stage` and `collect`); there is no second spelling table. The only duplicate is the `SGW_GAME_DIR` constant (N4).
- LA-10 `replay` takes `release` and `sink` beyond the spec's signature (it needs patch ids/roots and the unpack sink). Acceptable; LA-11 must pass them.
- LA-10 `moves` holds only cooked files a patch touched. A stock-spelled cooked PAK that no pending patch reads or writes is neither staged nor recorded, so LA-11 must not assume `moves` is the whole `Cache.en-US` to `SourceCache.en-us` rename. `install_all_into` runs `place_bundled_cooked_data` for the rest on the first tolerant Update, but LA-11's digest and tree are computed before that. Hand-off to LA-11: decide whether to rename the remaining PAKs in `light_comparison` or to leave them out of the expected tree, and state which in its report.
- LA-01 contract limit (not LA-10's): `patch_summary` flattens an alternative into `Pinned.alternatives` only when the alternative's source path equals the primary's (patch_summary.rs:99). An alternative start that reads a different path is not in `sources`, so `stage` does not copy it and the op is skipped as `source_missing` where a start could have applied. Whether any published recipe has such an alternative is not known from the code; check against the published recipes before LA-13 relies on the plan. Owner/architect call.
- Test quality in LA-10 not worth a note: test 5 (`an_already_patched_folder_yields_no_replacements`) passes whenever `pending` is empty, so it tests the plan more than the replay; it is the spec's test and stays. The fixture re-implements `light_plan::plan`'s `have()`/pending step inline (light_replay_tests.rs:132-140) and `release_of`/`hex` repeat helpers in `light_plan_tests.rs`; a change to `plan`'s lookup would not reach these tests. Accepted for a test module.
- The coder's report says no mechanical revert of the LA-10 tests was run. I traced each by reading: tests 1 to 4, the cooked move, the closed-code refusal and the event test fail on the reversals the report names; the exceptions are covered by N2, N3 and N7.
- `replay` logs `tracing::warn!` on a cancel as well as on a failure (light_replay.rs:57-59), as `replay.rs:284-286` does. Mirrors the existing code; leave it.
- Cannot confirm: the lane logs under `logs\la-04\` and `logs\la-10\`. Neither directory exists in the main checkout or the worktrees, as in stage 1. Ops should confirm the lane ran from the worktree. The engine crate is tested with `cargo test`, not nextest (documented harness issue from stage 1).
- `InstalledState` literal fallout: none. Neither packet adds a field. All eight `InstalledState { .. }` literals in engine and shell on both branches end in `..Default::default()` (stage 1 N1 holds on this base).