# Minigame System Wire Formats

> **Date**: 2026-03-01 (Mercury tables); SmartFox section added 2026-10-20
> **Type**: reference. **Audience**: engineers
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH for the Mercury tables (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture). The SmartFox tables come from the AS2 disassembly in the MG-R6 specs; nothing in them was run in the client or lab.
> **Sources**: `MinigamePlayer.def`, `alias.xml`, [spec-protocol-common.md](../../analysis/minigames/spec-protocol-common.md) and the per-game specs
> **Companions**: [minigame-system.md](../../gameplay/minigame-system.md) (what the server does), [minigame-architecture.md](minigame-architecture.md) (client analysis), [spec-launch-flow.md](../../analysis/minigames/spec-launch-flow.md)

This page has two layers. The first is the Mercury layer between the client and the game server (cell and base methods). The second, at the end, is the SmartFox layer between the Flash movie and the in-process minigame host.

---

**Interface**: `MinigamePlayer` (implemented by `SGWPlayer`)

Minigames are web-based (Flash/browser) games embedded in the client. The server manages matchmaking, call requests, and result processing.

### Client → Server (Exposed Cell Methods)

#### `startMinigame` — Begin a Minigame

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `endCurrentMinigame` — End Current Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `debugStartMinigame` — Debug: Start Specific Game

| Field | Type | Size |
|-------|------|------|
| `aGameId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `requestSpectateList` — Request Spectatable Players

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `spectateMinigame` — Spectate a Player

| Field | Type | Size |
|-------|------|------|
| `playerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `registerToMinigameHelp` — Register as Helper

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |

#### `updateRegisterToMinigameHelp` — Update Registration

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |
| `wantsRequests` | `UINT8` | 1B |

#### `minigameCallAccept` / `minigameCallDecline` — Respond to Call

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `minigameCallAbort` — Cancel Active Call

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameStartCancel` — Cancel Starting Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameContactRequest` — Request NPC Contact

| Field | Type | Size |
|-------|------|------|
| `ContactId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

### Client → Server (Exposed Base Methods)

#### `minigameCallRequest` — Request Help from Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `RemotePlayerName` | `WSTRING` | 4B len + N×2B |
| `TipAmount` | `INT32` | 4B |

### Server → Client

#### `onStartMinigame` — Launch Game URL

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `URL` | `WSTRING` | 4B len + N×2B — Flash/browser game URL |

#### `onStartMinigameDialog` — Show Game Selection

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Name` | `WSTRING` | 4B len + N×2B |
| `Difficulty` | `WSTRING` | 4B len + N×2B |
| `TCLevel` | `INT32` | 4B |
| `Verb` | `WSTRING` | 4B len + N×2B |
| `ArchetypeBitfield` | `INT32` | 4B |
| `CanPlay` | `UINT8` | 1B |
| `CanCall` | `UINT8` | 1B |
| `CanSpectate` | `UINT8` | 1B |

#### `onStartMinigameDialogClose` / `onEndMinigame`

*(No arguments)* — 1 byte each

#### `onSpectateList` — Available Players to Watch

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `playerIds` | `ARRAY<INT32>` | 4B count + N×4B |
| `playerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onMinigameRegistrationPrompt` — Registration Cost

