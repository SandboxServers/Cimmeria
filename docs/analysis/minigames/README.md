# Minigames campaign

> Type: explanation and ledger. Audience: the orchestrator, the architect, packet agents, the owner.
> Written 2026-10-19 by the architect against `main` @ `0d71b3b30`; updated at gate G1 the same day. Companions: [work packets](work-packets.md), [resume card](RESUME.md), [minigame system](../../gameplay/minigame-system.md), [client binary findings](../../reverse-engineering/findings/minigame-architecture.md), [wire formats](../../reverse-engineering/findings/minigame-wire-formats.md), [CAT-K audit](../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md).

## Purpose

Implement every Stargate Worlds minigame the client has content for, as a real server-validated game wherever the client content allows it. Where the content does not allow it, say so per game and harden what is left. Launch, result, failure and modifier plumbing, plus the content that starts the games, are in scope. The social layer (call a helper, spectate, contact) is not (D-MG4).

The campaign started with a research wave (wave 0). Gate G1 (§ Gates) read its specs and filled the port packets in waves 3 to 5, so every remaining packet is prescriptive.

## What the client has

The 11 packages under the client's `CookedPC\UI\Flash\` (the user's own client copy; nothing from it goes in git, D-MG10) each wrap one uncompressed Scaleform GFX movie. The architect scanned them on 2026-10-19; MG-R0 turned that into a repo tool, and the wave-0 specs confirm every row below. The tool's per-package inventory (offsets, md5s, function counts) is [client-packages.md](client-packages.md).

