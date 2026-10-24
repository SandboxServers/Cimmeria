# Wave 8a review (LA-01, LA-02, LA-05, LA-06, LA-07, LA-08)

Reviewed heads: la-01 94b311ebe, la-02 defa88e29, la-05 ec18ea33a (base origin/main e7487da3e), la-06 6fdc959fc, la-07 12d1d734c, la-08 47264fcb8 (all others on L tip e6f16395a). Both halves covered. Nothing was built or run; findings come from reading the diffs and the code around them.

## Verdict

fix (LA-05 and LA-08 need fixes; LA-01, LA-02, LA-06, LA-07 ship). verdicts: LA-01:ship, LA-02:ship, LA-05:fix, LA-06:ship, LA-07:ship, LA-08:fix.

R-LA 10 (downloaded installs): no change found on any fresh-install, full Update or Repair path.
- LA-02: `apply` is `apply_with(.., &ApplyOptions::default(), ..)`. With defaults every early return is the same error as before (`_ => return Err(error)`, `Err(detail) => return Err(PatchsetError::Delta{..})`, the same `ResultMismatch`), the write order and overlays are untouched, and `skipped` stays empty. No existing test is edited (the diff only adds tests). Guard: `default_options_still_fail_a_mismatched_source_and_write_nothing` fails if tolerance becomes the default.
- LA-01, LA-06, LA-08: new modules with no caller anywhere (grep of `seed_stage|stage_seed`, `patch_summary`, `file_index::compare`, `bundled_cooked_alias` finds only the definitions). LA-08 wires no caller, as required.
- LA-05: touches only the publication checker (`check_publication`) and the runbook; no runtime launcher code. `manifest::blob_url` already passes any absolute https URL through, so a cross-host index blob resolves at runtime.
- LA-07: deletes `adopt_existing_install` and `AdoptError`; grep over the whole worktree finds no caller outside the deleted tests. The `PathBuf` import removal is correct; `info!`, `StateError`, `install_layout` and `fake_manifest` are still used.

Merge checks: `git merge-tree --write-tree` for all 15 pairs of the six branches exits 0 (no conflicts); the `lib.rs` pair LA-01/LA-08 merges clean. LA-05 merged with L tip e6f16395a is clean, and `git diff --stat origin/main e6f16395a -- check.rs launcher-distribution-setup.md` is empty (no PR L / PR I2 overlap). Merge order 07, 02, 01, 06, 08 has no hazard.
Hygiene: no staged `launcher_targets.txt`, `dist` or `node_modules` in any of the six; docs CRLF (the LA-05 doc file has 476 CR and 476 LF; the worknotes are CRLF too); no IP, path or URL in a telemetry row (none of the six packets emits telemetry); file caps: patch_summary.rs 358, compare.rs 272, stage_seed.rs 247, apply.rs 340; the only soft-cap overrun is `patchset/src/tests.rs` at 510 (see Not fixed).

## Fix notes

LA-05 (PR I2)

N1 [LA-05]. docs/client/launcher-distribution-setup.md:156 — the heading says the publish steps run "after the `content-current` swap in step 5", but the list itself backs up, signs, uploads and swaps, so an operator reading it would swap first and back up after, which defeats the rollback — change `- Publish and roll back (owner only, after the `content-current` swap in step 5):` to `- Publish and roll back (owner only; this list expands step 5 above and replaces its upload and swap order):` — verify: Grep `after the .content-current. swap` in the file returns nothing; steps 1-7 still read backup, sign, verify, upload, swap, re-check, rollback; CR count equals LF count.

N2 [LA-05]. docs/client/launcher-distribution-setup.md:145,146,151,154,155 — the older bullets still say the index is attached to "the immutable per-publication release, next to the seed and patches" and that `blob` is "the attached asset name" / `-BlobPath <blob name>`, which contradicts the new bullet at :155 (a separate release `content-file-index-<yyyymmdd>`, `-BlobPath` the full https URL) — make these edits, keeping CRLF:
- :145 replace `Attach `file-index.json` to the immutable per-publication release, next to the seed and patches.` with `Attach `file-index.json` to its own release, `content-file-index-<yyyymmdd>` (see the publication bullet below).`
- :146 replace `set its `blob` to the attached asset name.` with `set its `blob` to the attached asset's full https URL.`
- :151 replace `-BlobPath <blob name>` with `-BlobPath <full https URL of the file-index.json asset>`.
- :154 replace `Upload `file-index.json` to the immutable per-publication release first` with `Upload `file-index.json` to the `content-file-index-<yyyymmdd>` release first`.
- :155 delete `, next to the seed and patches` after `content-file-index-<yyyymmdd>`.
— verify: Grep `immutable per-publication release|attached asset name|<blob name>|next to the seed and patches` in the file returns nothing; the diff shows only those lines changed, all CRLF.

N3 [LA-05]. docs/client/launcher-distribution-setup.md:162 — step 6 cannot run as written: `verify-file-index` requires `--manifest` and `--file-index` (crates/launcher-core/src/bin/verify-file-index.rs:6, :115) and `--previous` takes the backed-up manifest — replace `Download the release again and run `verify-file-index --previous <backup>`.` with `Download the live `manifest.json` and the new `file-index.json` again and run `verify-file-index --manifest <downloaded manifest.json> --file-index <downloaded file-index.json> --previous <backup>\manifest.json`. On any FAIL, roll back at once (step 7).` — verify: the command line names `--manifest`, `--file-index` and `--previous`; CRLF intact.

