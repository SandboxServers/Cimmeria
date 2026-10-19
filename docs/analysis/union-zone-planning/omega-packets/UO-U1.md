# UO-U1: client walk of both Omega routes (lab UAT)

> **Type:** How-to (UAT packet) · **Wave:** W5 · **Status:** UATPending. **Ask the user before any lab use, and wait for a yes.**
> **Index:** [omega-work-packets.md](../omega-work-packets.md)

| Field | Value |
|---|---|
| Role | the orchestrator asks the user; a Sonnet UAT lead (`claude` agent with the `lab-uat` skill) prepares, verifies and grades; Haiku `lab-driver` agents drive, one run per route, each under ~100k tokens |
| Size | four driver runs (Human, Free Jaffa, re-offer, services) |
| Branch / worktree | none for driving. The lead writes results to `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-U1-results.md` in a docs worktree `uo-u1`, and commits that file only. |
| Depends on | W1-W3 merged; W4 merged for the vendor and executed-test rows; a server built from `main` at or after the W4 merge, with the lab pointed at it |
| Ship order | W5, before UO-99 |

## Why

Every Omega test is resolve-only, seed-only or executor-level. None proves what the client shows. The campaign's last stop condition, "one click shows two dialogs", and the visible-feedback rule are client facts. The route is done when this packet has walked both factions (ledger § Provenance and stop conditions).

## Rules (binding)

1. **Ask first.** The orchestrator asks the user with AskUserQuestion and waits for a yes before any `lab_lease_acquire`, client start or server change. The lab is shared, and its input hooks swallow the user's own clicks.
2. Read the `lab-uat` skill (`.claude/skills/lab-uat/SKILL.md`) and `docs/guides/live-research-lab.md` before preparing. Front-load the gotcha sheet from that skill in every driver brief.
3. One lease per run, released at the end. Never `force` a takeover.
4. A driver's summary is a claim. The lead checks every step against `server_log_tail`, the packet tap or SigNoz before grading it.
5. Stand 3-4 m from each NPC (server range 5.0 m), with open floor behind you; click by `entity_id`.
6. No IPs, credentials or account names in any file or report. Name characters by role ("Human tester"), not by account.
7. A FAIL becomes a fix packet in the ledger, sliced by the architect. The lead never edits seeds or code.
8. Docs are CRLF; convert the results file with `[IO.File]::WriteAllText(p, ([IO.File]::ReadAllText(p)).Replace("`r`n","`n").Replace("`n","`r`n"))`.
9. No `git stash`, no `git worktree prune`, no push.

## Preparation (the lead, before any driver)

1. Read the merged seeds: `db/resources/Content/Seed/union_omega_*_chains.sql`, `db/resources/Worlds/Seed/spawnlist_union_omega.sql` and UO-01's worknote. Note the real coordinates of Modi (spawn 78), the C4 arrival point, the cast in world 80 and every service NPC.
2. Stage each tester with `server_stage_fixture` (lab-fixtures: missions, world and position applied at Play; fixture writes are silent, D-FX4):

   | Run | Archetype | Missions | World and position |
   |---|---|---|---|
   | R1 Human | 1 (any 1-4) | 1655 `active`, step 4940 `active` | `Omega_Site`, the `hub.arrival` point |
   | R2 Free Jaffa | 7 | 1655 `active`, step 4940 `active` | `Omega_Site`, `hub.arrival` |
   | R3 re-offer | 1 | 1655 `completed` (its S4942 key may stay `active`); 1423 **not held** | `Omega_Site_CmdCenter`, respawner 26 (10.0, 35.23, 9.987) |
   | R4 services | 1, then 7 | none required | `Omega_Site`, `hub.arrival` |

   If `lab_create_character` cannot make archetype 7, say so and mark R2 BLOCKED, not FAIL.
3. Start a packet tap for each run (`server_packet_tap_start`).

## Driver steps and the evidence to check

Each row is one driver step. "Check" is what the lead verifies in the server log or SigNoz before grading the row PASS.

### R1 Human (archetypes 1-4)

