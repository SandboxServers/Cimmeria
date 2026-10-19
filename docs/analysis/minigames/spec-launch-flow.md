# Minigame launch flow, modifiers and ability launch (MG-R7)

Research spec for the minigames campaign, Wave 0A. It answers the eight
questions in [work-packets.md § MG-R7](work-packets.md#mg-r7-launch-flow-modifiers-and-ability-launch)
and ends with **Packet inputs**: exact values for MG-21, MG-22, MG-23, MG-24,
MG-62 and MG-63.

Every claim carries a tag:

- **Confirmed**: read directly from a def, a seed row, the original Python or
  C++, the client UI Lua, or a published RE finding, cited by file and line.
- **Inferred**: a reasoned conclusion that no single source states. G1 should
  treat these as proposals.
- **Unresolved**: no source in this repo answers it. The packet says what
  would.

Client UI Lua is cited by path under the client install
(`SGWGame/Content/UI/...`). It is a client asset and is not in git; quotes stay
under 20 lines (D-MG10).

## Contents

1. [The start dialog: client methods 53 and 54, cell methods 24 and 30](#1-the-start-dialog)
2. [`BSF_PlayingMinigame` and the movement lock](#2-bsf_playingminigame-and-the-movement-lock)
3. [`techcomp`](#3-techcomp)
4. [`abilityBitfield`](#4-abilitybitfield)
5. [Item cheats: `instcc`, `CA0`-`CA4`, `consUsed`](#5-item-cheats)
6. [Ability-started games](#6-ability-started-games)
7. [Prison Boot 1597](#7-prison-boot-1597)
8. [Ability 802, event set 1132, sequence 2304](#8-ability-802-and-event-set-1132)
9. [Corrections to other docs](#9-corrections-to-other-docs)
10. [Packet inputs](#packet-inputs)

## 1. The start dialog

### Wire signatures (Confirmed)

| Dir | Index | Method | Args | Source |
| --- | --- | --- | --- | --- |
| S→C | 52 | `onStartMinigame` | `WSTRING URL` | `docs/protocol/client-method-dispatch-table.md:181` |
| S→C | 53 | `onStartMinigameDialog` | `WSTRING Name, WSTRING Difficulty, INT32 TCLevel, WSTRING Verb, INT32 ArchetypeBitfield, UINT8 CanPlay, UINT8 CanCall, UINT8 CanSpectate` | `client-method-dispatch-table.md:182`; `entities/defs/interfaces/MinigamePlayer.def:159-168` |
| S→C | 54 | `onStartMinigameDialogClose` | none | `client-method-dispatch-table.md:183`; `MinigamePlayer.def:169-170` |
| C→S | cell 24 | `startMinigame` | none, exposed | `docs/protocol/cell-method-dispatch-table.md:152-172`; `MinigamePlayer.def:402-404` |
| C→S | cell 30 | `minigameStartCancel` | none, exposed | `cell-method-dispatch-table.md:152-172`; `MinigamePlayer.def:467-469` |

The two cell methods take no arguments. The server must remember which game
the dialog offered (the original kept it on `self.minigame`; the def also has
the `pending*` properties) and act on that when cell 24 arrives.

### Client behaviour (Confirmed, `UI/Core/StartMinigame/StartMinigame.lua`)

- Line 507 subscribes `Events.StartMinigameDialog` to
  `onStartMinigameDialog(this, name, difficulty, tclevel, verb, archetypeBitfield, bCanPlay, bCanCall, bCanSpectate)`.
  The argument order is the def order.
- The window title is `name`. The info line is built at line 164 as
  `difficulty .. " TC " .. tclevel .. " " .. verb`, so `Difficulty` is free
  display text and `Verb` is the game noun.
- Lines 165-200 test `archetypeEnabled(archetypeBitfield, <name>)` for, in
  order: archaeologist, asgard, commando, goa'uld, jaffa, scientist, shol'va,
  soldier. A clear bit greys that archetype's label. This is a display-only
  "who can play this" hint; the client does not gate the Play button on it.
- `bCanPlay`, `bCanCall` and `bCanSpectate` enable the Play, Call-for-help and
  Spectate buttons.
- Play (`onPlayMinigameClicked`, line 27) calls `playMinigame()`, which sends
  **cell 24** `startMinigame`, then hides the window.
- Cancel and the close box (line 22) call `minigameStartCancel()`, which sends
  **cell 30**, then hide the window.
- Line 508 subscribes `Events.StartMinigameDialogClose` (client 54) to
  `onCloseClicked`, the same handler as Cancel. **A server-sent 54 therefore
  makes the client send cell 30 back.** The server must treat a cell 30 that
  arrives after its own 54 as a no-op.
- The dialog has no timeout of its own. The neighbouring call-for-help windows
  in the same file wait 60 s (`WaitingForResponse60`).

### Can Cimmeria send the dialog with no client patch? Yes (Confirmed)

Method 53 is in the shipped def, the client Lua handles it, and both buttons
route to exposed cell methods (24 and 30) that already exist in the dispatch
table. Nothing in the path needs a client change. What is missing is all
server-side:

- Nothing in `crates/` sends 53 or 54. The index constants exist at
  `crates/wire/src/cell/client_methods/minigame.rs:6,8`.
- Cell 30 is UNIMPLEMENTED at
  `crates/cell-methods/src/cell/cell_methods/minigame.rs:133`.

### What the original sent (Confirmed)

`deprecated/python/cell/SGWPlayer.py:1917` sends
`onStartMinigameDialog(minigame.name, "DifficultyLevel", minigame.techCompetency, gameName, archetypes, 1, 0, 0)`.

- `Difficulty` was the literal placeholder `"DifficultyLevel"`: the original
  never localised it. Cimmeria may send any short text (see Packet inputs).
- Every Python caller passes `techCompetency=1` and an archetype mask of
  `0xffff`, `0xff` or `0x7fff`; all of them launch Livewire.
- CanCall and CanSpectate were always 0: help-calls and spectating were never
  wired in the emulator, so the buttons stayed disabled.

`SGWPlayer.py:2010-2019` (`minigameStartCancel`) clears the flag and the lock
and reports result 3 (`NotStarted`). It never sends 54; the client had already
closed the window itself.

## 2. `BSF_PlayingMinigame` and the movement lock

### Values (Confirmed)

`BSF_PlayingMinigame = 4`, `BSF_MovementLock = 6` (enums). Movement locks are
reference-counted: `addMovementLock`/`removeMovementLock` in
`deprecated/python/cell/SGWBeing.py:770-787` set bit 6 on the first lock and
clear it when the count returns to 0.

### Where the original set and cleared them (Confirmed)

| Event | Flag 4 | Movement lock | Source |
| --- | --- | --- | --- |
| Dialog shown (`requestStartMinigame`) | set | add | `SGWPlayer.py:1921-1922` |
| Results arrive (`handleMinigameResults`) | clear | remove | `SGWPlayer.py:1928-1929` |
| Cancel (cell 30, `minigameStartCancel`) | clear | remove | `SGWPlayer.py:2013-2014` |
| Play (cell 24, `startMinigame`) | unchanged | unchanged | `SGWPlayer.py:1959-1971` |

So the original locked the player from the moment the dialog appeared, not
from the moment the game started. A player who closed the client mid-dialog
kept no lock (the entity was destroyed), but a player whose dialog never got an
answer stayed locked. That is the hole the 60 s expiry in MG-21 closes.

### Does the client react to bit 4? Yes (Confirmed)

`docs/reverse-engineering/findings/state-flag-broadcast.md:20,69`: the client's
state-flag handler `FUN_00e01c90` XOR-diffs the old and new flags, and
`TEST BL,0x10` (bit 4) routes to `FUN_00e31aa0`, the minigame UI lock. Bit 6,
in the `0xC4` mask, routes to `FUN_00dfff70`, which disables movement input.
`OnEvent_UI_TargetPlayingMinigameUpdate` is at `0x00cca420` (v5 worker-5b
checkpoint), and `UI/Core/Target/Target.lua:115` calls
`targetIsPlayingMinigame()` to mark a targeted player who is in a game. Bit 4
is therefore visible to other players through the state-flag broadcast, not
only to the owner.

## 3. `techcomp`

### Source in the original (Confirmed for the emulator, Inferred for CME)

- The cell passes `minigame.techCompetency` to `base.startMinigame`
  (`SGWPlayer.py:1970-1971`), and the base forwards it to `joinOK` as
  `techcomp` (`deprecated/cpp/src/baseapp/minigame_connection.cpp:424-441`).
- Every Python caller sets `techCompetency=1`. The emulator never derived it
  from a stat, the archetype or an item.
- The def gives the intended source: `processStartMinigameAction(instanceId, triggeringItem, triggeringMob, seed, tcOverride)`
  (`MinigamePlayer.def:410-416`) and the `pendingTC` property. A game started
  from an item or an `INT_Minigame*` object carries a TC override.
- The item seeds carry the TC on the item: `items.tech_comp` on every
  minigame instrument and consumable (`db/resources/Items/Seed/items.sql`).
  The dialog shows `TC <n>` beside the difficulty, the same "TC" the item
  names use ("Generic TC10 Mini-game Instrument"). **Inferred:** CME's
  `techcomp` was the TC level of the instrument (or of the target object's
  override), not a player stat.

### Range (Confirmed)

- `deprecated/python/cell/commands/Net.py:175` documents techCompetency as
  `(1-55)`.
- The item `tech_comp` values in the seeds are the set {3, 5, 8, 10, 13, 15,
  18, 20, 23, 25, 28, 30, 33, 35, 38, 40, 43, 45, 48, 50, 51, 53, 55}.

Clamp to 1-55. Rust today hard-codes 1 (`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`,
README F7).

### Launch parameters to `joinOK` variables (Confirmed)

The cell calls `base.startMinigame(gameId, ticket, difficulty, techComp, seed, abilities, intelligence, level)`
(`deprecated/python/base/SGWPlayer.py:77`; the call is
`deprecated/python/cell/SGWPlayer.py:1970-1971`). The C++ base writes the
values into the `joinOK` room variables
(`deprecated/cpp/src/baseapp/minigame_connection.cpp:424-441`).

| `startMinigame` arg | Original value | `joinOK` variable | Read by |
| --- | --- | --- | --- |
| `difficulty` | 1-5 | `difficulty` | every game |
| `techComp` | 1 (§ 3) | `techcomp` | every game |
| `seed` | random | `seed` | the extensions (MG-R6 Q2) |
| `abilities` | 0 | `abilityBitfield` | § 4 |
| `intelligence` | `getStat(intelligence).cur` | `intelligence` | ConverseBasicHumanoid only (MG-R6 Q2) |
| `level` | `self.level` | `pclevel` | MG-R6 Q2 |
| none | -1, 0 × 5 | `instcc`, `CA0`-`CA4` | § 5 |

Rust registers `intelligence = 0` and `player_level = 1` today
(`cell_dispatch/minigame.rs:104-106`). MG-02 should pass the cell's current
intelligence stat and the player's level.

### Launch variables in the URL

- **The host must set `ipaddress.value`.** The client's `onStartMinigame`
  handler sets `_root.ipaddress/port/playerid/ticket/gamename.value` from the
  URL (`docs/analysis/cellblock-autoplay/livewire-autosolve.md:62-66`). The
  movie goes offline (a local game, no server) only if `_root.ipaddress.value`
  and `_root.v1` are both undefined when it reaches frame 2 (MG-R6
  `spec-protocol-common.md` Q1, which owns the test). A set-but-empty host
  gives `.value == ''`: the movie stays online, its connect fails, and the
  player sees the failure screen.
- Rust builds the URL as `http://unused/{host}/{port}/{gameName}/{entityId}/{ticket}`
  from `minigame_external_host` (`cell_dispatch/minigame.rs:115-118`). An
  empty host setting would yield `http://unused//...`. MG-02 must assert a
  non-empty host in its URL builder.

## 4. `abilityBitfield`

### What is known (Confirmed)

- The original always sent 0: `SGWPlayer.py:1970-1971` passes a literal `0`
  for abilities with a TODO; the C++ `joinOK` forwards it unchanged
  (`minigame_connection.cpp:430`).
- The Python games read nothing from it. `activateabilitycheat` is dispatched
  in `Alignment.py:531`, `GoauldCrystals.py:534` and `Livewire.py:587` to
  `onActivateAbilityCheat(cheatNum)`, which is a stub returning `True`.
- Rust sends `abilities_mask` (`crates/minigame/src/minigame/games/livewire/setup.rs:388`)
  and accepts both cheat commands at `games/livewire/mod.rs:368`.
- `abilityBitfield` is **not** the dialog's `ArchetypeBitfield`. The Python
  `archetypes` mask (`0xffff`) is a client-53 display argument. Copying it
  into `abilities_mask` would light every cheat in the SWF
  (`docs/analysis/castle-rebuild/worknotes/m701.md:199-207`).

### Candidate abilities (Confirmed as seed rows; bit positions Inferred below)

| Ability | Name | Seed description |
| --- | --- | --- |
| 677 | Mini-game: Delay | adds or removes time on a timer |
| 678 | Mini-game: In Situ Context | adds or removes playfield spaces |
| 679 | Mini-game: Effigy Transfer | adds or removes obstacles |
| 680 | Mini-game: Contextual Guide | trades timer for an advantage |
| 681 | Mini-game: Grave Goods | exchanges moving parts |
| 682 | Mini-game: Totem Swap | alters obstacle function |
| 683 | Mini-game: Old Rite | stops the timer for a few seconds |
| 684 | Mini-game: Re-Survey | alters playfield function |
| 685 | Mini-game: Epiphany | alters timer function |
| 686 | Mini-game: Ethnographic Drift | exchanges obstacles for a parameter |
| 778 / 779 / 792 / 793 | Conversation: Mediator / Negotiator / Diplomat / Ambassador | +5 / +10 / +15 / +25 % Trump in Converse (passives, flags 128) |
| 809 | Mental Fortitude | +15 % mental resists in Mini-game State |
| 813 | Impressive Victory | disables abilities; +50 % XP on victory |
| 814 / 1195 | Flawless Victory | disables cheats (1195: abilities and cheats); +100 % XP |
| 815 | Battle Avoidance | +2 QR in Mini-game State |
| 2507 / 2508 | Reduce / Remove Distractions | +10 / +20 % interruption resistance |

All rows are in `db/resources/Abilities/Seed/abilities.sql`; none has an
`event_set_id` or an effect that names a bit. 677-686 are only the
Archaeologist's ten. The same file has 18 more "Mini-game:" abilities for
the other archetypes, 687-704 (Goa'uld 687-689, Asgard 690-695, Commando
696-697, Jaffa 698-699, Scientist 700-704): 28 in all, every one a passive
with no effects. 809, 815, 2507 and 2508 act on the player's combat stats
while bit 4 is set and are not SWF bits. 813, 814 and 1195 are scoring
modifiers the server applies to the reward, not SWF bits.

### What the SWFs do with the mask (Confirmed, MG-R0 tools)

Derived with `python -I tools/minigame-swf/list_functions.py <movie> --function <name>`
on the client's Livewire, Alignment, CrystalGame and GoauldCrystals movies,
functions `setupAbilityCheats`, `updateAbilityCheats`, `setupAbilityCheat`,
`activateAbilityCheat`, `activateAbilityCheatRequest`, `processAbilityCheat`
and `sendAbilityCheatInfo`.

- **The mask has 15 bits, 0-14.** All four client `setupAbilityCheats` loop
  `i` from 0 to 14 and draw button `abilityCheat<i>` when
  `abilityBitfield & (1 << i)` is set. The offline demo path picks a random
  mask in 1-32768, the same 15-bit range.
- **The cheat number on the wire is the bit index.** `activateAbilityCheat`
  sends `activateabilitycheat` with `cheatNum` = `i`.
- **Each ability cheat is single-use per game.** The extension's
  `activateAbilityCheatRequest` runs `processAbilityCheat` only if bit
  `cheatNum` is set, then clears it (`& ~(1 << cheatNum)`), stores the new
  mask and sends `abilitycheatstate` with it. The client's
  `updateAbilityCheats` greys the cleared button (frame `off`, alpha 30).
- **No server round trip.** The ability path never calls the host bridge
  (`__mg`); the item path does (§ 5). The original's
  `onActivateAbilityCheat` stub is not part of the SWF contract.
- **CrystalGame** sends `activateabilitycheat` and listens for
  `abilitycheatstate`, but its extension has no `activateAbilityCheatRequest`
  or `processAbilityCheat`. Whether its `handleRequest` handles the command
  inline is for MG-R2.

### Bit to cheat category (Confirmed, GoauldCrystals `processAbilityCheat`)

GoauldCrystals' `processAbilityCheat` switches on `cheatNum` 0-14 and plays a
sound per case whose path names the category. Livewire and Alignment switch
on the same 0-14, and their cases agree where checked (bit 10 restores
`instcc` from `origInstcc` in Livewire, Alignment and GoauldCrystals alike).

| Bit | GoauldCrystals sound | Category | Livewire effect (`processAbilityCheat`) |
| --- | --- | --- | --- |
| 0 | `crystalMG/time/chng` | timer: add or remove time | `addTime(5)` |
| 1 | `crystalMG/time/xchng` | timer: trade time for an advantage | `cutTime`, then destroys 2 `o*` pieces and 75 % of the `m*` pieces |
| 2 | `crystalMG/time/stop` | timer: stop | `revertTime`, `addTime(3)` |
| 3 | `crystalMG/time/alter` | timer: alter | `revertTime`, `countdownRate` × 2 |
| 4 | `crystalMG/plyfld/addremSpc` | playfield: add or remove spaces | `addTime(3)`, destroys 1 `me*` and 3 other `m*` pieces |
| 5 | `crystalMG/plyfld/addremCont` | playfield: add or remove contents | `addTime(3)`, destroys 3 `?e*` pieces |
| 6 | `crystalMG/plyfld/alter` | playfield: alter | `addTime(3)`, `playfieldActive = true` |
| 7 | `crystalMG/mvprts/addrem` | moving parts: add or remove | `addTime(3)`, destroys 25 % of the `m*` pieces |
| 8 | `crystalMG/mvprts/alter` | moving parts: alter | `revertTime`, `countdownRate` × 2 |
| 9 | `crystalMG/mvprts/xchng` | moving parts: exchange | `addTime(5)`, creates 3 moving electrodes, destroys 3 `o*` pieces |
| 10 | `crystalMG/tools/alter` | instrument: alter | restores `instcc` from `origInstcc`, resends item-cheat state |
| 11 | `crystalMG/tools/apply` | instrument: multiple functions | `addTime(3)`, `movingCheat = true` |
| 12 | `crystalMG/obst/addrem` | obstacles: add or remove | `addTime(3)`, destroys the 2 deepest `o*` pieces, flashes a `g*` piece |
| 13 | `crystalMG/obst/alter` | obstacles: alter | `addTime(3)`, flashes every `o*` piece |
| 14 | `crystalMG/obst/xchng` | obstacles: exchange | same as bit 9 |

The `o*`, `m*`, `e*` and `g*` prefixes are the movie's own piece names; what
each piece is belongs to the per-game specs (MG-R1 to MG-R5). The Livewire
column is the shape of each case, not a full rules port.

### 10 versus 15: reconciled

The SWF has 15 cheat *categories*, not 15 abilities, and 677-686 are not
bits 0-9. Each "Mini-game:" ability's seed description names exactly one
category, and the 28 abilities 677-704 cover 14 of the 15 (no ability names
bit 5).

### Bit to ability id (Inferred from seed descriptions; the SWF has no ids)

No movie contains an ability id; the bit index is all the SWF knows. The
mapping below matches each seed description's verb and object to a category
name. It is **Inferred**.

| Bit | Abilities (seed description) |
| --- | --- |
| 0 | 677 Delay, 690 Temporal Extension ("manipulates the timer"), 698 Delay (Jaffa) |
| 1 | 680 Contextual Guide, 691 Transtemporal Array |
| 2 | 683 Old Rite, 692 Chronosuppression |
| 3 | 685 Epiphany, 693 Transmute Time, 699 Second Chance |
| 4 | 678 In Situ Context |
| 5 | none |
| 6 | 684 Re-Survey |
| 7 | 687 Primal Savvy, 700 Analysis |
| 8 | 688 Indelicate Alteration, 704 Scientific Law |
| 9 | 681 Grave Goods, 689 Deft Replacement, 701 Experiment |
| 10 | 694 Hypersensorium, 702 Instrument Adaptation |
| 11 | 695 Polyphasic Routine, 703 Instrument Augmentation |
| 12 | 679 Effigy Transfer, 696 Bypass |
| 13 | 682 Totem Swap, 697 Shut Down |
| 14 | 686 Ethnographic Drift |

**Who owns what.** The launch side (cell → base `startMinigame`) builds the
mask: OR in bit `n` for each of the player's known abilities 677-704 mapped
to `n`. The minigame service owns the effect (the port of each game's
`processAbilityCheat`) and the single-use clearing. The Converse Trump
passives 778, 779, 792 and 793 are not in this mask: ConverseBasicHumanoid has
no `setupAbilityCheats` (its only cheat function is `checkForCheats`), so how
they enter is MG-R4's question.

## 5. Item cheats

### Fields (Confirmed)

- `joinOK` carries `instcc` and `CA0`-`CA4`
  (`minigame_connection.cpp:424-441`); the original sent `instcc = -1` and
  `CA0`-`CA4 = 0`. `Alignment.py`, `Livewire.py` and `GoauldCrystals.py` all
  default to `instcc = -1` and `ca = [0]*5`, and keep `consumablesUsed` and
  `consumableCheatBonus`, with a "TODO: Save consumablesUsed / instcc?".
- The def comment on `updateMinigameItemCheats(Ticket, instCC, CA0..CA4)`
  (base, not exposed, `MinigamePlayer.def:269-277`) calls `instCC` the "cheat
  class of instrument" and `CA<n>` the "class N cheat activations".
- Consumption has its own round trip: base `consumeItemByMinigame`
  (`MinigamePlayer.def:262-267`) and cell
  `consumeItemByMinigame(ticket, cheatClass, requestId)`
  (`MinigamePlayer.def:425-429`).
- Results report use: cell `handleMinigameResults(..., UINT8 instrumentUsed, UINT8 consumableUsed)`
  (`MinigamePlayer.def:368-375`).
- `activateitemcheat` reaches the same stub as the ability cheats.

### Semantics (Confirmed, MG-R0 tools)

Derived from Livewire's `setupItemCheats`, `setupItemCheat`,
`activateItemCheat`, `activateItemCheatRequest`, `processItemCheat`,
`handleItemCheatInfo`, `sendItemCheatInfo` and `handleInternalRequest`
(`list_functions.py <Livewire.gfx> --function <name>`; `handleInternalRequest`
is a frame-level function, found with `as2dis.py`). Alignment and
GoauldCrystals have the same functions; their `processItemCheat` switch on the
same 0-4.

- **Five item cheats, slots 0-4, one per cheat class.** The client draws
  `itemCheat0` to `itemCheat4`. `activateItemCheat(i)` sends
  `activateitemcheat` with `cheatNum` = `i`. Each game's `processItemCheat`
  has its own five effects (Livewire: slot 0 `addTime(2)` and destroys pieces,
  slot 1 `addTime(2)` and a flash, slot 2 `addTime(5)` and new pieces, slots 3
  and 4 `addTime(3)` and more). The rules belong to MG-R1 to MG-R5.
- **`instcc` is the highest slot the instrument covers.** In
  `setupItemCheats`, slot `i` gets the instrument border when `instcc >= i`,
  and `setupItemCheat` then shows `CA<i> + 1` uses. `instcc = -1` means no
  instrument.
- **Instrument use: once per game, no item consumed.** In
  `activateItemCheatRequest`, a `cheatNum <= instcc` runs `processItemCheat`,
  sets `instcc = -1` and sends `itemcheatstate`. Nothing goes to the server.
  Ability bit 10 (§ 4) restores `instcc` from `origInstcc`, which is a second
  instrument use.
- **Consumable use: one item, validated by the server.** Otherwise, when
  `CA<cheatNum> > 0`, the online path calls
  `__mg.minigameValidateItem(user, cheatNum)` (the host bridge). The server
  answers with internal requests that `handleInternalRequest` routes:
  `itemCheatActivate` sets `consUsed = true` and runs
  `processItemCheat(cheatNum)`; `itemCheatInfo` (`handleItemCheatInfo`)
  stores fresh `CA0`-`CA4` and resends `itemcheatstate`. This is the def's
  `consumeItemByMinigame(ticket, cheatClass, requestId)` and
  `updateMinigameItemCheats` round trip. `consUsed` becomes the results'
  `consumableUsed`.
- **`CA<n>` is cumulative (Inferred from the offline branch).** The offline
  branch decrements `CA0` to `CA<cheatNum>` by one per use, then clamps each
  higher slot to the one below it. Its demo values are 4, 3, 3, 2, 2. So
  `CA<n>` reads as "consumables usable in slot `n`" = the count with cheat
  class `>= n`, and a consumable of class `k` can fire any slot up to `k`.

So a consumable is consumed **on activation, during the game, after the
server validates it**, not at launch and not on victory.

### Slot to item id (Unresolved: the SWF has no item ids)

The seeds hold 160 instruments (`items.flags & 1`) and 174 consumables
(`flags & 2`):

- Generic instruments 2906-2913 (TC5-TC40), 3736 (TC13), 3737 (TC18),
  3743 (TC48), 3893 (TC51); per-archetype instruments from 5799.
- Generic consumables 2895-2902 (TC5-TC40), 3894-3897 (TC8, 13, 18, 23);
  "Minigame Consumable: <Archetype>" for 7 archetypes × 22 TC levels (6798
  Asgard, 6799 Archaeologist, 6800 Commando, 6801 Goa'uld, 6803 Jaffa, 6804
  Soldier, ... at TC3).

There are seven archetype classes plus Generic but only five `CA` slots, so a
slot is a cheat *class* (0-4), not an archetype. The SWF sees only class
numbers and counts; `itemCheatInfo` is a host-to-extension request carrying
`CA0`-`CA4`, not an id table. No seed column carries a cheat class either, so
which item has which class (perhaps a TC band) cannot be derived from this
repo. That mapping, and the `instcc` of each instrument, are server-owned:
the base computes them at launch and on each `itemCheatInfo`.

### Are the consumables obtainable? No (Confirmed, README M8)

- The archetype consumables are crafted from 154 blueprints in
  `db/resources/Entities/Seed/blueprints.sql`. Their blueprint items 8666-8820
  ("Blueprint: <Arch> Minigame Consumable TC<n>", flags 35840) appear on no
  `item_list_items` row and in no mission reward.
- No `item_list_items` row (the vendor and loot lists) and no mission reward
  names any minigame consumable, generic or archetype.
- The only minigame item any player can receive is the instrument **2907**
  "Generic TC10 Mini-game Instrument", as mission reward group 6, reward_id 7
  (`db/resources/Missions/Seed/mission_rewards.sql`).

## 6. Ability-started games

None of the target effects below has a `script_name`, an `event_set_id` or an
`effect_nvps` row (Confirmed, `db/resources/Effects/Seed/effects.sql` and
`Effects/Tables/effect_nvps.sql`). The original Python had no effect script
that starts a minigame. Everything about "what they did" below comes from the
seed names and descriptions.

| Ability → effect | Seed name / description | Starts a game? | Game | Who plays / target | Victory | Failure |
| --- | --- | --- | --- | --- | --- | --- |
| 1327 → 1640, 1641, 1642, **1643** | "Converse Minigame": 1640 pulser, 1641 range check, 1642 moniker removal, 1643 "Force Converse Minigame on Player" (flags 512) | Yes, 1643 | Converse | The caster plays; the target is the NPC. 1641 keeps the pair in range | none in seeds (Inferred: 1642 removes the NPC's moniker) | none |
| 2089 → 2811, 2812, **2813** | "Conversation: Humanoid: Basic": 2812 Engagement State 60 s, 2813 "Conversation Mini-Game" (flags 49244) | Yes, 2813 | Converse (basic humanoid) | The caster plays; the target NPC is held in the engagement state for 60 s | none in seeds | none (engagement times out) |
| 1597 → 1939 | Prison Boot | No: a lock, see § 7 | Livewire, started by item 3438 | The wearer | ability 1598 removes it | the boot stays |
| 1661 → 2032; 2837 → 4116 | 4116 "Minigame Win Container", 100 % self damage | No: post-result hook | n/a | The container entity | it destroys itself | n/a |
| 585 → 647 | Despawn Mob | No: post-result hook | n/a | `pendingMob` | the mob despawns | n/a |
| 586 → 648 | Consume Item | No: post-result hook | n/a | `pendingItem` | the triggering item is consumed | n/a |
| 1594 → 1936 | Cellblock success dialog | No: post-result hook | n/a | The player | shows the dialog | n/a |
| 1753 → 2186 | Castle Copplemann success dialog | No: post-result hook | n/a | The player | shows the dialog | n/a |

Only 1643 and 2813 start a game. The others are what CME's minigame results
handler fired after a game: they consume what started it (586 on
`pendingItem`, 585 on `pendingMob`, 2837 on a container), or show a result
dialog. Cimmeria already expresses those as content-engine
`on_victory_chains` / `on_failure_chains` (for example
`db/resources/Content/Seed/castle_cellblock_chains.sql:75-209`), which call
`launch_ability` with these ids. They need no new effect script.

### MG-24 effect-script contract (Inferred)

- One script, `start_minigame`, attached to 1643 and 2813.
- Params, read from the effect's `params` map (`EffectDef.params` is a
  `HashMap<String, String>`, `crates/entity/src/abilities/defs.rs:328-348`;
  a `param_str` helper beside `param_i32` is all that is missing):
  - `minigame_type`: the game name, the same string the content-engine
    `Action::StartMinigame { minigame_type, difficulty }` takes
    (`crates/content-engine/src/actions.rs:224`).
  - `difficulty`: 1-5, default 1, same range check as the content engine.
- Who plays: the **caster**. The effect's target (the NPC) is recorded as
  `pendingMob`, so a result hook can act on it.
- Path: the script calls the same cell launch path as the content action
  (dialog 53 → cell 24 → base `startMinigame`), not a second launcher.
- The effect does nothing if the caster is already playing (bit 4 set).

## 7. Prison Boot 1597

### What the seeds do (Confirmed, `castle_cellblock_chains.sql:75-209`)

- Mission 689 is a hidden internal tracker.
- Chain 1022 accepts 689 on `player_loaded` in Castle_CellBlock.
- Chain 1023 runs `launch_ability 1597` while 689 is not complete. The
  **server** casts 1597 on the player at load; the player never casts it.
- Chain 1024: `item_use` of item 3438 "Prison Boots" starts Livewire, with
  `on_victory_chains [1025]`.
- Chain 1025 runs `launch_ability 1598` "Disable Your Prison Boot" and
  completes 689. 1598's effects are 3081 ("Swaps the prison boot with a
  version that can be unequipped and that doesn't launch the minigame") and
  1942 ("launched from a minigame on success - removes the
  MISSION_Cellblock_PrisonBoot effect").
- Item 3438 (flags 34176) is worn from character creation by every
  non-Goa'uld Castle_CellBlock archetype (`char_creation_choices`).

### What is missing

Effect 1939 has `pulse_count 1` and no script, so today it does nothing: no
flag, no movement rejection. The prison boot does not hold anyone. The chain
file's comments already flag this as a HIGH-urgency UAT item.

### MG-63 design (Inferred)

- Effect 1939 becomes a persistent, non-pulsing effect with a script that
  adds one **counted** movement lock (§ 2) on apply and removes it on removal.
  The client then disables movement input through bit 6, with no client patch.
- The server's own movement gate rejects position updates while the lock
  count is above 0, so a modified client cannot walk out.
- Effect 1942 (from 1598) removes 1939; the lock count drops and bit 6 clears.
- 1939 must not survive a relog after 689 completes: chain 1023 already checks
  "689 not completed", so persisting the effect is unnecessary. Re-cast on
  load is the persistence.
- The launch item stays 3438; MG-63 does not change chains 1022-1025.
- Because the minigame launch also adds a lock (§ 2), the two locks must stack
  through the counter, never a raw flag set/clear, or the minigame's cleanup
  would free a booted player.

## 8. Ability 802 and event set 1132

### Confirmed facts

- Ability 802 "Reveal Mini-Games" ("Reveals Mini-Games on Mini-Map") is a
  passive (flags 128) with no effects and `event_set_id` NULL. Ability 1552
  "Reveal Research Targets" has the same description and the same empty shape.
- Event set 1132 "Minigame reveal target" maps to sequence 2304: event_id 2000,
  kismet `KIS-abilities_human.KIS-SA_Effect_Target`, NVPs
  `pfx1=PFX-abilities.PFX-MiniGameReveal`, `pfx1Socket=Buff`
  (`db/resources/Events/Seed/event_sets_sequences.sql`, `sequences.sql`,
  `sequences_nvp.sql`).
- No ability, effect, item or chain row references event set 1132.

### README M7: confirmed

802 reveals nothing server-side today. 1132 is a client-only particle effect
played on a target's `Buff` socket. **Inferred** original intent: owning 802
made the server play 1132 on nearby `INT_Minigame*` objects (and mark them on
the minimap) for that player only. Implementing it is out of the campaign's
current scope; if wanted, it is a per-player AoI event-set play keyed on the
passive, with no client patch.

## 9. Corrections to other docs

> [!WARNING]
> `docs/reverse-engineering/findings/minigame-architecture.md` merges the
> dialog fields into `onStartMinigame`. They belong to `onStartMinigameDialog`
> (client 53); `onStartMinigame` (client 52) takes only `WSTRING URL`. The
> dispatch table and the def are correct.

The archetype mask (`0xffff`) and `abilityBitfield` are different fields; see
§ 4 and `castle-rebuild/worknotes/m701.md:199-207`.

## Packet inputs

### MG-21: start dialog, flags, expiry

- Send client 53 with:

  | Arg | Value |
  | --- | --- |
  | `Name` | the game's display name (`MINIGAME_NAMES` entry, for example `Livewire`) |
  | `Difficulty` | `"Difficulty <n>"` with the 1-5 difficulty (the original sent the literal `"DifficultyLevel"`; any short text renders) |
  | `TCLevel` | the session's `techcomp` (MG-22) |
  | `Verb` | the game's display name again (the client prints `<Difficulty> TC <TCLevel> <Verb>`) |
  | `ArchetypeBitfield` | `0xFFFF` (all archetypes lit; display only) |
  | `CanPlay` | 1 |
  | `CanCall` | 0 |
  | `CanSpectate` | 0 |

- Remember the offered game on the cell entity until cell 24, cell 30 or
  expiry.
- **Expiry: 60 s.** On expiry send client 54 and treat the game as cancelled.
- Cell 24 starts the offered game. A cell 24 with no pending offer is
  rejected and logged.
- Cell 30 clears the pending offer and reports `NotStarted` (3) to the
  launch's failure path. A cell 30 with no pending offer is a silent no-op:
  the client echoes cell 30 after every server-sent 54.
- Flags: set bit 4 and add one counted movement lock when the dialog opens
  (the original's timing); clear both on cell 30, on expiry, on results and
  on disconnect.

### MG-22: `techcomp` and `abilityBitfield`

- `techcomp` = the `tech_comp` of the triggering instrument item
  (`pendingItem` / `tcOverride`) when there is one, else 1. Clamp to 1-55.
- `abilityBitfield`: 15 bits, bit `n` = cheat category `n` (§ 4, Confirmed).
  The ability-to-bit table in § 4 is Inferred from seed descriptions; ship it
  only behind a G1 decision, and keep **0** until the per-game ports
  (MG-R1 to MG-R5) implement the effects. The Trump passives are not bits.
- Never copy the dialog's archetype mask into it.
- `intelligence` = the cell's current intelligence stat; `pclevel` = the
  player's level (§ 3).
- The URL host segment (`ipaddress`) must be non-empty (§ 3)
  (an empty host fails the connect; it does not go offline).

### MG-23: item cheats

- `instcc` = -1 (no instrument), `CA0`-`CA4` = 0: the original's values.
- Consumption, if ever implemented: one item per `activateitemcheat`, during
  the game, through `consumeItemByMinigame(ticket, cheatClass, requestId)`,
  answered with `itemCheatActivate` and `itemCheatInfo` (§ 5). Instrument
  uses consume nothing and are once per game.
- The item-to-class map is not in the SWF or the seeds, and the consumables
  are unobtainable (M8). **Recommend G1 marks MG-23 not applicable** beyond
  keeping `instcc = -1` and the zeros.

### MG-24: effect-started games

- Script `start_minigame` on effects 1643 and 2813.
- Params: `minigame_type` (string), `difficulty` (1-5, default 1).
- The caster plays; the effect target becomes `pendingMob`.
- It goes through MG-21's dialog path.
- No-op if the caster already has bit 4.

### MG-62: seed values for effects 1643 and 2813

| Effect | `minigame_type` | `difficulty` | Victory | Failure |
| --- | --- | --- | --- | --- |
| 1643 | `Converse` (or the basic-humanoid variant, pending D-MG3) | 1 | none; optionally ability 1327's effect 1642 (moniker removal) on the NPC | none |
| 2813 | `Converse` (basic humanoid, pending D-MG3) | 1 | none | none; the 60 s engagement (2812) lapses |

### MG-63: Prison Boot

- Effect 1939: persistent, scripted, adds one counted movement lock on apply
  and removes it on removal.
- The server movement gate rejects movement while the lock count is above 0.
- Effect 1942 removes 1939.
- The launch item stays 3438; chains 1022-1025 are unchanged.
- Test: a booted player's position update is rejected; after the 1025 chain
  runs, it is accepted; a minigame started and finished while booted leaves
  the player still locked.
