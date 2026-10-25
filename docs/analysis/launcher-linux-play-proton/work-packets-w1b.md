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
  **PP-24, PP-25, PP-20, PP-22, PP-21, PP-23**. PP-20 is one branch run as
  session 1, PP-20a and PP-20b, merged once after PP-20b (`PP-20-ruling-1`).
  PP-22 and PP-21 are cut after PP-20 has merged; PP-23 after PP-21. Wave 2's PP-15 is cut after PP-20 and
  PP-25 have merged (it touches `LC/unpack/`).
- Wave review (one packet-reviewer, the six packets together) before W2's
  wave review. No PR: the campaign ships one final PR (D-PP17).
- Extra rule for this wave: **R10** (corrected by `PP-20-ruling-1`).
  `cimmeria-launcher-core` is a member of the **root** workspace. `D/Cargo.toml`
  only pulls it in as a path dependency, so `test` refuses it there ("not a
  member of the workspace"). Check it from the worktree root with
  `bash tools/build-lane/lane.sh cargo <verb> --locked --manifest-path Cargo.toml -p cimmeria-launcher-core ...`.
  The engine (`cimmeria-launcher-engine`) and the shell (`cimmeria-launcher-desktop`)
  keep R1's `--manifest-path crates/launcher/desktop/Cargo.toml`. The desktop
  manifest is used with `-p cimmeria-launcher-core` in one place only: PP-15's
  one-off lockfile `cargo check` without `--locked`.
  Files already past the soft cap: `LC/install.rs` (587 after PP-20 session 1;
  PP-20a shrinks it), `S/main.rs` (646),
  `E/storage/install_worker/mod.rs` (528), `LC/unpack/cab_portable/fixture.rs`
  (555). Add at most 30 lines to any of them; new tests go in a new sibling
  `*_tests.rs`, never in `install_tests.rs` (586).

## Contract

### C-7 Install progress phases (PP-20; re-specced by PP-20-ruling-1)

**New variants only.** `Downloading` and `Extracting` keep their fields, and
`UnpackSink` keeps its three fields. So none of the ~35 literal constructions
in the engine, shell, bins and tests changes, and PP-25's new `UnpackSink`
literal (`cab_portable/parallel_tests.rs`) still compiles. There is **no
`stage` field**: cabinet expansion is its own variant and its own wire phase.

```rust
// LC/install.rs `Progress`: three variants after `Extracting`
Copying  { copied: u64, total: u64 }  // the local archive copy (copy_into)
Checking { read: u64, total: u64 }    // SHA-256 of a downloaded, cached or patch archive (total > 0), or the RAR index pass (total 0)
Expanding { label: String, current: usize, total: usize, filename: String } // cabinet expansion inside a RAR seed
// S/host/install/contract.rs `JobProgress`, #[serde(tag = "phase", rename_all = "snake_case")]
Copy { current: u64, total: u64 }       // from Copying
Check { current: u64, total: u64 }      // from Checking
Expansion { current: u64, total: u64 }  // from Expanding
```
```ts
// every TS schema that decodes a JobProgress: still one flat struct, a wider literal list
{ phase: 'download' | 'copy' | 'check' | 'extraction' | 'expansion'; current: number; total: number }
```

Which unpack step is which: `UnpackSink::report(step, ..)` (moved to
`LC/unpack/progress.rs`) emits `Expanding` when `step == CABINET_STEP`
(`"expanding cabinets"`, the literal that `cab_portable/mod.rs` `complete()` and
`fdi.rs` already pass). Every other step (`unpacking RAR`, `unzipping`,
`patching`) emits `Extracting`. The engine's own `Extracting` literals
(adoption, mac_wine, incremental update) are unchanged and reach the wire as
`extraction`.

Mappings that stay stable:
- The summary schema's `TimedPhase` (`E/storage/launcher_summary/schema.rs`
  53-58) is not changed. `Copying` and `Checking` map to `TimedPhase::Download`;
  `Extracting` and `Expanding` map to `TimedPhase::Extraction`.
