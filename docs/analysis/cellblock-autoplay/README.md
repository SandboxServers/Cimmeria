# Castle Cellblock Autoplay Campaign

> Type: how-to (campaign launch and ledger). Audience: the coordinator session and packet workers.
> Updated: 2026-09-29. Companions: [work-packets.md](work-packets.md), [scenario-map.md](scenario-map.md), [livewire-autosolve.md](livewire-autosolve.md), [automated UAT how-to](../../guides/automated-uat.md), [Cellblock tester guide](../castle-cellblock-rebuild/uat-guide.md), [lab tooling backlog](../lab-automation/tooling-backlog.md).

## Goal

One repeatable command drives a fresh character through the whole Castle Cellblock tutorial in the real client, through the live research lab, and grades every step from evidence:

```json
{ "sections": ["castle-cellblock-walkthrough"], "server_version": "<service.version>" }
```

passed to `lab_uat_run`. The run starts at character creation and ends on arrival in Castle (world 8). It covers everything a tester does:

- movement between every room, including closed-door checks
- quests: every mission accept, step advance and completion, 622 through 688, plus 1360
- dialogs and greet topics
- Livewire, won with real input, four times (T25, T07, T10, T13)
- object use: switches, terminals, the P90 locker
- item pickups and equips (pistol 55, SMG, the stealth set)
- weapon switching (F1-F5), reloads and ammo, auto-attack and sustained fire
- slap packs under a health-threshold rule
- using Ambernol, and the cure
- seeking cover (the med-station desk, objective 2484)
- corpse looting, one item and Loot All, with no double grant
- the ring transports: ring 1 to 2, 2 to 3, and the Armory ring to Castle
- the relog sweep

