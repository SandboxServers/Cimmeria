# Minigame Architecture — Client Binary Analysis

> **Last updated**: 2026-10-20 (type table corrected in MG-72; the rest is the 2026-03-08 client analysis)
> **Source**: SGW.exe Ghidra decompilation
> **Confidence**: HIGH — decompiled handler + data tables + Lua bindings

---

## Architecture: Scaleform SWF + External Minigame Server

**No minigame logic lives in the client binary.** The client is a thin Scaleform host. All game logic runs inside Flash SWF files that connect directly to a separate minigame server via TCP socket.

### Flow

1. Server sends `onStartMinigame` with: URL, Name, Difficulty, Verb, TCLevel, ArchetypeBitfield, CanPlay/CanCall/CanSpectate flags, playerIds/Names, Cost
2. Client loads SWF file via path `%s.%s` (package.asset) → `/%s.swf`
3. Client passes Flash variables to the SWF:
   - `_root.ipaddress.value` — minigame server IP
   - `_root.port.value` — minigame server port
   - `_root.playerid.value` — player's entity ID
   - `_root.ticket.value` — authentication ticket
   - `_root.gamename.value` — minigame type name
4. Client calls `gotoAndPlay("Start")` on the Flash movie
5. SWF connects directly to the minigame server at IP:port using the ticket
6. All game logic runs in the SWF + minigame server

### Implication for Cimmeria

The game server only needs to:
- Send `onStartMinigame` / `onEndMinigame` to the client
- Handle `MinigameComplete` result codes from the client
- Manage the call/spectate/helper social layer
- A **separate minigame server** process is needed for actual game logic
- Five types are full games with server-side logic (Livewire, CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid); the other five (Hack, Activate, Analyze, Bypass, Converse) are shell SWFs of about 55.8 kB each that the server answers with a minimal shell session (see [spec-shells.md](../../analysis/minigames/spec-shells.md))
- How the shipped server implements each game: [minigame-system.md](../../gameplay/minigame-system.md)

---

## Minigame Type Table (data at `0x019d917c`)

10 minigame types defined in the binary:

| # | Internal Name | Display Name | Implementation Status |
|---|---------------|--------------|----------------------|
| 1 | `Livewire` | "Livewire" | Full SWF game |
| 2 | `CrystalGame` | "Crystal Game" | Full SWF game |
| 3 | `Alignment` | "Alignment" | Full SWF game |
| 4 | `GoauldCrystals` | "Goa'uld Crystals" | Full SWF game |
| 5 | `Activate` | "Placeholder Activate" | Shell SWF (the display name says "Placeholder") |
| 6 | `Analyze` | "Placeholder Analyze" | Shell SWF (the display name says "Placeholder") |
| 7 | `Bypass` | "Placeholder Bypass" | Shell SWF (the display name says "Placeholder") |
| 8 | `Hack` | "Hack" | Shell SWF |
| 9 | `Converse` | "Converse" | Shell SWF |
| 10 | `ConverseBasicHumanoid` | "Converse: Humanoid Basic" | Full game: its own SWF with an embedded extension, not a Converse sub-type |

**5 full games** (Livewire, CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid). The first four get custom Scaleform window titles. The server runs all five and validates their results itself.
**5 shells** (Hack, Activate, Analyze, Bypass, Converse). Each has a small SWF and no server-side game logic; the server accepts a client-declared `victory` only after a minimum play time (D-MG13).

An earlier revision of this table marked rows 5-7 "no SWF", called Hack "named but no special handling" and treated the Converse rows as dialogs. The SWFs exist (see the evidence table in [spec-protocol-common.md](../../analysis/minigames/spec-protocol-common.md)), so that reading was wrong.

## Result Codes (from `0x00c7d0b0`)

| Result String | Code | Meaning |
|---------------|------|---------|
| `Success` | 1 | Player completed successfully |
| `Failure` | 2 | Player failed |
| `Interrupted` | 3 | Game interrupted externally |
| `Defeated` | 4 | Player was defeated |

Sent via `Event_NetOut_MinigameComplete` with `ResultCode` property.

---

## Network Protocol

### Server → Client (NetIn)

| Event | Purpose |
|-------|---------|
| `onStartMinigame` | Start minigame (URL, IP, port, ticket, name, difficulty, etc.) |
| `onEndMinigame` | End the minigame |
| `onStartMinigameDialog` | Show pre-game team dialog |
| `onStartMinigameDialogClose` | Close pre-game dialog |
| `onMinigameRegistrationPrompt` | Show help registration prompt |
| `onMinigameRegistrationInfo` | Registration info update |
| `AddOrUpdateMinigameHelper` | Add/update helper player |
| `RemoveMinigameHelper` | Remove helper player |
| `MinigameCallDisplay` | Display call request to player |
| `MinigameCallResult` | Result of call attempt |
| `MinigameCallAbort` | Abort a call |
| `ShowMinigameContact` | Show minigame contact |
| `onSpectateList` | Spectator list |

### Client → Server (NetOut)

| Event | Purpose |
|-------|---------|
| `StartMinigame` | Player wants to start |
| `EndMinigame` | Player ended minigame |
| `MinigameComplete` | Finished with result code |
| `MinigameCallRequest` | Call another player to join |
| `MinigameCallAbort` | Cancel a call |
| `MinigameCallAccept` | Accept invitation |
| `MinigameCallDecline` | Decline invitation |
| `RegisterToMinigameHelp` | Register as helper |
| `UpdateRegisterToMinigameHelp` | Update registration |

### Debug Commands

`debugStartMinigame`, `debugMinigameInstance`, `debugSpectateMinigame`, `debugJoinMinigame`

---

## Lua Script Bindings (registered at `0x00acbb10`)

Exposed to UI scripts:

| Function | Purpose |
|----------|---------|
| `setMinigameActive` / `isMinigameActive` | Toggle/query active state |
| `playMinigame` / `endMinigame` | Start/end playing |
| `spectateMinigame` | Enter spectator mode |
| `targetIsPlayingMinigame` | Check if target is in minigame |
| `minigameStartCancel` | Cancel starting |
| `minigameCallPlayerById` / `ByName` | Call players |
| `minigameCallContactById` / `ByName` | Call contacts |
| `minigameCallPlayerAmount` | Number of call slots |
| `minigameCancelCallRequest` | Cancel pending call |
| `minigameAcceptCall` / `minigameDeclineCall` | Accept/decline calls |
| `minigameAbortObserverMode` | Leave observer mode |
| `registerToMinigameHelp` / `updateRegisterToMinigameHelp` | Help registration |
| `getMinigameHelpIsRegistered` / `WantsRequests` / `InRangeOnly` / `Note` | Help status queries |

---

## DHD (Dial Home Device) — Separate Module

The DHD (`0x019d8bd0`) is a separate Scaleform module using `/DHD.swf`:
- `_root.pointOfOrigin.value` — origin gate symbol
- `_root.knownGateCount.value` — number of known gates

Part of `GateTravel` class, not the minigame system.

---

## Key Addresses

| Address | Function | Notes |
|---------|----------|-------|
| `0x00e32140` | Main minigame handler | SWF loading, Flash variable passing |
| `0x00c7d0b0` | MinigameComplete handler | Result code parsing |
| `0x019d917c`–`0x019d9340` | Minigame type table | 10 entries with names + display names |
| `0x019d9100`–`0x019d916c` | Flash variable strings | ipaddress, port, playerid, ticket, gamename |
| `0x00acbb10` | Lua binding registration | playMinigame, spectateMinigame, etc. |
