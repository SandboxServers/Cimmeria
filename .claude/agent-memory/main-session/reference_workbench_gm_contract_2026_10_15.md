---
name: workbench GM contract
description: Dot registry and authority/evidence split for the developer workbench
type: reference
---

# Workbench GM contract (2026-10-15)

- The dot-console registry is `crates/cell-console/src/cell/console/registry/commands/mod.rs::COMMANDS`; `command_catalog()` projects its rows. DW-19 added target policy keys and version 1 metadata to those rows without a parallel name list. Source: the registry and `crates/cell-console/src/cell/console/mod.rs`.
- The raw lab tool `server_console_exec` sends `BaseToCellMsg::LabConsoleExec`; `crates/cell/src/cell/service/base_messages/lab_console.rs` applies the acting entity's GM gate. Captured feedback is not a verified state readback. Source: `crates/lab-mcp/src/tools/console.rs` and the cell handler.
- DW-19 leaves every dot row `manual_pending` / `unsupported` until per-command evidence proves typed arguments, effect, readback and recovery. The native slash catalog is a distinct source; line submission alone cannot prove a GM method was received. Source: `docs/analysis/dev-workbench/work-packets.md` and `docs/architecture/gm-cell-method-gating.md`.
