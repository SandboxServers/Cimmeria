---
name: vendor-stock-wire-review
description: 2026-10-10 tg-vendor review — vendor 78-82 parse stock two-array layouts, template from cell session only, free repair/recharge removed; residual gaps
metadata:
  type: project
---

Reviewed 2026-10-10 (worktree tg-vendor, commits 8daa63c87 + 095a8cdf6): cell `vendor/store_ops.rs` parses `u32 n,i32[n],u32 m,i32[m]` (78-80) and `u32 n,i32[n]` (81-82), bounds counts before allocating, refuses trailing bytes / n!=m / qty<=0, takes the template from `vendor_context` (player.vendor_entity -> vendor entity template_id). Base repair/recharge now take plain `i32` template; free branches deleted (CAT-E-01 closed).

**Why:** the stock client sends no template id on any store method; the old pair+trailing-template parser refused every stock request, so only forged packets reached the free path.

**How to apply:**
- Real revert guard for CAT-E-01 is the `Option<i32>` -> `i32` type plus the cell happy-path tests asserting the session template; the no-session test and the two base live-DB "no repair/recharge list" tests pass on revert (old cell also refused no-session; the live-DB tests pass a template, exercising the pre-existing paid-path refusal).
- Base `VendorLog::refused` only logs INFO; no player feedback for insufficient_cash / not_repairable / not_on_vendor_list etc. Now reachable from the stock UI.
- CAT-E-02 still open: `vendor_context` checks neither proximity nor same space, and `vendor_entity` is never cleared; template is read from whatever entity now holds that id (see [[exploit-entity-id-recycling]]).
