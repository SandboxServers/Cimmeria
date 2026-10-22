# Launcher UAT fixes: work packets (rules, contract, waves 1-2)

Ledger: [README.md](README.md). Waves 3-5 and the close-out: [work-packets-logs.md](work-packets-logs.md). Every packet re-greps its anchors (`file:line` was taken at `origin/main` `3c10f3624`) and returns `blocked` when an anchor no longer holds.

Paths: `E` = `crates/launcher/desktop/engine/src/storage/`, `A` = `E/adoption/`, `FE` = `crates/launcher/desktop/frontend/`, `T` = `crates/admin-api/src/routes/telemetry/`.

## Dispatch

| Wave | Packets and merge order into the integration branch | Integration branch (worktree) | Ships as |
|---|---|---|---|
| 1 | LF-01, LF-04, LF-05 in parallel → LF-02 → LF-03 | `launcher-uat-fixes/adoption-engine` (`lf-w1`) from `origin/main` | PR U1 |
| 2 | LF-06, LF-07 in parallel → LF-08 → LF-09 | `launcher-uat-fixes/adoption-ui` (`lf-w2`) from `origin/main` | PR U2 |
| 3 | LF-10, LF-11 in parallel → LF-12 | `launcher-uat-fixes/log-ingest-server` (`lf-w3`) from `origin/main` | PR U3 |
| 4 | LF-13, LF-14 in parallel → LF-15 → LF-16 and LF-19 in parallel → LF-17 | `launcher-uat-fixes/log-ingest-launcher` (`lf-w4`) from the `lf-w3` tip | PR U4 (stacked on U3) |
| 5 | LF-18 (D-LU2 = retire) | own branch | PR U5 |

- Waves 1, 2 and 3 have no shared files and run at the same time. Each packet branches from its integration branch's tip **after** the packets listed before it have merged in (`merge-noff.ps1`).
- Each packet's worktree is `lf-NN` on branch `launcher-uat-fixes/lf-NN-<slug>`. Retire it, its `B:\targets\` folder and its `sgw_lf-NN` database as soon as it merges into the integration branch.
- Review loops: one packet-reviewer pass per packet (at most 3), then one wave review before the PR. Wave 3, LF-14 and LF-19 also get the `network-security-auth` pass (D-LF11); per D-LU4 it hardens and never removes data.
- LF-02, LF-03, LF-12, LF-14, LF-15 and LF-19 go to `rust-gameserver-dev`, and LF-17 to `documentation-writer`. They use the same brief shape and the same rules.

## Rules (every packet; packet-coder reads these)

- **R1 Host and shell.** `Host: windows`. Use PowerShell only, never Bash. Start every command with `Set-Location <worktree>;`, use absolute paths, run it in the foreground and wait for it. Never call `cargo` directly; only through `pwsh tools/build-lane/lane.ps1`. Run the checks in the listed order, with at most 5 fix rounds per check. If `logs/<worktree>/` shows no new lane run, you ran from the wrong folder.
- **R2 Scope and git.** Edit only the files your packet names, plus the new files it names. No `git stash`, no `git worktree prune`, no push, no rebase. Make one local commit with the given subject, ending with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_017MT2fqiV1BEnczLgt4Z7np
  ```
- **R3 File caps.** Soft cap 500 lines, hard cap 700. A named file already over 450 lines gets only call lines and `mod` lines; the logic goes in the new file the packet names. Tests go in a sibling `*_tests.rs` file (`#[cfg(test)] #[path = "..."] mod tests;` as the neighbours do). Report every touched file's line count.
- **R4 Line endings.** Files under `docs/` are CRLF; keep them CRLF. Code files keep their current line endings. `git diff --stat` must show only your own lines.
- **R5 Privacy.** In any event field or message: never an absolute path, URL, host, IP, port, account or user name, token, session id, or an error's `Display`/`Debug`. Use closed snake_case codes, counts and sizes. On the `adoption` target only, a path **relative to the chosen folder** (`/`-separated) is allowed (R5a of launcher-telemetry).
- **R6 Log discipline.** `info` for one start and one outcome, `warn` for a recoverable failure the player sees, `error` for an unrecoverable one. At most 12 fields per event, snake_case keys, numbers as numbers. Never the key `count` (the server denies it; use a specific name).
- **R7 Tests.** Unit tests ([TESTING.md](../../../TESTING.md)). Every new test must fail when your change is reverted; say in the report which assertion fails and why. Pin the level, target and `event`/`outcome` fields, not message text. Capture tracing only with `crate::test_capture` (the existing helper; never add a second one).
- **R8 Frontend.** The decoders use `onExcessProperty:'error'`: a new or changed Rust DTO field and its TS schema land in the same commit. Every new `*.test.ts` file is added to the `npm test` script in `FE/package.json`. A visible button gives visible feedback on the first press.
- **R9 Report and return.** Write `<worktree>\.claude\reports\launcher-uat-fixes\<packet>-coder.md`: the files and line counts, each check and its result, the revert proof, and anything you rejected and why. Return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. When the packet does not cover something, return `blocked`; don't invent a design.
- **R10 Downloaded installs.** Behaviour for installs that were downloaded, or adopted by the verified path, stays byte for byte the same except where a packet says otherwise.

