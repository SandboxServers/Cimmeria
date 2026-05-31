---
name: chat-speaker-flags
description: ESpeakerFlags bitfield values, wire layout, and Python reference logic for onPlayerCommunication speaker_flags
metadata:
  type: reference
---

## ESpeakerFlags — canonical bit values

Source: `entities/defs/enumerations.xml` lines 131–140. UINT8 bitfield.

| Constant | Value | Notes |
|----------|-------|-------|
| `SPEAKER_None` | 0 | |
| `SPEAKER_GM` | 1 (bit 0) | access_level > 0 |
| `SPEAKER_Petition` | 2 (bit 1) | defined but never set in Python ref |
| `SPEAKER_DND` | 4 (bit 2) | dndMessage != None |

**AFK is NOT a speaker flag.** There is no `SPEAKER_AFK` in `enumerations.xml`. AFK state is stored on the player for auto-reply tells but does not affect the speaker_flags bitfield.

## Python reference: getSpeakerFlags (Chat.py)

```python
def getSpeakerFlags(self, player):
    flags = 0
    if player.dndMessage is not None:
        flags |= enums.SPEAKER_DND   # 0x04
    if player.accessLevel > 0:
        flags |= enums.SPEAKER_GM    # 0x01
    # TODO: Add PlatoonLeader, Petition flag checks
    return flags
```

GM threshold is `accessLevel > 0` — includes Moderator (level 1), not just GameMaster (level 2).

## Wire layout: onPlayerCommunication (method index 28)

```
[u32 speaker_len][UTF-16LE speaker...][u8 speaker_flags][u8 channel][u32 text_len][UTF-16LE text...]
```

Serializer lives in `crates/services/src/cell/chat.rs::serialize_on_player_communication()`. Already correct — only the value passed as `speaker_flags` needs fixing.

## State ownership in Rust

- `ConnectedClientState.access_level: u32` — already stored, populated from `PendingLogin` at login. Was `#[allow(dead_code)]`.
- `dnd_message` field — NOT YET ADDED as of 2026-05-27. Needs `Option<String>` on `ConnectedClientState`.
- `CHAT_SET_DND` (0xC4) handler in `dispatch.rs` — acknowledged only, does not store state. Needs implementation.
- `CHAT_SET_AFK` (0xC3) — log-only is correct; AFK doesn't affect speaker_flags.

## Ghidra addresses

| Symbol | Address |
|--------|---------|
| `Event_NetIn_onPlayerCommunication` | `0x019bcdfc` / `0x019bfeb4` |
| `onPlayerCommunication` flash handler | `0x00d76760` |
| `Event_NetOut_ChatSetAFKMessage` | `0x019b9978` |
| `Event_NetOut_ChatSetDNDMessage` | `0x019b99ac` |

Source: `docs/analysis/event-net-mapping.md` and `docs/protocol/message-catalog.md`.

## Related

- [[chat-afk-dnd-state]] (future — AFK auto-reply tell feature, separate from speaker_flags)
