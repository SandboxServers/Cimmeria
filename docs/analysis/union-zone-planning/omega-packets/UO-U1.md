# UO-U1: client walk of both Omega routes (lab UAT)

> **Type:** How-to (UAT packet) · **Wave:** W5 (R1-R4), W11 (R5) · **Status:** UATPending. **The owner runs this UAT in the lab. Agents never use the lab.**
> **Index:** [omega-work-packets.md](../omega-work-packets.md)

| Field | Value |
|---|---|
| Role | the owner runs it. The steps below are the owner's script; a Sonnet UAT lead (`claude` agent with the `lab-uat` skill) may grade the owner's logs and screenshots afterwards, without touching the lab |
| Host | `windows` |
| Size | five runs (Human, Free Jaffa, re-offer, services, R5 hub services of W8-W9); the R5 Asgard rows (W10) wait on F42 |
| Branch / worktree | none for driving. The results go to `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-U1-results.md` in a docs worktree `uo-u1`, which commits that file only. |
| Depends on | R1-R4: W1-W3 merged. R5: W8, W9 and UO-37..40 merged (all are, on the campaign branch). A server built from `main` at or after the last of those merges, with the lab pointed at it |
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
   | R5 hub services | 1 (any of 1-4 for S6), then 7; 5 once F42 exists | none required; at least 1 500 naquadah (Human), 500 (Free Jaffa) and 1 200 (Asgard), and 6 free bag slots | `Omega_Site`, `hub.arrival` |

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
| S3 | Click each Tau'ri vendor (R4 Human) and each Jaffa vendor (R4 Free Jaffa) | the store opens on the first click; buy one item | `vendor.open_store`, `vendor.purchase` spans. Selling back is refused by design (R5 S14) |
| S4 | Click one airman and one Free Jaffa Warrior | name shown, no prompt, nothing opens | no "interact:" dialog line for templates 63860/63861 |
| S5 | (boot) | | "DB-driven NPC population spawned (startup spaces)" `count` matches the W3 expectation |

### R5 hub services (W8, W9 and UO-37..40; W11)

Everything in R5 is in `Omega_Site` (world 18); points are (x, y, z) from the merged `spawnlist_union_omega.sql`, and the spawn id is the entity to click. Stand 3-4 m away and click each NPC once; every row needs visible feedback on that first click. Note the use-prompt icon each NPC shows (D-UO35). The Human tester covers the SGC wing and the Common Area, the Free Jaffa tester the Free Jaffa wing; both run S6. No row needs a mission.

**NPC points.**

| NPCs | Spawn (point) |
|---|---|
| SGC trainers (x -80, y -48.02) | 63804 Soldier (z -125), 63805 Scientist (z -128), 63806 Commando (z -131), 63807 Archaeologist (z -134) |
| Jaffa trainer | 63808 (306, -61.23, -104) |
| Auctioneer, Medic | 63819 (30, -44.91, -8), stock template 305; 63818 Castle Medic (39, -44.56, 7) |
| Human Weapons | 63841 T1 (-168.25, -36.42, -166), 63842 T2 (-166, -36.42, -164), 63843 T3 (-112, -47.82, -92), 63844 T4 (-112, -47.82, -95); Common Area extras 63845 T1 (82, -44.19, -2), 63846 T4 (-62, -45.84, -64) |
| Human Armor | 63847 T1 (-169.59, -36.42, -168.97), 63848 T2 (-166, -36.42, -169), 63849 T3 (-108, -47.82, -92), 63850 T4 (-108, -47.82, -95); extras 63851 T1 (78, -44.12, 8), 63852 T4 (-64, -45.75, -68) |
| Jaffa Weapons | 63853 T1 (390.35, -84.80, -44.56), 63854 T2 (387.85, -85.02, -44.56), 63855 T3 (384.85, -85.02, -45.56), 63856 T4 (381.85, -84.80, -44.56); extra 63857 T1 (30, -45.02, 0) |
| Jaffa Armor | 63859 T1 (216.50, -63.59, -66.17), 63860 T2 (213.50, -63.19, -66.17), 63861 T3 (210.50, -63.63, -66.17), 63862 T4 (207.50, -63.25, -66.17); extra 63863 T1 (48, -44.46, -2) |
| Crafting, Biomedical / Electronics / Power Systems / Materials | SGC 63960 (-87.97, -48.02, -99.76), 63961 (-85, -48.02, -100), 63962 (-83, -48.02, -98), 63963 (-78, -48.20, -99); Common 63964 (24, -37.17, -28), 63965 (26, -37.72, -30), 63966 (28, -37.12, -28), 63967 (26, -36.52, -26); Jaffa 63968 (291.14, -60.90, -109.35), 63969 (294.14, -61.23, -109.35), 63970 (297.14, -61.17, -110.85), 63971 (300.14, -61.23, -109.35) |
| Consumables | Common 63865 (56, -39.68, 20), 63866 (24, -44.57, -2), 63867 (52, -44.17, -2); SGC 63868 (-147, -38.88, -169.40), 63869 (-104, -47.82, -100), 63870 (-96, -48.03, -110); Jaffa 63871 (306, -61.23, -100), 63872 (310, -61.23, -104) |

