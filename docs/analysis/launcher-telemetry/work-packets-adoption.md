# Launcher telemetry: work packets (wave 5 adoption reasons, wave 6 file index)

Ledger: [README.md](README.md). Added 2026-10-21 for owner directives 3 and 4 ([owner-directives.md](owner-directives.md)). **The rules R1-R10 (plus R5a), the check table (D*/C*/S*/F1/L0) and contracts C1-C13 are in [work-packets.md](work-packets.md); read § Rules there first.** Every packet here is `Host: windows`: PowerShell only, never Bash, every command starts `Set-Location <worktree>;`, cargo only through `pwsh tools/build-lane/lane.ps1`, foreground, at most 5 fix rounds per check, no `git stash` / `git worktree prune` / push, one local commit. Files under `docs/` are CRLF. Soft cap 500 lines, hard cap 700: in a file over 450 lines add call lines only.

## Ownership against launcher-adopt-ux (D-LT16)

| Area | Owner | Notes |
|---|---|---|
| `unsafe_file` refusals: the survey, `UnsafeReport`, `Error::Unsafe`, status `unsafe_entries`, the refusal list on screen | AU (AU-01..AU-04) | LT never records a C14 rejection for an `UnsafeFile` site; AU-02 already logs those |
| Mode choice UI | AU (AU-05) | |
| The six adoption step events (`step`, `decision_outcome`, `error`) | AU (AU-09) | LT adds no field to them |
| Rolling log file, `adoption` forwarding, `LauncherSender` | AU (AU-08, AU-13) | LT events ride on them unchanged |
| Server row rules for `client.launcher.adoption` | AU (AU-14) | LT adds no server change |
| **Every other** refusal and fall-through: a specific check, entry and expected-vs-found in the UI (`last_rejection`), the log and telemetry (`adoption_rejected`) | **LT wave 5** | includes AU F17 (`TooLarge` shown as `corrupt_state`), which AU left unsliced |
| The file-index fallback reason (`index_fallback`, `adoption_index_fallback`) | **LT wave 5** (LT-30) | |
| Publishing the file index: tooling, runbook, owner publish | **LT wave 6** | any-install-adoption AI-14 becomes LT-37; D-AI8 stays the owner's |

## Branches and base

- Wave 5 integration: `launcher-telemetry/adoption`, worktree `lt-adopt-integration`. `ops` creates it from `lt-app-integration` **after LT-05 has merged there** (C9 capture helper, C1 table). Amendment 3: no AU merge is needed; AU wave 1 (`Error` without `Copy`, `unsafe_entries`) and AU-09 are on `main` via #1953, so `lt-app-integration` (created from `origin/main`) already has them. Packet branches `launcher-telemetry/lt-<nn>-<slug>` from its tip at dispatch; worktrees `lt-28` … `lt-34`.
- Wave 6 integration: `launcher-telemetry/file-index`, worktree `lt-index-integration`, from `origin/main`. Worktrees `lt-35`, `lt-36`.
- Each packet's report: `<worktree>\.claude\reports\launcher-telemetry\<packet>-coder.md`. Return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

## Contract

### C14: adoption rejection (LT-28)

```rust
// crates/launcher/desktop/engine/src/storage/adoption/rejection.rs (new, ≤300 lines incl. tests)
/// Closed list; the frontend text table (C17) and the shell keep the same codes.
pub const REJECTION_CHECKS: &[&str] = &[
    // source and folder (LT-28, LT-29)
    "not_a_game_folder", "folder_not_writable", "folder_in_use", "folder_overlaps_launcher",
    "folder_unreadable", "import_changed", "folder_changed_during_check", "folder_too_large",
    "folder_too_deep", "path_not_canonical", "unsupported_catalog", "unsupported_configuration",
    "stale_revision", "busy", "destination_exists", "destination_overlaps", "launcher_too_old",
    "identity_conflict", "comparison_failed", "exe_at_root",
    // verification and artifacts (LT-30, LT-31)
    "no_reusable_files", "seed_mismatch", "setup_rule_invalid", "file_unclassified",
    "artifact_missing", "artifact_hash_mismatch", "artifact_size_mismatch",
    "download_http_status", "download_failed", "local_seed_invalid", "local_artifact_mismatch",
    "manifest_digest_malformed", "patch_replay_mismatch", "patch_archive_unsupported",
    "patch_input_unexpected", "patch_layout_unsupported",
    // in place, reference, worker, consent (LT-32)
    "leftover_launcher_files", "unsupported_layout", "invalid_directory", "reference_zip_invalid",
    "worker_stopped", "blocking_files",
    // shell (LT-33)
    "import_required", "state_corrupt", "release_unavailable", "platform_unavailable",
    "recovery_required",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Rejection {
    pub check: &'static str,          // one of REJECTION_CHECKS (debug_assert in new())
    pub entry: Option<String>,        // R5a: folder-relative path or a manifest label
    pub expected: Option<String>,     // digest() form, a count, or a short closed phrase
    pub found: Option<String>,
    pub http_status: Option<u16>,
}
impl Rejection {
    pub fn new(check: &'static str) -> Self;
    pub fn entry(self, e: impl Into<String>) -> Self;     // each string cut to 120 bytes, keeping the tail
    pub fn expected(self, e: impl Into<String>) -> Self;
    pub fn found(self, f: impl Into<String>) -> Self;
    pub fn http_status(self, s: u16) -> Self;
}
/// "size 4096, sha256 0123456789ab" (the first 12 hex digits, lower case).
pub fn digest(size: u64, sha256_hex: &str) -> String;
/// `path` relative to `root`, '/'-separated, "." for the root; reuse AU's helper if one
/// exists (grep `fn relative` under storage/adoption/), else implement it here.
pub fn relative(root: &std::path::Path, path: &std::path::Path) -> String;
/// The snake_case code of `error`, identical to the shell's `AdoptionError` serde name.
pub fn code(error: &Error) -> &'static str;
/// Records `r` as the scope's rejection if none is recorded yet (the first is the cause,
/// later ones are consequences), WARN-logs it, and returns `error` unchanged.
pub fn reject(error: Error, r: Rejection) -> Error;
/// Runs `f` with a fresh thread-local slot and returns what it recorded. Nested scopes
/// get their own slot and restore the outer one on exit (used by the index fallback).
pub fn scope<R>(f: impl FnOnce() -> R) -> (R, Option<Rejection>);
/// (LT-33a) Logs `r` exactly like `reject`, with `code` = the caller's error code (the
/// shell's `AdoptionError` serde name), and records nothing in any slot. Only for
/// shell-originated refusals, which have no engine `Error`.
pub fn refuse(code: &str, r: &Rejection);
```

