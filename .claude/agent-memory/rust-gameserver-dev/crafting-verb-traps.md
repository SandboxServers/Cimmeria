---
name: crafting-verb-traps
description: Traps when writing a crafting verb on the induction engine - component-set subset trap, named instances vs design consumption, the non-Send Completion, live tests that pass under the wrong set rule
metadata:
  type: project
---

Learned writing the `craft` verb (2026-09-27, `crates/base-session/src/base/crafting/craft/`).

- **Component sets are often subsets of each other.** 146 seed sets have a sibling set whose designs cover theirs (blueprint 412: set 1 = 14x 5254, set 2 = 5254 + 5256). The craft wire (method 96) carries no set id, only one instance per component. Legacy `Crafter.py` picked the first *covered* set, which charges set 1 for a set-2 submission. Pick the set whose distinct designs *equal* the submitted designs. Blueprint 159 has two sets with identical designs and different quantities: take the first the bags can pay for.
- **A live test can pass under the wrong rule.** If set choice falls through to the next candidate when the first is unaffordable, a set-2 test with too few cores for set 1 passes even with the legacy "covered" rule. The pure rules test with plenty of stock is the real guard.
- **Do not hold a craft to its named instances.** The page names the last stack it found; two queued crafts name the same stack, and the first (crafting bag drains first) deletes it. With `named_items` in the `CraftTransaction`, the second fails `component_missing` although the bags hold enough. `craft` passes no named items; the request-time check already validated them. Reverse engineering is different: it consumes exactly its named instance (`consume_named`).
- **`Completion` is not `Send`-safe to borrow across an await.** It holds `rng: &mut dyn CraftRng`. Pass `done.env` and `&done.ids` to helper futures, never `&done`, or the job future stops being `Send` ("future cannot be sent between threads safely").
- **Re-check knowledge at completion.** Blueprint and discipline are checked at the request; a respec during the bar must not let queued crafts finish. `craft` reloads the state in `complete` and refuses with `reject_at_completion`.

Related: [[game-clock-and-timer-expiry-tests]], [[vacuous-guard-and-sentinel-collision-review]].
