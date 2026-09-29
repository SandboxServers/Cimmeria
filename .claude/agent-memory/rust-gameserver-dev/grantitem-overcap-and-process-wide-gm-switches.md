---
name: grantitem-overcap-and-process-wide-gm-switches
description: GrantItem writes one over-cap row when count doesn't merge (#1045); stack-respecting grants use AmmoReserve::return_rounds; GM switches readable below cell-console live in cimmeria-entity as player_id-keyed statics
metadata:
  type: project
---

`CellToBaseMsg::GrantItem` → `persist_grant` inserts ONE row with `stack_size = count` when the count doesn't fit an existing stack, so a count above `max_stack_size` makes an over-cap stack (issue #1045, filed 2026-09-28 during ammo AM-06). The merge path is capped; the fresh-slot path is not.

**Why:** found when `.giveammo` needed 500-round capped stacks.

**How to apply:** for a stack-respecting grant of a reserve-ammo type, go through `inventory::ammo_reserve::return_rounds` inside a transaction (as `ammo_gm_give::grant_rounds` does). It tops up existing stacks, opens capped ones and reports `remainder`. Don't chunk GrantItem from the cell: the cell doesn't know `max_stack_size`.

GM switches that a lower crate must read (such as `bInfiniteAmmo`, which AM-02's reload in cell-combat reads) can't live on `SpaceManager` during a campaign that freezes it, and they can't live in cell-console either. The pattern is a `player_id`-keyed process-wide set in `cimmeria-entity` (`ammo_infinite.rs`). Like `gm_aggro_off`, it holds until a restart. In tests use a unique sentinel player id per test, because the set is shared under `cargo test`.
