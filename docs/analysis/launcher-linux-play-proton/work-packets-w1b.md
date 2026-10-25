# Work packets: launcher-linux-play-proton (wave 1B, Linux install quality)

Ledger: [README.md](README.md). Rules R1-R9 and § Dispatch: [work-packets.md](work-packets.md).
Read them first; they apply here unchanged. **Host: linux** throughout.
Paths: `D/` = `crates/launcher/desktop/`, `E/` = `D/engine/src/`, `S/` = `D/shell/src/`,
`F/` = `D/frontend/src/`, `LC/` = `crates/launcher-core/src/`. Line numbers
were read at `origin/main` `85cc212f3`; find by name when they have moved.

Source: the owner's 2026-10-25 Deck install test (8m33s from a local
`client.rar`: copy 2m16s, an unreported full re-read about 2 min, cabinet
expansion 3m46s single-threaded at about 28 MB/s). The owner approved every
item here, parallel decompression included. None of it needs Proton.
OD-3 (decided 2026-10-25): PP-20 **removes** that re-read for a local archive
(D-PP26, F-23); it does not label it.

## Dispatch

- Runs beside wave 1 and wave 2; no Proton file is touched. Branches are cut
  from the integration tip; `ops` merges with `merge-noff.sh` in this order:
  **PP-24, PP-25, PP-20, PP-22, PP-21, PP-23**. PP-22 and PP-21 are cut after
  PP-20 has merged; PP-23 after PP-21. Wave 2's PP-15 is cut after PP-20 and
  PP-25 have merged (it touches `LC/unpack/`).
- Wave review (one packet-reviewer, the six packets together) before W2's
  wave review. No PR: the campaign ships one final PR (D-PP17).
- Extra rule for this wave: **R10.** `cimmeria-launcher-core` is checked with
  `bash tools/build-lane/lane.sh cargo <verb> --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-core ...`.
  Files already past the soft cap: `LC/install.rs` (577), `S/main.rs` (646),
  `E/storage/install_worker/mod.rs` (528), `LC/unpack/cab_portable/fixture.rs`
  (555). Add at most 30 lines to any of them; new tests go in a new sibling
  `*_tests.rs`, never in `install_tests.rs` (586).

## Contract

### C-7 Install progress phases (PP-20)

`LC/install.rs` `Progress` (lines 145-160) gains two variants; `Extracting`
gains a stage. The shell's `JobProgress` (`S/host/install/contract.rs` 254-259)
and the TS type (`F/install-workflow.ts:10`, `F/contract.ts:232`) carry them:

```rust
// LC/install.rs
pub enum ExtractStage { Archive, Cabinets }        // serde snake_case, Copy
Progress::Copying  { copied: u64, total: u64 }      // the local archive copy
Progress::Checking { read: u64, total: u64 }        // SHA-256 of a downloaded or cached archive (total > 0), or the RAR index pass (total 0)
Progress::Extracting { label, current, total, filename, stage: ExtractStage }
// S/host/install/contract.rs, #[serde(tag = "phase", rename_all = "snake_case")]
JobProgress::Copy { current: u64, total: u64 }
JobProgress::Check { current: u64, total: u64 }
JobProgress::Extraction { current: u64, total: u64, stage: ExtractStage }
```
```ts
type InstallProgress =
  | { phase: 'download' | 'copy' | 'check'; current: number; total: number }
  | { phase: 'extraction'; current: number; total: number; stage: 'archive' | 'cabinets' };
```

The summary schema's `TimedPhase` (`E/storage/launcher_summary/schema.rs` 53-58)
is **not** changed: `Copying` and `Checking` map to `TimedPhase::Download`,
both extract stages to `TimedPhase::Extraction` (forwarded schema stays stable).

### C-7b How a staged archive is checked before unpack (PP-20, OD-3)

