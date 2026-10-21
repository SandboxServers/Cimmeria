# Launcher adopt UX: work packets

Ledger: [README.md](README.md). Resume card: [RESUME.md](RESUME.md). Designed 2026-10-20 at `0d71b3b30`.

## Dispatch rules

- One `packet-coder` per packet, each in its own worktree. `ops` creates each worktree with `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`; worktrees live at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<name>`. No packet needs a database.
- `mk-worktree.ps1` always branches off `origin/main`. For a packet with a dependency, once the dependency is merged into the integration branch, `ops` runs `pwsh tools/build-lane/merge-noff.ps1 -C <wt> -Branch launcher-adopt-ux/integration` in that packet's worktree **before** the coder starts.
- **The wave ships as one PR** from the integration worktree `au-integration` (branch `launcher-adopt-ux/integration`). `ops` merges each packet branch into it with `merge-noff.ps1`, in the order in § Waves. A packet merged into integration has no PR of its own; after the wave PR merges, `ops` retires `au-01`..`au-06` with `rm-worktree.ps1 --merged-into <PR> au-01 au-02 au-03 au-04 au-05 au-06`.
- Report path: `<worktree>\.claude\reports\launcher-adopt-ux\<packet>-<role>.md`, where role is `coder`, `review-<N>` or `fix-<N>`.
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit messages end with these two lines:

  ```text
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
  ```

- No lab, live client or live server is used by any packet. Every check below runs on fixtures in temp folders.
- Order: packet loop for each packet, then the wave loop on `au-integration`, then the integration checks in § Waves, then the PR.

### Standard checks (each packet lists which it runs, in order)

Every command starts with `Set-Location <worktree>;` in the same PowerShell call. Run each in the foreground and wait for it. Read the lane's failures file instead of rerunning to see output.

| ID | Command |
|---|---|
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings` |
| D3e | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine storage::adoption` |
| D3s | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop host::adoption` |
| F1 | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend` |
| F2 | The in-place logic UAT, two calls. (1) `Set-Location <wt>; $env:LANE_VERBOSE='1'; pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --no-run`. Take the path from the line `Executable unittests src/main.rs (<path>)`; if it is relative, prefix the worktree path. (2) `Set-Location <wt>; $env:ADOPTION_UAT_BINARY='<path>'; npm run uat:adoption --prefix crates/launcher/desktop/frontend`. It must print `In-place adoption logic UAT passed`. |

---

## Contract

Every packet builds against these names exactly. Do not rename them. Paths are under `crates/launcher/desktop/`.

### C1. Engine report types (AU-01): `engine/src/storage/adoption/model.rs`

Add after the `Error` enum's `From` impls (after line 200):

```rust
/// Why one entry of a refused folder cannot be adopted (launcher-adopt-ux D-AU4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsafeReason {
    /// A symbolic link, or on Windows a junction.
    Link,
    /// A OneDrive or other cloud-files placeholder (Windows reparse tag).
    CloudFile,
    /// A file with more than one name (hard link count above 1).
    HardLink,
    /// Not a plain file or folder: another reparse point, a socket, a device.
    SpecialFile,
    /// A name the file index cannot hold: not ASCII, over 4096 bytes, or with `:` or `\`.
    Name,
    /// Another entry has the same name apart from letter case.
    CaseCollision,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsafeEntry {
    pub reason: UnsafeReason,
    /// Relative to the chosen folder, `/`-separated, at most 200 chars plus `…`.
    /// `"."` is the folder itself; `None` means the folder's own location is not
    /// its canonical path. Never absolute.
    pub path: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsafeReport {
    /// At most `MAX_UNSAFE_SHOWN` (5), in walk order (children sorted by name).
    pub entries: Vec<UnsafeEntry>,
    /// Every offender the survey found, listed or not.
    pub total: u32,
}
```

In the `Error` enum (lines 148-185): change the derive to `#[derive(Debug, Clone, PartialEq, Eq)]` (drop `Copy`) and add, after `UnsupportedMode,`:

```rust
    /// The folder holds entries that cannot be adopted safely; the report names
    /// up to five of them by relative path.
    Unsafe(Box<UnsafeReport>),
```

### C2. Inventory helpers (AU-01): `engine/src/storage/adoption/inventory.rs`

```rust
pub(super) const MAX_FILES: usize = 200_000;   // was private
pub(super) const MAX_DEPTH: usize = 64;        // was private
/// The scan's rule for an index name (one place, used by the scan and the survey).
pub(super) fn index_name_allowed(name: &str) -> bool;
/// Why `path` itself cannot be scanned, or `None` when it can. Read only: a
/// Windows handle is opened with FILE_READ_ATTRIBUTES and
/// FILE_FLAG_OPEN_REPARSE_POINT, so a cloud file is never recalled.
pub(super) fn unsafe_reason(path: &Path) -> Option<UnsafeReason>;
/// The reason a Windows reparse tag stands for (D-AU6). Pure, tested on every host.
pub(super) fn reparse_reason(tag: u32) -> UnsafeReason;
```

### C3. Survey (AU-01): new `engine/src/storage/adoption/survey.rs`

```rust
pub(super) const MAX_UNSAFE_SHOWN: usize = 5;
/// The entries of `root` that cannot be adopted safely. Read only and bounded.
pub(super) fn survey(root: &Path) -> UnsafeReport;
/// The error for a scan of `root` that failed with a bare `UnsafeFile`:
/// `Error::Unsafe(report)`, or the bare `UnsafeFile` again when the survey finds nothing.
pub(super) fn explain(root: &Path) -> Error;
/// `result` with a bare `Err(Error::Storage(StorageError::UnsafeFile))` replaced by `explain(root)`.
pub(super) fn explained<T>(root: &Path, result: Result<T, Error>) -> Result<T, Error>;
```

Log lines (target `adoption`, `tracing::warn!`), never with an absolute path:

| When | Fields | Message |
|---|---|---|
| each offender, first 100 | `reason = ?reason`, `path = %shown` (`"(the folder's location)"` for `None`) | `adoption refused: an entry cannot be adopted safely` |
| over 100 offenders, once | `total` | `adoption refused: more unsafe entries were not logged` |
| survey found nothing | none | `adoption refused: an entry changed or could not be checked` |

### C4. Shell status field (AU-03)

`shell/src/host/adoption/contract.rs`, `AdoptionStatus`, after `last_error`:

```rust
    /// The entries a `unsafe_file` refusal names (launcher-adopt-ux D-AU5);
    /// `None` for every other state. Paths are relative to the chosen folder.
    pub unsafe_entries: Option<adoption::UnsafeReport>,
```

Wire shape: `"unsafe_entries":{"entries":[{"reason":"hard_link","path":"later.txt"}],"total":2}` or `"unsafe_entries":null`. `last_error` stays `"unsafe_file"` whenever `unsafe_entries` is not null.

`From<adoption::Error> for AdoptionError` gains the arm `adoption::Error::Unsafe(_) => Self::UnsafeFile,` (AU-01 adds it so the shell compiles).

### C5. Frontend schema (AU-03): `frontend/src/adoption-workflow.ts`

```ts
const Unsafe=Schema.Struct({entries:Schema.Array(Schema.Struct({reason:Schema.Literals(['link','cloud_file','hard_link','special_file','name','case_collision']),path:Schema.NullOr(Schema.String)})),total:Count});
```

`AdoptionStatus` ends `…,last_error:Schema.NullOr(Code),unsafe_entries:Schema.NullOr(Unsafe)});`

### C6. UI ids and texts (AU-04, AU-05)

| Id or text | Owner | Value |
|---|---|---|
| refusal list | AU-04 | `<ul id="adoption-unsafe" class="caption" hidden aria-label="Entries that cannot be adopted safely"></ul>` |
| mode rows | AU-05 | `label.mode-option#adoption-mode-in-place-row`, `label.mode-option#adoption-mode-copy-row`; the radios keep ids `adoption-mode-in-place`, `adoption-mode-copy` |
| choose label, two modes, in place picked | AU-05 | `Choose the folder to adopt in place…` |
| choose label, two modes, copy picked | AU-05 | `Choose the folder to copy…` |
| choose label, otherwise | AU-05 | `Use an existing game folder…` (unchanged) |

### C7. UAT bridge commands (AU-03): `shell/src/host/adoption/uat_bridge_in_place.rs`

| Command | Does | Reply |
|---|---|---|
| `plant_hard_link` | `fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt"))` | `{}` |
| `remove_hard_link` | `fs::remove_file(folder.join("later-link.txt"))` | `{}` |

The survey sorts children by name, so the two entries come back as `later-link.txt` then `later.txt` (`-` sorts before `.`).

---

## Waves

| Wave | Packets | Ships as | Merge order into `launcher-adopt-ux/integration` |
|---|---|---|---|
| 1 | AU-01..AU-06 | one PR from `au-integration` | AU-01 → AU-05 → AU-06 → AU-02 → AU-03 → AU-04 |
| 2 | AU-07..AU-10 (launcher telemetry) | one PR from `au-tel-integration` (`launcher-adopt-ux/telemetry`) | AU-07 → AU-10 → AU-08 → sync with `main` after the wave-1 PR → AU-09; see [work-packets-wave2.md](work-packets-wave2.md) |

- AU-01, AU-05 and AU-06 start at once.
- AU-02 starts when AU-01 is integrated.
- AU-03 starts when AU-02 is integrated.
- AU-04 starts when AU-03 and AU-05 are integrated (it edits the same frontend files as both).
- AU-06 describes the behaviour and texts in this file, so it does not wait for code; the wave review checks it against the merged code.

**Integration checks**, in `au-integration` after AU-04 is merged and the wave review is `ship`: D1, then the full suites without a filter (`… -p cimmeria-launcher-engine` and `… -p cimmeria-launcher-desktop`), then D2e, D2s, F1 and F2. `ops` then ships with `ship.ps1 pr -C <au-integration> -m <msg>`. The PR body lists the doc-update-map rows from AU-06, what was tested, and the owner's packaged visual UAT (README § Packets).

---

## Wave 1

### AU-01: engine `UnsafeReport` types and the read-only survey

Host: windows · Branch `launcher-adopt-ux/AU-01-unsafe-survey` · Worktree `au-01` · Crates `cimmeria-launcher-engine`, `cimmeria-launcher-desktop` (one match arm) · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-01;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` (the commands in § Standard checks), in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. Every file you touch must end under 500; report the line counts of the files you touched.
5. Edit only the files listed below, plus any engine file that stops compiling because `Error` lost `Copy` (fix those with `.clone()` or a borrow, nothing else). If dropping `Copy` breaks more than about 15 sites, stop and return `blocked` with the list.
6. Tests are unit tests in a `#[cfg(test)]` module. Each must fail if the code it pins is reverted.
7. This packet only adds the survey. No scan site calls it yet (AU-02 does); do not wire it into `source.rs`.

**Contract.** C1, C2 and C3 above, verbatim.

**Do.**

1. `engine/src/storage/adoption/model.rs`: C1. `Serialize` and `Deserialize` are already in scope there (`FolderSource` derives them).
2. `engine/src/storage/adoption/inventory.rs`:
   - Lines 8-9: make `MAX_FILES` and `MAX_DEPTH` `pub(super)`.
   - Add `index_name_allowed` and use it in `scan` at line 210, so the rule lives in one place:

     ```rust
     pub(super) fn index_name_allowed(name: &str) -> bool {
         name.is_ascii() && name.len() <= 4096 && !name.contains(':') && !name.contains('\\')
     }
     // scan, line 210:
     if !index_name_allowed(&name) {
         return Err(StorageError::UnsafeFile.into());
     }
     ```

   - Add `reparse_reason` (pure, every host):

     ```rust
     #[cfg_attr(not(windows), allow(dead_code))]
     pub(super) fn reparse_reason(tag: u32) -> UnsafeReason {
         const MOUNT_POINT: u32 = 0xA000_0003;
         const SYMLINK: u32 = 0xA000_000C;
         // IO_REPARSE_TAG_CLOUD .. CLOUD_F differ only in bits 12-15.
         const CLOUD_MASK: u32 = 0xFFFF_0FFF;
         const CLOUD: u32 = 0x9000_001A;
         const ONEDRIVE: u32 = 0x8000_0021;
         match tag {
             MOUNT_POINT | SYMLINK => UnsafeReason::Link,
             tag if tag & CLOUD_MASK == CLOUD || tag == ONEDRIVE => UnsafeReason::CloudFile,
             _ => UnsafeReason::SpecialFile,
         }
     }
     ```

   - Add the Windows tag reader, next to `file_identity`. All names are in `windows_sys::Win32::Storage::FileSystem`, a feature the engine already enables. If a name does not resolve, grep the windows-sys 0.61 source under `$env:USERPROFILE\.cargo\registry\src` for it; do not add features.

     ```rust
     #[cfg(windows)]
     fn reparse_tag(path: &Path) -> Option<u32> {
         use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
         use windows_sys::Win32::Storage::FileSystem::{
             FileAttributeTagInfo, GetFileInformationByHandleEx, FILE_ATTRIBUTE_TAG_INFO,
             FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
             FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
         };
         let file = OpenOptions::new()
             .access_mode(FILE_READ_ATTRIBUTES)
             .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
             .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
             .open(path)
             .ok()?;
         // SAFETY: plain-old-data out parameter, filled by the call below.
         let mut info: FILE_ATTRIBUTE_TAG_INFO = unsafe { std::mem::zeroed() };
         // SAFETY: the handle is owned by `file` and stays open; the size is `info`'s.
         let ok = unsafe {
             GetFileInformationByHandleEx(
                 file.as_raw_handle(),
                 FileAttributeTagInfo,
                 (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
                 std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
             )
         };
         (ok != 0 && info.FileAttributes & 0x400 != 0).then_some(info.ReparseTag)
     }
     ```

   - Add `unsafe_reason`. It mirrors what `identity` refuses, so it must agree with the scan:

     ```rust
     pub(super) fn unsafe_reason(path: &Path) -> Option<UnsafeReason> {
         let metadata = std::fs::symlink_metadata(path).ok()?;
         #[cfg(windows)]
         {
             use std::os::windows::fs::MetadataExt;
             if metadata.file_type().is_symlink() || metadata.file_attributes() & 0x400 != 0 {
                 return Some(reparse_tag(path).map_or(UnsafeReason::Link, reparse_reason));
             }
         }
         if metadata.file_type().is_symlink() {
             return Some(UnsafeReason::Link);
         }
         if !metadata.is_file() && !metadata.is_dir() {
             return Some(UnsafeReason::SpecialFile);
         }
         #[cfg(unix)]
         {
             use std::os::unix::fs::MetadataExt;
             if metadata.is_file() && metadata.nlink() != 1 {
                 return Some(UnsafeReason::HardLink);
             }
         }
         match identity(path) {
             // On Windows, once reparse points are ruled out, `file_identity` refuses
             // a plain file only for its link count.
             #[cfg(windows)]
             Err(Error::Storage(StorageError::UnsafeFile)) if metadata.is_file() => {
                 Some(UnsafeReason::HardLink)
             }
             Err(Error::Storage(StorageError::UnsafeFile)) => Some(UnsafeReason::SpecialFile),
             _ => None,
         }
     }
     ```

3. Create `engine/src/storage/adoption/survey.rs` (C3). Module doc: `//! Explains a refused scan: which entries of the folder cannot be adopted safely, and why (launcher-adopt-ux D-AU4). Read only and bounded; it runs only after the scan refused the folder, and never descends into an offender.` Shape:

   ```rust
   use super::*;
   use std::collections::BTreeMap;

   pub(super) const MAX_UNSAFE_SHOWN: usize = 5;
   const MAX_UNSAFE_LOGGED: u32 = 100;
   const MAX_PATH_SHOWN: usize = 200;

   struct Survey {
       report: UnsafeReport,
       seen: usize,
       folded: BTreeMap<String, String>,
   }
   impl Survey {
       fn note(&mut self, reason: UnsafeReason, path: Option<String>) {
           if self.report.total < MAX_UNSAFE_LOGGED {
               tracing::warn!(target: "adoption", ?reason,
                   path = %path.as_deref().unwrap_or("(the folder's location)"),
                   "adoption refused: an entry cannot be adopted safely");
           }
           self.report.total = self.report.total.saturating_add(1);
           if self.report.entries.len() < MAX_UNSAFE_SHOWN {
               self.report.entries.push(UnsafeEntry { reason, path });
           }
       }
   }
   pub(super) fn survey(root: &Path) -> UnsafeReport { /* see below */ }
   fn walk(root: &Path, dir: &Path, depth: usize, survey: &mut Survey) { /* see below */ }
   /// `relative`, `/`-joined, lossy, cut to MAX_PATH_SHOWN chars plus `…`.
   fn shown(relative: &Path) -> String { /* … */ }
   pub(super) fn explain(root: &Path) -> Error { /* see below */ }
   pub(super) fn explained<T>(root: &Path, result: Result<T, Error>) -> Result<T, Error> {
       match result {
           Err(Error::Storage(StorageError::UnsafeFile)) => Err(explain(root)),
           other => other,
       }
   }
   ```

   - `survey`: if `inventory::unsafe_reason(root)` is `Some(reason)`, note it with path `"."` and do not walk. Otherwise `walk(root, root, 0, …)`. After the walk, if `total > MAX_UNSAFE_LOGGED`, warn once with `total` (C3 table). Return the report.
   - `walk`: return if `depth > inventory::MAX_DEPTH`. Read the directory; on a read error return (skip it). Collect the child paths, skipping entries that fail, and **sort** them. For each child:
     1. `seen += 1`; return once `seen > inventory::MAX_FILES`.
     2. `relative = child.strip_prefix(root)`; skip the child if that fails.
     3. If `inventory::unsafe_reason(&child)` is `Some(reason)`: note it, `continue` (never descend).
     4. The index name: every component of `relative` through `to_str()`, joined with `/`. If any component is not UTF-8, or `!inventory::index_name_allowed(&name)`: note `Name`, `continue`.
     5. If `folded.insert(name.to_ascii_lowercase(), name.clone()).is_some()`: note `CaseCollision`, `continue`.
     6. If `std::fs::symlink_metadata(&child)` says it is a directory, `walk(root, &child, depth + 1, survey)`.
     Each note uses `Some(shown(relative))`.
   - `explain`: `let report = survey(root);` if `report.total == 0`, warn `adoption refused: an entry changed or could not be checked` and return `StorageError::UnsafeFile.into()`; otherwise return `Error::Unsafe(Box::new(report))`.
4. `engine/src/storage/adoption/mod.rs`: add `mod survey;` after `mod source;` (line 27), and `#[cfg(test)] mod survey_tests;` in the test-module block, after `mod replay_tests;` (line 53).
5. `shell/src/host/adoption/contract.rs`: in `From<adoption::Error> for AdoptionError` (lines 156-182), add `adoption::Error::Unsafe(_) => Self::UnsafeFile,` after the `UnsupportedMode` arm. Nothing else in the shell.

**Tests**, in the new `engine/src/storage/adoption/survey_tests.rs` (`use super::*;`, plus `use super::survey::*;` and `super::inventory`). Build folders in `tempfile::tempdir()`; canonicalize the temp path before surveying.

- `reparse_tags_map_to_reasons` (every host): `0xA000_0003` and `0xA000_000C` are `Link`; `0x9000_001A`, `0x9000_101A`, `0x9000_701A`, `0x9000_F01A` and `0x8000_0021` are `CloudFile`; `0x8000_0017` (WOF compression) and `0x8000_001B` (app alias) are `SpecialFile`. Fails if a tag arm is removed.
- `a_clean_folder_has_nothing_to_explain`: files `a.txt` and `sub/b.txt`. `survey` gives `total == 0` and no entries, and `explain` returns `Error::Storage(StorageError::UnsafeFile)`.
- `hard_links_are_named_by_relative_path`: `a.txt` plain, `later.txt` hard-linked to `other.txt` (`std::fs::hard_link`). Expect entries `[HardLink Some("later.txt"), HardLink Some("other.txt")]`, total 2, and `explain` is `Error::Unsafe(_)` with that report. Fails if `unsafe_reason` stops detecting hard links, or if the path is not relative.
- `the_report_lists_five_and_counts_all`: seven hard-link pairs (`f0.txt`/`g0.txt` … `f6.txt`/`g6.txt`). Expect 5 entries, total 14.
- `#[cfg(not(target_os = "macos"))] an_offender_is_not_descended`: a directory named `é` (not ASCII) holding a hard-link pair. Expect exactly `[Name Some("é")]`, total 1. The pair inside is not reported, because the survey never descends into an offender; the test fails if `walk` recurses before checking the name.
- `#[cfg(windows)] a_junction_is_a_link`: make `root/Working` a junction to a second temp dir that holds a hard-link pair, with the pattern from `in_place_replace_tests.rs` lines 342-349 (`cmd /C mklink /J <link> <target>`; return early if it fails). Expect `[Link Some("Working")]`, total 1. Then survey the junction path itself (not canonicalized): expect `[Link Some(".")]`, total 1.
- `#[cfg(unix)] a_symlinked_folder_is_a_link`: the same with `std::os::unix::fs::symlink`.
- `#[cfg(target_os = "linux")] sockets_and_case_collisions`: a `std::os::unix::net::UnixListener` bound at `root/sock` gives `SpecialFile "sock"`; files `README.txt` and `Readme.txt` give `CaseCollision "Readme.txt"` (sorted, `E` before `e`). Expect both, total 2.

**Checks, in order.** D1, D2e, D2s (proves the shell's exhaustive match), D3e. Report the test count of D3e and the line counts of `model.rs`, `inventory.rs`, `survey.rs`, `survey_tests.rs`.

**Docs.** None (AU-06 owns the docs).

**Commit.** `AU-01: engine unsafe-entry survey and report types (launcher-adopt-ux)`

---

### AU-02: wire the survey into the source checks; relative-only logs

Host: windows · Branch `launcher-adopt-ux/AU-02-source-wiring` · Worktree `au-02` · Crate `cimmeria-launcher-engine` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/integration` merged in by `ops` (holds AU-01)

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-02;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` (§ Standard checks), in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. `source.rs` is 470 lines and must stay under 500: new tests go in the new file below, not in `source.rs`. Do not edit `in_place.rs` (669 lines).
5. No adoption log line may carry an absolute path or a user name (D-AU7): log relative paths, `?error` or nothing.
6. Edit only the files listed below.
7. Tests must fail when the wiring is reverted.

**Contract.** C1 and C3 (from AU-01, already in your tree). Use `survey::explained(root, result)`; do not copy its logic.

**Do.**

1. `engine/src/storage/adoption/source.rs`, `request_source` (lines 30-65):
   - line 42: `let scanned = survey::explained(&imported.source.game_directory, inventory::scan(&imported.source.game_directory, &CancellationToken::new()))?;`
   - line 47: `survey::explained(&root, inventory::scan(&root, &CancellationToken::new()))?`
   - line 52: `survey::explained(&root, refuse_links(&root))?;`
   - line 58: as line 47.
2. `source.rs`, `refuse_links` (lines 128-144): delete the warn at line 139 (the survey now logs the offender by relative path). Keep the `return Err(StorageError::UnsafeFile.into());`.
3. `source.rs`, the other absolute-path logs: line 89 drop `folder = %selected.display(),`; lines 164 and 182 drop `folder = %root.display(),`. Keep `?error` at 182 and every message.
4. `source.rs`, `canonical_paths` (lines 220-227): replace the bare refusal with

   ```rust
   if path.canonicalize()? != path {
       tracing::warn!(target: "adoption", reason = ?UnsafeReason::Link, "adoption refused: a source folder is not at its canonical path");
       return Err(Error::Unsafe(Box::new(UnsafeReport {
           entries: vec![UnsafeEntry { reason: UnsafeReason::Link, path: None }],
           total: 1,
       })));
   }
   ```

5. `engine/src/storage/adoption/in_place_preview.rs` line 97: `survey::explained(&folder, inventory::scan(&folder, &cancel))?`.
6. `engine/src/storage/adoption/tests.rs`, update the two tests that pin the bare refusal:
   - line 432-435 (unix symlink `link`): `let Err(Error::Unsafe(report)) = f.preview() else { panic!("expected an unsafe report") };` then assert entries `[UnsafeEntry { reason: UnsafeReason::Link, path: Some("link".into()) }]` and `total == 1`.
   - lines 447-450 (hard link `later.txt` → `other.txt`): the same pattern, entries `[HardLink "later.txt", HardLink "other.txt"]`, total 2.
   If either still shows a bare `Storage(UnsafeFile)`, the refusal came from a scan site this packet did not wrap: return `blocked` naming the site; do not wrap more sites.
7. Create `engine/src/storage/adoption/unsafe_source_tests.rs`; declare it in `mod.rs` beside `in_place_tests` as `#[cfg(all(test, not(target_os = "macos")))] mod unsafe_source_tests;`. Module doc: `//! Source checks name the entries they refuse (launcher-adopt-ux AU-02).` It uses `super::in_place_tests::Fixture` (`Fixture::new(None)`, `f.state`) and its own `layout(root)` helper that writes `Working/Binaries/SGW.exe` (`b"MZ"` is enough: nothing parses it before the refusal).

**Tests** (in `unsafe_source_tests.rs`):

- `#[cfg(windows)] a_junctioned_working_folder_is_named`: `target` = temp dir with `layout`-style `Binaries/SGW.exe`; `other` = temp dir; `other/Working` = junction to `target` (mklink pattern, return early if it fails). For each mode in `[AdoptionMode::InPlace, AdoptionMode::Copy]`: `request_source(&f.state, &SourceRequest::Folder { selected: other.path().to_path_buf() }, mode)` is `Err(Error::Unsafe(report))` with entries `[Link Some("Working")]` and total 1. After both, `target` holds no `.cimmeria-write-probe-*` entry. Fails if line 52 or line 58 is not wrapped.
- `a_hard_link_in_a_picked_folder_is_named` (Windows and Linux): `picked` = temp dir with `layout`, plus `notes.txt` hard-linked to `notes-copy.txt`. `request_source(…, AdoptionMode::Copy)` gives `[HardLink "notes-copy.txt", HardLink "notes.txt"]`, total 2.
- `#[cfg(windows)] a_source_folder_through_a_junction_is_not_canonical`: `link` = junction to a temp dir. `source::canonical_paths(&AdoptionSource::Folder(FolderSource { selected: link.clone(), root: link.clone(), scan_digest: [0; 32] }))` is `Err(Error::Unsafe(r))` with entries `[Link None]`, total 1.

**Checks, in order.** D1, D2e, D2s, D3e. Report D3e's test count, and grep `source.rs` for `display()` inside `tracing::` calls (expect none).

**Docs.** None (AU-06).

**Commit.** `AU-02: name refused entries at every source scan; relative-only adoption logs (launcher-adopt-ux)`

---

### AU-03: shell `unsafe_entries` status field, schema field, UAT bridge commands

Host: windows · Branch `launcher-adopt-ux/AU-03-status-field` · Worktree `au-03` · Crate `cimmeria-launcher-desktop`, plus `frontend/` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/integration` merged in (holds AU-01, AU-02)

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-03;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` (§ Standard checks), in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. `shell/src/host/adoption/mod.rs` is 493 lines: replace lines, do not add more than the one struct field there; the helpers go in `status.rs`.
5. The frontend decodes status with `onExcessProperty:'error'`: the Rust field and the TS schema field land in **this one commit**, or every status read fails as `schema`.
6. A shell contract change needs a test that fails when reverted (the shell test below), and the frontend fixture must carry the new field.
7. Edit only the files listed below.

**Contract.** C4, C5, C7.

**Do.**

1. `shell/src/host/adoption/mod.rs`, `struct Adoption` (lines 72-80): add `last_unsafe: Option<adoption::UnsafeReport>,` after `last_error`. Replace each `adoption.last_error = None;` (lines 268, 325, 465, 487) with `adoption.clear_failure();`.
2. `shell/src/host/adoption/status.rs`: add

   ```rust
   impl Adoption {
       /// No failure is shown.
       pub(super) fn clear_failure(&mut self) {
           self.last_error = None;
           self.last_unsafe = None;
       }
       /// `error` is shown; an unsafe refusal keeps its named entries.
       pub(super) fn fail(&mut self, error: adoption::Error) {
           self.last_unsafe = match &error {
               adoption::Error::Unsafe(report) => Some((**report).clone()),
               _ => None,
           };
           self.last_error = Some(error.into());
       }
   }
   ```

   In `settle_adoption`: line 126 becomes `adoption.clear_failure();`; line 128 `Err(error) => adoption.fail(error),`; line 137 `match outcome.err() { Some(error) => adoption.fail(error), None => adoption.clear_failure() }`. In `adoption_status`, after `last_error: adoption.last_error,` (line 106) add `unsafe_entries: adoption.last_unsafe.clone(),`.
3. `shell/src/host/adoption/contract.rs`, `AdoptionStatus` (lines 244-263): add the C4 field after `last_error`.
4. `shell/src/host/adoption/uat_bridge_in_place.rs` (lines 64-86): add the two C7 arms before the default arm, for example `"plant_hard_link" => { fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt")).unwrap(); Ok(serde_json::json!({})) }`.
5. `frontend/src/adoption-workflow.ts`: C5 (the `Unsafe` const after `Review`, line 15; the field at the end of `AdoptionStatus`, line 24).
6. `frontend/src/adoption-view.test.ts` line 10: the `idle()` fixture gains `unsafe_entries:null` after `last_error:null`.

**Tests.**

- Shell, in `shell/src/host/adoption/in_place_tests.rs`, `a_hard_linked_file_is_named_and_dismissed` (`#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`), in the style of the file's first test:
  1. `let mut f = Fixture::new(serde_json::json!({})); let _origin = f.serve().await; let folder = f.source.game_directory.clone();`
  2. `std::fs::hard_link(folder.join("later.txt"), folder.join("later-link.txt")).unwrap();`, then `f.host.begin_adoption_in_place(f.release()).unwrap();`.
  3. `let status = until(&f.host, "the unsafe refusal", Duration::from_secs(30), |s| s.activity == Activity::Idle && s.last_error.is_some()).await;`. Assert `last_error == Some(AdoptionError::UnsafeFile)` and `unsafe_entries` equals entries `[HardLink "later-link.txt", HardLink "later.txt"]`, total 2.
  4. Assert nothing changed: no `.cimmeria-install.json`, no `game` folder, `later.txt` still reads `edited by the user`.
  5. Remove `later-link.txt`, send the dismiss command (the file's `DISMISS` pattern or `AdoptionCommand::Dismiss { schema_version: 1 }`), and assert `last_error` and `unsafe_entries` are both `None`.
  6. Begin again; `reviewed(&f.host).await` returns a review.
  Fails if `fail` drops the report, if `clear_failure` keeps it, or if the field is removed.
- Shell wire shape, same file: `unsafe_entries_serialize_as_the_contract_says` — `serde_json::to_value(adoption::UnsafeReport { entries: vec![adoption::UnsafeEntry { reason: adoption::UnsafeReason::HardLink, path: Some("later.txt".into()) }, adoption::UnsafeEntry { reason: adoption::UnsafeReason::Link, path: None }], total: 2 })` equals `json!({"entries":[{"reason":"hard_link","path":"later.txt"},{"reason":"link","path":null}],"total":2})`. Fails if the serde names drift from C5.
- Frontend: the `idle()` fixture with `unsafe_entries:null` is the guard. With the schema field removed, every adoption-view test fails on `schema`.

**Checks, in order.** D1, D2s, D3s, F1, F2 (the UAT must still pass unchanged: it proves the real host's status decodes with the new field). Report D3s's test count and the line counts of `mod.rs` and `status.rs`.

**Docs.** None (AU-06).

**Commit.** `AU-03: adoption status carries the unsafe entries (launcher-adopt-ux)`

---

### AU-04: frontend refusal list and texts, UAT scenario 0

Host: windows · Branch `launcher-adopt-ux/AU-04-refusal-list` · Worktree `au-04` · Path `crates/launcher/desktop/frontend/` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/integration` merged in (holds AU-01, AU-02, AU-03, AU-05)

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-04;`.
2. Never run `cargo` directly; F2's build step goes through `pwsh tools/build-lane/lane.ps1`. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700.
5. Paths and names come from the player's disk: render them with `textContent` only, never `innerHTML` (the pattern at `adoption-view.ts` 147-151).
6. Every meaningful frontend change needs the REPL-style logic UAT (F2) on top of the unit tests, and each unit test must fail when its change is reverted.
7. Edit only the files listed below.

**Contract.** C5 (in your tree), C6, C7.

**Do.**

1. `frontend/ui/index.html` line 36: after the `<progress id="adoption-progress" …></progress>`, add the C6 `<ul id="adoption-unsafe" …>`.
2. `frontend/src/adoption-view.ts`:
   - Replace `unsafe_file` (line 24) with: ``unsafe_file:`A file or folder in the game folder could not be checked safely, or it changed while it was checked, so nothing was copied, moved or removed. Close the game and any program that syncs or backs up the folder, then try again. ${unchanged}`,``
   - D-AU9: in `duplicate_release_file` (line 31) delete `The launcher log names the file.`; in `leftover_launcher_files` (line 36) delete `The launcher log says which.`
   - Add, after `failureText` (line 39):

     ```ts
     const unsafeLead='These files or folders in the game folder cannot be adopted safely. Fix each one as described below, then choose the folder again. Nothing was copied, moved or removed.';
     const unsafeReasons:Record<UnsafeReason,string>={
      link:'a link or junction to another place. Put the real folder or file here instead, then try again.',
      cloud_file:'a OneDrive or other cloud-synced file. Move the whole game folder out of the synced folder (for example to C:\\Games), then try again.',
      hard_link:'a hard link: the same file is stored under another name too. Replace it with a plain copy, or remove it, then try again.',
      special_file:'not a plain file or folder. Remove it or move it out of the game folder, then try again.',
      name:'its name has characters the launcher cannot use safely (letters outside plain ASCII, “:” or “\\”), or is too long. Rename it or move it out of the game folder, then try again.',
      case_collision:'another entry has the same name apart from upper and lower case. Remove or rename one of them, then try again.',
     };
     type UnsafeReason=NonNullable<AdoptionStatus['unsafe_entries']>['entries'][number]['reason'];
     /** One line per named entry, then how many more were found. */
     export function unsafeItems(status:AdoptionStatus):string[]{
      const u=status.unsafe_entries;if(!u)return [];
      const items=u.entries.map(e=>`${e.path===null?'The game folder’s location':e.path==='.'?'The game folder itself':e.path}: ${unsafeReasons[e.reason]}`);
      if(u.total>u.entries.length)items.push(`…and ${u.total-u.entries.length} more not listed here.`);
      return items;
     }
     ```

   - `adoptionText` line 64: `if(status.last_error)return status.last_error==='unsafe_file'&&status.unsafe_entries?.entries.length?unsafeLead:failureText(status.last_error);`
   - `render`, after the progress lines (after line 127):

     ```ts
     const unsafe=!busy&&!working&&!review&&!r&&!current.error&&s?.last_error==='unsafe_file'?unsafeItems(s):[];
     const unsafeList=get('adoption-unsafe');
     unsafeList.replaceChildren(...unsafe.map(text=>{const item=document.createElement('li');item.textContent=text;return item;}));
     unsafeList.hidden=!unsafe.length;
     ```

3. `frontend/src/adoption-view.test.ts`:
   - line 214: the regex becomes `/neither copy matches.*recovery stopped\. Nothing was deleted/`.
   - New test `an unsafe refusal lists each entry, its reason and the fix`: status `{...idle(),backend:'in_place',modes:['in_place','copy'],last_error:'unsafe_file',unsafe_entries:{entries:[{reason:'hard_link',path:'later.txt'},{reason:'cloud_file',path:'Working/a.pak'},{reason:'link',path:'.'},{reason:'link',path:null},{reason:'name',path:'<img src=x onerror=alert(1)>'}],total:7}}`. Assert: the status text equals the lead text; `adoption-unsafe` is not hidden and has 6 `li`; the first reads `later.txt: a hard link: the same file is stored under another name too. Replace it with a plain copy, or remove it, then try again.`; the third starts `The game folder itself:`; the fourth starts `The game folder’s location:`; the last is `…and 2 more not listed here.`; `get('adoption-unsafe').querySelector('img')` is `null`. Fails if the list, a text or the escaping is reverted.
   - New test `every unsafe reason has its own fix text`: for each of the six reasons, `unsafeItems` of a one-entry status (`path:'x'`) gives one item; the six texts are distinct and none is empty.
   - New test `an unsafe refusal without entries shows the generic text and no list`: `{...idle(),last_error:'unsafe_file',unsafe_entries:null}`. Text matches `/could not be checked safely, or it changed while it was checked.*Close the game and any program that syncs or backs up the folder/`; the list is hidden.
   - New test `the list is gone once a review is shown`: `{...reviewing(),last_error:'unsafe_file',unsafe_entries:{entries:[{reason:'link',path:'x'}],total:1}}`; the list is hidden.
   - New test `no failure text points at a launcher log`: for `duplicate_release_file` and `leftover_launcher_files`, `assert.doesNotMatch(ui.text(),/log/)`.
4. `frontend/adoption-in-place-uat.mjs`, a new **scenario 0** between the label assertion (line 53, as AU-05 left it) and scenario 1:

   ```js
    // 0. A hard link in the folder: the refusal names both entries and the fix,
    // nothing changes, and once the link is gone scenario 1 runs as before.
    await send({command:'plant_hard_link'});
    await click('choose-adoption-destination');
    let refused=await until('the unsafe refusal',s=>s.last_error==='unsafe_file');
    assert.equal(refused.unsafe_entries.total,2);
    assert.match(text(),/^These files or folders in the game folder cannot be adopted safely/);
    const items=[...get('adoption-unsafe').querySelectorAll('li')].map(li=>li.textContent);
    assert.equal(items.length,2);for(const item of items)assert.match(item,/^later(-link)?\.txt: a hard link/);
    let held=await evidence();assert.equal(held.owner_marker,false);assert.equal(held.executable_in_old_place,true);
    await send({command:'remove_hard_link'});
   ```

   In scenario 1, after the review arrives (line 60), add `assert.equal(get('adoption-unsafe').hidden,true,'the refusal list is gone in review');`. Add to the `Exercised:` line: `a hard link in the folder refused with both entries, their reason and fix named, nothing changed;`.

**Checks, in order.** F1, then F2. Paste F2's `Exercised:` line into the report.

**Docs.** None (AU-06).

**Commit.** `AU-04: adoption refusal lists the unsafe entries and their fixes (launcher-adopt-ux)`

---

### AU-05: mode choice: one row each, selected state, named button

Host: windows · Branch `launcher-adopt-ux/AU-05-mode-choice` · Worktree `au-05` · Path `crates/launcher/desktop/frontend/` · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-05;`.
2. Never run `cargo` directly; F2's build step goes through `pwsh tools/build-lane/lane.ps1`. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Every button press and every choice gets visible feedback on the first press: changing a radio re-renders at once (D-AU8).
5. Keep the read of `box('adoption-mode-copy').checked` in the choose handler (`adoption-view.ts` 182-186): `adoption-native-uat.mjs` line 65 sets `checked` with no event.
6. linkedom does not uncheck the other radio of a group. Derive the selected mode from `adoption-mode-copy` alone, and in tests and the UAT set both radios explicitly.
7. Every meaningful frontend change needs the REPL-style logic UAT (F2) on top of the unit tests; each unit test must fail when its change is reverted.
8. Edit only the files listed below.

**Contract.** C6 (mode rows, choose labels).

**Do.**

1. `frontend/ui/index.html` line 37: replace the fieldset with

   ```html
   <fieldset id="adoption-modes" class="adoption-modes" hidden><legend>How to use the folder</legend>
   <label class="mode-option selected" id="adoption-mode-in-place-row"><input type="radio" name="adoption-mode" id="adoption-mode-in-place" value="in_place" checked><span><strong>Adopt it in place</strong><small>The folder becomes the launcher’s installation. Its game files move into a “game” folder inside it.</small></span></label>
   <label class="mode-option" id="adoption-mode-copy-row"><input type="radio" name="adoption-mode" id="adoption-mode-copy" value="copy"><span><strong>Make a verified copy</strong><small>Your folder is left as it is. The launcher writes a checked copy to a second folder you pick.</small></span></label>
   <p class="caption">Pick one, then press the button below to choose the folder.</p></fieldset>
   ```

2. `frontend/ui/style.css`: append

   ```css

   /* Adoption mode choice: one row per mode, the picked one outlined (launcher-adopt-ux AU-05). */
   .adoption-modes{border:1px solid var(--line);border-radius:9px;padding:10px 12px;margin:0 0 10px}
   .adoption-modes legend{font-size:12px;color:var(--muted);padding:0 4px}
   .mode-option{display:flex;gap:10px;align-items:flex-start;padding:8px 10px;margin:4px 0;border:1px solid transparent;border-radius:7px;cursor:pointer;font-size:12px;line-height:1.5}
   .mode-option small{display:block;color:var(--muted);font-size:11px;margin-top:2px}
   .mode-option.selected{border-color:var(--accent);background:#87d5e714}
   ```

3. `frontend/src/adoption-view.ts`, `render`, replacing line 120:

   ```ts
   const pair=idle&&s?.modes.length===2,copy=box('adoption-mode-copy').checked;
   get('adoption-modes').hidden=!pair;
   get('adoption-mode-in-place-row').className=copy?'mode-option':'mode-option selected';
   get('adoption-mode-copy-row').className=copy?'mode-option selected':'mode-option';
   for(const id of ['adoption-mode-in-place','adoption-mode-copy'])box(id).disabled=!enabled||!idle;
   button('choose-adoption-destination').textContent=pair?(copy?'Choose the folder to copy…':'Choose the folder to adopt in place…'):'Use an existing game folder…';
   ```

   After the consent listeners (line 195): `for(const id of ['adoption-mode-in-place','adoption-mode-copy'])listen(id,'change',()=>render());`
4. `frontend/src/adoption-view.test.ts`:
   - `mount` (lines 18-28): collect the mode the choose call sends. Add `modes:string[]=[]`; in the invoke stub, when `command==='choose_existing_game_folder'`, push `(args as {mode:string}).mode`; return `modes` from `mount`.
   - Replace the test at lines 297-303 with `the mode choice is one row per mode, shows the pick, and the button names it`:
     - `both` (modes `['in_place','copy']`) and `one` (`idle()`), as now: `adoption-modes` shown for `both`, hidden for `one`.
     - `both`: the fieldset text matches `/Adopt it in place.*becomes the launcher’s installation.*Make a verified copy.*left as it is/`; `adoption-mode-in-place-row` has class `selected` and `adoption-mode-copy-row` does not; the choose label is `Choose the folder to adopt in place…`.
     - Untick in place and tick copy (`await both.tick('adoption-mode-in-place',false); await both.tick('adoption-mode-copy');`): at once the copy row is `selected`, the in-place row is not, and the label is `Choose the folder to copy…`.
     - `await both.click('choose-adoption-destination')`: `both.modes` deep-equals `['copy']`.
     - `one`: the label is `Use an existing game folder…`.
   - New test `the mode rows are laid out one per line`: read `../ui/style.css` (as line 7 reads the HTML) and assert it contains `.mode-option{display:flex` and `.mode-option.selected{`. Fails if the CSS that fixes the one-line layout is dropped.
   - Line 164's label assertion stays as it is (that status offers one mode).
5. `frontend/adoption-in-place-uat.mjs`:
   - line 53: expect `Choose the folder to adopt in place…`.
   - after it: tick copy (set `adoption-mode-in-place` unchecked, `adoption-mode-copy` checked, dispatch `change` on copy, `await settle()`); assert `adoption-mode-copy-row` has class `selected` and the label is `Choose the folder to copy…`; then set them back (copy unchecked, in place checked, dispatch `change` on in place, `await settle()`) and assert the label is `Choose the folder to adopt in place…` again. Scenario 1 asserts `args.mode==='in_place'` (line 26), so the reset matters.
   - Add to the `Exercised:` line: `the mode rows show the pick and the choose button names it;`.

**Checks, in order.** F1, then F2. Paste F2's `Exercised:` line into the report.

**Docs.** None (AU-06).

**Commit.** `AU-05: adoption mode rows with a selected state and a named choose button (launcher-adopt-ux)`

---

### AU-06: player guide and migration contract: the refusal, its causes and fixes

Host: windows · Branch `launcher-adopt-ux/AU-06-docs` · Worktree `au-06` · Docs only · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**

1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-06;`.
2. Never `git stash`, `git worktree prune` or push. Commit locally and stop. No build is needed.
3. Every file you edit under `docs/` and `crates/launcher/desktop/docs/` is **CRLF**. After editing, check with `[IO.File]::ReadAllText('<absolute path>')` that no lone `\n` was introduced (`-match "(?<!\r)\n"` must be false); if it is, normalize with `[IO.File]::WriteAllText('<absolute path>', ($text -replace "\r?\n", "`r`n"))`. Use absolute paths.
4. Describe behaviour exactly as this packet states it. The texts below are the shipped UI texts (contract C6 and AU-04); copy them, do not paraphrase.
5. Never hand-edit a `<!-- gen:NAME -->` block. Files: soft cap 500 lines does not apply to these long guides, but add no more than about 60 lines in total.
6. Edit only the files listed below.

**Do.**

1. `docs/client/launcher-guide.md`, § "Use an existing game folder" (lines 173-214):
   - Steps 1-3: the mode choice now comes **before** the folder. Step 1: open **Settings** (⚙); under **Use an existing game folder**, pick a mode (Windows and Linux show both): each mode is its own row with a short description, and the picked row is outlined. Step 2: click the button, which names the pick (**Choose the folder to adopt in place…** or **Choose the folder to copy…**; with one mode it reads **Use an existing game folder…**). Step 3: pick the game folder (root, `Working` or `Binaries`). Keep the two mode descriptions; renumber the rest.
   - Add a subsection `#### If the launcher refuses the folder` after the "Before you adopt in place" list (after line 214). Content: the launcher lists up to five files or folders, each with its path inside the game folder and what to do, plus how many more it found; nothing was copied, moved or removed. Then a table of the six reasons with the fix: a link or junction (put the real folder or file there); a OneDrive or other cloud-synced file (move the whole game folder out of the synced folder, for example to `C:\Games`; "Always keep on this device" is not enough, because the files stay cloud files); a hard link (replace it with a plain copy, or remove it); not a plain file or folder (remove it or move it out; files compressed with `compact /EXE` count here: run `compact /U /EXE /S` in the game folder to decompress them); a name the launcher cannot use (letters outside plain ASCII, `:` or `\`, or too long: rename or move it); two names that differ only in case (remove or rename one). Close with: the check covers the whole game folder, including folders you added such as mods or backups; if the launcher shows the general "could not be checked safely" message without a list, a file changed while it was checked: close the game and any sync or backup program and try again.
   - Troubleshooting, "### Install fails or stops" (lines 772-780): add a bullet `**Use an existing game folder is refused:** the launcher lists the files and what to do; see [If the launcher refuses the folder](#if-the-launcher-refuses-the-folder).` Check the anchor matches the heading you wrote.
2. `crates/launcher/desktop/docs/migration.md`:
   - **Modes** (lines 172-181): one sentence: the two modes are rows with a selected state, and the choose button names the selected mode; the native picker that follows is the confirm step (D-AU8).
   - **Preflight** (lines 183-198): a new bullet for `unsafe_file`: the scan refuses a link, a junction or other reparse point (including OneDrive and other cloud-files placeholders), a hard-linked file, a special file, an index name that is not ASCII, over 4096 bytes or holds `:` or `\`, and two names equal apart from case, anywhere in the folder tree. On a refusal, a read-only survey (`engine/src/storage/adoption/survey.rs`) names up to five offenders in `AdoptionStatus.unsafe_entries` (`reason`, a `/`-separated path relative to the folder, `"."` for the folder itself, `null` when the folder's own location is not canonical) and a `total`. It reads Windows reparse tags without recalling cloud files, never descends into an offender, stops at the scan's 200 000-entry and 64-level bounds, and warn-logs at most 100 offenders under target `adoption` with relative paths only. A survey that finds nothing leaves the generic `unsafe_file` text (a race). Owner questions OQ-1..OQ-3 of `docs/analysis/launcher-adopt-ux/README.md` keep these refusals hard for now; link that README.
   - **Tests and UAT** (lines 311-328): add one sentence: `survey_tests.rs` and `unsafe_source_tests.rs` cover the reasons, the caps and the junction and hard-link refusals; the shell's `in_place_tests.rs` checks `unsafe_entries` end to end; scenario 0 of `adoption-in-place-uat.mjs` plants a hard link and checks the on-screen list.
3. `docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-ledger.md`: append, after the last entry (line 767 onward), `### 2026-10-20: adoption refusals name their entries` with two or three sentences (the refusal list, the mode rows, relative-only logs) and a link to `../../launcher-adopt-ux/README.md`.
4. Walk `docs/agents/doc-update-map.md` row 56 (launcher). Also check `docs/guides/macos.md` lines 135-139 and the desktop `crates/launcher/desktop/README.md` from line 312: expected no change (macOS offers only the copy, and the README already names `npm run uat:adoption`). Say in the report which rows you touched and which you checked without change.

**Test.** None (docs). The wave review checks every statement against the merged code.

**Checks.** The CRLF check in rule 3 for each edited file; every relative link you added resolves to an existing file (`Test-Path` on the joined absolute path) and every `#anchor` matches a heading.

**Commit.** `AU-06: document adoption refusals, their fixes and the mode rows (launcher-adopt-ux)`
