---
name: pre-split-branch-port-traps
description: Porting a branch written against the old monolithic cimmeria-services onto the split crates — placement rule, wire-crate limits, OTEL row guard, reassigned seed ids, sentinel collisions
metadata:
  type: project
---

Learned porting `feat/571-black-market-phase1` (June 2026, 13 commits) onto
the split crates as BM-01 (2026-09-27). Most of the branch moved with only
`crate::` path edits; these are the parts that did not.

1. **Place by the old module path, then pull down what the cell names.** A
   top-level `base::<feature>` module goes to `cimmeria-base-session` beside
   `contact_list` and `crafting` (both of the other base crates and
   `cimmeria-base` already depend on it, so no new edge). Anything a
   `CellToBaseMsg` variant carries, or that cell code calls, must move to
   `cimmeria-wire`, because no cell crate can see a base crate. For BM that
   was `BMSearchOptions` and the `onBMOpen` serializer
   (`cimmeria_wire::black_market`, crate-level like `crafting`); the base
   module re-exports the type so `super::types::X` paths still compile.
2. **`cimmeria-wire` has no sqlx.** A row struct that derives
   `sqlx::FromRow` and is also a serializer input cannot move down whole.
   Keep the row struct and the serializers that take it in the base crate;
   move only the pieces the cell needs.
3. **A new top-level `cimmeria-wire` module needs its own `OTEL_FILTER`
   row** (`cimmeria_wire::<module>=debug`) in
   `crates/server/src/logging/filters.rs`, even with no tracing calls:
   `parity_tests::crate_rows` fails on any uncovered top-level module.
   Modules added to an existing crate's subtree need nothing.
4. **Seed ids from a months-old branch are probably taken.** Template and
   spawn ids are handed out per campaign by the social-systems coordinator
   (debug hub 300-304/400-404, crafting 310-329, orgs 330-349/430-449, pets
   350-369/450-469, bank 370-389/470-489, social 390-399/490-499), and the
   Castle rebuild reused 168/238. Chain seed files were renamed too
   (`space_castle_cellblock_chains.sql` is now `castle_cellblock_chains.sql`,
   scoped `'space', 12`). Ask the coordinator for ids; do not pick them.
5. **Re-check live-DB sentinels against main.** The branch's
   `0x7000_0900`/`0x7000_0E00` bases inserted the same account/player ids as
   the vendor buyback and helper tests (issue #800's shape). Campaigns now
   take a whole `0x7000_Nxxx` block: BM `A`, bank `B`, crafting `C`.
6. **Prefer `cell::client_methods::<interface>` over adding to
   `mercury::method_idx`** when the branch added duplicates there
   ([[method-idx-duplicate-table-drift]]).
7. **`send_to_witness_reliable` now returns `WitnessSendOutcome`**, which is
   deliberately not `#[must_use]`, so June-era `...await;` call sites compile
   unchanged.

Related: [[services-split-extraction-traps]], [[crate-split-extraction-traps]],
[[vacuous-guard-and-sentinel-collision-review]].