**Steps.**

| # | Step | Visible result | Check (exact log text) |
|---|---|---|---|
| S6 | Click 63804-63808 with the Human tester, then with the Free Jaffa tester | the trainer window opens on the first click and lists the clicker's own tree at every trainer (all five use trainer list 1) | INFO target `abilities`, "Sent onTrainerOpen", `event="trainer_open"`, `list_id=1`, `trainer_template_id` = the spawn id; `archetype_id` and `offered`: 1 → 72, 2 → 64, 3 → 59, 4 → 65, 7 → 51. No WARN `trainer_empty_offering` or `trainer_offered_unbound` |
| S7 | Click the auctioneer 63819 | chat: "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)"; the window too, only with the client patch | INFO "fire_interact_tag: matched", `tag=UnionOmega_Auctioneer` (chain 63850); `onBMOpen` in the packet tap. A `bm.open_refused` row (chat "You are too far from the auctioneer.") within 5 m is a FAIL |
| S8 | Lose some health if a way exists (no hostile on the hub; check `.help`), then click the medic 63818; click again at full health | health refills; chat line from "Castle Medic": "Your injuries have been seen to, and you are patched up and ready to go." both times | per click, "fire_interact_tag: matched" `tag=UnionOmega_Medic`, then "Content: ChangeStat applied" (`stat_id=7`, `set_to_max=true`, `cur` = `max`, `chain_id=63851`), then "Content: npc bark" (`screen_id=63905`, `chain_id=63851`, `speaker="Castle Medic"`, `channel=say`). With no way to lose health, grade the refill half BLOCKED |
| S9 | Die near the Infirmary (as H9; BLOCKED if no self-damage command exists) | respawn at the Infirmary | INFO target `player.respawn`, `respawner_id=181`, `respawner_name="Omega Infirmary"` (39.00, -44.90, 3.00) |
| S10 | Click the eight crafting guides (table below) | chat line from "SGC Scientist": "Three vendors sell *X* Components: Human *X* Components in the SGC wing, Common *X* Components in the Common Area, Jaffa *X* Components in the Free Jaffa wing."; no window | "fire_interact_tag: matched" with the guide's own `tag`, then "Content: npc bark" with its `chain_id` and `screen_id`; no `trainer_open` row |
| S11 | None: D-UO32 (a) built no inn NPC and no battlemaster | | grade N/A; no spawn 63822 or 63823 exists |
| S12 | Click every vendor in the points table | the store opens on the first click | INFO target `vendor`, "vendor: store opened" (`event=store_opened`), `buy_count` per the vendor table; span `vendor.open_store`. `sell_count` is 0 for a bag without designs 21, 55 or 3437 (the finding below) |
| S13 | Buy the six items in the purchase table | each item lands in the bag; naquadah drops by its price | INFO "vendor: transaction committed", `action=buy`, `price`, `cash_before` − `cash_after` = price; span `vendor.purchase` |
| S14 | Drag one bought item back onto the store that sold it | chat "This vendor will not buy that item." | INFO "vendor: request refused", `reason=not_sellable`. If the client sends nothing, note it; not a FAIL |

**Guides (S10).** Two spawns share each chain; the clicked tag tells them apart.

