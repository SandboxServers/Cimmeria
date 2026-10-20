# FZ-03a: Rule 6 names on the base side of the letter turn-in

- **Wave:** 2. **Depends on:** FZ-01 merged.
- **Fixes:** F1, 30 of the 87 fields. FZ-03b fixes 39 and FZ-03c fixes 18.
- **Agent:** packet-coder. About 30k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03a`.
- **Branch:** `four-zone-review/FZ-03a-base-names`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03a\.claude\reports\four-zone-review\FZ-03a-coder.md`
- **Return (one line):** `FZ-03a <done|blocked|failed> <sha|-> <report path>`

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03a;`.
2. Every compiling `cargo` call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never run `cargo` directly. Logs land in `$env:LOCALAPPDATA\cimmeria-build\logs\fz-03a\`.
3. Edit only the four files below. These are forbidden: `git stash`, `git worktree prune`, fetch, push, rebase and `git add -A`.
4. **Logging Rule 6.** Every `*_id` field on a `tracing` event has its `*_name` partner on the same event. The pairs:
   - `entity_id` with `entity_name`;
   - `player_id` with `player_name`;
   - `mission_id` with `mission_name`;
   - `current_step_id` with `current_step_name`;
   - for a field with no name at all, put `// nt:id-only <reason of two or more words>` at the end of that field's own line, with one ID field per marked line.

   A `None` name is fine: the field is then omitted. Never set `NT_BASELINE_BLESS`.
5. Name sources:
   - `cimmeria_entity::known_names::player_name(player_id)` returns `Option<&'static str>`, and accepts `i32`, `u32` or an `Option` of them;
   - `cimmeria_names::book().mission(id)` and `.mission_step(id)` return `Option<&str>`.

   Both crates are already dependencies of every crate here. Add `use cimmeria_entity::known_names;` where a file lacks it.
6. Behaviour must not change. Add fields only: no new branches, no level changes, no message text changes.
7. Files have a hard cap of 700 lines.
8. Read files with the Read tool, because a hook refuses `Get-Content`. Search with Grep.
9. A check gets at most 5 fix rounds.

## Edits

Line numbers are as reviewed at `f08425985`. Find each by its content, because the merge may shift them by a few lines.

**A. `crates/base-methods/src/base/world_entry/methods/missions/letter_turn_in_route.rs`** (16 fields)

- The flagged lines are 100/101, 125/126, 137/138, 168/169, 178/179 and 189/190, which use `request.*`, and 211/212 and 220/221, which use `expected.*`.
- Directly after each `player_id = request.player_id,`, add these two lines:

  ```rust
  entity_name = known_names::player_name(request.player_id),
  player_name = known_names::player_name(request.player_id),
  ```

- In `send_result_if_current`, use `expected.player_id` in place of `request.player_id`. The last field there has no trailing comma, so add one.
- The entity is the player's avatar, so `entity_name` is the player name. Line 210 of `cell_handoff.rs` sets the same precedent.

**B. `crates/base-methods/src/base/world_entry/methods/missions/mod.rs`** (7 fields, in the `if mission_id == 1360 {` block around lines 194-215)

`player_name` is already a local; line 171 uses it.

- In each of the three arms, after `player_id,` add `player_name,`.
- After `mission_id,` add `mission_name = cimmeria_names::book().mission(mission_id),`.
- In the `Ok(false)` arm, after `current_step_id,` add `current_step_name = current_step_id.and_then(|s| cimmeria_names::book().mission_step(s)),`. `current_step_id` is an `Option<i32>`.

**C. `crates/base-session/src/base/helpers/letter_turn_in_send.rs`** (4 fields, lines 248/249 and 268/269)

After each `player_id = expected.player_id,`, add:

```rust
entity_name = known_names::player_name(expected.player_id),
player_name = known_names::player_name(expected.player_id),
```

**D. `crates/base-world-entry/src/base/world_entry_appearance/client_ready/cell_handoff.rs`** (3 fields)

- At lines 194/195 (the `LetterTurnInSession` send failure), make the event carry the same identity as the `ConnectEntity` error at lines 165-171:

  ```rust
  entity_id = ctx.entity_id,
  entity_name = ctx.player_name,
  player_id = ctx.player_id,
  player_name = ctx.player_name,
  ```

- At line 238 (`region_id,` in the `AdvanceRingDestination` error), change it to `region_id, // nt:id-only ring region index, the base holds no region names`.

## Checks, in order

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base-methods -p cimmeria-base-session -p cimmeria-base-world-entry --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-methods -p cimmeria-base-session -p cimmeria-base-world-entry`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server logging::unpaired_id_tests`. This **stays red** until FZ-03b and FZ-03c merge too. Pass condition: none of the four files above appears in its failure list. Paste the remaining file list into the report.

## Revert proof

Check 4 is the guard. Remove one `player_name = ...` line you added, and the file reappears in the failure list. Record that run in the report, then restore the line.

## Commit

```text
fix(logging): name the base-side letter turn-in IDs (FZ-03a)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```