- The adoption DTO `Phase` (`S/host/adoption/contract.rs` 218) is not changed.
  `Copying` and `Checking` map to `Download`, and `Expanding` maps to
  `Extraction`. A `Checking { total: 0 }` (the RAR index pass) is not shown.
- The archive worker (`E/archive_worker/process/mod.rs` 115) forwards
  `Expanding` the way it forwards `Extracting`. Otherwise the macOS Wine
  install would freeze its bar during cabinet expansion.

Where `check` reaches the wire (for the reviewer): install and repair
(`InstallStatus.progress`), game update (`GameUpdateStatus.progress`, a patch's
hash), and the seed-archive `verifying` field. Their TS decoders run
`onExcessProperty: 'error'` against a closed literal list. All three schemas
(`F/install-workflow.ts:10`, `F/game-update-workflow.ts:6`,
`F/contract.ts:232`) widen in the same packet as the shell mapping (PP-20b).

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

**Status (`PP-20-ruling-1`, 2026-10-25).** PP-20 runs as three sessions on
**one branch**, `launcher-linux-play-proton/pp-20` in worktree `pp-20`, in this
order: session 1 (done), **PP-20a**, **PP-20b**. `ops` merges the branch once,
after PP-20b, in PP-20's § Dispatch slot. Between PP-20a and PP-20b the
engine and the shell do not compile (new `Progress` variants meet exhaustive
matches). That is expected; nothing merges in between, and no other packet
branches from pp-20 before it merges.

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

**Session 1, done in `0e64f5b3a` (do not redo).** Step 9 (OD-3, C-7b):
`ArchiveCheck`; `copy_local` returns `HashedWhileCopied`; `fetch_seed` returns
`Sha256`; `verify_and_unpack(.., check, ..)`; `apply_patch` passes
`Sha256(&patch.sha256)`; the `seed_unpack_started` line. Also done: the one
`install_tests.rs` call (`ArchiveCheck::Sha256("00")`) and test (e) in
`LC/install_progress_tests.rs` (registered as `progress_tests`). Session 1
checks: core clippy `--all-targets -D warnings` ok; core `test --lib install`
ok (71 passed). Its worknote is `docs/analysis/launcher-linux-play-proton/worknotes/PP-20.md`
in the worktree.

**The ruling on its blocker.** No new field goes on `Extracting` or
`UnpackSink`; new variants replace them (C-7). The `seed_source_tests.rs`
visibility changes that the old step 9 asked for are dropped, because tests
(a) and (d) now live in that file. The old steps 1-8 and tests (a)-(d) are
replaced by PP-20a and PP-20b below. The Deck wall-time measurement moves to
the wave-1B Deck UAT (RESUME).

### PP-20a: launcher-core reports copy, check and expansion

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-20 session 1 (already on the branch).
**Branch:** `launcher-linux-play-proton/pp-20`, worktree `pp-20`: continue on it, do not cut a new branch.
**Commit:** `feat(launcher): install progress reports copy, check and expansion (PP-20a)`.

Rules for this packet (you skip CLAUDE.md, so these are the ones that apply):
1. Lane, bash, from the worktree root: `bash tools/build-lane/lane.sh cargo <verb> --locked --manifest-path Cargo.toml -p cimmeria-launcher-core ...` (R10). Never run cargo directly.
2. Edit only the files listed below. Do not touch `LC/unpack/cab_portable/` (PP-25 owns it) or `LC/unpack/fdi.rs`; both keep the literal `"expanding cabinets"`.
3. **Do not read whole files.** Read only the line ranges named here (numbers are at `0e64f5b3a`; find by name if they moved). Every code block below is code to paste as written.
4. Tests are unit tests on temp dirs. Each test must fail when its step is reverted: try the revert, see the test fail, restore, and name the line in the report.
5. Caps: soft 500, hard 700 lines. `LC/install.rs` must end at or below 587 lines, and `LC/unpack/mod.rs` below 490 (PP-15 needs room under 500).
6. After this packet the engine and shell do not compile. Do not edit or check them; that is PP-20b.
7. The worknote is CRLF; keep CRLF when you append a "PP-20a" section with the checks.
8. Commit with explicit paths (`git add <file>...`). No `git add -A`, no stash, no rebase.

