# Dev workbench target dossier — 2026-10-15

Source: `crates/lab/src/workbench/dossier.rs`, `crates/lab/src/supervisor/world/{memory,io}.rs`, `crates/lab-mcp/src/tools/{entities,witnesses,logs}.rs`, and `docs/analysis/dev-workbench/work-packets.md` DW-03.

- The selected runtime entity id is the client `Unit.Target` slot map value. The stock `client_entity_find` pins private slots for unit details, so an unleased observer read uses `LiveWorld::slots` and `client_entity_table` with `is_ready=false` instead. The client actor pose is read from the existing world snapshot.
- `unitMobId` belongs to the client unit API and is not `server_entity_get.template_id`. Only the runtime entity id joins the two layers. Server absence or entity id mismatch is an explicit gap.
- Server witnesses come from `server_witnesses`; the nondestructive `server_log_tail` supplies recent structured entity-id rows. No packet-tap drain is used for the read view. The latter is a log subset rather than a complete wire history; DW-08 will supply the capture broker for fuller evidence.
- A second selected-slot read after the other sources invalidates every dossier field if the target changed during the read. Live client validation remains pending because no lab lease was requested.
