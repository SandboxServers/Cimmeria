---
name: reference-dev-workbench-mission-2026-10-16
description: Mission X-ray evidence boundaries and existing read sources
type: reference
---

# Mission X-ray read boundaries (2026-10-16)

`crates/lab/src/supervisor/flows/ui_state.rs` returns visible mission tracker entries as ordered text lines without a mission ID. The entry order lets a reader associate a title and its following step text, but a title match is only UI evidence. `crates/lab-mcp/src/tools/sessions.rs` returns the server session, player entity and player ID together. `crates/lab/src/workbench/dossier.rs` documents that no trusted client instance/PID-to-server-session producer exists yet, so the DW-06 live route withholds player-specific `sgw_mission` reads. `resources.content_chains` can resolve mission-scoped chain IDs by `(scope_type, scope_id)`; such rows are a partial reference set, not an exhaustive list of chains with mission conditions.