Files: `LC/install.rs`, new `LC/install_hash.rs`, `LC/install_seed.rs`
(lines ~135 and ~150 only), `LC/seed_source.rs` (`stream_file` and its two
calls), `LC/seed_source_tests.rs` (append two tests), `LC/install_progress_tests.rs`
(append one test), `LC/unpack/mod.rs` (delete `fn report`, add one `mod`
line), new `LC/unpack/progress.rs`, new `LC/unpack/progress_tests.rs`,
`LC/unpack/rar.rs` (one line), and the worknote.

**Step 1. `LC/install.rs` `Progress` (145-160).** After the `Extracting { .. },` variant, add:
```rust
    /// The local archive copy (`VerifiedLocalSeed::copy_into`).
    Copying { copied: u64, total: u64 },
    /// SHA-256 of a downloaded, cached or patch archive (`total > 0`), or the
    /// RAR index pass (`total == 0`: no known total).
    Checking { read: u64, total: u64 },
    /// Cabinet expansion inside a RAR seed; fields as in `Extracting`.
    Expanding {
        label: String,
        current: usize,
        total: usize,
        filename: String,
    },
```

**Step 2. Move hashing into `LC/install_hash.rs`.** Create it with exactly:
```rust
//! SHA-256 of staged files, optionally reporting `Progress::Checking` (C-7).
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{InstallError, Progress};
use crate::install_progress::{ProgressReporter, ProgressSink};

/// How many bytes pass between two `Checking` reports.
const REPORT_EVERY: u64 = 8 * 1024 * 1024;

pub(super) fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), InstallError> {
    verify_sha256_reporting(path, expected, what, None)
}

/// [`verify_sha256`] that reports `Checking` to `progress` while it reads.
pub(super) fn verify_sha256_reporting(
    path: &Path,
    expected: &str,
    what: &str,
    progress: Option<&ProgressSink>,
) -> Result<(), InstallError> {
    let actual = hash_file_reporting(path, progress)?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(InstallError::HashMismatch {
            what: what.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Lower-case hex SHA-256 of the file at `path`.
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    hash_file_reporting(path, None)
}

/// [`hash_file`] that reports `Checking { read, total }` every 8 MiB and once
/// at the end, when `progress` is set.
pub(super) fn hash_file_reporting(
    path: &Path,
    progress: Option<&ProgressSink>,
) -> std::io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let total = f.metadata()?.len();
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 64];
    let mut read = 0u64;
    let mut next_report = REPORT_EVERY;
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        read += n as u64;
        if read >= next_report {
            if let Some(sink) = progress {
                sink.report(Progress::Checking { read, total });
            }
            next_report += REPORT_EVERY;
        }
    }
    if let Some(sink) = progress {
        sink.report(Progress::Checking { read, total });
    }
    // digest 0.11's `Array` output no longer implements `LowerHex`, so the old
    // `{:x}` no longer compiles: hex-encode the bytes explicitly.
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
```
Then, in `LC/install.rs`:
- Delete `fn verify_sha256` and `pub fn hash_file` (546-577, from `fn verify_sha256` to the closing `}` of `hash_file`).
- After `mod tolerant;` (36), add:
  ```rust
  #[path = "install_hash.rs"]
  mod hash;
  pub use hash::hash_file;
  use hash::{verify_sha256, verify_sha256_reporting};
  ```
