# UD-04b and UD-04c: crossing handshake tests

> **Type:** How-to (work packets) · **Wave:** U2 · **Status:** BlockedDependency
> **Index:** [dakara-work-packets.md](../dakara-work-packets.md)

These tests prove that each departure leaves the player in exactly the state the receiving chain gates on. They live in one Union module, so neither DK nor SGC has to own a cross-campaign test. They seed nothing.

## Contract excerpt (crossings, from UD-00's `crossings-contract.md`)

| X | Departure chain (owner) | State after departure | Receiving chain gate (owner) |
|---|---|---|---|
| C1 | SGC US-24 departure | 1570/4906 active | DK-10b rows 8046-8056, `archetype lte 6` + step 4906 |
| C2 | DK-30 departure | 1651/4928 active | SGC US-30 Hammond chain at 4928 |
| C2b | SGC US-30 departure | 1651/5171 active, item 6784 held | DK-30 Bra'tac chain at 5171 |
| C3 | DK-33 departure | 1654/4939 active, objective 5961 complete, 5962 open | SGC US-31 Hammond 5852 (Human) / 5853 (Jaffa) |

Read the frozen contract before starting. If it differs from this excerpt, the contract wins. Note the difference in the report.

## Rules (binding)

The ten rules in [UD-04a](UD-04a.md#rules-binding-packet-coder-skips-claudemd), with `ud-04b` / `ud-04c` as the worktree, plus:

- Do not edit any DK or SGC seed or test file. If a gate is missing or wrong, the test fails and you return `blocked` naming the owning packet. Do not fix the other side.
- Find the real chain IDs by grepping the merged seed files for the step IDs. Never hard-code a chain ID that isn't in a merged seed.

## Test shape (both packets)

For each crossing:

1. Build the departing context: archetype, world, and the mission at the departing step. Use `ctx_for`, `with_mission` and `with_step` as in `chain_replay_tests/mission_1324.rs:92-123`.
2. Resolve the departure trigger. Assert the action list contains the advance (or `complete_objective` for C3) **before** `Action::CrossWorldTeleport { world_name, position }`, with the contract's world and position.
3. Apply the resulting mission state by hand: the step after the advance, or the objective statuses.
4. Build the receiving context on the arrival world from that state. Resolve the receiving trigger. Assert the receiving chain resolves.
5. Negative: the same receiving trigger with the **pre-departure** state resolves nothing. This proves the order matters.

Why it fails on revert: if either side changes its step, gate, world name or position, the matching assertion breaks.

## UD-04b: crossing C1 (packet-coder, about 30k)

| Field | Value |
|---|---|
| Depends on | DK-10b merged; SGC US-24 merged |
| Branch / worktree | `union-dakara/ud-04b-c1-handshake` / `ud-04b` |
| Files | New `crates/cell-content/src/cell/content/chain_replay_tests/union_dakara_crossings.rs`; `mod.rs` (one line) |
| Commit subject | `test(content): 1570 SGC-to-Dakara crossing handshake (UD-04b)` |
| Tests | `live_db_c1_sgc_departure_lands_a_human_where_dk10b_receives`, for archetypes 1 to 6; `live_db_c1_receiver_refuses_the_pre_departure_state` |

## UD-04c: crossings C2, C2b and C3 (packet-coder, about 35k)

| Field | Value |
|---|---|
| Depends on | DK-30, DK-33, SGC US-30 and US-31 merged; UD-04b merged (same file) |
| Branch / worktree | `union-dakara/ud-04c-1651-1654-handshake` / `ud-04c` |
| Files | `union_dakara_crossings.rs` (extend; stay under 500 lines, or split into a `union_dakara_crossings/` folder with `mod.rs`, `c1.rs`, `c2.rs`, `c3.rs`) |
| Commit subject | `test(content): 1651 and 1654 crossing handshakes (UD-04c)` |
| Tests | One positive and one pre-state negative per crossing. For C2b, also check that item 6784 is granted at 4931 (assert the SGC chain's `add_item` action). For C3, assert the departure has **no** `AdvanceStep` for 1654 (D-UD7). Run both archetype variants of C3 (Human 5852, Jaffa 5853). |

## Lane (both)

```powershell
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
Set-Location <worktree>; pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::union_dakara_crossings
```

Check that the test count is above zero.

## Acceptance and telemetry

- Every test passes. A one-line sabotage of either side (comment out the advance) makes the matching test fail; record it in the report.
- SigNoz events each crossing must emit live (checked in UD-05, not here): `Content: cross-world teleporting entity` with the departing `chain_id` and the target `world`; then `Content: advancing step` (C1, C2, C2b) or `Content: complete objective` (C3) from the same chain, logged before it.

## Docs rows

None beyond the ledger. UD-99 writes the content docs.
