# Minigames work packets

> Type: how-to. Audience: the orchestrator, ops, packet-coder and domain agents, the packet-reviewer.
> Written 2026-10-19 by the architect against `main` @ `0d71b3b30`, to the [campaign-packet skill](../../../.claude/skills/campaign-packet/SKILL.md). Companions: [ledger, findings and decisions](README.md), [resume card](RESUME.md), [testing playbook](../../../TESTING.md).

Packet status lives in **one place**: the [README packet table](README.md#packets). This file holds the dispatch rules, the contract, the wave plan, and the packet text for waves 0 and 1. The later waves live in [packets/](packets/):

- [wave 2: launch and modifiers](packets/wave2-launch.md);
- [waves 3-5: per-game ports](packets/wave3-5-ports.md), filled in at gate G1 (D-MG21);
- [waves 6-7: content, UAT and close-out](packets/wave6-7-content-close.md).

## Dispatch rules

1. **Peer check first.** Before any dispatch, ops reports `git log --since="1 hour ago"` on `minigames/*` branches, the board's `minigames` subcategory, and any `.claude/worktrees/mg-*` folder. A live peer means stand down.
2. **One worktree per packet.** Ops runs `pwsh tools/build-lane/mk-worktree.ps1 minigames/<packet-lower>-<slug> <worktree>`; each packet header gives the branch and the worktree name. The test database is `sgw_<worktree>`. Never put two workers in one checkout. A stacked packet passes `--base <branch>` to its PR.
3. **Roles.** `packet-coder` (Haiku) takes the fully specified code, seed and test packets. Research, RE and design-heavy packets go to the named domain agent with the same brief shape. Reviews run per the skill § 4: a fresh read-only `packet-reviewer` per pass, fix notes to a fresh `packet-coder`, at most 3 passes. Add `server-authority-enforcer` as a parallel read-only pass on MG-22, MG-24, MG-63a and every `<G>2` and `<G>3` port slice; each packet header names any other advisor pass.
4. **Brief** (under 25 lines): the worktree path, this file's packet section path (the agent reads it), the report path, the commit attribution lines, and the return line. Never paste the packet into the brief.
5. **Report path:** `<worktree>\.claude\reports\minigames\<packet>-<role>.md`, where role is `coder`, `review-<N>` or `fix-<N>`; for example `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\mg-01\.claude\reports\minigames\MG-01-coder.md`. Never the root checkout. Copy anything worth keeping to `docs/analysis/minigames/worknotes/` before the worktree is retired.
6. **Return**, exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
7. **Escalation** follows the skill. A context stop leaves `worknotes/<packet>.md`, and a fresh coder continues (at most 3 sessions). Next comes one Sonnet implementer (`rust-gameserver-dev` for code and seeds, `documentation-writer` for docs), then the architect re-slices. A `decision` verdict, or `fix` after pass 3, goes to the architect for a ruling note.
8. **Owner decisions.** D-MG3, D-MG6 and D-MG11 were decided at G1 under the owner's autonomy directive (README § Decisions). A new owner call is `BlockedDecision` in the README; no packet guesses an answer.
9. **Lab use is ask-user-first** (D-MG16). No packet starts the client, the launcher or a lab session. MG-73 writes the lab recipes; the orchestrator asks the user before any `lab-driver` run.
10. **Gate G1 is done** (2026-10-19, [README § Gates](README.md#gates)). Waves 2-7 are prescriptive; a packet whose text still looks open stops and returns `blocked`.
11. **Integration branch.** Every packet from wave 2 on branches from `minigames/integration` (worktree `mg-integration`). Ops merges it back with `merge-noff.ps1` once its review loop says `ship`, then retires the packet worktree and its target dir. A chained packet's worktree is made only after its predecessor has merged. The campaign ships one PR at the end (owner directive).

## Waves

| Wave | Packets | Can start | Ships as | Merge order |
|---|---|---|---|---|
| 0A research | MG-R0, MG-R6, MG-R7 | now | one PR from integration worktree `mg-wave0a` (branch `minigames/wave0a`) | R0, R6, R7 |
| 0B research | MG-R1, MG-R2, MG-R3, MG-R4, MG-R5 | after 0A merges | one PR from integration worktree `mg-wave0b` (branch `minigames/wave0b`) | R5, R1, R2, R3, R4 |
| G1 | architect gate | after 0B merges | ledger and packet-text PR by the architect's worktree | none |
| 1 foundations | MG-01, MG-02, MG-03, MG-04, MG-05, MG-06 | now (in parallel with 0A) | one PR per packet; MG-05 stacks on MG-03 and MG-06 on MG-02 | MG-01, MG-03, MG-04, MG-05, MG-02, MG-06 |
| 2 launch | MG-22, MG-63a, MG-21, MG-24, MG-63b (MG-23, 25, 26 not applicable) | now: MG-22 and MG-63a; the rest chained | merged into `minigames/integration` | MG-22, MG-63a, MG-21, MG-24, MG-63b |
| 3 ports, setup | MG-AL1, MG-CG1, MG-GC1a, MG-GC1b, MG-CH1, MG-LW1 | now (GC1b after GC1a) | merged into `minigames/integration` (disjoint folders) | any; GC1a before GC1b |
| 4 ports, moves | MG-AL2a, MG-AL2b, MG-CG2, MG-GC2, MG-CH2 | each after its predecessor slice | merged into `minigames/integration` | chain order within a game |
| 5 ports, rules | MG-AL3a, MG-AL3b, MG-CG3, MG-GC3a, MG-GC3b, MG-CH3 | each after its predecessor slice; AL3b, GC3b and CH3 also after MG-22 | merged into `minigames/integration` | chain order within a game |
| 6 content | MG-62, MG-61, MG-64 (MG-63 replaced, MG-65 not applicable) | MG-64 now; the rest per row dependencies | merged into `minigames/integration` | MG-62, MG-61, MG-64 |
| 7 close | MG-71, MG-73, MG-72, campaign review, MG-99 | after wave 6 | merged into `minigames/integration`, then one campaign PR | MG-71, MG-73, MG-72, review fixes, MG-99 |

Waves 0 and 1 dispatched together and are integrated. Waves 3 to 5 run as four per-game chains plus MG-LW1; a game's next slice can start once its predecessor merges, without waiting for the other games. Wave 3 may run beside wave 2: the ports touch only their own game folders.

## Contract

Every packet builds against this section. A packet that cannot honour it stops and returns `blocked`.

### C1. Rules every packet carries

These are repeated in short form in each packet, because packet-coder does not read CLAUDE.md.

1. **Shell.** PowerShell only, never Bash. Start every command with `Set-Location <worktree>;`, and use absolute paths with `[IO.File]`.
2. **Cargo through the lane.** Every compiling cargo call goes through `pwsh tools/build-lane/lane.ps1`; `cargo fmt` may run directly. Run lane checks in the foreground. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`.
3. **Scope.** Edit only the files the packet names. If another file must change, stop and return `blocked` with the reason.
4. **File caps.** 500 lines soft, 700 hard. New code and tests go in new files when the target is already near the cap (README F17).
5. **Tests.** Every behaviour change has a test that fails when the change is reverted. Say in the report why it fails.
6. **Docs.** Files under `docs/` are CRLF. Never hand-edit a `<!-- gen:NAME -->` block; run `python tools/docs-gen/regen.py --skip-crate-graph` instead.
7. **Git.** No `git stash`, `git worktree prune`, push or PR. Commit once, locally, with the packet's subject and the brief's attribution lines.
8. **Seeds.** Seeds are the source of truth: edit `db/resources/`. Never add `db/scripts/*.sql` migrations.
9. **No lab.** Never start the client, the launcher or a lab tool.

### C2. Lanes per crate

| Crate dir | Package | Notes |
|---|---|---|
| `crates/minigame` | `cimmeria-minigame` | `tokio` test-util is on (`start_paused`, `advance`) |
| `crates/content-engine` | `cimmeria-content-engine` | |
| `crates/cell-content` | `cimmeria-cell-content` | chain-replay tests live here |
| `crates/cell` | `cimmeria-cell` | |
| `crates/cell-methods` | `cimmeria-cell-methods` | |
| `crates/cell-effect-scripts` | `cimmeria-cell-effect-scripts` | |
| `crates/base-world-entry` | `cimmeria-base-world-entry` | |
| `crates/wire` | `cimmeria-wire` | |
| `crates/cell-world` | `cimmeria-cell-world` | `EffectContext`, `SpaceManager`, `minigame_launch` (MG-22) |
| `crates/entity` | `cimmeria-entity` | `minigame_feature`, the stat-buff ledger (MG-21, MG-63b) |
| `crates/cell-combat` | `cimmeria-cell-combat` | the effect-queue drain (MG-24) |
| `crates/server` | `cimmeria-server` | the env switch init (MG-21) |

The check order for each crate the packet touches:

```powershell
Set-Location <wt>; cargo fmt -p <package>
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p <package> --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p <package>
```

Several `-p` flags may share one call. Report each check's final `status=` line.

### C3. Launch and result contract (MG-02 owns it; MG-06 and MG-24 build on it)

```rust
// crates/content-engine/src/actions.rs
Action::StartMinigame {
    minigame_type: String,
    difficulty: u32,                       // existing, serde default 1
    on_victory_chains: Vec<i64>,           // existing
    #[serde(default)] on_failure_chains: Vec<i64>,   // MG-02, params key "on_failure_chains"
}
// crates/wire/src/cell/messages/cell_to_base.rs
CellToBaseMsg::StartMinigame {
    entity_id: u32, player_id: i32, game_name: String, difficulty: u32,
    on_victory_chains: Vec<i64>,
    on_failure_chains: Vec<i64>,           // MG-02
    player_level: u32,                     // MG-02, from the cell entity
    intelligence: u32,                     // MG-02, from the cell entity
    tech_competency: u32,                  // MG-02 sends 1; MG-22 fills
    abilities_mask: u32,                   // MG-02 sends 0; MG-22 fills
}
CellToBaseMsg::MinigameResult { entity_id, ticket, result_code, on_victory_chains, on_failure_chains /* MG-02 */, notify_client }
// crates/wire/src/cell/messages/base_to_cell.rs
BaseToCellMsg::MinigameResult { entity_id: u32, result_code: u8, on_victory_chains: Vec<i64>, on_failure_chains: Vec<i64> /* MG-02 */ }
```

Result codes on the server channel: 0 canceled, 1 victory, 2 failure, 3 interrupted or not started, 4 defeated. The cell fires `on_victory_chains` on 1 and `on_failure_chains` on 2 and 4 (D-MG5). From MG-06 on, any code clears the minigame state flags.

### C4. Game module layout (MG-03 owns it; every port follows it)

```text
crates/minigame/src/minigame/games/
  kind.rs                 GameKind, Validation (MG-03)
  mod.rs                  create(), validation() delegate (MG-03)
  placeholder.rs          shells and not-yet-ported games
  <game>/mod.rs           <Game>Game, MinigameInstance impl, PORTED flag, create()
  <game>/setup.rs         difficulty table, board generation, fullgamestate (<G>1)
  <game>/moves.rs         move validation, cascades or plays, scoring (<G>2)
  <game>/rules.rs         victory, timer, hints, cheats (<G>3)
  <game>/tests_setup.rs, tests_moves.rs, tests_rules.rs
```

G1 adds files where a slice would pass the soft cap: `alignment/cascade.rs` and `alignment/moves_gravity.rs` (AL2b), `alignment/cheats.rs` (AL3b), `goauld_crystals/power.rs` (GC1b) and `goauld_crystals/cheats.rs` (GC3b), each with a matching `tests_<name>.rs`. Livewire's parity tests may go in `livewire/tests_parity.rs` (MG-LW1).

| Game (`target_key`, case-sensitive) | Folder | Port prefix |
|---|---|---|
| `Alignment` | `alignment` | MG-AL |
| `CrystalGame` | `crystal_game` | MG-CG |
| `GoauldCrystals` | `goauld_crystals` | MG-GC |
| `ConverseBasicHumanoid` | `converse_humanoid` | MG-CH |

`GameKind::validation()` returns `Server` for Livewire, and for a ported game only once its module's `pub const PORTED: bool` is `true`. Only the `<G>3` packet flips it. Until then `create()` hands the name to the placeholder. A half-ported game is never reachable from content.

Every game uses `SfsValue::list(...)` (MG-01) for arrays, and keeps every random draw on one `StdRng::seed_from_u64(session.seed as u64)`, so a REPL transcript is reproducible.

### C5. Spec doc format (every research packet writes to it)

Each `docs/analysis/minigames/spec-<game>.md` has these sections, in this order:

1. **Verdict.** A real game or a shell, with the evidence: package, GFX offset and length, md5, and the extension's function list.
2. **`joinOK` params read:** name, type, use.
3. **Board model:** dimensions per tier, piece ids, the initial generation as pseudocode, and whether the client must reproduce anything from `seed`.
4. **Difficulty table:** every tier, with the exact numbers from the extension and the Python value beside it where one exists.
5. **Message table:** one row per message, both directions, with each field's exact name and `SfsValue` type, the inbound validation rules and the reply.
6. **Scoring, win and loss:** goals, timer, and the result code sent.
7. **Cheats:** which `abilityBitfield` bits and item cheats it reads, and what each does.
8. **Hints and timers** the server must run.
9. **Golden transcripts:** at least two seeded sequences (seed, tier, inputs, expected outputs). Later packets turn them into `.mgs` scripts (C6).
10. **Port slicing:** what goes into `<G>1`, `<G>2` and `<G>3`, with files and line estimates. Each slice must fit about 40k working tokens.
11. **Open questions.**

Excerpts of client code are at most 20 lines each; no full disassembly goes in git (D-MG10).

### C6. REPL scripts (MG-05 owns the format)

```text
# comment
game Alignment difficulty 2 seed 7
start
< fullgamestate {board={0=..., 1=...}, timeRemaining=120}
> processmove current=12 target=13
< moveresult {...}
tick 4
abort
```

- The first non-comment line names the game, the tier and the seed.
- A `>` line sends one client command: `cmd key=value ...`, where a value is a number if it parses as one, `true` or `false` as a bool, and a string otherwise.
- A `<` line is the exact expected next output line. Keys are sorted, and nested objects render as `{k=v, ...}`.
- `tick N` runs N 250 ms ticks, and `abort` calls `aborted()`. `< victory` and `< failure` are the terminal outputs.
- Golden scripts live in `crates/minigame/tests/scripts/<game>/<name>.mgs`, and a test runs each with `repl::check_script`.

### C7. Seed ids for content packets

Minigame content chains use **24000-24049** (free on `main` @ `0d71b3b30`). Each packet re-checks its range with Grep before writing. Since G1 only MG-61 takes chains (24000-24001); 24002-24049 stay unused. Hand-authored effect NVPs use **25500-25509**: MG-63b takes 25500, MG-62 25501-25504. One packet owns one seed file (see each packet).

## Wave 0A: research, cross-cutting

Docs-and-tools only. Each agent writes its spec, commits, and stops. Ops merges the three branches into `mg-wave0a` with `merge-noff.ps1`; the wave reviewer reads the integrated tree. For these packets C1 rules 1, 3, 6, 7 and 9 apply; rule 2 applies only to R0's catalog regen and any lane-run tool.

### MG-R0: SWF extraction tools

- Agent: game-archaeology-specialist · Host: windows · Wave 0A
- Worktree `mg-r0` · Branch `minigames/mg-r0-swf-tools` · Commit `MG-R0: tools/minigame-swf extracts and lists the minigame GFX movies`
- **Goal.** A repo tool that turns the client's `UI\Flash\*.upk` into `.gfx` and `.swf` files plus a function listing, so wave 0B and later reviewers can rerun every claim.
- **Files (new):**
  - `tools/minigame-swf/extract_gfx.py`. It takes `<package-dir-or-file> --out <dir>`. It finds the GFX signature (`GFX`, or `CFX`, which is zlib-compressed) followed by a version byte and a u32 little-endian length. For each package it writes `<Name>.gfx`, `<Name>.swf` (signature rewritten to `FWS`/`CWS`) and `<Name>.json` (offset, length, md5, version). It refuses an `--out` inside the repo (D-MG10).
  - `tools/minigame-swf/as2dis.py`. `git mv` it from `docs/analysis/cellblock-autoplay/livewire-spike/as2dis.py`, fix the imports of the spike files that use it (Grep `as2dis`), and leave a one-line pointer in the spike README.
  - `tools/minigame-swf/list_functions.py`. It walks `DoAction`, `DoInitAction` and `DefineSprite` tags and prints `sprite frame function(args)` for every `DefineFunction`/`DefineFunction2`. `--strings` dumps the constant pools, and `--function <name>` disassembles one function.
  - `tools/minigame-swf/README.md` (CRLF): usage, and the untrusted-data rule (run with `python -I`, output outside the repo).
  - `tools/minigame-swf/tests/test_extract_gfx.py`. It builds a synthetic package in memory (padding, `GFX` header, length, body). It asserts the offsets, the `FWS` rewrite and the refusal of an in-repo `--out`. No client data.
- **Also:** `docs/analysis/minigames/client-packages.md` (CRLF), the inventory from running the tool on the user's client (`..\SGW\Client\game\Working\SGWGame\CookedPC\UI\Flash\`). One row per package: offset, length, md5, SWF version, function count, extension present (yes or no), shell (yes or no).
- **Checks:** `Set-Location <wt>; python -m unittest discover -s tools/minigame-swf/tests`, then `python tools/docs-gen/regen.py --skip-crate-graph` (the tools catalog is generated), then `python tools/docs-gen/regen.py --check`.
- **Docs rows:** the tools catalog (generated); [README § What the client has](README.md#what-the-client-has) cites `client-packages.md`.

### MG-R6: Livewire parity and the common SFS protocol

- Agent: minigame-systems-advisor · Host: windows · Wave 0A
- Worktree `mg-r6` · Branch `minigames/mg-r6-protocol-spec` · Commit `MG-R6: common minigame protocol spec and Livewire parity audit`
- **Read first:** `.claude/agent-memory/minigame-systems-advisor/*`, [minigame-wire-formats.md](../../reverse-engineering/findings/minigame-wire-formats.md), [minigame-system.md](../../gameplay/minigame-system.md), `crates/minigame/src/minigame/{protocol.rs,games/livewire/}`, `deprecated/python/base/minigame/Livewire.py`. Extract the SWFs with the MG-R0 tool if it has landed; otherwise use the spike `as2dis.py` and a scratch copy (output outside the repo).
- **Write** `docs/analysis/minigames/spec-protocol-common.md`, answering:
  1. Does any SWF run its embedded extension **locally** (offline mode), and if so, under which condition? Show the branch in Livewire's movie. If a game could run without the server, the server-validated design (D-MG2) needs a ruling: flag it.
  2. Every `joinOK` param each of the 10 games reads, as a table of game by param.
  3. Does any SWF derive board or refill state from `seed` that must match the server? If so, give the exact PRNG algorithm (for MG-25); if not, say "no PRNG port needed".
  4. End of game: which commands each SWF sends and expects for victory, failure, defeat and abort; how the SWF reports `MinigameComplete`; the mapping to server codes 0-4.
  5. Who owns the countdown: the client, the server, or both.
  6. Does any SWF need `t='o'` rather than `t='a'` on an object?
- **Write** `docs/analysis/minigames/spec-livewire-parity.md`: the extension's difficulty table, the wire-variant range (does the library export variant 4?), the library-name form, and cheat handling, compared with `games/livewire/setup.rs`. Every delta gets file:line and a proposed change for MG-LW1. "No delta" is a valid result.
- **Memory:** update `.claude/agent-memory/minigame-systems-advisor/livewire-client-swf-facts.md` and `game-implementation-status.md` with the confirmed facts.
- **Checks:** `pwsh tools/lint-md.ps1 docs/analysis/minigames/spec-protocol-common.md docs/analysis/minigames/spec-livewire-parity.md`; CRLF.

### MG-R7: Launch flow, modifiers and ability launch

- Agent: bigworld-engine-advisor (it may consult game-archaeology-specialist for Ghidra) · Host: windows · Wave 0A
- Worktree `mg-r7` · Branch `minigames/mg-r7-launch-spec` · Commit `MG-R7: minigame launch flow, modifiers and ability-launch spec`
- **Read first:** `docs/protocol/*-dispatch-table.md` (the minigame rows), `entities/defs/` (`MinigamePlayer`), [minigame-architecture.md](../../reverse-engineering/findings/minigame-architecture.md), `deprecated/python/cell/SGWPlayer.py:1903-1990`, `deprecated/python/cell/Minigame.py`, `deprecated/python/common/Config.py` (`MINIGAME_NAMES`), and Grep `deprecated/python` for `Minigame` in effect scripts. Seeds: abilities 677-686, 778, 779, 792, 793, 802, 809, 813-815, 1195, 1327, 1552, 1594, 1597, 1661, 1753, 2089, 2507, 2508, 2837, 585, 586; effects 1640-1643, 1939, 2032, 2811-2813, 4116, 647, 648; items whose names mention "Minigame"; event set 1132, sequence 2304.
- **Write** `docs/analysis/minigames/spec-launch-flow.md`, with a "Packet inputs" section at the end that gives MG-21 to MG-24 and MG-63 exact values:
  1. `onStartMinigameDialog` and `onStartMinigameDialogClose`: args, types and client behaviour. Cell methods 24 `startMinigame` and 30 `minigameStartCancel`: args. Can Cimmeria send the dialog with no client patch?
  2. Where the original set and cleared `BSF_PlayingMinigame` and the movement lock, and whether the client reacts to bit 4 (`targetIsPlayingMinigame`).
  3. `techcomp`: its source in the original (a stat, the archetype, an item "TC" level?) and its range.
  4. `abilityBitfield`: bit to ability id, from the Python, the SWF readers (`activateabilitycheat`) and the seeds.
  5. Item cheats `CA0`-`CA4`, `instcc` and `consUsed`: slot to item id, and the consumption semantics. Are the cheat consumables obtainable at all (README M8)?
  6. Ability-started games. For 1327/1643, 2089/2813, 1597/1939, 1661/2837, 585, 586, 1594 and 1753: what each did in the original, which game it starts, the victory and failure outcome, who the target is, and the effect-script contract for MG-24 (params, and how the game name and tier are chosen).
  7. Prison Boot 1597: who casts it, where, and what removes it. Propose a design for MG-63.
  8. Ability 802 "Reveal Mini-Games" and event set 1132: what they reveal; confirm or refute README M7.
- **Memory:** add a fact file under `.claude/agent-memory/bigworld-engine-advisor/` if a durable fact is found.
- **Checks:** `pwsh tools/lint-md.ps1 docs/analysis/minigames/spec-launch-flow.md`; CRLF.

## Wave 0B: research, per game

Agent: game-archaeology-specialist, one session per packet. Use the MG-R0 tools and write to the C5 format. Branch from `minigames/wave0a` once it has merged to `main` (or from `main` after). C1 rules 1, 3, 6, 7 and 9 apply. Checks for each: `pwsh tools/lint-md.ps1 <spec files>`; CRLF; no client bytes in git.

### MG-R1: Alignment spec

- Host: windows · Worktree `mg-r1` · Branch `minigames/mg-r1-alignment-spec` · Commit `MG-R1: Alignment rules spec from the embedded SFS extension`
- **Write** `spec-alignment.md` (C5).
- **Must answer:** the hex playfield geometry and neighbour rule (`findNeighbors`, `direction`); the swap legality rule (`processmove(current, target, cardType)`, `swapPieces`); cascade and refill (`checkForCascade`, `doCascade`), including whether refills come from `rng` (feeds D-MG12); scoring per goal type (cipher, protocol, code, force, power, achievement, experience, cash) and `doscore`; `processreset`; hints (`callhint`, `getHint`, `resethinttimer`); the cheat timers `stct`, `upct` and `osct`; `itemCheatInfo`, `itemCheatActivate` and `consUsed`; `dovictory` and `minigameCompleteGame`; what "bypasshexspinner" is (feeds D-MG3 option B).
- **Cross-check:** the difficulty tables in `deprecated/python/base/minigame/Alignment.py` (keyed 0-10; 120/120/100/80 s; 8x8 to 10x10).

### MG-R2: CrystalGame spec

- Host: windows · Worktree `mg-r2` · Branch `minigames/mg-r2-crystalgame-spec` · Commit `MG-R2: CrystalGame rules spec from the embedded SFS extension`
- **Write** `spec-crystalgame.md` (C5). It must cover `processMove`, `checkForVictory` and the `Crystal` object model. State whether it shares code with GoauldCrystals; if they share a rules core, say which functions, so G1 can slice one shared module.

### MG-R3: GoauldCrystals spec

- Host: windows · Worktree `mg-r3` · Branch `minigames/mg-r3-goauldcrystals-spec` · Commit `MG-R3: GoauldCrystals rules spec from the embedded SFS extension`
- **Write** `spec-goauldcrystals.md` (C5). It must cover `handleRequest`, `Swap` and `Crystal`, and find the victory test (`checkForVictory` was not found by name). Cross-check `deprecated/python/base/minigame/GoauldCrystals.py`'s tables. Same shared-core question as MG-R2.

### MG-R4: ConverseBasicHumanoid spec

- Host: windows · Worktree `mg-r4` · Branch `minigames/mg-r4-converse-spec` · Commit `MG-R4: ConverseBasicHumanoid card-game spec from the embedded SFS extension`
- **Write** `spec-conversebasichumanoid.md` (C5). It must cover the deck (`Card`, `Suit`), the play rules (`Match`, `Swap`), the opponent (is the NPC side played by the extension?), `checkForVictory`, and how the "Trump" passives 778, 779, 792 and 793 enter (an `abilityBitfield` bit?). It must also say whether the Converse shell is a dead parent of this game (feeds D-MG3).

### MG-R5: Shells, type table and content map

- Host: windows · Worktree `mg-r5` · Branch `minigames/mg-r5-shells-content` · Commit `MG-R5: shell-game spec, client type table and minigame content map`
- **Write** `spec-shells.md`: the complete message set and timer of the five shells (Hack, Activate, Analyze, Bypass, Converse); prove whether any of them holds rules (README F1); the client type table at `0x019d917c` and handler `0x00e32140`. Cover how the name in `onStartMinigame` picks the package, and what the client does with an unknown name. Also cover the audio hints (`activateMG.fev`: activate, repair and search variants; [audio-voice-inventory.md:515](../../client/audio-voice-inventory.md)).
- **Write** `content-map.md`: every seed reference to each game, by Grep over `db/resources/`. That covers `start_minigame` rows, interaction bits 256-4096 on templates (README F14), missions, steps and objectives (582/2307, 1244/3619/4203, 4291/4963, 652/789), dialogs, NPCs ("Marsh"), items, and the abilities from MG-R7's list. It also traces how a right-click on an `INT_Minigame*` object reaches the server. For step 2307, name the NPC, the dialog and the game, and propose the chain shape for MG-61 (D-MG7). Confirm or refute README M1-M5.

## Wave 1: foundations

Code packets; they do not depend on research. One PR per packet. MG-05 stacks on MG-03 and MG-06 on MG-02.

**Rules (inline, C1):** PowerShell only and `Set-Location <wt>;` per command; cargo only through `pwsh tools/build-lane/lane.ps1` (except `cargo fmt`), in the foreground; edit only the named files; 500/700-line caps, new tests in new files; a test that fails on revert; docs CRLF; no `git stash`, prune or push; one local commit with the brief's attribution; no lab.

### MG-01: Shared `SfsValue::list` helper

- Agent: packet-coder · Host: windows · Wave 1 · Depends on: none
- Worktree `mg-01` · Branch `minigames/mg-01-sfs-list` · Commit `MG-01: shared SfsValue::list helper for minigame arrays`
- **Files:**
  - New `crates/minigame/src/minigame/protocol_list.rs`, holding `impl SfsValue { pub fn list<I: IntoIterator<Item = SfsValue>>(items: I) -> SfsValue }`. It builds `SfsValue::Object` with keys `"0"`, `"1"`, ... in order, which is how SFS 1.x arrays travel (`<obj t='a'>`, numeric `n`).
  - `crates/minigame/src/minigame/mod.rs`: add `mod protocol_list;`.
  - `crates/minigame/src/minigame/games/livewire/setup.rs`: delete the local `fn array_to_obj` (line 378) and call `SfsValue::list(...)` where it was used.
- **Test (type 1 unit),** in new `crates/minigame/src/minigame/protocol_list_tests.rs`, registered from `protocol_list.rs` with `#[cfg(test)] #[path = "protocol_list_tests.rs"] mod tests;`:
  - `list_keys_are_indices`: three values become keys `"0"`, `"1"` and `"2"` holding those values.
  - `list_encodes_as_sfs_array`: `encode_extension` of a map holding a list contains `t='a'` and `n='2'`.

  The existing Livewire tests must stay green, which proves the swap is behaviour-neutral. On revert both new tests fail to compile, and the Livewire setup loses its helper.
- **Lanes:** C2 for `cimmeria-minigame`.
- **Docs:** none (internal helper); say so in the report.

### MG-02: Launch params and failure chains

- Agent: rust-gameserver-dev · Host: windows · Wave 1 · Depends on: none · Reviewer pass: plus server-authority-enforcer
- Worktree `mg-02` · Branch `minigames/mg-02-launch-params` · Commit `MG-02: minigame launch carries player params and on_failure_chains`
- **Goal.** Contract C3, end to end. Player level and Intelligence reach the SWF's `joinOK` instead of the hard-coded 1 and 0, and `on_failure_chains` fires on results 2 and 4 (D-MG5).
- **Files:**
  - `crates/content-engine/src/actions.rs`: add the `on_failure_chains` field with `#[serde(default)]`.
  - `crates/content-engine/src/loader/action.rs`: parse `params.on_failure_chains` the way `on_victory_chains` is parsed (line ~151).
  - `crates/wire/src/cell/messages/cell_to_base.rs`: add the C3 fields to `StartMinigame` and `MinigameResult`.
  - `crates/wire/src/cell/messages/base_to_cell.rs`: add `on_failure_chains` to `MinigameResult`.
  - `crates/cell-content/src/cell/content/executor/dispatch.rs` (`Action::StartMinigame` arm, line 177): fill `player_level` and `intelligence` from the player's cell entity. Use the existing level and stat accessors (Grep `Intelligence` in `crates/entity`). If the entity carries no Intelligence value, send 0 and say so in the report. Send `tech_competency: 1` and `abilities_mask: 0`, each with a `// MG-22` comment.
  - `crates/base-world-entry/src/base/world_entry/cell_dispatch/{progression_dispatch.rs,minigame.rs}`: pass the new fields to `registry.register(...)` in place of the literals `1`, `0`, `0`, `1`; carry `on_failure_chains` into the session and back out on the result.
  - `crates/minigame/src/minigame/session.rs`: `MinigameSession.on_failure_chains: Vec<i64>`, plus the `register` parameter.
  - `crates/minigame/src/minigame/server/result_dispatch.rs`: put `on_failure_chains` on `CellToBaseMsg::MinigameResult`.
  - `crates/cell/src/cell/service/base_messages/minigame.rs`, plus the `BaseToCellMsg::MinigameResult` arm in `base_messages/mod.rs`: fire `on_failure_chains` when `result_code` is 2 or 4, through `content::fire_chain_by_id` as the victory path does. Log at INFO with the same identity fields and `chains`.
  - Every struct literal and destructure of these types in tests. Use Grep (`StartMinigame {`, `MinigameResult {`, `MinigameSession {`), including `MinigameSession::offline` if MG-03 has merged.
- **Tests:**
  - Type 1: a loader test in `crates/content-engine/src/loader/tests/action_conversion.rs` (or a new sibling file if that one is over 600 lines). `on_failure_chains: [7]` parses; omitted gives `[]`.
  - Type 1: in `crates/cell/src/cell/service/base_messages/tests/minigame.rs`, result 2 fires the failure chain and not the victory chain; result 1 fires only victory; results 0 and 3 fire neither. Assert through the chain engine's fired list or the outbound messages, the way the existing tests there do.
  - Type 1: in `crates/base-world-entry/src/base/world_entry/cell_dispatch/tests.rs`, `StartMinigame { player_level: 17, intelligence: 9, .. }` registers a session with those values. Today's code registers 1 and 0.

  On revert, failure chains never fire and the session holds the literals.
- **Lanes:** C2 for `cimmeria-content-engine`, `cimmeria-wire`, `cimmeria-cell-content`, `cimmeria-base-world-entry`, `cimmeria-minigame` and `cimmeria-cell`. Also run `pwsh tools/build-lane/live-db-test.ps1 start_minigame` (the chain-replay tests that build `StartMinigame`).
- **Docs:** the `start_minigame` row of the content-action reference under `docs/content/` (Grep `start_minigame`): add `on_failure_chains`. Doc-update-map row "Game systems, content chains, or content-engine actions".

### MG-03: Game registry scaffold and offline session

- Agent: packet-coder · Host: windows · Wave 1 · Depends on: none
- Worktree `mg-03` · Branch `minigames/mg-03-game-kind` · Commit `MG-03: GameKind registry and per-game module scaffold`
- **Files:**
  - New `crates/minigame/src/minigame/games/kind.rs`:

    ```rust
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum GameKind { Livewire, Alignment, CrystalGame, GoauldCrystals, ConverseBasicHumanoid, Hack, Activate, Analyze, Bypass, Converse }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Validation { Server, ClientDeclared }
    impl Validation { pub fn as_str(self) -> &'static str { match self { Self::Server => "server", Self::ClientDeclared => "client_declared" } } }
    impl GameKind {
        pub const ALL: [GameKind; 10] = [/* in the order above */];
        pub fn from_name(name: &str) -> Option<GameKind>   // exact, case-sensitive: the client type-table names
        pub fn name(self) -> &'static str
        pub fn validation(self) -> Validation              // Livewire: Server; the four ports: Server only if <module>::PORTED; shells: ClientDeclared
    }
    ```

  - `crates/minigame/src/minigame/games/mod.rs`:
    - `mod kind; pub use kind::{GameKind, Validation};` and `mod alignment; mod crystal_game; mod goauld_crystals; mod converse_humanoid;`.
    - `create()` matches on `GameKind::from_name`. Livewire as today. Each of the four calls `<module>::create(session)`. The shells use `placeholder::PlaceholderGame::new`. `None` keeps today's WARN and placeholder.
    - `validation(name)` keeps its signature (`session_loop.rs` calls it) and returns `GameKind::from_name(name).map_or("client_declared", |k| k.validation().as_str())`.
    - Move the existing `validation_names_the_validator` test unchanged into the new test file.
  - New `games/{alignment,crystal_game,goauld_crystals,converse_humanoid}/mod.rs`, each exactly:

    ```rust
    //! <Game>: the port lands in MG-<XX>1..3 (docs/analysis/minigames/spec-<game>.md).
    use crate::minigame::game::MinigameInstance;
    use crate::minigame::session::MinigameSession;
    /// Flipped to `true` by MG-<XX>3 only, once victory is server-decided.
    pub(crate) const PORTED: bool = false;
    pub(crate) fn create(session: &MinigameSession) -> Box<dyn MinigameInstance> {
        Box::new(super::placeholder::PlaceholderGame::new(&session.game_name))
    }
    ```

  - New `crates/minigame/src/minigame/session_offline.rs`: `impl MinigameSession { pub fn offline(game_name: &str, difficulty: u32, seed: u32) -> MinigameSession }`. Use the field values of `make_session` in `games/livewire/tests.rs:15-34`, with `tech_competency: 1` and `player_level: 1`. Add `mod session_offline;` to `crates/minigame/src/minigame/mod.rs`.
- **Tests (type 1),** in new `games/kind_tests.rs` (registered from `kind.rs` with `#[cfg(test)] #[path = "kind_tests.rs"] mod tests;`):
  - every name in `GameKind::ALL` round-trips through `name()` and `from_name()`;
  - `from_name("livewire")` and `from_name("NoSuchGame")` are `None`;
  - the validation table, including `games::validation("NoSuchGame") == "client_declared"`;
  - `games::create(&MinigameSession::offline(n, 1, 7))` is `Some` for all 10 names and for an unknown one.
