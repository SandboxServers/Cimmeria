---
name: position-validation-finite-gate
description: Position validators must use `is_finite()`, not just `is_nan()` — and the regression test for it has to use an unbounded AABB
metadata:
  type: project
---

`position_within_bounds` in `crates/entity/src/movement_validation.rs` rejects non-finite coordinates (`NaN`, `+Infinity`, `-Infinity`) via an explicit `if !pos.x.is_finite() || !pos.y.is_finite() || !pos.z.is_finite()` gate before the AABB comparisons. **Don't** rely on the comparison conjunction alone — it works incidentally today but is fragile.

**Why:** every `>=` / `<=` against NaN is false (so NaN gets rejected for free), and `+Inf <= max` is false for finite `max` (so `+Inf` also gets rejected for free against the current finite `SpaceBounds::FALLBACK`). Both "for free" rejections vanish the moment a future contributor widens `max` toward `f32::MAX` or `f32::INFINITY`. The explicit `is_finite()` gate is cheap and removes the footgun.

**How to apply:**
- Any new position-shape validator (PR2 speed delta, PR4 navmesh containment) that does float comparisons against caller-supplied bounds must run `is_finite()` on each axis first. Don't trust comparison short-circuiting to do the work for you.
- The regression test for the `is_finite()` gate **must** use a wide-open AABB (`min = [-Inf, -Inf, -Inf]`, `max = [+Inf, +Inf, +Inf]`) so that only the explicit gate can reject — testing against `unit_bounds` (`[-100, -50, -100]..[100, 200, 100]`) passes even with `is_finite()` reverted to `is_nan()`, because `+Inf <= 100` is already false. Pinned by `position_with_infinity_axis_rejected` / `position_with_neg_infinity_axis_rejected` in `crates/entity/src/movement_validation.rs`.

**Test-quality pattern this teaches:** if a "fix" replaces an implicit behavior with an explicit one, the regression guard has to construct the input shape where the implicit behavior would have failed silently. Otherwise the test passes on revert and provides no protection.

See [[pr1-bounds-seam]] for the cell seam this validator plugs into, and [[movement-validation-anchors]] for the broader context.
