---
title: "Chat System"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Chat System

> **Last updated**: 2026-09-27
> **Status**: Spatial chat (say / emote / yell) works. Channel management, moderation, tells, and petitions are not implemented — an earlier "~95%" figure described the original Python `Chat.py`, not this server. Sending on any non-spatial channel (team, squad, command, server, tell) no longer disappears silently: the sender gets a feedback line on the registered `tell`/feedback channel explaining why, matching the legacy `onError` reply the Python cell sent for the same unsupported channels (`python/cell/SGWPlayer.py::processPlayerCommunication`).

## Overview

The chat system provides multi-channel text communication between players. It supports system channels (say, emote, yell, team, squad, command, officer, server, feedback, tell, splash) and user-created channels (chat, roleplay, alliance). Messages on cell-based channels are forwarded to the CellApp for spatial distribution; other messages are handled on the BaseApp.

The `Communicator` interface defines the entity-level chat API. The Rust implementation is split between [`base/dispatch/chat.rs`](../../crates/base/src/base/dispatch/chat.rs) (inbound base methods), [`cell/console/chat/`](../../crates/cell-console/src/cell/console/chat/mod.rs) (spatial fanout), and [`base/world_entry_chat.rs`](../../crates/base-session/src/base/world_entry_chat.rs) (channel registration at world entry).

## Implementation Status

Only five SGWPlayer base methods are dispatched at all — `chatJoin` (0xC0), `chatLeave` (0xC1), `sendPlayerCommunication` (0xC2), `chatSetAFKMessage` (0xC3), and `chatSetDNDMessage` (0xC4). Every other base method in the interface below is undispatched: a client that sends it falls into the unhandled-method path.

