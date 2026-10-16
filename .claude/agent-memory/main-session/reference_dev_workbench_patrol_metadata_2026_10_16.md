---
name: Developer workbench patrol metadata seam
description: Reviewed dot patrol and entity-authoring catalog availability and durability boundary
type: reference
---

# DW-20B metadata seam (2026-10-16)

- `crates/cell-console/src/cell/console/registry/workbench_world_author.rs` maps the exact 11 `patrol` and 17 `entity_authoring` registry specs to typed catalog metadata. The map is routed through `registry/workbench.rs`, then `command_catalog()` and `server_console_list`. Sources: `docs/analysis/dev-workbench/evidence/DW-20B-E.md` and `registry/commands/{patrol,entity_authoring}.rs`.
- Only `.path_show` (shared session-buffer read) and `.tag` (transient in-memory tag) are catalog-runnable. `.lookat` is catalog-disabled despite the evidence's conditional NPC allowance because command-wide availability cannot enforce target class or autosave state at execution. The other 25 require their cited server guards or exact readback before enablement. Source: `docs/analysis/dev-workbench/worknotes/DW-20B.md`.
- Authoring metadata uses `seed_commit_required` for queued SQL/live-DB effects. A `.seedconfirm` receipt is only enqueue feedback; positive base SQL result and a developer seed-file merge are separate. Sources: `docs/analysis/dev-workbench/evidence/DW-20B-E.md`, `crates/cell-console/src/cell/console/seed.rs` and `crates/base-session/src/base/console_authoring/mod.rs`.
