---
title: "Chat System"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Chat System

> **Last updated**: 2026-09-28
> **Status**: Spatial chat (say / emote / yell) works. The social-systems campaign (merged 2026-09-27, not yet tested with two real clients; the owner's [SS-UAT](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) covers it) added a flood limit and text rules (SS-00), tells, `chatIgnore` and the one-way Ignore filter (SS-C1), GM broadcast (SS-C2), a channel allowlist, GM mutes and a feedback line for every Communicator method the server does not implement (SS-C3), and channel ids that match the client's own, with no channel registration at login (SS-C4). Squad chat (organizations ORG-04) and team, command and officer chat (ORG-09) work; the organization channels are not yet tested with two real clients. User channels (issue #1039) work: `chatJoin` creates or joins a named channel and `chatLeave` leaves it, on a process-wide in-memory registry. Channel moderation (op/kick/ban/password) and petitions are still not implemented — an earlier "~95%" figure described the original Python `Chat.py`, not this server.

## Overview

The chat system provides multi-channel text communication between players. It supports system channels (say, emote, yell, team, squad, command, officer, server, feedback, tell, splash) and user-created channels (chat, roleplay, alliance, and any name a player picks). Messages on cell-based channels are forwarded to the CellApp for spatial distribution; user channels have no spatial component and are distributed entirely on the BaseApp; other messages are also handled on the BaseApp.

The `Communicator` interface defines the entity-level chat API. The Rust implementation is split between [`base/dispatch/chat.rs`](../../crates/base/src/base/dispatch/chat.rs) (inbound base methods, including `chatJoin`/`chatLeave`/user-channel posts), [`base/user_channels/`](../../crates/base-session/src/base/user_channels/) (the process-wide user-channel registry), [`cell/console/chat/`](../../crates/cell-console/src/cell/console/chat/mod.rs) (spatial fanout), [`base/dispatch/tell.rs`](../../crates/base/src/base/dispatch/tell.rs) and [`base/dispatch/ignore.rs`](../../crates/base/src/base/dispatch/ignore.rs) (tells and `chatIgnore`), [`base/dispatch/chat_gates.rs`](../../crates/base/src/base/dispatch/chat_gates.rs) (the channel allowlist and the mute gate), [`base/dispatch/communicator_unsupported.rs`](../../crates/base/src/base/dispatch/communicator_unsupported.rs) (the 0xC6-0xCE feedback arms), [`base/mutes/`](../../crates/base-session/src/base/mutes/) (the mute table and `.mute` / `.unmute`), [`base/contact_list/ignore/`](../../crates/base-session/src/base/contact_list/ignore/) (the Ignore cache), and [`base/world_entry_chat.rs`](../../crates/base-session/src/base/world_entry_chat.rs) (the login welcome line; no built-in channel is registered at login).

## Implementation Status

Seven Communicator base methods do something — `chatJoin` (0xC0), `chatLeave` (0xC1), `sendPlayerCommunication` (0xC2), `chatSetAFKMessage` (0xC3), `chatSetDNDMessage` (0xC4), `chatIgnore` (0xC5) and, as a channel `sendPlayerCommunication` may target, the user channels `chatJoin`/`chatLeave` create. The other eight (0xC6-0xCE minus `chatJoin`/`chatLeave`) each answer with a "not available yet" feedback line and do nothing else (SS-C3); see [Methods that are not available](#methods-that-are-not-available).

| Feature | Status | Notes |
|---------|--------|-------|
| Spatial channels (say / emote / yell) | DONE | `cell/console/chat/spatial.rs` broadcasts `onPlayerCommunication` to every player AoI witness of the speaker, except a witness who ignores the speaker. The speaker gets their own echo for `emote` and `yell` (even with nobody in range), but never for `say` — the client shows its own `say` line locally, so a server echo doubles it. See [chat-speaker-echo.md](../reverse-engineering/findings/chat-speaker-echo.md) |
| Channel registration on login | DONE (none needed for built-ins) | No `onChatJoined` for a built-in channel: the client hardcodes every one of them (SS-C4, D-ORG14) — see [System Channels](#system-channels). The client's own three default *user* channels (`channel-chat`/`channel-roleplay`/`channel-alliance`, or `chat`/`roleplay`/`alliance` for a non-GM) DO get `onChatJoined`, because they are ordinary `chatJoin` calls the client sends itself — see [User Channels](#user-channels) |
| DND status | DONE | `chatSetDNDMessage` sets/clears the flag; a message of 2+ characters sets DND, shorter clears it; the stored text is truncated to 128 Unicode scalar values |
| Speaker flags | PARTIAL | Only `GM` (0x01, from `access_level > 0`) and `DND` (0x04) are computed. No platoon-leader flag |
| Flood limit | DONE (not client-tested) | Every line is checked against a per-player bucket on the base before it reaches the cell: 5 back to back, then one a second. GameMaster and above are exempt. See [Flood limit and length cap](#flood-limit-and-length-cap) |
| Channel allowlist | DONE (not client-tested) | The base refuses a line on the server, feedback or splash channel, or on an id `EChannel` does not name, with a feedback line, before the cell sees it. A user channel id (12 and up) passes this allowlist by shape; membership is checked downstream. See [Channel allowlist and mutes](#channel-allowlist-and-mutes) |
| GM mute | DONE (not client-tested) | `.mute <name> <minutes> [reason]` and `.unmute <name>` (GameMaster and above). A muted player's say, emote, yell, organization lines and tells are refused with the time left. See [Channel allowlist and mutes](#channel-allowlist-and-mutes) |
| Text rules | DONE (not client-tested) | A line over 255 UTF-16 units, or with a control, bidi, zero-width or other invisible formatting character, is refused, not truncated or cleaned |
| GM console passthrough | DONE | A `.`-prefixed say from a GM is routed to the console handler; from a non-GM it falls through as ordinary chat |
| Channel join / leave | DONE (not client-tested) | `chatJoin` creates the named user channel or joins the existing one; `chatLeave` leaves it. See [User Channels](#user-channels) |
| AFK status | DONE (not client-tested) | `chatSetAFKMessage` stores the away message under the DND rules (2+ characters sets it, 128-scalar bound). It is not a speaker flag; a tell to an away player is answered with it. See [Tells and Ignore](#tells-and-ignore) |
| Organization channels (team / squad / command / officer) | DONE (not client-tested) | Squad lines reach every squad member (organizations ORG-04, `cell/console/chat/squad.rs`). Team, command and officer lines are handled on the base and reach the online members of the speaker's Team or Command, wherever they are (ORG-09, `base/organization/handlers/chat.rs`); see [Organization channels](#organization-channels). A player line on server (8) is refused at the base — see [System Channels](#system-channels) |
| Player-to-player tell | DONE (not client-tested) | Handled on the base, never forwarded to the cell: the client sends tells on channel 10 and the recipient gets them on 10. See [Tells and Ignore](#tells-and-ignore) |
| User channels | DONE (not client-tested) | `chatJoin` creates or joins a named channel (case-insensitive), `chatLeave` leaves it, and a post reaches every member. Global across worlds and cell spaces; no create/delete/password/member-list UI beyond join/leave. See [User Channels](#user-channels) |
| Channel operator system | NOT IMPL | `chatOp` answers with a feedback line |
| Channel moderation | NOT IMPL | `chatMute`, `chatKick`, `chatBan` answer with a feedback line. The GM mute is `.mute`, not `chatMute` |
| Channel password | NOT IMPL | `chatPassword` answers with a feedback line; the password is never decoded, and no channel `chatJoin` creates ever has one |
| Ignore list | DONE (not client-tested) | `chatIgnore` edits the [contact list](contact-list.md)'s `Ignore` list; tells and spatial chat honour it, one way (D-SS15). See [Tells and Ignore](#tells-and-ignore) |
| Friend list (nicknames) | NOT IMPL | `chatFriend` answers with a feedback line; `onNickChanged` is never sent |
| Petition system | NOT IMPL | `petition`, `announcePetition` answer with a feedback line |
| GM broadcast | DONE (not client-tested) | `/gmshout` (cell method 222 `sendGMShout`) and `.announce [space] <text>` send the GM's line to the GM's space or to every online player, on the server channel with the GM speaker flag. GameMaster and above only. See [GM broadcast](#gm-broadcast). The legacy `hearGMShout` hop is not used |
| Localized communication | NOT IMPL | `onLocalizedCommunication` never sent |
| Channel list | NOT IMPL | `chatList` answers with a feedback line |

DND text is capped server-side at 128 Unicode scalar values, not UTF-8 bytes or UTF-16 code units. This is a Cimmeria input policy; no client limit was reverse-engineered. A longer message still turns DND on, so the player sees `/dnd` take effect, but only the first 128 scalars are stored; truncation logs at DEBUG with `reason = "dnd_message_truncated"`, the length and the limit, without the message body. Malformed WSTRING input also preserves the previous state. A tell to a player in DND is still delivered, and the sender gets the DND text back (see [Tells and Ignore](#tells-and-ignore)).

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
| `onChatJoined` | ChannelName, ChannelID | Joined a **user** channel. `ChannelID` is the display id (channel id minus 12); never sent for a built-in channel |
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

Every built-in channel id is an `EChannel` value from `entities/defs/enumerations.xml`, and the client compiles the same values in as literals (`UIChannel.Say` … `UIChannel.Splash`; [ORG-E1 Q5](../reverse-engineering/findings/organization-restoration.md), decision [D-ORG14](../analysis/organizations/README.md#decisions)). The client needs no registration to send or show a line on any of them. The Rust constants are `CHAN_*` in `crates/wire/src/cell/chat.rs`, pinned against the XML by `chan_constants_match_enumerations_xml`. SS-C4 (2026-09-27) moved them onto the enum; before that the server used 7 for server, 9 for tell and 10 for splash.

| Channel | Id | Client display (`ChatWindow.lua`) | Server behaviour |
|---------|----|-----------------------------------|------------------|
| say | 0 | White, Info tab | Spatial fanout to AoI witnesses |
| emote | 1 | Yellow | Spatial fanout to AoI witnesses |
| yell | 2 | Light red | Spatial fanout to AoI witnesses (same radius as say today — no wider range implemented) |
| team | 3 | Violet | Handled on the base: the speaker's Team (ORG-09) |
| squad | 4 | Turquoise | Relayed to the speaker's squad by the cell (ORG-03) |
| command | 5 | Green | Handled on the base: the speaker's Command (ORG-09) |
| officer | 6 | Green | Handled on the base: the members of the speaker's Command whose rank holds `OfficerChat` (ORG-09) |
| server | 8 | Bright red line (`:1249`) **and a modal "Server Message" prompt** with an OK button (`:160-162`) | Server-to-client broadcasts only: `/gmshout` and `.announce` ([GM broadcast](#gm-broadcast)). A player line on 8 is refused at the base |
| feedback | 9 | Sky blue, Info tab (`:1250`, `:1274`) | Server-to-client system lines to one player: the login welcome, GM feedback, every refusal line. A player line on 9 is refused at the base |
| tell | 10 | Purple (red for a GM speaker) | Player-to-player, handled on the base ([Tells and Ignore](#tells-and-ignore)) |
| splash | 11 | Green | Nothing sends it yet. A player line on 11 is refused at the base |

Id 7 is not an `EChannel` value. The client has no `ChatMod.ChannelMap` entry for it (`ChatWindow.lua:1297-1312`), and `onMessageReceived` calls `ChannelMap[channelId](...)` for every id below 12 (`:93-104`), so a line on 7 raises a Lua error in the chat window and shows nothing. `/gmshout` and `.announce` sent on 7 until SS-C4, so they were invisible. No server-to-client send may use 7, pinned by `no_chan_constant_is_seven` and `no_player_communication_call_uses_a_literal_channel`.

**No registration.** The server sends no `onChatJoined` at login (SS-C4, decided 2026-09-27):

- The client hardcodes ids 0 to 11, so a built-in channel needs no registration (ORG-E1 Q5).
- The client treats every `onChatJoined(name, id)` as a **user** channel: `ChatMod.onChannelJoined` files it under `UIChannel.Chat + id` and prints "You have joined channel [id:name]" on feedback (`ChatWindow.lua:370-386`, `ChatWindow.int:107`).
- The legacy server sent `onChatJoined` only for user channels (id 12 and up), carrying the id minus 12 (`deprecated/python/base/SGWPlayer.py:162-163`). Its `playerLoggedIn` joined the server channel on the server side only (`deprecated/python/base/Chat.py:183-190`).

Until SS-C4 the Rust server sent eight `onChatJoined` (say through command, server, tell) with every login, which made eight bogus user channels on the client. `on_client_ready_burst_registers_no_built_in_channel` pins their absence. User channels (issue #1039, below) send `onChatJoined` from their own join path, not from login.

**Welcome line.** The login welcome ("Welcome to Stargate Worlds. Your player id is: N.") goes out on feedback (9), the channel the legacy `cell/SGWPlayer.py:541` names with a literal 9. It is not sent on server (8), because the client would open its modal prompt on every login.

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
- **Channel ids**: the `CHAN_*` constants in `crates/wire/src/cell/chat.rs`, pinned to `EChannel`; no channel is registered at login (`crates/base-session/src/base/world_entry_chat.rs` module doc)
- **Base-method ids**: `sgw_player_base` module in `base/dispatch/mod.rs`; full table in [sgwplayer-base-method-dispatch-table.md](../protocol/sgwplayer-base-method-dispatch-table.md)

## Flood limit and length cap

`sendPlayerCommunication` runs two gates on the base, in this order, before the line is forwarded to the cell (SS-00, decisions D-SS14 and D-SS12 in `docs/analysis/social-systems/README.md`):

1. **Flood limit.** Each player has a chat token bucket: a burst of 5 lines, then one more per second. A line with no token is dropped. The first drop sends the player "You are sending messages too quickly." on the feedback channel; further drops inside the next 5 seconds are silent, so a flood never turns into a flood of replies. Access level GameMaster (2) and above skip the bucket. The bucket covers every channel the player sends on, tells included.
2. **Text rules.** A line longer than 255 UTF-16 units (the unit of the client's `WSTRING`; a character outside the Basic Multilingual Plane counts two) is refused with "Your message is too long." A line containing a control character (tab and newline included), a bidi control, a zero-width or other format character, or a line or paragraph separator is refused with "Your message contains a character that cannot be sent." These are the organizations campaign's D-ORG10 rules, applied through the same function (`org_text::validate(TextField::ChatText, ..)`), not a second filter. A refused line still used up a token, so bad lines cannot be spammed past the flood limit.

The numbers are project policy, not recovered client data. The client's chat input has no length limit of its own (SS-E1 C-Q3), so 255 is the only cap. Drops log `rate_limit.exceeded` and refusals `chat.rejected`; see the `rate_limit` and `chat` rows of the [target catalog](../architecture/observability-target-catalog.md).

## Channel allowlist and mutes

SS-C3 (2026-09-27), security finding CAT-L-03 and decision D-SS26 in `docs/analysis/social-systems/README.md`. Both gates run in `sendPlayerCommunication` on the base, after the flood limit and before the text rules, so a refused line still costs a chat token.

**Channel allowlist.** The ids are the `EChannel` values the client sends (D-ORG14), the same `CHAN_*` constants as [System Channels](#system-channels). A player may speak on:

| Channel | Id | What happens |
|---|---|---|
| say, emote, yell | 0, 1, 2 | Forwarded to the cell |
| squad | 4 | Forwarded to the cell, which relays it to the squad (ORG-04) |
| team, command, officer | 3, 5, 6 | Handled on the base; never reach the cell (ORG-09, [Organization channels](#organization-channels)) |
| tell | 10 | Handled on the base ([Tells and Ignore](#tells-and-ignore)) |

Everything else is refused at the base with one feedback line, and nothing reaches the cell:

| Channel | Id | Feedback | `reason` |
|---|---|---|---|
| server, feedback, splash | 8, 9, 11 | "That channel is for system messages only. Players cannot post there." | `system_channel` |
| a user channel | 12 and up | "Custom chat channels are not available yet." | `user_channel` |
| an id `EChannel` does not name | 7 (the only one below 12) | "That chat channel does not exist." | `unknown_channel` |

The refusal logs `chat.channel_rejected` (WARN) with `channel`, `reason` and the player's ids.

**Mutes.** A GameMaster (access level 2) or higher mutes an online player with `.mute <name> <minutes> [reason]` (1 to 10,080 minutes, 7 days) and lifts it with `.unmute <name>`. The name resolves like a tell target. The GM gets a confirmation or the reason for a refusal: a bad duration, a name nobody online has, a name two players match, or a GM target (a GM is never muted, because GMs use the `.` console over say). The muted player is told "A GM has muted you for N minutes. You cannot chat or send tells until it ends." and, on `.unmute`, "A GM has lifted your mute. You can chat again."

While muted, every line the player sends on an allowed channel, tells included, is refused with "You are muted and cannot chat for another N minutes." A muted player's tell never reaches the recipient, and the sender gets no `onTellSent`. A muted player can still receive tells, but their AFK or DND message is not sent back while the mute lasts. The mute is held on the base by character (`player_id`), so it holds across relog and gate travel. It ends at its expiry, at `.unmute`, or when the server restarts: mutes are not saved (D-SS26). Offline characters cannot be muted or unmuted.

Events, all on the `chat` target: `chat.gm_mute` and `chat.gm_unmute` (INFO, the GM's `entity_id` / `account_id` / `player_id`, `subject_player_id`, `subject_account_id`, `duration_minutes`, `reason` as the GM typed it, `previous_remaining_secs` and `remaining_secs`), `chat.gm_mute_refused` and `chat.gm_unmute_refused` (WARN, `reason` = `usage` \| `bad_duration` \| `bad_reason` \| `bad_name` \| `not_online` \| `ambiguous` \| `target_is_gm` \| `not_muted` \| `base_channel_closed`), and `chat.muted_refused` (DEBUG, `reason = muted`, `channel`, `tell`, `remaining_secs`, `muted_by_account_id`).

## User Channels

Issue #1039. `chatJoin(WSTRING channelName, WSTRING password)` and `chatLeave(UINT8 channelId)` (0xC0/0xC1) were acknowledged no-ops until #840 even routed them to their handlers (before that, the encrypted receive loop sent the in-world bytes to the `ClientCache` methods instead — see the [System Channels](#system-channels) note on `onChatJoined`). The state lives in [`base/user_channels/registry.rs`](../../crates/base-session/src/base/user_channels/registry.rs) (`UserChannelRegistry`), a process-wide table like [`base/mutes/`](../../crates/base-session/src/base/mutes/): in memory only, keyed by wire channel id, one instance per base process (nothing is persisted, so a restart starts with no channels — every client rejoins its saved ones and the three defaults on login regardless).

**Client auto-join.** `ChatWindow.lua`'s `onPropertyUpdate` (fired once on the player's `AccessLevel` property) sends `/chatjoin channel-chat`, `/chatjoin channel-roleplay` and `/chatjoin channel-alliance` for a GM (any non-zero access level), or `/chatjoin chat`/`roleplay`/`alliance` for an ordinary player (`:1109-1141`), and `onModLoaded` resends `/chatjoin` for every channel the client's own saved data (`GChatMod_SavedData.channels`) remembers from its last session (`:1046-1063`). Both are ordinary `chatJoin` calls indistinguishable on the wire from a manual `/chatjoin`, so the server does the same thing for all three: create-or-join, then `onChatJoined`.

**Join.** `chatJoin` normalises the name with the same D-ORG10 rules as an organization name (`TextField::ChannelName`: ASCII letters, digits, space, `'` `-` `.`, ends trimmed, internal whitespace collapsed) and looks it up case-folded (`name_key`, D-SS13 style) against every existing channel. A match joins that channel; no match creates one, with a wire id the registry allocates (the lowest free id from 12 up, reused once a channel that held it is deleted). This is a deliberate improvement over the legacy `Chat.py::ChatChannelManager.joinChannel`, which only ever joined a pre-existing channel and otherwise failed with a server-side warning and *no feedback at all* — the one code path that created a channel, `requestCreateChannel`, was never wired to any base method the client could call, so the legacy server's own players could never actually create a channel. On success the caller gets `onChatJoined(name, wireId - 12)`, which is itself the client's "You have joined channel" line (`ChatWindow.lua::onChannelJoined`) — the server never sends a second feedback line for a success, for an auto-join or a manual one alike, because the wire cannot tell the two apart. A refusal (a bad name, already a member, or either channel-count cap below) gets exactly one feedback line and logs `chat.channel_join_rejected` (WARN).

**Leave.** `chatLeave(displayId)` translates back to the wire id (`displayId + 12`) and removes the caller. Success sends `onChatLeft(name)`, which is the client's "You have left channel" line and the trigger that drops the channel's tab subscriptions (`ChatWindow.lua::onChannelLeft`); again, no separate feedback line follows. The last member leaving deletes the channel immediately and frees its wire id — there is no `CHANNEL_FLAG_KeepIfEmpty` here, because (unlike the legacy server, which pre-created "chat"/"roleplay"/"alliance" at boot before anyone had joined) nothing here ever pre-creates a channel. A `chatLeave` naming a channel the caller is not in (including one that never existed) is refused with "You are not in that chat channel." and logs `chat.channel_leave_rejected` (WARN).

**Post.** A `sendPlayerCommunication` on a channel id 12 and up is a shape-only pass through the allowlist (`chat_gates.rs::check_channel`); `chat.rs::post_to_user_channel` then requires the caller's entity to be a member of that exact wire id before sending anything — a client cannot post to a channel it never joined no matter what byte it sends (server authority). A member post reaches every member, the speaker included: unlike say/emote/yell there is no local client echo to avoid doubling on a user channel, and the legacy `ChatChannel.sendMessage` always sent to every member without excluding the sender. A non-member post is refused with the same "You are not in that chat channel." line and logs `chat.channel_post_rejected` (WARN); a successful post logs `chat.channel_post` (INFO, `recipients`, `text_units`, never the text).

**Scope: global.** A user channel is server-wide, not per-world or per-space, matching the legacy `ChatChannelManager` singleton: gate travel changes a player's cell and space, never their base `SGWPlayer` entity, and membership is keyed on that entity id, so it survives gate travel untouched. Nothing here reaches the cell — unlike squad chat, a user channel has no spatial component to distribute.

**Lifecycle and cleanup.** A character's membership in every channel it holds is dropped when its base entity is destroyed: both `logOff` variants (`base/dispatch/session.rs::handle_log_off`, full exit and return to character select) and the disconnect/timeout/duplicate-login teardown (`base-session/helpers/mod.rs::destroy_client_entities`) call `UserChannelRegistry::leave_all(entity_id)` — the same call sites that already drop queued crafting inductions. No `onChatLeft` is sent for this: there is no client left to tell. Gate travel is deliberately **not** one of these sites (the entity id it reuses is the same one channel membership is keyed on).

**Limits (project policy, not recovered data).** A channel name is 1-32 UTF-16 units (`MAX_CHANNEL_NAME_UNITS`). A player may hold at most 10 channels at once (`MAX_CHANNELS_PER_PLAYER`); joining an 11th is refused with "You are in too many chat channels already." The server holds at most 200 channels at once (`MAX_USER_CHANNELS`), well under the 256-id ceiling the wire's `UINT8` display id imposes; past either cap, or if every wire id up to `u8::MAX` is somehow taken, a join is refused with "No more chat channels can be created right now." None of these numbers were recovered from the legacy server, which bounded neither.

## Methods that are not available

Each Communicator base method the server does not implement answers the press with its own line on the feedback channel and logs `chat.method_unsupported` (WARN, `method`, `payload_len`, `reason = not_implemented`, the player's ids). The arguments are not decoded. A press costs a chat token like a chat line, so a flood of them gets the one "too quickly" line instead of one reply per packet.

| Index | Method | Feedback |
|---|---|---|
| 0xC6 | `chatFriend` | "Adding friends from chat is not available yet. Use the contact list instead." |
| 0xC7 | `chatList` | "Listing the players in a chat channel is not available yet." |
| 0xC8 | `chatMute` | "Muting players in a chat channel is not available yet. You can ignore a player instead." |
| 0xC9 | `chatKick` | "Kicking players from a chat channel is not available yet." |
| 0xCA | `chatOp` | "Chat channel operators are not available yet." |
| 0xCB | `chatBan` | "Banning players from a chat channel is not available yet." |
| 0xCC | `chatPassword` | "Chat channel passwords are not available yet." |
| 0xCD | `petition` | "Petitions to the GMs are not available yet." |
| 0xCE | `announcePetition` | "Petition announcements are not available yet." |

## GM broadcast

A GameMaster (access level 2) or higher can send one line to many players (SS-C2, decision D-SS16 in `docs/analysis/social-systems/README.md`):

| Entry point | Scope | Notes |
|---|---|---|
| `/gmshout` (native, cell method 222 `sendGMShout(UINT8 isGlobal, WSTRING Text)`) | `isGlobal = 0`: the GM's space instance. Any other value: every online player | The client sends it straight from the slash command, with no Lua in between. How the client splits the typed text into the two arguments is not recovered |
| `.announce <text>` | Every online player | For a client without the slash binding. The typed words are re-joined with single spaces |
| `.announce space <text>` | The GM's space instance | `space` is matched in any case and only as the first word |

Every recipient gets `onPlayerCommunication(<GM's character name>, SPEAKER_GM, CHAN_SERVER, text)` with `CHAN_SERVER` = 8, the GM included, so the GM sees the line as the players do. The client shows it as a bright red line and opens a modal "Server Message" prompt with an OK button (`ChatWindow.lua:160-162`). Until SS-C4 it went out on 7, which the client does not display. The space scope is sent by the cell, which knows the players in the GM's space; the global scope goes through the base, which sends it to every session in the online name index that has a player entity (character select and mid-world-entry sessions are skipped).

The GM gate runs before anything else: cell method 222 is in the SGWGmPlayer tail, so a caller below GameMaster gets `onErrorCode` and the shout goes nowhere (`gm_gate`, security finding CAT-L-06). The text follows the chat rules above (255 UTF-16 units at most, no control or format characters) and blank text is refused; each refusal tells the GM why. There is no flood limit on GM broadcasts, the same exemption GMs have on ordinary chat.

SigNoz: `chat.gm_broadcast` (INFO, the audit row: actor ids, `scope`, `source` = `native` or `console`, `space_id`, the text), `chat.gm_broadcast_delivered` (INFO, the recipient count per scope), `chat.gm_broadcast_rejected` (WARN, `reason` = `empty_text`, `too_long`, `malformed_args`, `no_text`, ...).

## Organization channels

Team (3), command (5) and officer (6) lines pass the same flood limit, allowlist, mute and text rules as every other line, and are then handled on the base, which holds the memberships (organizations ORG-09, `crates/base-session/src/base/organization/handlers/chat.rs`). The base reads the speaker's Team or Command from the database and sends `onPlayerCommunication(speaker, flags, channel, text)` to every member who is in the world now, whatever space they are in, and to the speaker too. Officer is a Command channel: only members whose rank holds `OfficerChat` get the line, and a speaker without it is refused. No channel is registered at login: the client knows 3, 5 and 6 itself (D-ORG14). Every refusal is one feedback line:

| Refusal | Feedback line |
|---|---|
| Team line from a character in no Team | "You are not in a team." |
| Command or officer line from a character in no Command | "You are not in a command." |
| Officer line from a rank without `OfficerChat` | "Your rank cannot speak on the officer channel." |
| The membership cannot be read (no database, a database error) | "Organization chat is unavailable right now. Try again later." |

SigNoz: one `org.chat` row per line on the `org` target (`outcome`, `reason`, `channel`, `org_id`, `recipients`, `text_units`, never the text); the full field list is the `org` row of [observability.md](../architecture/observability.md).

## Tells and Ignore

SS-C1 (2026-09-27), decisions D-SS13, D-SS15 and D-SS17 in `docs/analysis/social-systems/README.md`.

**Tells.** `sendPlayerCommunication` on the tell channel (10) passes the flood limit and text rules above and is then handled on the base; it never reaches the cell. The target name resolves against the players online now: the exact name first, then a case-insensitive match if exactly one player has it. The recipient gets `onPlayerCommunication(sender, flags, 10, text)` and the sender `onTellSent(recipient, text)` (client method 30). If the recipient is away, the tell is still delivered, and the sender also gets the recipient's DND message (or, without one, their AFK message) on channel 10, spoken by the recipient. Every refusal is one feedback line to the sender and nothing to anyone else:

| Case | Feedback |
|------|----------|
| No target | "Who do you want to send a tell to?" |
| Own name | "You cannot send a tell to yourself." |
| Nobody online by that name | "Player X is not online." (the legacy `Chat.py:351-354` reply) |
| Two online players match after case folding | "More than one player is named X. Type the exact name." |
| The recipient ignores the sender | "X is not accepting your messages." |
| A target longer than 64 characters, or with a control, bidi or format character | "That is not a valid character name." (the name is not echoed) |

**Ignore.** The contact list's `Ignore` list (flags 301) is the only source. If A has B on it, B's tells to A are refused as above and B's say, emote and yell are not sent to A. It works one way only: A's lines still reach B, and nobody is hidden from anyone's AoI. Names compare case-insensitively (the D-SS13 fold), so an entry the contact-list window stored as "bob" still ignores "Bob"; if two characters differ only in case, one entry covers both. The list is copied to the base session (for tells) and the cell entity (for spatial chat) at every world entry and after every change, from `chatIgnore` or from the contact-list window.

`chatIgnore(name, 1)` adds a real character by its exact stored name (the typed name is resolved the same way as a tell target, offline characters included). It refuses the player's own character, a name already on the list (in any case), and a list that already holds 100 names (a project cap from the CAT-L-04 audit finding, not a recovered limit). The duplicate and cap checks and the insert run in one transaction under a lock on the list's row, so two overlapping adds cannot both take the last slot, and a unique index on `(list_id, lower(player_name))` keeps one entry per name in every contact list. A name longer than 64 characters, or with a control, bidi or format character, is refused before any database work. The contact-list window's own "add to Ignore" obeys the same 100-name cap under the same lock: the names that fit are added and the player is told how many were not. `chatIgnore(name, 0)` removes a name on the list. Each call spends a token from the chat bucket first, like a chat line, with the same "too quickly" reply. Both go through the contact-list member operations, so the contact-list window updates, and both answer with a feedback line ("You are now ignoring X." / "You are no longer ignoring X." or the reason for a refusal).

An AFK or DND message follows the chat character rules (no control, bidi or format characters) over its whole text, before it is cut to 128 characters, because other players read it in the away reply; a message that breaks them is refused with a feedback line and the previous one is kept.

Returning to character select clears the AFK message and the Ignore cache with the rest of the per-character state.

Events, all on the `chat` target: `chat.tell_delivered` (INFO), `chat.tell_refused` (with `reason`), `chat.ignore_added` / `chat.ignore_removed` (with `before` / `after`), `chat.ignore_refused`, `chat.ignore_synced` / `chat.ignore_sync_failed`, `chat.spatial_ignored` (the count of withheld witnesses), `chat.ignore_set_applied` / `chat.ignore_set_dropped` (cell) and `chat.afk_set`. Message text is never logged, only its length.

## Testing and GM tools

- **UAT.** The owner's checklist is [SS-UAT](../analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release), steps 7-10: tells, Ignore, the flood limit, GM broadcast and `.mute`; steps 9b-9c ([unified-uat.md](../guides/unified-uat.md#mail-chat-and-duels)) cover user channels: join, post from two clients, leave, relog and the refusal cases. Tells and Ignore need two accounts; a solo tester can check that a tell to their own name or to an offline name is refused.
- **GM tools.** `/gmshout` and `.announce [space] <text>` broadcast; `.mute` / `.unmute` moderate. See [commands.md](../commands.md).
- **Two-client test.** `two_client_tell` in `crates/wireclient/tests/it/` exchanges a tell between two wire clients (type 11, not run in CI).
- **SigNoz.** Chat logs on the `chat` target (`chat.tell_delivered`, `chat.tell_refused`, `chat.ignore_*`, `chat.gm_broadcast`, `chat.gm_mute`, `chat.channel_rejected`, `chat.method_unsupported`, `chat.channel_joined`, `chat.channel_left`, `chat.channel_join_rejected`, `chat.channel_leave_rejected`, `chat.channel_post`, `chat.channel_post_rejected`), and flood drops on `rate_limit`. Message text is never logged, only its length.

## Remaining Work

1. **Organization channels** — squad (ORG-04), team, command and officer (ORG-09) are implemented but not yet tested with two real clients (organizations campaign, and the [group system](group-system.md))
2. **Yell radius** — say, emote, and yell all fan out to the same AoI witness set; yell should use a wider range
3. **Channel moderation** — `chatOp`/`chatMute`/`chatKick`/`chatBan`/`chatPassword` answer with a feedback line and do nothing (join/leave/post are implemented; see [User Channels](#user-channels)). GM mutes are still `.mute`-only, not saved across a restart, and keyed by character, so an alt escapes them (an owner question in the [session resume](../analysis/social-systems/handoffs/session-resume.md#owner-questions)). A mute does not block mail
4. **NPC speech** — how `onSystemCommunication`'s Speaker field works for NPCs is still unrecovered
5. **Profanity filter** — none
6. **User channels not client-tested** — issue #1039's join/leave/post logic is unit- and wire-tested but not yet run against a real client on the colo

## Related Docs

- [organization-system.md](organization-system.md) - Guild channels (command, officer)
- [group-system.md](group-system.md) - Squad/team channels
- [contact-list.md](contact-list.md) - The Ignore list that chat, mail and duels honour
- [chat-wire-formats.md](../reverse-engineering/findings/chat-wire-formats.md) - Client evidence (SS-E1, ORG-E1 Q5)
- [chat-speaker-echo.md](../reverse-engineering/findings/chat-speaker-echo.md) - Why `say` never echoes to the speaker but `emote`/`yell` do
- [Social-systems ledger](../analysis/social-systems/README.md) - Decisions D-SS12 to D-SS17 and D-SS26, and the owner questions