### Checks (copy the exact command)

| ID | Command |
|---|---|
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | the same with `-p cimmeria-launcher-desktop` |
| D3e `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` (engine tests use `cargo test`; nextest cannot list that crate) |
| D3s `<f>` | the same with `-p cimmeria-launcher-desktop <f>` |
| C1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-launcher-core` |
| C2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-launcher-core --all-targets -- -D warnings` |
| C3 `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-launcher-core <f>` |
| S1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-admin-api` |
| S2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-admin-api --all-targets -- -D warnings` |
| S3 `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api --lib <f>` |
| F1 | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend` |
| E2E | the whole engine suite: D3e with no filter (run once, last, by LF-02 and LF-03) |

## Contract

### C-LF1: tree diff (`A/tree_diff.rs`, LF-01)

```rust
use super::inventory::Tree; // BTreeMap<String, (u64, [u8; 32])>, folder-relative `/` keys

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct TreeDiff {
    pub missing: Vec<(String, u64, [u8; 32])>,     // expected, absent from actual
    pub changed: Vec<Changed>,                     // present in both, size or digest differs
    pub unexpected: Vec<(String, u64, [u8; 32])>,  // actual, absent from expected
    pub case_only: Vec<(String, String)>,          // (expected key, actual key), same bytes
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Changed { pub path: String, pub expected: (u64, [u8; 32]), pub actual: (u64, [u8; 32]) }

/// Pairs keys exactly first, then by `str::to_lowercase()` for the keys left over.
/// `presence_only` keys (matched case-insensitively) are never `changed`; a missing one is still `missing`.
/// Output vectors are sorted by key.
pub(super) fn diff(expected: &Tree, actual: &Tree, presence_only: &[&str]) -> TreeDiff;

impl TreeDiff {
    pub(super) fn blocking(&self) -> bool;  // !missing.is_empty() || !changed.is_empty()
    pub(super) fn is_empty(&self) -> bool;  // all four empty
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TreeOutcome { Matched, Accepted, Mismatch, Unrecorded }
impl TreeOutcome { pub(super) fn code(self) -> &'static str; } // "matched" | "accepted" | "mismatch" | "unrecorded"

/// The fields of C-LF2, without `event`; pure so tests need no capture.
pub(super) fn tree_checked_fields(check: &str, outcome: TreeOutcome, diff: Option<&TreeDiff>) -> Vec<(&'static str, String)>;
/// Emits C-LF2 at the level C-LF2 gives.
pub(super) fn log_tree_checked(check: &str, outcome: TreeOutcome, diff: Option<&TreeDiff>);
```

When two keys of one tree fold to the same lowercase key, pair those exactly or report them as missing/unexpected; never panic.

### C-LF2: event `adoption_tree_checked` (target `adoption`)

| Field | Value |
|---|---|
| `event` | `"adoption_tree_checked"` |
| `check` | `"light"` or `"verified"` |
| `outcome` | `matched` (diff empty, INFO), `accepted` (only `case_only`/`unexpected`, WARN), `mismatch` (blocking, ERROR), `unrecorded` (no expected tree in the stage, WARN) |
| `missing`, `changed`, `unexpected`, `case_only` | counts (u64); omitted when `diff` is `None` |
| `missing_paths`, `changed_paths`, `unexpected_paths`, `case_paths` | up to 3 samples each, joined by `"; "`, omitted when empty; the whole value at most 230 bytes (stop adding samples before it would pass 230; cut a single over-long sample on a char boundary) |

Sample forms (`h12` = the first 12 lowercase hex digits of the SHA-256, kept under the server's 24-hex scrub):
- missing / unexpected: `"{path} {size} {h12}"`
- changed: `"{path} {exp_size}>{act_size} {exp_h12}>{act_h12}"`
- case: `"{expected}>{actual}"`

At most 11 fields. Paths are folder-relative (R5a). The verified path logs `check = "verified"`, `outcome = "mismatch"` with no diff fields.

### C-LF3: the stage files (LF-02, LF-03)

- `<stage>/expected-tree.json`: `{"version":1,"tree":{"<key>":[<size>,"<64 hex>"],...}}`. Written in `admit` for Light only, before the `Staged` record, through the same atomic-write helper the stage already uses. Not part of `Plan` (Plan is `deny_unknown_fields` and digested; it does not change).
- `<stage>/setup-originals/<key>`: byte copies of each `SETUP_FILES` entry that exists in `game/` after the moves (key resolved case-insensitively against `game/`).
- `<stage>/setup-originals.json`: `{"version":1,"files":[{"path":"<key as found on disk>","existed":true|false}]}`, one row per `SETUP_FILES` entry, written **last**, after every copy is flushed.

## Wave 1: adoption engine (PR U1)

### LF-01: tree diff and the `adoption_tree_checked` helper

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-01`, branch `launcher-uat-fixes/lf-01-tree-diff`, from the `lf-w1` tip. **Rules:** R1-R9. **Contract:** C-LF1, C-LF2.
- **Files:** new `A/tree_diff.rs`, new `A/tree_diff_tests.rs`; `A/mod.rs` (one `mod tree_diff;` line).
- **Do:** implement C-LF1 exactly. `log_tree_checked` calls `tree_checked_fields` and emits one `tracing` event on target `adoption` at C-LF2's level, with `event = "adoption_tree_checked"`. The message: `"adoption tree check: <outcome>"`. Nothing calls it yet; mark the module `#[allow(dead_code)]` only if clippy refuses otherwise, and say so in the report.
- **Tests (unit, `tree_diff_tests.rs`):**
  1. Equal trees → empty diff, `Matched`.
  2. `Working/SGWGame/a.txt` expected vs `Working/sgwgame/a.txt` actual, same bytes → one `case_only`, not blocking.
  3. Size differs → `changed`, blocking; digest differs with the same size → `changed`.
  4. Expected key absent → `missing`, blocking. Extra actual key → `unexpected`, not blocking.
  5. A presence-only key with a different digest → nothing; absent → `missing`.
  6. Case-only and changed at once (`a/B.txt` vs `A/b.txt`, different bytes) → `changed` with the expected path.
  7. `tree_checked_fields`: 5 missing entries give `missing = "5"` and exactly 3 samples; a changed sample has the `>` form with 12-hex prefixes; every value ≤ 230 bytes with 3 samples of 200-byte non-ASCII paths (char-boundary cut).
  8. `log_tree_checked` under `crate::test_capture`: `Mismatch` → ERROR, target `adoption`, `event`, `outcome = "mismatch"`; `Unrecorded` → WARN with no count fields.
  Revert proof: the module is new; the report names the assertion each test pins.
- **Checks:** D1, D2e, D3e `tree_diff`.
- **Docs:** none (LF-02 owes them).
- **Commit:** `feat(launcher): adoption tree diff and adoption_tree_checked event (LF-01)`

### LF-04: scans accept valid-Unicode names (F2)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-04`, branch `launcher-uat-fixes/lf-04-unicode-names`, from the `lf-w1` tip. **Rules:** R1-R10. **Contract:** D-LF3 in the README.
- **Files:** `A/inventory.rs` (503 lines: call lines plus one small fn only), `A/survey.rs`, `A/survey_tests.rs`, `E/game_telemetry/launcher_event.rs` (only if step 3 finds a byte cut), and its existing test file.
- **Do:**
  1. In `inventory.rs` add `fn scan_name_allowed(name: &str) -> bool`: `!name.is_empty() && name.len() <= 4096 && !name.contains(':') && !name.contains('\\') && !name.chars().any(char::is_control)`. Use it in `scan_inner` (`:314`) where `index_name_allowed` (`:123`) is used today. `index_name_allowed` stays for every other caller (grep them; list them in the report). Names that are not valid UTF-8 (`OsStr::to_str() == None`) stay refused with `UnsafeReason::Name`.
  2. `survey.rs:80-82` keeps mapping `UnsafeReason::Name`; update its explanation text only if it says "ASCII".
  3. Grep `launcher_event.rs` for the 240-byte cut. If it slices a `String` by byte index without `is_char_boundary`, cut at the largest char boundary ≤ 240 instead and add a test with a 250-byte string of `á`. If it is already boundary-safe, change nothing there and say so.
  4. Update the survey tests at `:72`, `:88`, `:161` that build a non-ASCII name to expect acceptance. Keep one refusal test, using a name with a control character (`"bad\u{7}.bmp"`).
- **Tests (unit):** a scan of a temp folder holding `Working/Binaries/wxRes/TerrainEdit_Gradient másolata.bmp` returns it in the tree with its size and digest (fails on revert: `UnsafeFile`). A name with `\u{7}` is still refused.
- **Checks:** D1, D2e, D3e `inventory`, D3e `survey`, D3e `launcher_event`.
- **Docs:** none (LF-17 writes the support note).
- **Commit:** `fix(launcher): adoption scans accept non-ASCII stock file names (LF-04)`

### LF-05: an invalid login-server list refuses the light preview (F1d)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-05`, branch `launcher-uat-fixes/lf-05-server-list`, from the `lf-w1` tip. **Rules:** R1-R10. **Contract:** D-LF4.
- **Files:** `A/light.rs` (367 lines), `A/light_tests.rs`, plus the rejection type and its TS schema only if step 2 needs a new variant.
- **Do:**
  1. Find the validation that `launcher-core` `client_setup` `login_servers::write` (`:130`) runs before it refuses a list. If it is a private fn, return `blocked` (that is a core change this packet does not own) and name the fn. If it is public, call it from the light preview before `apply_setup` (`light.rs:318`) builds the expected tree.
  2. On failure, refuse the preview the way light already refuses (grep `adoption_rejected` in `light.rs` and `in_place_preview.rs`), with the check code `login_servers_invalid`. If the check codes are a serde enum the frontend decodes, add the variant to the Rust enum and the TS schema in the same commit (R8) and run F1 too. If that takes more than two files beyond the named ones, return `blocked`.
- **Tests (unit, `light_tests.rs`):** a `Light` fixture (`:46`) whose installation has an invalid server list (use a value the core validator refuses; read its tests) → the preview is refused with `login_servers_invalid`, and nothing in the folder moved (fails on revert: the preview succeeds). A valid list still previews.
- **Checks:** D1, D2e, D3e `light`, then F1 if step 2 touched TS.
- **Docs:** none.
- **Commit:** `fix(launcher): refuse a light adoption preview with an invalid server list (LF-05)`

### LF-02: Light final check by diff; Recover finishes old adoptions (F1, G1)

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `lf-02`, branch `launcher-uat-fixes/lf-02-light-check`, from the `lf-w1` tip after LF-01 and LF-05 merged in. **Rules:** R1-R10. **Contract:** C-LF1, C-LF2, C-LF3; D-LF1.
- **Files:** `A/verification.rs` (`Compared`, `:357`), `A/mod.rs` (`Preview`, `:122`, `:348`, `:397`), `A/light.rs` (`:218-221`, `:274-286`), `A/in_place_preview.rs` (`:167`, `:231`, only to pass the field through), `A/publication.rs` (`:169`, only if `Preview` construction there needs the field), `A/in_place.rs` (627 lines: call lines only), new `A/in_place_check.rs`, new `A/in_place_check_tests.rs`, `A/light_tests.rs`.
- **Do:**
  1. Add `expected_tree: Option<Tree>` to `Compared` and `Preview`. Light sets `Some(tree.clone())` at `light.rs:218-221`, after `apply_setup`, from the same `tree` that gives `expected_digest`. Every other constructor sets `None`.
  2. In `in_place.rs` `admit`, before the `Staged` record write (`:249-256`), write `<stage>/expected-tree.json` (C-LF3) when `expected_tree` is `Some`. Put the write fn in `in_place_check.rs`.
  3. Move `validate` (`in_place.rs:506-515`) into `in_place_check.rs` and split it:
     - `provenance.check` is not Light: the existing digest comparison, unchanged. On a miss it calls `log_tree_checked("verified", Mismatch, None)` instead of today's `tracing::error!` with `game = %game.display()` (that field breaks R5).
     - Light: read `expected-tree.json`. Absent → `log_tree_checked("light", Unrecorded, None)` and `Ok(())`. Present → `scan` the game folder, `diff(&expected, &actual, &SETUP_FILES)`, log `Matched`, `Accepted` or `Mismatch`, and return `Err(StorageError::Corrupt.into())` only when `blocking()`. A file that fails to parse is treated as absent, with `outcome = "unrecorded"`.
  4. `finish` (`:542`) calls the new `validate` at the same place (`:593`). Nothing else in `finish` changes order.
  5. `provenance.reference_digest` stays the predicted digest. Grep every read of `reference_digest`; if anything other than the two validators and `publication.rs:282`/`:418` reads it after `Published`, return `blocked` and list it.
- **Tests (unit):**
  1. **Case divergence (F1a):** a light fixture whose `game/` already has `Working/SGWGame/...`, while a replayed patch creates a file under `Working/sgwgame/...`. On today's code `finish` fails with `Corrupt`; with the change it publishes, and the capture shows `outcome = "accepted"` with `case_only ≥ 1`. If the fixture cannot reproduce a case split on the test file system, say so and pin step 3 with a hand-made expected tree instead.
  2. **LoginInternal absent (2b):** the fixture lacks `LoginInternal.lua`; the adoption publishes.
  3. **Real loss:** the `finish` hook at `Point::SetUp` deletes one release file → `Corrupt`, `outcome = "mismatch"`, `missing_paths` names it; the record stays `Promoted`.
  4. **Old adoption:** a `Promoted` light adoption whose stage has no `expected-tree.json`, run through `publication_recovery::recover` (`:68`) → `Published`, `outcome = "unrecorded"`.
  5. **Verified unchanged:** the verified fixture's existing digest-mismatch test still fails with `Corrupt` and now logs `check = "verified"`.
  Revert proof: tests 1, 2 and 4 fail with `Corrupt` on `main`.
- **Checks:** D1, D2e, D3e `in_place`, D3e `light`, D3e `publication_recovery`, then E2E.
- **Docs:** none (LF-17).
- **Commit:** `fix(launcher): light adoption checks the tree per file and explains a mismatch (LF-02)`

### LF-03: setup originals and Abandon after `Promoted` (D-LF2)

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `lf-03`, branch `launcher-uat-fixes/lf-03-abandon-promoted`, from the `lf-w1` tip after LF-02 merged in. **Rules:** R1-R10. **Contract:** C-LF3; D-LF2.
- **Files:** `A/in_place_check.rs` (or a new `A/setup_originals.rs` if it would pass 400 lines), `A/in_place.rs` (call line only), `A/in_place_abandon.rs` (72 lines), the shell's adoption status (`shell/src/host/adoption/status.rs:250-271`) only if `can_abandon` is not computed from `abandonable`, new `A/setup_originals_tests.rs`.
- **Do:**
  1. In `finish`'s `Staged` branch, after `move_files` and **before** `record.phase = Phase::Promoted` (`in_place.rs:581`), for Light only: copy each existing `SETUP_FILES` entry into `<stage>/setup-originals/`, then write `setup-originals.json` last (C-LF3). A rerun overwrites both. Verified adoptions do nothing new (R10).
  2. `abandonable` (`in_place_abandon.rs:9`): `Staged`, or `Promoted` + Light + `setup-originals.json` present and parseable.
  3. `abandon` for a `Promoted` record, before today's `Way::Abandon` moves: for each manifest row, `existed: true` → copy the original back over the game file; `existed: false` → delete the game file if present. Then delete the files `finish` wrote after `Promoted` that are not release files: the ledger that `reference::ledger(..).save(&game)` writes (find its file name), and `content-ready.json` if present. Then run the existing abandon steps unchanged. Stock-case renames are not undone (D-LF2).
  4. Log through the existing abandon event, adding `restored_setup = <n>` (a count).
  5. Check how the shell computes `can_abandon` for `Reconciliation::Copy`. If it calls `abandonable`, nothing to do; if not, make it call it.
- **Tests (unit):**
  1. A light adoption that fails `validate` after `Promoted` (reuse LF-02 test 3's hook) → `abandonable` is true → `abandon` restores `SGW.exe` byte-identical to the pre-adoption copy, removes a `LoginInternal.lua` that did not exist before, and the folder's file set matches the pre-adoption scan (stock-case aside). Fails on revert: `abandonable` is false.
  2. A `Promoted` light record whose stage has no `setup-originals.json` → not abandonable.
  3. A verified `Promoted` record → not abandonable (unchanged).
- **Checks:** D1, D2e, D2s if the shell changed, D3e `abandon`, D3e `in_place`, then E2E.
- **Docs:** none (LF-17 and the close-out). D2 of launcher-telemetry (abandon wording) stays open.
- **Commit:** `feat(launcher): light adoption can be abandoned after setup started (LF-03)`

## Wave 2: adoption UI (PR U2)

All four packets: `Host: windows`, **Rules:** R1-R4, R8, R9, **Check:** F1. Markup is static in `FE/ui/index.html`. Tests use node:test with linkedom (`parseHTML` of `index.html`, `mountX(document, fakeInvoke)`, then settle); copy the pattern from the test file named.

### LF-06: one owner for an interrupted adoption (F3, D-LF5)

- **Agent:** packet-coder. **Worktree:** `lf-06`, branch `launcher-uat-fixes/lf-06-one-owner`, from the `lf-w2` tip.
- **Files:** `FE/src/install-view.ts`, `FE/src/launch-view.ts`, `FE/src/launch-workflow.ts` (only if `launchBlocked` text lives there), `FE/src/install-view.test.ts`, `FE/src/launch-view.test.ts`.
- **Do:** find the operation kind string for an adoption in `contract.ts` (`Operation.kind`, `:4-10`; expect `"adopt"`) and treat an operation as an *adopt in progress* when that kind's state is not terminal.
  1. `install-view.ts`: while an adopt is in progress, hide the install primary (`:55-60`), the recovery text (`:98`, `recovery` at `:52`) and `#inspect-install` (`:78-79`). Everything else unchanged.
  2. `launch-view.ts`: while an adopt is in progress, replace the generic error (`:8`) and "Another operation owns the game." (`:17`) with one line, and hide `#inspect-launch`:
     - state `reconciliation_required`: `Finish or abandon the interrupted adoption in the adoption panel before you play.`
     - any other non-terminal state: `An adoption review is open. Finish or cancel it in the adoption panel.`
  3. A non-adopt operation keeps today's text exactly.
- **Tests:** install-view: an adopt in `reconciliation_required` → primary, recovery text and `#inspect-install` hidden; an install in `reconciliation_required` → unchanged. launch-view: the two adopt lines, `#inspect-launch` hidden; an install operation keeps "Another operation owns the game.". Revert proof: the generic text shows.
- **Commit:** `fix(launcher): only the adoption panel speaks for an interrupted adoption (LF-06)`

### LF-07: display paths and one folder line (F4a)

- **Agent:** packet-coder. **Worktree:** `lf-07`, branch `launcher-uat-fixes/lf-07-display-paths`, from the `lf-w2` tip.
- **Files:** new `FE/src/display-path.ts`, new `FE/src/display-path.test.ts`, `FE/src/adoption-view.ts` (287 lines), `FE/src/adoption-view.test.ts`, `FE/package.json` (test list).
- **Do:**
  1. `export function displayPath(p: string): string`: `\\?\UNC\server\share\x` → `\\server\share\x`; `\\?\C:\x` → `C:\x`; anything else unchanged.
  2. At `adoption-view.ts:215-217`, run `source` and `destination` through `displayPath`. When they are equal after that (in place and light), show one line, `Folder: <path>`, instead of two; a copy keeps its two labelled lines.
  3. `:154`: `Stargate Worlds found in X.` → `Found Stargate Worlds in <displayPath(source)>.` (the full folder, not `folderName`).
- **Tests:** `displayPath` for the three forms; the review of an in-place plan shows one `Folder:` line with no `\\?\`; a copy plan shows two lines. Revert proof: the `\\?\` prefix shows.
- **Commit:** `fix(launcher): adoption review shows plain folder paths once (LF-07)`

### LF-08: release details toggle and one set of counts (F4b, D-LF6)

- **Agent:** packet-coder. **Worktree:** `lf-08`, branch `launcher-uat-fixes/lf-08-release-details`, from the `lf-w2` tip after LF-07 merged in.
- **Files:** `FE/src/adoption-view.ts`, `FE/src/adoption-view.test.ts`, `FE/ui/style.css` (36 lines).
- **Do:**
  1. First grep the manifest patch type in `crates/launcher-core/src` for a display name (`name`, `title`, `label`). If one exists and reaches the review DTO, use it in step 2's summary and say so; otherwise use counts only.
  2. `:120` and `:124`: the summary keeps `Signed release` with the patch count and the download count and total size. The manifest digest, the patch ids in signed order and the download ids move into `<details class="release-details"><summary>Release details</summary>…</details>` under it.
  3. Counts: keep the plan's counts at `:121`. Drop from `:154`/`:155` any number `:121` already shows. Leave `adoption-index-view.ts` unchanged. List the before and after strings in the report.
  4. `style.css`: `details > summary { cursor: pointer; list-style: none; }`, `details > summary::-webkit-details-marker { display: none; }`, `details > summary::before { content: "+ "; }`, `details[open] > summary::before { content: "- "; }`. No `▶` character anywhere in `FE/src` or `FE/ui`.
- **Tests:** the review's visible text (outside `details`) holds no 64-hex string; `details.release-details` holds the digest and every patch id; each plan count appears once in the visible text. Revert proof: the digest shows in the summary.
- **Commit:** `fix(launcher): adoption review hides hashes behind Release details (LF-08)`

### LF-09: telemetry line, maintenance trigger, button gaps, UAT (F5, F6)

- **Agent:** packet-coder. **Worktree:** `lf-09`, branch `launcher-uat-fixes/lf-09-review-polish`, from the `lf-w2` tip after LF-08 merged in.
- **Files:** `FE/src/adoption-view.ts`, `FE/src/adoption-view.test.ts`, `FE/ui/index.html`, `FE/adoption-light-uat.mjs`.
- **Do:**
  1. `:127`: add the missing space before the inserted sentence, and make the sentence (shown when `!review.game_telemetry_available`): `This build has no game diagnostics module, so only launcher diagnostics are sent.`
  2. `:205`: the maintenance trigger is `hidden` (not `disabled`) while its confirmation is open; it shows again when the confirmation closes.
  3. `index.html:12`, `:24`, `:42`: wrap each pair of confirmation buttons in `<div class="settings-actions">…</div>` (the class already exists in `style.css`).
  4. Extend `adoption-light-uat.mjs` with REPL-style steps that print and assert: the one-owner banner (LF-06), one folder line (LF-07), the details toggle (LF-08), the telemetry sentence spacing, and the trigger hiding. Run it with `node` after F1 and paste its output in the report.
- **Tests:** the sentence has a space before it; the trigger is hidden while open and visible after cancel; each of the three confirmations has its buttons inside `.settings-actions`. Revert proof: `disabled` instead of `hidden`; the joined sentence.
- **Commit:** `fix(launcher): adoption review spacing, maintenance trigger and button gaps (LF-09)`
