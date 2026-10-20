# Minigame content map (MG-R5)

Research map for the minigames campaign, Wave 0B. It answers the content half
of [work-packets.md § MG-R5](work-packets.md): every seed reference to each
game, the `start_minigame` rows, interaction bits 256-4096, the named missions,
steps and objectives, dialogs, NPCs and items, the abilities from MG-R7's list,
how a right-click on a minigame object reaches the server, step 2307 and a
chain shape for MG-61 (D-MG7), and verdicts on README M1-M5. The shell movies
are in [spec-shells.md](spec-shells.md).

Tags follow [spec-launch-flow.md](spec-launch-flow.md): **Confirmed** (read
from a seed row, Rust, Python or client data, cited by file and line),
**Inferred** (a reasoned proposal for G1) and **Unresolved** (no source here
answers it).

All seed paths are under `db/resources/`. Line numbers are at commit
`fbdb30075`.

## Contents

1. [Verdicts on README M1-M5 and F12, F14](#1-verdicts)
2. [Seed references by game](#2-seed-references-by-game)
3. [The `start_minigame` rows](#3-the-start_minigame-rows)
4. [Interaction bits 256-4096](#4-interaction-bits-256-4096)
5. [Missions, steps and objectives](#5-missions-steps-and-objectives)
6. [Dialogs and NPCs](#6-dialogs-and-npcs)
7. [Items](#7-items)
8. [Abilities](#8-abilities)
9. [Right-click to server](#9-right-click-to-server)
10. [Step 2307 and the MG-61 chain shape](#10-step-2307-and-the-mg-61-chain-shape)
11. [Open questions](#11-open-questions)

## 1. Verdicts

| Item | Verdict | Evidence |
| --- | --- | --- |
| M1 (shells hold no rules) | **Confirmed** | [spec-shells.md §1](spec-shells.md#1-verdict) |
| M2 (no content names Converse or ConverseBasicHumanoid) | **Confirmed** | No quoted game name other than `'Livewire'` anywhere in `db/resources` (§2). The only Converse leads are step 2307's text, Anat's screen 5437 and Harset's Hansen exchange (§6) |
| M3 (no template carries bits 512-4096) | **Confirmed** | §4 |
| M4 (only step 2307 names a game, and only in text) | **Confirmed** | §5 |
| M5 (no `start_minigame` row sets a failure chain) | **Confirmed** | §3 |
| F12 (10 `start_minigame` rows) | **Corrected: 12** | §3 |
| F14 (bits 256-4096 unused except 256) | **Confirmed** | §4 |

## 2. Seed references by game

A Grep for each game name, quoted, over `db/resources/` (Confirmed):

| Game | Seed references |
| --- | --- |
| `Livewire` | The 12 `start_minigame` rows (§3); interaction asset `GLB_Components.Minigame_Livewire` (`Worlds/Seed/interactions.sql:83`, id 8); template 303 'Debug Hub - Livewire Terminal' (§4) |
| `Activate` | Interaction asset `GLB_Components.Minigame_Activate` (`interactions.sql:85`, id 9) |
| `Analyze` | Interaction asset `GLB_Components.Minigame_Analyzer` (`interactions.sql:9`, id 10) |
| `Bypass` | Interaction asset `GLB_Components.Minigame_Bypass` (`interactions.sql:11`, id 11) |
| `Converse` | Interaction asset `GLB_Components.Minigame_Converse` (`interactions.sql:13`, id 12) |
| `Hack`, `Alignment`, `CrystalGame`, `GoauldCrystals`, `ConverseBasicHumanoid` | None |

The interaction rows give each kind a body set, cursor and minimap icon; none
starts a game. Interaction ids 8-12 are the bit numbers of
`docs/content/interaction-flags.md:59-65` (256 = bit 8 Livewire through
4096 = bit 12 Converse), so each row is the asset set for one bit (Confirmed
by the matching names).

Enum rows that name minigames (Confirmed, all under `Enums/` seeds):
`EInteractionNotificationType.sql:14-18`, `EUnitType` (MinigamePlayer,
MinigameGiver), `ESubsystem` (SUBSYSTEM_Minigame), `ETextType` (five
Minigame* values), `EMoniker` (MONIKER_Minigame_Reuse_Timer),
`EConditionHandlerFeedback.sql:162-171`, `EStateField` (BSF_PlayingMinigame),
`EGiveCashTag` (GCT_MinigameCall, GCT_MinigameContact), `EItemFlag`
(MinigameInstrument, MinigameConsumable), `EMinigameCallResult` and
`EMinigameCallEndedResult`.

The content loader does not validate names: test fixtures use a lower-case
`"livewire"` (`start_minigame_params.rs:29`, `negative_logging.rs:68`) and
load (Confirmed).

## 3. The `start_minigame` rows

Twelve rows, all `'Livewire'`, none with `on_failure_chains` (Confirmed). All
paths are under `Content/Seed/`.

| File | Line | Chain | Victory chain | Difficulty |
| --- | --- | --- | --- | --- |
| `castle_702_704_chains.sql` | 742 | 1292 | 1293 | default |
| `castle_706_708_chains.sql` | 785 | 1356 | 1357 | default |
| `castle_701_chains.sql` | 381 | 1233 | 1234 | default |
| `castle_cellblock_chains.sql` | 198 | 1024 | 1025 | default |
| `castle_cellblock_chains.sql` | 706 | 1016 | 1017 | default |
| `castle_cellblock_chains.sql` | 1109 | 1041 | 1042 | default |
| `castle_cellblock_chains.sql` | 1504 | 1060 | 1061 | default |
| `dakara_e1_m1649_terminal_chains.sql` | 100 | 8223 | 8231 | 1 |
| `dakara_e1_m1650_chains.sql` | 182 | 8258 | 8261 | 1 |
| `debug_area_plaza_chains.sql` | 109 | 13004 | 13005 | default |
| `debug_hub_chains.sql` | 98 | 7004 | 7005 | default |
| `sgc_w1_chains.sql` | 422 | 3023 | 3024 | default |

README F12 counts 10. The 12 above come from a Grep for `start_minigame` over
`Content/Seed/` at `fbdb30075` (Confirmed); which two rows the README missed is
not worth reconstructing.

## 4. Interaction bits 256-4096

Bits from `docs/content/interaction-flags.md:59-65` and
`crates/entity/src/interaction_flags.rs:36-44` (Confirmed): 256
`INT_MinigameLivewire`, 512 `INT_MinigameActivate`, 1024
`INT_MinigameAnalyze`, 2048 `INT_MinigameBypass`, 4096 `INT_MinigameConverse`.
There is no bit for Hack, Alignment, CrystalGame, GoauldCrystals or
ConverseBasicHumanoid.

- **Templates (Confirmed).** A scan of every `entity_templates*.sql` row finds
  exactly one template with any of these bits: template 303
  'Debug Hub - Livewire Terminal', `interaction_type` 256
  (`Entities/Seed/entity_templates.sql:1020`). No template carries 512-4096.
- **Chains (Confirmed).** No `set_interaction_type` mask in any seed sets
  512-4096.
- **What the bits do (bit 256 Confirmed; 512-4096 Inferred).** They make the
  client show the minigame cursor and offer the click. They start nothing on
  the server; §9 shows the path.

## 5. Missions, steps and objectives

| Mission | Step | Objective | Text | Status | Source |
| --- | --- | --- | --- | --- | --- |
| 582 'A Question of Loyalty' | 2307 'Conversation Minigame With Marsh' (index 0) | 2675 (text `' '`), task 3277 | names a "Conversation Minigame" | mission enabled; **step disabled** | `Missions/Seed/missions.sql:1487`, `mission_steps.sql:5335` |
| 582 | 3274 'Convince Marsh you are innocent.' (index 1) | | | | `mission_steps.sql:5337` |
| 1244 'Find the Mole' | 3619 'Search Petbe's quarters.' | 4203 'Placeholder minigame.' | | objective disabled | `mission_objectives.sql:559` |
| 1460 'Improving Security' | 4291 | 4963 'to do: add button to dialog for minigame' | | | `mission_objectives.sql:1751` |
| 185 'What does this Button Do?' | 652 | 789 'TEMP: Interact with research object.\nTODO: Minigame' | | | `mission_objectives.sql:3585` |

All Confirmed. Mission 582 is level 1, label 'General', moniker family
`Harset_OPC_13`. Its sequel is mission 677 'A Question of Loyalty, Part 2'
(`missions.sql:1637`).

- **Only step 2307 names a game kind (M4 Confirmed).** 4203, 4963 and 789 say
  "minigame" with no kind.
- **Objective 4203** belongs to template 244 and is routed under Harset D-H13
  (D-MG6 = A); this campaign does not wire it.
- **Objective 789 (Inferred: Activate or Analyze).** "Interact with research
  object" fits the Analyzer interaction (id 10); nothing in the seeds says
  which.
- **Objective 4963 (Inferred: Bypass or Hack).** A security mission with a
  "button to dialog" note reads as a dialog-launched game; nothing says which.

## 6. Dialogs and NPCs

### Mission 582's offer: dialog set 530 (Confirmed)

`dialog_set_maps` rows 2935-2937 (`Dialogs/Seed/dialog_set_maps.sql:2089-2093`):

| Dialog | Type | Speaker | Text, abridged |
| --- | --- | --- | --- |
| 2431 | DUIST_DefaultBlurb | 948 Blackstock | "Marsh wants to see you. Something about an SGC sighting here on Harset." Buttons Accept, More Info |
| 2432 | DefaultDialog | 941 Colonel Marsh | "It's best not to keep the Colonel waiting..." |
| 2433 | DefaultDialog | 941 Colonel Marsh (screen 43073) | "Blackstock presented me with some very interesting videotapes... Captain Whelan... You were taped escorting him to the gate... Do you want to try and convince me why you shouldn't be punished?" |

Dialog 2433 is the step-2307 prompt (Inferred from its text). **None of the
three has `accepts_mission_id` or an `event_set`, and no `add_dialog_set`
action adds set 530 (Confirmed).** Mission 582 cannot be offered today.

Mission 677's set 624 (dialogs 2434-2437) carries Blackstock's punishment
lines with flags 134217728, 268435456 and 536870912 (Confirmed). It is the
"lost the argument" branch (Inferred).

### `(Minigame)` dialog topics (Confirmed)

`dialog_set_maps` rows 1047, 1317, 1318, 1338, 903, 904, 3195, 5334 and 5335
carry topics titled `(Minigame)`; rows 1571, 1582, 5324, 5364, 5365, 3781 and
6980 carry the flag only. None is added by any chain. Dialog screens 1415,
5437 (Anat, "Am I beautiful?", a Converse-style exchange), 5872 (help text)
and 60103 mention minigames.

The Harset Hansen exchange (`Content/Seed/harset_opcore_chains.sql:761`) is a
conversation that Harset D-H12 plays as a dialog choice, not a Converse game
(Confirmed).

### Marsh (Confirmed)

| Spawn | World | Template | Tag | Source |
| --- | --- | --- | --- | --- |
| 344 | 68 (Harset) | 10 | `CmdCenter_Marsh` | `spawnlist.sql:821` |
| 7 | 12 | | `Preparation_ColMarsh` | `spawnlist.sql` |
| 118 | 8 (Castle) | | `Castle_ColMarsh` | `spawnlist.sql` |

Speakers: 261 'Col. Marsh', 941 'Colonel Marsh', 948 'Blackstock'. Existing
Harset chains already key on `CmdCenter_Marsh`: 6501 (mission 1360, step
4038), 6511, 6512 (mission 1361 briefing) and 6527 (mission 1361, step 4694),
all using dialog slot 10 (`harset_opcore_chains.sql`).

## 7. Items

488 item rows mention minigames (Confirmed, scan of `Items/Seed/items.sql`).
They fall into five groups by flags:

| Group | Flags | Ids |
| --- | --- | --- |
| Blueprints | 35840 | many |
| Archetype instruments | 36225 | many |
| 'Minigame Consumable: X' | 36226 | many |
| Generic instruments | 35841 | 2906-2913 |
| Generic consumables | 35842 | 2895-2902, 3894-3897 |

How they feed `instcc` and `CA0`-`CA4` is [spec-launch-flow.md §5](spec-launch-flow.md#5-item-cheats)
and README M8. No shell reads them ([spec-shells.md §7](spec-shells.md#7-cheats)).

## 8. Abilities

MG-R7's list, checked against trainer, tree, kit and chain seeds (Confirmed):

| Ability | Where it is used |
| --- | --- |
| 778, 779, 792, 802, 809, 677-686, 1552, 815, 2507, 2508, 813, 1195 | Archeologist trainer (`trainer_abilities.sql:404-532`) and ability tree |
| 793 | Not in the tree |
| 1327, 2089 (Converse launchers) | **Granted nowhere** |
| 1597, 1598 | Used by Cellblock chains 1023 and 1025 |
| 585, 586, 1594, 1661, 1753, 2837 | No use |

Ability 802 drives event set 1132 'Minigame reveal target' and sequence 2304
PFX-MiniGameReveal (spec-launch-flow.md §8). The ability-launch path itself is
spec-launch-flow.md §6.

## 9. Right-click to server

How a right-click on an `INT_Minigame*` object reaches the server (Confirmed,
except where tagged):

1. **Client.** The interaction bit (§4) shows the minigame cursor and the
   client sends the cell method `interact` with the target id (bit 256
   Confirmed; 512-4096 Inferred).
2. **Cell `interact`** (`crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`,
   `handle_interact`): validates the arguments and liveness, reroutes a hostile
   target to `right_click_attack`, checks range (`interact_target_in_range`),
   pins the target (`pin_interaction_target`), tries a trainer, then fires the
   content triggers `interact_tag` and `interact_template` (`fire_interact_tag`,
   `fire_interact_template`).
3. **Fallback** (`crates/cell-interactions/src/cell/interactions/dispatch/interact.rs`):
   per-player dialog sets, DHD, org registrar, then the static
   `NpcInteractionType`; with none it calls `interact_no_effect`.
4. **Start.** Only a chain's `start_minigame` action starts a game
   (`crates/cell-content/src/cell/content/executor/dispatch.rs:177-219`). It
   sends `CellToBaseMsg::StartMinigame` with `player_level`, `intelligence`,
   `tech_competency = 1` and `abilities_mask = 0` (MG-22).
5. **Base** builds the `onStartMinigame` URL and sends it
   (`crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs:126-130`);
   the client loads the package by name ([spec-shells.md §12](spec-shells.md#12-client-type-table-and-handler-0x00e32140)).

So an object with a minigame bit and no matching chain gives `interact_no_effect`
(Confirmed). The `MinigamePlayer` cell methods (indices 20-34,
`crates/cell-methods/src/cell/cell_methods/minigame.rs`) are all unimplemented
except END_CURRENT, which sends `CancelMinigame` (Confirmed).

## 10. Step 2307 and the MG-61 chain shape

### The answer

| | Value | Tag |
| --- | --- | --- |
| NPC | Colonel Marsh, speaker 941, spawn 344, tag `CmdCenter_Marsh`, world 68 | Confirmed |
| Dialog | Set 530; dialog 2433 is the prompt (2431 is Blackstock's offer, 2432 Marsh's line) | Confirmed rows; role Inferred |
| Game | `ConverseBasicHumanoid` | Inferred: Marsh is human, and it is the only Converse movie with rules (`Converse` is a shell, spec-shells.md §1) |

### Proposed chain shape for MG-61 (D-MG7)

Modeled on chain 6501 (`harset_opcore_chains.sql:195-251`), which already
talks to the same Marsh (Inferred; for G1):

1. **Offer.** Bind set 530 to Marsh, or give dialog 2431
   `accepts_mission_id = 582` in the seed, so mission 582 can be accepted.
2. **Chain A, start.**
   - Trigger: `interact_tag 'CmdCenter_Marsh'` (player).
   - Conditions: `world 68`; `mission_status 582 eq active`;
     `step_status 582 '2307' eq active`; `neq` guards against mission 1360
     step 4038 and mission 1361 step 4694, so the existing Marsh chains stay
     disjoint.
   - Actions: `display_dialog 2433`, then
     `start_minigame 'ConverseBasicHumanoid'` with
     `{"on_victory_chains":[B], "on_failure_chains":[C]}`.
3. **Chain B, win.** Complete objective 2675 and advance to step 3274.
4. **Chain C, loss (optional).** The punishment path toward mission 677 and
   set 624.

Also needed: set `step_enabled = true` on step 2307 once the game is
server-validated (D-MG7), and give Marsh's template the bit that shows the
right cursor (4096 `INT_MinigameConverse`, Confirmed bit; using it here is
Inferred) or rely on the dialog. Whether the
Converse launch abilities 1327/2089 should be granted is a separate question
(§8). If ConverseBasicHumanoid turns out to have no usable hook, D-MG7 says the
architect re-rules at G1.

## 11. Open questions

- **Q-C1 (Unresolved).** Which games objectives 789 and 4963 meant. Nothing in
  the seeds or the client names them.
- **Q-C2 (Unresolved).** Whether bits 512-4096 change client behaviour beyond
  the cursor. A lab right-click on a template given bit 512 would show it.
- **Q-C3 (Unresolved).** Whether the 2009 content gave Marsh a Converse launch
  through ability 1327 or 2089 rather than a chain.
