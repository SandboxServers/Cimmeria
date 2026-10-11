---
name: table-driven-live-db-refusal-cases
description: Table-driven live-DB refusal tests need one sentinel character per case and collected failures, or one dropped check masks the rest; dry-run-and-rollback as a precheck
metadata:
  type: feedback
---

A table-driven live-DB test of several refusals (FX-03 `live_db_fixture_item_refusals_write_nothing`) must seed a fresh character per case (accounts `BASE + 2*i` inside the packet's sentinel slot) and collect failures into one `assert!(failures.is_empty(), "{failures:#?}")`.

**Why:** with one shared character and `expect_err` per case, the first mutated check writes state, so later cases fail for the wrong reason (or the loop stops at the first), and a revert proof cannot show each check failing on its own. Found 2026-10-11 while proving FX-03's four item checks in one mutation run.

**How to apply:** any multi-case refusal guard that writes rows. Related precheck trick from the same packet: when "does it fit" depends on what the write's own deletes free (inventory slots), run the real write in a transaction and roll it back as the precheck, rather than duplicating the capacity math. See [[testing-patterns-index]].
