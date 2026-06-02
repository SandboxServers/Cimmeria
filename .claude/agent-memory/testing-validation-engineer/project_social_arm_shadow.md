---
name: gotcha-social-arm-shadows-crafting-95
description: cell::cell_methods::player::social::dispatch has a SPEND_APPLIED_SCIENCE_POINTS arm (95) that returns true — making bool-based routing tests for crafting/social split invisible
metadata:
  type: project
---

`crates/services/src/cell/cell_methods/player/social.rs:66` has an arm:
```
SPEND_APPLIED_SCIENCE_POINTS => {
    if args.len() >= 4 { ... }
    tracing::info!(entity_id, discipline_seq_id, "UNIMPLEMENTED: spendAppliedSciencePoints");
    true
}
```

This is a stub left from an earlier wiring attempt before the crafting submodule got its own arm. As of PR #427 it has not been removed.

**Why this matters:** PR #427 fixed a routing bug where the outer router's range `(CRAFT..=RESPEC_CRAFTING)` (96..=100) dropped index 95 into the social arm. The "fix" widened it to `(SPEND_APPLIED_SCIENCE_POINTS..=RESPEC_CRAFTING)` (95..=100). But because **both** the crafting submodule AND the social submodule have arms for index 95 that return `true`, **any test asserting `handled == true` is blind to which branch was taken**. You can revert the fix and `assert!(handled)` still passes — the wrong handler runs, but it still returns true.

**How to apply:**

1. Any future cell-routing test for index 95 (and any other index where multiple submodules might have stub arms) must use `LogCapture` to assert WHICH submodule fired the log, not just whether `handled` was true.
2. The crafting-side log is `"UNIMPLEMENTED: spendAppliedSciencePoints (Phase 2)"` (the `(Phase 2)` suffix is the distinguishing marker). Social's is `"UNIMPLEMENTED: spendAppliedSciencePoints"` (no suffix). Use `LogCapture::install().find_message(Level::INFO, "(Phase 2)")` to pin the crafting branch.
3. As a follow-up clean-up issue worth filing: remove the social-side `SPEND_APPLIED_SCIENCE_POINTS` arm. It's dead code now that crafting owns the routing. Doing so would make a simple `assert!(handled)` test sufficient for the outer-router check.
4. There may be other "shadowed" arms in this dispatcher tree. When auditing a routing-fix PR, grep the other submodule dispatchers for the same method constant to check for shadows.

Reference: pushed `outer_dispatch_routes_spend_asp_to_crafting_not_social` in commit `fc821bd6` (PR #427) as the canonical example of a LogCapture-based routing pin.

Related: [[gotcha-inner-dispatch-bypasses-outer-router]]
