# lab-autopilot work packets

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-31
> **Companions:** [Ledger](README.md), [contract](contract.md), [resume card](RESUME.md), waves [1](packets-wave1.md), [2](packets-wave2.md), [3](packets-wave3.md), [4](packets-wave4.md), [5](packets-wave5.md), [6](packets-wave6.md), [user directives](user-directives.md), [stack inventory](inventory-lab.md), [UE3 back-port facts](ue3-backport-verified.md)

Seven waves, 46 packets. The findings and decisions (D-AX1..D-AX18) are in the [ledger](README.md); every name, key, method and constant is in [contract.md](contract.md). This file holds the dispatch rules, the ops chores, the rules every packet carries, the checks, the wave table and wave 0. Line numbers are from `main` `2019922fe`; re-grep every anchor before editing.

Revised 2026-10-31 against the live pass ([ue3-backport-verified.md § Live pass](ue3-backport-verified.md#live-pass-lab-client-2026-10-31), L1-L8): AX-01 and AX-02 shrank to what is still unknown, AX-55 no longer waits on AX-01, and four packets were added (AX-03 login combo, AX-04 screensaver, AX-26 and AX-27 key fail-safe).

## Dispatch rules

- **Worktrees.** Ask `ops` for one worktree per packet: `pwsh tools/build-lane/mk-worktree.ps1 lab-autopilot/AX-NN-<slug> lax-NN`. Worktrees live at `<repo root>\.claude\worktrees\lax-NN`; briefs give the absolute path. A packet that depends on another packet of its wave gets its branch at the wave integration head after that packet merged in (`branch-at.ps1`).
- **Integration.** Waves 1-5 each ship as **one PR** from an integration worktree `lab-autopilot/waveN` (`lax-wN`), created by `ops` from main once the previous wave's PR merged. Packets merge into it with `merge-noff.ps1` in the wave's merge order. The wave review loop runs on that head before the PR opens. Wave 0 packets ship one PR each and have no wave loop (they share no file; the campaign review covers them). Wave 6 is split (table below).
- **Agents.** Code packets go to `packet-coder` unless the packet names `rust-gameserver-dev` (runner and DLL packets, which stalled Haiku before). Doc packets go to `documentation-writer`. AX-01 goes to `game-archaeology-specialist`. The live packets (AX-02, AX-64) follow the `lab-uat` skill and wait for the user's OK (D-AX4). Each code packet gets a packet-review loop (`packet-reviewer`, at most 3 passes).
- **Owner gates.** No lab use until the user says yes, every time (directive 7). Nothing in waves 1-5 waits on AX-01 except the C6 `actors` field (AX-24) and the `actor_growth` detector (AX-51), which stay off until AX-01 confirms the actor count. D-AX2, D-AX3 and D-AX10 are owner calls that block nothing until wave 6.
- **Report path:** `<worktree>\.claude\reports\lab-autopilot\<packet>-<role>.md`. **Return line:** `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Retire** each packet worktree as soon as it is merged into the integration branch (`rm-worktree.ps1`, or `--merged-into <PR>` after the wave PR merges). Never `git worktree prune`. Never pass `--target-dir`.
- **Commit attribution**, at the end of every commit message. A `packet-coder` (Haiku) commit:

```text
Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_015Wz3WjavngSwPxsZyH3GGx
```

  A Sonnet commit (`rust-gameserver-dev`, fix rounds, `documentation-writer`) names `Claude Sonnet 5.5` with the same session line. Never name a model that did not write the commit.

## Ops chores (not packets)

These are machine chores, not code. Each needs the user's OK; the orchestrator asks, never an agent.

| ID | When | What | Who |
|---|---|---|---|
| O1 Lab rebuild and re-verify | Once before wave 1 dispatches, then again before AX-02 and before AX-64, each time after the merges that run tests | The installed lab daemon, `lab.exe` and the lab DLL are older than `main`. From a clean worktree at the current `main` SHA: `pwsh tools/lab/install.ps1 -DryRun`, then `pwsh tools/lab/install.ps1`, then `pwsh tools/lab/daemon.ps1 restart`; stage the lab DLL (`--features lab-bridge`) the way [live-research-lab.md](../../guides/live-research-lab.md) gives it. Static re-verify: `pwsh tools/lab/daemon.ps1 status` is running, and the daemon and DLL report the `main` SHA they were built from. Live re-verify (heartbeat, `input_status`, `module_info`) is the first step of the next approved live run, never a run of its own. | `hands-on` session or the user (the `ops` command list has no lab install). The restart drops every lab lease, so the lab must be idle. |
| O2 Test-database cleanup | Before wave 1 (and whenever Postgres crash recovery takes minutes) | About 690 leftover per-worktree `sgw_*` databases on the bundled test Postgres make its crash recovery take about 15 minutes. List them, keep every database whose name `rm-worktree.ps1`'s `Get-DbName` maps from a live `git worktree list` entry (and its slot clones), plus the non-worktree databases, and drop the rest, with a dry-run list shown to the user first. Destructive; never part of a packet. | `hands-on` session with the user's OK; restart recipe in agent memory `reference_local_postgres_restart` |

## Rules every packet carries

A `packet-coder` does not read CLAUDE.md; these rules are the repo rules it needs.

- **R1 Shell.** `Host: windows`: PowerShell only, never the Bash tool. Start every command with `Set-Location <your worktree>;`. Use absolute paths for every file write (`[IO.File]` relative paths land in the main checkout).
- **R2 Lane.** Every compiling cargo call (`check`, `build`, `clippy`, `nextest`, `test`, and `fmt` for symmetry) goes through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never call cargo directly; never pass `--target-dir`. Read the lane log under `logs/<worktree>/` before rerunning; at most 5 fix rounds per check.
- **R3 Scope.** Edit only the files your packet names. No `git stash`, no `git worktree prune`, no rebase, no push. When every check passes, `git add <explicit paths>` and commit at once with the packet's subject and the attribution lines.
- **R4 Caps.** Soft cap 500 lines, hard cap 700 per file. New code goes in new sibling files. Never grow a file already over 700: `server/mod.rs` (732) until AX-15 splits it, `supervisor/world/click.rs` (733) until AX-30 splits it, and `uat/runner/tests.rs` (921), which takes no additions ever. Files near the cap get at most the lines the packet allows: `server/uat.rs` 641, `server/instances.rs` 678, `supervisor/world/sim.rs` 670 (AX-13 splits it), `uat/runner/mod.rs` 652 (AX-15 splits it), `composite/ensure.rs` 660, `ui/item_action.rs` 629, `world/movement.rs` 595, `world/steer.rs` 577, `world/camera.rs` 580, `world/io.rs` 571, `uat/runner/actions.rs` 572, `supervisor/input.rs` 596, `supervisor/mod.rs` 645, `supervisor/process.rs` 669, `supervisor/flows/login.rs` 657.
- **R5 Docs.** `docs/**/*.md` are CRLF; check `([IO.File]::ReadAllText('<abs>') -match "[^\r]\n")` is `False`. Never hand-edit a `<!-- gen:NAME -->` block.
- **R6 Tests.** Every behaviour change has a test that fails when the change is reverted; name it and say why in the report. Tests sit in a sibling file (`<name>_tests.rs` via `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`) or in a `#[cfg(test)] mod tests` of a new file. World tools test against `supervisor/world/sim.rs` `Sim`; bridge calls against `supervisor/events/fake_bridge.rs`; server corroboration against an injected fake (`uat/runner/test_server.rs`, `server/record/fakes.rs`); DLL code host-side with game natives stubbed off i686, as `bridge/mod.rs` tests do.
- **R7 No lab, no secrets.** Tests never reach the lab: no test reads `CIMMERIA_LAB_*` from the environment, opens ports 8770 or 8779, starts the daemon or a client, kills or enumerates real processes, or calls an MCP tool on a live server. No IPs, account or character names, tokens or paths of a real machine in code, tests or reports. Never run `tools/testhost/stage.sh` (bash-only; CI `client-dll-boot.yml` runs it).
- **R8 Contract.** Names, JSON keys, methods, error codes and constants come from [contract.md](contract.md). A gap: report `blocked - contract gap: <what>`; never invent one.
- **R9 Tool policy.** Every new routed tool goes in exactly one list of `crates/lab/src/lease/policy.rs` (`OPEN`, `LEASED` or `OWN_LEASE`; the tests `every_routed_tool_is_classified` and `no_tool_is_in_two_lists` in `lease_tests.rs` enforce it) and in `crates/lab/src/proof/claims.rs` (`JUDGED` or `NO_CLAIM`; unlisted means unjudged). Its `#[tool]` description says what evidence it returns. A packet that adds no tool says so in its report.
- **R10 Client-side safety.** DLL packets put all new code under `crates/client-telemetry/src/bridge/`, add no hook, touch nothing compiled without `lab-bridge`, and write the [C13](contract.md#c13-client-side-safety-proof-every-dll-packet) proof, one line per item, in the report.

### Checks

| ID | Command (prefix every one with `Set-Location <worktree>;`) |
|---|---|
| L1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab` then `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings` |
| L2 | `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab` |
| T1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-client-telemetry`, then `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-client-telemetry --target i686-pc-windows-msvc --all-targets -- -D warnings`, then the same clippy with `--features lab-bridge` |
| T2 | `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-client-telemetry --target i686-pc-windows-msvc`, then the same with `--features lab-bridge` |
| A1 | `python tools/agent-skills/sync.py`, then `git status --short .agents .codex` lists the regenerated mirrors |
| D1 | the R5 CRLF check on every `docs/**/*.md` the packet wrote |

CI: `.github/workflows/lab.yml` runs clippy and nextest for `-p cimmeria-lab -p cimmeria-lab-cli`; `client-dll-boot.yml` boots the DLL in `sgw-testhost`.

## Waves

| Wave | Packets | Ships as | Merge order |
|---|---|---|---|
| 0 Fixes, RE and spike (this file) | AX-01, AX-02, AX-03, AX-04 | one PR each; AX-02 only after O1 and the user's OK | AX-04, AX-03, AX-01 as they finish; AX-02 last |
| 1 [Honest results](packets-wave1.md) | AX-10..AX-16 | one PR `lab-autopilot/wave1` | AX-10, AX-15, AX-11, AX-12, AX-13, AX-14, AX-16 |
| 2 [Bridge foundations, key fail-safe](packets-wave2.md) | AX-20..AX-26 | one PR `lab-autopilot/wave2` | AX-20, AX-21, AX-24, AX-25, AX-22, AX-23, AX-26 |
| 3 [World evidence, stops, recoveries](packets-wave3.md) | AX-27, AX-30..AX-36 | one PR `lab-autopilot/wave3` | AX-30, AX-27, AX-31, AX-32, AX-33, AX-34, AX-35, AX-36 |
| 4 [UI, items, input, strict switch](packets-wave4.md) | AX-40..AX-46 | one PR `lab-autopilot/wave4` | AX-45, AX-40, AX-46, AX-41, AX-42, AX-43, AX-44 |
| 5 [Watch, detectors, walker, trace](packets-wave5.md) | AX-50..AX-56 | one PR `lab-autopilot/wave5` | AX-50, AX-51, AX-52, AX-53, AX-54, AX-55, AX-56 |
| 6 [Agents, docs, UAT, close-out](packets-wave6.md) | AX-60..AX-65 | AX-60..AX-63 one PR `lab-autopilot/wave6`; AX-64 (live) writes its ledger in its own docs PR; AX-65 its own PR | AX-60, AX-61, AX-62, AX-63; AX-64; AX-65 |

Wave sizes: waves 1, 2, 4 and 5 hold 7 packets and wave 3 holds 8. AX-27 sits in wave 3 because it needs wave 2's `HoldBook` and `release_all_input`, and it is small (about 120 lines); the wave 3 reviewer gets it with AX-35, which calls the same helper.

Order between waves: wave 0's AX-03 and AX-04 start at once from main, and AX-01 too; O1 and O2 run before wave 1 dispatches. Wave 1 opens at once (AX-10 and AX-15 in parallel). Wave 2's DLL packets (AX-20, AX-21, AX-24, AX-25) touch only `crates/client-telemetry` and may start from main in parallel with wave 1; its daemon packets (AX-22, AX-23, AX-26) start after wave 1 merged, and `lax-w2` is created from main then. Waves 3, 4 and 5 each open when the previous wave's PR merged. AX-55 (DLL only) may branch from main as soon as the wave 2 PR merged (it routes a method in `bridge/dispatch.rs`, which wave 2 edits). Wave 6 opens after wave 5.

## Live versus static: what is still open

| Item | Kind | Where |
|---|---|---|
| Actor count `[[GWorld+0x50]+0x3C/+0x40]` (gates C6 `actors`, C12 `actor_growth`) | static | AX-01 |
| Clock owner of `tickSync` `+0x32C`; `WorldInfo.TimeSeconds` offset (nice to have; C4 already uses `clock::game_time`) | static | AX-01 |
| Coast and input latency (AX-54 priors), screenshot frame bracketing (AX-52), hover id `PC+0x4C0` (AX-31), `game_t` against wall time (C4) | live, ask first | AX-02 |
| Player death bit `+0x158` bit 0 on the player (tested on an NPC only; AX-42 keeps it optional) | live, ask first, needs a zone with hostiles | AX-02 optional step |
| Mouse-look by DirectInput motion with no button held | live, ask first | AX-02 optional step |
| Terrain and outdoor floors answer `0x2086` (the ledge guard self-checks meanwhile) | live, ask first | AX-64 U4 |
| Login combo pick, screensaver refusal, key release on daemon drop and lease end | live, ask first | AX-64 |
| Extent traces, `MultiLineCheck` | not needed; the campaign uses zero-extent `SingleLineCheck` only | none |

Answered by the live pass and not re-asked anywhere: the trace address, convention, `ret 0x1c`, `FCheckResult` layout and flags (`0x2086`, `0x20BF`; `0x3FFF` faults); `GWorld` `0x01EE2684` is class `World`; loading is a `GWorld` pointer change, never a `0x01EE2B6C` poll; `eventGetPlayerViewPoint` `0x005D9690`; `FindFunctionChecked` `0x0049FFB0`; 521.5 counts per radian at gain 20; the window registry holds about 21 000 windows; `client_call_native` takes at most 6 arguments.

## Wave 0: fixes, RE and spike

AX-03 and AX-04 fix two lab failures seen in October 2026; each ships its own PR from main and needs nothing else. AX-01 is static and gates only the `actors` field. AX-02 needs O1 and the user's OK (D-AX4); the orchestrator asks, never an agent. AX-02's numbers feed priors only; until it runs, AX-54 uses the defaults in its packet.

### AX-01 Static RE: one anchors record, the actor count and the clock owner

- **Host:** windows · **Agent:** game-archaeology-specialist · **Branch:** `lab-autopilot/AX-01-anchors` · **Worktree:** `lax-01`
- **Commit:** `docs(re): lab-autopilot client anchors from the static and live passes (AX-01)`
- **Rules:** R1, R3, R5, R7. Static analysis only: Ghidra MCP or headless Ghidra per the `re-lookup` skill. No live client, no x64dbg, no lab.

Most of the old questions are answered: read [ue3-backport-verified.md](ue3-backport-verified.md) § Static pass 2 and § Live pass first, and the agent memory `ue3-reflection-trace-anchors.md`. Do **not** re-derive the trace call, `FCheckResult`, the trace flags, `GWorld`'s class, the loading flag, `0x005D9690`, `FindFunctionChecked`, the mouse gain or the window registry; copy them with their source row. Tasks:

1. **Actor count, the only gate left.** Confirm that `[GWorld + 0x50]` is the persistent level and that level `+0x3C` / `+0x40` are the `Actors` `TArray` data pointer and count (find engine code that iterates them; `supervisor/world/memory.rs` l.285-332 already walks this chain live to find the player controller, which is live evidence the array holds actors). Mark `actor_count` `confirmed` only on code evidence plus that live use; otherwise `likely`, and say so. It gates C6 `actors` (AX-24) and C12 `actor_growth` (AX-51), nothing else.
2. **Clock owner (optional, not a gate).** The object whose `+0x32C` the `tickSync` handler `0x00DD6D00` writes, and the `WorldInfo.TimeSeconds` offset if a short search finds it. C4's `game_t` already uses `hooks/ability_trace/clock.rs`.
3. **Fix labels.** Grep `docs/reverse-engineering/` for any line calling `0x01EE2684` WorldInfo; it is `GWorld` (object "TheWorld", class `World`, vtable `0x01838E7C`, live L7). WorldInfo is the actor a BSP hit reports (vtable `0x018A02B4`, live L2). Note on `0x01EE2B6C` that it is 1 during `LoadMap` (static S4.1) but never seen set from the bridge drain (live L3).
4. Write `docs/reverse-engineering/findings/lab-autopilot-anchors.md` (CRLF, standard header block: Type Reference, Audience, Last updated, Companions): one section per area (traces, world and loading, view point and camera, death, reflection, CEGUI, actors, clock), ending with a table `anchor | address/offset | confidence (live, static, likely) | source row | still open`. Add one row to the findings index if `docs/reverse-engineering/findings/README.md` has a table of findings. Commit any agent-memory fact with it.

Checks: D1. Report: the table, and the `actor_count` verdict (AX-24 and AX-51 read it).

### AX-02 Live spike: coast, latency, frame bracketing, hover, clock

- **Host:** windows · **Agent:** a `lab-uat` coordinator (`hands-on`, Sonnet) with a Haiku `lab-driver` for the movement steps · **Branch:** `lab-autopilot/AX-02-spike` · **Worktree:** `lax-02`
- **Status:** BlockedDecision D-AX4 (the user's OK for this lab run), after O1.
- **Commit:** `docs(lab): lab-autopilot live spike numbers (AX-02)`
- **Rules:** R1, R3, R5, R7. Existing tools only; no code change, no native calls, no shim page, no memory writes. Follow the `lab-uat` skill: confirm the lab is free, take the lease, get in the world with `lab_ensure_in_world`, open ground away from NPCs. Raw-memory reads (`client_mem_read`, `client_batch`) are made by the coordinator, never by the driver (they are on its denylist). First step: the O1 live re-verify (heartbeat, `input_status`, `module_info` show the `main` build).

The trace, loading, view-point, gain and window questions were answered by the 2026-10-31 live pass; do not repeat them. Steps, each repeated 5 times, numbers recorded raw:

1. **Coast and latency, walking and running.** Read the pawn location (`actor + 0xDC`, as `client_entity_find` reports it) in a `client_batch` of `mem_read` and `wait 50` ops. `client_input_key W down`, 2 s, `W up`; sample every 50 ms from 300 ms before the down to 1.5 s after the up. Record: ms from down to the first position change (input latency), speed at the up, metres travelled after the up (coast distance), and `coast_s = coast_distance / speed`. End every run with `client_input_release` and check `input_status` shows nothing held.
2. **Frame bracketing of screenshots.** The heartbeat tick from `lab_client_status`, then `lab_screenshot`, then the tick again: record the tick delta and the wall ms of the screenshot.
3. **Hover.** Point at an NPC with `client_hover_probe`; read the controller's hover id (`PC + 0x4C0`, PC from `client_mem_read` per `hooks/nav/controller.rs`) at the same moment. Record whether they name the same actor.
4. **Game clock.** Two `client_events_read` timestamps (the DLL's `game_t`) 1 s of wall time apart, 5 times: record the `game_t` delta against the wall delta.
5. **Optional, player death.** Only when a zone with hostiles is reachable without content changes: read the player entity's `GameBeing + 0x158` before and after a death (`client_die_and_respawn` or a hostile). Record whether bit 0 sets. Skip and say so otherwise.
6. **Optional, DI mouse-look.** `client_input_mouse dx=200` with no button held; read the camera yaw (`[PC + 0x2B0] + 0x36C`) before and after. Record whether it moved and by how much (expected about 200 / 521.5 rad if DI motion turns the camera).

Write `docs/analysis/lab-autopilot/spike-ax02.md` (CRLF): a table per step with the runs, the median, and a one-line verdict per question (`coast prior for AX-54: <s>`, `bracketing: PrintWindow spans <n> ticks`, `hover id agrees: yes/no`, `game_t tracks wall time: yes/no (<ratio>)`, `player dead bit: yes/no/skipped`, `DI-only mouse-look turns: yes/no/skipped`). Record the client and server build the run used; no account or character names.

Checks: D1. Report: the verdict lines; AX-31, AX-33, AX-42, AX-52 and AX-54 read them.

### AX-03 lab_login selects the login-screen server from the account file

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-03-login-combo` · **Worktree:** `lax-03` (from `main`)
- **Commit:** `fix(lab): lab_login picks the login-screen server the account file names (AX-03)`
- **Rules:** R1-R4, R6, R7, R8. Contract: [C17](contract.md#c17-login-screen-server-combo-ax-03). No tool is added (R9: say so in the report).

Why: the client's login screen preselects the first entry of a Lua hash table, which can be a dead server, and nothing in the lab sets it; the lab mints its telemetry token for the account file's server, so the client can log into a different one.

Files and changes:

1. New `crates/lab/src/supervisor/flows/login_combo.rs`:
   - `ComboPick`, `decide_combo`, `combo_rows_chunk` and `select_combo_chunk` exactly as C17 gives them. Quote names into Lua with the helper `select_server_chunk` (`flows/login.rs` l.129) already uses; grep it, never hand-roll escaping.
   - `combo_rows_chunk`: `pcall` around `Login_ServerTypeCombo:getItemCount()` and a loop over `getListboxItemFromIndex(i):getText()` for `i = 0 .. n-1`; returns each text as one result, or `"no_combo"` when the window or a call is missing.
   - `select_combo_chunk(name)`: `pcall` around `findItemWithText(name, nil)`; when that is nil or errors, the index loop comparing `getText()`; then `clearAllSelections()`, `setItemSelectState(item, true)`, `setText(name)`; returns `getSelectedItem():getText()` or `"none"`.
   - `impl Supervisor { pub(super) async fn pick_login_combo(&self, run: &mut FlowRun<'_>, creds: &ResolvedLogin) -> Result<Option<String>, FlowError> }`: run `combo_rows_chunk` as step `login_combo_rows`; `decide_combo(rows, creds.server.as_deref())`; `Skip` gives `Ok(None)`; `Error(msg)` gives `Err(run.fail("login_combo", msg))`; `Use(name)` runs `select_combo_chunk(&name)` as step `login_combo` and fails with C17's text when the read-back differs; on success `run.record("login_combo", t0, json!({"login_server": name, "rows": rows.len()}))` and `Ok(Some(name))`.
2. `flows/login.rs` (657 lines; at most 10 added): `mod login_combo;` wiring as the sibling modules are wired (grep `flows/mod.rs`; one line there instead if modules are declared there), then in `login_steps` inside `if screen == StartScreen::Login {` and before `type_login_credentials`: `let login_server = self.pick_login_combo(run, creds).await?;`. Keep `login_server` (an `Option<String>`, `None` outside the login screen) and add `"login_server": login_server` to the final `json!`.
3. Tests in a `#[cfg(test)] mod tests` in `login_combo.rs`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `no_rows_skips` | `[]` and any `want` give `Skip` | C17 row 1 |
| `one_row_is_used_whatever_the_name` | `["Alpha"]` with `Some("Beta")` and with `None` gives `Use("Alpha")` | C17 row 2 |
| `named_row_wins_case_insensitively` | `["Alpha","Beta"]`, `Some("beta")` gives `Use("Beta")` | C17 row 3 |
| `several_rows_and_a_miss_errors_with_rows` | `["Alpha","Beta"]`, `Some("Gamma")` gives `Error` containing `Gamma`, `Alpha` and `Beta` | C17 row 4: never the silent preselect |
| `several_rows_and_no_name_errors` | `["Alpha","Beta"]`, `None` gives `Error` naming both rows | C17 row 5 |
| `select_chunk_quotes_the_name` | a name containing `"` and `\` yields a chunk where the name appears only through the shared quoting helper's output | C17 |
| `select_chunk_reads_back` | the chunk contains `setItemSelectState`, `getSelectedItem` and `pcall` | C17 read-back |

Checks: L1, L2. Report: the `login.rs` line count, and "no tool added; policy.rs unchanged". The live check is AX-64's login row.

### AX-04 lab_client_start names or clears a running screensaver

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-04-screensaver` · **Worktree:** `lax-04` (from `main`)
- **Commit:** `fix(lab): detect a running screensaver by process before the launch probe (AX-04)`
- **Rules:** R1-R4, R6, R7, R8. Contract: [C18](contract.md#c18-screensaver-before-launch-ax-04). No tool is added (R9: say so in the report). Policy unchanged: a non-secure screensaver is killed today, a secure one is refused.

Files and changes (all in `crates/lab/src/supervisor/display.rs`, 266 lines; at most 90 added; `process.rs` is 669 lines and is not touched):

1. Factor the Toolhelp walk of `win::sgw_process_pids` (l.130-160) into `win::process_entries() -> Vec<(u32, String)>`; `sgw_process_pids` becomes `sgw_pids_in(process_entries())`.
2. Pure `pub fn scr_entries(entries: impl IntoIterator<Item = (u32, String)>) -> Vec<(u32, String)>`: keep entries whose image name ends in `.scr`, case-insensitive.
3. Pure `SaverAction` and `screensaver_action` exactly as C18 gives them, with C18's refusal text.
4. Pure `pub fn with_running_scr(msg: String, scr: &[(u32, String)]) -> String`: appends ` (running: <name> pid <pid>, ...)` when `scr` is non-empty, else returns `msg` unchanged.
5. `win::dismiss_screensaver` takes the pids to kill (`&[u32]`) instead of enumerating windows with `process::running_pids_where`; the one caller passes `Kill`'s pids.
6. `ensure_display_for_launch` (l.202): first `let scr = scr_entries(win::process_entries());` and act on `screensaver_action(&scr, win::screensaver_secure())`: `Refuse` returns the error at once; `Kill` kills, logs `"lab launch: dismissed a non-secure screensaver"` with the names, and settles as today; `None` continues. Keep the SPI path as a second check. A failed D3D probe's error goes through `with_running_scr` with a fresh `scr_entries(win::process_entries())`.
7. Tests in the existing `#[cfg(test)] mod tests` of `display.rs` (pure functions only; no test enumerates or kills real processes, R7).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `scr_entries_match_by_suffix` | `Ribbons.SCR` and `ribbons.scr` kept; `scrcons.exe`, `foo.scr.exe`, `SGW.exe` dropped | step 2 |
| `no_scr_no_action` | `screensaver_action(&[], false)` is `None` | C18 |
| `non_secure_scr_is_killed` | `[(4242, "Ribbons.scr")]`, not secure: `Kill(vec![4242])` | C18 |
| `secure_scr_is_refused_by_name_and_pid` | same, secure: `Refuse` whose text contains `Ribbons.scr` and `4242` | C18 |
| `probe_failure_names_running_scr` | `with_running_scr("x".into(), &[(7, "a.scr".into())])` contains `a.scr pid 7`; with `&[]` it is `"x"` | step 4 |

Checks: L1, L2. Report: the `display.rs` line count, and "no tool added; policy.rs unchanged". The live check is AX-64's screensaver row.
