# Contact List System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire formats, client Lua bridge); MEDIUM (presence semantics); LOW (flags bitmap internals)
> **Sources**: `SGW.exe` Ghidra; `ContactListManager.def`; `alias.xml`;
>   `deprecated/python/base/SGWPlayer.py`; `crates/services/src/cell/{cell_methods,client_methods}/contact_list.rs`;
>   `crates/services/src/wire_log/decoders/generated.rs`; `db/resources/Social/Types/E*.sql`;
>   `db/resources/Texts/Seed/texts.sql`; `docs/reverse-engineering/findings/contact-list-wire-formats.md`
> **Tracking issue**: replaces #71; resolves #275

## #275 answer

**The `contactListFlagsUpdate` handler EXISTS** — method index 58 in
`crates/services/src/cell/cell_methods/contact_list.rs`. It correctly parses `list_id` (i32) + `flags`
(u32) and dispatches to a `tracing::info!` **no-op stub**. So #275's "verify handler exists" is answered
YES; the work is to implement it, folded into this issue.

## Completeness assessment

A **generic named-list system** (not a hardcoded friends/ignore pair). The Python server never implemented
it (`chatIgnore`/`chatFriend` are trace-only stubs; no `ContactListManager` class). Rust has all 6 cell
handlers as `UNIMPLEMENTED` stubs and 5 server→client decoders, but no persistence, no fanout, no login push.

| Sub-system | % |
|---|---|
| Wire format C→S / S→C | ~90% / ~90% |
| Handler dispatch routing | ~80% |
| Handler logic | 0% |
| Persistence (DB schema) | 0% (no table) |
| Online-presence fanout | 0% |
| **Overall** | **~15%** |

## Architecture

Each list: `(id INT32, name WSTRING, flags UINT32, members WSTRING[])`. Two system lists seeded via text
monikers: **300 = "Friends"**, **301 = "Ignore"** (`texts.sql` lines 42541/42563). `ERelationshipType`:
Friend/Foe/Neutral. `EMemberInfo`: 9-value presence descriptor (Position/Space/Level/Health/Focus/Energy/
Name/Archetype/World) — hypothesised to be the per-list `flags` subscription mask.

`/friend` and `/ignore` slash commands (`Event_SlashCmd_ChatFriend` vtable `0x018444d0`,
`…ChatIgnore` `0x018444b4`) are a **separate NetOut path** (`SGWNetworkManager` handlers `0x00d58010` /
`0x00d57ed0`) routing `chatFriend(name, nick, flag)` / `chatIgnore(name, flag)` — both Python stubs.

Client Lua bridge (6 functions): `getAllContactLists` (`0x00aac8f0`), `getContactList` (`0x00aac870`),
`requestCreateContactList`, `requestDeleteContactList`, `requestRenameContactList`,
`requestModifyContactList` (`0x00aac9c0` → `FUN_00ada540` → `FUN_00e61b10` builds `{aListId, aFlags}`).

## Wire messages

### Client → Server (cell methods, RTTI-confirmed, post cyclic-shift correction)

- 55 `contactListCreate` — `WSTRING name, UINT32 flags` (RTTI `0x00e5f950`)
- 56 `contactListDelete` — `INT32 listId` (`0x00e5f970`)
- 57 `contactListRename` — `INT32 listId, WSTRING name` (`0x00e5f990`)
- 58 `contactListFlagsUpdate` — `INT32 listId, UINT32 flags` (`0x00e5f9b0`)
- 59 `contactListAddMembers` — `INT32 listId, ARRAY<WSTRING> names` (`0x00e5f9d0`)
- 60 `contactListRemoveMembers` — `INT32 listId, ARRAY<WSTRING> names` (`0x00e5f9f0`)

### Server → Client (client methods, decoded in `wire_log/decoders`)

- 85 `onContactListUpdate` — `INT32 listId, WSTRING name, UINT32 flags` (create + update)
- 86 `onContactListDelete` — `INT32 listId`
- 87 `onContactListAddMembers` — `INT32 listId, ARRAY<WSTRING> names`
- 88 `onContactListRemoveMembers` — `INT32 listId, ARRAY<WSTRING> names`
- 89 `onContactListEvent` — `WSTRING playerName, UINT32 eventId, INT32 dataValue`

`EContactListEvent` (`EContactListEvent.sql`): 0 LoggedInStatus (dataValue 1/0), 1 GainLevel (new level),
2 Death, 3 GateTravel. dataValue for 1–3 inferred from convention.

## Flows

- **Login push** (expected): after world entry, server pushes `onContactListUpdate` [85] per list, then
  `onContactListAddMembers` [87] with rosters. Client is server-pushed, does not poll.
- **Add/remove**: `requestModifyContactList(listId, name, add)` → `contactListAddMembers`/`RemoveMembers`
  [59/60] → echo [87]/[88]. Ignore is just membership in the "Ignore" list; chat-filtering must live in the
  chat dispatch layer.
- **Presence fanout**: on A login/logout/level/death/gate, push `onContactListEvent` [89] to every online
  player who has A in any list.

## Open questions

1. Does `requestModifyContactList` route to `contactListAddMembers/RemoveMembers` (59/60) or
   `contactListFlagsUpdate` (58)? `FUN_00e61b10` builds `{listId, flags}` which matches CM 58, but the Lua
   name implies member edit. → x64dbg D.1.
2. `aFlags` semantics (EMemberInfo subscription mask?). → x64dbg D.2.
3. `onContactListEvent` dataValue for GainLevel/Death/GateTravel. → x64dbg D.3.
4. ChatFriend/ChatIgnore — base-layer vs cell-layer RPC. → x64dbg D.4.

## Dynamic-analysis needs (x64dbg)

- **D.1** BP `0x00ada540` then step into `FUN_00e61b10` / `0x00e62560` — which event is emitted on add-friend.
- **D.2** BP `0x00e5f9b0` (`contactListFlagsUpdate` emit) — capture `flags` across list configs.
- **D.3** BP `0x00d8fa20` (`onContactListEvent` emit) — capture playerName/eventId/dataValue for login,
  level-up, death, gate travel (needs two clients).
- **D.4** BP `0x00d57fa0` / `0x00d57e60` — capture ChatFriend/ChatIgnore payloads.
- **D.5** BP `0x00aac8f0` (`getAllContactLists`) — confirm server pushes lists at login (poll vs push).

## Persistence

No `sgw_contact_list`/`sgw_friend`/`sgw_ignore` table exists anywhere — must be added.

## Ghidra annotations

None applied. Recommended renames: `FUN_00e61b10`→`ContactList_EmitFlagsUpdate`,
`FUN_00adfcf0`→`Lua_getContactList_bridge`, `FUN_00ada540`→`ContactListManager_RequestModify_Dispatch`,
plus a plate comment on `0x00e5f9b0` citing the cyclic-shift correction.