```rust
// LC/install_seed.rs (the `seed` module of install.rs)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ArchiveCheck<'a> {
    /// Hash the staged file before unpacking: downloads, the seed cache, patches.
    Sha256(&'a str),
    /// `VerifiedLocalSeed::copy_into` hashed the bytes it wrote, fsynced the
    /// copy and renamed it into place only on a match: no second full read.
    HashedWhileCopied,
}
impl ArchiveCheck<'_> {
    pub(super) fn code(self) -> &'static str // "sha256" | "hashed_while_copied"
}
// LC/install.rs: `sha256: &str` becomes `check: seed::ArchiveCheck<'_>`
async fn verify_and_unpack(ctx, tmp, dest, check, label, opts, only_targets)
```

Closed log line, `LC/install_seed.rs`, no-backend path only, just before
`verify_and_unpack`, after the existing `Unpacking seed into` line:
`info!(target: "launcher.install", event = "seed_unpack_started", check = check.code(), bytes = seed.size, "unpacking the seed archive")`.
`check` takes only the two codes above. No path, no file name (R8).

## Wave 1B

### PP-20: Install progress names the real phase; a local archive is read once less

**Agent:** packet-coder. **Host:** linux. **Depends:** none. **Branch:** `launcher-linux-play-proton/pp-20`, worktree `pp-20`.
**Commit:** `feat(launcher): install progress names each phase, and a copied archive is not re-read (PP-20)`.

Files: `LC/install.rs`, `LC/install_seed.rs` (203), `LC/seed_source.rs`,
`LC/seed_source_tests.rs` (visibility only, step 9), `LC/install_tests.rs`
(the one `verify_and_unpack` call near 160 only), `LC/unpack/mod.rs` (lines
98-105 and 152-161 only), `LC/unpack/rar.rs` (line ~108 only), `S/host/install/contract.rs`,
`E/storage/install_worker/mod.rs` (`watch_phases` 253-278 only),
`F/install-workflow.ts`, `F/contract.ts` (line ~232 only), new
`LC/install_progress_tests.rs`; new `LC/install_hash.rs` only if step 3's
cap note needs it. Do **not** touch `LC/unpack/cab_portable/` (PP-25 owns it).

Finding F-23 (read at `origin/main` `85cc212f3`): a local-archive install
reads the 4.1 GB archive three times. Read 1: the pick-time
`verify_local_seed` (`seed_source.rs` 198-227, size then SHA-256). Read 2:
`copy_into`/`copy_to` (240-288) re-checks the source's identity and size,
hashes **the same chunks it writes** to `<dest>.local-copy`, fsyncs, and
renames onto the staged path only on a hash match. Read 3: `install_seed.rs`
`apply` logs `Unpacking seed into …` (line 52), then `verify_and_unpack`
(`install.rs` 394-429) runs `verify_sha256` → `hash_file` (539-570) over the
staged file. Read 3 only computes a SHA-256 and compares it; nothing later
uses the digest, and `unpack::unpack_with` opens and parses the RAR itself.
So read 3 is redundant **on the local-copy path only**. For a download
(`download_to_file` hashes nothing) and for a patch it is the only integrity
check and stays. What removing it gives up (owner-accepted, D-PP26): a check
of the staged copy as it sits on disk, after the rename, which would catch a
storage fault that corrupted the written bytes or a change to the staged file
in the launcher's own folder before unpack.

1. C-7 types. Every existing `Progress::Extracting { .. }` literal gets
   `stage: ExtractStage::Archive` unless step 4 says otherwise.
2. `seed_source.rs` `stream_file` (398-425): when it streams the **copy**
   (`VerifiedLocalSeed::copy_into`, 240-288), report `Progress::Copying`
   instead of `Downloading { label: CHECK_LABEL }`. The pick-time verify
   (`start_verify`) keeps its current event. Add a parameter, not a global.
   `copy_to` keeps every check it has (identity, size, hash of the written
   bytes, `sync_all`, rename on match); D-PP26 relies on them.
3. `install.rs`: keep `pub fn hash_file(path)` with its signature (callers:
   `bin/build-file-index.rs`, `bin/verify-file-index.rs`,
   `E/storage/update/incremental.rs`). Add a private
   `fn hash_file_reporting(path, progress: Option<&ProgressSink>)` that
   `hash_file` calls with `None`; it reports `Progress::Checking { read, total }`
   every 8 MiB and once at the end. `verify_sha256` gains
   `progress: Option<&ProgressSink>`: `verify_and_unpack` passes the sink, the
   seed-cache check (`install_seed.rs` ~127) passes a clone of `ctx.progress`,
   the backend check (~71) passes `None`. Cap (R10): if `install.rs` would grow
   by more than 30 lines, move `hash_file`, `hash_file_reporting` and
   `verify_sha256` into a new `LC/install_hash.rs` (`#[path]` module of
   `install.rs`, `pub use` of `hash_file` so `install::hash_file` still resolves).
