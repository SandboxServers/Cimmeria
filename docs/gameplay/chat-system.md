---
title: "Chat System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Chat System

> **Last updated**: 2026-07-25
> **Status**: Spatial chat (say / emote / yell) works. Channel management, moderation, tells, and petitions are not implemented — an earlier "~95%" figure described the original Python `Chat.py`, not this server.

## Overview

The chat system provides multi-channel text communication between players. It supports system channels (say, emote, yell, team, squad, command, officer, server, feedback, tell, splash) and user-created channels (chat, roleplay, alliance). Messages on cell-based channels are forwarded to the CellApp for spatial distribution; other messages are handled on the BaseApp.

The `Communicator` interface defines the entity-level chat API. The Rust implementation is split between [`base/dispatch/chat.rs`](../../crates/services/src/base/dispatch/chat.rs) (inbound base methods), [`cell/chat.rs`](../../crates/services/src/cell/chat.rs) (spatial fanout), and [`base/world_entry_chat.rs`](../../crates/services/src/base/world_entry_chat.rs) (channel registration at world entry).

## Implementation Status

Only five SGWPlayer base methods are dispatched at all — `chatJoin` (0xC0), `chatLeave` (0xC1), `sendPlayerCommunication` (0xC2), `chatSetAFKMessage` (0xC3), and `chatSetDNDMessage` (0xC4). Every other base method in the interface below is undispatched: a client that sends it falls into the unhandled-method path.

