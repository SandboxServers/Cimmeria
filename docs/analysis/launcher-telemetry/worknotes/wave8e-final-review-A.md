# Wave 8e final review A: parity, seedless gate, never-block, recovery

Branch `launcher-telemetry/light-adoption`, worktree `lt-light-integration`, tip `ebf8a46f3`, diff against `origin/main`. Scope A only. Items already ruled on in `wave8*-review.md` are not raised again.

## Verdict

pass

No verified defect. Three observations are listed under "Not fixed"; none needs a coder change before PR LA #1983 ships.

## Fix notes

None.

## Not fixed

Checked and found sound (each pointed to code, so the next reader need not redo it):

- R-LA 10, Update. `crates/launcher/desktop/engine/src/storage/update/incremental.rs:33-35`: `seedless_allowed` is `matches!(check, Some(FileCheck::Light))`. `None` (downloaded, old ledger), `Full` (copy-adopted, full in-place), an `Err` and a `Corrupt` result from `adoption_check()` all take the Full arm of `choose_path`, which is `seed_source_for(release)` plus strict `install_all` (`InstallOptions::STRICT`). `storage/update/{commit,recovery,staged_content}` are unchanged. Those install kinds therefore take the same Update path as on `origin/main`.
- R-LA 10, Repair. `storage/repair/preparation/mod.rs` is untouched: `seed_source_for(&installed.release)` and strict `install_all`. A light install repairs from the seed exactly like any other install. The provenance stays `check = light` afterwards. That is harmless: the next Update plans incrementally only if `incremental::plan` accepts the ledger, otherwise it rebuilds from the seed.
- R-LA 10, Launch. `launch/adoption_tags.rs` and `launch/worker.rs:64` only add the `adoption_check` and `patches_incomplete` telemetry fields. Admission is unchanged, so incomplete patches never block a launch.
- Seedless gate. `incremental::plan` (`incremental.rs:52-85`) returns `None` (seed rebuild) when `seed_adopted` is set, when the ledger seed sha differs from the target's seed (this covers an Update whose target has a different seed), when the applied patches are not an in-order prefix of the target's patches, or when an applied patch has a different sha in `previous` (a republished patch). `applied.len() > target.len()` is guarded before `target[applied.len()..]`, so the slice cannot panic. A second Update after an incremental one has an applied prefix equal to the previous target and plans only the new patches.
- Hard-link safety. `write_set` (`incremental.rs:203-235`) covers every pending patch output (plus the landing-dir alias), every `incomplete_patches[].skipped` path, `launcher-installed.json`, `SGW.exe` and `LoginInternal.lua`. These are the only in-place writers; everything else goes through temp file plus rename, which replaces the link and not the live content. A failed write set (`None`) maps to `write_set_unreadable` and a plain `Failure::Failed`; the live tree is untouched.
- D-LA9, panics. In the non-test code of `adoption/{light,light_plan,light_replay,light_order,index_compare,in_place_preview,in_place_replace}.rs` the only byte slices are `light.rs:131` (`&name[..16]`, `&name[28..]`) and `in_place_replace.rs:23-24` (`source[..27]`, `source[..16]`). Both follow `bundled_cooked_alias(..).is_some()`, which does `rel.get(..28)` and `eq_ignore_ascii_case("Working/SGWGame/Cache.en-US/")` (28 ASCII bytes). Offsets 16, 27 and 28 are therefore in range and on char boundaries. `light.rs:194` (`scanned[from]`) is filtered by `scanned.contains_key(from)`. `light.rs:240` (`summaries.len() - pending.len()`) and `light.rs:255` (`manifest.patches[i]`) use indexes that `light_plan::plan` builds from the same manifest. The index comparison is called with `?` only for cancel; `compared.state` carries unreachable, invalid and not-published as data (`light.rs:105-108`, `223-226`), so those states cannot block.
- Recovery and abandon. `place_cooked_tier` is idempotent (`plain_dir` false returns `Ok`), runs before any file moves on both ways, and refuses links and reparse points with `UnsafeFile`. `cooked_tier` returns `None` unless `check` is `Light`, so non-light plans never reach the rename. Abandon is allowed only in `Phase::Staged`, as before.

Observations that need no change now (owner or follow-up call):

1. Decision, low. The incremental stage fails (`Failure::Failed`, with `incremental_failed(reason)` telemetry) rather than falling back to a seed rebuild when the live tree cannot be staged: `stage_seed::seed_stage` refuses symlinks and reparse points, and `staged_content::digest` has UTF-8-name, 200k-entry and depth-64 limits. The live tree and the ledger stay intact and a retry reports the same reason, so nothing is half-written. The player can still repair or reinstall. Falling back to the seed on `seed_stage_failed` would add a 4.1 GB download the spec does not ask for; leave it unless the UAT shows real installs hitting it.
2. Info, low. `choose_path` reads the ledger and walks the patch lists while the state lock is held (`update/preparation/mod.rs`, around lines 199-231). The reads are one small JSON file, not a tree walk, so lock hold time is not a practical concern. Not verified under contention.
3. Info, low. `light.rs:141` still says "the original stays where the player left it". The comment is stale for the placed-tier case; it is already recorded as a known loose end in RESUME.md, so it is not raised as a note.

Not inspected (turn budget): the test modules, shell-host adoption wiring, and `adoption/index_source.rs` internals. D-LA9 for those is covered by `wave8c6-review.md`.