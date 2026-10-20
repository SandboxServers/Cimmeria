# H05: minigame competency and abilities contract

Packet PU-06, advisor note. Docs only; offline; no lab, no cargo. Input: ledger rows H05, H25, H35, H36 in [README](../README.md), HA-05 and HA-09 in [work-packets.md](../work-packets.md), [owner decisions](owner-decisions-2026-10-18.md) (O18-13). Output feeds a later implementation packet and the HA-09 dispatch decision.

Labels: FACT = read in this worktree (HEAD 040ef6394) on 2026-10-19. ORIGINAL = read in `deprecated/` (the legacy C++ and Python tree). INFERENCE = follows from facts, not run. **PROVISIONAL = a design choice, labelled per O18-13 until source evidence and in-game behaviour are checked. Every section 4 to 7 design item is PROVISIONAL.** Nothing here was verified in game. No ID is allocated: where one is needed the text says `ID needed: <kind>`.

## 1. Verdict

1. FACT: the ticket mint passes constants. `start_minigame` in `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` calls `registry.register(...)` with `tech_competency = 1` (comment `TODO: read from player entity`), `seed` random, `abilities_mask = 0`, `intelligence = 0`, `player_level = 1`. The same call site receives only `entity_id`, `player_id`, `game_name`, `difficulty` and `on_victory_chains` from the cell.
2. FACT: `difficulty` is already a per-chain param (`start_minigame_difficulty.rs` pins it). Competency is the only scaling input still constant. Livewire scales the playfield, moving and obstacle counts (floored), the timer base and `read_out` off it (`games/livewire/setup.rs`, `tc = tech_competency`). The goal total is not scaled: `goal_total = level.goals` is a fixed per-difficulty constant (`setup.rs:145`).
3. FACT: `abilities_mask` reaches the SWF as `abilityBitfield` (`setup.rs:388`) and is stored on the Livewire game, but the server-side cheat commands `activateitemcheat` and `activateabilitycheat` are acknowledged stubs with no effect (`games/livewire/mod.rs:368`). So `abilities = 0` is harmless today and a non-zero mask is a client-visible change with no server rule behind it.
4. FACT: nothing in the seeded Livewire chains (1016, 1041, 1060) depends on competency above 1. The Harset content that does (HA-09, mission 1377 Replitech hacks) is not authored yet. So H05 is a fidelity gap, not a blocker for anything that ships today.
5. PROVISIONAL recommendation: close H05 with a small change that carries a caller-supplied `tech_competency` on the `start_minigame` chain action (default 1, the value every audited original call site passes; section 4), so mint stops hard-coding it. Do this before HA-09 authors a hack that promises scaling. It is not a prerequisite for HA-05 (1348 Shut Down Lethander), which has no minigame evidence in this ledger (section 7).

## 2. What the server must enforce

These hold today and must survive the change. Each has an existing guard; the implementation packet must not weaken it.

| Rule | Where (FACT) | Why it matters to H05 |
|---|---|---|
| Competency and abilities are server-minted, never client-supplied | `register()` takes them from the base; the SWF only receives them as `joinOK` vars | A client-supplied value would let a player pick the easiest board or grant itself cheats |
| Ticket is one-time and bound to game name and entity | `session.rs` authenticate-and-claim; tests `wrong_game_name_fails`, `wrong_ticket_fails`, `remove_allows_re_register` | Competency is fixed at mint; a re-used ticket must not re-roll it |
| One live session per entity | duplicate `register` returns `None` (INFO inside 2 s, WARN after) | A player cannot open two boards at different competency |
| Connection admitted only from the game connection's IP | `client_ip` check at accept | Ticket theft does not move a board |
| Result is whatever the server decided | `GameOutput::Victory` -> `MinigameResult` receipt; the cell fires `on_victory_chains` only on `result_code == 1` | The SWF cannot declare success |
| Victory receipt must match the player the cell holds | `handle_minigame_result` checks `entity.player_id == receipt player_id` once, before any chain | A reused entity slot cannot collect the previous player's reward |
| Difficulty clamp | Livewire clamps to tiers 1-4 with a `warn!` | See `difficulty-ranges` memory: the 1-5 versus 1-4 mismatch is original |