- **Lanes:** C2 for `cimmeria-minigame`.
- **Docs:** none (internal); the lib.rs doc comment's game list gains "Alignment, CrystalGame, GoauldCrystals, ConverseBasicHumanoid (scaffold)".

### MG-04: Client-declared hardening

- Agent: packet-coder · Host: windows · Wave 1 · Depends on: none · Reviewer pass: plus server-authority-enforcer
- Worktree `mg-04` · Branch `minigames/mg-04-min-play` · Commit `MG-04: client-declared minigames need 5 s of play before a victory counts`
- **Rule (D-MG13):** a shell's `victory` counts only after at least 5 s since `started()`. An earlier one is refused, a WARN is logged, and the session stays open.
- **Files:** `crates/minigame/src/minigame/games/placeholder.rs` only, plus the new test file.
  - Add `pub(crate) const CLIENT_DECLARED_MIN_PLAY: std::time::Duration = Duration::from_secs(5);` and a field `started_at: Option<tokio::time::Instant>`, set in `started()`.
  - Add `fn victory_allowed(started_at: Option<Instant>, now: Instant) -> bool` (false when `None`).
  - In `message("victory")`, when it is not allowed: `tracing::warn!(game = %self.name, elapsed_ms, reason = "min_play_time", "Client-declared minigame victory refused: under the minimum play time")` and `return vec![]`.
