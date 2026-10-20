# UO-U1: client walk of both Omega routes (lab UAT)

> **Type:** How-to (UAT packet) · **Wave:** W5 (R1-R4), W11 (R5) · **Status:** UATPending. **The owner runs this UAT in the lab. Agents never use the lab.**
> **Index:** [omega-work-packets.md](../omega-work-packets.md)

| Field | Value |
|---|---|
| Role | the owner runs it. The steps below are the owner's script; a Sonnet UAT lead (`claude` agent with the `lab-uat` skill) may grade the owner's logs and screenshots afterwards, without touching the lab |
| Host | `windows` |
| Size | five runs (Human, Free Jaffa, re-offer, services, R5 hub services of W8-W9) |
| Branch / worktree | none for driving. The results go to `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-U1-results.md` in a docs worktree `uo-u1`, which commits that file only. |
| Depends on | R1-R4: W1-W3 merged. R5: the W8 and W9 PRs merged. A server built from `main` at or after the last of those merges, with the lab pointed at it |
| Ship order | R1-R4 in W5; R5 in W11, before UO-98 |

## Why

Every Omega test is resolve-only, seed-only or executor-level. None proves what the client shows. The campaign's last stop condition, "one click shows two dialogs", and the visible-feedback rule are client facts. The route is done when this packet has walked both factions (ledger § Provenance and stop conditions).

## Rules (binding)

1. **The owner runs the lab.** No agent calls `lab_lease_acquire`, starts a client or changes the lab server for this packet. The lab is shared, and its input hooks swallow the owner's own clicks. An agent only prepares the fixtures list and grades results the owner hands over.
2. Read the `lab-uat` skill (`.claude/skills/lab-uat/SKILL.md`) and `docs/guides/live-research-lab.md` before preparing. Keep the gotcha sheet from that skill at hand during the run.
3. One lease per run, released at the end. Never `force` a takeover.
4. A summary is a claim. Whoever grades checks every step against `server_log_tail`, the packet tap or SigNoz before grading it.
5. Stand 3-4 m from each NPC (server range 5.0 m), with open floor behind you; click by `entity_id`.
6. No IPs, credentials or account names in any file or report. Name characters by role ("Human tester"), not by account.
7. A FAIL becomes a fix packet in the ledger, sliced by the architect. The grader never edits seeds or code.
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
   | R5 hub services | 1, then 7 (each archetype the trainers serve, where a character exists) | none required; at least 5 000 naquadah for S12 | `Omega_Site`, `hub.arrival` |

   If `lab_create_character` cannot make archetype 7, say so and mark R2 BLOCKED, not FAIL.
3. Start a packet tap for each run (`server_packet_tap_start`).

## Steps and the evidence to check

Each row is one step. "Check" is what the grader verifies in the server log or SigNoz before grading the row PASS.

### R1 Human (archetypes 1-4)