A row passes only on evidence at least as strong as a tester's, graded by the runner's native tiers ([automated-uat.md § Native levels](../../guides/automated-uat.md#native-levels)). A Livewire step that cheated to its win, or a click that fell back to Lua, is never a pass.

## Where things stand (baseline `d8bff1baa`, 2026-09-29)

Nothing in this campaign has been built or run yet. Two research passes produced the inputs:

- [scenario-map.md](scenario-map.md): the walkthrough as rows W00-W19 in walking order, each with its tool calls, coordinates, entity tags and checks. Also the blocks for the added scope (FIGHT, AMMO, WEAPON, SLAP-PACK, LOOT, cover, Ambernol), the runner changes and the gap list. It was checked against the code, the seed and the installed client's UI files. Unconfirmed points are marked **(unverified)**.
- [livewire-autosolve.md](livewire-autosolve.md): how the lab wins Livewire with real input. It works from the board the server already knows and a hit map of the client's own movie, and confirms each click by hover first. It solved all 1,200 simulated boards. Its §1-§4 supersede the scenario map's LIVEWIRE block wherever they differ (the start button is verified there).

What exists on `main` and carries over:

- the `lab_uat_run` / `lab_uat_attest` / `lab_uat_report` runner (#1101)
- the world tools (#1099), combat tools (#1100) and UI and item tools (#1102)
- the spec format, and the single T01/T02 row in [castle-cellblock.toml](../../guides/uat-specs/castle-cellblock.toml)

What stops a full run today, in the order the packets fix it:

1. **The installed lab supervisor predates #1099-#1102.** None of the world, combat, UI or UAT tools are in the running binary. The repo has no install step (AP-00).
2. **The runner can't carry one character through a long walkthrough.** Resuming recreates the character, a failed row doesn't stop the rows after it, and the intro dialog is closed before T01 checks it (AP-02).
3. **Nothing can play Livewire,** so the run stops at the cell door (MG-L0 to MG-L5).
4. **`client_move_to` steers in straight lines** and the repo has no Cellblock routes (AP-11).
5. **There is no fight loop,** so about 16 fights have nothing to drive them (AP-04, AP-12).
6. **Server checks need a local lab endpoint and `${player_id}`.** The colo endpoint answered 403 (AP-00, AP-03, AP-13).

## Coordinator launch prompt

You coordinate this campaign. Work from [work-packets.md](work-packets.md) one packet at a time; each packet is one PR.

1. Record `git rev-parse HEAD` in the ledger below, and compare with the baseline. Re-check any cited path that changed under `crates/lab/`, `crates/lab-mcp/`, `crates/client-telemetry/src/bridge/`, `crates/minigame/` or `docs/guides/uat-specs/`.
2. Read [CLAUDE.md](../../../CLAUDE.md), [AGENTS.md](../../../AGENTS.md), [TESTING.md](../../../TESTING.md), [automated-uat.md](../../guides/automated-uat.md) and [live-research-lab.md](../../guides/live-research-lab.md). Give a worker only the scenario-map or design sections its packet cites.
3. Take the lab lease (`lab_lease_acquire`) before any live step ([automated-uat.md § Before you start](../../guides/automated-uat.md#before-you-start)). One agent drives the client at a time. Live packets (MG-L0, AP-30 onwards) are never run in parallel.
4. Answer the decisions below before dispatching the packets they gate. Record each answer as a new row; never rewrite a row.
5. Workers: one worktree and one build-lane slot each, as [development-workflow.md](../../agents/development-workflow.md) says. `rust-gameserver-dev` writes code. `minigame-systems-advisor` reviews MG packets, `combat-systems-advisor` reviews AP-12, `items-systems-advisor` reviews AMMO, LOOT and SLAP-PACK rows, and `testing-validation-engineer` reviews every runner change. Retire each worktree the day its PR merges.
6. Per-packet progress goes in the ledger below, not in `docs/project-status.md` or `docs/gap-analysis.md` (close-out packet AP-32 only).
7. A packet that finds the evidence wrong (an **(unverified)** point that doesn't hold) fixes the scenario map or design in the same PR.

## Decisions

Status **PROPOSED** means the coordinator's recommended default. It becomes a decision only when the owner answers.

| ID | Status | Decision | Gates |
|---|---|---|---|
| D-AP1 | PROPOSED | The main run is a Tau'ri Commando (male, SGU). The Jaffa pass is a second section, and only W04, W05, W09 and W16 differ. It skips AMMO, because the staff takes no bullets. | AP-20, AP-22 |
| D-AP2 | PROPOSED | The walkthrough runs against a **local** server with a local `cimmeria-lab-mcp`. The colo endpoint refused the lab (403), and every server clause would be UNVERIFIED there. A colo run is a later option once that is fixed. | AP-00, AP-30 |
| D-AP3 | PROPOSED | Moving the cursor through the virtual `GetCursorPos` plus CEGUI Lua, inside an N1 flow, counts as N1. `crates/lab/src/supervisor/input.rs` already assumes this; the owner confirms it. | MG-L3, AP-12 |
| D-AP4 | PROPOSED | The Livewire hit map is generated at lab start from the user's own client copy. Nothing derived from client assets goes into git. | MG-L4 |
| D-AP5 | PROPOSED | Build MG-F2 (`server_minigame_act`, tier X): it injects a winning move through the game's normal rule check. It is a safety net for when the N1 player regresses, and its rows grade NATIVE_SHORTFALL, never PASS. | MG-F2 |
| D-AP6 | PROPOSED | **Defer** MG-F3 (a `.minigame win` GM command). It is a new GM power that posts to Discord on every use, and MG-F2 already covers the automation need. Reopen if human testers want it. | MG-F3 |
| D-AP7 | PROPOSED | Fix the Livewire wire-library parity bug (MG-P1) as its own PR, independent of the lab work. The solver copes either way. | MG-P1 |
| D-AP8 | PROPOSED | Using Ambernol before step 2343 is a silent no-op, which breaks the project's first-press-feedback rule. Add a refusal message as its own content packet (AP-40). The walkthrough asserts the message once that lands. | AP-40 |
| D-AP9 | PROPOSED | T29 (flank objectives) stays BLOCKED: the Mess Hall and Hallway05 have no cover data. Seeding cover sets is content work outside this campaign. | AP-20 |
| D-AP10 | PROPOSED | Holster is asserted as the server's automatic holster only (10 s out of combat). The client has no holster binding, so no manual-holster row is written. | AP-20 |
| D-AP11 | PROPOSED | The walkthrough is repeatable on demand, after each release, on a workstation with the client installed. Unattended nightly runs are out of scope; they would need a dedicated machine with its own client. | AP-31 |

## Ledger

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Merged**, **LiveVerified**, **Done**.

| Packet | Title | Status | PR | Notes |
|---|---|---|---|---|
| AP-00 | Lab install script and local endpoint | Ready | | |
| AP-01 | Spec and doc drift fixes | Ready | | |
| AP-02 | Runner: one character through a campaign | Ready | | |
| AP-03 | Runner: tool captures, ids and clause ops | Ready | | |
| AP-04 | Runner: repeat-until, row timeouts, timing clauses | BlockedDependency | | AP-03 |
| AP-10 | Targeting by template, alive or dead | Ready | | |
| AP-11 | Routes: server path tool, recorder, Cellblock routes | Ready | | live half after AP-00 |
| AP-12 | `client_fight` composite | BlockedDependency | | AP-10 |
| AP-13 | Server state tools (player, mission, inventory, journal) | Ready | | |
| AP-14 | Weapon, ammo and appearance readers | Ready | | |
| AP-15 | Client event payloads (backlog C5) | Ready | | optional; AP-13 covers the ids server-side |
| MG-L0 | Livewire live input spike | BlockedDependency | | AP-00 |
| MG-L1 | Minigame snapshot and traffic tap | Ready | | |
| MG-L4 | Livewire hit-map generator | BlockedDecision | | D-AP4 |
| MG-L2 | `server_minigame_state` and solver | BlockedDependency | | MG-L1, MG-L4 |
| MG-L3 | `client_minigame_play` N1 flow | BlockedDependency | | MG-L0, MG-L2 |
| MG-L5 | Livewire rows in the spec | BlockedDependency | | MG-L3, AP-20 |
| MG-F2 | `server_minigame_act` (X safety net) | BlockedDecision | | D-AP5, MG-L1 |
| MG-F3 | `.minigame win` GM command | BlockedDecision | | D-AP6 (proposed defer) |
| MG-P1 | Livewire wire-library parity fix | BlockedDecision | | D-AP7 |
| AP-20 | Walkthrough spec W00-W19 (Tau'ri) | BlockedDependency | | AP-02, AP-03; rows land in waves |
| AP-21 | AMMO rows folded into the walkthrough | BlockedDependency | | AP-20, AP-14 |
| AP-22 | Jaffa pass section | BlockedDependency | | AP-20 |
| AP-30 | First live run and spec fixes | BlockedDependency | | AP-00, AP-20 wave 1 |
| AP-31 | Full end-to-end run, repeatability how-to | BlockedDependency | | everything above |
| AP-32 | Close-out | BlockedDependency | | AP-31 |
| AP-40 | Ambernol early-use feedback | BlockedDecision | | D-AP8 |

## Side findings from the research

These were found along the way and are tracked here so they aren't lost:

- **Livewire parity bug:** some playfield and moving wires never draw, and variant 4 is never chosen ([livewire-autosolve.md §7](livewire-autosolve.md#7-side-finding-wire-library-suffix-parity-bug)). Tracked as MG-P1.
- **Ambernol:** using it early gives no feedback. Tracked as AP-40.
- **Doc drift:** `docs/gameplay/combat-system.md` still lists holster as a stub; `requestHolsterWeapon` is implemented. Fixed in AP-01.
- **Broken spec alias:** `@window_click_row` points at a tool that doesn't exist (#1102 shipped `client_window_click`), and `consumables.toml` row I2 passes arguments `client_item_action` doesn't take. Fixed in AP-01.
- **Castle 708's DHD:** the step is a Livewire repair followed by the DHD dial window, which is not a minigame. A DHD tool is outside this campaign, which ends at arrival in Castle.
