# Work packets: launcher-linux-play-proton (wave 1B, PP-20a)

Continues [work-packets-w1b.md](work-packets-w1b.md): the PP-20a section, moved out to keep both files under the 700-line cap.

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
