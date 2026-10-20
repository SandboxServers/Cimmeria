# CS-02a — opt-in guarded minigame callback resolver

> **Type:** Reference. **Audience:** Castle packet implementers and reviewers.
> **Date:** 2026-10-14. **Status:** Integrated; dormant helper only.
> **Companions:** [Castle ledger](../README.md), [packet gates](../work-packets.md#cs-0203-conditional-graph-candidates-not-runtime-seals), [test policy](../../../../TESTING.md).

## Purpose and boundary

This packet builds a **dormant, opt-in** cell-content helper for a future
Castle 1520/1521 minigame victory chain. It evaluates a triggerless chain's
conditions against the player's **current** world and mission state before
executing its actions. No Castle callback, seed, minigame result handler or
client behavior changes in CS-02a. Existing `fire_chain_by_id` callers keep
their current behavior; CS-02c will opt in only after the separate CS-02b1/CS-02b2
receipt/identity contract and the playable mission graphs are sealed.

This is a preparatory safety primitive, not a claim that a received result
came from the proper ticket or player. The current Base result path forwards
victory without one-use redemption, and `BaseToCellMsg::MinigameResult` lacks
original player, ticket and game fields. CS-02b1/CS-02b2 must establish those facts
before any Castle callback uses this helper. The helper must not be invoked
from a client-supplied command or used as a generic substitute for result
authentication.

## Reviewed implementation shape

Add `fire_guarded_minigame_chain_by_id` under
`crates/cell-content/src/cell/content/event_dispatch/`. Its inputs are the
requested chain ID, callback entity ID, engine, CellToBase sender and mutable
SpaceManager. Derive the player ID from the **present** entity; require
`is_player=true` and `player_id=Some(nonzero)`. Return without execution on
missing entity or NPC, even if an NPC has a stray `player_id`.

Build `ExecutionContext` with source entity and the existing
`populate_world_context` and `populate_mission_context` helpers. Synthesize
`TriggerEvent { trigger_type: CustomEvent, params.event_name:
"__direct_invoke_<chain_id>" }`, matching the loader's trigger for a chain
with no trigger rows. Resolve with `ChainEngine::resolve_event_filtered` and
admit **only** the requested chain ID. Let normal resolver rules require a
loaded/enabled matching chain and evaluate its conditions. Execute the
resolved actions with the derived player ID; if resolution returns no
actions, return without a fallback to `get_chain_actions` or
`fire_chain_by_id`. This selects a matching synthetic-name CustomEvent
chain; the event resolver cannot prove that the matching event was created
by the loader rather than an explicit DB trigger row. CS-02c must assert
zero trigger rows for every Castle callback in its live-DB guard.
Log a structured guard refusal for missing/non-player/zero-ID entities and
a matched dispatch with chain and entity IDs; normal `content.resolve`
condition logs cover world/mission/step refusal. Do not log a ticket or
account secret. CS-02a emits no mission progress event because it wires no
Castle chain.
Do not change `ChainEngine`, the existing direct-fire function, or unrelated
callback handling in this packet.

The future Castle victory rows must require world 8, mission 1520 active at
step 4453 or mission 1521 active at step 4456, and `delay_ms=0` for every
mission-changing action. A positive delay would defer mutation beyond this
guard and needs a separate revalidation at the deferred drain. CS-02a does
not add those rows or claim their authority checks passed.

## Exact edit allowlist

| File | Permitted change |
|---|---|
| `crates/cell-content/src/cell/content/event_dispatch/mod.rs` | Register and re-export the helper module and test module; leave `fire_chain_by_id` behavior intact. |
| `crates/cell-content/src/cell/content/event_dispatch/direct_invoke.rs` | Add the opt-in helper implementation in a named sibling module. |
| `crates/cell-content/src/cell/content/event_dispatch/direct_invoke_tests.rs` | New in-memory tests for current-state condition resolution, chain-ID filtering and fail-closed entity checks. |
| `crates/cell-content/src/cell/content/mod.rs` | Re-export only the new helper for later explicit use by CS-02c; no caller is switched in this packet. This avoids an unreachable public item in the private dispatcher module. |
| `docs/content/content-engine-vocabulary.md` | Document the new opt-in callback seam, original direct-fire behavior and receipt limitation. |
| `docs/analysis/castle-continuation/README.md`, `RESUME.md`, `worknotes/CS-02a.md` | Record implementation/review/test result and remaining CS-02b1/CS-02b2/CS-02c gates. |
| `.claude/agent-memory/main-session/reference_castle_continuation_evidence_2026_10_14.md`, `MEMORY.md` | Update only if implementation reveals a reusable fact missing from the docs. |

Do not edit Base/Cell message schemas, the existing minigame result handler,
seed rows, ticket registry, client files or an unrelated callback in this
packet. If the helper needs one of those, stop and amend this contract with
authority review before continuing. No new chain ID is claimed here.

Review amendment, 2026-10-14: independent packet review of `84a6348ed`
found the inline helper raised `event_dispatch/mod.rs` from 680 to 746 lines,
above the 700-line limit in `CLAUDE.md`. The exact allowlist now includes
`direct_invoke.rs` solely to move the existing helper body out of `mod.rs`.
This is a file-organization fix; behavior and acceptance tests stay sealed.

## Dispatch seal

Feature code base at seal: `24edb0aeae8086441d8fe4c77d0196ac0fbd56b9`.
The packet branch is `castle/cs-02a-guarded-callback`, worktree
`.claude/worktrees/castle-cs-02a`, and its isolated database is
`sgw_castle_cs_02a`. The worker must start from the later **exact campaign
HEAD recorded by the coordinator immediately before dispatch**, which also
contains this contract. Verify the new branch/worktree is clean, its HEAD
equals that recorded SHA, and its merge-base with `content/castle-campaign`
equals that SHA. `mk-worktree.ps1` starts at `origin/main`, so use the
execution contract's empty-branch rebase onto the recorded feature SHA and
stop on any mismatch. The coordinator records the actual SHA and command in
this worknote before sending the worker brief.

Expected coder size: three Rust files, one helper and focused unit tests,
with no seed, schema, network or UI edit. Estimated **12–18k working tokens**
plus about 35–40k fixed startup context, below the 90k cap and the campaign
target of about 40k working tokens. Split and re-review if this estimate
fails in implementation. Commit subject:
`feat(castle): add opt-in guarded minigame callback resolver`.
The coder owns only the allowlist above, reports one result, and does not
push or open a PR. The independent review/fixer and integration wave gates
remain with the coordinator.

Dispatch base verification, 2026-10-14: `mk-worktree.ps1` created the clean
`castle/cs-02a-guarded-callback` branch at `origin/main`
`817d3792c8d312652c5f67ecc242100f0bef48cd`. In the packet worktree,
`git rebase --onto 899fcee60a0c4cd17b15ca59b86db1cbf995664f
origin/main castle/cs-02a-guarded-callback` succeeded. Its `HEAD` and
`git merge-base HEAD content/castle-campaign` both equal
`899fcee60a0c4cd17b15ca59b86db1cbf995664f`, and `git status --short`
is empty. This is the **exact worker base SHA**. The later record commit in
the integration branch is coordinator documentation; the worker starts from
this verified SHA and does not pull that later commit into its packet.

## Acceptance tests and revert proof

Pick in-memory unit tests under `TESTING.md`; no live DB or client test is
needed for this dormant helper. The positive fixture should register a
triggerless synthetic callback with conditions world 8, mission 1520 active
and step 4453 active, then observe exactly one action. Change the same
player to step 4454 before invocation and assert no action; also test wrong
world, inactive mission and another player's active step. Use a second chain
with the same synthetic event name and assert the requested-ID filter keeps
its action out. An otherwise matching **disabled** chain must not execute;
the old `get_chain_actions` path ignores its enabled flag.

For missing entity, a player with `player_id=Some(0)`, and an NPC with stray
`player_id=Some(100)`, use an action such as `GrantItem` that the old executor
would send even with player ID 0, then assert **no** CellToBase message. A
state-only action could no-op by itself and make a false pass.
Test an unconditioned, triggerless chain through the new helper to pin
compatibility. Keep existing `fire_chain_by_id` and Cell handler tests
unchanged: CS-02a does not switch their call path. Verify a chain with a
real non-synthetic trigger cannot be invoked through this helper.

After the worker commits the fix and tests, remove only the helper's
condition-aware resolution while leaving tests intact: the wrong-step test
must execute the action and fail. Restore only the touched implementation
file and rerun. Independently remove the player-presence check: the missing
entity/NPC message assertion must fail. Record commands, exit codes, failed
assertions and restored passes here. Do not use a test that only mirrors the
helper's return value.

Focused build-lane checks:

```powershell
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-cell-content --lib direct_invoke -- --nocapture
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
```

`cargo fmt` does not compile; every compiling `cargo` invocation above goes
through the lane. CS-02b1/CS-02b2 together own server-produced result redemption,
Cell incarnation and original player identity. CS-02c owns handler wiring, actual Castle seed, live-DB
chain replay, forged/cancel/replacement negatives and client UAT. None is
credited by passing CS-02a.

## Implementation and verification (2026-10-14)

The opt-in `fire_guarded_minigame_chain_by_id` now derives a nonzero player ID from the present player entity, populates current world/mission context, resolves only the requested synthetic-name CustomEvent chain, and executes only resolved actions. Existing direct-fire callers are unchanged. The helper is re-exported for a later explicit CS-02c opt-in. No Castle callback, seed, schema, ticket registry or client path changed.

Focused in-memory tests cover one action in world 8 with mission 1520 active at step 4453, refusal after a step 4454 transition, wrong world, completed mission, another player's wrong step, same-name other chain ID, disabled chain, a real non-synthetic trigger, unconditioned chain parity, and missing/NPC/zero-ID entities with unconditional GrantItem.

| Check | Result |
|---|---|
| `pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content` | `status=ok exit=0` (format applied after initial `--check` diff; final `--check` also `status=ok exit=0`). |
| `pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings` | `status=ok exit=0`. |
| `pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo test -p cimmeria-cell-content --lib direct_invoke -- --nocapture` | `status=ok exit=0`; 4 passed, including final rerun after both restorations. |
| Temporary replacement of only the helper's resolver with `get_chain_actions`, then `cargo test ... direct_invoke_resolves_current_world_mission_and_step` through the lane | `status=failed exit=101`; assertion `wrong step ran the action`, observed counter 2 instead of 1. Restored the implementation file. |
| Temporary removal of only the helper's player-presence guard, then `cargo test ... direct_invoke_requires_present_nonzero_player` through the lane | `status=failed exit=101`; assertion `entity 9999 emitted GrantItem`. Restored the implementation file. |

The helper alone does not authenticate a minigame result or redeem a one-use ticket. CS-02b1/CS-02b2 together own result receipt, Cell incarnation and original-player identity. CS-02c must wire only after that contract, assert zero explicit trigger rows for the callback, enforce world/mission/step gates with immediate mission-changing actions, and perform live-DB replay and client UAT. No live DB or client result is credited to CS-02a.

## Independent review fix (2026-10-14)

- Fixed: the inline helper had raised `event_dispatch/mod.rs` to 746 lines, above the 700-line repository limit. The unchanged helper body now lives in `event_dispatch/direct_invoke.rs`; `mod.rs` is 684 lines.
- `cargo fmt --all -- --check`: `status=ok exit=0` after applying the reported module order.
- `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`: `status=ok exit=0`.
- `cargo test -p cimmeria-cell-content --lib direct_invoke -- --nocapture`: `status=ok exit=0`, four passed.

## Integration (2026-10-14)

Independent packet review found only the 700-line file limit violation;
the amended allowlist and fixer commit `8f09c1eb8` resolved it. The packet
branch merged into `content/castle-campaign` at `fff93c150`. The integrated
branch reran `cargo test -p cimmeria-cell-content --lib direct_invoke --
--nocapture` through the build lane: `status=ok exit=0`, four passed. The
helper remains opt-in and unused by the current minigame result handler.