LA-08

N4 [LA-08]. crates/launcher-core/src/stage_seed.rs:52-60 — only entries under `src` are checked for links; `src` itself may be a junction or symlink, which `read_dir` and `hard_link` then follow, against "a symlink or reparse point anywhere" in the spec — after the `dst` AlreadyExists check (line 57, before `let mut plan`) insert:
    if is_link(&fs::symlink_metadata(src)?) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "seed source is a symbolic link or reparse point",
        ));
    }
— verify: the new test `a_junction_as_src_is_refused` (N5) fails when these lines are removed.

N5 [LA-08]. crates/launcher-core/src/stage_seed.rs:202-223 — `a_symlink_in_src_is_refused` returns early ("skipped") whenever the session cannot create a symlink, the usual case on Windows without Developer Mode, so the `FILE_ATTRIBUTE_REPARSE_POINT` branch of `is_link` is then never tested — add these after it (junction via `cmd /C mklink /J`, which needs no elevation; assert instead of skipping):
    #[cfg(windows)]
    fn junction(link: &Path, target: &Path) {
        let out = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .unwrap();
        assert!(out.status.success(), "mklink /J failed: {out:?}");
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_in_src_is_refused() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let dst = out.path().join("stage");
        junction(&tmp.path().join("game/alias"), &tmp.path().join("game/data"));
        let err = seed_stage(tmp.path(), &dst, &BTreeSet::new(), true, &never).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!dst.exists());
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_as_src_is_refused() {
        let tmp = source();
        let out = tempfile::tempdir().unwrap();
        let alias = out.path().join("alias");
        junction(&alias, tmp.path());
        let dst = out.path().join("stage");
        let err = seed_stage(&alias, &dst, &BTreeSet::new(), true, &never).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(!dst.exists());
    }
— verify: with `| reparse` removed from `is_link` (line 127) `a_junction_in_src_is_refused` fails; with the N4 lines removed `a_junction_as_src_is_refused` fails; both pass restored. Say so in the coder report.

## Not fixed

- LA-02 `patchset/src/tests.rs` is 510 lines (was 422): over the 500 soft cap, under the 700 hard cap. Splitting needs `Fixture`, `fixture()` and `zip_to_disk` widened to `pub(crate)` (existing lines), which the spec's no-edit rule discourages. Accept; move later tests to a new `tolerant_apply_tests.rs` if the next patchset packet adds more.
- LA-02 tolerant branches `DeltaFailed` and `ResultMismatch` have no test (the spec lists four tests, none for them), so deleting either `continue` branch would pass the suite. Optional follow-up: hand-build a zip whose recipe has a matching source sha and (a) a garbage delta, (b) a wrong `result_sha256`; assert `skipped` holds the matching reason and strict returns `Delta` / `ResultMismatch`.
- LA-02 `default_options_still_fail_a_mismatched_source_and_write_nothing` is equivalent to the existing `a_non_stock_source_fails_and_writes_nothing` (`apply` is the same call). Harmless; it still pins the default options.
- LA-01 `Pinned.alternatives` keeps alternative hashes only for sources at the same path as a primary source. An alternative start that reads a path no primary source reads is dropped from `sources` (the patchset spec does not forbid it; no shipped patch does it today). LA-10's "copy every source path the patch reads" would miss it. Owner or LA-10 decision: accept and document, or add alternative-only source paths as their own `Pinned` entries in LA-01. Not a defect against the LA-01 text.
- LA-01 `summarize` prefixes `sgw_game` patches with `Working/SGWGame`, but `patch_dest` also supports a flattened client (`<install>/SGWGame`). The packet takes no install dir, so it cannot tell; LA-09/LA-10 callers work on stock-layout folders. Accept; revisit only if a flattened client is ever adopted.
- Private `hex()` helpers: LA-01 `patch_summary.rs:193` and LA-06 `state_hex` repeat helpers that already exist privately in launcher-core (`overlay_pack.rs:253`, `file_index/mod.rs:292`). The repo already has many such copies; not worth a shared module in this wave.
- LA-05 and LA-07 worknotes (`docs/analysis/launcher-telemetry/worknotes/LA-05.md`, `LA-07.md`) are untracked in their worktrees, and LA-08 has none, unlike LA-01/02/06 which commit theirs. For LA-05 leave it uncommitted (PR I2 is based on main). For LA-07 `ops` or the coordinator should commit it into the integration branch. Not a code defect.
- `.claude/agent-memory/main-session/reference_adoption_contract_audit_2026_10_04.md` and `docs/analysis/playtests/2026-10-03-macos-wine/worknotes/adoption-contract-audit.md` still name the removed `adopt_existing_install` as dated findings; leave them.
- Revert behaviour: LA-01/06/08 tests fail by compile error when the module is reverted, as the spec accepts. LA-05's `an_https_index_blob_on_another_host_passes` and `a_relative_index_blob_under_an_https_seed_passes` fail against the old `blob_forms_match`; `an_http_index_blob_fails_index_blob_form` pins the failure code. LA-08 `write_set_files_are_independent_copies` fails if the write-set check is removed (a hard link would let the write change `src`). `linked_files_share_content` asserts counts only (as the spec says), so it would not catch a link that duplicated content; accept.