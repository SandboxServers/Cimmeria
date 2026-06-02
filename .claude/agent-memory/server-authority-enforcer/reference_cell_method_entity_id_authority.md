---
name: reference-cell-method-entity-id-authority
description: Cell-method handlers receive player_eid from session, not the entity_id the client supplies in the 4-byte prefix
metadata:
  type: reference
---

In `crates/services/src/base/connect_loop/cell_arms.rs:64-89,119-138`,
when a `0x80..0xBF` cell-method packet arrives:

1. The 4-byte `entityId` prefix is parsed into `entity_id_from_client`
   (line 87-88) and logged for diagnostics, but
2. The value forwarded to `BaseToCellMsg::CellMethodCall` is `player_eid`,
   the server-tracked entity id pulled from the `ConnectedClientState`
   at the inbound `SocketAddr` (line 64-67, 122, 134).

This means a client cannot route a cell-method call against another
player's entity_id by lying in the prefix bytes. Useful when auditing
any cell-method-shaped client message (mail, trade, vendor, ability,
inventory, etc.) — the entity-id-spoof exploit class is blocked at
the framing layer, so per-handler audits don't need to re-derive it.

What is NOT blocked here:
- The `args` payload (the rest of the bytes after the entity-id prefix)
  remains fully client-supplied and must be validated per-handler.
- Any handler that reads a target entity-id out of `args` (e.g.
  `SetTarget`, `UseAbility target_id`, `payCODForMailMessage MailId`)
  must validate ownership/perception/range itself.
