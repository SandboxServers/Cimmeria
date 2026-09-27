---
name: connected-map-view-over-parallel-index
description: Base-side "who is online" lookups should be a view over the connected-client map plus a session flag, not a parallel map; there are 6+ teardown paths and several bypass destroy_client_entities
metadata:
  type: project
---

`OnlinePlayerIndex` (SS-00, `crates/base-session/src/base/player_index/`) is a borrowed view over
`HashMap<SocketAddr, ConnectedClientState>` filtered on `listed_online && player_name && active_player_id`,
not a second `Arc<Mutex<HashMap<name, ..>>>`.

**Why:** sessions leave the map through `helpers::destroy_client_entities` (reasons
`client_disconnect`, `inactivity_timeout`, `send_error`, `duplicate_login`, `logoff`) AND through
`gate_travel::abandon_unspaced_session`, which deliberately skips `destroy_client_entities`
(no `EntityManager` in the cell-dispatch chain). A parallel map needs a hook in each, threaded
through contended signatures, and the next teardown path forgets it. The one in-session transition
a view cannot see is full-exit `logOff(1)`: it keeps the session and `player_name` until the client's
disconnect reaps it, so it needs the explicit `listed_online = false`.

**How to apply:** any new per-player lookup keyed by something on the session (name, account,
org membership) should first try a view over `connected`. Only add a maintained index when O(n)
over sessions is measurably too slow, and then grep every `clients.remove(` before claiming coverage.
Related: [[cross-world-transfer-flow]], [[ring-transport-fsm]].

**Session-end announcements (ORG-06, 2026-09-27):** "went offline" fanout (contact list + org [37] id 0)
fires from `destroy_client_entities` (now takes `transport` + `db_pool`) via
`session_presence::spawn_offline`, and from `logOff`; both gate on `listed_online` and `logOff` clears
it, so a full exit is announced exactly once. The gate-travel abandon path still announces nothing.
