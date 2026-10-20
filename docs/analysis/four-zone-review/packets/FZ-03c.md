# FZ-03c: Rule 6 names on the minigame result path

- **Wave:** 2. **Depends on:** FZ-01 merged.
- **Fixes:** F1, 18 of the 87 fields. FZ-03a fixes 30 and FZ-03b fixes 39.
- **Agent:** packet-coder. About 30k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03c`.
- **Branch:** `four-zone-review/FZ-03c-minigame-names`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03c\.claude\reports\four-zone-review\FZ-03c-coder.md`
- **Return (one line):** `FZ-03c <done|blocked|failed> <sha|-> <report path>`

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03c;`.
2. Every compiling `cargo` call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never run `cargo` directly. Logs land in `$env:LOCALAPPDATA\cimmeria-build\logs\fz-03c\`.
3. Edit only the four files below, plus the new test file and its `mod` line (see Test). These are forbidden: `git stash`, `git worktree prune`, fetch, push, rebase and `git add -A`.
4. **Logging Rule 6.** Every `*_id` field on a `tracing` event has its `*_name` partner on the same event, for example:
   - `entity_id` with `entity_name`;
   - `player_id` with `player_name`;
   - `receipt_player_id` with `receipt_player_name`;
   - `expected_player_id` with `expected_player_name`;
   - `actual_player_id` with `actual_player_name`.

   A `None` name is fine. Never set `NT_BASELINE_BLESS`.
5. Name sources:
   - **The `cimmeria-minigame` crate has neither `cimmeria-entity` nor `cimmeria-names`; do not add them.** Its name source is `MinigameSession.player_name: Option<String>`, captured at launch.
   - `cimmeria-cell` has `cimmeria-entity`, so use `cimmeria_entity::known_names::player_name(id) -> Option<&'static str>` there. It accepts `i32`, `u32` or an `Option` of them.
6. Behaviour must not change. Add one struct field and log fields only.
7. Files have a hard cap of 700 lines.
8. Read files with the Read tool, because a hook refuses `Get-Content`. Search with Grep.
9. A check gets at most 5 fix rounds.

## Edits

Line numbers are as reviewed at `f08425985`; find each by its content.

**A. `crates/minigame/src/minigame/session_result.rs`** (4 fields)

1. In `pub struct ResultReceipt`, after `pub player_id: i32,`, add:

   ```rust
   /// Character name captured at launch, for logs only.
   pub player_name: Option<String>,
   ```

2. In the one constructor (`let receipt = ResultReceipt {` in `deposit_result_decision`, line 127), after `player_id: session.player_id,` add `player_name: session.player_name.clone(),`. Grep has found no other `ResultReceipt {` in the workspace. Grep again; if the compiler names another, add `player_name: None` there and list it in the report.
3. The `receipt_expired` warn (line 71) becomes:

   ```rust
   tracing::warn!(entity_id = receipt.entity_id,
       entity_name = receipt.player_name.as_deref(),
       player_id = receipt.player_id,
       player_name = receipt.player_name.as_deref(),
       game = %receipt.game_name, reason = "receipt_expired",
       ...
   ```

   Keep the rest as it is.
4. Change the `receipt_capacity` warn (line 145) the same way, with `evicted.` in place of `receipt.`.

**B. `crates/minigame/src/minigame/server/result_dispatch.rs`** (2 fields)

The `result_deposit_refused` warn at line 52 gets two more fields:

```rust
tracing::warn!(entity_id = session.entity_id,
    entity_name = session.player_name.as_deref(),
    player_id = session.player_id,
    player_name = session.player_name.as_deref(),
    game = %session.game_name, phase, reason = "result_deposit_refused",
```

The error at line 64 already shows this shape.

**C. `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`** (8 fields)

- In `start_minigame`, the `no_current_owner` warn (82-87) and the `minigame_url_not_registered` warn (156-163) each get these after `player_id,`:

  ```rust
  entity_name = player_label,
  player_name = player_label,
  ```

  `player_label` is the local from line 61, which the info at lines 62-66 already uses this way. If `player_label` was moved before line 156, use `player_label.clone()` at the earlier site, or whatever the existing line-170 warn does.
- In `minigame_result`, the `result_owner_changed` warn (252-257) and the `result_owner_changed_after_close` warn (277-282) each get these after `player_id,`:

  ```rust
  entity_name = receipt.player_name.as_deref(),
  player_name = receipt.player_name.as_deref(),
  ```

  Both events come before `receipt.on_victory_chains` is moved, at line 295.

**D. `crates/cell/src/cell/service/base_messages/minigame.rs`** (4 fields)

1. Add `use cimmeria_entity::known_names;`.
2. After `receipt_player_id = player_id,` (line 31), add `receipt_player_name = known_names::player_name(player_id),`.
3. After each of the two `expected_player_id = player_id,` lines (44 and 55), add `expected_player_name = known_names::player_name(player_id),`.
4. After `actual_player_id = entity.player_id.unwrap_or(0),` (line 56), add `actual_player_name = known_names::player_name(entity.player_id),`.
5. Leave line 43's `entity_id, // nt:id-only ...` as it is.

## Test (unit, minigame crate)

Create `crates/minigame/src/minigame/server/receipt_deposit_tests.rs`. Declare it in `crates/minigame/src/minigame/server/mod.rs` as `#[cfg(test)]` and `mod receipt_deposit_tests;`, in alphabetical order before `mod session_snapshot_tests;`. FZ-08 adds a second test to this file later.

Copy the setup from `terminal_outcome_delivery_first_keeps_enqueued_victory` in `server/terminal_outcome_tests.rs` (lines 321-348):

- `use super::*;`;
- `SessionRegistry::new()`;
- `register` with its 13 arguments;
- `authenticate_and_claim`;
- `publish_board`. Copy its `published_board` helper too, or make that helper `pub(super)` and call it.

The player name is `register`'s 11th argument, `player_name: Option<String>` (`session.rs` line 154), which is the first `None` in the copied call. Pass `Some("Hacker".into())` there.

The test `deposited_receipt_carries_the_launch_player_name`:

1. Runs the setup above.
2. Asserts `registry.deposit_result(&session, 1, true).await`.
3. Gets the receipt with `registry.redeem_result(&ticket).await.unwrap()`.
4. Asserts `receipt.player_name.as_deref() == Some("Hacker")`.

It fails if the constructor line from A.2 is reverted to `None`. Rule 3 allows these two extra files.

## Checks, in order

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`. If it fails, run `pwsh tools/build-lane/lane.ps1 cargo fmt --all`, then check again.
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-minigame -p cimmeria-base-world-entry -p cimmeria-cell --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-minigame -p cimmeria-base-world-entry -p cimmeria-cell`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server logging::unpaired_id_tests`. This **stays red** until FZ-03a and FZ-03b merge too. Pass condition: none of the four files above appears in its failure list. Paste the remaining list into the report.

## Revert proof

1. Change the A.2 constructor line to `player_name: None,` and rerun the new test: it fails. Restore the line.
2. Remove one added `player_name` log line and rerun check 4: the file reappears in the failure list. Restore it.

Record both runs in the report.

## Commit

```text
fix(logging): name the minigame result-path IDs (FZ-03c)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```