- **Tests (type 1 and 12),** in new `games/placeholder_tests.rs` (registered with `#[cfg(test)] #[path = "placeholder_tests.rs"] mod tests;`), all `#[tokio::test(start_paused = true)]`:
  - `victory_before_min_play_is_refused`: `started()`, then `victory` at once gives no `GameOutput::Victory`;
  - `victory_after_min_play_counts`: `tokio::time::advance(CLIENT_DECLARED_MIN_PLAY)`, then `victory` gives `Victory`;
  - `victory_before_start_is_refused`;
  - if `cimmeria_test_support` has a `LogCapture`, assert the WARN row with `reason = "min_play_time"` (type 12).

  On revert, the first test gets `Victory`.
- **Existing tests:** run the crate's nextest. Any server test that drives a `"Hack"` victory and now fails gets `tokio::time::advance(CLIENT_DECLARED_MIN_PLAY).await` before the victory send. If it is not `start_paused`, switch it. These edits are allowed in `crates/minigame/src/minigame/server/*tests*.rs` only.
- **Lanes:** C2 for `cimmeria-minigame`.
- **Docs:** [CAT-K-minigame.md](../../security-audit/2026-05-31-server-authority/findings/CAT-K-minigame.md), CAT-K-01: a status line, "Mitigated for the shell games by MG-04 (5 s minimum play); the ported games are server-validated (D-MG2)." Doc-update-map row "server-authority or security audit".