- Put `#[cfg(test)]` on the line above `use sha2::{Digest, Sha256};` (26); the tests reach `Sha256` through `use super::*`. Delete `use std::io::Read;` (22). If a test module then fails to compile for a missing `read`, add it back as `#[cfg(test)] use std::io::Read;`.
- In `verify_and_unpack` (396-436), add `let check_progress = ctx.progress.clone();` just before `let label = label.to_string();` (420). Then change `verify_sha256(&tmp, &expected, &label)` (423) to `verify_sha256_reporting(&tmp, &expected, &label, Some(&check_progress))`.
- `install::hash_file` keeps its signature and path (callers: `bin/build-file-index.rs`, `bin/verify-file-index.rs`, `E/storage/update/incremental.rs`). The backend check in `install_seed.rs` (line 94) keeps `verify_sha256` with no progress.

**Step 3. `LC/install_seed.rs`, the seed-cache check.** Add `let progress = ctx.progress.clone();`
just before `let complete = tokio::task::spawn_blocking(` (135). Then change
`match verify_sha256(&cached, &expected, "seed") {` (150) to
`match verify_sha256_reporting(&cached, &expected, "seed", Some(&progress)) {`.

**Step 4. `LC/seed_source.rs` `stream_file` (398-425).** Add the parameter
`event: fn(u64, u64) -> Progress,` after `progress: &ProgressSink,`, and
replace the `progress.report(Progress::Downloading { .. });` statement
(419-423) with `progress.report(event(done, size));`. Above `fn stream_file`, add:
```rust
/// The pick-time check's event. It is unchanged; the seed-archive view shows it.
fn check_event(done: u64, total: u64) -> Progress {
    Progress::Downloading {
        label: CHECK_LABEL.to_string(),
        downloaded: done,
        total,
    }
}

/// The install-time copy's event (C-7).
fn copy_event(copied: u64, total: u64) -> Progress {
    Progress::Copying { copied, total }
}
```
In `verify_local_seed` (214), pass `check_event` as the new argument; in
`copy_to` (274), pass `copy_event`. Nothing else in `copy_to` changes: the
identity, size, hash of the written bytes, `sync_all` and rename on a match
stay (D-PP26 relies on them).

**Step 5. Unpack.** Create `LC/unpack/progress.rs` with exactly:
```rust
//! How an unpack step becomes a progress event (C-7). Cabinet expansion is
//! its own phase; every other step is extraction.
use std::path::Path;

use super::UnpackSink;
use crate::install::Progress;
use crate::install_progress::ProgressReporter;

/// The step `cab_portable` and `fdi` report while they expand cabinets.
pub(super) const CABINET_STEP: &str = "expanding cabinets";

/// How many listed RAR entries pass between two index reports.
const INDEX_EVERY: usize = 256;

impl UnpackSink {
    pub(super) fn report(&self, step: &str, current: usize, total: usize, path: &Path) {
        let label = format!("{} ({step})", self.label);
        let filename = path.display().to_string();
        let event = if step == CABINET_STEP {
            Progress::Expanding {
                label,
                current,
                total,
                filename,
            }
        } else {
            Progress::Extracting {
                label,
                current,
                total,
                filename,
            }
        };
        self.progress.report(event);
    }

    /// The RAR listing pass: `Checking { read: entries, total: 0 }` every 256
    /// entries (no known total, so the UI shows no percent).
    pub(super) fn report_index(&self, entries: usize) {
        if entries % INDEX_EVERY == 0 {
            self.progress.report(Progress::Checking {
                read: entries as u64,
                total: 0,
            });
        }
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
```
In `LC/unpack/mod.rs`, delete `fn report` (98-105) from `impl UnpackSink`
and keep `check_cancel`. Add `mod progress;` after `mod prerequisites;` (26).
If clippy then reports `Progress` or `ProgressReporter` unused in the `use`
lines (38-39), remove only that name and keep `ProgressSink`. If that breaks
the inline tests' compile, keep the name behind `#[cfg(test)]` instead.

