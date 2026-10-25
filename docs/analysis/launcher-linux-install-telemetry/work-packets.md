# Work packets: launcher Linux install and https summaries

Ledger: [README.md](README.md). Each packet runs in its own worktree, made by
`ops`. Every packet is `Host: linux`.

## Dispatch rules

Amended 2026-10-24 by ruling LI-02-ruling-1 (copy at
`.claude/reports/launcher-linux-install-telemetry/superseded/LI-02-ruling-1.md`). LI-01..LI-04 are superseded by #2020/#2023
(`unpack::cab_portable`, README F-15), and the integration branch is abandoned
(D-LI10).

- **Wave 1:** LI-05 and LI-06 branch from `main` and ship as **one PR each**.
  LI-06 must not depend on whether LT-41 has landed (D-LI9). The wave-1 loop
  runs once LI-06 has `ship`. Merge order: LI-05 (PR #2025), then LI-06.
- **Wave 2:** LI-09 branches from `main` **after LI-06 merges**, and ships as
  **its own PR** once ship gate G-LI9 passes (D-LI17).
  - LI-05 and LI-09 edit different sections of the same three docs. LI-05
    merges first (wave 1). Otherwise `ops` rebases LI-09; the conflict is
    textual only.
- **Track A (wave A), independent of waves 1 and 2:** worktree `li-07`, branch
  `launcher-linux-install-telemetry/li-07`, cut from `main` by `ops`. LI-08
  commits first, then LI-07 on the same branch. Run one packet-review loop over
  both commits, then ship them as **one PR** through `ship-pr`.
  - LI-07 needs the owner's real client RAR on this Deck, with its path in
    `SGW_CLIENT_RAR`. Dispatch LI-07 only once the orchestrator has that path.
- **Integration branch** `…/integration`: abandoned, and nothing merges into
  it. It was pushed to `origin` (tip `9d61134f7`, the LI-01 merge). Close-out
  deletes it there and locally, with the local `…/li-01`, `…/li-02` and
  `…/li-03`. `ops` cannot delete branches, so hands-on or the owner does it.
- Branches are named `launcher-linux-install-telemetry/li-0N`, and worktrees
  `li-0N`.
- Reviews: one packet-reviewer loop per packet (LI-08 and LI-07 share one),
  plus the wave-1 loop. Wave 2 and wave A are one PR each, so they have no wave
  loop.

### Rules every packet carries (R1-R9)

- **R1. Builds go through the lane.** Never run `cargo check`, `build`, `test`,
  `nextest` or `clippy` directly; a hook refuses them. From the worktree root:
  - root crate: `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-launcher-core --all-targets --features test-fixtures -- -D warnings`, then
    `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-launcher-core --features test-fixtures`
  - desktop workspace: `bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --target-dir target/desktop --all-targets -- -D warnings`, then
    `bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --target-dir target/desktop`
  - `cargo fmt` may run directly: `cargo fmt -p cimmeria-launcher-core`, or
    `cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml`.
  - The order is fmt, then clippy, then tests. A lane failure that is not your
    code (exit 28, disk, lock): stop and report `blocked`.
- **R2. Test type** ([TESTING.md](../../../TESTING.md)): unit tests in a
  `#[cfg(test)] mod tests` in the file, or in a sibling `tests.rs` when the file
  would pass 500 lines. Every behavior change needs a test that **fails when
  the change is reverted**. Say in the commit body which test that is.
- **R3. File caps**: 500 lines soft, 700 hard. Split along the seams this file names.
- **R4. Line endings**: files under `docs/` are CRLF. For any `.md`, keep its
  existing endings (check with `file <path>`), and never rewrite a whole file.
  Rust files are LF.
- **R5. Git**: commit locally with an explicit path list. No `git stash`, no
  push, no rebase, no PR. `ops` does those.
- **R6. Scope**: edit only the files your packet names. If a needed file is not
  listed, stop and report `blocked`, naming it.
- **R7. Telemetry**: forwarded launcher targets (`LAUNCHER_EVENT_TARGETS` in
  `engine/src/storage/game_telemetry/launcher_event.rs`) carry closed codes
  only. They carry no paths, error text or URLs.
- **R8. Do not add or upgrade any crate.** No dependency edits and no
  `external/` changes.
- **R9. Docs**: update only the doc rows your packet lists, and name them in
  the commit body.

## Contract

### MS-CAB facts (reference)

`unpack::cab_portable` on `main` implements these (F-15). Nothing here is to
be built; LI-07 and LI-08 rely on them.

- **CFHEADER** (little-endian):
  - `"MSCF"`, then `u32` reserved1, `u32` cbCabinet, `u32` reserved2,
    `u32` coffFiles, `u32` reserved3, `u8` versionMinor, `u8` versionMajor,
    `u16` cFolders, `u16` cFiles, `u16` flags, `u16` setID, `u16` iCabinet.
  - If flags & 0x4 (RESERVE): `u16` cbCFHeader, `u8` cbCFFolder, `u8` cbCFData,
    then cbCFHeader bytes to skip.
  - If flags & 0x1 (PREV): szCabinetPrev and szDiskPrev, both NUL-terminated.
  - If flags & 0x2 (NEXT): szCabinetNext and szDiskNext, both NUL-terminated.
- **CFFOLDER** (cFolders of them, right after the header):
  - `u32` coffCabStart, `u16` cCFData, `u16` typeCompress, then cbCFFolder
    reserve bytes.
  - The compression type is `typeCompress & 0x000F`: 0 = none, 1 = MSZIP,
    2 = Quantum, 3 = LZX.
- **CFFILE** (cFiles of them, at coffFiles):
  - `u32` cbFile, `u32` uoffFolderStart, `u16` iFolder, `u16` date, `u16` time,
    `u16` attribs, then szName (NUL-terminated).
  - The name is UTF-8 when attribs & 0x80, and Latin-1 otherwise.
  - Names use `\` as the separator.
- **CFDATA** (at coffCabStart, cCFData of them):
  - `u32` csum (ignored, D-LI5), `u16` cbData, `u16` cbUncomp, cbCFData reserve
    bytes, then cbData bytes.
  - cbUncomp must be at most 32768.
- **iFolder special values:**
  - `0xFFFD` CONTINUED_FROM_PREV: the file is in folder 0 of this cabinet, and
    this is a continuation record.
  - `0xFFFE` CONTINUED_TO_NEXT: the file is in the last folder of this
    cabinet, and this is its starting record.
  - `0xFFFF` PREV_AND_NEXT: folder 0, a continuation record.
- **Spanning:**
  - Cabinet *i*'s folder 0 continues cabinet *i-1*'s last folder exactly when
    cabinet *i* has a file with iFolder `0xFFFD` or `0xFFFF`. Otherwise it is a
    new folder.
  - uoffFolderStart is the offset within the **logical** (merged) folder.
  - Each file is counted once, from its starting record (iFolder < `0xFFFD`,
    or `0xFFFE`). Records `0xFFFD` and `0xFFFF` are skipped.
- **Split block:**
  - When the last CFDATA of a folder that continues has `cbUncomp == 0`, it is
    a fragment.
  - Its bytes are joined in front of the bytes of the first CFDATA of the
    continuing folder in the next cabinet, and that block's `cbUncomp` is the
    real size.
  - `"CK"` appears only at the start of the joined bytes.
- **MSZIP:**
  - Each block is `"CK"` followed by a raw deflate stream ending in a final
    block, and decompresses to cbUncomp (at most 32768) bytes.
  - Each block may refer back to the previous block's **output**, used as the
    dictionary.
  - The history resets at the start of every logical folder.

### Types and signatures (`mscab`): struck

Superseded by `unpack::cab_portable` on `main` (F-15, ruling LI-02-ruling-1).
No packet builds `mscab`.

### Test fixture writer (LI-02): struck

Superseded: `cab_portable/fixture.rs` on `main` is the every-host,
MakeCAB-shaped writer (ruling LI-02-ruling-1).

### Telemetry contract (LI-05, LI-06, LI-09)

- `UnpackError::code()` returns one of: `io`, `zip`, `rar`, `cab`,
  `unknown_format`, `unsafe_path`, `entry_conflict`, `patchset`,
  `patchset_hash`, `cancelled`.
- `InstallError::unpack_code() -> Option<&'static str>` returns `Some` only for
  `InstallError::Unpack`.
- The forwarded field is `unpack_kind`.
- The local-only targets are `launcher.install.detail` (event
  `install_stage_detail`) and `launcher.repair.detail` (event
  `repair_stage_detail`), each with field `detail`.
- `SummaryEndpoint::upgraded_from_http_login_server(&str) -> Option<SummaryEndpoint>`.
  The new resolve source is `login_server_https_upgrade`.
- LI-09 moves the check behind it into `launcher-core` as
  `telemetry_endpoint::https_upgrade_origin(&str) -> Option<String>`, which
  returns `"https://<name>"`, and LI-06's function delegates to it.

---

## Struck: LI-01 to LI-04

Each is superseded by #2020/#2023 (ruling LI-02-ruling-1, README F-15). Do not
dispatch, merge or ship any of them.

- **LI-01** (`mscab` header and MSZIP decoder): merged only into the abandoned
  integration branch (`9d61134f7`).
- **LI-02** (test cabinet writer): closed at `664e59001`, never merged.
- **LI-03** (`mscab` plan): `8d31da787`, never merged.
- **LI-04** (extraction and wiring): exists on `main`. Its `SGW.exe` size
  assert moved to LI-07.

## Wave 1: LI-05 and LI-06

### LI-05: unpack detail on install and repair stage failures

- Host: linux. Branch `launcher-linux-install-telemetry/li-05` from `main`; worktree `li-05`.
- Commit: `feat(launcher): log the unpack error's kind and local detail on a stage failure (LI-05)`
- Rules: R1-R9.
- Lane:
  - root: `-p cimmeria-launcher-core`.
  - desktop: `-p cimmeria-launcher-engine`.

**Files**

1. `crates/launcher-core/src/install_codes.rs`: add `impl crate::unpack::UnpackError { pub fn code(&self) -> &'static str }`.
   - It is an exhaustive `match` with no `_` arm, mapping each variant to the
     contract code: `Io → "io"`, `Zip → "zip"`, `Rar → "rar"`, `Cab → "cab"`,
     `UnknownFormat → "unknown_format"`, `UnsafePath → "unsafe_path"`,
     `EntryConflict → "entry_conflict"`, `Patchset → "patchset"`,
     `PatchsetHash{..} → "patchset_hash"`, `Cancelled → "cancelled"`.
   - Add `impl InstallError { pub fn unpack_code(&self) -> Option<&'static str> }`
     beside `code()` and `http_status()`, returning `Some(e.code())` only for
     `Self::Unpack(e)`.
2. `crates/launcher/desktop/engine/src/storage/install_worker/events.rs`, `install_stage_failed`:
   - Add `unpack_kind = error.unpack_code(),` to the existing `warn!`, after
     `reason`. `Option` fields are recorded only when `Some`, like `http_status`.
   - Then add a second event:
     ```rust
     tracing::warn!(target: "launcher.install.detail", event = "install_stage_detail", operation,
         detail = %error, "install: stage failure detail (local log only)");
     ```
3. `crates/launcher/desktop/engine/src/storage/repair/events.rs`,
   `repair_stage_failed`: the same, with target `launcher.repair.detail` and
   event `repair_stage_detail`.
4. Check before you commit:
   - Search `crates/launcher/desktop/` for `starts_with("launcher.install` and
     `starts_with("launcher.repair`. If anything forwards by prefix, stop and
     report `blocked`.
   - Search `game_telemetry/` for `http_status`. If forwarded fields pass an
     allowlist, add `unpack_kind` next to `http_status` there and list that file
     in the commit.
   - Do **not** add the `.detail` targets to `LAUNCHER_EVENT_TARGETS`.

**Tests** (unit; use the existing `capture` and `event_named` helpers in each events.rs. If the helper does not record the target, extend it in that test module.)

- `an_unpack_failure_logs_its_detail_locally` (install and repair):
  - The error is `InstallError::Unpack(UnpackError::Cab("expanding the cabinet set in /home/u/x/Data needs Windows".into()))`.
  - Assert that the `install_stage_detail` event exists with target
    `launcher.install.detail`, and that its `detail` contains `needs Windows`.
  - Assert `wire_target("launcher.install.detail") == None`.
  - This fails when reverted: there is no detail event.
- `the_forwarded_failure_carries_a_closed_kind_and_no_text`:
  - On `install_stage_failed`, `reason == "unpack"` and `unpack_kind == "cab"`.
  - No field value contains `/home/` or `needs Windows`.
- A non-unpack error (any other `InstallError` variant the tests already build)
  has no `unpack_kind` field.
- `install_codes`: a table test listing every `UnpackError` variant and its code.

**Docs (rows 34 and 57)**

- `docs/operations/telemetry.md`, the launcher event table (around lines 709-721):
  - Add `unpack_kind` to the `install_stage_failed` and `repair_stage_failed` rows.
  - Add one sentence: the `.detail` targets stay in the local log and are never forwarded.
- `crates/launcher/desktop/docs/launcher-telemetry.md`: the same sentence,
  next to the "no … arbitrary error text" rule.
- `crates/launcher/desktop/docs/privacy-inventory.md`: one sentence in the
  local-log section: the stage-failure detail holds the error text and paths,
  and it leaves the machine only through **Upload debug logs**, which scrubs it.

### LI-06: https upgrade for the summary endpoint

- Host: linux. Branch `launcher-linux-install-telemetry/li-06` from `main`; worktree `li-06`.
- Commit: `fix(launcher): send summaries over https when the login server is http (LI-06)`
- Rules: R1-R9.
- Lane: desktop `-p cimmeria-launcher-engine`, then `-p cimmeria-launcher-desktop`.

**Files**

1. `crates/launcher/desktop/engine/src/storage/launcher_summary/endpoint.rs` (233 lines). Add this beside `for_login_server`:
   ```rust
   /// An http login server on a public DNS name: the same name over https on 443,
   /// with `/api/` as the base. The game keeps http; summaries never do.
   /// IP literals, localhost, `*.localhost`, single-label names and credentials are refused.
   pub fn upgraded_from_http_login_server(login_server: &str) -> Option<Self> {
       let url = Url::parse(login_server).ok()?;
       if url.scheme() != "http" || !url.username().is_empty() || url.password().is_some() {
           return None;
       }
       let name = url.host_str()?;
       // `Url` normalises IPv4 literals and wraps IPv6 in brackets.
       if name.parse::<Ipv4Addr>().is_ok() || name.starts_with('[') || !upgradable(name) {
           return None;
       }
       Self::parse(&format!("https://{name}/api/"))
   }
   ```
   - `fn upgradable(name: &str) -> bool` is true only when all of these hold:
     `name.contains('.')`, `!name.ends_with('.')`, `name != "localhost"` and
     `!name.ends_with(".localhost")`.
   - Change `resolve` to this order, and update its doc comment:
     ```rust
     if let Some(e) = first.and_then(Self::for_login_server) { return (Some(e), "login_server"); }
     if let Some(e) = first.and_then(Self::upgraded_from_http_login_server) { return (Some(e), "login_server_https_upgrade"); }
     match DEFAULT_SUMMARY_BASE.and_then(Self::parse) { ... unchanged ... }
     ```
   - Update the doc comment on `for_login_server` and on the struct (line 6-7):
     http login servers are upgraded and never used as http.
2. `crates/launcher/desktop/shell/src/host/summary.rs`, test
   `this_build_derives_an_https_endpoint_from_the_first_login_server` (about
   lines 369-387):
   - `http://play.example:8081` now resolves to
     `https://play.example/api/telemetry/launcher-summary` with source
     `login_server_https_upgrade`.
   - Update any comment in that file that says http login servers give no endpoint.

**Tests** (unit, in `endpoint.rs`)

- `an_http_login_server_upgrades_to_https_on_443`:
  - `http://play.example:8081` gives `https://play.example/api/telemetry/launcher-summary`.
  - `http://Play.Example:8081/login` gives the same (lower-case, path dropped).
- `the_upgrade_refuses_ips_local_names_and_credentials`: each of these gives
  `None`:
  - `http://127.0.0.1:8081`
  - `http://192.168.1.10:8081`
  - `http://[2001:db8::1]:8081`
  - `http://localhost:8081`
  - `http://game.localhost:8081`
  - `http://sgwserver:8081`
  - `http://play.example.:8081`
  - `http://user@play.example:8081`
  - `http://u:p@play.example`
  - `https://play.example`
  - `ftp://play.example`
- Rewrite `resolve_falls_through_to_no_https_endpoint` so it no longer asserts
  that `DEFAULT_SUMMARY_BASE == None`:
  - Compute `let fallback = if DEFAULT_SUMMARY_BASE.is_some() { "default" } else { "no_https_endpoint" };`.
  - `resolve(Some("http://play.example:8081"))` has source
    `login_server_https_upgrade`. This fails when reverted.
  - `resolve(Some("http://127.0.0.1:8081"))` and `resolve(None)` have source
    `fallback`.
  - `https://play.example` still gives `login_server`.
  - This keeps LT-41 free to set the constant (D-LI9).
- `resolve_never_yields_plain_http_for_a_login_server`: for every input above,
  plus the http and https inputs already in the file, `resolve(Some(x)).0`
  is either `None` or has `ingest_url().scheme() == "https"`.
- Keep `plain_http_login_servers_are_refused` as it is: `for_login_server`
  itself still refuses http.

**Docs (rows 35, 57, 61 and 54)**

- `crates/launcher/desktop/docs/privacy-inventory.md`, the summaries endpoint
  row (around line 81) and line 31:
  - The destination is the login server's https origin. For an http login
    server on a DNS name, it is the same name over https on 443. Plain http is
    never used, and IPs and local names get no endpoint.
- `docs/architecture/launcher-summary-telemetry.md`, line 12 and lines 338-344:
  - The resolve order and the new source `login_server_https_upgrade`.
- `crates/launcher/desktop/docs/launcher-summaries.md`, line 9 and lines 451-453: the same.
- `docs/operations/telemetry.md`: the `summary_endpoint_resolved` outcome
  values gain `login_server_https_upgrade`.
- `docs/analysis/launcher-telemetry/work-packets-coverage.md`, under LT-41:
  add one line:
  "Amended by launcher-linux-install-telemetry LI-06: an http login server on
  a DNS name now resolves to `login_server_https_upgrade` before the default,
  so LT-41's tests must use an IP or single-label http server to reach
  `default`."

---

## Wave 2: LI-09

LI-09 (game telemetry never over plain http) is in
[work-packets-li09.md](work-packets-li09.md). It depends on LI-06 being merged
to `main`, and ships as its own PR behind gate G-LI9.

---

## Wave A: LI-08, then LI-07 (one PR from `main`)

### LI-08: docs for the native cabinet expander

Re-specced by ruling LI-02-ruling-1.

- Host: linux. Worktree `li-07`, branch `launcher-linux-install-telemetry/li-07`,
  cut from `main` by `ops`. LI-08 commits first, and LI-07 follows on the same
  branch.
- Commit: `docs(launcher): Linux expands the cabinet set natively (LI-08)`. The
  body lists doc-update-map row 57.
- Rules: R4-R6 and R9. No lane run (docs only).
- Facts: README F-15, D-LI1, D-LI3 and D-LI12.

**Files (doc-update-map row 57).** Keep each file's line endings (check with
`file <path>`), and edit only the lines named.

