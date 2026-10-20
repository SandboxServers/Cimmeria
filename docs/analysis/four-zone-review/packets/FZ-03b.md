# FZ-03b: Rule 6 names on the cell side of the letter turn-in

- **Wave:** 2. **Depends on:** FZ-01 merged.
- **Fixes:** F1, 39 of the 87 fields. FZ-03a fixes 30 and FZ-03c fixes 18.
- **Agent:** packet-coder. About 35k working tokens.
- **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03b`.
- **Branch:** `four-zone-review/FZ-03b-cell-names`.
- **Report:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03b\.claude\reports\four-zone-review\FZ-03b-coder.md`
- **Return (one line):** `FZ-03b <done|blocked|failed> <sha|-> <report path>`

## Rules (inlined; you do not read CLAUDE.md)

1. Use PowerShell only, with absolute paths. Start every call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\fz-03b;`.
2. Every compiling `cargo` call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never run `cargo` directly. Logs land in `$env:LOCALAPPDATA\cimmeria-build\logs\fz-03b\`.
3. Edit only the four files below. These are forbidden: `git stash`, `git worktree prune`, fetch, push, rebase and `git add -A`.
4. **Logging Rule 6.** Every `*_id` field on a `tracing` event has its `*_name` partner on the same event. The pairs:
   - `entity_id` with `entity_name`;
   - `player_id` with `player_name`;
   - `chain_id` with `chain_name`;
   - `marsh_entity_id` with `marsh_entity_name`;
   - `marsh_id` with `marsh_name`;
   - `item_id` with `item_name`.

   Put each ID field on its own line. A `None` name is fine. Never set `NT_BASELINE_BLESS`.
5. Name sources. Both crates are already dependencies of `cimmeria-cell-content` and `cimmeria-cell-world`.
   - `cimmeria_entity::known_names::player_name(id)` returns `Option<&'static str>`, and accepts `i32`, `u32` or an `Option` of them. It holds no borrow, so it is safe next to `&mut SpaceManager` and across `.await`.
   - `cimmeria_names::book().chain(id)` and `.item(id)` return `Option<&'static str>`.
   - `SpaceManager::entity_label(id)` returns `Option<&str>`, borrowed from the manager.
6. Behaviour must not change. Add fields and one function parameter only: no level changes, no message text changes.
7. Files have a hard cap of 700 lines.
8. Read files with the Read tool, because a hook refuses `Get-Content`. Search with Grep.
9. A check gets at most 5 fix rounds.

## Edits

Line numbers are as reviewed at `f08425985`; find each by its content.

**A. `crates/cell-content/src/cell/content/executor/letter_turn_in.rs`** (17 fields)

1. Add `use cimmeria_entity::known_names;` to the imports, next to `use cimmeria_entity::missions::...`.
2. `fn feedback` (line 92):
   - change the signature to `fn feedback(entity_id: u32, player_id: i32, text: &str, tx: &mpsc::Sender<CellToBaseMsg>)`;
   - change its warn (line 98) to:

     ```rust
     tracing::warn!(entity_id, entity_name = known_names::player_name(player_id),
         reason = "feedback_channel_unavailable", %error,
         "Frost letter feedback could not be queued");
     ```

   - update all six callers in `request` (lines 157, 178, 194, 209, 242 and 246) to `feedback(entity_id, player_id, <TEXT>, tx)`. `player_id` is a parameter of `request`.
3. In `send_request`, the queued info (lines 107-112) becomes:

   ```rust
   entity_id = pending.request.entity_id,
   entity_name = known_names::player_name(pending.request.player_id),
   player_id = pending.request.player_id,
   player_name = known_names::player_name(pending.request.player_id),
   marsh_entity_id = pending.marsh_entity_id,
   marsh_entity_name = MARSH_TAG,
   ```

   In the not-queued warn (lines 116-118), add the same `entity_name` and `player_name` lines.
4. In `request`:
   - **The `invalid_action` warn (151-156).** After `entity_id,` add `entity_name = known_names::player_name(player_id),`. After `chain_id,` add `chain_name = cimmeria_names::book().chain(chain_id),`.
   - **The three `... click refused` infos (187-193, 202-208, 235-241).** Each one gets these after `chain_id,`:

     ```rust
     entity_name = known_names::player_name(player_id),
     player_name = known_names::player_name(player_id),
     chain_name = cimmeria_names::book().chain(chain_id),
     ```

**B. `crates/cell-content/src/cell/content/letter_turn_in_result.rs`** (15 fields)

1. Add `use cimmeria_entity::known_names;`.
2. **In `remove_letter_binding`, the warn at line 121.** Split it into one field per line, and name both entities:

   ```rust
   tracing::warn!(
       entity_id,
       entity_name = space_mgr.entity_label(entity_id),
       marsh_id,
       marsh_name = space_mgr.entity_label(marsh_id),
       %error,
       "Frost letter interaction update not queued"
   );
   ```

   The `.await` has already finished at that point, so the shared borrow is fine. If the borrow checker refuses it, compute both labels as owned `String`s (`.map(str::to_owned)`) before the `tx.send(...)`.
3. **In `apply_letter_turn_in_result`.** After `let entity_id = result.request.entity_id;` (line 133), add `let player_name = known_names::player_name(result.request.player_id);`. Then make these additions:
   - lines 142-147, 152-156, 164-169 and 194-199: after each `player_id = result.request.player_id,`, add `entity_name = player_name,` and `player_name,`;
   - line 188 (`tracing::warn!(entity_id, %error, ...)`): add `entity_name = player_name,` after `entity_id,`;
   - lines 204-208 (`mission_tracker_locked`): add `entity_name = player_name,` after `entity_id,`;
   - lines 218-225: add `entity_name = player_name,` and `player_name,` after the `player_id` line, and `item_name = cimmeria_names::book().item(committed.item_id),` after the `item_id` line.
4. Check the rest of the file with Grep for `_id` fields in `tracing::` events, from line 238 to the end. If one is unpaired, pair it the same way, and list it in the report.

**C. `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs`** (6 fields)

In each of the three events (lines 37-42, 65-70 and 73-78), add these after `entity_id,`:

```rust
chain_name = cimmeria_names::book().chain(chain_id),
entity_name = space_mgr.entity_label(entity_id),
```

**D. `crates/cell-world/src/cell/space_manager/phase.rs`** (1 field)

In `set_player_phase`'s `phase.changed` info (line 78), add `player_name = self.entity_label(player_id),` after `player_id,`. `events` is an owned `Vec` and the `get_entity_mut` borrow has ended, so `&self` is free.

## Checks, in order

1. `pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check`. If it fails, run `pwsh tools/build-lane/lane.ps1 cargo fmt --all`, then check again.
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content -p cimmeria-cell-world`
4. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-server logging::unpaired_id_tests`. This **stays red** until FZ-03a and FZ-03c merge too. Pass condition: none of the four files above appears in its failure list. Paste the remaining list into the report.

## Revert proof

Check 4 is the guard. Remove the `chain_name` line from one `direct_invoke.rs` event and rerun: `direct_invoke.rs` reappears in the failure list. Record that run in the report, then restore the line.

## Commit

```text
fix(logging): name the cell-side letter turn-in IDs (FZ-03b)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_012n1ut2hRgmMP5C6ZXG9HEV
```
