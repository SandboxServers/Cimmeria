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

### Candidate abilities (Confirmed as seed rows; their bit positions are Unresolved)

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
`event_set_id` or an effect that names a bit. 677-686 are the Archaeologist's
ten in-game cheats; the SWF reads `abilityBitfield` to decide which of its
cheat buttons to show. 809, 815, 2507 and 2508 act on the player's combat
stats while bit 4 is set and are not SWF bits. 813, 814 and 1195 are scoring
modifiers the server applies to the reward, not SWF bits.

### Bit to ability (Unresolved)

No server source in this repo maps a bit to an ability. The map lives in the
SWFs' `activateabilitycheat` readers, which MG-R0's tools decompile. The most
likely shape (**Inferred**) is bit `n` = the `n`-th ability of 677-686 for
Archaeologist cheats, with the Converse Trump passives read by the Converse
SWF only. MG-22 must not ship a non-zero mask until an MG-R0/R1-R5 spec names
the bits.

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

### Semantics (Inferred)

Read together, the def describes this flow:

1. At launch the base reads the player's equipped or used instrument and sends
   its cheat class as `instcc` (-1 when there is none).
2. `CA<n>` is the number of activations available in cheat class `n`: the count
   of matching consumables the player holds.
3. When the SWF sends `activateitemcheat` with a class, the minigame service
   asks the cell to consume one item of that class
   (`consumeItemByMinigame(ticket, cheatClass, requestId)`); on success it
   pushes fresh counts with `updateMinigameItemCheats`.
4. The results report whether an instrument and a consumable were used.

So a consumable is consumed **on activation, during the game**, not at launch
and not on victory.

### Slot to item id (Unresolved)

The seeds hold 160 instruments (`items.flags & 1`) and 174 consumables
(`flags & 2`):

- Generic instruments 2906-2913 (TC5-TC40), 3736 (TC13), 3737 (TC18),
  3743 (TC48), 3893 (TC51); per-archetype instruments from 5799.
- Generic consumables 2895-2902 (TC5-TC40), 3894-3897 (TC8, 13, 18, 23);
  "Minigame Consumable: <Archetype>" for 7 archetypes × 22 TC levels (6798
  Asgard, 6799 Archaeologist, 6800 Commando, 6801 Goa'uld, 6803 Jaffa, 6804
  Soldier, ... at TC3).

There are seven archetype classes plus Generic but only five `CA` slots. No
seed column carries a cheat class, so the slot to item mapping cannot be
derived from this repo. It would come from the SWF (`itemCheatInfo`, MG-R1) or
a CME data table we do not have.

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
- `abilityBitfield` = **0** until an MG-R0 to MG-R5 spec names the SWF bits.
  The candidate abilities are 677-686 and the Trump passives 778, 779, 792,
  793 (§ 4). Flag at G1.
- Never copy the dialog's archetype mask into it.

### MG-23: item cheats

- `instcc` = -1 (no instrument), `CA0`-`CA4` = 0: the original's values.
- Consumption, if ever implemented: one item per `activateitemcheat`, during
  the game, through `consumeItemByMinigame(ticket, cheatClass, requestId)`.
- The slot to item id map is not derivable, and the consumables are
  unobtainable (M8). **Recommend G1 marks MG-23 not applicable** beyond
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
