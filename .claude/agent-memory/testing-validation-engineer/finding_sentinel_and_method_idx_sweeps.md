---
name: finding-sentinel-and-method-idx-sweeps
description: 2026-09-25 static sweeps — live-DB sentinel duplicates (4 real, 5 cross-table, decimal-offset spill) and method-index vs .def flattening (zero mismatches); reusable sweep approach
metadata:
  type: project
---

Two static sweeps done 2026-09-25 for new tickets (`live-db-sentinel-collision`, `method-idx-def-conformance-test`).

**Sentinels:** the `0x7000_0500` resync/mail duplicate was not alone. There were three more same-table (account/player) duplicates: `0x0400` use_instance vs missions, `0x0D00` gate_travel vs player_load/meta, and `0x1700` character delete vs cell_dispatch/state_field. Five more were cross-table only. Many modules use DECIMAL offsets (`TEST_BASE + 100..1201`), which spill past their 0x100 block. For example, move_/tests spans 0x0200..0x06B1. No exact collision today, by arithmetic luck only. The audit doc's "next free 0x7000_0E00" is wrong, because vendor/helpers owns it.
**Why:** neighbour doc-comments are the only registry, and they are incomplete.
**How to apply:** don't trust a "next free" claim. Grep all `const .*= 0x7xxx_xxxx` across crates/services/src and expand `BASE + off` before proposing a sentinel. The recommended guard is a crate-wide source lint (one value per file), not an opt-in registry.

**Method indices:** a Python flattener (walk the Parent chain root to leaf; at each level, Implements interfaces first, then own methods; Exposed-only for cell/base) reproduces 157/109/226/30 against entities/defs. Every Rust surface matched: method_idx, client_methods, console locals, cell_methods through `cell_method_name` (names.rs), gm, base dispatch, and the Account BASEMSG_ON_* consts. `cell_method_name` and `wire_log::client_names::outbound_method_name` are compiled name tables. Use them as alias-free conformance hooks rather than regex-matching prefix-stripped const names.

Related: [[feedback-revert-to-verify-regression-guards]]

Ticketed as #800 (sentinels) and #801 (method_idx conformance), 2026-09-25.
