# Minigames campaign

> Type: explanation and ledger. Audience: the orchestrator, the architect, packet agents, the owner.
> Written 2026-10-19 by the architect against `main` @ `0d71b3b30`. Companions: [work packets](work-packets.md), [resume card](RESUME.md), [minigame system](../../gameplay/minigame-system.md), [client binary findings](../../reverse-engineering/findings/minigame-architecture.md), [wire formats](../../reverse-engineering/findings/minigame-wire-formats.md), [CAT-K audit](../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md).

## Purpose

Implement every Stargate Worlds minigame the client has content for, as a real server-validated game wherever the client content allows it. Where the content does not allow it, say so per game and harden what is left. Launch, result, failure and modifier plumbing, plus the content that starts the games, are in scope. The social layer (call a helper, spectate, contact) is not (D-MG4).

The campaign starts with a research wave. Every port packet consumes a per-game spec written by that wave, so the port packets in waves 3 to 5 are skeletons until gate G1 (§ Gates) fills them in.

## What the client has

The 11 packages under the client's `CookedPC\UI\Flash\` (the user's own client copy; nothing from it goes in git, D-MG10) each wrap one uncompressed Scaleform GFX movie. The architect scanned them on 2026-10-19 with throwaway scripts. MG-R0 turns that into a repo tool, and the W0 research confirms every row below.