| Field | Type | Size |
|-------|------|------|
| `Cost` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `minigameRegistrationInfo` — Current Registration State

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Registered` | `UINT8` | 1B |
| `InRangeOnly` | `UINT8` | 1B |
| `WantsRequests` | `UINT8` | 1B |
| `Note` | `WSTRING` | 4B len + N×2B |

#### `addOrUpdateMinigameHelper` — Helper List Entry

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `PlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Note` | `WSTRING` | 4B len + N×2B |
| `Level` | `UINT8` | 1B |
| `Archetype` | `UINT8` | 1B |
| `Friend` | `UINT8` | 1B |

#### `removeMinigameHelper`

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

#### `minigameCallDisplay` — Incoming Call Request

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `CallingPlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Archetype` | `INT32` | 4B |
| `Level` | `INT32` | 4B |
| `TipAmount` | `INT32` | 4B |
| `ExpiresAt` | `INT32` | 4B |
| `GameName` | `WSTRING` | 4B len + N×2B |
| `GameDifficulty` | `WSTRING` | 4B len + N×2B |
| `GameVerb` | `WSTRING` | 4B len + N×2B |
| `GameTC` | `INT32` | 4B |
| `NPCTitle` | `WSTRING` | 4B len + N×2B |

#### `minigameCallResult` — Call Outcome

| Field | Type | Size |
|-------|------|------|
| `ResultCode` | `INT32` | 4B |
| `StartTime` | `FLOAT` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `minigameCallAbort` — Call Cancelled

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

#### `showMinigameContact` — NPC Contact Card

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Id` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Title` | `WSTRING` | 4B len + N×2B |
| `Icon` | `WSTRING` | 4B len + N×2B |
| `Time` | `INT32` | 4B |
| `Success` | `INT32` | 4B |
| `Cost` | `INT32` | 4B |

---

## Launch flow and result codes

How the two layers connect (full detail in [spec-launch-flow.md](../../analysis/minigames/spec-launch-flow.md)):

| Step | Layer | Message | Notes |
|---|---|---|---|
| 1 | Mercury, server to client | `onStartMinigameDialog` (client method 53) | Only with `CIMMERIA_MINIGAME_START_DIALOG` on; the offer lasts 60 s |
| 2 | Mercury, client to server | `startMinigame` (cell method 24) | Accepts the live offer. `minigameStartCancel` (30) drops it |
| 3 | Mercury, server to client | `onStartMinigame(URL)` (client method 52) | `http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}` |
| 4 | SmartFox | `verChk`, `login` (ticket), `joinOK`, then `xtReq` commands | The movie connects on its own after step 3 |
| 5 | Mercury, server to client | `onEndMinigame` (client method 55) | After the result; `endCurrentMinigame` (cell method 25) cancels from the client side |

Result codes, as the minigame host reports them to the cell:

| Code | Name | Chains fired |
|---|---|---|
| 0 | Canceled | None |
| 1 | Victory | `on_victory_chains` |
| 2 | Defeat | `on_failure_chains` |
| 3 | Not started (interrupted) | None |
| 4 | Defeated | `on_failure_chains` (the host does not send it today) |

---

## SmartFox messages per game

These are the `xtReq` commands the Flash movie and the server exchange once the SmartFox login succeeds. They are listed by name only; each spec holds the field tables, the original handler and the port's validation. The server validates all five full games itself and answers shells with a minimal session ([validation modes](../../gameplay/minigame-system.md#games-at-a-glance)).

| Game | Client sends | Server sends | Spec |
|---|---|---|---|
| Livewire | `opendoor`, `closedoor`, `processover`, `processout`, `processmove`, `processreset`, `activateitemcheat`, `activateabilitycheat` | `fullgamestate`, `opendoor`, `closedoor`, `over`, `out`, `destroy`, `timerupdate`, `countdownupdate`, `resetgame`, `victory`, `failure` | [spec-livewire-parity.md](../../analysis/minigames/spec-livewire-parity.md) |
| CrystalGame | `processmove`, `processreset`, `activateitemcheat`, `activateabilitycheat` | `fullgamestate`, `updateslot`, `timerupdate`, `victory`, `failure` | [spec-crystalgame.md](../../analysis/minigames/spec-crystalgame.md) |
| Alignment | `swappieces`, `cascade`, `checkForCascade`, `doscore`, `callhint`, `resethinttimer`, `dovictory`, `processreset`, `activateitemcheat`, `activateabilitycheat` | `fullgamestate`, `hexspinner`, `feedback`, `powerupconvert`, `bypasshexspinner`, `hexmove`, `cascadecheckcomplete`, `updatescore`, `activate`, `hint`, `clicktohint`, `abilitycheatstate`, `timerupdate`, `victory`, `failure` | [spec-alignment.md § 5](../../analysis/minigames/spec-alignment.md#5-message-table) |
| GoauldCrystals | `playfieldclick`, `activateitemcheat`, `activateabilitycheat`, `processreset` | `fullgamestate`, `squarestate`, `powerstatus`, `selectedupdate`, `timerupdate`, `triggersound`, `abilitycheatstate`, `victory`, `failure` | [spec-goauldcrystals.md](../../analysis/minigames/spec-goauldcrystals.md) |
| ConverseBasicHumanoid | `playermove`, `newhand`, `processreset`, `activatevictory`, `failure` | `fullgamestate`, `updatecards`, `updatescore`, `emptyhand`, `newdeal`, `activatevictory`, `victory`, `failure` | [spec-conversebasichumanoid.md](../../analysis/minigames/spec-conversebasichumanoid.md) |
| Hack, Activate, Analyze, Bypass, Converse (shells) | `victory` (from the movie's `win_btn`) | `fullgamestate`, `timerupdate`, `victory`, `failure` | [spec-shells.md](../../analysis/minigames/spec-shells.md) |

What the shipped server does with the commands that need care:

- **Client `victory`.** The five full games refuse it (CrystalGame and Alignment log WARN `client_victory_refused`; GoauldCrystals falls into `unknown_command`; ConverseBasicHumanoid gives no reply). Shells accept it only after a 5 s minimum play time (D-MG13).
- **`processreset`.** Refused in CrystalGame, Alignment and GoauldCrystals (WARN `reset_refused`, no reply). ConverseBasicHumanoid refuses it until a deal ends. Livewire still accepts it and answers `resetgame` then `fullgamestate`.
- **Cheats.** `activateabilitycheat` works in Alignment and GoauldCrystals (bit checked and cleared server-side, answered by `abilitycheatstate`). Item cheats are ignored everywhere, and Livewire ignores both kinds.
- **Unknown commands.** A command a game does not handle is a WARN `unknown_command` and no reply. Each game throttles its refusal logging differently.

See also the per-game rules in [minigame-system.md](../../gameplay/minigame-system.md#crystalgame-rules).

---

## Implementation Notes

- **Minigame URLs**: The server sends `onStartMinigame` a URL of the shape `http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}`. The client does not browse to it: it splits the path to find the SmartFox host, port, game name, player id and ticket, then loads the SWF `{gameName}.swf` and sets those as Flash variables (see [minigame-architecture.md](minigame-architecture.md)).
- **Shell timer**: the five shells run a 60 s countdown on the server (`fullgamestate`, then `timerupdate`); with no `victory` after 240 ticks the server sends `failure`.