The engine re-exports `Rejection`, `refuse` and `IndexFallback` from `storage/adoption/mod.rs` (LT-33a); the `rejection` module itself stays private.

The WARN, emitted only in `rejection.rs` (by `reject` and `refuse`, through one private `fn warn(code: &str, r: &Rejection)`):

```rust
tracing::warn!(target: "adoption", event = "adoption_rejected", check = r.check, code = code(&error),
    entry = r.entry.as_deref(), expected = r.expected.as_deref(), found = r.found.as_deref(),
    http_status = r.http_status,
    "adoption: check {} failed for {}: expected {}, found {}; the folder was not adopted", ...);
```

(`None` values are left out, not logged as empty.) At most 8 fields, so it fits AU's 16-field row rule. Rules for every site packet:

- Call `reject` **only** where the `Err` leaves the operation. A caught error that falls back (the index path) runs inside a nested `scope` and converts what it recorded (C15).
- Replace `return Err(Error::X)` with `return Err(rejection::reject(Error::X, Rejection::new("<check>").entry(..).expected(..).found(..)))`, and `map_err(|_| Error::X)` with `map_err(|e| rejection::reject(Error::X, Rejection::new("<check>").found(<closed code from e>)))`. The returned variant stays the same, so existing tests that match `Err(Error::X)` keep passing.
- Never a `Display`/`Debug` of an error, an absolute path, a URL or a host in any field (R5, R5a).
- Do not touch `UnsafeFile` or `Error::Unsafe` sites (AU owns them).

### C15: index fallback (LT-30)

```rust
// in the new storage/adoption/index_source.rs (LT-30)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IndexFallback {
    pub reason: &'static str,        // see below
    pub check: Option<&'static str>, // the nested scope's Rejection.check, if any
    pub http_status: Option<u16>,
    pub seed_bytes: u64,             // manifest.seed.size: what the build downloads
}
pub enum IndexOrigin { Published, Built(IndexFallback) }
```

Reasons: `index_not_published` (no `file_index` in the manifest), `index_reference_malformed`, `index_download_failed`, then one per `FileIndexError` variant (`index_size_mismatch`, `index_digest_mismatch`, `index_parse_failed`, `index_bind_failed`, …; match every variant by name, grep `enum FileIndexError` in `crates/launcher-core/src/file_index/mod.rs`). Events on target `adoption`:

- `adoption_index_fallback` (warn: `reason`, `check`, `http_status`, `seed_bytes`): "adoption: expected the published file index, got <reason>; downloading the signed seed (<seed_bytes> bytes) to check the folder instead".
- `adoption_index_published` (info: `cached: bool`).
- A cache file that does not verify (`read_cache`) is `debug` only; it is downloaded again.

The comparison report that reaches the review carries `index_fallback: Option<IndexFallback>` (None when published).

### C16: shell DTOs (LT-33a..LT-33c; amended 2026-10-21)

```rust
// shell/src/host/adoption/rejection_dto.rs (new, LT-33b)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AdoptionRejection { pub check: String, pub entry: Option<String>, pub expected: Option<String>, pub found: Option<String>, pub http_status: Option<u16> }
impl From<&adoption::Rejection> for AdoptionRejection { /* field by field; check.to_owned() */ }
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct IndexFallbackView { pub reason: String, pub check: Option<String>, pub http_status: Option<u16>, pub seed_bytes: u64 }
impl From<&adoption::IndexFallback> for IndexFallbackView { /* field by field */ }
pub(super) fn progress_item(observed: &Observed) -> Option<ProgressItem>; // Observed = engine install::Progress

// contract.rs (LT-33b)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressItem { Seed, Patch, FileIndex, IndexBuild, Reference }
// Progress keeps `Copy` and gains:  #[serde(skip_serializing_if = "Option::is_none")] pub item: Option<ProgressItem>
// AdoptionStatus gains, after last_error (always serialized, null when none):  pub last_rejection: Option<AdoptionRejection>
// review.rs Review gains:  #[serde(skip_serializing_if = "Option::is_none")] pub index_fallback: Option<IndexFallbackView>
```

`progress_item` matches the engine label exactly; anything else is `None`, and copy-phase progress is always `None`:

| Engine variant | Label | `item` |
|---|---|---|
| `Downloading` | `"seed"` | `seed` |
| `Downloading` | starts with `"patch "` | `patch` |
| `Downloading` | `"file index"` | `file_index` |
| `Extracting` | `"File index"` (the index built from the seed) | `index_build` |
| `Extracting` | `"Authenticated adoption reference"` | `reference` |
| `Extracting` | `"Adoption replay"` | `patch` |