PROVISIONAL additions the implementation must make:

- **R-H05-1.** Competency is set once, at mint, from the `start_minigame` chain row (default 1). It is stored on `MinigameSession` and never re-read mid-game.
- **R-H05-2.** The resolved value is clamped to a documented range before it reaches `register()`. The Livewire tables are linear in `tc`, so an unbounded value gives an unbounded board. The range is a maintainer decision (section 8, D1); the placeholder is 1 up to the maximum the seeded tables were tuned for.
- **R-H05-3.** The abilities mask is built from abilities the player actually holds, server-side, and masked to the bits the SWF understands. A bit the server cannot honour stays 0 until a cheat rule exists (section 4.3).
- **R-H05-4.** A chain row that omits `tech_competency` uses `1`. An out-of-range value is rejected at load, as `difficulty` already is (`crates/content-engine/src/loader/action.rs:132-146`), which is an authoring error, not a runtime fallback. If a player-state resolver is ever added, it is a fallback for rows without the param, returns `1` and `0` until a source is evidenced, logs at `warn!` with `reason`, `entity_id`, `entity_name` and the fallback values, and never refuses the launch: a player must still get a board.
- **R-H05-5.** The effective competency, abilities mask and difficulty are logged at INFO on mint (fields, not format strings) so a UAT run is debuggable from SigNoz alone.

## 3. What the SmartFox session and ticket lifecycle gives

FACT, from the code and the original; the lifecycle needs no change for H05.

1. **Mint.** Cell `StartMinigame` -> base `start_minigame` -> `SessionRegistry::register` -> 64-hex one-time ticket. Base pushes `onStartMinigame(URL)`, URL `http://unused/{host}/{port}/{game}/{entityId}/{ticket}`.
2. **Connect.** The SWF opens TCP and sends the SmartFox 1.x login: `<msg t='sys'><body action='login' r='0'><login z='Livewire'><nick>ENTITYID</nick><pword>TICKET</pword></login></body></msg>`. `z` must equal the minted game name; `nick` the entity id; `pword` the ticket. The `joinOK` reply carries the game params, including `techcomp`, `abilityBitfield`, `pclevel`, `seed` and `difficulty`. These are the only channel by which competency reaches the SWF (ORIGINAL: `minigame_connection.cpp:430-434`).
3. **Run.** The game is a server state machine. The client sends actions; the server validates against the board it generated from the session.
4. **Report.** Terminal outcome -> `CellToBaseMsg::MinigameResult` -> base pushes `onEndMinigame()` and forwards `BaseToCellMsg::MinigameResult` -> cell fires chains on code 1.
5. **Expiry.** A pending (never connected) session expires after 180 s, swept every 60 s. A connected session is exempt from age expiry. ORIGINAL had no timeout at all (`session-lifecycle-original` memory), so the sweep is a substitute for two unimplemented client RPCs, not a port.
6. **Cancel.** `cancel_minigame` (player closed the window) releases the registration and reports Canceled. ORIGINAL: abort reports Canceled (0), never Defeat (2).

INFERENCE: because competency is fixed at mint and the ticket is one-time, there is no state-transfer problem during play. The "state transfer" in the ledger wording reduces to one read at mint (section 4).

## 4. Competency and abilities: what to resolve, and from where

ORIGINAL: the caller of the cell `Minigame` constructor supplies `techCompetency` (`deprecated/python/cell/Minigame.py:5-21`; section 4.2). The base `startMinigame(gameId, ticket, difficulty, techComp, seed, abilities, intelligence, level)` takes it (`deprecated/python/base/SGWPlayer.py:77`), and `Atrea.registerMinigameSession` takes it too. How the cell hands the value to the base is not traced in this note. The Rust cell-to-base message dropped all but `game_name` and `difficulty`. So the point where the value is set has two options.

