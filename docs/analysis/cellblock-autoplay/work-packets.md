# Castle Cellblock Autoplay Work Packets

> Type: how-to (packet specifications). Audience: the coordinator and packet workers.
> Updated: 2026-09-29. Companions: [README.md](README.md) (goal, decisions, ledger), [scenario-map.md](scenario-map.md), [livewire-autosolve.md](livewire-autosolve.md), [automated-uat.md](../../guides/automated-uat.md), [TESTING.md](../../../TESTING.md).

## Dispatch rules

- **One PR per packet.** The ledger in [README.md](README.md#ledger) is the status of record; update it in the packet's own PR.
- **Status.** Packets start **Ready**, **BlockedDependency** or **BlockedDecision**. A BlockedDecision packet waits for its D-AP row.
- **Worktrees and builds.** Each worker has its own worktree. Every compiling `cargo` call goes through the build lane (`bash tools/build-lane/lane.sh ...`). Retire the worktree the day the PR merges.
- **Tests.** A runner or tool change ships with unit tests that fail when the change is reverted. A new lab-mcp tool gets a unit test for its response shape, and a live-DB test if it reads the database (`live_db` in the name, `require_db_or_skip!`).
- **Spec validation.** A spec change must pass `cargo test -p cimmeria-lab`, which validates every committed spec. When a planned tool lands, update the `committed_specs_plan_against_main_tools` pin in the same PR.
- **Docs.** A new tool or capability alias updates the capability table (`crates/lab/src/uat/tools.rs`) and [automated-uat.md](../../guides/automated-uat.md). A new `server_*` tool also updates [live-research-lab.md](../../guides/live-research-lab.md) and the ADR's tool list. The CLAUDE.md doc map row for the live research lab applies.
- **Telemetry.** New server tools and runner decisions log with `reason=` on refusal, per [negative-logging-convention.md](../../architecture/negative-logging-convention.md).
- **Live work.** Take the lab lease (`lab_lease_acquire`) first. Never run two live packets at once.
- **Wrong evidence.** A packet that disproves an **(unverified)** point fixes [scenario-map.md](scenario-map.md) or [livewire-autosolve.md](livewire-autosolve.md) in the same PR.

Reference sections: **SM** is [scenario-map.md](scenario-map.md), **LW** is [livewire-autosolve.md](livewire-autosolve.md).

---

## Phase 0: Foundations

### AP-00 Lab install script and local endpoint

**Depends:** none. **Owner decision:** D-AP2. **Effort:** S.

**Problem.** The installed `%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria-lab.exe` was built at or before #1094. It has no world, combat, UI or UAT tools. The repo has no install step, so this will keep recurring (SM §2).

**Scope.**

- `tools/lab/install-lab.ps1`, plus a `.sh` twin if the lab docs keep both.
- It builds through the lane:
  - `cimmeria-lab`
  - i686 `cimmeria-client-telemetry --features lab-bridge`
  - i686 `cimmeria-client-patches`
  - i686 `cimmeria-start32`
- It refuses while `SGW.exe` or the supervisor holds the files, and prints what to close.
- It copies the four binaries into the bin folder and prints the tool count the new supervisor reports.
- `--check` compares the installed binary's tool list against `main`'s (the routed tool names) and reports what is missing.
- It documents the local endpoint: `CIMMERIA_LAB_MCP_URL` and `CIMMERIA_LAB_MCP_TOKEN` for a local server, and the matching `.mcp.json.example` entry.

**Acceptance.**

- A fresh install lists `client_move_to`, `client_window_click` and `lab_uat_run`.
- `lab_uat_run {plan_only:true}` over every committed spec returns the documented ready/blocked split.
- The live-research-lab guide's Setup section points at the script.

### AP-01 Spec and doc drift fixes

**Depends:** none. **Effort:** S.

- **`tools.rs`:** point `@window_click_row` at `client_window_click` (or rename the alias to `@window_click` and update the specs that use it).
- **`consumables.toml` I2:** use `client_item_action {action:"use", name:...}`, and read counts with the lua `getQuantityForSlot` idiom or `client_inventory` `/diff/by_item`. Neither the `type_id` argument nor the `/total` pointer exists.
- **`docs/gameplay/combat-system.md`:** holster is implemented (`crates/cell-methods/src/cell/cell_methods/combatant.rs`, `requestHolsterWeapon`), and the server holsters automatically after 10 s out of combat (`crates/cell/src/cell/service/ticks/holster.rs`).
- **Pin:** update `committed_specs_plan_against_main_tools` for any row that is no longer wrongly BLOCKED.

### AP-02 Runner: one character through a campaign

**Depends:** none. **Effort:** M. **Reviewer:** testing-validation-engineer. Runner changes R1, R2, R3 and R9 (SM §1).

- **R1 resume:**
  - Persist the section's fresh-character name and archetype in `run.json`.
  - On `run_dir` resume, if that name is at character select, adopt it instead of creating it.
  - Today it fails "name taken" and the row is BLOCKED.
- **R2 sequential:**
  - `[section] sequential = true`, and a row-level `depends_on = ["W03"]`.
  - When a prior row ends FAIL or BLOCKED, the rows after it become BLOCKED with `prior row <id> = <result>`.
  - NEEDS_HUMAN, UNVERIFIED and NATIVE_SHORTFALL do not stop the chain.
- **R3:** `[section.fresh] finish_intro = false` stops `ensure_state` from closing dialog 2982 (`runner/session.rs`, the `lab_finish_dialog` call after creation).
- **R9:** the fresh name is derived from `run_id` and the section id, so two fresh sections in one run (Tau'ri and Jaffa) don't collide.

**Tests.**

- Unit tests over a fake tool router:
  - a resumed run adopts rather than creates
  - a FAIL blocks the rest of a sequential section, and a NEEDS_HUMAN does not
  - `finish_intro = false` makes no `lab_finish_dialog` call
  - two sections get different names
- Each test fails with the change reverted.

### AP-03 Runner: tool captures, ids and clause ops

**Depends:** none. **Effort:** M. **Reviewer:** testing-validation-engineer. Runner changes R4, R5 and R6.

- **R4:** `capture = "tool"` and `capture = "server"` actions, with `tool`, `args`, `pointer` and `var`. They store a JSON value (entity id, ammo count, stack size) in `${var}`.
- **R5:** after `ensure_state`, fill `${player_id}` (from `server_sessions` by character name) and `${entity_id}` (from `client_player_state`). The row is UNVERIFIED, never FAIL, when the server endpoint is unreachable.
- **R6 new ops:**
  - `delta`: observed minus a captured var equals `value`
  - `bit_set` and `bit_clear`: for `state_field` (BSF Dead 0x1, AutoCycling 0x2, Crouching 0x4, InCombat 0x8, MovementLock 0x40) and `interaction_type_flags` (u64)
  - `any`: an array element matches a sub-object, e.g. `{name:"Health Slappack", delta:-1}` in `/diff/by_item`
- **Docs:** the spec reference in [automated-uat.md](../../guides/automated-uat.md#write-a-row-spec).

**Tests.**

- Clause-grader unit tests for each op, including u64 bit tests above 2^53, which must not go through f64.
- Tests for capture from JSON and server results.
- R5's UNVERIFIED path.

### AP-04 Runner: repeat-until, row timeouts, timing clauses

**Depends:** AP-03. **Effort:** M. **Reviewer:** testing-validation-engineer. Runner changes R7, R8 and G-TIME.

- **R7:** an action gains `repeat = { until = <clause>, max = N, interval_ms = M }`. It drives:
  - "fire until the target is dead"
  - "use a slap pack while HP < 50%"
  - "retry a click that didn't open the window"
  Each iteration is recorded in `actions[]`.
- **R8:**
  - A per-row `timeout_ms`: the row ends FAIL "row timeout" with the evidence so far.
  - A partial-row resume marker, so a walkthrough that outlives an MCP call carries on from the last completed row.
- **G-TIME:** a `timing` clause over two step labels (`after`, `before`, `max_gap_ms`) for T16/T17's ordering.

**Tests.** Unit tests covering:

- the until-met and max-reached paths
- row timeout grading
- the timing clause's pass and fail

---

## Phase 1: MCP tools

### AP-10 Targeting by template, alive or dead

**Depends:** none. **Effort:** S-M. Gaps G-TGT1 and G-TGT2 (SM §3.2).

**Problem.** Most Cellblock objects have no client name: the Guard corpse, the door button, the vial, every switch and terminal. The drone's name is an empty string. Fifteen guards share "NID Guard", and picking by name takes the nearest one, dead or alive.

**Scope.**

- `client_entity_find` gains `template_id` and `alive: true|false` filters.
- `client_world_click`, `client_target` and `client_move_to` accept `{template_id, nearest|index, alive}` alongside `entity_id`, `name` and `point`.
- First, verify that `unitMobId` equals the server template id. If it doesn't, resolve template to entity id through `server_entity_query {template_id, space_id}`, which is tier-neutral because the click is still real input.

**Tests.**

- Unit tests for the filter over a canned entity table.
- One recorded live check that the drone (template 4) and a dead guard resolve, attached to the PR.

### AP-11 Routes: server path tool, recorder, Cellblock routes

**Depends:** none for the code; AP-00 for the live recording. **Effort:** M. Gap G-NAV1 (SM §3.1).

**Problem.**

- `client_move_to` is straight-line steering with stuck recovery.
- The server navmesh (`data/spaces/castle_cellblock.nav`) is patchy in the stasis room, and Preparation and topside are separate components joined only by the rings.
- Every room-to-room leg needs waypoints, and there are none in the repo.

**Scope.**

- **`server_nav_path {space, from, to}`,** a new read-only lab-mcp tool. It returns the navmesh corridor corners from the server's `find_path`, or `reason=no_path`.
- **`lab_route_record {name}` / `lab_route_stop`.** They sample `client_player_state /position` at 2 Hz while a person walks, then simplify to waypoints (keep a point when the heading changes by more than 20° or at 8 m).
- **Committed routes:** `docs/guides/uat-specs/routes/castle-cellblock.toml` holds named legs in walking order: stasis → Region8 → cell block → med station → ring 1, ring 2 → Preparation locker/terminal → ring switch, ring 3 → Mess Hall → hallways → barracks/crate → Armory → Armory ring. They are recorded once and replayed by `client_move_to {waypoints}`.
- **Spec field:** `route = "<name>"` resolves to the leg's waypoints.
- **Closed doors:** a leg that must fail at a closed door (the stasis door before T25's Livewire, the cell door before T07) is written as an expected `stuck`.

**Tests.**

- Unit tests for simplification and route loading.
- The spec validator rejects an unknown route name.
- Live: every leg replays twice in a row from its start, recorded in the PR.

### AP-12 `client_fight` composite

**Depends:** AP-10. It uses AP-04's `repeat` if merged, but doesn't need it. **Effort:** M. **Advisors:** combat-systems-advisor, items-systems-advisor. Gap G-CMB1 (SM §5 FIGHT, SLAP-PACK, AMMO).

**Behaviour.** One N1 call drives one fight, all with real input:

1. Target the enemy (`client_target` by entity or template, alive).
2. Press T to toggle auto-attack, then fire once, because the server only arms the loop after the first committed ability.
3. Loop until the target's `mortal` combat record arrives or the timeout passes:
   - **Heal:** when `health/current < heal_below * health/max`, use a slap pack (`client_item_action {action:"use", name:"Health Slappack"}`). It has no cooldown, so it may repeat at once.
   - **Reload:** when `ammo/current == 0`, or below `reload_below`, press R.
   - **Keep in range:** a `max_range` breach moves toward the target.

**Arguments.**

- `target`, `timeout_ms`
- `heal_below` (default 0.5), `reload_below` (default 0)
- `max_range`
- `auto_attack` (default true; false fires the hotbar ability each cooldown instead)

**Returns.**

- `{killed, elapsed_ms, shots, hits, heals:[{at_hp, after_hp}], reloads:[{before, after}], ammo_start, ammo_end}`
- `native_level`: N1 unless a sub-step fell back

**Tests.**

- A state-machine unit test over scripted tool results: heal fires below the threshold, reload fires on an empty clip, it stops on `mortal`, it times out.
- A negative test: no heal above the threshold.
- Live evidence: one Mess Hall fight.

### AP-13 Server state tools

**Depends:** none. **Effort:** M. Backlog S1-S4, gaps G-SRV, S2 and G-DLG1 (server half).

New read-only `cimmeria-lab-mcp` tools, each taking `player_id` or a character name:

| Tool | Returns | Source |
|---|---|---|
| `server_player_state` | world, space, position, level, archetype, health, access level, `state_field`, `weapon_holstered`, active bandolier slot | the live cell snapshot joined with `sgw_player` |
| `server_mission_state` | active and completed missions with current step and `completed_objective_ids`, the in-memory counters (`messhall_kills`) and `fired_once_chains` | a new `LabQuery` on `CellEntity.missions`; the DB copy lags a round trip |
| `server_inventory` | per-container rows with item names, stack counts and ammo per bandolier slot | `sgw_inventory` joined with `resources.items` |
| `server_journal_tail {since_seq}` | the per-player `player.journal` ring with a cursor | existing ring |

The journal also gains the kinds it lacks today: mission accept, item grant, dialog display with the dialog id, cover entered with the cover set id, and minigame result. That gives exactly-once checks for dialogs (T01, T05, T09, T11, T16) without client payloads, and a cover proof that doesn't depend on a DEBUG log reaching SigNoz (G-COVDBG).

**Tests.**

- Unit tests for the response shapes.
- A live-DB test for `server_inventory` (`live_db` in the name).
- Cell-level tests showing the journal records each new kind once.

### AP-14 Weapon, ammo and appearance readers

**Depends:** none. **Effort:** S-M. Gaps G-AMMO1 and G-APP (SM §5 AMMO, WEAPON).

- **Ammo picker.** Read the weapon-bar ammo picker once (`client_window_read {window:"WeaponBarWin", include_nodes:true}`) and record its widget names in the scenario map. Then add a `client_ammo_select {ammo_type|name}` N1 flow: click the ammo icon, then the row. This unblocks AMMO-04 to 08 and AMMO-11.
- **Appearance.** `server_entity_get` gains `/appearance`: the equipped item per visible slot, as the server composites it for witnesses. Equips and the T21 "arrives equipped" check then need no human.
- **Weapon switch.** Confirm `client_player_state /ammo/weapon_item_id` follows F1-F5, and note the stock UI's no-op when the pressed slot is already active.

**Tests.**

- The appearance field's shape against a fixture entity.
- The picker flow's widget mapping, over a canned window tree.

### AP-15 Client event payloads (optional)

**Depends:** none. **Effort:** L (reverse engineering). Backlog C5.

The client-side ids for dialog, mission and item events. AP-13's journal covers the same assertions from the server, so this packet is only worth doing if a row needs the client's own view. Leave it Ready but unscheduled.

---

## Phase 1b: Livewire (LW §5)

The details, file references and test lists are in [livewire-autosolve.md §5](livewire-autosolve.md#5-work-packets). Order: MG-L0, then MG-L1 and MG-L4 in parallel, then MG-L2, then MG-L3, then MG-L5. MG-F2 runs alongside as the safety net.

| Id | Packet | Depends | Effort |
|---|---|---|---|
| MG-L0 | **Live spike, no code** (about 1 h). Answers LW §6 Q1-Q4: does a lab cursor move reach the movie, is the movie letterboxed, how fast is the hover feedback, does `cover_mc` clear after the door opens. | AP-00 | S |
| MG-L1 | `lab_snapshot()` on the minigame trait, a per-entity minigame traffic tap, and the outcome kept after the session ends | none | M |
| MG-L4 | Hit-map generator: a port of [livewire-spike/](livewire-spike/) run against the user's `Livewire.upk` at lab start (D-AP4) | D-AP4 | M |
| MG-L2 | `server_minigame_state` and `server_minigame_tap_read`, plus the planned-click solver over the live board | MG-L1, MG-L4 | S-M |
| MG-L3 | `client_minigame_play` N1 flow: open, Play, start button, then per goal wire move, hover-confirm and click; capability row `@minigame_play`. The placeholder games use the same flow with a single `win_btn` click | MG-L0, MG-L2 | M |
| MG-L5 | Rows T25, T07, T10, T13 use `@minigame_play`, with the LW §4 checks: server victory, chains fired, step advanced, the window hides | MG-L3, AP-20 | S |
| MG-F2 | `server_minigame_act` (X): injects a `processmove` through the game's rule check | D-AP5, MG-L1 | S |
| MG-F3 | `.minigame win\|lose\|cancel` GM command | D-AP6 (proposed defer) | S-M |
| MG-P1 | Parity: add the library suffix `1..=4` to every wire kind, and include variant 4 (LW §7). A byte-level `fullgamestate` test checks every `wireLibs` entry against the movie's export names | D-AP7 | S |

---

## Phase 2: The walkthrough spec

### AP-20 Walkthrough spec W00-W19 (Tau'ri)

**Depends:** AP-02 and AP-03, then waves as tools land. **Effort:** L in total, split into waves. **Advisors:** mission-systems-advisor for step and chain ids; items-systems-advisor for LOOT, SLAP-PACK and equips.

**File.** A new section `castle-cellblock-walkthrough` in `docs/guides/uat-specs/castle-cellblock-walkthrough.toml`:

- `character = "fresh"`, `sequential = true`, `finish_intro = false`, Tau'ri Commando (D-AP1)
- The existing `castle-cellblock` T01/T02 row stays as the quick smoke test.

**Rows.** SM §4 in walking order: Region1 → Region8 → Region2 → Region11. Each row is built from SM §4 and the SM §5 blocks, and the added scope is placed where it happens naturally:

| Wave | Rows | Scope added | Needs |
|---|---|---|---|
| 1 | W00 T25 (door checks only), W01 T01/02, W02 T03/04, W03 T26, W04 T05/06 | Prison Boot walk-lock check, dialogs and greet topics, pistol 55 equip, first fight, first corpse loot (one item, then Loot All, then reopen), slap-pack pickup | AP-02, AP-03, AP-10, AP-11 |
| 2 | W06 T08, W07 T09 | Ambernol pickup, early use (a no-op until AP-40), cover at the med-station desk (objective 2484 through `server_mission_state` / journal), drone fight with AMMO-14, using Ambernol and the cure, the vial count drops by exactly one | wave 1, AP-12, AP-13 |
| 3 | W05 T07, W08 T10, W10 T13, W00's Livewire | Livewire wins, then the ring switch and ring 1 → 2 | MG-L5 |
| 4 | W09 T11/12 (+T30), W11 T27, W12 T28/T32 | Marsh briefing, the P90 locker, SMG equip, weapon switch F1/F2 (pistol ↔ SMG, active slot and weapon id change), ring 2 → 3, Marsh follows and barks | wave 3, AP-14 |
| 5 | W13 T14, W14 T15 | Mess Hall and hallways: `client_fight` with auto-attack, reloads, slap packs below 50%, auto-holster 10-12 s after each fight, every corpse looted, the auto-attack range check (bit 0x2 stays set out of range, hits stop, then resume) | AP-04, AP-12, AP-13 |
| 6 | W15 T16/17 (+T31), W16 T18/19, W17 T20, W18 T21/22 | Straegis scene order (timing clause), the Aftermath crate loot window (six items, one taken, then Loot All, then reopen), barracks fights, stealth set equip (appearance reader), Armory terminal (688 must not complete there), Armory ring to Castle, Castle arrival with Frost's Letter and 1360 intact | AP-04, AP-14 |
| 7 | W19 T23 | Relog sweep rows | wave 6 |

**Every row.**

- Uses `route =` legs (AP-11) for movement.
- Checks machine-readably first (SM §3.3): server mission state, inventory diffs, player state, window reads.
- Asks a `human` question only for what no reader sees, such as the Stasis Sickness icon.
- Adds a `signoz` clause for each server event the tester guide names.

**Blocked rows.** T29 stays BLOCKED with D-AP9's reason, and AMMO-02/03/18 stay out (two clients).

**Tests.** `cargo test -p cimmeria-lab` validates every wave; `plan_only` shows which rows are ready.

### AP-21 AMMO rows

**Depends:** AP-20 wave 5, AP-14. **Effort:** S-M.

Fold the single-client AMMO rows into the walkthrough (SM §5 AMMO): AMMO-19 in W00, AMMO-14 in W06, and AMMO-04 to 08, 10 to 13 in W13. Where the guide uses a GM setup (`.giveammo`), write it as a G-tier setup action. AMMO-20 needs Castle NIDs; add it only if W18 reaches them.

### AP-22 Jaffa pass section

**Depends:** AP-20 through wave 6, AP-02's R9. **Effort:** S.

A second section, `castle-cellblock-walkthrough-jaffa`, holds only the rows that differ (SM §6): W04 (5021), W05 (5020), W09 (5022/5023) and W16 (loot table 11, 3482 jacket and 2797 staff). It reaches W04 with a flagged G setup (`/gmgotoxyz`, `/gmmissionassign 638 1`) and skips AMMO. WEAPON becomes pistol ↔ staff.

---

## Phase 3: Live

### AP-30 First live run and spec fixes

**Depends:** AP-00, AP-20 wave 1. **Effort:** M, repeated per wave.

1. Run each wave with `lab_uat_run` against a local server (D-AP2).
2. Attest the SigNoz clauses with `lab_uat_attest`.
3. Fix every spec error the run exposes, and every **(unverified)** point it settles, in the scenario map.
4. Commit the run's `ledger.md` summary (not the bundle) under `runs/` in this folder.
5. File real defects as issues. A product bug is not fixed inside a spec PR.

### AP-31 Full end-to-end run and repeatability how-to

**Depends:** every wave, MG-L5. **Effort:** M.

**Criterion.** Two full runs in a row from a fresh character, each ending with every row PASS, NEEDS_HUMAN or a documented BLOCKED (T29, the two-client rows).

**Docs.**

- A "Run the Cellblock walkthrough" section in [automated-uat.md](../../guides/automated-uat.md): prerequisites, install check (AP-00 `--check`), the call, expected duration, and how to resume.
- Link it from [unified-uat.md](../../guides/unified-uat.md#castle-cellblock-tutorial).
- Update the spec coverage table in automated-uat.md.

### AP-32 Close-out

**Depends:** AP-31. **Effort:** S.

- Update [docs/project-status.md](../../project-status.md) and [docs/gap-analysis.md](../../gap-analysis.md) once.
- Update the [lab tooling backlog](../lab-automation/tooling-backlog.md) to mark S1-S4, B-items and the rest delivered.
- Promote verified facts from agent memory to docs.
- Retire the worktrees.

---

## Content fix

### AP-40 Ambernol early-use feedback

**Depends:** D-AP8. **Effort:** S. **Advisor:** items-systems-advisor.

**Problem.** Using the Ambernol vial (item 19) before step 2343 does nothing and shows nothing.

**Fix.** Add a player-visible refusal: a chat line or error code the client already renders. The native consumable path stands aside for item 19, so the refusal belongs where that path skips it.

**Tests.**

- A regression guard that the refusal fires before 2343 and not after.
- The item is not consumed on refusal.

The walkthrough's W06 then checks the message.