| Package | GFX movie | What the movie holds | Verdict (G1) | Spec (packet) |
|---|---|---|---|---|
| `Livewire.upk` | 410,721 B | Full game and an embedded server extension | **Recovered.** Server-validated today; 18 parity deltas (MG-LW1) | [spec-livewire-parity.md](spec-livewire-parity.md) (MG-R6) |
| `Alignment.upk` | 293,868 B | Embedded AS1 SmartFox server extension: a hex match-3 with cascades, hints, goal scores, item and ability cheats, `seed`/`rng` | **Recoverable.** Ported in MG-AL1..AL3b | [spec-alignment.md](spec-alignment.md) (MG-R1) |
| `CrystalGame.upk` | 94,203 B | Embedded extension: `processMove`, `checkForVictory`, `Crystal` | **Recoverable.** A lights-out puzzle, mod 6, on a 14x13 board; no cheats. Ported in MG-CG1..3 | [spec-crystalgame.md](spec-crystalgame.md) (MG-R2) |
| `GoauldCrystals.upk` | 154,559 B | Embedded `handleRequest`, `Swap`, `Crystal` | **Recoverable.** A path-building puzzle. Ported in MG-GC1a..GC3b | [spec-goauldcrystals.md](spec-goauldcrystals.md) (MG-R3) |
| `ConverseBasicHumanoid.upk` | 227,957 B | Embedded extension: `Card`, `Suit`, `Match`, `Swap`, `checkForVictory` | **Recoverable.** A timed card contest against an NPC hand. Ported in MG-CH1..3 | [spec-conversebasichumanoid.md](spec-conversebasichumanoid.md) (MG-R4) |
| `Hack.upk`, `Activate.upk`, `Analyze.upk`, `Bypass.upk`, `Converse.upk` | about 55.8 kB each, near-identical | **Shells**: a SmartFoxClient, a countdown timer, `fullgamestate`, `victory`/`failure` and a `win_btn` that sends `victory`. No rules | **Not recoverable as games.** Client-declared and hardened (MG-04, D-MG3 = A); MG-64 adds a server timer | [spec-shells.md](spec-shells.md) (MG-R5) |
| `DHD.upk` | 93,455 B | The dialling UI (`GateTravel`) | Not a minigame. Out of scope | none |

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Five of the ten client game types are shells with no rules: Hack, Activate, Analyze, Bypass and Converse. Their only win path is a button that sends `victory`. | GFX scan above; `Placeholder Activate/Analyze/Bypass` display names in the type table at `0x019d917c` | MG-R5, MG-04, MG-64, D-MG3 |
| F2 | Alignment, CrystalGame, GoauldCrystals and ConverseBasicHumanoid embed their original SmartFox server extension (AS1) in the SWF. That code is the rules source. | Function names in each GFX: `processmove`, `swapPieces`, `doCascade`, `checkForVictory`, `getHint`, `activateitemcheat`, `Card`, `Suit` | MG-R1..R4, D-MG1 |
| F3 | `deprecated/python/base/minigame/Alignment.py` and `GoauldCrystals.py` are unfinished half-ports. Their difficulty tables look real; their message handlers are copy-pasted Livewire (`opendoor`, wire names). | Read 2026-10-19 | D-MG1 |
| F4 | `minigame-architecture.md` is wrong twice. Line 34 says the placeholders have no SWF (they have shells), and lines 44-57 class Hack and Converse as "other" and CBH as a sub-type, though CBH is a full game. | This scan | MG-72 |
| F5 | Livewire is server-validated and works, but differs from the client extension in 18 places (spec-livewire-parity lines 37-58). Deltas 1 and 2 (wire library suffixes, variant range 1-4) make wires invisible today. | `games/livewire/setup.rs:102,261,327`; spec-livewire-parity.md | MG-LW1, D-MG17 |
| F6 | The factory sends every non-Livewire name to the placeholder. `kind.rs` `validation()` reads each module's `PORTED`, so a port flips one constant. | `crates/minigame/src/minigame/games/kind.rs` | MG-03 (integrated) |
| F7 | Base used to hard-code `tech_competency = 1`, `abilities_mask = 0`, `intelligence = 0` and `player_level = 1`. MG-02 made the cell supply them; the ability mask is still 0. | `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` | MG-02 (integrated), MG-22 |
| F8 | There was no failure path. MG-02 added `on_failure_chains` (D-MG5). | `crates/cell/src/cell/service/base_messages/minigame.rs` | MG-02 (integrated) |
| F9 | The original launch put up a pre-game dialog (client 53 `onStartMinigameDialog`); the client then called cell method 24 `startMinigame`, or 30 `minigameStartCancel`. Cimmeria skips the dialog, and both methods log UNIMPLEMENTED. | spec-launch-flow.md § 1; `crates/cell-methods/src/cell/cell_methods/minigame.rs:70,133` | MG-R7, MG-21, D-MG11, D-MG19 |
| F10 | `BSF_PlayingMinigame` (bit 4) and the counted movement lock are now held from launch to the result (MG-06), but the server does not refuse a move while they are held. | MG-06 merge; `client_move.rs` | MG-06 (integrated), MG-63a |
| F11 | Ability-started games are not wired. Ability 1327 (effect 1643), 2089 (effect 2813) and 1597 "Prison Boot" (effect 1939, released by 1942) all have `effects.script_name` NULL, so the boot holds nobody. | `db/resources/Effects/Seed/effects.sql`; spec-launch-flow.md §§ 6-7 | MG-24, MG-62, MG-63a, MG-63b, D-MG8, D-MG20 |
| F12 | Seeded content uses only Livewire (12 `start_minigame` rows). Mission 582 step 2307 "Conversation Minigame With Marsh" is `step_enabled = false`. Objective 4203 runs on Harset's interact route (D-H13). Objectives 4963 and 789 are authoring stubs that name no game. | `mission_steps.sql:5335`; content-map.md | MG-61, D-MG6, D-MG22 |
| F13 | Modifier content exists and nothing reads it: the archaeologist minigame abilities, the "Trump" passives and about 300 cheat consumables. The consumables have no source (M8). | spec-launch-flow.md § 5 | MG-22, D-MG17 |
| F14 | The interaction bits 512 (Activate), 1024 (Analyze), 2048 (Bypass) and 4096 (Converse) exist; there is no Hack bit. No seeded template uses them. | [interaction-flags.md](../../content/interaction-flags.md); content-map.md | MG-R5 |
| F15 | No repo tool extracted the GFX movies; MG-R0 added `tools/minigame-swf/` extraction and `list_functions.py`. | [tools/minigame-swf](../../../tools/minigame-swf/README.md) | MG-R0 (integrated) |
| F16 | `SfsValue` had no list form; MG-01 added `SfsValue::list`. | `protocol.rs` | MG-01 (integrated) |
| F17 | Several minigame files are at or near the 700-line hard cap (`server/tests.rs`, `server/listener_tests.rs`, `session_tests.rs`, `server/session_loop.rs`). New code and tests go in new files. | line counts 2026-10-19 | contract C2 |
| F18 | The social layer (cell methods 26-29 and 31-34) is still stubs. | `cell_methods/minigame.rs` | D-MG4 |
| F19 | CAT-K-01, the placeholder's instant client-declared victory, is closed for the shells by MG-04's 5 s floor. | CAT-K-minigame.md | MG-04 (integrated), D-MG13 |

