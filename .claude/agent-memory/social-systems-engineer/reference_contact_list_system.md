---
name: reference-contact-list-system
description: Contact list system wire format, state machine, DB schema, and cascade rules for issue #572
metadata:
  type: reference
---

## Contact List System — #572

### Wire format (C→S cell methods, CM 55–60)

All 6 client→server methods are in `ContactListManager` interface (cell methods).
Cell method dispatch goes to `cell/cell_methods/contact_list.rs::dispatch()`.

Wire encoding of WSTRING: `[u32 char_count LE][UTF-16LE × char_count]`
Wire encoding of ARRAY<WSTRING>: `[u32 count LE][WSTRING × count]`

| CM | Method | Wire layout |
|----|--------|-------------|
| 55 | contactListCreate | WSTRING name @ 0, u32 flags after name |
| 56 | contactListDelete | i32 list_id @ [0..4] |
| 57 | contactListRename | i32 list_id @ [0..4], WSTRING name @ 4 |
| 58 | contactListFlagsUpdate | i32 list_id @ [0..4], u32 flags @ [4..8] |
| 59 | contactListAddMembers | i32 list_id @ [0..4], ARRAY<WSTRING> @ 4 |
| 60 | contactListRemoveMembers | i32 list_id @ [0..4], ARRAY<WSTRING> @ 4 |

MAX_MEMBERS_PER_REQUEST = 100 (abuse prevention clamp in parse_wstring_array).

### Wire format (S→C client methods, CM 85–89)

| CM | Method | Wire layout |
|----|--------|-------------|
| 85 | onContactListUpdate | i32 list_id, WSTRING name, u32 flags |
| 86 | onContactListDelete | i32 list_id |
| 87 | onContactListAddMembers | i32 list_id, ARRAY<WSTRING> names |
| 88 | onContactListRemoveMembers | i32 list_id, ARRAY<WSTRING> names |
| 89 | onContactListEvent | WSTRING player_name, u32 event_id, i32 data_value |

Rust constants: `ON_CONTACT_LIST_UPDATE=85`, `ON_CONTACT_LIST_DELETE=86`,
`ON_CONTACT_LIST_ADD_MEMBERS=87`, `ON_CONTACT_LIST_REMOVE_MEMBERS=88`,
`ON_CONTACT_LIST_EVENT=89`.

EContactListEvent values: 0=LoggedInStatus (data_value: 1=online, 0=offline),
1=GainLevel, 2=Death, 3=GateTravel.

### DB schema

Two tables (in `db/sgw/Social/Tables/`):
- `sgw_contact_list (list_id SERIAL PK, player_id INT4 FK→sgw_player, name TEXT, flags INT4)`
  - UNIQUE(player_id, name) — system list names must be unique per player
  - CASCADE DELETE on player_id FK
- `sgw_contact_list_member (list_id INT4 FK→sgw_contact_list, player_name TEXT)`
  - PK(list_id, player_name)
  - CASCADE DELETE on list_id FK

System lists: Friends (flags=300), Ignore (flags=301). Created idempotently via
`ensure_system_lists()` using `WITH ins AS (INSERT … ON CONFLICT DO NOTHING RETURNING list_id) SELECT … UNION ALL SELECT … LIMIT 1`
— no WAL write on conflict (no-write-amplification pattern).

Sequence: `sgw_contact_list_list_id_seq` in `db/sgw/Social/Sequences/`.

Seed data (player_ids 62-68, list_ids 1-14) in `db/sgw/Social/Seed/`.

### Code layout

```
crates/services/src/
  base/contact_list/
    mod.rs                   — pub(crate) re-exports
    wire.rs                  — S→C packet builders (5 functions) + MAX_MEMBERS_PER_REQUEST
    persistence.rs           — DB CRUD + live-DB tests
    handlers/
      mod.rs                 — push_contact_lists_on_login + re-exports
      crud.rs                — handle_create/delete/rename/flags_update/add_members/remove_members
      presence_fanout.rs     — fanout_login_status + collect_watcher_entity_ids
  base/world_entry/cell_dispatch/
    mod.rs                   — routes ContactList* cell→base msgs to contact_list_dispatch
    contact_list_dispatch.rs — route() fn
  cell/cell_methods/
    contact_list.rs          — CM 55-60 cell-side handlers, WSTRING parsing; imports MAX_MEMBERS_PER_REQUEST from wire
  cell/messages/cell_to_base.rs  — ContactList* variants (6 total)
```

### Login/logout fanout (Phase 4, LoggedInStatus only)

Login fanout: `client_ready.rs::handle_on_client_ready` spawns
`fanout_login_status(name, true, ...)` via `tokio::spawn` (fire-and-forget)
after `push_contact_lists_on_login`. The list-push itself stays on the critical path.

Logout fanout: `dispatch/session.rs::handle_log_off` spawns
`fanout_login_status(name, false, ...)` via `tokio::spawn` (fire-and-forget).
Player name is snapshotted before cleanup; Arcs are cloned into the spawned task.

Abrupt disconnect (`destroy_client_entities` in `helpers/mod.rs`) is
synchronous and lacks db_pool — offline fanout on abrupt disconnect is
deferred to Phase 5 (TODO #572).

`fanout_login_status` in `handlers/presence_fanout.rs`:
1. Calls `find_watchers(pool, player_name)` → Vec<i32> of watcher player_ids
2. Finds each watcher's entity_id via `collect_watcher_entity_ids` (pure fn, unit-tested)
3. Sends CM 89 (onContactListEvent, eventId=0, data_value=1/0) to each online watcher

### db_pool threading

`db_pool: &Option<Arc<PgPool>>` was added to:
- `dispatch_sgw_player_base_method` (in `dispatch/mod.rs`)
- `handle_log_off` (in `dispatch/session.rs`)
- Tests: `routing_logging.rs` (4 sites) and `chat_speaker_flags.rs` (6 sites)
  all pass `&None, // db_pool` to suppress fanout in unit tests.

### sqlx query pattern

Do NOT use `sqlx::query_as!(TypeName, ...)` (bang macro) — it requires a live
DATABASE_URL or cached .sqlx/ at compile time and will fail in CI without DB.
Use `sqlx::query_as::<_, TupleType>("...")` (no bang) or `sqlx::query_scalar(...)`
instead. Discovered when fixing persistence.rs during #572 impl.

### Invariants covered

- Invariant #2 (stale guild rank — analogous: stale online status): fanout
  happens after DB state is committed, not before.
- Invariant #5 (ignore-list leak): not yet implemented — ignore-list check
  before fanout is Phase 5.

### Deferred (Phase 5)

- GainLevel/Death/GateTravel events (CM 89, eventId 1/2/3)
- Abrupt-disconnect offline fanout via `destroy_client_entities`
- Ignore-list check before queueing fanout events
- Slash commands and modify-button routing