**Step 6. `LC/unpack/rar.rs`.** After `check_bounds(limits, listed, bytes)?;`
(79), add `sink.report_index(listed);`. Line 108 stays as it is. If the rar test
near 159 (`while let Ok(Progress::Extracting { .. }) = rx.try_recv()`) now
stops early, change only that loop so it skips other events, and say so.

**Tests.**

(a) Append to `LC/seed_source_tests.rs`. Revert check: pass `check_event` in `copy_to`.
```rust
/// The install-time copy reports `Copying`, not the pick-time check's event (C-7).
#[test]
fn copy_into_reports_only_copying_and_ends_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("player.rar");
    let bytes = vec![3u8; 3 * 1024 * 1024];
    std::fs::write(&path, &bytes).unwrap();
    let (check_sink, _checks) = recording_sink();
    let verified =
        verify_local_seed(&path, &seed_for(&bytes), &CancellationToken::new(), &check_sink)
            .unwrap();
    let (sink, recorder) = recording_sink();
    let dest = dir.path().join(".tmp-seed-test.download");
    verified
        .copy_into(&dest, &CancellationToken::new(), &sink)
        .unwrap();
    let events = recorder.events.lock().unwrap();
    let size = bytes.len() as u64;
    assert!(!events.is_empty());
    assert!(
        events.iter().all(|e| matches!(e, Progress::Copying { .. })),
        "{events:?}"
    );
    assert!(
        matches!(events.last(), Some(Progress::Copying { copied, total }) if *copied == size && *total == size),
        "{events:?}"
    );
}
```

(b) Append to `LC/install_progress_tests.rs`, and add `use super::hash::hash_file_reporting;`
and `use super::{Progress, ProgressSink};` at its top. Revert check: delete the
two `sink.report` calls in `hash_file_reporting`.
```rust
#[test]
fn hash_file_reporting_reports_every_8_mib_and_at_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nine.bin");
    let size = 9 * 1024 * 1024u64;
    std::fs::write(&path, vec![5u8; size as usize]).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sink = ProgressSink::from(tx);
    hash_file_reporting(&path, Some(&sink)).unwrap();
    let mut checks = Vec::new();
    while let Ok(event) = rx.try_recv() {
        match event {
            Progress::Checking { read, total } => checks.push((read, total)),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(checks.len(), 2, "{checks:?}");
    assert_eq!(checks.last(), Some(&(size, size)));
}
```

(c) Create `LC/unpack/progress_tests.rs`. Read `LC/unpack/mod.rs` lines
361-403 (the test `installer_rar_from_portable_cabinets_expands_on_every_host`)
and copy its fixture build, from `let dir = tempfile::tempdir()` through
`write_stored_rar4(&archive, &refs);`, into the marked spot. Drop its `use` line;
the imports below replace it. Revert check: make `report` always build `Extracting`.
```rust
use std::path::Path;

use super::CABINET_STEP;
use crate::install::Progress;
use crate::unpack::cab_portable::fixture::{write_cab_set, CabOptions};
use crate::unpack::test_fixtures::{incompressible, sink, write_stored_rar4};
use crate::unpack::unpack;

#[test]
fn only_the_cabinet_step_reports_expanding() {
    let (sink, mut rx) = sink();
    sink.report(CABINET_STEP, 1, 2, Path::new("a"));
    sink.report("unpacking RAR", 1, 2, Path::new("b"));
    assert!(matches!(rx.try_recv().unwrap(), Progress::Expanding { .. }));
    assert!(matches!(rx.try_recv().unwrap(), Progress::Extracting { .. }));
}

// The installer RAR: its own files are `Extracting`, the cabinet files
// `Expanding`. Fails if the cabinet code stops passing CABINET_STEP.
#[test]
fn a_cabinet_seed_reports_both_phases() {
    // <- the fixture build copied from unpack/mod.rs (it defines `dir` and `archive`);
    //    write `incompressible(..)` where the original wrote `super::test_fixtures::incompressible(..)`
    let dest = dir.path().join("install");
    let (sink, mut rx) = sink();
    unpack(&archive, &dest, &sink).unwrap();
    let (mut archive_files, mut cabinet_files) = (0, 0);
    while let Ok(event) = rx.try_recv() {
        match event {
            Progress::Extracting { .. } => archive_files += 1,
            Progress::Expanding { .. } => cabinet_files += 1,
            _ => {}
        }
    }
    assert!(archive_files > 0, "no RAR entry was reported");
    assert_eq!(cabinet_files, 3, "one Expanding per cabinet file");
}
```