| Package | GFX movie | What the movie holds | Provisional verdict | Spec (packet) |
|---|---|---|---|---|
| `Livewire.upk` | 410,721 B | Full game and an embedded server extension | **Recovered.** Server-validated today | [spec-livewire-parity.md](spec-livewire-parity.md) (MG-R6) |
| `Alignment.upk` | 293,868 B | Embedded AS1 SmartFox server extension: a hex match-3 with cascades, hints, goal scores, item and ability cheats, `seed`/`rng` | **Recoverable** from the extension | [spec-alignment.md](spec-alignment.md) (MG-R1) |
| `CrystalGame.upk` | 94,203 B | Embedded extension: `processMove`, `checkForVictory`, `Crystal` | **Recoverable** | [spec-crystalgame.md](spec-crystalgame.md) (MG-R2) |
| `GoauldCrystals.upk` | 154,559 B | Embedded `handleRequest`, `Swap`, `Crystal`; no `checkForVictory` string found yet | **Likely recoverable** | [spec-goauldcrystals.md](spec-goauldcrystals.md) (MG-R3) |
| `ConverseBasicHumanoid.upk` | 227,957 B | Embedded extension: `Card`, `Suit`, `Match`, `Swap`, `checkForVictory` (a card game; the "Trump" passives 778/779/792/793 point at it) | **Recoverable** | [spec-conversebasichumanoid.md](spec-conversebasichumanoid.md) (MG-R4) |
| `Hack.upk`, `Activate.upk`, `Analyze.upk`, `Bypass.upk`, `Converse.upk` | about 55.8 kB each, near-identical | **Shells**: a SmartFoxClient, a trivial extension (`handleRequest`, `sendResponse`), a countdown timer, `fullgamestate`, `victory`/`failure` and a `win_btn` that sends `victory`. No rules | **Not recoverable as games.** Client-declared, hardened (MG-04); D-MG3 decides the content | [spec-shells.md](spec-shells.md) (MG-R5) |
| `DHD.upk` | 93,455 B | The dialling UI (`GateTravel`) | Not a minigame. Out of scope | none |

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Five of the ten client game types are shells with no rules: Hack, Activate, Analyze, Bypass and Converse. Their only win path is a button that sends `victory`. | GFX scan above; `Placeholder Activate/Analyze/Bypass` display names in the type table at `0x019d917c` | MG-R5, MG-04, D-MG3 |
| F2 | Alignment, CrystalGame, GoauldCrystals and ConverseBasicHumanoid embed their original SmartFox server extension (AS1) in the SWF. That code is the rules source. | Function names in each GFX: `processmove`, `swapPieces`, `doCascade`, `checkForVictory`, `getHint`, `activateitemcheat`, `Card`, `Suit` | MG-R1..R4, D-MG1 |
| F3 | `deprecated/python/base/minigame/Alignment.py` and `GoauldCrystals.py` are unfinished half-ports. Their difficulty tables look real; their message handlers are copy-pasted Livewire (`opendoor`, wire names). | Read 2026-10-19 | D-MG1 |
| F4 | `minigame-architecture.md` is wrong twice. Line 34 says the placeholders have no SWF (they have shells), and lines 44-57 class Hack and Converse as "other" and CBH as a sub-type, though CBH is a full game. | This scan | MG-72 |
| F5 | Livewire is server-validated and works. The Rust matches the Python on the wire-variant range (`randrange(1, 4)`, so variants 1-3) and on the bare library names (`pGray`, `mYellowStripe`). Nobody has checked either against the SWF library. | `games/livewire/setup.rs:102,261,327`; `Livewire.py:133-135,539` | MG-R6, MG-LW1 |
| F6 | The factory sends every non-Livewire name to the placeholder. `games::validation()` returns a label string that four `session_loop.rs` log rows use. The commented Alignment and GoauldCrystals arms follow the Python. | `crates/minigame/src/minigame/games/mod.rs` | MG-03 |
| F7 | Base hard-codes `tech_competency = 1`, `abilities_mask = 0`, `intelligence = 0` and `player_level = 1` when it registers a session. The SWFs read `techcomp`, `abilityBitfield`, `intelligence`, `pclevel`, `CA0`-`CA4` and `instcc` from `joinOK`. | `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` `start_minigame` | MG-02, MG-22, MG-23 |
| F8 | There is no failure path. The cell fires chains only on result 1, and `Action::StartMinigame` carries only `on_victory_chains`. | `crates/cell/src/cell/service/base_messages/minigame.rs:35` | MG-02, D-MG5 |
| F9 | The original launch put up a pre-game dialog (`onStartMinigameDialog`); the client then called cell method 24 `startMinigame`, or 30 `minigameStartCancel`. Cimmeria skips the dialog, and both methods log UNIMPLEMENTED. | `deprecated/python/cell/SGWPlayer.py:1903-1990`; `crates/cell-methods/src/cell/cell_methods/minigame.rs:70,133` | MG-R7, MG-21, D-MG11 |
| F10 | `BSF_PlayingMinigame` (bit 4) is never set, and the player can walk away mid-game. The original set the flag and added a movement lock on launch, and cleared both on the result. `cimmeria_wire::state_field` has no `BSF_PLAYING_MINIGAME` constant. | `SGWPlayer.py` `requestStartMinigame`/`handleMinigameResults`; `crates/wire/src/state_field.rs:81` | MG-06, D-MG15 |
| F11 | Ability-started games are not wired. Ability 1327 "Converse Minigame" (effect 1643 "Force Converse Minigame on Player"), 2089 "Conversation: Humanoid: Basic" (effect 2813) and 1597 "Prison Boot" (effect 1939, "You cannot move until it is disabled") all have `effects.script_name` NULL. The content engine's `ability_used` trigger is defined but never emitted. | `db/resources/Effects/Seed/effects.sql`; `crates/content-engine/src/triggers/mod.rs:44,261` | MG-R7, MG-24, MG-62, MG-63, D-MG8 |
| F12 | Seeded content uses only Livewire (10 `start_minigame` rows). Mission 582 step 2307 "Conversation Minigame With Marsh" is `step_enabled = false`, index 0 (Harset, "A Question of Loyalty"). Objective 4203 already runs on Harset's interact-grant-destroy route (D-H13, chain 1032). Objectives 4963 ("add button to dialog for minigame") and 789 ("TODO: Minigame") are authoring stubs. | `mission_steps.sql:5335`; Harset ledger | MG-R5, MG-61, MG-64, MG-65, D-MG6, D-MG7 |
| F13 | Modifier content exists and nothing reads it. It covers the archaeologist minigame abilities (677-686, 802 "Reveal Mini-Games", 809, 813-815, 1195, 1552, 2507, 2508), the "Trump" passives (778, 779, 792, 793) and about 300 consumable item rows ("Archaeologist Minigame Consumable TC3..15"). It also covers the win container (1661, 2837), despawn mob (585), consume item (586), the success dialogs (1594, 1753) and event set 1132 "Minigame reveal target". | Ability, item and event seeds | MG-R7, MG-22, MG-23 |
| F14 | The interaction bits 512 (Activate), 1024 (Analyze), 2048 (Bypass) and 4096 (Converse) exist; there is no Hack bit. No seeded template uses them. | [interaction-flags.md](../../content/interaction-flags.md) | MG-R5 |
| F15 | No repo tool extracts the GFX movies. The cellblock-autoplay spike has an AS2 disassembler (`docs/analysis/cellblock-autoplay/livewire-spike/as2dis.py`). | [tools catalog](../../../tools/README.md) | MG-R0 |
| F16 | `SfsValue` has no list form. Livewire builds arrays as numeric-key objects in a private `array_to_obj` (`livewire/setup.rs:378`), and every new game needs the same helper. | `protocol.rs`, `setup.rs` | MG-01 |
| F17 | Several minigame files are at or near the 700-line hard cap: `server/tests.rs` 680, `server/listener_tests.rs` 679, `session_tests.rs` 664, `server/session_loop.rs` 638, `session.rs` 589, `protocol.rs` 549. New code and tests go in new files. | line counts 2026-10-19 | contract C2 |
| F18 | The social layer (cell methods 26-29 and 31-34: spectate, help registration, call accept/decline/abort, contact request) is still stubs. | `cell_methods/minigame.rs` | D-MG4 |
| F19 | CAT-K-01, the placeholder's instant client-declared victory, is still open for the shells. | CAT-K-minigame.md | MG-04, D-MG13 |