| # | Step | Visible result | Check (exact log text) |
|---|---|---|---|
| H1 | Play into `Omega_Site` | in world near the gate | "fire_player_loaded: matched" (63810); "Content: adding dialog set" (7015, slot 65) |
| H2 | Click Modi | one dialog (6168) on the first click | "interact: per-player dialog set → onDialogDisplay", once |
| H3 | Close Modi's dialog | it closes | "fire_dialog_choice: matched" (63811); "Content: removing dialog set" |
| H4 | Walk onto transport pad 14 (an Asgard beam that uses the ring-transport dispatch; `docs/content/zone-audit.md` § Omega_Site) and pick the Command Center | arrive in world 80 by pad 17 | "fire_teleport_in: matched" (63801) and (63820); "Content: advancing step" (1655 → 4942); "Content: adding dialog set" (map 63800, slot 63800) with `interactions_count = 1` |
| H5 | Click Jackson | one dialog (4641) on the first click | one "interact: per-player dialog set → onDialogDisplay"; Bra'tac, Teal'c and Thor show no use prompt |
| H6 | Log out and back in, in world 80 | Jackson still offers | "Content: adding dialog set" with `interactions_count = 1` (never 2) |
| H7 | Accept on 4641's last screen | the dialog closes; the mission tracker shows 1423 | "fire_dialog_choice: matched" (63822); "Content: removing dialog set" **before** "Content: completing mission" (1655); "Content: accepting mission" (1423); "Content: granting stargate address" (6). "Content: accept_mission refused by offer guard — skipping persist + follow-up event" is absent |
| H8 | Open the stargate dial list | Beta's address is listed | the address-grant row in the packet tap |
| H9 | On a second Human, die on the hub. The legacy `.kill` GM command is not registered (`docs/guides/unified-uat.md:905`) and the hub has no hostile NPC, so the owner first checks `.help` for a self-damage command; with none, mark H9 BLOCKED (UO-03's seed test still covers the row) | respawn on the gate plaza | INFO target `player.respawn`, "Respawn: no usable respawner was named -- using the one nearest the death position", `respawner_id=180`, `respawner_name="Omega Site Gate Plaza"` |

Across R1: no "Content: completing mission" line for 1655 appears before H7 (F21).

### R2 Free Jaffa (archetype 7)

The same rows as R1, with Bra'tac (map 63801, slot 63801, dialog 5025, chains 63830-63832) and mission 1500. Extra rows:

| # | Step | Visible result | Check |
|---|---|---|---|
| J1 | Click Jackson in world 80 | no dialog, no prompt | no "interact:" line naming slot 63800 |
| J2 | After the Accept | the tracker shows 1500, never 1423 | "Content: accepting mission" (1500) only |

### R3 re-offer

| # | Step | Visible result | Check |
|---|---|---|---|
| O1 | Play into world 80 with 1655 done and 1423 not held | Jackson shows an "available" marker | "fire_player_loaded: matched" (63840); "Content: adding dialog set" (63802) |
| O2 | Accept 4641 | 1423 is back in the tracker | "fire_dialog_choice: matched" (63842); "Content: accepting mission" (1423); no "Content: completing mission"; no gate grant |
| O3 | Abandon 1423 from the mission log, in world 80 | Jackson offers again | "Mission abandoned", then "fire_mission_abandoned: matched" (63841), then "Content: adding dialog set" (63802) |

### R4 services and ambient (needs W2 and W3; vendors need W4)

| # | Step | Visible result | Check |
|---|---|---|---|
| S1 | Click the banker | the vault opens on the first click | `bank.banker_interact` span |
| S2 | Click the Team registrar, then the Command registrar | `CreateTeamWin`, then `CreateCommandWin` | `org.registrar_open` span and its INFO outcome row, twice |
| S3 | Click each Tau'ri vendor (R4 Human) and each Jaffa vendor (R4 Free Jaffa) | the store opens on the first click; buy one item, sell it back | `vendor.open_store`, `vendor.purchase`, `vendor.sell` spans |
| S4 | Click one airman and one Free Jaffa Warrior | name shown, no prompt, nothing opens | no "interact:" dialog line for templates 63860/63861 |
| S5 | (boot) | | "DB-driven NPC population spawned (startup spaces)" `count` matches the W3 expectation |

### R5 hub services (W8 and W9; W11)

Spawn ids and points come from the merged `spawnlist_union_omega.sql`. Stand 3-4 m away and click each NPC once; every row needs visible feedback on that first click. Note the use-prompt icon each NPC shows (D-UO35).

| # | Step | Visible result | Check |
|---|---|---|---|
| S6 | Click each class trainer (63804-63808) with a Human, then with a Free Jaffa | the trainer window opens on the first click, listing the clicker's own tree | INFO target `abilities`, `event="trainer_open"`, `list_id=1`, the clicker's `archetype_id`; no WARN `trainer_empty_offering` |
| S7 | Click the auctioneer (63819) | the chat line about the Black Market; the Black Market window too, only with the client patch | "fire_interact_tag: matched" (63850); `onBMOpen` in the packet tap |
| S8 | Lose some health, then click the medic (63818); click again at full health | health refills; the medic's line in chat both times | "Content: ChangeStat applied", then "Content: npc bark" (screen 63905), each click |
| S9 | Die near the Infirmary (as H9; BLOCKED if no self-damage command exists) | respawn at the Infirmary | INFO target `player.respawn`, `respawner_id=181`, `respawner_name="Omega Infirmary"` |
| S10 | Click each crafting guide (63810-63817) | a chat line naming three vendors of that specialty; no window | "Content: npc bark" (screens 63901-63904); no `trainer_open` |
| S11 | Only if D-UO32 is (b): click the inn NPC (63822) and the battlemaster (63823) | one chat line each saying what the building is; nothing opens | "Content: npc bark" (63906, 63907) |
| S12 | Click every W8 vendor (Human and Jaffa Weapons and Armor, with the extra-spot spawns; the 12 crafting vendors; the consumables vendors); buy one item from one vendor per list and sell it back | the store opens on the first click | `vendor.open_store` span; INFO `vendor` `store_opened` whose `buy_count` equals that list's row count (the stock worknotes' § Store-open proof); `vendor.purchase` and `vendor.sell` spans |

The Asgard trainer and vendors (W10) wait for a playable Asgard start (F42); their client rows join this packet when one exists.

## SigNoz

For anything older than the 500-line log tail, run these filters per the `telemetry-triage` skill: `service.name` of the lab server, body contains each quoted string, time range = the run. Paste each filter and its hit count in the results file.

## Results file

`worknotes/UO-U1-results.md`, CRLF: one table per run with row id, PASS / FAIL / BLOCKED, the evidence line (log text or span name with its timestamp), and the screenshot path for any FAIL. List every FAIL as a proposed fix packet. State plainly which rows were never run.

## Return

`UO-U1 <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-u1\.claude\reports\union-omega\UO-U1-lead.md`
