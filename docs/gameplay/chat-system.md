# Chat System

> **Last updated**: 2026-03-01
> **Status**: ~95% implemented

## Overview

The chat system provides multi-channel text communication between players. It supports system channels (say, emote, yell, team, squad, command, officer, server, feedback, tell, splash) and user-created channels (chat, roleplay, alliance). Messages on cell-based channels are forwarded to the CellApp for spatial distribution; other messages are handled on the BaseApp.

The `ChatChannelManager` singleton in `python/base/Chat.py` manages all channel state. The `Communicator` interface defines the entity-level chat API.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| System channels (11 types) | DONE | say, emote, yell, team, squad, command, officer, server, feedback, tell, splash |
| User channels | DONE | Create, join, leave, delete, password protection |
| Default user channels | DONE | chat, roleplay, alliance (persistent) |
| Player-to-player tell | DONE | Via `CHAN_tell` with offline detection |
| Channel join/leave | DONE | Password validation, auto-delete empty channels |
| Channel operator system | DONE | `chatOp` promotes players to channel ops |
| Channel moderation | DONE | `chatMute`, `chatKick`, `chatBan` |
| Channel password | DONE | `chatPassword` to set/change |
| AFK/DND status | DONE | `chatSetAFKMessage`, `chatSetDNDMessage` |
| Ignore list | DONE | `chatIgnore` adds/removes |
| Friend list (nicknames) | DONE | `chatFriend` with `onNickChanged` |
| Speaker flags | DONE | DND, GM flags in messages |
| Cell channel forwarding | DONE | Cell channels forwarded via `processPlayerCommunication` |
| Petition system | DONE | `petition`, `announcePetition` |
| GM shout | DONE | `hearGMShout` broadcast |
| Localized communication | DONE | `onLocalizedCommunication` with string tokens |
| Channel list | DONE | `chatList` to enumerate members |

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

| Channel | Enum | Flags | Distribution |
|---------|------|-------|-------------|
| say | `CHAN_say` | OnCell | Spatial (nearby players) |
| emote | `CHAN_emote` | OnCell | Spatial |
| yell | `CHAN_yell` | OnCell | Spatial (wider range) |
| team | `CHAN_team` | OnCell | Strike team members |
| squad | `CHAN_squad` | OnCell | Squad members |
| command | `CHAN_command` | OnCell | Command (guild) members |
| officer | `CHAN_officer` | OnCell | Guild officers |
| server | `CHAN_server` | DisallowPlayerMessages | Server-wide broadcasts |
| feedback | `CHAN_feedback` | OnCell, DisallowPlayerMessages | System feedback |
| tell | `CHAN_tell` | (none) | Direct player-to-player |
| splash | `CHAN_splash` | OnCell, DisallowPlayerMessages | Splash screen messages |

## Channel Flags

| Flag | Constant | Purpose |
|------|----------|---------|
| `CHANNEL_FLAG_OnCell` | -- | Messages processed on CellApp |
| `CHANNEL_FLAG_DisallowPlayerMessages` | -- | Players cannot speak |
| `CHANNEL_FLAG_KeepIfEmpty` | -- | Channel persists when empty |

## Speaker Flags (ESpeakerFlags)

| Flag | Enum | Purpose |
|------|------|---------|
| `SPEAKER_DND` | -- | Speaker has DND enabled |
| `SPEAKER_GM` | -- | Speaker is a Game Master |

## Data References

- **Channel manager singleton**: `python/base/Chat.py` -> `ChannelManager`
- **Constants**: `common.Constants` (channel flags, MIN_USER_CHANNEL)
- **Enumerations**: `CHAN_say` through `CHAN_splash`, `SPEAKER_*` flags

## RE Priorities

1. **Spatial distribution** - How cell channels determine message range (say vs yell radius)
2. **Platoon leader flag** - Referenced in `getSpeakerFlags()` TODO
3. **Mute/ban persistence** - Whether mutes and bans survive server restarts
4. **NPC speech** - How `onSystemCommunication` Speaker field works for NPCs

## Related Docs

- [organization-system.md](organization-system.md) - Guild channels (command, officer)
- [group-system.md](group-system.md) - Squad/team channels
