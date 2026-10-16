---
name: workbench slash 1a evidence
description: Source-backed target and signature gaps in the first 27 patch-012 GM slash entries
type: reference
---

# Slash entries 1–27 (2026-10-16)

- `data/client-patches/012-gm-slash-commands/commands.toml` is a typed client event inventory, not a proof of cell-method binding. `crates/lab/src/workbench/slash_catalog.rs::method_candidate` leaves `binding_verified: false`; DW-24B must prove submission. Source: `docs/analysis/dev-workbench/evidence/DW-25A-E.md`.
- The client `access="p"` filter does not authorize a server command. `crates/cell/src/cell/dispatch/router.rs` invokes `crates/cell-world/src/cell/dispatch/gm_gate.rs` before `crates/cell-console/src/cell/console/gm/mod.rs::dispatch`; 109+ needs server-sourced GameMaster level. Source: `docs/architecture/gm-cell-method-gating.md` and the three modules.
- For this slice, 111, 112, 117–119, 126, 129 and 130 have no `gm/mod.rs::dispatch` arm. `/gmgetmobattribute`, `/gmshowip`, `/gmshowinventory` and `/gmremoveitem` have patch/server argument mismatches; `/gmshowflag` permits omitted flag in TOML but the handler requires it. Source: patch TOML, `docs/protocol/cell-method-dispatch-table.md`, `gm/mod.rs`, `gm/query.rs`, `gm/give.rs`.
- `gmMissionAssign`/`gmMissionAdvance` queue `MissionUpdate`; base writes `sgw_mission` while abandon deletes a non-repeat row and preserves repeat history. `gmGiveTrainingPoints` writes `sgw_player.training_points` by guarded `UPDATE ... RETURNING`, then mirrors to cell/client. Queue/feedback/client display are separate from DB readback. Source: `gm/missions.rs`, `crates/base-methods/src/base/world_entry/methods/missions/mod.rs`, `gm/give_training_points.rs`, `crates/base-methods/src/base/world_entry/methods/progression/grant_training_points.rs`.
