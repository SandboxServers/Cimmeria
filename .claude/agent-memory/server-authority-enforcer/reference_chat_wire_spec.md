---
name: reference-chat-wire-spec
description: Ghidra-anchored wire shapes for chat / contact-list / communication NetOut events + base/cell dispatch indices
metadata:
  type: reference
---

## Chat / Contact-list / Communication wire surface

### Base-layer SGWPlayer methods (msg_id 0xC0..=0xD8)

| msg_id | name | wire shape | Rust handler | status |
|---|---|---|---|---|
| `0xC0` | `chatJoin` | `WSTRING channelName, WSTRING password` | `dispatch.rs:158-169` | stub (auto-ack) |
| `0xC1` | `chatLeave` | `UINT8 channelId` | `dispatch.rs:171-175` | stub (debug log) |
| `0xC2` | `sendPlayerCommunication` | `UINT8 channel, WSTRING target, WSTRING text` | `dispatch.rs:77-156` | impl. |
| `0xC3` | `chatSetAFKMessage` | `WSTRING message` | `dispatch.rs:177-189` | stub (log only) |
| `0xC4` | `chatSetDNDMessage` | `WSTRING message` | `dispatch.rs:191-233` | impl. (sets `dnd_message`) |

### Cell-method indices used by CAT-L surface

| index | name | wire shape (after 4B entityId prefix stripped) | Rust handler |
|---|---|---|---|
| 10 | `BroadcastMinimapPing` | `i32 org_id, f32 x, f32 y, f32 z` | `cell_methods/organization.rs:48-64` stub |
| 55 | `contactListCreate` | `WSTRING listName, u32 flags` | `cell_methods/contact_list.rs:22-26` stub |
| 56 | `contactListDelete` | `i32 list_id` | `cell_methods/contact_list.rs:27-33` stub |
| 57 | `contactListRename` | `i32 list_id, WSTRING newName` | `cell_methods/contact_list.rs:34-40` stub |
| 58 | `contactListFlagsUpdate` | `i32 list_id, u32 flags` | `cell_methods/contact_list.rs:41-53` stub |
| 59 | `contactListAddMembers` | `i32 list_id, member_array` | `cell_methods/contact_list.rs:54-60` stub |
| 60 | `contactListRemoveMembers` | `i32 list_id, member_array` | `cell_methods/contact_list.rs:61-71` stub |
| 73 | `who` | (likely WSTRING filter) | `cell_methods/player/interaction.rs:17-20` stub |

### NetOut events with NO server handler (warn-arm only)

Ghidra-confirmed via string registration but no msg_id arm in `dispatch.rs`
and no cell-method arm in any per-interface dispatcher:

| Ghidra string addr | event class | server-side gate required |
|---|---|---|
| `019b9b50` | `Event_NetOut_SendGMShout` | `access_level >= GameMaster` |
| `019b9b80` | `Event_NetOut_Petition` | rate-limit + sender from session |
| `019b30c0` | `Event_NetOut_ChatFriend` | self-add guard + size cap |
| `019be840` | `Event_NetOut_ChatIgnore` | self-add guard + size cap |
| `019b9a90` | `Event_NetOut_ChatOp` | per-channel op-bit (server-tracked) |
| `019b9a38` | `Event_NetOut_ChatMute` | per-channel op-bit |
| `019b9a64` | `Event_NetOut_ChatKick` | per-channel op-bit |
| `019b9ab8` | `Event_NetOut_ChatBan` | per-channel op-bit |
| `019b9ae4` | `Event_NetOut_ChatPassword` | per-channel op-bit |

### Speaker flags (matches `python/base/Chat.py::getSpeakerFlags`)

```
SPEAKER_GM       = 0x01  -- set when c.access_level > 0
SPEAKER_Petition = 0x02  -- enum-only, never set by Python reference; intentionally omitted
SPEAKER_DND      = 0x04  -- set when c.dnd_message is Some
```

`dispatch.rs:131-141` computes these from `ConnectedClientState` exclusively —
**never from any wire field**. This is the canonical "GM bit lives on the
session, not the inbound packet" example in the codebase. Tests 1-4 at
`dispatch.rs:588-660` regression-pin the byte-exact behavior.

### Channel IDs (`python/Atrea/enums.py::EChannel`)

```
0  CHAN_SAY      -- spatial, AoI broadcast
1  CHAN_EMOTE    -- spatial, AoI broadcast
2  CHAN_YELL     -- spatial, wider AoI broadcast
3  CHAN_TEAM     -- team only
4  CHAN_SQUAD    -- squad only
5  CHAN_COMMAND  -- guild/command
6  CHAN_OFFICER  -- guild officer
7  CHAN_SERVER   -- system broadcasts only -- NOT from clients
8  CHAN_FEEDBACK -- system feedback
9  CHAN_TELL     -- direct P2P (handled by BaseApp, not cell)
10 CHAN_SPLASH   -- splash screen
```

Cell-side `chat::handle_chat_message` (`chat.rs:65-95`) gates broadcast to
CHAN_SAY/EMOTE/YELL only. CHAN_TELL falls through to debug-log
(tell-routing is unimplemented). The client cannot set CHAN_SERVER and have
it broadcast through Cimmeria's current code, but if future PRs add
CHAN_TEAM/SQUAD arms without a per-channel membership check, an attacker
can chat into channels they're not a member of. → CAT-L-03.

### Witness fan-out invariant

`broadcast_to_witnesses` reads from `space_mgr.get_entity(sender_id).witnesses`
(`cell/chat.rs:101-119`), which is server-computed AoI. NO client-supplied
target list. This is the right pattern — preserve it on any future
per-channel broadcast addition.