| Feature | Status | Notes |
|---------|--------|-------|
| Spatial channels (say / emote / yell) | DONE | `cell/chat.rs` broadcasts `onPlayerCommunication` to every AoI witness of the speaker |
| Channel registration on login | DONE | 8 channels pushed at `onClientReady` — see [System Channels](#system-channels) |
| DND status | DONE | `chatSetDNDMessage` sets/clears the flag; a message of 2+ characters sets DND, shorter clears it |
| Speaker flags | PARTIAL | Only `GM` (0x01, from `access_level > 0`) and `DND` (0x04) are computed. No platoon-leader flag |
| GM console passthrough | DONE | A `.`-prefixed say from a GM is routed to the console handler; from a non-GM it falls through as ordinary chat |
| Channel join / leave | ACK-ONLY | `chatJoin` / `chatLeave` parse their payload, log, and return. Channels are auto-joined at login; there is no join/leave state to change |
| AFK status | ACK-ONLY | `chatSetAFKMessage` is deliberately log-only — AFK is not a speaker flag, and the auto-reply-tell path it feeds is unported |
| Non-spatial channels (team / squad / command / server) | NOT IMPL | Registered with the client so the UI shows them, but the cell's channel match has no arm — messages hit the `_ =>` debug-log fallthrough and go nowhere |
| Player-to-player tell | NOT IMPL | `tell` (channel 9) is registered and used for one-way server→client messages (welcome text, GM feedback), but no player-originated tell is routed |
| User channels | NOT IMPL | No create / delete / password / member list |
| Channel operator system | NOT IMPL | `chatOp` undispatched |
| Channel moderation | NOT IMPL | `chatMute`, `chatKick`, `chatBan` undispatched |
| Channel password | NOT IMPL | `chatPassword` undispatched |
| Ignore list | NOT IMPL | `chatIgnore` undispatched. The [contact list](contact-list.md) system does persist an `Ignore` list, but nothing consults it to suppress messages |
| Friend list (nicknames) | NOT IMPL | `chatFriend` / `onNickChanged` undispatched |
| Petition system | NOT IMPL | `petition`, `announcePetition` undispatched |
| GM shout | NOT IMPL | `hearGMShout` never sent |
| Localized communication | NOT IMPL | `onLocalizedCommunication` never sent |
| Channel list | NOT IMPL | `chatList` undispatched |

## Entity Definition (Communicator.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `ignoredList` | ARRAY\<WSTRING\> | BASE | Players being ignored |
| `channels` | ARRAY\<PYTHON\> | CELL_PRIVATE | Subscribed channel data |
| `AFK` | UINT8 | BASE | AFK status flag |
| `DND` | UINT8 | BASE | Do-Not-Disturb status flag |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onSystemCommunication` | TextType, StringId, Speaker, tokenList | System message |
| `onPlayerCommunication` | Speaker, SpeakerFlags, Channel, Text | Player message |
| `onLocalizedCommunication` | Speaker, SpeakerFlags, Channel, Text, tokenList | Localized message |
| `onTellSent` | Target, Text | Confirm tell delivery |
| `onChatJoined` | ChannelName, ChannelID | Joined channel notification |
| `onChatLeft` | ChannelName | Left channel notification |
| `onNickChanged` | PlayerName, PlayerNickname, AddRemoveFlag | Friend nickname change |

### Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `processPlayerCommunication` | Speaker, SpeakerFlags, Target, Channel, Text | Distribute cell-based message |

### Base Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `chatJoin` | YES | ChannelName, Password | Join user channel |
| `chatLeave` | YES | ChannelID | Leave channel |
| `sendPlayerCommunication` | YES | Channel, Target, Text | Send chat message |
| `chatSetAFKMessage` | YES | Message | Set AFK message |
| `chatSetDNDMessage` | YES | Message | Set DND message |
| `chatIgnore` | YES | PlayerName, Flag | Add/remove ignore |
| `chatFriend` | YES | PlayerName, Nickname, Flag | Add/remove friend |
| `chatList` | YES | ChannelID | List channel members |
| `chatMute` | YES | ChannelID, PlayerName, Flag | Mute/unmute player |
| `chatKick` | YES | ChannelID, PlayerName | Kick from channel |
| `chatOp` | YES | ChannelID, PlayerName | Promote to operator |
| `chatBan` | YES | ChannelID, PlayerName, Flag | Ban/unban from channel |
| `chatPassword` | YES | ChannelID, Password | Set channel password |
| `petition` | YES | Message | Submit GM petition |
| `announcePetition` | YES | Message | Announce petition |

## System Channels

Eight channels are registered with the client at `onClientReady` (`DEFAULT_CHAT_CHANNELS` in `base/world_entry_chat.rs`). Sending on an **unregistered** channel id makes the client raise its red unknown-channel splash popup, so the server must stay inside this set:

| Channel | Id | Registered | Server behaviour |
|---------|----|------------|------------------|
| say | 0 | yes | Spatial fanout to AoI witnesses |
| emote | 1 | yes | Spatial fanout to AoI witnesses |
| yell | 2 | yes | Spatial fanout to AoI witnesses (same radius as say today — no wider range implemented) |
| team | 3 | yes | Accepted from the client, then dropped — no group backing |
| squad | 4 | yes | Accepted from the client, then dropped — no group backing |
| command | 5 | yes | Accepted from the client, then dropped — no organization backing |
| officer | 6 | **no** | Not registered; would trigger the unknown-channel popup |
| server | 7 | yes | Server-to-client broadcasts only |
| tell | 9 | yes | Used server-to-client for the welcome message and GM feedback. There is no dedicated feedback channel (8 is unregistered), so GM feedback rides `tell` |
| splash | — | **no** | Not registered |

## Channel Flags

| Flag | Constant | Purpose |
|------|----------|---------|
| `CHANNEL_FLAG_OnCell` | -- | Messages processed on CellApp |
| `CHANNEL_FLAG_DisallowPlayerMessages` | -- | Players cannot speak |
| `CHANNEL_FLAG_KeepIfEmpty` | -- | Channel persists when empty |

## Speaker Flags (ESpeakerFlags)

Computed in `base/dispatch/mod.rs::speaker_flags` and stamped onto every outbound `onPlayerCommunication`:

| Flag | Value | Set when |
|------|-------|----------|
| `GM` | 0x01 | Speaker's `access_level > 0` (Moderator or higher) |
| `Petition` | 0x02 | Defined in the enum; never set — the petition path is unimplemented |
| `DND` | 0x04 | Speaker has a non-empty DND auto-reply message |

## Data References

- **Enumerations**: `EChannel` (`CHAN_say` … `CHAN_splash`), `ESpeakerFlags`
- **Channel registration**: `DEFAULT_CHAT_CHANNELS` in `base/world_entry_chat.rs`
- **Base-method ids**: `sgw_player_base` module in `base/dispatch/mod.rs`; full table in [sgwplayer-base-method-dispatch-table.md](../protocol/sgwplayer-base-method-dispatch-table.md)

## Remaining Work

1. **Player-to-player tell** — the highest-value gap; `tell` is registered and the client UI expects it
2. **Group / organization channels** — team, squad, command are registered but have no membership backing; blocked on the [group system](group-system.md)
3. **Yell radius** — say, emote, and yell all fan out to the same AoI witness set; yell should use a wider range
4. **Ignore enforcement** — wire the contact-list `Ignore` list into the chat fanout
5. **User channels + moderation** — create/join/password/op/mute/kick/ban are all undispatched
6. **AFK auto-reply** — `chatSetAFKMessage` is accepted but the auto-reply-tell path it feeds does not exist
7. **NPC speech** — how `onSystemCommunication`'s Speaker field works for NPCs is still unrecovered

## Related Docs

- [organization-system.md](organization-system.md) - Guild channels (command, officer)
- [group-system.md](group-system.md) - Squad/team channels
