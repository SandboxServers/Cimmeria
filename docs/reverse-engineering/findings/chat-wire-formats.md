# Chat / Communication Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `Communicator.def`, `alias.xml`

---

**Interface**: `Communicator` (implemented by `SGWPlayer`)

### Client → Server (Exposed Base Methods)

#### `sendPlayerCommunication` — Send Chat Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `Channel` | `UINT8` | 1B | EChannel enum (say, tell, party, etc.) |
| `Target` | `WSTRING` | 4B len + N×2B | Target player (for tells) or empty |
| `Text` | `WSTRING` | 4B len + N×2B | Message text |

#### `chatJoin` — Join Chat Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelName` | `WSTRING` | 4B len + N×2B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `chatLeave` — Leave Chat Channel

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatSetAFKMessage` / `chatSetDNDMessage`

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `message` | `WSTRING` | 4B len + N×2B |

#### `chatIgnore` — Ignore/Unignore Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B — 1=ignore, 0=unignore |

#### `chatFriend` — Add/Remove Friend

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNick` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatList` — Request Channel Member List

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatMute` — Mute/Unmute in Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatKick` — Kick from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatOp` — Grant Operator Status

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatBan` — Ban/Unban from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatPassword` — Set Channel Password

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `petition` / `announcePetition` — GM Petition

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aMessage` | `WSTRING` | 4B len + N×2B |

### Server → Client

#### `onSystemCommunication` — System Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `TextType` | `INT32` | 4B | Message type enum |
| `StringId` | `INT32` | 4B | Localized string ID |
| `Speaker` | `WSTRING` | 4B len + N×2B | NPC name (for NPC speech) |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken | Token substitutions |

**`StringToken` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stringID` | `INT32` | 4B |
| `literal` | `WSTRING` | 4B len + N×2B |

#### `onPlayerCommunication` — Player Chat Message

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B — ESpeakerFlags |
| `Channel` | `UINT8` | 1B — EChannel |
| `Text` | `WSTRING` | 4B len + N×2B |

#### `onLocalizedCommunication` — Localized Player Chat

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B |
| `Channel` | `UINT8` | 1B |
| `Text` | `WSTRING` | 4B len + N×2B |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken |

#### `onTellSent` — Tell Confirmation

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aTarget` | `WSTRING` | 4B len + N×2B |
| `aText` | `WSTRING` | 4B len + N×2B |

#### `onChatJoined` — Channel Join Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |
| `ChannelID` | `UINT8` | 1B |

#### `onChatLeft` — Channel Leave Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |

#### `onNickChanged` — Nickname Update

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNickname` | `WSTRING` | 4B len + N×2B |
| `aAddRemoveFlag` | `UINT8` | 1B |

---

## Implementation Notes

- **Chat routing**: Most chat goes through base methods (name-based lookup needed). Only `processPlayerCommunication` is a cell method for local area chat.

---

## SS-E1 client evidence (2026-09-27)

> Static Ghidra RE against `SGW.exe` (no debugger), for the social-systems campaign's SS-E1 packet.
> Answers C-Q1–C-Q4 from `docs/analysis/social-systems/work-packets.md`.

### C-Q1 — `/tell` channel byte (answered elsewhere; not re-derived)

Per the packet's own instruction, this is ORG-E1 Q5, already closed by the organizations
campaign (`docs/analysis/organizations/README.md`, D-ORG14, 2026-09-27, commit `dc6bfdfd`):
**every well-known `EChannel` id is a hardcoded literal in the client binary, byte-identical to
`entities/defs/enumerations.xml`** (say=0, emote=1, yell=2, team=3, squad=4, command=5,
officer=6, server=8, feedback=9, **tell=10**, splash=11, user channels from 12). The client does
not learn these from `onChatJoined`; that only matters for dynamic ids >= 12. No new RE was done
here — cited to avoid duplicating ORG-E1's work, per instruction.

### C-Q2 — `Event_SlashCmd_GMShout` (PARTIAL)

Confirmed the native `/gmshout` slash command exists in the binary as `Event_SlashCmd_GMShout`
(`0x005add50`/`0x005add70`/`0x005ade90`, the standard `TypedEmitInfo`/RTTI boilerplate), chained
directly to `CME_EventSignal_VEvent_NetOut_SendGMShout` (`0x00cf48d0`) and
`register_NetOut_SendGMShout` (`0x00cfdbb0`) — i.e. the slash command is wired straight to cell
method `sendGMShout`, with **no Lua involvement**: grepped every `.lua` file under
`Content/UI/Core/ChatWindow/` for `gmshout`/`GMShout` and found nothing. This matches the
convention already recorded in project memory (`project_gm_commands_native_console`): commands
with a native binding use the client's `/` console directly, no scripted layer.

I did not trace the native slash-command tokenizer that splits the typed text into
`(isGlobal, Text)` — the def's `sendGMShout(UINT8 isGlobal, WSTRING Text)` argument list is
already pinned by audit A-28, and nothing found here contradicts it. Treat the exact arg-split
heuristic (e.g. does a leading token like "global" or "space" select `isGlobal`?) as UNRESOLVED;
it is non-blocking for SS-C2, which only needs to decode the two fields the def already declares.

### C-Q3 — chat input max length (CLOSED — negative evidence)

`ChatWindow.layout`'s `Chat_Input` control (`Window Type="RichTextEditbox"`) has **no
`MaxTextLength` property** set anywhere in the layout file. For comparison, every other editbox
checked in the same client tree that imposes a client-side cap declares one explicitly
(`BlackMarket.layout`, `PlayerSearch.layout`, `Social.layout`, `StartMinigame/*.layout` all set
`MaxTextLength`). **The chat input is uncapped client-side.** D-SS12's 255-character server cap is
therefore a new invariant with nothing in the client to match or exceed — there is no recovered
client constant to reconcile it against.

Evidence: grep for `MaxTextLength` across `Content/UI/Core/ChatWindow/*.layout` (zero hits) vs.
`Content/UI/Core/{BlackMarket,PlayerSearch,Social,StartMinigame}/*.layout` (multiple hits).

### C-Q4 — `onTellSent` rendering and AFK/DND auto-replies (UNRESOLVED, not independently re-RE'd)

I relied on the existing 2026-09-27 chat research report (`kg-2026-09-27/chat.md` §1.6, citing
`deprecated/python/base/Chat.py:351-354` and Cimmeria's own `gm_feedback.rs` precedent) rather
than re-deriving this from Ghidra this session — it already answers how a missing/offline tell
target is reported, and how AFK/DND auto-replies are expected to ride back on the tell channel.
I did not independently confirm `onTellSent`'s exact client-side rendering (which UI element shows
it, whether it differs from an ordinary tell) or re-verify the AFK/DND special-casing against the
binary. Treat this question as still open for a future pass if SS-C1 needs stronger evidence than
the legacy-Python + existing-pattern reasoning the report already provides.