## Decisions

| Id | Decision | Status | Reason |
|---|---|---|---|
| D-MG1 | The rules source is the AS1 server extension embedded in each SWF. The Python half-ports are a cross-check only, and the client binary is the source for the launch flow. | Decided | F2, F3: the extension is the code the original server ran; the Python never worked |
| D-MG2 | Every recovered game is server-validated. The server generates the board, validates every move, scores, runs the timer and decides the result. A client `victory` for a server-validated game is refused with a WARN. The server sends what the SWF expects, so nothing needs a client patch. | Decided | Server-authoritative rule; CAT-K-01 |
| D-MG3 | The five shell games stay client-declared and hardened (option A). No aliasing (B, MG-26) and no new rules (C). | Decided (orchestrator, owner autonomy directive 2026-10-19) | No seeded content names a shell (content-map); B would show the player a different game |
| D-MG4 | The social layer (help, call, spectate, contacts) is out of scope and goes to a follow-up campaign. | Decided (owner may overrule) | No design data beyond the client UI; it does not block any game |
| D-MG5 | `start_minigame` gains an `on_failure_chains` param. Those chains fire on result 2 (Failure) and 4 (Defeated). Results 0 (canceled) and 3 (interrupted or not started) fire nothing. | Decided; shipped in MG-02 | F8; the client's result strings sit at `0x00c7d0b0` |
| D-MG6 | Objective 4203 keeps Harset's D-H13 interact route (option A); MG-65 is not applicable. | Decided (orchestrator, owner autonomy directive 2026-10-19) | Changes a shipped Harset flow for no player gain |
| D-MG7 | Mission 582 step 2307 is enabled once ConverseBasicHumanoid is server-validated (MG-CH3). Target and hook confirmed at G1: D-MG22. | Decided | The brief's content-wiring goal; no client patch |
| D-MG8 | Abilities start minigames through a new effect script, `StartMinigame`, set on the effect's `script_name` in the seed. The `ability_used` content trigger is not used. | Decided (owner may overrule) | The effects already name the action (1643, 2813); range and cooldown checks run first |
| D-MG9 | Difficulty tiers come from each extension's own table. Content may say 1-5; a game with fewer tiers clamps to its highest. | Decided | Today's clamp is Livewire-only |
| D-MG10 | No client assets and no full disassembly go in git. The tools extract from the user's own client into a scratch folder. Specs carry pseudocode, tables and short excerpts of at most 20 lines each. | Decided | Repo invariant: the client is not in git |
| D-MG11 | MG-21 ships the original pre-game dialog behind `CIMMERIA_MINIGAME_START_DIALOG`, default **off**. Turning it on is a follow-up after a lab UAT (MG-73 recipe). | Decided (orchestrator, owner autonomy directive 2026-10-19) | Visible behaviour change; off changes nothing |
| D-MG12 | The server owns randomness. The SWF PRNG is not ported (MG-25 not applicable, D-MG17). | Decided | No client derives state from `seed` |
| D-MG13 | Client-declared games (the shells) need at least 5 s of play. An earlier `victory` is refused with a WARN and the session stays open. One result per session. | Decided; shipped in MG-04 | CAT-K-01 |
| D-MG14 | Item cheats in scope if MG-R7 maps them. | Superseded by D-MG17 | M8: the consumables are unobtainable |
| D-MG15 | `BSF_PlayingMinigame` and a counted movement lock are held from launch to the result. Every end path clears them. | Decided; shipped in MG-06 | F10 |
| D-MG16 | Every game gets a REPL transcript UAT (MG-05 harness), checked in and run as a test. Lab UAT is **ask-user-first**. | Decided | Repo rule and user memory |
| D-MG17 | G1 process and protocol rulings. **MG-23** is not applicable: the cheat consumables are unobtainable (M8) and no item-to-slot map exists, so every port sends `instcc = -1`, `CA0`-`CA4 = 0` and answers `activateitemcheat` with WARN `item_cheat_unavailable`. **MG-25** is not applicable (no client reads `seed`). **MG-26** is not applicable (D-MG3 = A). **MG-LW1** is needed: all 18 parity deltas, plus the adjacent `wire.cut` fix. Every port follows one message rule set (`client_victory_refused`, `unknown_command`, `aborted()` sets `canceled` and sends nothing). MG-63 splits into MG-63a and MG-63b in wave 2. Every packet merges into `minigames/integration`; one PR at the end. | Decided (G1) | Specs MG-R6, MG-R7; owner directive (one PR, autonomous to release-ready) |
| D-MG18 | MG-22: one shared cell launch helper. `techcomp` stays 1 (M9). `abilityBitfield` is built at launch from the player's known abilities and filtered per game by `kind.rs` `ability_bits()`, which is 0 until the game is `PORTED`. | Decided (G1) | spec-launch-flow § 4; a game sees only the bits it implements |
| D-MG19 | MG-21: the dialog flow is client 53, then cell 24 or 30, behind D-MG11's switch. Deviations from the original: the hold flags are set at launch, not when the dialog opens, and offer expiry is checked lazily when cell 24 arrives. | Decided (G1) | An unanswered dialog must not lock a player |
| D-MG20 | MG-24: the script is `StartMinigame` (PascalCase, the registry convention). The caster plays. The script queues a request; the cell drains the queue after effects resolve, through the same `start_or_offer` path, so D-MG11 applies to ability launches too. | Decided (G1) | Effect scripts cannot await the base channel |
| D-MG21 | Port slicing and recorded deviations. Alignment: AL1, AL2a, AL2b, AL3a, AL3b (`NEW_TYPE_USES_TIER = false`, no reshuffle, the `n1` and hoisting fixes, bit 10 is a no-op spend). CrystalGame: CG1-3 (reset rescrambles). GoauldCrystals: GC1a, GC1b, GC2, GC3a, GC3b (tutorial kept; armed item modes and path mode 2 not ported). ConverseBasicHumanoid: CH1-3 (timers held after `newdeal`, a client `failure` is a Defeat, pairing by play order). No shared crystals core. | Decided (G1) | Each slice fits about 40k working tokens; deviations are listed in the specs and the G1 report |
| D-MG22 | Content. MG-61: ConverseBasicHumanoid at difficulty 1 on Marsh (`interact_tag CmdCenter_Marsh`, world 68, dialog 2433), chains 24000-24001, guarded against missions 1360 and 1361; no offer path (M10). MG-62: effects 1643 and 2813 get `StartMinigame` with NVPs 25501-25504 and no outcome chains. MG-63b: the boot is a movement-only hold; a respawn frees the wearer until the next load (accepted). MG-64: no shell content exists, so it becomes the placeholder's 60 s server timer and a load-time check on minigame names. | Decided (G1) | content-map.md §§ 6, 10; spec-launch-flow §§ 6-7; spec-shells §§ 8, 10 |
| D-MG23 | MG-74: a row of six reusable terminals in the Debug Area (world 1300, plaza north edge), one per ported game (CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid, Livewire) plus the `Hack` shell as the MG-64 timer fixture. GM test fixtures, not player content, so D-MG3 = A still holds. Each is its own copy of template 303 with a `display_name` label and cursor bit 256 (the only minigame bit with a Confirmed click path; Q-C2 stays open), no mission gate, difficulty 1, and shared win/loss bark chains so the D-MG5 failure hook is exercised. Ids: chains 24050-24069, templates 1600-1609, spawns 14100-14109, dialog 60120, screens 200020-200029 (C7). Split into MG-74a (seeds and test) and MG-74b (docs). | Decided (architect, 2026-10-20) | Hand and lab testing (MG-73) need a fixed launcher per game; content-map.md § 9; seeds only, no client patch |