### 4.1 Resolve site (PROVISIONAL, decision D2)

- **Option A (recommended): cell holds the value, message carries it.** Add `tech_competency` to `CellToBaseMsg::StartMinigame`. The chain row is cell-side, so there is no base-to-cell round trip and no stale copy. The original also kept the value on the caller side (`cell/Minigame.py`), not in the base. The other three mint constants (abilities, intelligence, player level) stay constants (sections 4.3 and 4.4). Cost: one message-shape change across `content-engine`, `cell-content`, `cell`, `base-world-entry`, plus tests.
- **Option B: base resolves.** Base holds only the connected-client snapshot. With a chain-row source there is nothing for the base to resolve, so this hop adds a message and no information. Rejected.

### 4.2 Source of competency (PROVISIONAL, decision D3; not BlockedEvidence)

FACT: a repo grep finds `tech_competency` only on crafting and item rows (`crates/cell-catalog/src/crafting/catalog.rs`) and in the minigame session. There is no player-entity property for tech competency in the code read for this note. ORIGINAL: the value is caller-supplied. Every audited `Minigame(...)` call passes the literal 1 as `techCompetency`: `deprecated/python/cell/missions/Castle_CellBlock/HackTheRings.py:32`, `Preparation.py:71`, `Prisoner_329.py:67`, `deprecated/python/cell/missions/SGC_W1/SecurityOffice.py:62`, `deprecated/python/cell/spaces/Castle.py:114`. The debug command `deprecated/python/cell/commands/Net.py:165` defaults it to 1 (its docstring says 1-55). Nothing in the original computes it from player state. The `Minigame.py` docstring still reads `??? (1)`. The `techCompetency = 0` field at `deprecated/python/Atrea/__init__.py:54` sits on a documentation stub class, so it is not evidence of runtime behaviour.

Consequence: **the value is a per-launch input on the chain row, and the original shows no other source.** Proposed (PROVISIONAL): an optional `tech_competency` param on the `start_minigame` chain action, default 1, parsed beside `difficulty` in `crates/content-engine/src/loader/action.rs:120-148`, carried by the message (4.1). Player-derived candidates (a skill level, the player level `pclevel`, an item-derived value from `minTC` or the item `tech_comp` column) have no original evidence and are not proposed. This is not BlockedEvidence: the original answers the question (caller-supplied, always 1 in the audited calls). What stays open is whether any authored board should go above 1, which is a maintainer decision (D3, section 8). If a player-derived source is ever wanted, a resolver may sit behind the param as a fallback for rows that omit it; until a source is evidenced it returns 1, the same as the default. The implementation must not invent a formula.

### 4.3 Abilities mask (PROVISIONAL)

FACT: the Livewire cheat commands are stubs on both sides. The original activation handlers `onActivateItemCheat` and `onActivateAbilityCheat` (`deprecated/python/base/minigame/Livewire.py:457-462`) only `return True`. The original cheat effects (stop time, reverse time, one step) are in `tick` (`Livewire.py:476-495`, timer states 2 and 3 and move mode 2). So there is an original effect to port but no original activation rule. Until a server rule exists, a non-zero mask would show cheat buttons that do nothing: a visible lie. Keep `abilities = 0` until a packet ports the effects. PROVISIONAL and invented: the activation checks (the server must check the mask bit, the remaining uses and the cooldown on `activateabilitycheat`, not trust the click) are this note's own design, not in the original. That port is its own packet; it is not needed for HA-09 unless a hack is authored to require a cheat.

### 4.4 Intelligence and player level

FACT: both are passed as constants (0 and 1). `player_level` is stored on the Livewire game and exported as `pclevel`. PROVISIONAL: resolve level from the player entity in the same change, since it is cheap and the cell already holds it; leave intelligence 0 unless a game reads it (no Rust game does today).