(d) The revert guard for session 1's step 9. Append to `LC/seed_source_tests.rs`
a copy of `install_all_uses_a_local_seed_without_downloading` (166-202),
renamed `a_local_seed_install_reads_the_copy_once`, with two changes. First,
add `let (progress, recorder) = recording_sink();` before `install_all` and
pass `progress,` instead of `progress: ProgressSink::latest().0,`. Second, after
`assert_installed_pe(..)`, add:
```rust
    let events = recorder.events.lock().unwrap();
    assert!(
        events.iter().any(|e| matches!(e, Progress::Copying { .. })),
        "{events:?}"
    );
    // Read 3 is gone (OD-3): the only `Checking` allowed is the RAR index pass.
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Progress::Checking { total, .. } if *total > 0)),
        "the staged copy was hashed again: {events:?}"
    );
```
Revert check: in `LC/install_seed.rs` `apply`, temporarily add
`let check = ArchiveCheck::Sha256(&seed.sha256);` right after the `let check = match ..;`
statement (70-73). The test must fail on the `Checking` assertion. Then delete the line.

Must still pass untouched: `a_file_replaced_after_the_check_is_refused_by_copy_into`,
`hash_mismatch_deletes_the_download`,
`complete_authenticated_seed_cache_works_offline_but_corruption_fetches_again`,
`installer_rar_from_portable_cabinets_expands_on_every_host`, and test (e).

Lane (R10, core only): `fmt -p cimmeria-launcher-core`; `clippy --all-targets -- -D warnings`;
`test --lib install`; `test --lib seed_source`; `test --lib unpack`. Paste each
lane status line. Report `wc -l` for `LC/install.rs` and `LC/unpack/mod.rs`.
Docs: none (PP-13 row).

### PP-20b: engine, shell and frontend carry the new phases

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-20a committed on the branch.
**Branch:** `launcher-linux-play-proton/pp-20`, worktree `pp-20`: continue on it.
**Commit:** `feat(launcher): the shell and frontend carry copy, check and expansion (PP-20b)`.

Rules for this packet:
1. Lane, bash, from the worktree root: `bash tools/build-lane/lane.sh cargo <verb> --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <crate> ...`, where `<crate>` is `cimmeria-launcher-engine` or `cimmeria-launcher-desktop` (R1). Core checks use R10's root manifest. Never run cargo directly.
2. Frontend (R2), in `crates/launcher/desktop/frontend`: `npm run check`, then `npm test`.
3. Edit only the files listed below. **Do not read whole files**: read the named lines only. The code blocks are code to paste.
4. Tests are unit tests. Each must fail when its edit is reverted; name the line in the report.
5. `E/storage/install_worker/mod.rs` is past the soft cap: add at most 10 lines.
6. Keep each file's existing formatting style. Run `fmt` for the two crates and keep what it produces.
7. Worknote: append a "PP-20b" section, CRLF. Commit with explicit paths only.

Files: `E/storage/install_worker/mod.rs` (`watch_phases` 261-265),
`E/archive_worker/process/mod.rs` (line 115), `S/host/install/contract.rs`
(254-274 and its `mod tests`), `S/host/adoption/rejection_dto.rs` (48-64
and its `mod tests`), `S/host/adoption/status.rs` (46, 59-65, 217-227),
`F/install-workflow.ts` (line 10), `F/game-update-workflow.ts` (line 6, the
`progress:` struct only), `F/contract.ts` (line 232), `F/install-workflow.test.ts`
(one test), and the worknote.