## Missing content register

What the client never shipped, or the seeds never authored.

| # | Missing | Effect | Confirmed by |
|---|---|---|---|
| M1 | Rules for Hack, Activate, Analyze, Bypass and Converse | They cannot be server-validated; D-MG3 = A | MG-R5 (confirmed) |
| M2 | Which NPCs and dialogs use Converse rather than ConverseBasicHumanoid | Converse content wiring stays limited to what the seeds name | MG-R5 (confirmed) |
| M3 | Placed world objects with interaction bits 512-4096 | No seeded template opens the four shell types from a right-click | MG-R5 (confirmed) |
| M4 | Mission content for non-Livewire games | Only step 2307 names one; 4963 and 789 are authoring stubs | MG-R5 (confirmed) |
| M5 | Failure or defeat outcomes | No authored failure chains; D-MG5 adds the hook | MG-R5 (confirmed) |
| M6 | Social-layer design data (help, call, spectate) | Out of scope, D-MG4 | none |
| M7 | Hidden objects for "Reveal Mini-Games" (802; event set 1132; sequence 2304) | The reveal ability has nothing to reveal | MG-R7 (confirmed) |
| M8 | A source for the cheat consumables | Confirmed unobtainable in play. Item cheats are not ported (D-MG17) | MG-R7 (confirmed) |
| M9 | The original tech-competency table | Accepted: `techcomp` stays 1 (D-MG18) | MG-R7 (confirmed) |
| M10 | An offer path for mission 582 | Its giver Blackstock (NPC 948) has no spawn and dialog set 530 is unbound, so step 2307 is reachable only by GM. Follow-up after the campaign | content-map.md § 10 (G1) |