## 5. Result validation and the cell hand-off

FACT, unchanged by H05; recorded so the implementation does not regress it.

1. The game, not the client, decides the outcome. Livewire validates each move against the generated board and sets `outcome = "victory"` itself; the client's `victory` message is only honoured by the placeholder games.
2. The placeholder games (Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid; Alignment and GoauldCrystals by fallthrough) accept a single `victory` command and win instantly. **They have no competency to scale.** So competency only matters for Livewire today. Any HA-09 "hack" step that is meant to be a real challenge must use `Livewire` (interaction bit 256); there is no `INT_MinigameHack` cursor bit (`docs/content/interaction-flags.md`).
3. Hand-off: result code 1 is the only code that fires `on_victory_chains`. Codes: 0 Canceled, 1 Victory, 2 Defeat, 3 NotStarted (ORIGINAL constants; the wire names are in `minigame_result_name`).
4. PROVISIONAL test requirements for the implementation packet (each must fail when the change is reverted, except the guard in the last bullet):
   - a base-side test: a `StartMinigame` carrying competency 3 registers a session with `tech_competency == 3` (fails on the current constant 1);
   - a default test: a `start_minigame` row without `tech_competency` registers competency 1, and a loader test reads the param beside `difficulty` (the `action_conversion.rs` pattern);
   - a clamp test: a value above the range registers at the cap;
   - a Livewire test: at difficulty 4 with the same seed, competency 1 and competency 5 differ in `read_out` (`E-1` and `E-5`, `setup.rs:155`), in the floored timer base (`param_timer`, 12 and 13, `setup.rs:144`, observable through `time_units` only when the timer type is 0, `setup.rs:160`) and in the count of obstacle wires `o1`..`oN` (3 and 4, `setup.rs:213`, `setup.rs:324`). Difficulty 4 is used because it crosses floors at tc 1 and 5 (timer, moving 5 and 6, obstacles). Goal total is not a witness: it is fixed per difficulty (`setup.rs:145`). Playfield does not cross a floor there (6 and 6), so do not use it;
   - a guard, not a revert-proof test: `abilities_mask` stays 0 until section 4.3 lands. It passes with or without the plumbing, because the mask is 0 either way, so it pins a constant rather than the change.

## 6. Stays BlockedEvidence

H05 itself is a contract. These stay blocked, and this note does not unblock them.

| Item | Why blocked | What would unblock |
|---|---|---|
| HA-05 (1348 Shut Down Lethander) Market waves | Private waves, counts, timing, cleanup not evidenced (ledger H25). Also gated on the Market exit arch (1006 versus 1551) and spawn 308 / raised-point re-measure. No minigame requirement is evidenced for 1348 | HA-01a client look at the Market exit; HA-02 follow-ups; a mission-text or client source for the wave design |
| HA-09 (1377 Replitech) device and crates anchors | No Harset graph for 1377; the device/crate locations are unplaced (placements.md lists no Replitech anchor) | Placement evidence, then a graph; the note in section 7 for the minigame step |
| HA-09 (1374 Security Holes) | Haughty/Angry/Bar placement and scan guards (H36); item 4396 scanner is PROVISIONAL (HA-15e) | Door/stall relation from HA-01a; four scan anchors |
| HA-01a client look | Whether the real client lets a player through the candidate Market arches, and the use-on-target question (uat.md B2) are owner/client observations, not offline facts | A client session; this packet cannot supply it |
| Abilities and cheats (4.3) | Server cheat rules are unported | A separate port packet |
| Whether the SWF honours `techcomp` for non-Livewire games | The placeholder SWFs have only a win button (`livewire-client-swf-facts` memory) | Not needed unless those games are ported |

## 7. Design the HA-09 author may assume (PROVISIONAL)