- `crates/launcher/desktop/README.md`, line 348 on `origin/main`. Replace the
  sentence "The original archive's spanning cabinet set still requires Windows
  FDI." with: "Windows expands the original archive's spanning cabinet set with
  FDI. Linux and the Steam Deck use the portable MSZIP expander
  (`unpack::cab_portable`), with no Wine. macOS keeps the Wine helper." Re-wrap
  only the lines that sentence sits on. Leave the rest of the paragraph (the
  Mac/Wine integration text) as it is.
- `crates/launcher/desktop/docs/wine-validation.md`, section "RAR and cabinet
  entry preflight": after the paragraph that ends "…subsequent Wine validation
  of that rebuilt helper." (line 57 on `origin/main`), add one paragraph:
  "Linux no longer reaches the Wine cabinet path: it expands the set in-process
  with `unpack::cab_portable` (#2020). This section's Wine requirements apply to
  macOS only."
- Do not touch `crates/README.md` or `docs/client/launcher-guide.md`. Both are
  already current.

### LI-07: real-client verification

Re-specced by ruling LI-02-ruling-1. It verifies `main`'s `cab_portable` and
carries LI-04's `SGW.exe` size assert. It stays the real-client gate for U1.

- Host: linux (this Steam Deck). Worktree `li-07`, branch `…/li-07`, after the
  LI-08 commit.
- Rules: R1, R4, R5 and R6.
- **Precondition:** the owner's real client RAR is on this Deck, and its
  absolute path is in `SGW_CLIENT_RAR` (the brief gives it). If the variable is
  unset or the file is missing, change nothing and return `blocked` with the
  reason `no client RAR`. The run needs about 20 GB free.

1. In `crates/launcher-core/src/unpack/mod.rs`, test `real_client_rar`, right
   after the line `assert!(crate::install_layout::sgw_exe(&dest).is_file());`
   (line 462 on `origin/main`), add
   `assert_eq!(std::fs::metadata(crate::install_layout::sgw_exe(&dest)).unwrap().len(), 31_228_448);`
   (F-10). Run `cargo fmt -p cimmeria-launcher-core`, then
   `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-launcher-core --features test-fixtures --all-targets -- -D warnings`.
2. Commit only that file:
   `test(launcher-core): real_client_rar checks the SGW.exe size (LI-07)`. The
   body says the test is `#[ignore]` and driven by env vars, so it is a
   verification guard, not a revert test.
3. Make an empty directory that does not exist yet, for example
   `/home/deck/sgw-unpack-check`.
4. From the `li-07` worktree root, with `SGW_CLIENT_RAR` set, run
   `SGW_UNPACK_DEST=/home/deck/sgw-unpack-check bash tools/build-lane/lane.sh cargo test -p cimmeria-launcher-core --features test-fixtures real_client_rar -- --ignored --nocapture`.
5. Write `docs/analysis/launcher-linux-install-telemetry/worknotes/li-07.md`
   (new, CRLF). Record pass or fail, the elapsed time, the file count under the
   test directory, the `SGW.exe` size and the first error line on failure. Name
   `unpack::cab_portable` as the code under test, with the `main` SHA the branch
   was cut from (the brief gives it).
6. Delete the test directory. Commit only the worknote:
   `docs(launcher): LI-07 real-client unpack check`.
7. On failure, still commit the worknote, report `failed` with the first error
   line, and fix nothing. Any `cab_portable` fix becomes a new
   architect-specced packet against `main`.

LI-08 and LI-07 ship as **one PR** from `…/li-07` through `ship-pr` (by
`ops`). The PR body lists doc-update-map row 57.