1. `watch_phases` (261-265): replace the two `Some(..)` arms with:
   ```rust
            Some(
                install::Progress::Downloading { .. }
                | install::Progress::Copying { .. }
                | install::Progress::Checking { .. },
            ) => TimedPhase::Download,
            Some(install::Progress::Extracting { .. } | install::Progress::Expanding { .. }) => {
                TimedPhase::Extraction
            }
   ```
   No schema change (C-7).
2. `E/archive_worker/process/mod.rs:115`: change the pattern to
   `if let Some(Progress::Extracting {current,total,..} | Progress::Expanding {current,total,..})=latest {`.
3. `S/host/install/contract.rs`: give `JobProgress` (256-259) five variants
   in this order: `Download`, `Copy`, `Check`, `Extraction`, `Expansion`, each
   `{ current: u64, total: u64 }`. In `progress()`, add:
   ```rust
        Progress::Copying { copied, total } => JobProgress::Copy {
            current: (*copied).min(MAX),
            total: (*total).min(MAX),
        },
        Progress::Checking { read, total } => JobProgress::Check {
            current: (*read).min(MAX),
            total: (*total).min(MAX),
        },
        Progress::Expanding { current, total, .. } => JobProgress::Expansion {
            current: (*current as u64).min(MAX),
            total: (*total as u64).min(MAX),
        },
   ```
   Test, in its `mod tests`:
   ```rust
    #[test]
    fn new_install_phases_serialise_flat_without_paths() {
        use super::{progress, Progress};
        let json = |event: &Progress| serde_json::to_string(&progress(event)).unwrap();
        let copy = Progress::Copying { copied: 3, total: 9 };
        assert_eq!(json(&copy), r#"{"phase":"copy","current":3,"total":9}"#);
        let check = Progress::Checking { read: 4, total: 0 };
        assert_eq!(json(&check), r#"{"phase":"check","current":4,"total":0}"#);
        let expand = Progress::Expanding {
            label: "private".into(),
            current: 1,
            total: 2,
            filename: "/private/file".into(),
        };
        assert_eq!(json(&expand), r#"{"phase":"expansion","current":1,"total":2}"#);
    }
   ```
   `S/host/install/tests.rs` `progress_projection_excludes_paths_and_caps_javascript_integers`
   must pass untouched: `extraction` keeps its exact JSON.
4. `S/host/adoption/rejection_dto.rs` `progress_item` (48-64): change
   `Observed::Extracting { label, .. } =>` to
   `Observed::Extracting { label, .. } | Observed::Expanding { label, .. } =>`, and
   add the arm `Observed::Copying { .. } | Observed::Checking { .. } => None,`.
   After `progress_item`, add:
   ```rust
/// Whether adoption shows this event. The RAR index pass (`Checking` with no
/// total) counts entries, not bytes, so it is not shown.
pub(super) fn progress_shown(observed: &Observed) -> bool {
    !matches!(observed, Observed::Checking { total: 0, .. })
}
   ```
   Test, in its `mod tests` (it already has `use super::*;`):
   ```rust
    #[test]
    fn the_rar_index_pass_is_hidden_but_a_hash_is_shown() {
        assert!(!progress_shown(&Observed::Checking { read: 256, total: 0 }));
        assert!(progress_shown(&Observed::Checking { read: 1, total: 2 }));
        assert!(progress_shown(&Observed::Copying { copied: 1, total: 2 }));
    }
   ```