- A Replitech hack step is a `start_minigame` action with `target_key = 'Livewire'`, `params {"difficulty": N, "on_victory_chains": [...]}`. Choose `difficulty` explicitly (1-4; 5 clamps to 4). Do not promise level-scaled difficulty to the player until section 4 lands; at present every board is competency 1 (the default, and what every audited original call passed).
- The victory chain, not the minigame, advances the mission step. Pair every launch chain with a login-restore chain that re-sets the `INT_MinigameLivewire` (256) bit, or the cursor icon disappears on relog (`chain-wiring-and-gaps` memory; Castle precedents 1045/1046/1065).
- A Defeat or Canceled result fires nothing, so the object stays interactive and the player may retry. That is the intended behaviour; do not author a failure chain unless a design needs one (a failure hook does not exist: only code 1 routes).
- Search-style steps that do not need a board should avoid the minigame entirely (`search-without-minigame` memory).

## 8. Maintainer decisions

| Id | Question | Recommendation (PROVISIONAL) |
|---|---|---|
| D1 | Competency range and cap | Cap at the top of the range the seeded Livewire tables were tuned for; confirm the number from the Python `difficultyLevels` tables before the packet |
| D2 | Resolve site (4.1) | Option A, cell holds the value and the message carries it |
| D3 | Competency per launch (4.2) | Add an optional `tech_competency` param to the `start_minigame` chain action, default 1, which every audited original call passed. Whether any authored board goes above 1 is the maintainer's call; this is not blocked evidence |
| D4 | Ship the plumbing before HA-09 or with it | Before: the shape change touches the same minigame files as FZ-03c and FZ-08 (ASSUMED, not checked against their file lists; from `pre-uat-offline-work.md:26`), so it waits for wave 3 of the four-zone campaign |
| D5 | Port the Livewire cheat rules | Defer; no Harset requirement |

## 9. Sequencing

1. Nothing here blocks the docs-only ledger refresh (PU-09).
2. The implementation packet (chain param, message field, mint change, and the tests in 5.4) is small. It edits `cell_dispatch/minigame.rs`, the `StartMinigame` message, `executor`, and the chain loader `crates/content-engine/src/loader/action.rs`. ASSUMED, not checked: that FZ-03c and FZ-08 also touch these files. If they do, dispatch it after four-zone wave 3.
3. HA-05 and HA-09 stay BlockedEvidence regardless (section 6). H05 can be marked "contract written, implementation pending D1-D4" in the ledger by PU-09.

## 10. Sources

- `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` (mint constants).
- `crates/minigame/src/minigame/session.rs`, `session_claim.rs`, `server/` (ticket, claim, TTL, cancel, result dispatch).
- `crates/minigame/src/minigame/games/livewire/{mod,setup}.rs`, `games/mod.rs`, `games/placeholder.rs`.
- `crates/cell/src/cell/service/base_messages/minigame.rs` (result routing and player-match guard).
- `crates/cell-content/src/cell/content/chain_replay_tests/start_minigame_difficulty.rs`.
- `crates/content-engine/src/loader/action.rs:120-148` (`start_minigame` params, `difficulty` range).
- ORIGINAL: `deprecated/python/base/SGWPlayer.py:77-103`, `deprecated/python/cell/Minigame.py:5-21`, `deprecated/python/cell/SGWPlayer.py:1903-1920`, `deprecated/python/cell/missions/Castle_CellBlock/HackTheRings.py:32`, `Preparation.py:71`, `Prisoner_329.py:67`, `deprecated/python/cell/missions/SGC_W1/SecurityOffice.py:62`, `deprecated/python/cell/spaces/Castle.py:114`, `deprecated/python/cell/commands/Net.py:165-182`, `deprecated/python/Atrea/__init__.py:54,295`, `deprecated/python/base/minigame/Livewire.py:457-510`, `deprecated/cpp/src/baseapp/minigame_connection.cpp:430-434`.
- Project memory: `.claude/agent-memory/minigame-systems-advisor/` (`chain-wiring-and-gaps`, `difficulty-ranges`, `session-lifecycle-original`, `game-implementation-status`).