| Feature | Status | Notes |
|---------|--------|-------|
| Spatial channels (say / emote / yell) | DONE | `cell/console/chat/spatial.rs` broadcasts `onPlayerCommunication` to every AoI witness of the speaker |
| Channel registration on login | DONE | 8 channels pushed at `onClientReady` — see [System Channels](#system-channels) |
| DND status | DONE | `chatSetDNDMessage` sets/clears the flag; a message of 2+ characters sets DND, shorter clears it; the stored text is truncated to 128 Unicode scalar values |
| Speaker flags | PARTIAL | Only `GM` (0x01, from `access_level > 0`) and `DND` (0x04) are computed. No platoon-leader flag |
| Flood limit | DONE (not client-tested) | Every line is checked against a per-player bucket on the base before it reaches the cell: 5 back to back, then one a second. GameMaster and above are exempt. See [Flood limit and length cap](#flood-limit-and-length-cap) |
| Text rules | DONE (not client-tested) | A line over 255 UTF-16 units, or with a control, bidi, zero-width or other invisible formatting character, is refused, not truncated or cleaned |
| GM console passthrough | DONE | A `.`-prefixed say from a GM is routed to the console handler; from a non-GM it falls through as ordinary chat |
| Channel join / leave | ACK-ONLY | `chatJoin` / `chatLeave` parse their payload, log, and return. Channels are auto-joined at login; there is no join/leave state to change |
| AFK status | ACK-ONLY | `chatSetAFKMessage` is deliberately log-only — AFK is not a speaker flag, and the auto-reply-tell path it feeds is unported |
| Non-spatial channels (team / squad / command / server) | NOT IMPL | Registered with the client so the UI shows them, but the cell has no group/organization backing (team/squad/command) or is server-broadcast-only (server) to distribute the message; the sender gets a feedback line (`onPlayerCommunication` on the feedback channel) instead of a silent drop — see [System Channels](#system-channels) |
| Player-to-player tell | NOT IMPL | `tell` (channel 9) is registered and used for one-way server→client messages (welcome text, GM feedback), but no player-originated tell is routed; the sender gets the same "not supported yet" feedback line rather than silence |
| User channels | NOT IMPL | No create / delete / password / member list |
| Channel operator system | NOT IMPL | `chatOp` undispatched |
| Channel moderation | NOT IMPL | `chatMute`, `chatKick`, `chatBan` undispatched |
| Channel password | NOT IMPL | `chatPassword` undispatched |
| Ignore list | NOT IMPL | `chatIgnore` undispatched. The [contact list](contact-list.md) system does persist an `Ignore` list, but nothing consults it to suppress messages |
| Friend list (nicknames) | NOT IMPL | `chatFriend` / `onNickChanged` undispatched |
| Petition system | NOT IMPL | `petition`, `announcePetition` undispatched |
| GM broadcast | DONE (not client-tested) | `/gmshout` (cell method 222 `sendGMShout`) and `.announce [space] <text>` send the GM's line to the GM's space or to every online player, on the server channel with the GM speaker flag. GameMaster and above only. See [GM broadcast](#gm-broadcast). The legacy `hearGMShout` hop is not used |
| Localized communication | NOT IMPL | `onLocalizedCommunication` never sent |
| Channel list | NOT IMPL | `chatList` undispatched |

DND text is capped server-side at 128 Unicode scalar values, not UTF-8 bytes or UTF-16 code units. This is a Cimmeria input policy; no client limit was reverse-engineered. A longer message still turns DND on, so the player sees `/dnd` take effect, but only the first 128 scalars are stored; truncation logs at DEBUG with `reason = "dnd_message_truncated"`, the length and the limit, without the message body. Malformed WSTRING input also preserves the previous state. DND auto-replies remain unimplemented.

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
| team | 3 | yes | No group backing yet — sender gets a "not supported yet" feedback line instead of a silent drop |
| squad | 4 | yes | No group backing yet — sender gets a "not supported yet" feedback line instead of a silent drop |
| command | 5 | yes | No organization backing yet — sender gets a "not supported yet" feedback line instead of a silent drop |
| officer | 6 | **no** | Not registered; would trigger the unknown-channel popup. A player message that somehow arrives with this id still gets routed through the same feedback path (which always replies on the registered feedback channel, never channel 6 itself) |
| server | 7 | yes | Server-to-client broadcasts only (`CHANNEL_FLAG_DisallowPlayerMessages` in the legacy `ChatChannelManager`). A player who tries to speak here gets a feedback line explaining the channel is system-only — legacy only logged this server-side and left the client in silence |
| tell | 9 | yes | Used server-to-client for the welcome message and GM feedback. There is no dedicated feedback channel (8 is unregistered), so GM feedback — and now the "channel not supported" reply for team/squad/command/officer/server/tell — rides `tell` |
| splash | — | **no** | Not registered |

> **Id conflict (2026-09-27).** `EChannel` in `entities/defs/enumerations.xml:113-128` numbers the channels server 8, feedback 9, tell 10, splash 11 and user channels from 12, with 7 unused; the legacy `deprecated/python/base/Chat.py:144-154` builds every channel from those values. The ids in this table are what the Rust server registers today (`DEFAULT_CHAT_CHANNELS`), not the enum. Whether the client hardcodes any `EChannel` id, or takes every id from `onChatJoined`, is still to be checked; the organizations campaign changes the registered ids only with that evidence ([decision D-ORG14](../analysis/organizations/README.md#decisions), [audit A-40](../analysis/organizations/audit.md)).

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

## Flood limit and length cap

`sendPlayerCommunication` runs two gates on the base, in this order, before the line is forwarded to the cell (SS-00, decisions D-SS14 and D-SS12 in `docs/analysis/social-systems/README.md`):

1. **Flood limit.** Each player has a chat token bucket: a burst of 5 lines, then one more per second. A line with no token is dropped. The first drop sends the player "You are sending messages too quickly." on the feedback channel; further drops inside the next 5 seconds are silent, so a flood never turns into a flood of replies. Access level GameMaster (2) and above skip the bucket. The bucket covers every channel the player sends on, tells included.
2. **Text rules.** A line longer than 255 UTF-16 units (the unit of the client's `WSTRING`; a character outside the Basic Multilingual Plane counts two) is refused with "Your message is too long." A line containing a control character (tab and newline included), a bidi control, a zero-width or other format character, or a line or paragraph separator is refused with "Your message contains a character that cannot be sent." These are the organizations campaign's D-ORG10 rules, applied through the same function (`org_text::validate(TextField::ChatText, ..)`), not a second filter. A refused line still used up a token, so bad lines cannot be spammed past the flood limit.

The numbers are project policy, not recovered client data. The cap may come down to the client's own input limit once SS-E1 reports it (it never goes above 255). Drops log `rate_limit.exceeded` and refusals `chat.rejected`; see the `rate_limit` and `chat` rows of the target catalog in [observability.md](../architecture/observability.md).

## GM broadcast

A GameMaster (access level 2) or higher can send one line to many players (SS-C2, decision D-SS16 in `docs/analysis/social-systems/README.md`):

| Entry point | Scope | Notes |
|---|---|---|
| `/gmshout` (native, cell method 222 `sendGMShout(UINT8 isGlobal, WSTRING Text)`) | `isGlobal = 0`: the GM's space instance. Any other value: every online player | The client sends it straight from the slash command, with no Lua in between. How the client splits the typed text into the two arguments is not recovered |
| `.announce <text>` | Every online player | For a client without the slash binding. The typed words are re-joined with single spaces |
| `.announce space <text>` | The GM's space instance | `space` is matched in any case and only as the first word |

Every recipient gets `onPlayerCommunication(<GM's character name>, SPEAKER_GM, CHAN_SERVER, text)`, the GM included, so the GM sees the line as the players do. The channel is whatever `CHAN_SERVER` holds (7 today; the organizations campaign owns that id, D-ORG14). The space scope is sent by the cell, which knows the players in the GM's space; the global scope goes through the base, which sends it to every session in the online name index that has a player entity (character select and mid-world-entry sessions are skipped).

The GM gate runs before anything else: cell method 222 is in the SGWGmPlayer tail, so a caller below GameMaster gets `onErrorCode` and the shout goes nowhere (`gm_gate`, security finding CAT-L-06). The text follows the chat rules above (255 UTF-16 units at most, no control or format characters) and blank text is refused; each refusal tells the GM why. There is no flood limit on GM broadcasts, the same exemption GMs have on ordinary chat.

SigNoz: `chat.gm_broadcast` (INFO, the audit row: actor ids, `scope`, `source` = `native` or `console`, `space_id`, the text), `chat.gm_broadcast_delivered` (INFO, the recipient count per scope), `chat.gm_broadcast_rejected` (WARN, `reason` = `empty_text`, `too_long`, `malformed_args`, `no_text`, ...).

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
