---
name: phasing-as-aoi-membership
description: Per-player phasing is client-safe as witness-set filtering in compute_player_aoi; which handlers bypass witness sets and need their own gate
metadata:
  type: project
---

Per-player phase visibility (T-08e, 2026-10-17, design in
`docs/analysis/tollana-continuation/evidence/phasing.md`) needs no client patch: filter
candidates in `cell-world/.../space_manager/aoi.rs::compute_player_aoi` next to
`is_introducible()`. The existing diff then emits LeftAoI (0x0B+0x0C) / EnteredAoI + cascade, the
same path as walk-away/back and `npc_respawn` re-create. ENTITY_INVISIBLE alone is not isolation
(client keeps the record).

**Why:** the client's EntityManager never knows why an entity entered or left; the only hard rule
is no second CREATE without a Leave between (enter-count assert).

**How to apply:** handlers that take a client-supplied entity id do NOT read witness sets and
need their own gate: `interact.rs::handle_interact` (before the faction-10 reroute),
`fire_los.rs` (beside #906 OtherSpace), `npc_ai/aggro_gates.rs::evaluate_candidate`,
`ammo_splash.rs::splash_targets`, cell-content dialog/step_anchor. `request_entity_update.rs`
already refuses non-witness ids. Base fan-out (`witness_broadcast.rs`) is safe by construction.
Status: design only, UATPending on a live toggle.