## Gates

- **G1 (architect): Done 2026-10-19.** Every wave-0 spec re-read. The port packets in [packets/wave3-5-ports.md](packets/wave3-5-ports.md) are filled (message tables, file plans, golden transcripts) and re-sliced to fit 40k working tokens (D-MG21). MG-25 and MG-23 are not applicable; MG-LW1 is needed (D-MG17). Wave 2 is re-cut in [packets/wave2-launch.md](packets/wave2-launch.md) (D-MG18-20, MG-63a/b), and wave 6 in [packets/wave6-7-content-close.md](packets/wave6-7-content-close.md) (D-MG22). No owner decision blocks any packet.

## Packets

Status words: Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done. Integrated means merged into `minigames/integration`. Wave detail and merge order: [work-packets.md § Waves](work-packets.md#waves).

| Packet | Title | Wave | Agent | Depends on | Status |
|---|---|---|---|---|---|
| MG-R0 | SWF extraction tools | 0A | game-archaeology-specialist | none | Integrated |
| MG-R6 | Livewire parity and common SFS protocol spec | 0A | minigame-systems-advisor | none | Integrated (`95756b98b`) |
| MG-R7 | Launch flow, modifiers and ability launch spec | 0A | bigworld-engine-advisor | none | Integrated (`e9644d779`) |
| MG-R1 | Alignment spec | 0B | game-archaeology-specialist | MG-R0 | Integrated (`e476cd854`) |
| MG-R2 | CrystalGame spec | 0B | game-archaeology-specialist | MG-R0 | Integrated (`b9e45cfc2`) |
| MG-R3 | GoauldCrystals spec | 0B | game-archaeology-specialist | MG-R0 | Integrated (`3dffcbab1`) |
| MG-R4 | ConverseBasicHumanoid spec | 0B | game-archaeology-specialist | MG-R0 | Integrated (`026527b36`) |
| MG-R5 | Shells, type table and content map | 0B | game-archaeology-specialist | MG-R0 | Integrated (`0fe7355ac`) |
| MG-01 | Shared `SfsValue::list` helper | 1 | packet-coder | none | Integrated |
| MG-02 | Launch params and failure chains | 1 | rust-gameserver-dev | none | Integrated (`8b15eeb0a`) |
| MG-03 | Game registry scaffold (`GameKind`) and offline session | 1 | packet-coder | none | Integrated |
| MG-04 | Client-declared hardening (minimum play time) | 1 | packet-coder | none | Integrated |
| MG-05 | Minigame REPL harness | 1 | packet-coder | MG-03 | Integrated |
| MG-06 | `BSF_PlayingMinigame` and movement lock | 1 | rust-gameserver-dev | MG-02 | Integrated (`4647624c2`) |
| MG-22 | Shared launch helper, `abilityBitfield` from the player | 2 | packet-coder | MG-02, MG-06 | Ready |
| MG-63a | Server refuses movement while a minigame is held | 2 | rust-gameserver-dev | MG-06 | Ready |
| MG-21 | Optional pre-game dialog, cell methods 24 and 30 | 2 | rust-gameserver-dev | MG-22 | BlockedDependency |
| MG-24 | `StartMinigame` effect script | 2 | rust-gameserver-dev | MG-21 | BlockedDependency |
| MG-63b | Prison Boot movement hold and release | 2 | rust-gameserver-dev | MG-63a, MG-24 | BlockedDependency |
| MG-23 | Item cheats | 2 | none | D-MG17 | Done (not applicable) |
| MG-25 | SWF PRNG port | 2 | none | D-MG17 | Done (not applicable) |
| MG-26 | Shell alias routing | 2 | none | D-MG3 = A | Done (not applicable) |
| MG-AL1 | Alignment setup and state shell | 3 | packet-coder | MG-04 | Ready |
| MG-CG1 | CrystalGame board, scramble, fullgamestate | 3 | packet-coder | MG-04 | Ready |
| MG-GC1a | GoauldCrystals table, goals, crystals | 3 | packet-coder | MG-04 | Ready |
| MG-GC1b | GoauldCrystals power search, unsolved-board loop | 3 | packet-coder | MG-GC1a | BlockedDependency |
| MG-CH1 | ConverseBasicHumanoid deal, accords, fullgamestate | 3 | packet-coder | MG-04 | Ready |
| MG-LW1 | Livewire parity fixes | 3 | packet-coder | MG-R6, MG-05 | Ready |
| MG-AL2a | Alignment geometry, matching, swap | 4 | rust-gameserver-dev | MG-AL1 | BlockedDependency |
| MG-AL2b | Alignment cascade, gravity, refill | 4 | rust-gameserver-dev | MG-AL2a | BlockedDependency |
| MG-CG2 | CrystalGame moves and reset | 4 | rust-gameserver-dev | MG-CG1 | BlockedDependency |
| MG-GC2 | GoauldCrystals clicks, path moves, swaps | 4 | rust-gameserver-dev | MG-GC1b | BlockedDependency |
| MG-CH2 | ConverseBasicHumanoid plays, NPC, hand scoring | 4 | rust-gameserver-dev | MG-CH1 | BlockedDependency |
| MG-AL3a | Alignment score, victory, timer, hints | 5 | packet-coder | MG-AL2b | BlockedDependency |
| MG-AL3b | Alignment ability cheats, goldens; ported | 5 | packet-coder | MG-AL3a, MG-22 | BlockedDependency |
| MG-CG3 | CrystalGame timer, victory, goldens; ported | 5 | packet-coder | MG-CG2 | BlockedDependency |
| MG-GC3a | GoauldCrystals timer, timeout, reset | 5 | packet-coder | MG-GC2 | BlockedDependency |
| MG-GC3b | GoauldCrystals ability cheats, goldens; ported | 5 | packet-coder | MG-GC3a, MG-22 | BlockedDependency |
| MG-CH3 | ConverseBasicHumanoid timers, deal flow, victory, goldens; ported | 5 | packet-coder | MG-CH2, MG-22 | BlockedDependency |
| MG-62 | Converse abilities 1327 and 2089 seeds | 6 | packet-coder | MG-24, MG-63b, MG-CH3 | BlockedDependency |
| MG-61 | Mission 582 step 2307, Converse with Marsh | 6 | packet-coder | MG-CH3 | BlockedDependency |
| MG-63 | Prison Boot 1597 | 6 | none | split at G1 | Replaced by MG-63a and MG-63b |
| MG-64 | Shell placeholder timer and minigame-name check | 6 | packet-coder | MG-04 | Ready |
| MG-65 | Objective 4203 upgrade | 6 | none | D-MG6 = A | Done (not applicable) |
| MG-74a | Debug Area minigame terminals: seeds and chain-replay test (D-MG23) | 6 | packet-coder | MG-61, MG-64 | BlockedDependency |
| MG-74b | Debug Area minigame terminals: docs | 6 | packet-coder | MG-74a | BlockedDependency |
| MG-71 | Cross-game TCP session test and REPL UAT index | 7 | packet-coder | waves 3-6 | BlockedDependency |
| MG-72 | Docs sweep | 7 | documentation-writer | waves 1-6 | BlockedDependency |
| MG-73 | Lab UAT recipes (ask-user-first) | 7 | documentation-writer | waves 3-6 | BlockedDependency |
| MG-99 | Close-out | 7 | documentation-writer | campaign review | BlockedDependency |
