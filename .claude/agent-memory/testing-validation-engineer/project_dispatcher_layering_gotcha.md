---
name: gotcha-inner-dispatch-bypasses-outer-router
description: Tests that call a cell-methods submodule's dispatch directly do NOT exercise the outer router's range, even if the docstring claims they do
metadata:
  type: project
---

In `crates/services/src/cell/cell_methods/player/`, dispatch is layered:
- `dispatch.rs::dispatch` is the **outer** router (takes 6 args including `&ChainEngine`). It matches against constant ranges (`CALL_FOR_AID..=RESET_MY_ABILITIES`, `ORG_CREATION..=CANCEL_MOVIE`, etc.) and routes to submodule dispatchers.
- Each submodule (`crafting.rs`, `social.rs`, `combat.rs`, `world.rs`, ...) has its own `dispatch` function (typically 5 args, sometimes 6). These are the **inner** dispatchers — invoked by the outer router based on the range arm.

**Why:** Routing bugs almost always live in the *outer* router (wrong range, wrong arm order). The inner dispatchers usually have stable match arms once they exist. A test like `dispatch(1, X, &args, &tx, &mut mgr).await` written inside a submodule's `#[cfg(test)] mod tests { use super::*; ... }` block calls **the submodule's dispatch directly**, completely bypassing the outer router.

**How to apply:** When auditing a PR that claims to fix a routing/range bug in `dispatch.rs`:

1. Find the test. Check whether the `dispatch(...)` call has 5 or 6 arguments. 5-arg means it's calling the inner submodule's dispatch (testing arm coverage, not routing). 6-arg with `&ChainEngine` means it's calling the outer router (testing routing).
2. Look at the test's imports: `use super::*` inside `submodule/mod.rs` brings in that submodule's `dispatch`. To call the outer router, the test must be in `dispatch.rs`'s test module or import explicitly with `use super::super::dispatch::dispatch`.
3. To pin an outer-router fix, the test MUST live in `dispatch.rs`'s test module. The strong-shape test calls `outer::dispatch(1, METHOD, ...)` and asserts on *side effects that distinguish which inner branch was taken* — not on `handled: bool`, because multiple inner dispatchers may legitimately handle the same method index (see [[gotcha-social-arm-shadows-crafting-95]]).

The canonical pattern for "right submodule routed" is `LogCapture::install()` + assert the submodule-distinctive log message fired. The negative-logging convention doc (`docs/architecture/negative-logging-convention.md`) describes the field-naming pattern.
