---
name: project-craft-verb-named-instance-vs-design-consume
description: cr07's `craft` verb (CraftTransaction) validates named item_ids exist at completion but consumes by design/SUM, not by instance — a stale-instance false rejection is possible across a player's own queued jobs
metadata:
  type: project
---

Reviewed 2026-09-27 (read-only, worktree `.claude/worktrees/cr07`, branch `craft/cr07-craft`) for the crafting-campaign `craft` verb (cell method 96, issue tracked under crafting campaign — see [[project_crafting_campaign]]).

**Shape of the code:**
- `crates/base-session/src/base/crafting/craft/rules.rs` `plan_craft()` always sets `CraftTransaction.consume_named = Vec::new()` and puts every submitted instance into `named_items`. Actual consumption is entirely by `(design_id, quantity)` via `transaction/consume.rs::consume_design`, which sums `stack_size` across `CRAFTING_INPUT_BAGS = [INV_CRAFTING(15), INV_MAIN(1)]` and drains stacks in bag order — never keyed to the specific `item_id` the client named.
- But `transaction/mod.rs:94` (`apply_in_tx`) still calls `consume::check_named_items` on every named instance before consumption. That function (`consume.rs:50-99`) requires the *literal* `item_id` row to still exist, still belong to the player, still be the right `type_id`, and still sit in bags 1/15 — or it fails the whole job with `ComponentMissing` (consume.rs:70-75).

**The gap:** crafting jobs run strictly serially per player (`session/state.rs` — one `active` induction, rest queued in a `VecDeque`), each taking `INDUCTION_DURATION` (~3s). If job A (queued first) fully drains the exact stack (`item_id`) that job B's request also named — because at request time both jobs independently picked "the last stack found" for the same design and that stack later gets exhausted — job B's completion-time `check_named_items` throws `ComponentMissing` even if the player's bags hold *plenty* of that design from a stack acquired between A and B (a new `item_id` from looting/vendor/another merge). Contrast: if A only partially drains the shared stack (UPDATE not DELETE), job B correctly falls through to `consume_design`'s own `NotEnoughComponents` check, which is accurate. Only the full-delete case produces a spurious/wrongly-labeled failure.

Confirmed no existing test exercises this path — `transaction/tests/named.rs` only covers the RE-style exact-consumption (`consume_named` non-empty) cases, not the craft-style all-by-design case with two sequential jobs.

**Why it's not the classic double-consume trap:** this isn't `remove_item`-adjacent-to-UseInventoryItem — crafting isn't a content-engine chain here, it's a self-contained DB transaction. It's a narrower design-vs-instance mismatch specific to how `craft` populates `named_items` for a check that only matters for exact (`consume_named`) consumption.

**Resolved in the same branch (2026-09-27):** `plan_craft` now leaves `named_items` empty. The named instances only choose the component set at the request; the completion consumes by design, so a queued craft whose named stack an earlier job drained still runs. Guard: `craft::tests::live::a_craft_behind_another_is_queued_and_told` (fails with `named_items` populated). The recommendation below is kept for context.

**Recommendation given at review time:** either (a) accept as a rare, safe-but-annoying edge case (no corruption — the reject path fully resyncs inventory) and add a regression test documenting the accepted behavior, or (b) skip `check_named_items` entirely for jobs whose `consume_named` is empty, since `consume_design`'s own existence+quantity check already fully re-validates ownership/location/amount without needing the literal instance to survive.

**Container map confirmed while reviewing** (`crates/cell-catalog/src/item_placement.rs`): `INV_MAIN = 1`, `INV_CRAFTING = 15`, storage `17..=20`. Container 1 is the main bag only — equipped items live in a separate container, so bag-1 crafting consumption cannot accidentally eat an equipped item. `sgw_inventory` (`db/sgw/Inventory/Tables/sgw_inventory.sql`) carries `bound`, `durability`, `charges`, `ammo`, `cur_ammo_type` columns; crafting's `consume_design`/`consume_instance` touch only `stack_size` and correctly ignore `bound`/`durability`/`charges` since components are stackable materials, not gear.
