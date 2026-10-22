# Launcher telemetry: work packets, wave 8 (light adoption, LA-01..LA-20)

Design and decisions: [light-adoption-proposal.md](light-adoption-proposal.md) (v2, owner answers 2026-10-21). Ledger: [README.md](README.md). Anchors are at the launcher integration tip `e6f16395a`. Before editing, re-grep every anchor at your base tip.

Paths: **core** = `crates/launcher-core/src/`, **patchset** = `crates/patchset/src/`, **engine** = `crates/launcher/desktop/engine/src/storage/`, **shell** = `crates/launcher/desktop/shell/src/`, **frontend** = `crates/launcher/desktop/frontend/`.

## Rules R-LA (every packet; read these before your packet)

1. **Host: windows.** PowerShell only, never Bash. Start every command with `Set-Location <your worktree>;`.
2. **Lane, foreground, in this order, for each crate you touched.** Never call cargo directly.
   - `pwsh tools/build-lane/lane.ps1 cargo fmt -p <crate> -- --check`
   - `pwsh tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings`
   - `pwsh tools/build-lane/lane.ps1 cargo nextest run -p <crate>`

   The crates are `cimmeria-patchset`, `cimmeria-launcher-core`, `cimmeria-launcher-engine` and `cimmeria-launcher-desktop` (the shell). Frontend: `Set-Location <wt>\crates\launcher\desktop\frontend; npm run check; npm test`. At most 5 fix rounds per check. Check that the lane log sits under `logs\<your worktree>\`.
3. **Git.** No `git stash`, no `git worktree prune`, no push, no rebase. Make one local commit with the subject given, ending with the attribution lines from your brief. Then stop.
4. **File size.** Soft cap 500 lines, hard cap 700. `engine/adoption/publication.rs` (697) and `in_place.rs` (688) take at most 3 new lines each. `verification.rs` (514), `adoption/mod.rs` (460) and shell `main.rs` (635) take call lines only. New logic goes in the new files named here.
5. **Tests** follow TESTING.md: in-crate unit or integration tests, with tempdirs and tiny hand-built zips. Each new test must fail when your change is reverted, and your report says why. No network, lab, live server, SigNoz or release asset. Reuse the fixture builders the packet names (grep for them; they may have moved).
6. **Telemetry is R5-clean.** No absolute path, URL, host, IP or user name in any field. Only folder-relative paths (target `adoption` only), `patch:<id>` labels, counts, closed codes and `digest()` forms. Tests pin `target`, `event` and field values, never message text.
7. Rust serde types and their TypeScript schema land in the same commit (`onExcessProperty: 'error'`).
8. Files under `docs/` are CRLF. Write no IPs, credentials or account names anywhere.
9. **Report** to `<worktree>\.claude\reports\launcher-telemetry\<packet>-coder.md`. Return exactly one line: `<packet> <done|blocked|failed> <sha|-> <report-path>`. When the packet's facts don't match the code, return `blocked` and give the reason; don't improvise.
10. **No behavior change for downloaded installs (D-LA14, owner 2026-10-21, hard rule).** A player with no install who downloads the game (the seed comes from the Internet Archive) must get exactly today's behavior: fresh install, Update and Repair. On those paths the requests, the staged `game/` tree and the `launcher-installed.json` bytes stay identical to today. The seedless (incremental) Update runs **only** for an install whose installed-content record carries adoption provenance with `check = light` (see C-LA § discriminator). The strict entry points (`apply`, `unpack`, `install_all`, `install_all_with_seed_extractor`, `update/preparation` full path, everything under `repair/`) keep their behavior. Never edit an existing test of these paths to make it pass; if one fails, return `blocked`. The only permitted difference on a downloaded install is that `update_staged` gains `mode = "full"`, `ops_skipped = 0`, `stage_linked = 0` and `stage_copied = 0`.

## Branches, waves, merge order

- **PR LA:** integration worktree `lt-light-integration`, branch `launcher-telemetry/light-adoption`. `ops` creates it from the `launcher-telemetry/launcher` tip (PR L #1981, open). Each packet worktree is `la-<nn>` on branch `launcher-telemetry/la-<nn>-<slug>`, from the integration tip at dispatch. `ops` merges each reviewed packet into the integration branch with `merge-noff.ps1`, then retires its worktree the same day. Once PR L merges, `ship.ps1` restacks PR LA onto `origin/main`. The PR opens after LA-19.
- **PR I2:** LA-05 alone, worktree `la-05`, branch `launcher-telemetry/la-05-index-blob-form`, from `origin/main`. It is a single packet, so it has a packet review loop but no wave loop.
- **Wave 8a** (parallel): LA-01, LA-02, LA-06, LA-07, LA-08 (and LA-05 for PR I2). **Merge order** into the integration branch: LA-07, LA-02, LA-01, LA-06, LA-08. The wave review covers the five.

### Starting wave 8a while PR L #1981 is open

Every step is an `ops` request; record each SHA in the orchestrator's ledger.

1. `git fetch origin`. Record `L_TIP` = the `origin/launcher-telemetry/launcher` SHA (it was `e6f16395a` when these packets were written; if it moved, use the new tip, because every packet re-greps its anchors) and `MAIN_TIP` = `origin/main`.
2. Check for overlap between PR I2 and PR L: `git diff --stat <MAIN_TIP> <L_TIP> -- crates/launcher-core/src/file_index/check.rs docs/client/launcher-distribution-setup.md`. If it is empty, the two PRs cannot conflict. If it is not empty, LA-05 still starts from `MAIN_TIP`, and whichever of PR L and PR I2 merges second resolves that file in its own restack.
3. Create the worktree `lt-light-integration`, branch `launcher-telemetry/light-adoption`, at `L_TIP`. Push it with upstream so it is backed up. No PR yet.
4. Create five packet worktrees **at `L_TIP`** (the integration tip, which has nothing merged yet): `la-01` `launcher-telemetry/la-01-patch-summary`, `la-02` `launcher-telemetry/la-02-tolerant-apply`, `la-06` `launcher-telemetry/la-06-index-compare`, `la-07` `launcher-telemetry/la-07-remove-adopt`, `la-08` `launcher-telemetry/la-08-stage-seed`.
5. Create `la-05` at **`MAIN_TIP`**, branch `launcher-telemetry/la-05-index-blob-form`. It ships as PR I2 through `ship.ps1 pr` after its packet review. It never merges into the integration branch: PR LA gets it from `main` when it restacks.
6. **If PR L gains commits before it merges** (review fixes), `ops` merges the new `L_TIP` into `launcher-telemetry/light-adoption` with `merge-noff.ps1`, between packet merges, and records the new `L_TIP`. Packet worktrees already in flight are left alone; they merge into the integration branch as they are.
7. **When PR L merges**, `ops` restacks the integration branch onto `origin/main` with `ship.ps1`, as above, giving the last recorded `L_TIP` as the old base (the "rebase onto a recorded SHA" op). Packet worktrees created after that start from the restacked tip. Packets still in flight then merge into the restacked branch; a conflict follows step 8.
8. **A merge conflict** in `merge-noff.ps1` (expected only in `core/lib.rs` module lines): `ops` stops. A packet-coder in that packet's worktree merges the integration tip into the packet branch (a merge, not a rebase), resolves only the conflicting lines, re-runs the packet's lane checks and commits. Then `ops` retries the merge.

**Parallel lanes for 8a: all six run at once.** No two of them edit the same file except `core/lib.rs`, where LA-01 (`pub mod patch_summary;` after `patch_dest`) and LA-08 (`pub mod stage_seed;` after `seed_source`) each add one line, with an unchanged line between them.

| Lane | Packet | Worktree, base | Crates it builds | Shares a file with |
|---|---|---|---|---|
| 1 | LA-02 | `la-02`, `L_TIP` | patchset, then core nextest | none |
| 2 | LA-07 | `la-07`, `L_TIP` | core, then engine clippy (the longest) | none |
| 3 | LA-01 | `la-01`, `L_TIP` | core | LA-08 (`lib.rs`, one line each) |
| 4 | LA-06 | `la-06`, `L_TIP` | core | none |
| 5 | LA-08 | `la-08`, `L_TIP` | core | LA-01 (`lib.rs`) |
| 6 | LA-05 | `la-05`, `MAIN_TIP` | core | none (different base) |

The build lane's heavy slots set the real concurrency: if there are fewer than six, the extra jobs queue in the lane on their own, and nobody needs to stagger dispatch. Dispatch all six in the background; each packet's review loop starts when its coder returns. The wave review covers the five PR LA packets after they merge in the order above. LA-05 has only its packet review.
- **Wave 8b:** LA-03 → LA-04; LA-09 → LA-10; LA-12 alongside; LA-11 after LA-10 and LA-12; LA-11b after LA-11 (added 2026-10-21 from the wave 8b3 review; it has its own packet review loop). **Merge order:** LA-03, LA-09, LA-12, LA-04, LA-10, LA-11, LA-11b.
- **Wave 8c:** LA-13 (after LA-04, LA-08, LA-11, which adds `Provenance.check`, and LA-11b, which places the cooked tier), LA-14 → LA-15 → LA-16 → LA-18, and LA-17 (after LA-11). **Merge order:** LA-13, LA-17, LA-14, LA-15, LA-16, LA-18.
- **Close:** LA-19 (docs, `documentation-writer`, in `la-19`) goes into the integration branch; then PR LA ships. LA-20 runs after PR I2 merges with green CI.
- **Sonnet (`rust-gameserver-dev`):** LA-10, LA-11, LA-11b, LA-13. **Haiku (`packet-coder`):** all the others except LA-19 and LA-20.

## Contract C-LA (types every packet builds against)

```rust
// patchset/src/apply.rs (LA-02)
#[derive(Debug, Clone, Default)]
pub struct ApplyOptions {
    pub tolerate_ops: bool,                          // skip-and-record instead of returning an error
    pub only_targets: Option<BTreeSet<String>>,      // lower-case '/' paths, relative to the apply dir; Some => ops on these targets only, NO overlays
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason { SourceMissing, SourceMismatch, DeltaFailed, ResultMismatch }
impl SkipReason { pub fn code(self) -> &'static str } // "source_missing" | "source_mismatch" | "delta_failed" | "result_mismatch"
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedOp { pub target: String /* as written in the recipe */, pub reason: SkipReason }
// ApplyReport gains: pub skipped: Vec<SkippedOp>   (always empty when tolerate_ops == false)
pub fn apply_with(zip_path: &Path, install_dir: &Path, opts: &ApplyOptions, progress: &mut dyn FnMut(&str)) -> Result<ApplyReport>;
// apply(..) == apply_with(.., &ApplyOptions::default(), ..)  (behaviour unchanged)

// core/patch_summary.rs (LA-01)
pub struct Pinned { pub path: String /* client-root relative, '/', with patch_dest prefix */, pub sha256: String /* lower hex */, pub alternatives: Vec<String> }
pub struct PatchSummary { pub state_key: String, pub sources: Vec<Pinned>, pub outputs: Vec<Pinned> }
pub fn summarize(zip: &Path, entry: &PatchEntry) -> Result<PatchSummary, PatchSummaryError>;
pub fn pending(summaries: &[PatchSummary], have: impl Fn(&str) -> Option<String>) -> Vec<usize>; // signed-order indices

// core/state.rs (LA-03)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedTarget { pub path: String /* client-root relative '/' */, pub reason: String /* SkipReason::code() */ }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompletePatch { pub state_key: String, pub skipped: Vec<SkippedTarget> }
// InstalledState gains:
//   #[serde(default, skip_serializing_if = "Vec::is_empty")] pub incomplete_patches: Vec<IncompletePatch>
// A patch in incomplete_patches is ALSO in applied_patches (D-LA18); "fully applied" = in applied and not incomplete.
impl InstalledState {
    pub fn is_fully_applied(&self, state_key: &str) -> bool;
    pub fn skipped_op_count(&self) -> usize;
    pub fn record_incomplete(&mut self, state_key: &str, skipped: Vec<SkippedTarget>); // empty => removes the entry
}
// core/unpack (LA-03)
pub fn unpack_with(archive: &Path, dest: &Path, sink: <same as unpack>, opts: &cimmeria_patchset::ApplyOptions)
    -> Result<Vec<cimmeria_patchset::SkippedOp>, UnpackError>;   // unpack(..) == unpack_with(.., &Default) mapped to ()

// core/install.rs (LA-04)
pub async fn install_all_tolerant(ctx: InstallContext<'_>) -> (Result<(), InstallError>, InstallReport);
pub async fn install_all_tolerant_with_seed_extractor(ctx: InstallContext<'_>, backend: SeedBackend<'_>) -> (Result<(), InstallError>, InstallReport);

// core/stage_seed.rs (LA-08)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SeedStats { pub linked: u64, pub copied: u64, pub bytes_copied: u64 }
pub fn seed_stage(src: &Path, dst: &Path, write_set: &BTreeSet<String> /* lower-case '/' rel */, link: bool, cancel: &dyn Fn() -> bool) -> std::io::Result<SeedStats>;

// discriminator (LA-13, D-LA14): which installs may take the seedless Update
// engine/installed_content/mod.rs
pub(super) fn adoption_check(&self) -> Result<Option<FileCheck>, StorageError>; // on DesktopState: reads the installed-content Record; None = no `adoption` provenance (a launcher-downloaded install)
// engine/update/incremental.rs
pub(super) fn seedless_allowed(check: Option<FileCheck>) -> bool { matches!(check, Some(FileCheck::Light)) }
// Not the discriminator: `InstalledState.seed_adopted` (legacy, refused by Play, imported from old ledgers by migration),
// nor anything else in game/launcher-installed.json (a downloaded install writes the same shape).

// core/file_index/compare.rs (LA-06)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)] #[serde(rename_all = "snake_case")]
pub enum FileClass { Stock, Patched, Prepared, Changed, Missing, Extra }
pub struct IndexComparison { pub counts: BTreeMap<FileClass, u64>, pub files: Vec<(String, FileClass)> /* non-Stock only, sorted by path */ }
pub fn compare(index: &FileIndex, folder: &BTreeMap<String /* '/' rel, any case */, String /* lower hex */>) -> IndexComparison;
// core/install_layout.rs (LA-06)
pub fn bundled_cooked_alias(rel: &str) -> Option<String>; // "Working/SGWGame/Cache.en-US/x" (case-blind) -> "Working/SGWGame/SourceCache.en-us/x"

// engine/adoption (LA-09..LA-12)
pub(super) struct LightPlan { pub summaries: Vec<PatchSummary>, pub zips: Vec<PathBuf>, pub pending: Vec<usize>, pub download_bytes: u64 }
pub(super) struct LightReplay { pub replacements: Vec<Replacement>, pub incomplete: Vec<IncompletePatch> }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum IndexState { Compared, IndexNotPublished, IndexUnreachable, IndexInvalid }
pub(super) struct IndexReport { pub state: IndexState, pub comparison: Option<IndexComparison> }
// model.rs (LA-11): AdoptionMode gains Light (serde "light"); Provenance gains
//   #[serde(default, skip_serializing_if = "FileCheck::is_full")] pub check: FileCheck   // enum FileCheck { Full, Light }, snake_case
// AdoptionPreview gains  #[serde(default, skip_serializing_if = "Option::is_none")] pub light: Option<LightSummary>
pub struct LightSummary {
    pub patches_total: u32, pub patches_current: u32, pub patches_pending: u32, pub download_bytes: u64,
    pub skipped: Vec<LightSkip>,          // capped at 500; LightSkip { path: String, patch: String /* state_key */, reason: String }
    pub skipped_total: u32,
    pub index_state: IndexState, pub index_counts: BTreeMap<FileClass, u64>,
    pub index_files: Vec<(String, FileClass)>, // capped at 500
    pub index_files_total: u32,
}
```

**New events** (info, `tracing`, R5):

| Event | Target | Fields | Packet |
|---|---|---|---|
| `adoption_light_planned` | `adoption` | `patches_total`, `patches_current`, `patches_pending`, `download_bytes` | LA-09 |
| `adoption_light_replayed` | `adoption` | `patches_replayed`, `replacements`, `ops_skipped`, `patches_incomplete` | LA-10 |
| `adoption_index_compared` | `adoption` | `state`, `stock`, `patched`, `prepared`, `changed`, `missing`, `extra`, `changed_sample`, `missing_sample` | LA-12 |
| `patch_incomplete` | `launcher.install` | `patch` (`patch:<id>`), `ops_skipped` | LA-04 |
| `update_staged` gains fields | `launcher.game_update` | `mode` (`incremental`/`full`), `ops_skipped`, `stage_linked`, `stage_copied` (the full path always sends `full`, 0, 0, 0) | LA-13 |
| `launch_started` gains fields | (unchanged) | `adoption_check` (`light`/`full`/`none`), `patches_incomplete` | LA-17 |

Samples (D-LA17): each holds at most 10 paths, comma-joined, at most 600 characters, taken **only** from `IndexComparison.files` entries whose path the index names (`Changed`, `Missing`). Never `Extra`.

---

## Wave 8a

### LA-01: patch summaries (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-01`, branch `launcher-telemetry/la-01-patch-summary`. **Depends:** none.
- **Files:** new `core/patch_summary.rs` (≤400 lines including tests), registered in `core/lib.rs`.
- **Contract:** C-LA § `patch_summary`.
- **Shape:**
  - Read a recipe the way `core/file_index/build.rs` reads recipe sources (grep `RECIPE_NAME`). Use `cimmeria_patchset::{RECIPE_NAME, Recipe}` (`patchset/recipe.rs:19-69`).
  - The *sources* are each op's `sources[]` `(path, sha256)`, with the op's `alternatives` flattened into `Pinned.alternatives`.
  - The *outputs* are each op's `(target, result_sha256)`, plus every other zip entry except the recipe, `deltas/` entries and directories, hashed with sha256. A zip with no recipe contributes every entry as an output.
  - Prefix every path with the patch's destination: grep `patch_dest` / `state_key` in `core/install.rs` and `manifest.rs`. Use `/` separators.
  - `pending`: a patch is *current* when, for every output path, `have(path)` (case-blind) equals that output's sha **or** the sha of the same path in the output of any later patch. It returns the indices of the patches that are not current, in order.
- **Tests (unit, tiny zips built in the test):**
  - `outputs_of_a_plain_zip_are_hashed_entries`
  - `recipe_ops_give_sources_and_results` (alternatives kept)
  - `a_stock_folder_has_every_patch_pending`
  - `a_folder_at_patch_2_of_3_has_only_the_last_pending`
  - `a_later_output_counts_as_current`

  Reverted, the module does not exist.
- **Lane:** `cimmeria-launcher-core` (fmt, clippy, nextest).
- **Docs:** none. **Commit:** `LA-01: summarize patch sources and outputs for light adoption`.

### LA-02: tolerant apply (patchset)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-02`, branch `launcher-telemetry/la-02-tolerant-apply`. **Depends:** none.
- **Files:**
  - `patchset/apply.rs`: `ApplyReport` at `:11-19`, `apply` at `:43-47`, the compute loop at `:58-106`, the overlays at `:126-141`.
  - `patchset/lib.rs:43`: the re-export.
  - If `apply.rs` passes 500 lines, put the new types in a new `patchset/apply_options.rs`.
- **Contract:** C-LA § `patchset`.
- **Shape:**
  - Rename the body of `apply` to `apply_with(.., opts, ..)`. `apply` calls it with `&ApplyOptions::default()`.
  - When `opts.only_targets` is `Some(set)`, skip every op whose lower-cased target is not in `set` (it is not counted anywhere), and **do not write overlays**.
  - When `opts.tolerate_ops` is set, these early returns become `report.skipped.push(SkippedOp{target, reason}); continue;`:
    - `:86-88`: `SourceMissing` → `SourceMissing`; `SourceMismatch` or `PatchOutputMismatch` → `SourceMismatch`.
    - `:93-96`: → `DeltaFailed`.
    - `:98-104`: → `ResultMismatch`.

    A delta entry missing from the zip (`:89-91`) stays fatal. With the flag off, nothing changes.
  - The write order (`:112-119`) is unchanged; skipped ops never enter `pending`.
  - **R-LA 10:** `apply` is what every fresh install, full Update and Repair uses. With default options its result, its errors and the bytes it writes are unchanged, and no existing apply test is edited.
- **Tests:** add them beside the existing apply tests (grep `fn apply` in `patchset/src/**/tests*` or the `tests/` dir for the fixture builder).
  - Tolerant, with one op on a mismatched source: the overlay is written, the matching op is rebuilt, `skipped == [SourceMismatch]`, and the mismatched file is unchanged on disk.
  - Strict, on the same input: still `Err(SourceMismatch)` and nothing written.
  - Tolerant, with a missing source: `SourceMissing`.
  - `only_targets = {a}`: only op `a` runs, and no overlay is written.

  Reverted, the tolerant tests fail on `Err`.
- **Lane:** `cimmeria-patchset`, then `cimmeria-launcher-core` nextest (its only caller).
- **Docs:** none. **Commit:** `LA-02: tolerant per-op apply with a skipped list`.

### LA-05: accept any https index blob, document the publish (PR I2)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-05` from **`origin/main`**, branch `launcher-telemetry/la-05-index-blob-form`. **Depends:** none.
- **Files:**
  - `core/file_index/check.rs`: `blob_forms_match` `:117-126`, `blob_form` `:135-146`, the check-order doc `:22-28`, tests `:271` and `:342`.
  - `core/bin/verify-file-index.rs`, only if it names the check.
  - `docs/client/launcher-distribution-setup.md` § File index, `:136-151` (CRLF).
- **Shape:**
  - Rename the check `blob_form_matches_seed` to `index_blob_form`.
  - It passes when the index blob is relative (no `://`), or is `https://<non-empty host>/…` on **any** host. It fails for any other scheme, with the detail "the file index blob must be an https URL or a relative name". The seed's blob form no longer matters.
  - Keep the check order and the `blob_form` helper.
- **Tests:**
  - Update `a_consistent_candidate_passes_every_check` for the new name.
  - Replace `a_relative_index_blob_under_an_https_seed_fails_the_blob_form` with three tests:
    - `an_https_index_blob_on_another_host_passes` (seed on `https://archive.example/…`, index on `https://github.com/…`)
    - `a_relative_index_blob_under_an_https_seed_passes`
    - `an_http_index_blob_fails_index_blob_form`

  Reverted, the cross-host test fails.
- **Docs (runbook):** state the tracked script `tools/content-release/prepare-file-index.ps1`. State that each publication uploads `file-index.json` to a **new** GitHub release `content-file-index-<yyyymmdd>`, and that `-BlobPath` is that asset's full `https://github.com/<owner>/<repo>/releases/download/content-file-index-<yyyymmdd>/file-index.json` URL. Add these steps:
  1. Back up the live `manifest.json` and `.sig`.
  2. Sign.
  3. Run `cimmeria-patchset verify`.
  4. Upload.
  5. Swap with `gh release upload content-current --clobber`.
  6. Download again and run `verify-file-index --previous <backup>`.
  7. Rollback: `gh release upload content-current --clobber <backup>\manifest.json <backup>\manifest.json.sig`.

  Include no key material or addresses.
- **Lane:** `cimmeria-launcher-core`.
- **Commit:** `LA-05: accept any https file-index blob; document the publish and rollback`.

### LA-06: index comparison and the cooked-data alias (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-06`, branch `launcher-telemetry/la-06-index-compare`. **Depends:** none.
- **Files:**
  - New `core/file_index/compare.rs` (≤350 lines including tests), declared in `core/file_index/mod.rs`.
  - `core/install_layout.rs`: a new `bundled_cooked_alias`, next to the rename at `:56-81`.
- **Contract:** C-LA § `compare`, `bundled_cooked_alias`.
- **Rules for `compare`.** Lower-case the keys and use `index.histories()` (`file_index/mod.rs:223-234`). For each folder file `p` with sha `h`:
  1. The index has states for `p`:
     - the seed state's sha equals `h` → `Stock`;
     - otherwise any patch state's sha equals `h` → `Patched`;
     - otherwise `p` is an `AslrOff` rule path and `h` equals the rule's sha, or `p` is a `LoginServers` rule path → `Prepared` (`SetupRule`, `mod.rs:86-96`);
     - otherwise → `Changed`.
  2. The index has no state for `p`: a setup-rule path as above → `Prepared`; otherwise `Extra`.
  3. Every seed-layer path absent from the folder → `Missing`. A path that only patches introduce and that is absent is not counted.
  4. `files` holds every non-`Stock` entry; use the index's spelling for `Missing`.
- **Why a new function and not `verification::classify`:** that one answers "is this folder the release?" (`NeedsPatching` and the rest, `pub(super)`, needs login servers). This one answers "how does this folder differ from the seed?" The wave reviewer should not flag the overlap.
- **Tests:**
  - One fixture `FileIndex` built in the test (copy the builder pattern from `file_index` tests; grep `fn index_fixture` or `SeedLayer {`) that yields every class once.
  - A case-blind match.
  - `bundled_cooked_alias("working/sgwgame/cache.en-us/a.pak") == Some("Working/SGWGame/SourceCache.en-us/a.pak")` and `None` for other paths.

  Reverted, nothing compiles.
- **Lane:** `cimmeria-launcher-core`.
- **Docs:** none. **Commit:** `LA-06: compare a folder against the file index by class`.

### LA-07: remove the dead adopt (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-07`, branch `launcher-telemetry/la-07-remove-adopt`. **Depends:** none.
- **Files:** `core/install.rs` (`AdoptError` `:508`, `adopt_existing_install` `:546`, through the end of that fn), `core/install_tests.rs:273-328`, and the `lib.rs` re-exports if any.
- **Shape:** first grep `adopt_existing_install` and `AdoptError` across `crates/`. If any non-test caller exists, return `blocked`. Then delete.
- **Test:** none new (deletion); the core suite stays green.
- **Lane:** `cimmeria-launcher-core`, then `cimmeria-launcher-engine` clippy.
- **Docs:** the `crates/launcher-core` README line, if it names the function. **Commit:** `LA-07: remove the unused adopt_existing_install`.

### LA-08: stage seeding with hard links and a write set (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-08`, branch `launcher-telemetry/la-08-stage-seed`. **Depends:** none.
- **Files:** new `core/stage_seed.rs` (≤350 lines including tests), registered in `lib.rs`.
- **Contract:** C-LA § `stage_seed`.
- **Shape:**
  - Walk `src` with `std::fs::read_dir`, recursively, without following links. A symlink or reparse point anywhere returns `io::ErrorKind::InvalidInput` (on Windows, check `FILE_ATTRIBUTE_REPARSE_POINT` through `std::os::windows::fs::MetadataExt::file_attributes() & 0x400`).
  - Create every directory under `dst`.
  - For each file, take the lower-cased `/` relative path `r`:
    - `link && !write_set.contains(r)`: `std::fs::hard_link`; on `Err`, `std::fs::copy` and count it as copied.
    - Otherwise: `std::fs::copy`.
  - Check `cancel()` every 64 files and return `io::ErrorKind::Interrupted`.
  - `dst` must not exist on entry (`AlreadyExists`).
  - **R-LA 10:** this is a library function only. Wire no caller in this packet: no install, update or repair code calls it until LA-13, and LA-13 calls it only on the light-adopted branch.
- **Tests:**
  - `write_set_files_are_independent_copies`: write to the `dst` copy, and `src` is unchanged.
  - `linked_files_share_content` (Windows-only `#[cfg(windows)]`): the counts say linked.
  - `link_false_copies_everything`.
  - `a_symlink_in_src_is_refused` (`#[cfg(unix)]` symlink, or a Windows junction via `std::os::windows::fs::symlink_dir` only if it succeeds without elevation; otherwise skip with a comment).
  - `cancel_interrupts`.

  Reverted, the module is gone.
- **Lane:** `cimmeria-launcher-core`.
- **Docs:** none. **Commit:** `LA-08: seed a stage from a tree with hard links outside a write set`.

---

## Wave 8b

### LA-03: incomplete-patch ledger and tolerant unpack (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-03`, branch `launcher-telemetry/la-03-incomplete-ledger`. **Depends:** LA-02 merged.
- **Files:**
  - `core/state.rs`: `InstalledState` `:45-62`, helpers near `:89-99`.
  - `core/unpack/mod.rs`: `unpack` `:127`.
  - `core/unpack/zip.rs`: `extract` `:45-61`, which today drops the `ApplyReport` at `:54-59`.
- **Contract:** C-LA § `state.rs`, `unpack`.
- **Shape:**
  - `unpack_with` threads `opts` to `zip::extract`, which calls `cimmeria_patchset::apply_with` and returns `report.skipped`. A RAR or a plain zip returns an empty vec. `unpack` calls `unpack_with` with the default options and discards the vec.
  - The `tracing::info!` at `:54-59` gains `skipped = report.skipped.len()`.
  - `record_incomplete` replaces the entry for `state_key`, or removes it when `skipped` is empty.
  - **R-LA 10:** `unpack` (strict) is unchanged for every caller; a downloaded install's ledger never gains `incomplete_patches`, so its `launcher-installed.json` bytes stay as today.
- **Tests:**
  - A ledger JSON without `incomplete_patches` parses, and re-serializes byte-identically (existing pattern near `state.rs:313`). This is the downloaded-install guard for the ledger.
  - A round trip with one incomplete patch.
  - `is_fully_applied` is false for an incomplete key and true for a clean one.
  - `unpack_with` in tolerant mode on a recipe zip with a mismatched source returns one `SkippedOp`; in strict mode it returns `Err`.

  Reverted, they fail to compile or assert.
- **Lane:** `cimmeria-launcher-core`.
- **Docs:** none. **Commit:** `LA-03: record incomplete patches and return skipped ops from unpack`.

### LA-04: tolerant install_all with incomplete retry (core)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-04`, branch `launcher-telemetry/la-04-tolerant-install`. **Depends:** LA-03 and LA-07 merged.
- **Files:** `core/install.rs`: `install_all` `:178`, `_with_seed_extractor` `:187`, `install_all_into` `:197-278`, `apply_patch` → `verify_and_unpack` (`unpack` at `:353`). If `install.rs` passes 560 lines, move the new retry helper to a new `core/install_tolerant.rs`.
- **Contract:** C-LA § `install.rs`.
- **Shape:**
  - Add a private `struct InstallOptions { tolerate_ops: bool }`, passed to `install_all_into`, `apply_patch` and `verify_and_unpack`, which calls `unpack_with` with `ApplyOptions{ tolerate_ops, only_targets: None }`. The two new public fns pass `tolerate_ops: true`; the existing ones pass `false`, so their behaviour is unchanged.
  - **Recording.** On `Ok(skipped)`, keep the existing `applied_patches.push`. Map `skipped` to `SkippedTarget{ path: <patch dest prefix>/<target>, reason: code() }`, call `state.record_incomplete(key, list)`, then `save`. If `skipped` is not empty, emit `tracing::info!(target: "launcher.install", event = "patch_incomplete", patch = %format!("patch:{id}"), ops_skipped)`.
  - **Retry**, tolerant only, before the patch loop:
    - For each `IncompletePatch` in `state.incomplete_patches`, in manifest order, find its `PatchEntry`. If it is missing, keep the entry and continue.
    - Download it as `apply_patch` does, then unpack with `only_targets = Some(<its skipped paths, prefix stripped, lower-case>)`.
    - Record the new skipped list; an empty list clears the entry.
  - The `after`-dependency logic is unchanged.
  - **R-LA 10:** `install_all` and `install_all_with_seed_extractor` serve the fresh install, the full Update and Repair of every downloaded install. They pass `tolerate_ops: false`, never run the retry, never call `record_incomplete`, and write the same files and ledger bytes as today. Do not edit any existing `install_tests.rs` test.
- **Tests:** add to `install_tests.rs`, or a new `install_tolerant_tests.rs` if that file is over 500 lines. Reuse its stub transport and its zip helpers.
  1. Tolerant install over a pre-seeded dir with one changed source: the result is `Ok`, the patch is in `applied_patches` and in `incomplete_patches`, and the overlay landed.
  2. The same input with strict `install_all`: `PatchesFailed`, as today.
  3. Retry: restore the source in the dir, run tolerant again, and the entry clears.
  4. Strict ignores the incomplete list: strict `install_all` over a dir whose ledger already lists an incomplete patch requests no blob for that patch's retry, and leaves `incomplete_patches` as it found it.

  Reverted, (1) and (3) fail. (4) is the R-LA 10 guard: it fails when the retry runs in strict mode. Prove it by removing the `tolerate_ops` check locally, and say so in the report.
- **Lane:** `cimmeria-launcher-core`, then `cimmeria-launcher-engine` clippy.
- **Docs:** none. **Commit:** `LA-04: tolerant install_all that records and retries incomplete patches`.

### LA-09: light plan (engine)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-09`, branch `launcher-telemetry/la-09-light-plan`. **Depends:** LA-01 merged.
- **Files:** new `engine/adoption/light_plan.rs` (≤300 lines) and `light_plan_tests.rs`, declared in `engine/adoption/mod.rs` (one line).
- **Contract:** C-LA § `LightPlan`.
- **Shape:**
  - `pub(super) fn plan(transport, release, cache, scan: &inventory::Index, cancel) -> Result<LightPlan, Error>`.
  - Fetch every patch with `artifacts::fetch_some` (`artifacts.rs:131`), seed `false`.
  - `summarize` each one.
  - Run `pending` with `have(p)` = the scan's sha, as lower hex, for `p`. If `p` is absent, try `install_layout::bundled_cooked_alias` in reverse: look the stock `Cache.en-US` spelling up when `p` is under `SourceCache.en-us`. Grep `inventory::Index` (`inventory.rs:23`) for the entry type (`[u8;32]`).
  - `download_bytes` = the sum of the manifest sizes of the blobs `fetch_some` had to download, not cached ones; if `fetch_some` does not say, use every patch.
  - Emit `adoption_light_planned`.
  - Map refusals through the existing C14 `rejection::reject` (`rejection.rs:176`) with the existing artifact checks.
- **Tests (engine stub transport, as `adoption/replay_tests.rs` uses it):**
  - A stock fixture plans every patch as pending.
  - A fixture already at the last patch plans none.
  - The event carries the four counts.
  - No request asks for the seed (assert on the stub's requested URLs).

  Reverted, the module is gone.
- **Lane:** `cimmeria-launcher-engine`.
- **Docs:** none. **Commit:** `LA-09: plan a light adoption from patch summaries`.

### LA-10: tolerant light replay into replacements (engine, Sonnet)

- **Agent:** rust-gameserver-dev. **Host:** windows. **Worktree:** `la-10`, branch `launcher-telemetry/la-10-light-replay`. **Depends:** LA-03 and LA-09 merged.
- **Files:** new `engine/adoption/light_replay.rs` (≤450 lines) and `light_replay_tests.rs`. Read `replay.rs` (`fn replay` `:190`, scratch `:214`, patch loop `:230-259`, `copy_local` `:306`, replacements `:261-280`) and **mirror it; do not edit it**.
- **Contract:** C-LA § `LightReplay`.
- **Shape:**
  - `pub(super) fn replay(plan: &LightPlan, folder: &Path, scan, prep: &Path, cancel) -> Result<LightReplay, Error>`. It creates `<prep>/scratch` and then, for each pending index in order:
    - Copy every source path the patch reads and every output path it writes. Copy from scratch when an earlier patch wrote the file, otherwise from the folder, using `bundled_cooked_alias` for the stock spelling. Missing files are not copied.
    - Run `unpack::unpack_with(zip, scratch.join(dest), sink, &ApplyOptions{ tolerate_ops: true, only_targets: None })`.
    - Turn the skipped ops into `IncompletePatch` (prefix the dest).
  - After the loop, every scratch file whose sha differs from the folder's (or that is absent from the folder) becomes `Replacement{ reason: NeedsPatching }` under `<prep>/replacements`, as at `replay.rs:261-280`. Record the cooked-data rename the way LA-11 needs it: a list of `(from, to)` moves in `LightReplay`, if the folder used the stock spelling.
  - Emit `adoption_light_replayed`.
  - `patch_input_unexpected` and `patch_replay_mismatch` are **not** raised in light mode.
- **Tests:**
  1. A stock fixture gets every patch-touched file as a replacement, and `incomplete` is empty.
  2. A fixture with one changed source: adoption still succeeds, the overlay replacement exists, `incomplete` holds that path with `source_mismatch`, and no replacement exists for it.
  3. A chained case: patch 2's op reads patch 1's skipped target and is also recorded.
  4. An already-patched fixture yields 0 replacements.
  5. The event fields.

  Reverted, (2) fails on a refusal.
- **Lane:** `cimmeria-launcher-engine`.
- **Docs:** none. **Commit:** `LA-10: tolerant light replay with incomplete patches`.

### LA-11: Light mode, digest, provenance, ledger (engine, Sonnet)

- **Agent:** rust-gameserver-dev. **Host:** windows. **Worktree:** `la-11`, branch `launcher-telemetry/la-11-light-mode`. **Depends:** LA-10 and LA-12 merged.
- **Files:**
  - `engine/adoption/model.rs`: `AdoptionMode` `:39`, `Provenance` `:279` (`deny_unknown_fields` `:278`), and the preview struct.
  - `engine/adoption/mod.rs`: the dispatch at `:224` (call lines only).
  - `engine/adoption/in_place_preview.rs`: the call `:129`.
  - New `engine/adoption/light.rs` (≤450 lines): `light_comparison`, the light digest, `LightSummary` assembly.
  - `engine/adoption/reference.rs`: `ledger()` `:108`.
  - Provenance construction for in place: `in_place.rs:184` (≤3 lines).
  - `publication.rs`: ≤3 lines, only if `check` must be threaded there.
- **Contract:** C-LA § `model.rs`.
- **Shape:**
  - `Light` dispatches to `in_place_preview::preview` with a `light: bool`. At `:129` it calls `light::light_comparison`, which runs, in order:
    1. the scan, through `inventory::scan_with_progress` (LA-12), reporting hashed bytes of the total to the preview's `ProgressSink` (D-LA13);
    2. `light_plan::plan`;
    3. `index_compare::compare_to_published(state_root, release, transport, &scanned, &cancel, &progress)` (LA-12); propagate only `Error::Cancelled`. Remove LA-12's two `allow(dead_code)` attributes.
    4. `light_replay::replay`.

    It returns the same `Compared` shape (`verification.rs:348`) plus `LightSummary`.
  - Compute `expected_digest`/`expected_paths` from the light tree: the folder scan, overlaid with the replacements and the cooked-data moves, then the setup bytes, then the synthesized ledger. Mirror `expected_tree` (`verification.rs:295`), `expected_prepared` (`:464`) and `published_digest` (`:329`), without the index.
  - `reference::ledger` gains an `incomplete: &[IncompletePatch]` parameter. Existing callers pass `&[]`.
  - Set `Provenance.check = Light`. Confirm that `verify_provenance` (`publication.rs:625`) and `published_record` (`:654`) accept it unchanged.
  - `content_valid` is unchanged.
- **Tests:**
  1. End to end: preview, then confirm, on a stock fixture. It publishes, `content_valid` holds, and the provenance says `light`.
  2. A fixture with a changed source publishes too; its `game/launcher-installed.json` lists the patch in both `applied_patches` and `incomplete_patches`, and `content_valid` holds.
  3. A pre-LA provenance JSON without `check` verifies, and its plan digest is byte-identical.
  4. Recovery after `Staged` for a light publication.

  Reverted, (1) and (2) fail.
- **Lane:** `cimmeria-launcher-engine`.
- **Docs:** `crates/launcher/desktop/docs/migration.md` § in place: one paragraph on Light, the tolerant replay and `incomplete_patches`.
- **Commit:** `LA-11: Light adoption mode with a light digest and incomplete-patch ledger`.
- **Review amendment (architect, 2026-10-21, wave 8b3 review "Not fixed" 1).** The cooked-data split tree is fixed in **LA-11b**, not in LA-11's fix pass. The la-11 fix coder applies N1, N2, N3(b), N3(c) and N4 and **skips N3(a)**: N3(a) pins the stored-copy `moves` behaviour that LA-11b replaces, and LA-11b's tests cover that path. The empty-scan refusal (`light.rs:87-89`) stays. An empty folder has nothing to adopt, which is not a changed input, so D-LA9 does not apply and no owner call is needed.

### LA-11b: Light places the bundled cooked-data tier (engine, Sonnet)

- **Agent:** rust-gameserver-dev. **Host:** windows. **Worktree:** `la-11b`, branch `launcher-telemetry/la-11b-cooked-tier`, from the integration tip **after LA-11 (with its fix pass) has merged**. **Depends:** LA-11 merged. Anchors are at la-11 `dd8e72e65`; re-grep them at your base.
- **Problem (the bug this fixes).** The 2009 cabinets put the bundled PAKs in `Working/SGWGame/Cache.en-US`. The client reads the bundled tier only from `Working/SGWGame/SourceCache.en-us`, and every patch reads and writes it there. `install_layout::place_bundled_cooked_data` (core `install_layout.rs:72-81`) renames the directory, but only when `SourceCache.en-us` does not exist yet. Light today moves the untouched stock-spelled PAKs into `game/` under `Cache.en-US` (as `Matched` rows), while a file a patch touched lands under `SourceCache.en-us` (as a replacement, or as a stored copy through `LightReplay.moves`). The client then sees a split tier. The old copy of a patched PAK also stays under `Cache.en-US`, because the skip at `light.rs:128` compares folder names with placed names. Later, `install_all` (core `install.rs:267`) no longer renames the directory, because `SourceCache.en-us` now exists, so LA-13's patches lose their cooked sources.
- **Design (decided, do not change).** Light applies the same rule as `place_bundled_cooked_data` in its review, and the moves carry it out with **one directory rename**: no per-file `source_path` support in `move_files`, no copy of any PAK, no change to `LightReplay` or LA-10, and no plan schema change.
  - **The rule ("place"):** the scan holds at least one file `f` with `bundled_cooked_alias(f).is_some()`, and no file whose lower-cased name starts with `working/sgwgame/sourcecache.en-us/`. When the rule does not hold (the folder has no stock tier, or already has both tiers), Light behaves exactly as today.
  - **The placed name** of a stock-tier file `f` = `format!("{}SourceCache.en-us/{}", &f[..16], &f[28..])`. `&f[..16]` keeps the folder's own spelling of `Working/SGWGame/`, and `&f[28..]` is the part after `Cache.en-US/`. Both slices are safe: `bundled_cooked_alias` matched those bytes as ASCII.
  - **At publish:** a light plan whose rows rename the stock tier gets that directory renamed under the picked folder (`Cache.en-US` to `SourceCache.en-us`, in the folder's spelling of the parent) at the start of the moves. Abandon renames it back after the files return.
- **Files (only these):**
  - `engine/adoption/light.rs` (≤450 lines): the rows and the tree (Shape 1).
  - `engine/adoption/in_place_replace.rs` (285 lines, stays ≤400): the new `cooked_tier`, `place_cooked_tier`, `restore_cooked_tier` and `plain_dir` (Shape 2).
  - `engine/adoption/in_place.rs` (690 lines): **at most +2 net lines**, exactly the three edits in Shape 3. If it would pass 692 lines, return `blocked`.
  - New `engine/adoption/light_cooked_tests.rs` (≤350 lines), declared next to `light_tests` in `engine/adoption/mod.rs` (one `#[cfg(test)] mod` line).
  - `crates/launcher/desktop/docs/migration.md`: one sentence in the Light paragraph (CRLF).
- **Shape 1: `light::light_comparison` (after `light_replay::replay`, the block at `:113-168`).**
  1. `let place = scanned.keys().any(|k| bundled_cooked_alias(k).is_some()) && !scanned.keys().any(|k| k.to_ascii_lowercase().starts_with("working/sgwgame/sourcecache.en-us/"));`
  2. A local `placed_name(name: &str) -> String` returns the placed name (above) when `place` holds and `bundled_cooked_alias(name).is_some()`, and `name.to_string()` otherwise.
  3. `replaced` becomes a set of `r.path.to_ascii_lowercase()`. The scan loop computes `let path = placed_name(name);` and skips the file when `moved.contains(name)` or `replaced.contains(&path.to_ascii_lowercase())`. This skip removes the duplicate old copy of a patched PAK. Otherwise it inserts `path` into `tree` and pushes `Difference { path, source_path: Some(name.clone()), classification }`, where `classification` is `Matched` when `path == *name` and `KnownTransform` when the file was renamed (the full path uses the same class for a spelling change, `verification.rs:405-409`).
  4. The replacement rows' `source_path` is the folder file the replacement stands for: `name` when the scan holds `r.path` exactly; otherwise, when `place` holds, the scanned `name` whose `placed_name(name)` equals `r.path` case-blind (build a `BTreeMap<String /*lower placed*/, &String /*folder name*/>` once); otherwise `None`. With this, `replace_file` sets the stock original aside into the work folder like any other patched file, instead of moving it into `game/`.
  5. When `place` holds, `moves` is empty: skip the `replayed.moves` filter entirely. Every move is already a renamed row, so no PAK is copied into `<prep>/replacements`. When `place` does not hold, keep the `moves` code exactly as it is (it is reachable only for a folder that already has both tiers).
  6. Update the module doc (`:1-4`) with one sentence: "A folder with only the stock cooked-data tier is reviewed as placed (`SourceCache.en-us`), as a fresh install places it; the moves rename the directory."
  7. Leave `LightReplay`, `light_replay.rs` and `light_plan.rs` untouched.
- **Shape 2: `in_place_replace.rs`.** Import `super::in_place::Way`, `cimmeria_launcher_core::install_layout::bundled_cooked_alias` (or the path `light_plan.rs`/`index_compare.rs` use) and `FileCheck`.
  ```rust
  /// The stock cooked-data tier a light adoption places (LA-11b), as folder-relative
  /// `(stock, placed)` directories in the folder's spelling. `None` unless the plan is
  /// light and one of its rows renames a stock-spelled file to its placed path.
  fn cooked_tier(plan: &Plan) -> Option<(String, String)> {
      if !matches!(plan.provenance.check, FileCheck::Light) { return None; }
      plan.report.files.iter().find_map(|d| {
          let source = d.source_path.as_deref()?;
          (bundled_cooked_alias(source).is_some() && !d.path.eq_ignore_ascii_case(source))
              .then(|| (source[..27].to_string(), format!("{}SourceCache.en-us", &source[..16])))
      })
  }
  /// Whether `path` is a plain directory. Missing is `false`; a link, junction or file is refused.
  fn plain_dir(path: &Path) -> Result<bool, Error>   // symlink_metadata: NotFound => Ok(false);
      // Ok(m) if failed_cleanup::plain(&m) && m.is_dir() => Ok(true); Ok(_) => Err(StorageError::UnsafeFile.into());
      // other error => Err(StorageError::Io.into())   (mirror `present` in in_place.rs)
  /// Before any file moves (adopt, and abandon too, so that a run that stopped before
  /// the rename can move files back): rename the stock tier to the placed one. Idempotent.
  pub(super) fn place_cooked_tier(way: Way<'_>, from: &Path, to: &Path) -> Result<(), Error>
  /// After abandon moved the files back: rename the placed tier back to the stock spelling.
  pub(super) fn restore_cooked_tier(way: Way<'_>, to: &Path) -> Result<(), Error>
  ```
  - `place_cooked_tier`: `(plan, root)` = `(plan, from)` for `Way::Adopt`, `(plan, to)` for `Way::Abandon`. `None` from `cooked_tier` returns `Ok(())`. With `stock`/`placed` = `inventory::native(root, ..)`:
    - `!plain_dir(&stock)?`: return `Ok(())`. The tier is already renamed: a re-run, recovery, or the second call.
    - `plain_dir(&placed)?`: try `std::fs::remove_dir(&placed)`. An empty leftover directory goes, matching the scan-based rule. If it is not empty, log `tracing::debug!(target: "adoption", "light adoption: both cooked-data tiers hold files; the directory stays as it is")` and return `Ok(())` (a stored-move row looks like a rename row, so this case publishes as today; `move_files` still returns `Corrupt` if a placed file is in neither place).
    - `std::fs::rename(&stock, &placed)?;` then `tracing::debug!(target: "adoption", "light adoption: placed the bundled cooked-data tier")`. No path field (R-LA 6).
  - `restore_cooked_tier`: `Way::Adopt` returns `Ok(())`. For `Way::Abandon { plan }` with `Some((stock, placed))`, under `root = to`: `(plain_dir(placed)?, plain_dir(stock)?)` = `(true, false)` renames `placed` to `stock`; `(true, true)` logs `tracing::warn!(target: "adoption", "abandoning light adoption: both cooked-data tiers are in the folder; the placed one stays")`; any other case does nothing. Then `Ok(())`.
- **Shape 3: `in_place.rs` (+2 net lines, nothing else).**
  1. `enum Way<'a>` (`:326`) becomes `pub(super) enum Way<'a>` (same line).
  2. In `move_files` (`:382`), first body line, before `let mut reviewed = None;`: `super::in_place_replace::place_cooked_tier(way, from, to)?;`
  3. The tail `remove_emptied(from, files)` (`:457`) becomes `remove_emptied(from, files)?;` followed by `super::in_place_replace::restore_cooked_tier(way, to)`.

  `move_files` already runs for publish (`move_release_files`), for recovery (`finish`, phase `Staged`) and for abandon, so these two calls cover all three. The emptied renamed tier under the folder goes through the existing `remove_emptied(root, ..)`. A full-check plan has `check = Full`, so `cooked_tier` is `None` and both calls return at once (R-LA 10: full in-place and copy adoption are unchanged).
- **Tests (`light_cooked_tests.rs`).** Build fixtures as `light_tests.rs` does (the stock fixture, preview, confirm and recovery helpers), with patches from `light_replay_tests::{patch_zip, OpFixture, release_with_roots, OVERLAY}` (already `pub(in crate::storage::adoption)`; grep them). The folder holds `Working/SGWGame/Cache.en-US/A.pak` and `.../B.pak` (distinct small bytes). The release's one pending patch edits `Working/SGWGame/SourceCache.en-us/A.pak` (an overlay with new bytes is enough).
  1. **Split-tree regression.** Preview and confirm. After publish: `game/Working/SGWGame/SourceCache.en-us/A.pak` holds the patched bytes; `game/.../SourceCache.en-us/B.pak` holds B's stock bytes; neither `game/Working/SGWGame/Cache.en-US` nor `<folder>/Working/SGWGame/Cache.en-US` exists; `content_valid` holds. Reverted (Shape 1 and 3 removed), B is under `game/.../Cache.en-US` and the test fails.
  2. **Review rows.** In the preview: B's row has path `.../SourceCache.en-us/B.pak`, `source_path = Some(".../Cache.en-US/B.pak")` and `KnownTransform`. A's row is `NeedsPatching` with `source_path = Some(".../Cache.en-US/A.pak")`. No row's path is under `Cache.en-US`. `<prep>/replacements` holds A.pak only, not B.pak (B is renamed, never copied).
  3. **Both tiers: no rename.** The folder also holds `Working/SGWGame/SourceCache.en-us/C.pak`. After publish, `game/` holds both `Cache.en-US/B.pak` and `SourceCache.en-us/C.pak`, as `place_bundled_cooked_data` would leave them.
  4. **Abandon puts the stock tier back.** Confirm with a hook that fails at the first `Point::Moved` (after the directory rename), then abandon (as the in-place abandon tests do). `<folder>/Working/SGWGame/Cache.en-US/{A,B}.pak` hold their stock bytes, `<folder>/Working/SGWGame/SourceCache.en-us` does not exist, and `game/` is gone.
  5. **Recovery.** Fail at `Point::BeforePromotion` (before the rename), then at `Point::Moved` in a second fixture, and recover through the existing recovery entry (grep `a_light_publication_recovers_after_staged`). Both publish the tree of test 1.
  6. **Empty leftover.** The folder also holds an empty `Working/SGWGame/SourceCache.en-us` directory with no files. It publishes as in test 1.

  Every existing `in_place_tests`, `in_place_recovery_tests`, `in_place_replace_tests`, `golden_tests`, `light_tests` and `light_replay_tests` test passes **unedited**; if one fails, return `blocked` (R-LA 10). Say in the report which tests fail when Shape 3's two calls are removed (expected: 1, 4 and 5).
- **Lane:** `cimmeria-launcher-engine`. Per the LA-11 review, the engine crate's tests run with `pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-launcher-engine` (not nextest); fmt and clippy as R-LA 2.
- **Docs:** `migration.md`, Light paragraph, one sentence: "A folder that has only the 2009 `Working\SGWGame\Cache.en-US` tier is adopted with it renamed to `SourceCache.en-us`, the place a fresh install uses; abandon renames it back." CRLF.
- **Commit:** `LA-11b: light adoption places the bundled cooked-data tier by one directory rename`.

### LA-12: published-index comparison and scan progress (engine)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-12`, branch `launcher-telemetry/la-12-index-compare`. **Depends:** LA-06 merged.
- **Amended 2026-10-21** after the first run returned `blocked`: `scan` hashes during the walk (`inventory.rs:262-321`, `hash` at `:317`), and `published` needs a cancel token and a progress sink. The design below replaces the old precondition; do not re-check it. Anchors are at `c4126f495`; re-grep them.
- **Files** (only these):
  - New `engine/adoption/index_compare.rs` (≤350 lines including tests).
  - `engine/adoption/mod.rs`: one line, `mod index_compare;`, directly after `mod index_source;` (`:18`).
  - `engine/adoption/index_source.rs`: one word. `fn published(` (`:99`) becomes `pub(super) fn published(`. Nothing else.
  - `engine/adoption/inventory.rs` (430 lines): `scan` (`:261-333`) and two new functions, at most 45 net new lines; the file stays under 500.
  - `engine/adoption/verification_tests.rs`: visibility only. `fn seed_bytes` (`:48`) and `async fn origin` (`:102`) become `pub(super)`. Change no test body.
- **Contract:** C-LA § `IndexState`, `IndexReport`, and the `adoption_index_compared` event.
- **Shape, part 1: scan progress (D-LA13).** The full-check path keeps `scan` exactly as it is today (R-LA 10): no pre-pass, the same walk order, the same errors and the same `Index`. Progress is a separate entry point that adds a metadata-only sizing pass in front of the same walk.
  1. Move the body of `scan` (`:262-332`, the nested `walk` and everything after it) unchanged into a new private `fn scan_inner(root: &Path, cancel: &CancellationToken, on_file: &mut dyn FnMut(u64)) -> Result<Index, Error>`. Give the nested `walk` one more parameter, `on_file: &mut dyn FnMut(u64)`, and pass it through the recursive call at `:315`. Replace `out.insert(name, hash(&path, cancel)?);` (`:317`) with:
     ```rust
     let entry = hash(&path, cancel)?;
     let size = entry.size;
     out.insert(name, entry);
     on_file(size);
     ```
     Change nothing else in the body: no reordering, no new checks, no new error.
  2. `pub(super) fn scan(root: &Path, cancel: &CancellationToken) -> Result<Index, Error> { scan_inner(root, cancel, &mut |_| {}) }`. Same signature as today; no existing caller changes.
  3. New private `fn sized(root: &Path, cancel: &CancellationToken) -> Result<u64, Error>`, the metadata-only pre-pass. It never opens or hashes a file and never rejects; the real walk does the safety checks. A nested `fn add(path, depth, files: &mut usize, total: &mut u64, cancel)`:
     - `check_cancel(cancel)?` first (the only `Err` it returns: `Error::Cancelled`).
     - `depth > MAX_DEPTH` → return `Ok(())`. `let Ok(children) = std::fs::read_dir(path) else { return Ok(()) };`
     - For each `child` in `children.flatten()`: stop (`return Ok(())`) once `*files >= MAX_FILES`. `let Ok(kind) = child.file_type() else { continue };`. `kind.is_dir()` → recurse with `depth + 1`. `kind.is_file()` → `*files += 1; *total = total.saturating_add(child.metadata().map(|m| m.len()).unwrap_or(0));`. Anything else (a symlink or junction) is skipped. `DirEntry::file_type` never follows links.
  4. New `pub(super) fn scan_with_progress(root: &Path, cancel: &CancellationToken, on_bytes: &dyn Fn(u64 /*hashed*/, u64 /*total*/)) -> Result<Index, Error>`:
     ```rust
     let total = sized(root, cancel)?;
     on_bytes(0, total);
     let mut hashed = 0u64;
     scan_inner(root, cancel, &mut |size| {
         hashed = hashed.saturating_add(size);
         on_bytes(hashed, total.max(hashed)); // a file that grew after sizing never shows hashed > total
     })
     ```
  5. Put `#[cfg_attr(not(test), allow(dead_code))]` on `scan_with_progress` and `sized`, with the comment `// LA-11 wires this into the light preview.` Without it, clippy `-D warnings` fails, because nothing outside tests calls them until LA-11.
- **Shape, part 2: `compare_to_published`** in `index_compare.rs`. Put `#![cfg_attr(not(test), allow(dead_code))] // LA-11 calls this.` as the file's first line.
  - `IndexState` and `IndexReport` exactly as in C-LA. Add `impl IndexState { pub fn code(self) -> &'static str }` returning `"compared"`, `"index_not_published"`, `"index_unreachable"` or `"index_invalid"` (the serde names).
  - Signature: the caller owns cancellation and progress, as with `acquire_index` (`index_source.rs:47`). A cancel aborts the preview, the same as everywhere else; every other failure becomes a state.
    ```rust
    pub(super) fn compare_to_published(
        state_root: &Path,              // the same launcher state root acquire_index gets: the index cache lives there
        release: &VerifiedRelease,
        transport: &artifacts::Transport,
        scan: &inventory::Index,
        cancel: &CancellationToken,
        progress: &ProgressSink,        // crate::install_progress::ProgressSink; the index download reports to it
    ) -> Result<IndexReport, Error>     // Err is only ever Error::Cancelled
    ```
    It never calls `built()`, never fetches the seed, and never calls `published_event` or `fallback_event`.
  - The body, in order:
    1. `let Some(reference) = &release.manifest().file_index else { → IndexNotPublished };`
    2. `let (outcome, _recorded) = rejection::scope(|| index_source::published(state_root, release, transport, reference, cancel, progress));`. The scope keeps a rejection recorded inside `published` from becoming the preview's cause. Drop `_recorded`.
    3. Map `outcome`:

       | `outcome` | Result |
       |---|---|
       | `Ok(Ok((index, _cached)))` | compare (step 4) → `Compared` |
       | `Ok(Err(f))` with `f.reason == "index_download_failed"` | `IndexUnreachable` |
       | `Ok(Err(f))`, any other reason (`index_reference_malformed`, `index_size_mismatch`, `index_digest_mismatch`, `index_parse_failed`, `index_bind_failed`, `index_read_failed`) | `IndexInvalid` |
       | `Err(Error::Cancelled)` | `return Err(Error::Cancelled)`; no event |
       | `Err(_)`, anything else (the verified index could not be cached) | `IndexUnreachable` |

    4. Compare: build `BTreeMap<String, String>` from `scan`. The key is `bundled_cooked_alias(name).unwrap_or_else(|| name.clone())` (`cimmeria_launcher_core::install_layout`), the value is the lower-hex `entry.sha256`. Then `file_index::compare(&index, &folder)`, and `comparison: Some(..)`. Every non-`Compared` state has `comparison: None`.
  - Emit `adoption_index_compared` once on every return except a cancel. Mirror `published_event` (`index_source.rs:294`): `tracing::info!(target: "adoption", event = "adoption_index_compared", state = report.state.code(), stock, patched, prepared, changed, missing, extra, changed_sample, missing_sample, "<message>")`. The counts come from `comparison.counts` (0 when absent or `None`). Build each sample with one helper, `sample(files, class) -> String`: walk `comparison.files` in order, keep only `class` (`FileClass::Changed` or `FileClass::Missing`, never `Extra`), take at most 10 paths, comma-join them, and stop before the string would pass 600 characters. With no comparison, both samples are `""`.
- **Tests** in `index_compare.rs` (`#[cfg(test)] mod tests`). The transport tests use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` and call `compare_to_published` inside `tokio::task::block_in_place`, as `verification_tests.rs:115-133` (`acquire`) does. Reuse `super::super::verification_tests::{release, published_index, reference, seed_bytes, origin}`, `artifacts::Transport::loopback(format!("{}/manifest.json", server.uri()))`, `ProgressSink::latest().0` and `crate::test_capture::capture` (see `verification_tests.rs:158-175`). The folder is a `tempfile::tempdir()` scanned with `inventory::scan`.
  1. Absent ref: `release(&seed, None)` → `IndexNotPublished`, `comparison` is `None`.
  2. 404: a `MockServer` with a 404 mounted on `/index/file-index.json` (as in `verification_tests.rs:202-222`) → `IndexUnreachable`.
  3. Flipped digest: `reference(&index, Some(hex of Sha256::digest(b"another index")))` → `IndexInvalid`.
  4. Compared: `origin(&seed, &index)`. The folder holds `later.txt` = `b"changed entry"` and `my_screenshot_Steve.png` = any bytes, and no `Working/Binaries/SGW.exe`. Expect `Compared`, `counts[Changed] == 1`, `counts[Extra] == 1` and `counts[Missing] >= 1`. If `later.txt` is not an index path in this fixture, return `blocked` with what the index holds; don't change the fixture builders.
  5. Event (capture test 4): exactly one `adoption_index_compared` row, `state == "compared"`, `changed == "1"`, `extra == "1"`, `changed_sample` contains `later.txt`, and **no captured field of any row** contains `my_screenshot_Steve`.
  6. Progress: a tempdir with `a.bin` (3 bytes), `d/b.bin` (5 bytes) and `d/e/c.bin` (7 bytes). Record every `on_bytes` call in a `RefCell<Vec<(u64, u64)>>`. Assert the first call is `(0, 15)`, the last is `(15, 15)`, `hashed` never decreases, and the `Index` it returns equals `inventory::scan` of the same folder.
  7. Cancel: a cancelled token makes `scan_with_progress` return `Err(Error::Cancelled)`, and `on_bytes` is never called.

  Reverted, the module is gone, so 1-7 no longer exist. Every existing `inventory::scan` test and every `verification_tests` test still passes unedited; if one fails, return `blocked` (R-LA 10).
- **Lane:** `cimmeria-launcher-engine` (fmt, clippy, nextest, per R-LA 2).
- **Docs:** none. **Commit:** `LA-12: compare an adopted folder against the published index, never blocking`.
- **Hand-off to LA-11:** LA-11 calls `compare_to_published` with its `cancel` and `&progress` and propagates only `Error::Cancelled`. LA-11 also does its light-path scan through `scan_with_progress`, reporting to the preview's `ProgressSink`, and removes the two `allow(dead_code)` attributes from this packet.

---

## Wave 8c

### LA-13: incremental update for light-adopted installs (engine, Sonnet)

- **Agent:** rust-gameserver-dev. **Host:** windows. **Worktree:** `la-13`, branch `launcher-telemetry/la-13-incremental-update`. **Depends:** LA-01, LA-04, LA-08, LA-11 and LA-11b merged.
- **Cooked tier (amended 2026-10-21, LA-11b).** A light-adopted `game/` has its bundled cooked tier placed: every PAK is under `Working/SGWGame/SourceCache.en-us`, unless the picked folder already had both tiers. Build every light fixture in that layout, with no `Working/SGWGame/Cache.en-US`. Do not add any cooked-tier handling to the incremental path. `install_all_tolerant` already calls `place_bundled_cooked_data` on the stage (core `install.rs:267`), and that call is a no-op here.
- **Hard rule (R-LA 10, D-LA14, owner 2026-10-21):** the seedless path is **only** for installs adopted through light adoption. A downloaded install's Update is today's seed rebuild, byte for byte: same requests (seed included), same `seed_source_for` call, same strict `install_all` into an empty stage, same staged tree and ledger. Extending the seedless path to every install is a later option (D-LA14b); this packet must not do it.
- **Files:**
  - `engine/update/preparation/mod.rs` (389 lines): `start_with_backend` `:106-147` (the branch, under the lock, after `installed_content()` `:135`), `reconstruct` `:187-311`, stage `:205-215`, `install_all` `:217` and Wine `:236`. If the file would pass 450 lines, move the incremental half of `reconstruct` into `incremental.rs`.
  - `engine/installed_content/mod.rs` (452 lines): the new `adoption_check` (≤15 lines), beside `adopted_in_place` `:139`. It reads the `Record` and returns `adoption_shape(&record)?.map(|p| p.check)`.
  - New `engine/update/incremental.rs` (≤350 lines) and its tests.
  - `engine/update/events.rs`: `update_staged` `:68-78`.
  - Do not touch `repair/`.
- **Shape:**
  - **The branch (discriminator first).** In `start_with_backend`, under the lock, right after `installed_content()` has verified the record:
    - `let light = incremental::seedless_allowed(owner.adoption_check()?)`;
    - only when `light` is true, call `incremental::plan(..)`;
    - when `light` is false, **or** `plan` returns `None`, run today's code unchanged: `seed_source_for` and the full `reconstruct`.

    Model it as `enum UpdatePath { Full(SeedSourceHandle), Incremental(IncrementalPlan) }`, carried into `reconstruct`. The `Full` arm is today's lines `:205-247` moved, not edited. Both arms rejoin at `stash_install_report` `:248`.
  - A downloaded install never calls `incremental::plan`, `seed_stage` or `install_all_tolerant`, and never reads its `game/launcher-installed.json` for this purpose.
  - The light provenance survives the Update: `publish_update_index` (`installed_content/mod.rs:212`) keeps `adoption`, so the next Update of a light-adopted install takes the seedless path again. A copy adoption (`check = full`), a legacy import and a launcher-downloaded install always take `Full`.
  - `incremental::plan(game: &Path, target: &Release) -> Option<IncrementalPlan>` returns `Some` when all of these hold:
    - `game/launcher-installed.json` loads;
    - its `seed_sha256` equals the target manifest's seed sha;
    - its `applied_patches` are, in order, a prefix of the target's patch `state_key`s.

    Otherwise it returns `None`, and the full path runs as today, including `seed_source_for`. Skip `seed_source_for` on the incremental path.
  - **Write set:**
    - the outputs (LA-01 `summarize`) of every patch not in `applied_patches`, which needs those blobs: fetch them through the same transport `install_all` uses, or download them into the cache first (grep how `install_all` downloads a patch);
    - every `incomplete_patches[].skipped[].path`;
    - `launcher-installed.json`;
    - the setup-rule paths (grep the client setup for the `SGW.exe` and login-file constants).
  - `stage_seed::seed_stage(game, stage, &write_set, cfg!(windows), cancel)`.
  - Then `install_all_tolerant` (or the `_with_seed_extractor` variant on Wine) **with the stage as `install_dir`**. The seed is skipped because the ledger matches (F7).
  - Everything after `:249` is unchanged: `content_valid`, `staged_content::digest` (on unix it refuses `nlink != 1`, which is why `link` is false there), the prepared record, and commit with swap, backup and recovery.
  - `update_staged` gains `mode`, `ops_skipped` (from the stage's ledger), `stage_linked` and `stage_copied`. The `Full` arm always sends `full`, 0, 0, 0.
  - Progress: on the `Incremental` arm only, report `Progress::Extracting` with label `"Preparing update"` while seeding the stage. The `Full` arm's progress is unchanged.
- **Tests (engine update tests; reuse their stub transport and fixtures, grep `prepare_native` tests).** "Light fixture" = an installed-content record whose `adoption` provenance has `check = light` (build it the way the LA-11 tests do). "Downloaded fixture" = the same game folder and ledger with **no** `adoption` in the record, as a launcher install writes it.
  1. A light fixture at patch 2 of 3, with the seed blob **absent** from the stub: Update stages and commits; only patch 3 was requested; `mode = incremental`; after commit the record still carries `check = light`.
  2. A light fixture whose target has a different seed sha: `mode = full`, and the seed is requested.
  3. A light fixture with a changed source for patch 3: the update still commits, and the new ledger lists patch 3 in `incomplete_patches`.
  4. A light fixture: the live `game/` file in the write set is unchanged until commit (it is modified in the stage only).
  5. Repair still requests the seed (an existing test is enough; name it in the report).
  6. **Downloaded install never takes the new path (R-LA 10 guard).** The downloaded fixture at patch 2 of 3, where seed sha and ledger prefix **do** qualify, and the stub serves the seed. Assert all of these:
     - the seed blob was requested;
     - `mode = full`, `stage_linked = 0` and `stage_copied = 0`;
     - the committed `game/` tree and `launcher-installed.json` bytes equal those of a strict `install_all` run into an empty directory with the same stub;
     - `incomplete_patches` is absent from the ledger.

     With the stub **not** serving the seed, the Update fails exactly as today.
  7. A copy-adopted fixture (`check = full`) that qualifies the same way: `mode = full`, and the seed is requested.
  8. Unit test: `seedless_allowed(None)`, `seedless_allowed(Some(Full))` are false; `seedless_allowed(Some(Light))` is true.
  9. **Cooked patch (LA-11b layout).** A light fixture at patch 2 of 3, with `Working/SGWGame/SourceCache.en-us/A.pak` and `B.pak` and no `Cache.en-US`; patch 3 edits `A.pak`. The Update commits with `mode = incremental`, `A.pak` holds patch 3's output, `B.pak` is unchanged, and `incomplete_patches` is absent.

  Reverted, (1) fails because the seed is requested. Remove the discriminator locally (let `plan` run for every install): (6) and (7) must fail. Say so in the report.
- **Lane:** `cimmeria-launcher-engine`.
- **Docs:** none (LA-19). **Commit:** `LA-13: seedless incremental update for light-adopted installs only`.

### LA-14: shell modes and the one-level-down search

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-14`, branch `launcher-telemetry/la-14-shell-modes`. **Depends:** LA-11 merged.
- **Files:**
  - `shell/host/adoption/status.rs:80-84`: `Backend::InPlace` → `[Light, Copy]`.
  - `shell/host/adoption/events.rs` (`mode_chosen` `:40`, `folder_chosen` `:50`): `Light` counts as in place.
  - `shell/main.rs` `:372-405`: call lines only.
  - `engine/adoption/source.rs`: `resolve_folder` `:93`, the one-level-down search.
  - `engine/adoption/rejection.rs`: add `several_game_folders` to `REJECTION_CHECKS` `:7-22` and map it in `code` `:111`.
  - `frontend/src/adoption-workflow.ts:19-21`: the literal `'light'`.
- **Shape:** in `resolve_folder`, when none of the pick, its parent or its grandparent resolves, list the pick's direct child directories, skipping links, and run the existing root test on each:
  - exactly one → use it;
  - two or more → `several_game_folders` (found = the count);
  - none → `not_a_game_folder` as today.
- **Tests:**
  - `status.rs`: `light` is listed first.
  - Source tests (grep `resolve_folder` tests): one child is picked; two children are refused with check `several_game_folders`.
  - A frontend schema test accepts `'light'`.

  Reverted, they fail.
- **Lane:** `cimmeria-launcher-engine`, `cimmeria-launcher-desktop`, then the frontend.
- **Docs:** none. **Commit:** `LA-14: Light mode in the shell and a one-level-down folder search`.

### LA-15: shell status and schema for the light review and incomplete patches

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-15`, branch `launcher-telemetry/la-15-light-status`. **Depends:** LA-14 merged.
- **Files:**
  - The shell adoption review contract: grep `index_fallback` under `shell/host/adoption/`, where C16 added it, and add `light: Option<LightSummary>` beside it.
  - The installed-content status the main view reads: grep `installed_content` in `shell/host/` and add `patches_incomplete: u32` and `ops_skipped: u32`, read from `game/launcher-installed.json` through `InstalledState::load`, with `skipped_op_count()`.
  - The progress mapping for the scan bytes from LA-12 (`ProgressItem` C16: label "Scanning your game folder", `done`/`total` in bytes).
  - The TS schemas in `frontend/src/adoption-workflow.ts` and the matching status schema file, in the same commit.
- **Tests:**
  - A shell serialization test for a preview with `light` set.
  - A TS schema test that decodes it, and one where an unknown key fails.
  - The status reports `patches_incomplete` for a fixture ledger.

  Reverted, they fail.
- **Lane:** `cimmeria-launcher-desktop`, then the frontend.
- **Docs:** none. **Commit:** `LA-15: light review, index comparison and incomplete patches in the shell status`.

### LA-16: adoption view copy and first press

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-16`, branch `launcher-telemetry/la-16-adoption-view`. **Depends:** LA-15 merged.
- **Files:** `frontend/ui/index.html:37-39` (mode rows; rebuild `dist/index.html` as the frontend's build does, if it is committed); `frontend/src/adoption-view.ts` (`:123-124`, `:154-158`, `:230`, `:242`). If `adoption-view.ts` passes 300 lines, put the new rendering in a new `frontend/src/adoption-light-view.ts`. Add the C17 reason texts (grep `several_game_folders`' neighbours, e.g. `not_a_game_folder`, in the C17 table file).
- **Copy:**
  - **Mode rows.**
    - **Use it where it is (recommended):** "The launcher takes over this folder and brings it up to date with the current patches. Its game files move into a “game” folder inside it."
    - **Make a checked copy:** the existing text, plus "Checks every file; downloads the full game archive (4.1 GB) unless a file list is published."
  - **First press.** On picker return, set "Checking your game folder…" in the same frame, before the preview resolves. A cancelled picker shows "No folder chosen."
  - **Review.**
    - "Stargate Worlds found in <folder name only>."
    - "Patches already applied: <k> of <n>. To apply: <m> (<size> to download)."
    - When `skipped_total > 0`: "Some patch changes can't be applied: <N> files differ from the originals the patches expect. They stay as they are. You can still use this folder." Below it, a `<details>` list of `path — patch`.
    - The confirm button **Use this folder** is never disabled by skipped ops.
  - **Done.** "This folder is now your Cimmeria install. If the game will not start, use Repair (it downloads the full game, 4.1 GB)."
  - **Refusals.** `several_game_folders`: "This folder holds more than one Stargate Worlds install. Pick the one you want."
- **Tests (`adoption-view.test.ts`, or a new `adoption-light-view.test.ts`):**
  - The first-press status is set before the preview promise resolves.
  - The skipped list renders and the confirm button stays enabled.
  - `several_game_folders` has its text.

  Reverted, they fail.
- **Lane:** frontend (`npm run check; npm test`).
- **Docs:** none. **Commit:** `LA-16: light adoption review, skipped-change list and first-press status`.

### LA-17: launch tag (engine)

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-17`, branch `launcher-telemetry/la-17-launch-tag`. **Depends:** LA-11 merged.
- **Files:** `engine/launch/events.rs:24-28` (`launch_started`); `engine/launch/worker.rs:64` (the call); the source of the provenance, `installed_for_launch` (`engine/installed_content/mod.rs:157-208`).
- **Shape:**
  - `adoption_check` = `light`, `full` or `none`, from the provenance's `check`; `none` when no provenance exists.
  - `patches_incomplete` = `InstalledState::load(game).incomplete_patches.len()`, or 0 on any load error.
  - Stay within 12 fields. If `launch_started` would pass 12, return `blocked` with the current field list.
- **Test:** the capture test pins `adoption_check = light` and `patches_incomplete = 1` for a light fixture, and `none`/`0` for a plain install. Reverted, the fields are absent.
- **Lane:** `cimmeria-launcher-engine`.
- **Docs:** none (LA-19). **Commit:** `LA-17: tag launches with the adoption check and incomplete patches`.

### LA-18: index comparison panel, main-view hint, REPL UAT

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `la-18`, branch `launcher-telemetry/la-18-compare-view`. **Depends:** LA-16 merged.
- **Files:** new `frontend/src/adoption-index-view.ts` (≤250 lines) and its test; one call line in `adoption-view.ts`; the main-view status renderer (grep where the installed status line renders, e.g. `content_ready` or `installed` in `frontend/src/`); new `frontend/adoption-light-uat.mjs`, modelled on `frontend/adoption-in-place-uat.mjs` (110 lines), with an npm script `uat:adoption-light`.
- **Copy:**
  - **Comparison.**
    - `Compared`: "Compared with the published file list: <stock> original, <patched> already patched, <prepared> prepared by the launcher, <changed> changed, <missing> missing, <extra> extra." Then a `<details>` list "Show the files" (path and class; "and <n> more" when `index_files_total` > the list).
    - `IndexNotPublished`: "No published file list to compare against yet."
    - `IndexUnreachable`: "The file list could not be downloaded; the folder was not compared."
    - `IndexInvalid`: "The published file list failed its check; the folder was not compared."
    - None of them blocks.
  - **Main view,** when `patches_incomplete > 0`: "<ops_skipped> patch changes could not be applied. If the game crashes or misbehaves, use Repair (downloads the full game, 4.1 GB)." Play stays enabled.
- **UAT script.** It drives the view logic REPL-style, as the in-place UAT does, through these scenarios, and asserts the texts and that the confirm button and Play are enabled:
  - a clean stock folder;
  - skipped changes;
  - each index state;
  - an install with incomplete patches.
- **Tests:** each index state's text, the list cap line, and the main-view hint shown only when the count is above 0. Reverted, they fail.
- **Lane:** frontend, then `npm run uat:adoption-light`.
- **Docs:** none. **Commit:** `LA-18: index comparison panel, incomplete-patch hint and the light adoption UAT`.

---

## Close

### LA-19: docs (PR LA)

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `la-19`, from the integration tip after LA-18. **Depends:** LA-01..LA-18 integrated.
- **Files (CRLF):**
  - The player guide adoption section (grep `docs/guides/` for "existing game folder"): Light, what "can't be applied" means, Repair.
  - `docs/operations/telemetry.md`: the field list, with the six event rows in the C-LA events table.
  - `crates/launcher/desktop/docs/`: the update doc (grep "Update" in that folder) covers the incremental path, the write set, hard links and Repair as the full rebuild. It states that the seedless Update applies **only** to installs adopted through light adoption (`check = light` in the adoption provenance), and that a launcher-downloaded install updates exactly as before, from the seed (D-LA14).
  - Walk `docs/agents/doc-update-map.md` and list the rows touched in the report.
- **Check:** no `<!-- gen: -->` block is edited by hand. No IPs. **Commit:** `LA-19: document light adoption, incremental update and the index comparison`.

### LA-20: re-run the LT-37 publish (after PR I2)

- **Agent:** hands-on. Authorized by the owner 2026-10-21 (the LT-37 run) and reaffirmed in D-LA11: "re-run the publish with signing and rollback". **Host:** windows. **Depends:** PR I2 merged with green CI (ask `ops` to confirm).
- **Steps:** follow the LA-05 runbook from a fresh worktree at `origin/main`, building the tools through the lane.
  1. Reuse `B:\lt37-publish`. Re-download the live pair and compare its sha with `worknotes/LT-37.md`. If it moved, rebuild the index.
  2. Run `prepare-file-index.ps1` with `-BlobPath` set to the new release asset URL. Every check must PASS.
  3. Create the release `content-file-index-<yyyymmdd>` and upload `file-index.json`.
  4. Back up the live pair, sign, and `verify`.
  5. Swap `content-current`.
  6. Download again and run `verify-file-index --previous <backup>`. On any FAIL, roll back at once.
- **Report:** update `worknotes/LT-37.md` with the outcome, the digests, the release name and the rollback line. No key material, no addresses. Retire the worktree and the `B:\targets\` folder.
