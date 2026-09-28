---
name: plugin-architecture-compat-boundary
description: Client-compat rules for #962 feature plugins (ADR docs/architecture/plugin-architecture.md §2): single FIFO CellToBaseMsg channel, hook points at the inline call's exact line, cell/base index spaces separate
metadata:
  type: reference
---

Confirmed as advisor for #962 (2026-09-28); canonical text is ADR §2 (C1-C8).

- Moving per-entity state into `CellEntity::extensions` cannot change what the
  client sees: SGW's 436 properties use only CELL_PRIVATE / BASE / CELL_PUBLIC,
  so every client-visible value is an explicit send. Only send ORDER can break.
- Therefore: plugins emit on the one FIFO `CellToBaseMsg` channel (no
  per-feature channels, including for the future `PluginMsg` envelope), and a
  hook point fires at exactly the line the inline call occupied
  (`TickStage` / `EntityHookPoint` variants document their position).
- Cell and base method indices are separate namespaces (0x80 / 0xBD+sub vs
  0xC0); plugins register flattened indices from `cimmeria-wire` constants,
  decode stays in core, GM gate runs before plugin lookup.
- Pilot gotcha: tests in crates below a leaf plugin cannot install it; tests
  that drive a hook point move up or install the plugin via dev-dependency.

Related: [[na38-client-orders-reliable-stream]], [[entity-def-and-pak-ground-truth]].