### MG-05: Minigame REPL harness

- Agent: packet-coder · Host: windows · Wave 1 · Depends on: MG-03 (stack: `--base minigames/mg-03-game-kind` until MG-03 merges)
- Worktree `mg-05` · Branch `minigames/mg-05-repl` · Commit `MG-05: minigame REPL harness and golden-script runner`
- **Goal.** The per-game logic UAT (D-MG16): drive any `MinigameInstance` from typed commands or a C6 script, with no network.
- **Files:**
  - New `crates/minigame/src/minigame/repl.rs` (`pub mod repl;` in `minigame/mod.rs`), at most 350 lines:

    ```rust
    pub struct Repl { game: Box<dyn MinigameInstance>, ended: Option<&'static str> }
    impl Repl {
        pub fn new(game_name: &str, difficulty: u32, seed: u32) -> Option<Repl>; // games::create(&MinigameSession::offline(..))
        pub fn start(&mut self) -> Vec<String>;
        pub fn send(&mut self, line: &str) -> Result<Vec<String>, String>;       // "cmd k=v ..."
        pub fn tick(&mut self, n: u32) -> Vec<String>;
        pub fn abort(&mut self) -> Vec<String>;
    }
    pub fn render(out: &GameOutput) -> String;      // "< victory" | "< failure" | "< cmd {k=v, ...}", keys sorted, nested objects recursive
    pub fn run_script(text: &str) -> Result<String, String>;   // a transcript with every output line
    pub fn check_script(text: &str) -> Result<(), String>;     // compare `<` lines; Err names the 1-based line
    ```

    Render the `cmd` from the `_cmd` key of a `GameOutput::Send` map; the other keys go in braces. Numbers use Rust `{}` formatting of the `f64`.
  - New `crates/minigame/src/bin/minigame_repl.rs`, with manual `std::env::args` parsing (no new dependency):
    - `--game <name> [--difficulty N] [--seed N]` reads stdin lines (the C6 commands) and prints the outputs;
    - `--script <file>` runs `check_script` and exits 1 on a mismatch;
    - `--record <file>` prints `run_script`'s transcript, for making a golden.
  - New golden `crates/minigame/tests/scripts/livewire/d1_seed7.mgs`. Record it with `--record` from a 3-line script (`game Livewire difficulty 1 seed 7`, `start`, `abort`) and commit the recorded output.
