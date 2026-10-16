---
name: workbench slash 2a evidence
description: Source-backed minigame, item and signature gaps in patch-012 slash entries 55–81
type: reference
---

# Slash entries 55–81 (2026-10-16)

- Patch 012's one non-`/gm` word `/debugminigamecomplete` is still client-role-filtered and its candidate method 191 is server-GM-gated, but `gm/mod.rs::dispatch` has no handler. The separate minigame result is generated from an authenticated, ticketed SmartFox session; only Livewire's outcome is server validated, while placeholders accept client-declared victory. Base's replacement-ticket check suppresses stale UI close only; Cell can still fire an old queued result-1 victory chain after a newer board opens. Source: `docs/analysis/dev-workbench/evidence/DW-26A-E.md`, `crates/cell-world/src/cell/dispatch/gm_gate.rs`, `crates/minigame/src/minigame/server/{handshake,session_loop,result_dispatch}.rs`, `crates/minigame/src/minigame/games/mod.rs`, `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`, `crates/cell/src/cell/service/base_messages/minigame.rs`.
- `gmRechargeItem` (187) has no GM handler. The independent player vendor `rechargeItems` method 82 checks the server vendor session/list and owned inventory row, then transactionally updates `sgw_inventory.charges` and debits `sgw_player.naquadah`. Do not infer that this path implements the slash command. Source: `docs/analysis/dev-workbench/evidence/DW-26A-E.md`, `crates/cell-console/src/cell/console/gm/mod.rs`, `crates/base-methods/src/base/world_entry/methods/vendor/{recharge,paid_recharge/mod}.rs`.
- The patch/server signature mismatches in this slice are `/gmsetmobattribute` (missing target), `/gmdespawn` (patch zero args versus INT32 method), and `/gmkilltarget` (patch Integer versus INT64 method); `/gmspawnbycmd` is only a semantic event-name candidate. No native binding is yet proved. Source: `data/client-patches/012-gm-slash-commands/commands.toml`, `docs/protocol/cell-method-dispatch-table.md`, `crates/lab/src/workbench/slash_catalog.rs`, `docs/analysis/dev-workbench/evidence/DW-26A-E.md`.