4. `unpack/mod.rs`: `UnpackSink` gets `pub stage: ExtractStage` (default
   `Archive`), and `report` (98-105) puts it in the event. In `unpack_with`,
   before `cab_set::expand` (line 160), clone the sink with
   `stage: ExtractStage::Cabinets` and pass the clone.
5. `rar.rs`: the listing pass (`open_for_listing`, 63-80) reports
   `Progress::Checking { read: entries_seen, total: 0 }` every 256 entries
   (`total: 0` = no known total; the frontend shows no percent).
6. `contract.rs` `progress()` (254-274): map the new variants (C-7).
7. `watch_phases`: map per C-7's last paragraph; no schema change.
8. TS: widen the type (C-7); `npm run check` must pass. No copy text here
   (PP-21).
9. **Drop read 3 (OD-3, D-PP26, C-7b).** In `install_seed.rs`: add
   `ArchiveCheck`; `copy_local` returns `Ok(ArchiveCheck::HashedWhileCopied)`;
   `fetch_seed` returns `ArchiveCheck::Sha256(&seed.sha256)` for the cache hit
   and the download, and `copy_local`'s value from the switch branch (~166).
   `apply` passes the value to `verify_and_unpack` and emits C-7b's
   `seed_unpack_started` line just before it. In `install.rs`,
   `verify_and_unpack` takes `check` instead of `sha256`: with `Sha256(s)` it
   runs `verify_sha256` exactly as today (delete on `HashMismatch`), with
   `HashedWhileCopied` it goes straight to `unpack::unpack_with`. `apply_patch`
   (~384) passes `ArchiveCheck::Sha256(&patch.sha256)`; the `install_tests.rs`
   call near 160 passes `ArchiveCheck::Sha256("00")` (that test must still
   pass: the download path keeps its hash). The backend branch of `apply`
   (66-78) is **unchanged**: it has no production caller and authenticates
   before platform code sees the archive. In `seed_source_tests.rs` make
   `recording_sink`, `seed_for`, `manifest_for`, `minimal_pe` and
   `assert_installed_pe` `pub(crate)`, and in `seed_source.rs` make the test
   module `pub(crate) mod tests;`; find `write_stored_rar4` with
   `grep -rn "fn write_stored_rar4" crates/launcher-core/src` and import it.

Tests (unit, `install_progress_tests.rs`, registered from `install.rs` with
`#[cfg(test)] #[path = "install_progress_tests.rs"] mod progress_tests;`):
(a) a local-seed copy of a 3 MiB temp file reports only `Copying`, ending
`copied == total`; (b) `hash_file_reporting` on a 9 MiB temp file reports
`Checking` twice and ends `read == total`; (c) `unpack_with` on the existing
portable-cabinet RAR fixture (the one `installer_rar_from_portable_cabinets_expands_on_every_host`
builds, `unpack/mod.rs:361`) reports `stage: Cabinets` for cabinet files and
`Archive` for RAR files; (d) **the revert guard for step 9**: an `install_all`
with a verified local seed, built like `install_all_uses_a_local_seed_without_downloading`
(`seed_source_tests.rs:166`) but with `progress` from `recording_sink()`,
succeeds, installs `SGW.exe` (`assert_installed_pe`), and records **no**
`Progress::Checking` with `total > 0` anywhere in the run (the only
`Checking` events allowed are the RAR index pass, `total == 0`). It fails
when step 9 is reverted, because read 3 then reports `Checking { total: size }`
through step 3; (e) `ArchiveCheck::Sha256("x").code() == "sha256"` and
`ArchiveCheck::HashedWhileCopied.code() == "hashed_while_copied"`
(launcher-core has no log-capture helper and no `tracing-subscriber`
dev-dependency; do not add one). Each test fails when its step is reverted;
name the line in the report. Must pass untouched:
`a_file_replaced_after_the_check_is_refused_by_copy_into`,
`hash_mismatch_deletes_the_download` (with its one-line call change) and
`complete_authenticated_seed_cache_works_offline_but_corruption_fetches_again`.
Shell: a `contract.rs` test that `Progress::Copying` serialises as
`{"phase":"copy",...}`.