5. `S/host/adoption/status.rs`:
   - In both `.as_ref().map(|observed| {` chains (46 and 59), insert
     `.filter(|observed| super::rejection_dto::progress_shown(observed))` between `.as_ref()` and `.map(`.
   - Phase match (62-65): replace it with
     `Observed::Downloading { .. } | Observed::Copying { .. } | Observed::Checking { .. } => Phase::Download,`
     and `Observed::Extracting { .. } | Observed::Expanding { .. } => Phase::Extraction,`.
   - In `counts` (217-227), replace the two arms with:
     ```rust
        Observed::Downloading {
            downloaded: current,
            total,
            ..
        }
        | Observed::Copying {
            copied: current,
            total,
        }
        | Observed::Checking {
            read: current,
            total,
        } => ((*current).min(MAX), (*total).min(MAX)),
        Observed::Extracting { current, total, .. }
        | Observed::Expanding { current, total, .. } => {
            ((*current as u64).min(MAX), (*total as u64).min(MAX))
        }
     ```
6. TS, the same literal list in three schemas:
   - `F/install-workflow.ts:10` and `F/game-update-workflow.ts:6`: change `Schema.Literals(['download','extraction'])` to `Schema.Literals(['download','copy','check','extraction','expansion'])`.
   - `F/contract.ts:232`: change `phase: Schema.Union([Schema.Literal("download"), Schema.Literal("extraction")]),` to `phase: Schema.Literals(["download", "copy", "check", "extraction", "expansion"]),`.
   - Add no view copy here (PP-21 owns it). If `npm run check` reports a non-exhaustive use of `phase`, stop and report the file and line; do not edit it.
   - `F/install-workflow.test.ts`: read lines 60-90 (the test `recovery binds current revision; path-bearing progress is rejected`). Next to it, add one test in the same style: a running status whose progress is `{phase:'check',current:1,total:0}` is accepted, and one with `{phase:'expansion',current:1,total:2}` is accepted. Revert check: narrow line 10 back.

Lane: engine `fmt`, `clippy --all-targets -- -D warnings`, `test --lib install`,
`test --lib archive_worker`; shell `fmt`, `clippy --all-targets -- -D warnings`,
`test --lib install`, `test --lib adoption`, `test --lib game_update`; then core
(R10) `clippy --all-targets -- -D warnings` once more, and R2. Paste each
status line. Then tell the orchestrator that the branch is ready for `ops` to
merge (PP-20 slot). Docs: none (PP-13 row).

### PP-21: Install view copy, phase lines, rate and ETA

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-20 merged. **Branch:** `pp-21`.
**Commit:** `feat(launcher): the install view says what it is doing, with a rate and ETA (PP-21)`.

Files: `F/install-view.ts` (`progressLine` 222-238, the label choice near
108 and the phase reset near 43 and 90-92), `F/transfer-rate.ts` only if a helper is missing, the
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
| `extraction` | `Unpacking the archive… N% · N files/s · ETA` |
| `expansion` | `Expanding game files… N% · N files/s · ETA` |

Rate and ETA use `makeRateTracker` (`F/transfer-rate.ts` 62-97) for every
phase, reset on any change of `phase` (C-7 has no `stage`), shown once the tracker has
a rate (its 1 s minimum span). Byte phases show MB/s; extraction shows
files/s. Drop the "at least 100 MiB" condition for `copy` and `check` only.

Tests: one per table row (the line for a sample event), and the tracker
resets when `phase` changes from `extraction` to `expansion`. UAT script: feed two scripted event sequences,
the local archive copy → check(total 0) → extraction → expansion,
and the download download → check(total > 0) → check(total 0) →
extraction → expansion, and print one
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
   fields `phase` (the C-7 wire name: `download`, `copy`, `check`,
   `extraction`, `expansion`), `bytes` (last `current` of a byte phase, else
   0), `files` (last `current` of `extraction` or `expansion`, else 0),
   `elapsed_ms`. `watch_phases` emits it when the C-7 phase changes (track
   it beside `TimedPhase`, which merges copy, check and download) and when
   the job ends. A `check` with
   `total == 0` (the RAR index pass) is not a byte phase: `bytes` is 0. After
   D-PP26 a local-archive install logs `copy`, then `check` with `bytes=0`,
   then `extraction`, then `expansion`; a `check` with bytes there is a regression.
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
