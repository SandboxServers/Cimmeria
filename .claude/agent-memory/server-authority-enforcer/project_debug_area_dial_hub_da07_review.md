---
name: project-debug-area-dial-hub-da07-review
description: PR #1232 DA-07 Debug Area outbound-only dial hub (gate 29) reviewed 2026-10-04 — cleared trust shape, where each hub filter lives, Men'fa gate 22 arrival ~192 m under the playable surface
metadata:
  type: project
---

PR #1232 (DA-07) reviewed 2026-10-04: CONDITIONAL (rebase onto #1230 only), no trust hole.

Cleared shape, reuse when reviewing anything that grants gate addresses:
- The GM power is a *grant* into the in-memory `CellEntity::known_stargates` (like gmDHD), never a bypass in `player_knows_stargate`. That function refuses `debug_dial_hub` targets *before* the book check, with the unknown-address refusal.
- The hub is resolved from the caller's world (lowest gate id), not from the clicked DHD prop. `interact_range` (cell-world space_manager/interact_range.rs) already refuses cross-space interacts (`OtherSpace`), so the shared-map world 73 / world 1300 overlap is closed.
- The in-memory book dies on every world entry: `InitPlayerState` overwrites it from the DB, and no cell-to-DB save writes `known_stargates`. The only SQL writers are `persist_arrival` (both halves) and `append_known_stargate`, both hub-filtered. A new writer is the place to look for a leak.
- Mutation-tested: every hub filter has a guard that fails on revert, the SQL ones on a live DB.

Open residuals (both closed in the PR's fix batch, 2026-10-04: gate 22 is on `HUB_EXCLUDED_GATES` pending a DA-06 pin, and `handle_dial_gate` now refuses a destination world with no space before teardown):
- Gate 22 Men'fa (SGU): row y=-191.9. `menfa_light.nav` has a floor at y≈-0.08 directly above, and only a fragment at -194.7 sits 13 m away. The PR's "13.2 m, 2.8 m below, client settles you" understates it. The 2026-09-27 survey's "~192 m below" was right.
- `handle_dial_gate` still has no `world_is_enterable` guard, so gmDHD to one of the 14 worlds with no space strands the GM.

**How to apply:** if a future gate or address feature adds a grant path or an SQL writer to `known_stargates`, check it filters `debug_dial_hub`. See [[reference-world-1300-not-gm-enforced]].
