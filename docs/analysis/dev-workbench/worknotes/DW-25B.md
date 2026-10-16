# DW-25B — slash metadata entries 28–54

Implemented 27 disabled, typed patch 012 slash descriptors in `crates/lab/src/workbench/slash_descriptors_1b.rs` with exact source order, argument labels/types/required flags, target, impact, readback, compensation and per-command blocking reasons. Regression tests guard the exact signatures, optional argument, width mismatches, grant/reload/travel impacts and execution refusal. The [palette reference](../slash-palette-reference.md) summarizes the operator contract.

Remaining: DW-24B must prove native submission, parser and accepted receipts before any action is enabled; DW-27B merges all six metadata slices into the catalog. `/gmsetmobabilityset` additionally needs a native optional/default contract and exact live known-ability readback. No live lab or client operation was performed.
