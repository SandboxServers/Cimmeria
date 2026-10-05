---
name: base-side-log-naming-and-lock-traps
description: Naming IDs on base-side log lines (Rule 6) - which resolver to use, what the base cannot name, and the match-scrutinee lock trap that turns a name lookup into a nested lock
metadata:
  type: project
---

Base-side Rule 6 naming (NT-24, 2026-10-04), `crates/base-session/src/base/session_identity.rs`:

- `session_identity(&c)` when the session is in hand (also inside a held `connected` lock; it takes no lock).
- `identity_for_addr(connected, addr)`: locks `connected`.
- `identity_for_entity` / `entity_name_for(connected, entity_to_addr, eid)`: locks both maps one after the other, never nested. For an NPC id it scans all of `connected` and returns `None`.
- `identity_for_player(connected, player_id)` for cell->base persistence messages that carry only the DB id. `identity_for_player_in(&clients, ..)` when the map is already locked.
- The base has **no NPC names**. The one exception: AoI messages carry `NpcAoIData.name_id`, and `cimmeria_names::book().text(name_id)` names the NPC (`cell_dispatch/aoi_names.rs`).
- At logout the session is already gone. `PersistPosition` / `SystemOptionsUpdate` read the name back with `UPDATE ... RETURNING player_name`, the name's only source at that point.

**Lock trap:** in `match entity_to_addr.lock().unwrap().get(&id).copied() { None => { ... } }` the guard lives for the whole match. A name lookup in the `None` arm then locks `connected` (or `entity_to_addr` again: a self-deadlock). Read the map on its own `let` line first. Fixed this way in `teleport.rs` and `bandolier.rs`.

**Why:** the base half of the server has no SpaceManager. Each lookup takes a map lock, so a careless lookup inside a held guard deadlocks the whole receive loop.

**DEBUG is not free here:** OTEL_FILTER exports every cimmeria crate at DEBUG, and the file layers write many modules at TRACE, so a lookup inside a `debug!` still runs. Never take an extra `connected` / `entity_to_addr` lock just to name a per-packet or per-AoI-event line. Read the name under the lock the code already holds (`intern_opt(c.player_name.as_deref())` is a hash hit), or mark the ID `nt:id-only` with a hot-path reason. Rare WARN/ERROR and one-shot lines may look the name up. NT-23's `session_identity::player_name_for_entity` does one map hop with no scan.

**How to apply:** before adding a name to a base log line, check which guards are live at that line and how often the line fires. Related: [[witness-entity-method-dual-fn]], [[observability-test-and-throttle-traps]].