Lane (R1, R10): core fmt, clippy, `test --lib install`, `test --lib seed_source`
and `test --lib unpack`; shell fmt, clippy, `test --lib install`; engine
clippy (it calls `install::hash_file`); then R2 `npm run check`, `npm test`.
If the owner's `client.rar` is on the Deck, report the wall time from
`seed_unpack_started check="hashed_while_copied"` to the first `extraction`
event (expected seconds, not about 2 minutes). Docs: none (PP-13 row).

### PP-21: Install view copy, phase lines, rate and ETA

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-20 merged. **Branch:** `pp-21`.
**Commit:** `feat(launcher): the install view says what it is doing, with a rate and ETA (PP-21)`.

Files: `F/install-view.ts` (`progressLine` 222-238 and the phase reset near
43 and 90-92), `F/transfer-rate.ts` only if a helper is missing, the
install-view test file beside the existing frontend tests, and the install
REPL UAT script (find with `ls D/frontend/*uat*.mjs`; if none covers install,
create `D/frontend/install-uat.mjs` and add `"uat:install": "node install-uat.mjs"`
to `D/frontend/package.json`).

Copy (exact strings; `X of Y` uses the existing size formatter):

| Event | Line |
|---|---|
| `download` | unchanged ("Downloading verified game content… X of Y · rate · ETA") |
| `copy` | `Copying your archive… X of Y · rate · ETA` |
| `check`, `total > 0` | `Checking the download… N% · rate · ETA` (a downloaded, cached or patch archive; a local archive never shows it, D-PP26) |
| `check`, `total == 0` | `Reading the archive index…` |
| `extraction`, `archive` | `Unpacking the archive… N% · N files/s · ETA` |
| `extraction`, `cabinets` | `Expanding game files… N% · N files/s · ETA` |

Rate and ETA use `makeRateTracker` (`F/transfer-rate.ts` 62-97) for every
phase, reset on any change of `phase` or `stage`, shown once the tracker has
a rate (its 1 s minimum span). Byte phases show MB/s; extraction shows
files/s. Drop the "at least 100 MiB" condition for `copy` and `check` only.

Tests: one per table row (the line for a sample event), and the tracker
resets when `stage` changes. UAT script: feed two scripted event sequences,
the local archive copy → check(total 0) → extraction/archive →
extraction/cabinets, and the download download → check(total > 0) →
check(total 0) → extraction/archive → extraction/cabinets, and print one
PASS/FAIL line per table row; exit 1 on a FAIL. The local sequence must
never print `Checking the download…`.
Checks (R2): `npm run check`, `npm test`, the UAT script; paste its output
in the report (project rule: frontend changes need a REPL logic UAT).

### PP-22: Install log lines: per-phase bytes and the archive pick

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-20 merged. **Branch:** `pp-22`.
**Commit:** `feat(launcher): install logs each phase with its bytes, and every archive pick (PP-22)`.

Files: `E/storage/install_worker/events.rs` (305), `E/storage/install_worker/mod.rs`
(`watch_phases` only, R10 cap), `S/host/seed_archive.rs` (437; lines 87-100,
190-200, 371-383), `S/host/setup_events.rs` (111), their tests.

