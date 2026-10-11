---
name: unentered-entity-destroy-and-db-await-seam
description: Removing a player entity that never entered uses DestroyEntity, not DisconnectEntity; a fake TCP listener makes a session change land inside a DB await deterministically
metadata:
  type: project
---

Verified 2026-10-11 (CH-14 follow-up, `play_character/mod.rs::remove_unentered_entity`).

- **Cell `DisconnectEntity` is the logout path and does `player_id`-keyed work**: it persists the
  last position (`PersistPosition` by player_id) and the org plugin's squad `on_disconnect` removes
  the *player_id* from its squad. For an entity whose entry never completed (a relaunch takeover
  replaced the session, often with the same character), that hits the replacing session's
  character. Use `DestroyEntity` (the gate-travel teardown, keyed by entity id only).
- `DestroyEntity` has no reply, but the cell handles its base channel strictly in order
  (`message_loop.rs` awaits each `handle_base_message`), so freeing the id in `EntityManager`
  right after the send is queued is safe against #999 recycling. If the send fails, withhold
  the id (session teardown's policy).
- base-world-entry had no `cimmeria-common` dep; `EntityId` is not re-exported by
  `cimmeria-entity`, so freeing a `u32` id needs that dep.

**Test seam:** to change state *inside* a `query_world_entry` DB await (e.g. replace the session
before a `NO_ENTITY_ID` bail), bind a `tokio::net::TcpListener` on port 0, do the change on
`accept`, drop the socket; point a lazy pool at it with `PgConnectOptions::new().host().port()`
(no URL string, so the test-support URL guard, which only allows `@127.0.0.1:1/` literals, is
not tripped). The query fails only after the connection drops, so the order is guaranteed.

Related: [[testing-patterns-index]], [[session-scoped-side-tables-and-teardown]]