| # | Driver step | Visible result | Check (exact log text) |
|---|---|---|---|
| H1 | Play into `Omega_Site` | in world near the gate | "fire_player_loaded: matched" (63810); "Content: adding dialog set" (7015, slot 65) |
| H2 | Click Modi | one dialog (6168) on the first click | "interact: per-player dialog set → onDialogDisplay", once |
| H3 | Close Modi's dialog | it closes | "fire_dialog_choice: matched" (63811); "Content: removing dialog set" |
| H4 | Walk onto transport pad 14 (an Asgard beam that uses the ring-transport dispatch; `docs/content/zone-audit.md` § Omega_Site) and pick the Command Center | arrive in world 80 by pad 17 | "fire_teleport_in: matched" (63801) and (63820); "Content: advancing step" (1655 → 4942); "Content: adding dialog set" (map 63800, slot 63800) with `interactions_count = 1` |
| H5 | Click Jackson | one dialog (4641) on the first click | one "interact: per-player dialog set → onDialogDisplay"; Bra'tac, Teal'c and Thor show no use prompt |
| H6 | Log out and back in, in world 80 | Jackson still offers | "Content: adding dialog set" with `interactions_count = 1` (never 2) |
| H7 | Accept on 4641's last screen | the dialog closes; the mission tracker shows 1423 | "fire_dialog_choice: matched" (63822); "Content: removing dialog set" **before** "Content: completing mission" (1655); "Content: accepting mission" (1423); "Content: granting stargate address" (6). "Content: accept_mission refused by offer guard — skipping persist + follow-up event" is absent |
| H8 | Open the stargate dial list | Beta's address is listed | the address-grant row in the packet tap |
| H9 | On a second Human, die on the hub. The legacy `.kill` GM command is not registered (`docs/guides/unified-uat.md:905`) and the hub has no hostile NPC, so the lead first checks `.help` for a self-damage command; with none, mark H9 BLOCKED (UO-03's seed test still covers the row) | respawn on the gate plaza | INFO target `player.respawn`, "Respawn: no usable respawner was named -- using the one nearest the death position", `respawner_id=180`, `respawner_name="Omega Site Gate Plaza"` |

Across R1: no "Content: completing mission" line for 1655 appears before H7 (F21).

### R2 Free Jaffa (archetype 7)

The same rows as R1, with Bra'tac (map 63801, slot 63801, dialog 5025, chains 63830-63832) and mission 1500. Extra rows:

| # | Driver step | Visible result | Check |
|---|---|---|---|
| J1 | Click Jackson in world 80 | no dialog, no prompt | no "interact:" line naming slot 63800 |
| J2 | After the Accept | the tracker shows 1500, never 1423 | "Content: accepting mission" (1500) only |

### R3 re-offer

| # | Driver step | Visible result | Check |
|---|---|---|---|
| O1 | Play into world 80 with 1655 done and 1423 not held | Jackson shows an "available" marker | "fire_player_loaded: matched" (63840); "Content: adding dialog set" (63802) |
| O2 | Accept 4641 | 1423 is back in the tracker | "fire_dialog_choice: matched" (63842); "Content: accepting mission" (1423); no "Content: completing mission"; no gate grant |
| O3 | Abandon 1423 from the mission log, in world 80 | Jackson offers again | "Mission abandoned", then "fire_mission_abandoned: matched" (63841), then "Content: adding dialog set" (63802) |

### R4 services and ambient (needs W2 and W3; vendors need W4)

| # | Driver step | Visible result | Check |
|---|---|---|---|
| S1 | Click the banker | the vault opens on the first click | `bank.banker_interact` span |
| S2 | Click the Team registrar, then the Command registrar | `CreateTeamWin`, then `CreateCommandWin` | `org.registrar_open` span and its INFO outcome row, twice |
| S3 | Click each Tau'ri vendor (R4 Human) and each Jaffa vendor (R4 Free Jaffa) | the store opens on the first click; buy one item, sell it back | `vendor.open_store`, `vendor.purchase`, `vendor.sell` spans |
| S4 | Click one airman and one Free Jaffa Warrior | name shown, no prompt, nothing opens | no "interact:" dialog line for templates 63860/63861 |
| S5 | (boot) | | "DB-driven NPC population spawned (startup spaces)" `count` matches the W3 expectation |

## SigNoz

For anything older than the 500-line log tail, run the lead's filters per the `telemetry-triage` skill: `service.name` of the lab server, body contains each quoted string, time range = the run. Paste each filter and its hit count in the results file.

## Results file

`worknotes/UO-U1-results.md`, CRLF: one table per run with row id, PASS / FAIL / BLOCKED, the evidence line (log text or span name with its timestamp), and the screenshot path for any FAIL. List every FAIL as a proposed fix packet. State plainly which rows were never run.

## Return

`UO-U1 <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-u1\.claude\reports\union-omega\UO-U1-lead.md`