## Decisions

| Id | Decision | Status | Reason |
|---|---|---|---|
| D-MG1 | The rules source is the AS1 server extension embedded in each SWF. The Python half-ports are a cross-check only, and the client binary is the source for the launch flow. | Decided | F2, F3: the extension is the code the original server ran; the Python never worked |
| D-MG2 | Every recovered game is server-validated. The server generates the board, validates every move, scores, runs the timer and decides the result. A client `victory` for a server-validated game is refused with a WARN. The server sends what the SWF expects, so nothing needs a client patch. | Decided | Server-authoritative rule; CAT-K-01 |
| D-MG3 | The five shell games (Hack, Activate, Analyze, Bypass, Converse). **A (recommended):** keep them client-declared and hardened (MG-04); wire content to them only where a mission names them (MG-64). **B:** alias a shell name to a real game on the server (for example Bypass to Alignment, if MG-R5 confirms the hex "bypass" mode). No client patch is needed, but the player sees a different game. **C:** author new rules. That needs SWF changes, so it is out of scope. | **BlockedDecision** (owner) | A content and design call, not a technical one |
| D-MG4 | The social layer (help, call, spectate, contacts) is out of scope and goes to a follow-up campaign. | Decided (owner may overrule) | No design data beyond the client UI; it does not block any game |
| D-MG5 | `start_minigame` gains an `on_failure_chains` param. Those chains fire on result 2 (Failure) and 4 (Defeated). Results 0 (canceled) and 3 (interrupted or not started) fire nothing. | Decided | F8; the client's result strings sit at `0x00c7d0b0` |
| D-MG6 | Objective 4203 (mission 1244 step 3619, "Placeholder minigame."). **A (recommended):** keep Harset's D-H13 interact route. **B:** upgrade it to a real minigame once one is ported (MG-65). | **BlockedDecision** (owner) | Changes a shipped Harset flow |
| D-MG7 | Mission 582 step 2307 is enabled only once the game it names is server-validated (MG-R5 says which; ConverseBasicHumanoid is expected). If MG-R5 finds no NPC or dialog hook for it, the architect re-rules at G1. Seeds only. | Decided | The brief's content-wiring goal; no client patch |
| D-MG8 | Abilities start minigames through a new effect script, `StartMinigame`, registered in `EFFECT_SCRIPTS` and set on the effect's `script_name` in the seed. The `ability_used` content trigger is not used. | Decided (owner may overrule) | The effects already name the action (1643, 2813), the effect has the caster and the target, and range and cooldown checks run first. Emitting `ability_used` for every ability is a wider change |
| D-MG9 | Difficulty tiers come from each extension's own table. Content may say 1-5; a game with fewer tiers clamps to its highest. | Decided | Today's clamp is Livewire-only |
| D-MG10 | No client assets and no full disassembly go in git. The tools extract from the user's own client into a scratch folder. Specs carry pseudocode, tables and short excerpts of at most 20 lines each. | Decided | Repo invariant: the client is not in git |
| D-MG11 | MG-21 ships the original pre-game dialog behind a server switch, default **off**. Turning it on changes what players see before every minigame (existing client UI, no patch). | **BlockedDecision** (owner: default on after lab UAT?) | Visible behaviour change |
| D-MG12 | The server owns randomness. The SWF PRNG is ported (MG-25) only if MG-R6 shows the client derives anything from `seed`. | Decided | Do not port code nobody needs |
| D-MG13 | Client-declared games (the shells) need at least 5 s of play. An earlier `victory` is refused with a WARN and the session stays open. One result per session (today's idempotency). | Decided | CAT-K-01; the shells' own timer runs longer than 5 s |
| D-MG14 | Item cheats (`CA0`-`CA4`, `instcc`, `consUsed`) are in scope if MG-R7 maps them to item ids. The server consumes the item through base inventory, never the client. | Decided | F13; server-authoritative |
| D-MG15 | `BSF_PlayingMinigame` and a counted movement lock are held from launch to the result, as the original did. Every end path clears them: result, cancel, launch failure, TTL sweep, logout. | Decided | F10 |
| D-MG16 | Every game gets a REPL transcript UAT (MG-05 harness), checked in and run as a test. Lab UAT is **ask-user-first**: no packet starts the client, and the orchestrator asks before any lab-driver run. | Decided | Repo rule and user memory |

## Missing content register

What the client never shipped, or the seeds never authored. Each row names the packet that confirms it.

| # | Missing | Effect | Confirmed by |
|---|---|---|---|
| M1 | Rules for Hack, Activate, Analyze, Bypass and Converse | They cannot be server-validated; D-MG3 | MG-R5 |
| M2 | Which NPCs and dialogs use Converse rather than ConverseBasicHumanoid | Converse content wiring stays limited to what the seeds name | MG-R5 |
| M3 | Placed world objects with interaction bits 512-4096 | No seeded template opens the four shell types from a right-click | MG-R5 |
| M4 | Mission content for non-Livewire games | Only step 2307 names one; 4963 and 789 are authoring stubs | MG-R5 |
| M5 | Failure or defeat outcomes | No authored failure chains; D-MG5 adds the hook and content may use it | MG-R5 |
| M6 | Social-layer design data (help, call, spectate) | Out of scope, D-MG4 | none |
| M7 | Hidden objects for "Reveal Mini-Games" (802; event set 1132; sequence 2304) | The reveal ability has nothing to reveal | MG-R7 |
| M8 | A source for the cheat consumables (are the blueprints craftable or dropped?) | Item cheats may be unreachable in play | MG-R7 |
| M9 | The original tech-competency table (TC to difficulty or bonus) | `techcomp` may stay a guess | MG-R7 |

## Gates

- **G1 (architect, after wave 0B merges).** Re-read every spec and update this ledger. The architect also:
  - fills the port packet sections in [packets/wave3-5-ports.md](packets/wave3-5-ports.md) (message tables, file plan, golden transcripts);
  - rules whether MG-25 and MG-LW1 are needed;
  - confirms the D-MG7 target game, the MG-63 Prison Boot design and the MG-23 item map;
  - re-sizes any port that does not fit 40k working tokens.

  No port packet dispatches before G1.

## Packets

Status words: Ready, BlockedDependency, BlockedDecision, Writing, Review, Integrated, UATPending, Done. Wave detail, PR shape and merge order: [work-packets.md § Waves](work-packets.md#waves).

| Packet | Title | Wave | Agent | Depends on | Status |
|---|---|---|---|---|---|
| MG-R0 | SWF extraction tools | 0A | game-archaeology-specialist | none | Ready |
| MG-R6 | Livewire parity and common SFS protocol spec | 0A | minigame-systems-advisor | none | Ready |
| MG-R7 | Launch flow, modifiers and ability launch spec | 0A | bigworld-engine-advisor | none | Ready |
| MG-R1 | Alignment spec | 0B | game-archaeology-specialist | MG-R0 | BlockedDependency |
| MG-R2 | CrystalGame spec | 0B | game-archaeology-specialist | MG-R0 | BlockedDependency |
| MG-R3 | GoauldCrystals spec | 0B | game-archaeology-specialist | MG-R0 | BlockedDependency |
| MG-R4 | ConverseBasicHumanoid spec | 0B | game-archaeology-specialist | MG-R0 | BlockedDependency |
| MG-R5 | Shells, type table and content map | 0B | game-archaeology-specialist | MG-R0 | BlockedDependency |
| MG-01 | Shared `SfsValue::list` helper | 1 | packet-coder | none | Ready |
| MG-02 | Launch params and failure chains | 1 | rust-gameserver-dev | none | Ready |
| MG-03 | Game registry scaffold (`GameKind`) and offline session | 1 | packet-coder | none | Ready |
| MG-04 | Client-declared hardening (minimum play time) | 1 | packet-coder | none | Ready |
| MG-05 | Minigame REPL harness | 1 | packet-coder | MG-03 | BlockedDependency |
| MG-06 | `BSF_PlayingMinigame` and movement lock | 1 | rust-gameserver-dev | MG-02 | BlockedDependency |
| MG-21 | Pre-game dialog, `startMinigame` (24), `minigameStartCancel` (30) | 2 | rust-gameserver-dev | MG-R7, MG-06, G1 | BlockedDependency |
| MG-22 | `techcomp` and `abilityBitfield` from the player | 2 | packet-coder | MG-R7, MG-02, G1 | BlockedDependency |
| MG-23 | Item cheats `CA0`-`CA4`, `instcc`, `consUsed` | 2 | rust-gameserver-dev | MG-R7, MG-22, G1 | BlockedDependency |
| MG-24 | `StartMinigame` effect script | 2 | rust-gameserver-dev | MG-R7, MG-02, G1 | BlockedDependency |
| MG-25 | SWF PRNG port (only if G1 says so) | 2 | packet-coder | MG-R6, G1 | BlockedDependency |
| MG-26 | Shell alias routing (only under D-MG3 B) | 2 | packet-coder | MG-03, D-MG3 | BlockedDecision |
| MG-AL1 | Alignment board and setup | 3 | packet-coder | MG-R1, MG-01, MG-03, G1 | BlockedDependency |
| MG-CG1 | CrystalGame board and setup | 3 | packet-coder | MG-R2, MG-01, MG-03, G1 | BlockedDependency |
| MG-GC1 | GoauldCrystals board and setup | 3 | packet-coder | MG-R3, MG-01, MG-03, G1 | BlockedDependency |
| MG-CH1 | ConverseBasicHumanoid deal and setup | 3 | packet-coder | MG-R4, MG-01, MG-03, G1 | BlockedDependency |
| MG-LW1 | Livewire parity fixes (only if G1 says so) | 3 | packet-coder | MG-R6, G1 | BlockedDependency |
| MG-AL2 | Alignment moves, cascades, scoring | 4 | rust-gameserver-dev | MG-AL1 | BlockedDependency |
| MG-CG2 | CrystalGame moves and scoring | 4 | rust-gameserver-dev | MG-CG1 | BlockedDependency |
| MG-GC2 | GoauldCrystals moves and scoring | 4 | rust-gameserver-dev | MG-GC1 | BlockedDependency |
| MG-CH2 | ConverseBasicHumanoid plays and scoring | 4 | rust-gameserver-dev | MG-CH1 | BlockedDependency |
| MG-AL3 | Alignment victory, timer, hints, cheats; ported | 5 | packet-coder | MG-AL2, MG-05, MG-22 | BlockedDependency |
| MG-CG3 | CrystalGame victory, timer, cheats; ported | 5 | packet-coder | MG-CG2, MG-05, MG-22 | BlockedDependency |
| MG-GC3 | GoauldCrystals victory, timer, cheats; ported | 5 | packet-coder | MG-GC2, MG-05, MG-22 | BlockedDependency |
| MG-CH3 | ConverseBasicHumanoid victory, timer, cheats; ported | 5 | packet-coder | MG-CH2, MG-05, MG-22 | BlockedDependency |
| MG-61 | Mission 582 step 2307, Converse with Marsh | 6 | packet-coder | MG-R5, MG-CH3 (or the G1 target) | BlockedDependency |
| MG-62 | Converse abilities 1327 and 2089 seeds | 6 | packet-coder | MG-24, MG-CH3 | BlockedDependency |
| MG-63 | Prison Boot 1597 | 6 | rust-gameserver-dev | MG-R7, MG-24, G1 | BlockedDependency |
| MG-64 | Shell-game content per D-MG3 (objectives 4963, 789, templates) | 6 | packet-coder | D-MG3, MG-R5 | BlockedDecision |
| MG-65 | Objective 4203 upgrade (only under D-MG6 B) | 6 | packet-coder | D-MG6 | BlockedDecision |
| MG-71 | Cross-game TCP session test and REPL UAT index | 7 | packet-coder | waves 3-6 | BlockedDependency |
| MG-72 | Docs sweep | 7 | documentation-writer | waves 1-6 | BlockedDependency |
| MG-73 | Lab UAT recipes (ask-user-first) | 7 | documentation-writer | waves 3-6 | BlockedDependency |
| MG-99 | Close-out | 7 | documentation-writer | campaign review | BlockedDependency |
