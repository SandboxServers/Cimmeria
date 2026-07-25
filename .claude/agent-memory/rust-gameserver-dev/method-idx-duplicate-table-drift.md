---
name: method-idx-duplicate-table-drift
description: Two client-method index tables exist; cell/client_methods is authoritative, mercury::method_idx is a partial drifted copy — prefer the former for new emit paths
metadata:
  type: project
---

The repo has **two** server→client method-index tables, and they have
drifted:

- `crates/services/src/cell/client_methods/` — one file per interface
  (`player.rs`, `missionary.rs`, `combatant.rs`, ...), complete, matches
  `entities/defs/*.def` and `docs/protocol/client-method-dispatch-table.md`.
  **Treat as authoritative.**
- `crates/services/src/mercury/mod.rs::method_idx` — a partial flat copy
  covering only the indices some emit path happened to need. This is the
  one that drifts.

**Why it matters:** the drift is not inert. `method_idx` carried
`ON_STORE_OPEN = 80` / `ON_STORE_UPDATE = 81` under a fabricated
"SGWVendorStore interface (80-81)" heading — no such interface exists in
`entities/defs/interfaces/`. 80/81 are Missionary's `onMissionUpdate` /
`onStepUpdate`, so the vendor emit path shipped the store payload to the
client's mission handlers. The correct `client_methods::player` constants
were defined and referenced nowhere. Fixed 2026-07, but the structural
hazard remains: nothing stops the two tables diverging again.

Second-order damage worth knowing about: `method_idx` gets used to *label*
sub-slot bytes when interpreting pcaps, so a wrong constant silently
mislabels wire evidence. `docs/audits/entity-property-sync-section2-audit-2026-05-16.md`
and `docs/audits/mercury-rust-conformance-2026-05-15.md` both label
sub_idx 19/20 as `ON_STORE_OPEN`/`ON_STORE_UPDATE` in their evidence
tables; those rows are actually mission traffic. The audits' *conclusions*
(idbase = 61, not 62) stand on other rows and are unaffected.

**How to apply:** For a new server→client emit, import from
`crate::cell::client_methods::<interface>`. Before trusting any
`method_idx` constant, cross-check it against the matching
`client_methods/` file and the dispatch table. If a `method_idx` comment
names an interface, verify that `entities/defs/interfaces/<Name>.def`
actually exists — that check is what exposed this bug.

Related: [[witness-entity-method-dual-fn]] — the same "two places, both
need changing" shape on the fan-out side.
