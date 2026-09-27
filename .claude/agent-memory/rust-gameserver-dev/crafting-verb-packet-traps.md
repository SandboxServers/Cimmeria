---
name: crafting-verb-packet-traps
description: Wiring a crafting verb (craft/research/reverse engineer/alloy/respec) onto the base - stub-pinning dispatch tests, testable handler shape, the alloy page's 10-slot cap
metadata:
  type: project
---

Learned wiring `alloying` (CR-09, branch craft/cr09-alloy, 2026-09-27):

- Replacing a verb's "not available yet" stub breaks tests in OTHER crates that pin the stub line:
  `crates/base-world-entry/.../cell_dispatch/tests_dispatch_arms/crafting_arm.rs` (sends an Alloy
  request) and `crafting_gate.rs` (sends a Craft request, expects "Crafting is not available yet.").
  With `db_pool: None` a real verb answers `Unavailable` ("<Action> is unavailable right now. Nothing
  was changed.", reason `unavailable`), so update the expected line and reason there.
- Testable shape: `handle_<verb>(…)` calls `handle_<verb>_in(crafting_sessions(), …)`; tests pass
  their own `Arc<CraftingSessions>` with a `ManualScheduler` and fire `scheduler.take()` +
  `expire_at` to finish the bar. `CraftCtx` is built from borrowed fixture fields.
- The alloy page has exactly 10 elementary slots (`AlloyPage.lua:467-468`); the base drops a longer
  list as `malformed`. Live tests that send 11+ ids get silently dropped -- use stacks with
  `stack_size > 1` (the rule counts stack quantity) to reach the counts within 10 ids.
- Tier-1 elementary fixtures that sit in carried bags: Normal 5188, Good 5189, Great 5395,
  Fantastic 2891 (bag 1 only), Poor 2492. All max stack 1 in the seed.

**Why:** CR-07/CR-08/CR-10 hit the same seams.
**How to apply:** grep `not available yet` across crates before landing a verb; follow the
`alloy/` layout (rules.rs pure, job.rs induction, mod.rs inputs + answer). See
[[crafting-induction-engine-seams]].