1. `events.rs`: new `install_phase_finished` on target `launcher.install`:
   fields `phase` (the C-7 phase name, plus `stage` for extraction),
   `bytes` (last `current` of a byte phase, else 0), `files` (last `current`
   of extraction, else 0), `elapsed_ms`. `watch_phases` emits it when the
   phase (or stage) changes and when the job ends. A `check` with
   `total == 0` (the RAR index pass) is not a byte phase: `bytes` is 0. After
   D-PP26 a local-archive install logs `copy`, then `check` with `bytes=0`,
   then the extraction stages; a `check` with bytes there is a regression.
2. `setup_events.rs`: `seed_archive_picked` (fields `bytes`, the archive
   size) and `seed_archive_pick_refused` (field `reason`, a closed code:
   `signing_key_unavailable`, `release_unavailable`, `verify_failed`,
   `busy`), both `launcher.install`. No path, no file name (R8).
3. `seed_archive.rs`: call them on the `choose_seed_archive` path
   (371-383 and `settle()` 190-200) where today nothing logs, including the
   dev-key refusal mapped at 87-100. Keep the existing
   `seed_archive_started/_finished/_failed` lines.

Tests: with the crate's existing log-capture helper (find it with
`grep -rn "fn capture\|tracing_test\|with_default" E/storage/install_worker S/host`):
a phase change Copy → Check emits one `install_phase_finished` with
`phase="copy"` and `bytes`; a dev-key pick refusal emits
`seed_archive_pick_refused reason="signing_key_unavailable"` and no path.

Lane (R1): engine and shell fmt, clippy, `test --lib install` / `test --lib seed_archive`.

### PP-23: The archive flow shows and changes the game folder

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-21 merged. **Branch:** `pp-23`.
**Commit:** `feat(launcher): installing from an archive shows where the game goes (PP-23)`.

Finding: the archive button (`D/frontend/ui/index.html:47`, `F/install-view.ts`
`chooseArchive` 176-177, `installFromArchive` 199-206) installs into the saved
`preferences.install_directory`, which is the default
`<app data>/Stargate Worlds` unless Settings changed it (`F/view.ts` 138-156,
command `choose_install_directory`, `S/main.rs` 110-134). The owner's test
landed in the default without noticing. Ruling D-PP25: show it, do not add
a second dialog.

Files: new `F/install-folder.ts` (the choose-and-save flow moved out of
`F/view.ts` 138-156 into `export async function chooseInstallFolder(...)`),
`F/view.ts` (call it), `F/install-view.ts`, `D/frontend/ui/index.html`
(one line and one button beside `#install-seed-archive`), the frontend tests,
the install UAT script from PP-21.

1. Beside the archive button: `Game folder: <install_directory>` and a
   `Change…` button that calls `chooseInstallFolder`. The path text comes
   from the preferences the native side returned (R9: the frontend never
   supplies a path; it only displays one).
2. The line updates on the first change (project rule: visible feedback on
   the first press); a cancelled dialog leaves it unchanged and says nothing.
3. Settings keeps working through the same helper (no second copy of the flow).

Tests: the line renders the saved folder; after a mocked successful choose it
shows the new folder; cancel leaves it. UAT: add a "folder shown, change,
cancel" step to the install UAT; paste the output.
Checks (R2): `npm run check`, `npm test`, the UAT script.

### PP-24: Upload log lines name the scheme and host

**Agent:** packet-coder. **Host:** linux. **Depends:** none. **Branch:** `pp-24`.
**Commit:** `feat(launcher): log-upload lines say which scheme and host they used (PP-24)`.

Files: `E/storage/log_upload/ingest.rs` (440; `log_finished` 419-436 and its
call sites in `send_with` 289-372), `E/storage/game_telemetry/launcher_session.rs`
(385; `LauncherSender::post_batch` and `usable_session` 238-252), their tests.

1. `post_batch`: after each response, `info!(target: "launcher.telemetry", event = "log_upload_post", scheme, host, status)`
   with `scheme` and `host` from the endpoint URL it posted to (`url.scheme()`,
   `url.host_str()`), and `status` the HTTP status as `u16` (0 when no response).
   No port, no path, no query.