`last_rejection` is the engine's recorded rejection for the **worker** outcome that set `last_error` (the preview through `PreviewWorker::last_rejection()`, the confirmation through `ConfirmationWorker::last_rejection()`, LT-33a). It is set together with `last_error` and cleared with it, and stays `None` when the engine recorded nothing (for example `Io` from a copy fault, or the publication scan's unnamed `SourceChanged`). `index_fallback` lives on the review, not on `AdoptionStatus`. **Shell-originated refusals** (returned synchronously as the command's `Err(AdoptionError)`) are logged through the engine's `refuse` (LT-33c) and are never stored in `last_rejection`; the command's error code is already shown. The TS schema in `frontend/src/adoption-workflow.ts` gains the same fields **in the same commit** (R8, `onExcessProperty:'error'`).

### C17: frontend texts (LT-34)

New `frontend/src/adoption-reasons.ts`: `export function rejectionText(r: AdoptionRejection): string` and `export function fallbackText(f: IndexFallbackView): string`. A text per check, then "File: <entry>." and "Expected <expected>, found <found>." when present. An unknown check shows "Check: <check>." (never the platform text). Examples: `artifact_hash_mismatch` "A signed file did not match its download"; `no_reusable_files` "None of the folder's files match this release"; `folder_in_use` "Another launcher has this folder open"; `index_not_published` (fallback) "This release has no published file index, so the launcher downloads the signed game archive (<n> GB) to check your files. To skip that, use your game archive." Write one sentence per check; no new button.

## Wave 5: adoption reasons (PR A, from `lt-adopt-integration`)

Shared for LT-28..LT-34: rules R1-R10 with R5a; contract C14; checks D1, D2e, D3e `adoption` for engine packets (add D2s, D3s `adoption` for shell, F1 for frontend). Tests (R7): pin target `adoption`, `event`, `check` and `code`, never message text; use the C9 capture helper (`engine/src/test_capture.rs`, from LT-05); drive at least one real failing path through the public preview. Docs: none except LT-34.

### LT-28: rejection core and plumbing

- **Worktree:** `lt-28`, branch `launcher-telemetry/lt-28-rejection-core`. **Contract:** C14.
- **Files:** new `engine/src/storage/adoption/rejection.rs` (declare in `adoption/mod.rs`); `adoption/mod.rs` (390 lines on main, larger after AU: call lines only); `adoption/worker.rs`.
- **Change:**
  1. Implement C14 with inline tests.
  2. Plumbing: grep `pub fn preview` and `pub fn confirm` (and the copy-mode equivalent) in `adoption/mod.rs`. Wrap each body in `rejection::scope` and keep the result in a new field `last_rejection: Option<Rejection>` on the struct that owns them, with `pub fn last_rejection(&self) -> Option<&Rejection>`. If the work runs on the retained worker (`worker.rs`), wrap the job closure there in `scope` and send the `Option<Rejection>` back with the job result. If carrying it needs changes outside `mod.rs` and `worker.rs`, return `blocked` with the shape you found.
  3. Sites in `mod.rs`: `idle` (:345-370) `stale_revision` (expected and found revisions as numbers), `busy`; :224-229 `launcher_too_old` (expected = the release's minimum launcher, found = this version) and `identity_conflict`; :239-250 `destination_exists` / `destination_overlaps` (entry = the destination's last path component only); :287-294 the copy-mode comparison error, `comparison_failed`, `found = code(&error)` (today it is silent).
- **Tests:** `digest`, `relative`, `code` for every variant (a `match` with no wildcard, so a new variant fails to compile), first-wins and nested `scope`; `reject` under capture; one real path: a copy preview into an existing destination returns the same `Err` as before and `last_rejection().check == "destination_exists"`. Reverted, `last_rejection()` is `None`.
- **Commit:** `LT-28: adoption rejection record, log and plumbing`.

### LT-29: source, inventory and comparison checks

- **Worktree:** `lt-29`, branch `launcher-telemetry/lt-29-source-checks`, after LT-28. **Files:** `adoption/source.rs`, `adoption/inventory.rs`, `adoption/comparison.rs`.
- **Sites** (main line numbers; re-grep, AU wave 1 moved some):
  - source.rs :15-19 `import_changed` (expected/found = the 12-digit prefix of the saved and requested confirmation digests); :36-37 `folder_unreadable` (`found` = `error_kind`); :55-56 `folder_overlaps_launcher`; :71 `folder_unreadable`; :89-90 `not_a_game_folder` (`expected` = "SGW.exe and the game folders", `entry` = "."); :163-165, :181-184 `folder_not_writable` (`found` = `error_kind`); :189-194 `unsupported_catalog` / `unsupported_configuration` (closed phrases "custom manifest URL" / "custom diagnostics module"; no URL); :222-223 is a path-canonical `UnsafeFile`: record `path_not_canonical` **only if AU wave 1 did not already turn it into `Error::Unsafe`**; :287 `folder_in_use` (`entry` = "launcher.lock"); :308-319 `folder_changed_during_check` (entry = which of the three files, expected vs found digest).
  - inventory.rs :183-190 `folder_too_large` / `folder_too_deep` (expected = the cap, found = the count or depth); :144, :164-166, :230-232 when they return `SourceChanged` (not `UnsafeFile`): `folder_changed_during_check` with entry, expected and found size. Leave every `UnsafeFile` site alone.
  - comparison.rs :8-9 `exe_at_root`.
- **Tests:** extend source.rs :376-410 (NotAGameFolder) and :450 (FolderNotWritable) to assert `last_rejection` (or the event under capture) with the check; one `folder_too_large` with a small cap if the cap is injectable, else a unit test of the helper.
- **Commit:** `LT-29: name the failed source and inventory checks`.

### LT-30: verification and the index fallback

- **Worktree:** `lt-30`, branch `launcher-telemetry/lt-30-verification`, after LT-28. **Contract:** C14, C15.
- **Files:** `adoption/verification.rs` (675 lines: at the hard cap); new `adoption/index_source.rs` (≤350 lines); `adoption/mod.rs` (one `mod` line); the report type that reaches the review (grep `game_telemetry_available` in `adoption/` to find the report struct; add `index_fallback` next to it).
- **Change:**
  1. **Move** `IndexOrigin`, `acquire_index`, `download_for`, `published`, `read_cache`, `verify`, `built` and `build_error` (verification.rs ~512-700) into `index_source.rs` unchanged, with `pub(super) use` so callers still compile. verification.rs must end under 520 lines.
  2. In `index_source.rs`, implement C15: `IndexOrigin::Built(IndexFallback)`; `published()` returns `Result<Result<FileIndex, IndexFallback>, Error>`; run `artifacts::index_blob` and `verify` inside a nested `rejection::scope` and copy its `check` and `http_status` into the fallback; emit `adoption_index_fallback` or `adoption_index_published` (C15). Replace the free-string warn at :546. Cancellation still aborts; the `stored?` abort at :616 becomes `reject(Error::Storage(Io), Rejection::new("folder_unreadable").entry("file_index").found(kind))`.
  3. The comparison report gets `index_fallback` from the origin.
  4. verification.rs sites: :117 `no_reusable_files` (expected = "at least 1 matching file", found = "0 of <n>"); :119-120 `folder_changed_during_check`; :180 `file_unclassified` (entry); :296, :313 `setup_rule_invalid` (entry = rule path); :670-671 `seed_mismatch` (entry "seed", `digest` expected vs found); :688-693 per `FileIndexError` variant.
- **Tests:** extend verification_tests.rs:147 (`a_published_index_with_the_wrong_digest_falls_back_to_the_build`) to assert `IndexOrigin::Built` with `reason == "index_digest_mismatch"` and one `adoption_index_fallback` event; a manifest with no `file_index` gives `index_not_published` with `seed_bytes == manifest.seed.size`; a 404 from the wiremock origin gives `index_download_failed` with `http_status == 404` once LT-31 lands (until then assert `http_status == None` and say so). Reverted, the fallback has no reason.
- **Commit:** `LT-30: give the file-index fallback and verification refusals a reason`.

### LT-31: artifacts and patch replay

- **Worktree:** `lt-31`, branch `launcher-telemetry/lt-31-artifacts`, after LT-28. **Files:** `adoption/artifacts.rs`, `adoption/replay.rs`.
- **Sites:** artifacts.rs :87-89 `artifact_missing` (entry = slot label); :196-198 `artifact_hash_mismatch` (entry = `seed` or `patch:<id>`, `digest` expected vs found); :215-221 `local_seed_invalid` (found = the `LocalSeedError` variant as snake_case); :260-261 `manifest_digest_malformed`; :319-333 `local_artifact_mismatch`; :366 `download_failed` (found = `timeout`|`connect`|`body`|`other` from the `reqwest::Error` predicates); :369 `download_http_status` with `http_status`; :375, :396 `artifact_size_mismatch` (expected = signed size, found = Content-Length or bytes read). Never the URL. replay.rs :206-225, :240-250 `patch_archive_unsupported` / `patch_layout_unsupported` (entry `patch:<id>`); :254-261 `patch_replay_mismatch` (entry = target relative path, `digest` expected vs found); :289-304 `patch_input_unexpected`; replace the `?error` at :271 with `code = rejection::code(&error)`.
- **Tests:** artifacts_tests: a wiremock 500 gives `download_http_status` with `http_status=500`; a body whose sha differs gives `artifact_hash_mismatch` with both digests; replay_tests: one mismatch case asserts `patch_replay_mismatch`.
- **Commit:** `LT-31: name the failed artifact and replay checks`.

### LT-32: in-place preview, reference, worker and consent

- **Worktree:** `lt-32`, branch `launcher-telemetry/lt-32-in-place`, after LT-28. **Files:** `adoption/in_place_preview.rs`, `adoption/reference.rs`, `adoption/worker.rs` (the panic and channel sites only), `adoption/model.rs` (the `consent` check, call lines only), `adoption/in_place.rs` (669 lines: the `refused` helper at :17-19 only).
- **Sites:** in_place_preview.rs :49-91 `invalid_directory` ×4 (`expected` = the closed reason phrase already there); :109-111 `unsupported_layout`; :143-148 `comparison_failed`; :215-253 `leftover_launcher_files` (entry = the leftover's relative path, `found` = which marker: `content_ready`, `game_not_empty`, `marker`, `work_folder`). reference.rs: do not touch all ~20 sites; wrap the one public entry point's error (grep `pub(super) fn` in reference.rs) in `reject(e, Rejection::new("reference_zip_invalid").entry(<zip entry name if the error carries it, else "reference">))`. worker.rs :20-21, :81 `worker_stopped` (found = `panic` or `channel_closed`; never the panic payload). model.rs `consent` :134-135 `blocking_files` (entry = the first blocking file, found = the count). In `in_place.rs` `refused`, replace `?error` with `code`.
- **Tests:** one in-place preview with a leftover `content-ready.json` asserts `leftover_launcher_files` and the entry; the consent test asserts `blocking_files` with the count.
- **Commit:** `LT-32: name the in-place, reference and consent refusals`.

### LT-33a..LT-33c: shell status, progress item and shell refusals (re-sliced 2026-10-21)

The first LT-33 coder blocked on six packet gaps; the rulings are in [LT-33-amendment.md](LT-33-amendment.md). LT-33 is now three sequential packets on the adoption integration branch (`launcher-telemetry/adoption`, tip `9285f623a` holds LT-28..LT-32). Each starts from the integration tip after the previous one merges. All paths below are relative to `crates/launcher/desktop/` unless they start with `crates/` or `docs/`.

**Rules every LT-33 packet carries** (packet-coder does not read CLAUDE.md):

1. **Host: windows.** PowerShell only, never Bash. Every command starts `Set-Location <worktree>;` with the absolute worktree path given in the packet. Run each lane command in the foreground and wait for it.
2. Never run `cargo` directly; only the lane commands listed in the packet (`pwsh tools/build-lane/lane.ps1 ...`).
3. Edit only the files the packet lists. One local commit with the given subject. No `git stash`, no push, never `git worktree prune`.
4. File caps: soft 500 lines, hard 700. A file already over 450 lines gets call or declaration lines only. Re-count after `cargo fmt`; a file over 700 is a `blocked` return.
5. R5/R5a: no `Display`/`Debug` of an error, no absolute path, URL or host in any log field or `Rejection` field. Only closed phrases, counts, digests from `digest()`, folder-relative paths and manifest labels.
6. Tests pin target `adoption`, `event`, `check` and `code`, never message text. Every behavior change has a test that fails when the change is reverted.
7. At most 5 fix rounds per failing check, then return `blocked` with the last `status=` line.
8. Line numbers are from `9285f623a`; re-grep before editing. If a quoted line is not where the packet says and you cannot find it by grep, return `blocked` with what you found. Do not invent a different design.
9. Report to `<worktree>\.claude\reports\launcher-telemetry\<packet>-coder.md` (checks run with their `status=` lines, files and line counts, any stop rule hit). Return exactly `<packet> <done|blocked|failed> <sha|-> <report-path>`.
10. Files under `docs/` are CRLF.

#### LT-33a: engine refusal log, confirmation rejection, re-exports

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33` (already at `9285f623a`, unedited), branch `launcher-telemetry/lt-33-shell`. **Contract:** C14 (`refuse`, re-exports).
- **Files (only these):**
  - `engine/src/storage/adoption/rejection.rs` (300 lines).
  - `engine/src/storage/adoption/worker.rs` (159 lines).
  - `engine/src/storage/adoption/mod.rs` (457 lines: declaration lines only).
  - new `engine/src/storage/adoption/confirmation_tests.rs` (≤120 lines).
- **Change:**
  1. `rejection.rs`: move the body of the `tracing::warn!` in `reject` (:152-175) into a private `fn warn(code: &str, r: &Rejection)` with the same fields and message. `reject` calls `warn(code(&error), &r)` before recording, exactly as today. Add `pub fn refuse(code: &str, r: &Rejection) { warn(code, r) }` with a doc comment ("Logs a shell-originated refusal like `reject`; records nothing"). If the name `refuse` is already used in `adoption/`, name it `log_refusal` and say so in the report.
  2. `mod.rs`: replace `use rejection::Rejection;` (:85) with `pub use rejection::{refuse, Rejection};`. Add `pub use index_source::IndexFallback;` next to it. Leave `#[allow(dead_code)] mod rejection;` (:26) as it is (the module stays private). Next to `#[cfg(test)] mod tests;` (:66-67) add `#[cfg(test)] mod confirmation_tests;`.
  3. `worker.rs`:
     - `ConfirmationWorker` (:55-60) gains a private field `rejection: watch::Receiver<Option<Rejection>>` and, in its `impl`, `pub fn last_rejection(&self) -> Option<Rejection> { self.rejection.borrow().clone() }` with the doc comment "The check that refused the confirmation, once `result` is set; `None` when nothing was recorded."
     - In `start_confirmation_with` (:126-159): add `let (rejection_send, rejection) = watch::channel(None);` next to the result channel. Inside `spawn_blocking`, wrap the `catch_unwind(...)` in `rejection::scope(|| ...)` exactly like `start_preview_backend` does at :95-105, and change `.unwrap_or(Err(StorageError::PersistenceUncertain.into()))` to `.unwrap_or_else(|_| Err(rejection::reject(StorageError::PersistenceUncertain.into(), Rejection::new("worker_stopped").found("panic"))))`. Bind `let (outcome, recorded) = rejection::scope(...)`; then `rejection_send.send_replace(recorded);` **before** `send.send_replace(Some(outcome));` (a reader that sees the result must see the rejection). Add `rejection` to the `ConfirmationWorker { .. }` literal.
- **Tests:**
  - In `rejection.rs`'s test module, copy the capture pattern of the existing `reject` test (it uses `crate::test_capture::capture` and finds the row with `event == "adoption_rejected"`). New test `refuse_logs_the_shell_code_and_records_nothing`: `let ((), recorded) = scope(|| { let (_, logs) = capture(|| refuse("import_required", &Rejection::new("import_required").found("no_saved_import"))); /* assert on logs */ });`. Assert one row with target `adoption`, level `WARN`, `check == "import_required"`, `code == "import_required"`, `found == "no_saved_import"`, and `recorded == None`.
  - New `confirmation_tests.rs`, test `a_refused_confirmation_carries_its_failed_check`. Use `use super::tests::{choices, Fixture};` and `use super::*;`. Copy the test attribute and the `start_confirmation(...)` call (with `preview.report.preview_handle` / the handle the existing test uses) from the test in `tests.rs` around :541. Steps: `let f = Fixture::new();` → `let preview = f.preview().unwrap();` → `std::fs::write(f.source.launcher_directory.join("launcher-config.json"), b"{}").unwrap();` → start the confirmation → `let mut result = worker.result.clone(); tokio::time::timeout(std::time::Duration::from_secs(30), result.wait_for(|r| r.is_some())).await.unwrap().unwrap();`. Assert the outcome is `Some(Err(e))` with `rejection::code(&e) == "source_changed"`, and `worker.last_rejection()` is `Some(r)` with `r.check == "folder_changed_during_check"`, `r.entry.as_deref() == Some("launcher-config.json")`, and `r.expected` and `r.found` both `Some`. Reverted (no scope in the worker), `last_rejection()` is `None`.
  - **Stop rule:** if the outcome is not `source_changed`, or the entry differs, do not change the engine to make it pass. Return `blocked` with the observed code, check and entry.
- **Checks:**
  - `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33; pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`
  - `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33; pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`
  - `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33; pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine adoption`
  - `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33; pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings` (the shell must still compile; it uses none of the new items yet).
  - Expected: `refuse_logs_the_shell_code_and_records_nothing` and `a_refused_confirmation_carries_its_failed_check` listed and passing; no existing `adoption` test fails.
- **Commit:** `LT-33a: log shell refusals and carry the confirmation rejection in the engine`.

#### LT-33b: status, review and progress DTOs with the TS schema

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33b` (ops creates it at the integration tip after LT-33a merges), branch `launcher-telemetry/lt-33b-status`. **Contract:** C16.
- **Files (only these):**
  - new `shell/src/host/adoption/rejection_dto.rs` (≤200 lines, with inline tests).
  - new `shell/src/host/adoption/rejection_tests.rs` (≤200 lines).
  - `shell/src/host/adoption/contract.rs` (267 lines).
  - `shell/src/host/adoption/status.rs` (261 lines).
  - `shell/src/host/adoption/review.rs` (177 lines).
  - `shell/src/host/adoption/mod.rs` (497 lines: declaration and field lines only, at most 4).
  - `frontend/src/adoption-workflow.ts` (schema only) and every frontend file that holds a full `AdoptionStatus` literal (grep `unsafe_entries:` under `frontend/`; today `src/adoption-view.test.ts:10` and possibly `adoption-in-place-uat.mjs`).
- **Change:**
  1. `rejection_dto.rs`: the two structs, the two `From` impls and `progress_item` exactly as in C16 and its table. `use cimmeria_launcher_engine::install::Progress as Observed;` (as status.rs does). Match `Observed::Downloading { label, .. }` / `Observed::Extracting { label, .. }` on `label.as_str()`.
  2. `contract.rs`: add `ProgressItem` (C16) next to `Progress` (:220-225); `Progress` gains `item` with the skip attribute and keeps `Copy`. `AdoptionStatus` (:245-267) gains `pub last_rejection: Option<AdoptionRejection>` right after `last_error`, with no skip attribute.
  3. `mod.rs`: `mod rejection_dto;` with the other `mod` lines (:15-44), `pub use rejection_dto::{AdoptionRejection, IndexFallbackView};` if `contract.rs` needs the name (else `use`), `#[cfg(test)] mod rejection_tests;`, and the field `last_rejection: Option<AdoptionRejection>` in `struct Adoption` (:73-82) after `last_unsafe`. If `Adoption` is built with a struct literal anywhere (grep `Adoption {`), add `last_rejection: None` there.
  4. `status.rs`:
     - Progress (:41-67): the copy branch sets `item: None`; the preview branch sets `item: rejection_dto::progress_item(observed)`.
     - `cancellable` (:99-101) uses `progress` after the struct field takes it; keep it compiling (`Progress` is still `Copy`, so no change should be needed).
     - Status literal: `last_rejection: adoption.last_rejection.clone(),` after `last_error`.
     - `settle_adoption` preview branch: right after `try_result()` (:112) and before `adoption.preview = None;` (:113), read `let rejection = adoption.preview.as_ref().and_then(|w| w.last_rejection().cloned());`. Pass it to `fail` at :131.
     - Confirm branch: extend the `finished` tuple (:139-145) to `(job.in_place, outcome, job.worker.last_rejection())` and pass the third value to `fail` at :149.
     - `fail` (:164) becomes `fail(&mut self, error: adoption::Error, rejection: Option<adoption::Rejection>)` and sets `self.last_rejection = rejection.as_ref().map(AdoptionRejection::from);`. `clear_failure` (:159) also sets `self.last_rejection = None;`. These are the only two `fail` callers.
  5. `review.rs`: `Review` gains `index_fallback` (C16) after `download`; `Review::of` sets `index_fallback: report.index_fallback.as_ref().map(IndexFallbackView::from),`.
  6. `frontend/src/adoption-workflow.ts` (same commit, R8):
     - status (next to `last_error` at :25): `last_rejection:Schema.NullOr(Schema.Struct({check:Schema.String,entry:Schema.NullOr(Schema.String),expected:Schema.NullOr(Schema.String),found:Schema.NullOr(Schema.String),http_status:Schema.NullOr(Count)}))`.
     - progress (:22): `item:Schema.optionalKey(Schema.Literals(['seed','patch','file_index','index_build','reference']))`.
     - Review (:6-15): `index_fallback:Schema.optionalKey(Schema.Struct({reason:Schema.String,check:Schema.NullOr(Schema.String),http_status:Schema.NullOr(Count),seed_bytes:Count}))`.
     - Follow the file's existing helper names (`Count`, `Schema.optionalKey`, `Schema.NullOr`, `Schema.Literals`). If one of them is spelled differently there, use the file's spelling.
     - Add `last_rejection:null` after `last_error:null` in every full status literal found by the grep (`adoption-view.test.ts:10` `idle()`; literals that spread `...idle()` need nothing).
- **Tests:**
  - `rejection_dto.rs` inline: `progress_items_map_the_engine_labels` (every table row plus `"something else"` → `None`, and a `Downloading` label `"patchwork"` → `None`); `a_rejection_and_a_fallback_convert_field_by_field`.
  - `rejection_tests.rs` (`use super::fixture::*; use super::*;`, `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):
    - `a_refused_preview_shows_the_failed_check_beside_its_error`: `let mut f = Fixture::new(serde_json::json!({}));` → start a `wiremock::MockServer`, mount `path("/seed.zip")` answering `ResponseTemplate::new(500)` (copy the mount from `Fixture::serve`, fixture.rs:52-60) → `f.serve_from(format!("{}/manifest.json", server.uri()));` → `f.begin().unwrap();` → `let s = until(&f.host, "the refusal", Duration::from_secs(30), |s| s.last_error.is_some()).await;`. Assert `s.last_error == Some(AdoptionError::Network)` and `s.last_rejection` is `Some(r)` with `r.check == "download_http_status"`, `r.entry.as_deref() == Some("seed")`, `r.http_status == Some(500)`. Reverted (status.rs not copying), `last_rejection` is `None`.
    - `a_refused_confirmation_shows_the_failed_check`: copy the setup of a confirming test (`play_tests.rs:125-138`: `Fixture::new`, `f.serve().await`, `begin_adoption_preview` or `f.begin()`, `reviewed(&f.host).await`). After the review and before `f.host.adoption_command(confirmation(&review)).unwrap();`, write `b"{}"` to `f.source.launcher_directory.join("launcher-config.json")`. Wait with `until(.., |s| s.last_error.is_some())`. Assert `last_error == Some(AdoptionError::SourceChanged)` and `last_rejection.check == "folder_changed_during_check"`.
    - `the_status_json_names_the_new_fields`: `serde_json::to_value(f.host.adoption_status().unwrap())` on a fresh fixture has key `last_rejection` equal to `null`; `serde_json::to_value(Progress { phase: Phase::Download, current: 1, total: 2, item: Some(ProgressItem::Seed) })` has `"item":"seed"`, and with `item: None` has no `item` key.
    - **Stop rule:** if a preview or confirmation reaches a different `last_error` (for example a local seed is used and the preview reaches review), do not change the fixture or the engine. Return `blocked` with the observed `last_error` and `last_rejection`.
  - `frontend/src/adoption-view.test.ts`: test `the status decoder accepts a rejection, a progress item and an index fallback`: `mount` replies with `{...reviewing(), last_rejection:{check:'download_http_status',entry:'seed',expected:null,found:null,http_status:500}, progress:{phase:'download',current:1,total:2,item:'seed'}, review:{...review(), index_fallback:{reason:'index_not_published',check:null,http_status:null,seed_bytes:1}}}` and asserts no schema error is shown (copy how an existing test checks a decoded status renders).
- **Checks** (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33b`; prefix each with `Set-Location <worktree>;`):
  - `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`
  - `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings`
  - `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop adoption`
  - `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend`
  - Expected: the five new Rust tests and the new TS test pass; no existing test fails.
- **Commit:** `LT-33b: carry the adoption rejection, progress item and index fallback to the UI`.

#### LT-33c: log shell-originated refusals, and the migration doc

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33c` (ops creates it after LT-33b merges), branch `launcher-telemetry/lt-33c-refusals`. **Contract:** C14 (`refuse`), C16 (shell refusals are logged, not stored).
- **Files (only these):**
  - new `shell/src/host/adoption/refusal.rs` (≤120 lines, with its tests).
  - `shell/src/host/adoption/events.rs` (one word: `fn code` becomes `pub(super) fn code`).
  - `shell/src/host/adoption/mod.rs` (~500 lines: call lines and one `mod` line only; must stay under 560).
  - `shell/src/host.rs` (one `use` line).
  - `shell/src/main.rs` (616 lines: the `ReleaseUnavailable` line only).
  - `docs/migration.md` under `crates/launcher/desktop/` (CRLF; one paragraph).
- **Change:**
  1. `refusal.rs`:

     ```rust
     //! Shell-originated adoption refusals: logged like engine rejections (C14), never stored.
     use super::{events, AdoptionError};
     use cimmeria_launcher_engine::adoption;
     /// Logs `adoption_rejected` with `check` and the closed phrase `found`, and returns `error`.
     pub(crate) fn refusal(error: AdoptionError, check: &'static str, found: &'static str) -> AdoptionError {
         adoption::refuse(&events::code(error), &adoption::Rejection::new(check).found(found));
         error
     }
     ```

     Use the same `cimmeria_launcher_engine::adoption` path that `mod.rs` uses for `adoption::Error`.
  2. `mod.rs`: `pub(crate) mod refusal;` with the other `mod` lines, and `use refusal::refusal;`. Wrap exactly these sites (line numbers from `9285f623a`; re-grep the error variant):

     | Line | Error | `check` | `found` |
     |---|---|---|---|
     | :56 | `CorruptState` (legacy import unreadable) | `state_corrupt` | `legacy_import_unreadable` |
     | :57 | `ImportRequired` | `import_required` | `no_saved_import` |
     | :65 | `InvalidDirectory` (location unresolvable) | `invalid_directory` | `location_unresolvable` |
     | :68 | `InvalidDirectory` (copy folder exists) | `invalid_directory` | `copy_folder_exists` |
     | :142 | `PlatformUnavailable` | `platform_unavailable` | `no_backend` |
     | :145 | `PersistenceUncertain` | `recovery_required` | `reopen_required` |
     | :148 | `RecoveryRequired` | `recovery_required` | `reconciliation_pending` |
     | :151 | `IdentityConflict` | `identity_conflict` | `owned_installation` |
     | :160 | `Busy` | `busy` | `operation_running` |
     | :235 | `InvalidDirectory` (copy without a location) | `invalid_directory` | `no_location` |
     | :253 | `Busy` (`admit_adoption_preview`) | `busy` | `adoption_active` |

     Forms: `return Err(AdoptionError::X)` → `return Err(refusal(AdoptionError::X, "<check>", "<found>"))`; `.map_err(|_| AdoptionError::X)` → `.map_err(|_| refusal(AdoptionError::X, "<check>", "<found>"))`; `.ok_or(AdoptionError::X)` → `.ok_or_else(|| refusal(AdoptionError::X, "<check>", "<found>"))`. **Never** `ok_or(refusal(..))` or any other eager form: it would log on success. Do not wrap the macOS-only sites (:300, :303), the `UnsupportedMode` sites (:210, :232) or any confirm-side site (:407-436). **Stop rule:** if one of the listed sites is reached from `adoption_status` (status polling; grep the enclosing fn's callers), leave it unwrapped and list it in the report.
  3. `host.rs`: next to `pub(crate) use adoption::events as adoption_events;` add `pub(crate) use adoption::refusal::refusal as adoption_refusal;`.
  4. `main.rs` (:394-396): `.map_err(|_| AdoptionError::ReleaseUnavailable)?` → `.map_err(|_| crate::host::adoption_refusal(AdoptionError::ReleaseUnavailable, "release_unavailable", "catalog_unavailable"))?`.
  5. `crates/launcher/desktop/docs/migration.md`, after the `AdoptionStatus.unsafe_entries` text (:201-214): one paragraph on `AdoptionStatus.last_rejection` (the failed check, file, expected and found for a worker refusal, `null` otherwise), `Review.index_fallback` (why the seed archive is downloaded) and `Progress.item` (the closed list). Say that a refusal returned directly by a command is logged as `adoption_rejected` with its check and is not stored. CRLF.
- **Tests** (in `refusal.rs`, `#[cfg(test)] mod tests`; copy the capture pattern from the tests in `events.rs`: grep `ForwardLayer::new`; it uses `tracing::subscriber::with_default(Registry::default().with(ForwardLayer::new(tx)), || ..)` and reads `records[i].level` and `.fields["..."]`):
  - `refusal_logs_the_check_and_returns_the_error`: under the capture, `refusal(AdoptionError::ImportRequired, "import_required", "no_saved_import")` returns `AdoptionError::ImportRequired` and logs one record with `event == "adoption_rejected"`, `check == "import_required"`, `code == "import_required"`, `found == "no_saved_import"`.
  - `a_shell_refusal_logs_its_check` (`#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, `use super::super::fixture::*;`): `let mut f = Fixture::new(serde_json::json!({}));` → `f.serve_from("http://127.0.0.1:9/manifest.json".into());` → `std::fs::create_dir_all(f.location().join(COPY_FOLDER)).unwrap();` (the `mod.rs` constant; if it is not visible, the literal `"Stargate Worlds"`) → under the capture call `f.begin()`. Assert `Err(AdoptionError::InvalidDirectory)` and one record with `event == "adoption_rejected"`, `check == "invalid_directory"`, `code == "invalid_directory"`, `found == "copy_folder_exists"`. Reverted (mod.rs:68 unwrapped), no such record. **Stop rule:** if `f.begin()` returns anything else, return `blocked` with the result.
- **Checks** (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-33c`; prefix each with `Set-Location <worktree>;`):
  - `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`
  - `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings`
  - `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop adoption`
  - Expected: both new tests pass; no existing test fails; `mod.rs` under 560 lines after fmt.
- **Commit:** `LT-33c: log shell-originated adoption refusals with their check`.

### LT-34: frontend reasons and the player guide

- **Worktree:** `lt-34`, branch `launcher-telemetry/lt-34-frontend`, after LT-33c. **Contract:** C17 (and the C16 field shapes). **Checks:** F1. `last_rejection` is set only for worker refusals (C16); a synchronous command error shows its code as today.
- **Files:** new `frontend/src/adoption-reasons.ts` and `adoption-reasons.test.ts`; `adoption-view.ts` (201 lines); `adoption-view.test.ts`; `docs/client/launcher-guide.md` § "Use an existing game folder" (CRLF).
- **Change:**
  - Under the failure text, render `rejectionText(status.last_rejection)` when present.
  - `failureText` (:39): an unknown code shows "Adoption stopped: <code>." instead of the platform-unavailable text; add an `import_required` entry.
  - While `activity` is preparing and `progress.item` is `seed`, `patch` or `index_build` (C16 table), show "Downloading the signed game archive to check your files (no usable file index)". In the review, show `fallbackText(review.index_fallback)` when present.
  - Each `reportUiEvent`-free: these are engine-logged already; add no `ui_event` call.
  - Guide: one short subsection "Why adoption downloaded the game archive" and one "Reading an adoption refusal" (check, file, expected vs found). Walk `docs/agents/doc-update-map.md` and list the rows.
- **Tests:** each C17 branch; an unknown code renders its code; extend adoption-view.test.ts:290 (each failure code renders its own text) with a rejection; the preparing-seed text.
- **Commit:** `LT-34: show why an adoption was refused or downloaded the archive`.

## Wave 6: publish the file index (PR I, from `lt-index-integration`)

Background: `Manifest.file_index: Option<FileIndexRef { blob, size, sha256 }>` (`crates/launcher-core/src/manifest.rs:177`, `file_index/mod.rs:35-50`). No index is published, so every adopt downloads the 4.1 GB seed (any-install-adoption AI-14, D-AI8; `docs/analysis/any-install-adoption/worknotes/campaign-close.md:52-54`). `build-file-index` builds one; `cimmeria-patchset sign|verify` signs with Ed25519 (`crates/patchset/src/bin/cimmeria-patchset.rs`); the release key is offline with the owner (`docs/client/launcher-distribution-setup.md:20-22`). `Manifest::validate` (manifest.rs:243-265) checks the `after` chain. Nothing checks a candidate manifest plus index offline before publishing.

### LT-35: `verify-file-index`: chain, binding and parity before publishing

- **Worktree:** `lt-35`, branch `launcher-telemetry/lt-35-verify-file-index`, from `lt-index-integration`. **Rules:** R1-R4, R7, R9, R10. **Checks:** C1, C2, C3 `file_index`, C3 `manifest_chain`.
- **Files:** new `crates/launcher-core/src/manifest_chain.rs` (≤250, inline tests; declare in `lib.rs`); new `crates/launcher-core/src/file_index/check.rs` (≤300, inline tests; declare in `file_index/mod.rs`); new `crates/launcher-core/src/bin/verify-file-index.rs` (≤150); `crates/launcher-core/Cargo.toml` only if bins are listed explicitly (copy the `build-file-index` entry).
- **Code:**

```rust
// manifest_chain.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainBreak { SeedChanged, PatchRemoved(String), PatchMoved(String), PatchChanged { id: String, field: &'static str }, AfterChanged(String) }
/// `next` may only append patches after `previous`'s last one and add or replace
/// `file_index`. Seed and every previous patch (order, id, after, size, sha256, blob) unchanged.
pub fn append_only(previous: &Manifest, next: &Manifest) -> Result<(), Vec<ChainBreak>>;

// file_index/check.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding { pub check: &'static str, pub ok: bool, pub detail: String }
/// Checks, in order: manifest_parses, manifest_valid (Manifest::validate), file_index_present,
/// blob_form_matches_seed (same form as the seed's `blob`: both absolute https URLs on one host,
/// or both relative), index_under_cap, index_size, index_digest, index_parse_verified,
/// index_bind, then append_only when `previous` is given, then parity when `built` is given
/// (the candidate index equals the one `file_index::build` makes from the local seed and
/// patches, compared as parsed values).
pub fn check_publication(candidate: &[u8], index: &[u8], previous: Option<&[u8]>, built: Option<&FileIndex>) -> Vec<Finding>;
```

- **Bin:** `verify-file-index --manifest <candidate.json> --file-index <file-index.json> [--previous <current.json>] [--seed <seed.zip> --patch <id>=<zip>...]`. With `--seed`, build the index exactly as `build-file-index` does (reuse its library call; grep `file_index::build` in that bin) and pass it as `built`. Print `PASS <check>` or `FAIL <check>: <detail>` per finding; exit 1 on any FAIL. It never signs, uploads or reads a key.
- **Tests:** `append_only`: identical passes; appended patch passes; removed, reordered, changed sha and changed `after` each fail with the right `ChainBreak`. `check_publication` on a small fixture (copy the fixture builder from the existing `file_index` tests; grep `fn build` tests): all PASS; a flipped digest byte fails `index_digest`; a manifest whose `file_index.size` is wrong fails `index_size`; a reordered patch with `previous` fails `append_only`. Reverted, the tool does not exist and these tests fail.
- **Docs:** none (LT-36).
- **Commit:** `LT-35: verify a file index and manifest chain before publishing`.

### LT-36: publish runbook and prepare script

- **Agent:** packet-coder. **Worktree:** `lt-36`, after LT-35. **Rules:** R1-R4, R9. No cargo beyond D/C checks; PowerShell only.
- **Files:** new `tools/content-release/prepare-file-index.ps1` (≤200; first read `tools/README.md` § Tools catalog and follow its naming; add one catalog row, inside no `gen:` block); `docs/client/launcher-distribution-setup.md` § "File index" (~136-150, CRLF).
- **Script** (`-Seed <zip> -Patch <id=zip>... -Current <manifest.json> -Out <dir> -BlobPath <the blob value> -BuildExe <path> -VerifyExe <path>`; the exes come from a lane build, found with `$env:LANE_VERBOSE='1'` as LT-25 does):
  1. `cimmeria-patchset verify` the current manifest against the release public key given as `-PubKey` (hex; public, never the private key).
  2. Run `build-file-index` into `<Out>\file-index.json`.
  3. Write `<Out>\manifest.candidate.json` = the current manifest plus the `file_index` entry (`blob` = `-BlobPath`, size, sha256), changing nothing else (ConvertFrom-Json/ConvertTo-Json -Depth 20; then run step 4 to prove nothing else moved).
  4. Run `verify-file-index --manifest candidate --file-index … --previous <Current> --seed … --patch …`; stop on any FAIL.
  5. Print, but never run, the owner's remaining commands: sign with the offline key, `cimmeria-patchset verify` the signature, upload `file-index.json` to the content release named by D-LT15, upload the signed manifest and `.sig` to `content-current`, then LT-27 U7.
- **Docs:** replace "publishing is an owner step waiting on D-AI8" with the runbook: the script, the chain rule (append only; `after` links never remapped, per the manifest-swap rule), the parity check as the launcher-verify step, and the owner-only signing and upload steps. No addresses, no key material.
- **Test:** none at runtime (the logic is LT-35's); the report shows one dry run against LT-35's fixture or says why it could not run.
- **Commit:** `LT-36: runbook and prepare script for publishing the file index`.

### LT-37: owner publishes the file index (BlockedDecision D-LT15)

- **Agent:** none. The owner (key holder) runs LT-36's printed steps after PR I merges: prepare, sign with the offline release key, verify, upload, swap `content-current`. No agent signs, uploads or touches the key (D-LT15, D-AI8).
- **Then:** LT-27 U7 on the laptop that would not adopt. The orchestrator records the result in the ledger.