- **Tests (type 1),** in new `crates/minigame/src/minigame/repl_tests.rs`:
  - `render` sorts keys and nests objects;
  - `send("noSuchCmd")` returns no lines and no error;
  - `check_script` reports the right line number on a deliberate mismatch;
  - `check_script(include_str!("../../tests/scripts/livewire/d1_seed7.mgs"))` is `Ok`. Livewire seeds `StdRng` from `session.seed`, so it is deterministic.

  On revert the module is gone, and the golden guards Livewire setup drift.
- **Lanes:** C2 for `cimmeria-minigame`. Then `Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-minigame --bin minigame_repl -- --script crates/minigame/tests/scripts/livewire/d1_seed7.mgs` (exit 0).
- **Docs:** `python tools/docs-gen/regen.py --skip-crate-graph` (the tools catalog lists `crates/*/src/bin/`). Add a two-line "REPL harness" note to [TESTING.md](../../../TESTING.md) under the minigame or type-1 section. Doc-update-map row "Test conventions".

### MG-06: `BSF_PlayingMinigame` and movement lock

- Agent: rust-gameserver-dev · Host: windows · Wave 1 · Depends on: MG-02 (stack on `minigames/mg-02-launch-params`)
- Worktree `mg-06` · Branch `minigames/mg-06-playing-flag` · Commit `MG-06: hold BSF_PlayingMinigame and a movement lock while a minigame runs`
- **Rule (D-MG15):** on launch, set bit 4 and take one counted `BSF_MOVEMENT_LOCK` reference. Clear both on **every** end path.
- **Files:**
  - `crates/wire/src/state_field.rs`: add `pub const BSF_PLAYING_MINIGAME: u32 = 1 << 4;`, documented like its neighbours.
  - `crates/cell-content/src/cell/content/executor/dispatch.rs` (the `StartMinigame` arm): after a successful send, if the player's `state_field & BSF_PLAYING_MINIGAME == 0`, call `set_state_flag(BSF_PLAYING_MINIGAME)` and `set_state_flag(BSF_MOVEMENT_LOCK)` (`crates/entity/src/cell_entity/state_flags.rs`). Send the state-field update the way the existing movement-lock writers do (Grep `BSF_MOVEMENT_LOCK` under `crates/cell*`).
  - `crates/cell/src/cell/service/base_messages/minigame.rs`: for any `result_code`, if bit 4 is set, unset both and send the update.
  - `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs`: `send_launch_failure` also sends `BaseToCellMsg::MinigameResult { result_code: 3, on_victory_chains: vec![], on_failure_chains: vec![], .. }`, so a failed launch frees the player.
  - Audit the other end paths and fix any that never reach the cell's `MinigameResult` handler. The paths are: client cancel (cell method 25 → `CancelMinigame`), disconnect, the TTL sweep of a never-claimed ticket (`SessionRegistry::spawn_sweep`), and logout or world change. List each path and its outcome in the report.
- **Tests (type 1),** in a new file beside `crates/cell/src/cell/service/base_messages/tests/minigame.rs` if that one is over 600 lines:
  - a launch sets bits 4 and 6;
  - result 0 clears both;
  - a stun's movement lock taken before the launch survives the minigame's clear, because the lock is counted;
  - a launch failure (result 3) clears both.

  On revert the bits never set, or never clear.
- **Lanes:** C2 for `cimmeria-wire`, `cimmeria-cell-content`, `cimmeria-cell`, `cimmeria-base-world-entry` and `cimmeria-minigame`.
- **Docs:** [minigame-system.md](../../gameplay/minigame-system.md): one paragraph in the launch-flow section, "While playing", covering the flag, the lock and the end paths. No other wave-1 packet edits that file.
