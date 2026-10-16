# Dev workbench target dossier — 2026-10-15

Source: `crates/lab/src/workbench/dossier.rs`, `crates/lab/src/supervisor/world/{memory,io}.rs`, `crates/lab-mcp/src/tools/{entities,witnesses,logs}.rs`, and `docs/analysis/dev-workbench/work-packets.md` DW-03.

- The selected runtime entity id is the client `Unit.Target` slot map value. The stock `client_entity_find` pins private slots for unit details, so an unleased observer read uses `LiveWorld::slots` and `client_entity_table` with `is_ready=false` instead. The client actor pose is read from the existing world snapshot.
- `unitMobId` belongs to the client unit API and is not `server_entity_get.template_id`. Only the runtime entity id joins the two layers. Server absence or entity id mismatch is an explicit gap.
- `ServerTools::from_env` reaches one daemon-global endpoint, while the dossier has no live `CaptureContext` producer. Matching a numeric target entity id does not bind the client instance to that server. The live dossier therefore reports `server_session_unbound` and leaves server fields, log rows and position delta null. A future bound capture must identify the instance, selected id, player id and server session and match exactly one `server_sessions` row before a join. This remains pending implementation.
- Once bound, witnesses come from `server_witnesses`; nondestructive `server_log_tail` requests the full 500-row ring, filters structured entity-id rows, and reports `log_tail_truncated` when the reply is incomplete. It is still a buffered subset rather than full wire history; DW-08 supplies fuller capture evidence.
- The client `entity_table` row's embedded id and pointer must agree with the world snapshot, and a `WorldEntity.pose_error` is an explicit client position gap.
- A second selected-slot read after the other sources invalidates every dossier field if the target changed during the read. Live client validation remains pending because no lab lease was requested.