2. The `ChunkSink` trait (used by `send_with`) gains
   `fn endpoint(&self) -> Option<(String, String)> { None }` (default method);
   `LauncherSender` returns the last posted scheme and host.
   `log_finished` adds `scheme` and `host` fields (`"-"` when `None`).

Tests: a fake sink that reports `("https","play.example")` makes
`log_upload_finished` carry `scheme="https" host="play.example"`; a sink
returning `None` logs `"-"`. Lane (R1): engine fmt, clippy, `test --lib log_upload`.

### PP-25: Expand independent cabinet folders in parallel

**Agent:** rust-gameserver-dev. **Host:** linux. **Depends:** none. **Branch:** `pp-25`.
**Commit:** `perf(launcher): expand independent cabinet folders on several threads (PP-25)`.

Why the split is by folder: MSZIP keeps a 32 KiB history across the blocks of
one folder (`LC/unpack/cab_portable/mszip.rs` 30-59), so one folder's blocks
decode in order. Folders are independent once `plan()` (`cab_portable/mod.rs`
122-221) has joined the ones that continue across cabinets.

Files: `LC/unpack/cab_portable/mod.rs` (389; the loop at 101-109, `Progress`,
`expand_folder` 230-295 read path), new `LC/unpack/cab_portable/positional.rs`,
new `LC/unpack/cab_portable/parallel_tests.rs`. `mod.rs` must stay under 500
lines: move code into a new `cab_portable/schedule.rs` if needed.

1. `positional.rs`: `pub(super) fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()>`
   with `#[cfg(unix)]` `std::os::unix::fs::FileExt::read_exact_at` and
   `#[cfg(windows)]` a `seek_read` loop (the module also builds in Windows
   tests). Replace every seek-then-read on the shared `sources` handles in
   the expand path with it, so the handles opened at 79-82 stay the only
   handles (keep the comment at 78) and threads never share a cursor.
2. Schedule: `workers = min(std::thread::available_parallelism(), 4, folders.len())`.
   Assign folders deterministically: sort by compressed bytes descending
   (ties by plan order), give each to the worker with the fewest bytes so far.
   Run with `std::thread::scope`; each worker expands its folders in plan order.
3. Progress: the `done` counter becomes an `AtomicUsize` shared by workers;
   each finished file still reports once (the `complete()` path at 382-389).
   `total` and the INF count check (85-98) are unchanged.
4. Errors and cancel: the first error wins; set a shared `AtomicBool` that
   every worker checks before each block and each file; return that first
   error after the scope ends. `sink.check_cancel()` still stops all workers.
5. Add `pub(super) fn expand_chain_with(.., workers: usize)`; `expand_chain`
   calls it with the computed count. Tests call it directly.
6. Unchanged: preflight, case and unsafe-name refusals before any output,
   the LZX/Quantum refusal, every size check listed in `mszip.rs` and
   `mod.rs`, file dates. Every existing test in `cab_portable/tests.rs` and
   `unpack/mod.rs` must pass untouched.

Tests (`parallel_tests.rs`, unit, with the builders in `fixture.rs`):
(a) a set of 3 cabinets with 6 MSZIP folders (one continued across a cabinet
boundary) expands byte-identical with `workers` 1 and 4; (b) with `workers` 4
the sink sees reports from more than one thread id (record
`std::thread::current().id()` in a test sink); fails when the loop is made
sequential again; (c) a corrupt block in folder 3 of 6 fails with the same
error kind as with `workers` 1; (d) a cancelled token stops before any file.
Run the existing history and split-block guard
`expands_a_spanning_set_with_split_blocks_and_cross_block_history` and say it passes.

Lane (R1, R10): core fmt, clippy, `test --lib cab_portable`, `test --lib unpack`.
Then the ignored real-archive test on the Deck if the archive is present:
`... test --lib real_client_rar -- --ignored` (find its env var in
`unpack/mod.rs` near 448; the owner's `client.rar` path goes in the env only,
never in a file). Report the before/after expansion time if it ran.
Docs: `crates/launcher/desktop/README.md` cabinet paragraph (lines 69-75,
348-349): one sentence that folders expand in parallel (R6 CRLF).
</content>
</invoke>