| Spawn (point) | Tag | Chain | Screen, *X* |
|---|---|---|---|
| 63810 (32, -35.97, -22); 63817 (-123, -46.43, -122) | `UnionOmega_Guide_Biology`; `UnionOmega_Guide_AdvBiology` | 63852 | 63901, Biomedical |
| 63811 (80, -31.90, 24); 63812 (11.89, -43.87, 45.20) | `UnionOmega_Guide_Electronics`; `UnionOmega_Guide_AdvElectronics` | 63853 | 63902, Electronics |
| 63813 (80, -32.51, 27); 63814 (63.37, -44.10, 43.17) | `UnionOmega_Guide_Engineering`; `UnionOmega_Guide_AdvEngineering` | 63854 | 63903, Power Systems |
| 63815 (44, -36.06, -22); 63816 (-125, -46.57, -120) | `UnionOmega_Guide_Physics`; `UnionOmega_Guide_AdvPhysics` | 63855 | 63904, Materials |

**Vendors (S12).** Sell, repair and recharge lists are list 2 on every Omega vendor; `repair_count` and `recharge_count` are not graded.

| Role | Templates → buy lists | `buy_count` |
|---|---|---|
| Human Weapons T1-T4 | 63812-63815 → 63800-63803 | 20 each |
| Human Armor T1-T4 | 63816-63819 → 63804-63807 | 18 each |
| Jaffa Weapons T1-T4 | 63831-63834 → 63808-63811 | 20 each |
| Jaffa Armor T1 / T2 / T3 / T4 | 63835-63838 → 63812-63815 | 17 / 16 / 14 / 15 |
| Crafting (SGC 63820-63823, Common 63824-63827, Jaffa 63839-63842) | Biomedical → 63817, Electronics → 63818, Power Systems → 63819, Materials → 63820 | 20 each |
| Consumables | 63828 → 63816 | 20 |

**Purchases (S13).** Prices follow D-UO29 (weapons 200 × tech comp, armor 100 ×, crafting 25 ×).

| Tester | Vendor | Item | Price |
|---|---|---|---|
| Human | 63841 Human Weapons T1 | AR 21 Assault Rifle | 1 000 |
| Human | 63847 Human Armor T1 | BDU Boots | 100 |
| Human | any Consumables | Health Slappack TC1 | 25 |
| Human | 63960 SGC Biomedical | BMAS-5 Field Crafting Tool | 125 |
| Free Jaffa | 63853 Jaffa Weapons T1 | Serpent Staff | 200 |
| Free Jaffa | 63859 Jaffa Armor T1 | Standard Boots | 100 |

**Finding (sell list 2).** List 2 buys only designs 21, 55 and 3437, and no Omega vendor stocks them, so nothing bought on the hub sells back. S14 checks the refusal line, not a sale.

### R5 Asgard rows (W10; wait on F42)

These need an Asgard (archetype 5) character, which waits on a playable Asgard start (F42); grade them BLOCKED until one exists. Any tester may click the NPCs earlier as a store-open check, graded as S12 rows (the trainer then shows the tester's own tree).

| # | Step | Visible result | Check |
|---|---|---|---|
| A1 | Click the Asgard trainer 63809 (-39, -45.02, 21) | the Asgard tree on the first click | "Sent onTrainerOpen", `trainer_template_id=63809`, `list_id=1`, `archetype_id=5`, `offered=66` |
| A2 | Click Programs 63824-63827 (templates 63843-63846 → lists 63821-63824), Armor 63828-63831 (63847-63850 → 63825-63828) and Crafting 63832-63835 (63851-63854 → 63817-63820); all at y -45.02 near (-36, 24) | the store opens on the first click | "vendor: store opened", `buy_count` 20 (Programs), 14 (Armor), 20 (Crafting) |
| A3 | Buy the Warrior Attack Program (600) at 63824 and the Fyrstyr Absorption Vest (500) at 63828 | naquadah drops by the price | "vendor: transaction committed", `action=buy` |
| A4 | The Council hearing and the 481 offer (UO-41, UO-42) | | rows join this packet when those packets merge |

## SigNoz

For anything older than the 500-line log tail, run these filters per the `telemetry-triage` skill: `service.name` of the lab server, body contains each quoted string, time range = the run. Paste each filter and its hit count in the results file.

## Results file

`worknotes/UO-U1-results.md`, CRLF: one table per run with row id, PASS / FAIL / BLOCKED, the evidence line (log text or span name with its timestamp), and the screenshot path for any FAIL. List every FAIL as a proposed fix packet. State plainly which rows were never run.

## Return

`UO-U1 <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-u1\.claude\reports\union-omega\UO-U1-lead.md`
