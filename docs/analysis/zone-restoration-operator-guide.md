# Zone Restoration: Operator Test Guide

> Type: how-to. Audience: the human operator (game owner) running in-client UAT. Written 2026-09-18 during the weekly-usage crunch, from the three coordinator sessions' own status reports. Nothing below has been run in a client yet; every "expected" line is what the code and tests say should happen, not an observed result.

Companions: [Cellblock handoff](castle-cellblock-rebuild/handoffs/session-resume.md), [Castle handoff](castle-rebuild/handoffs/session-resume.md), [Harset handoff](harset-rebuild/handoffs/session-1-resume.md), and each campaign's `README.md` for its full UAT milestone list.

> **Step-by-step runbook** for every milestone (action, expected result, what to report) is the last section of this file: [Step-By-Step UAT Runbook](#step-by-step-uat-runbook). Use it when you sit down to test.

## Before You Test Anything

1. **Which build.** Only merged work is on `main`. Open PRs are testable only by building that branch (the PR number and branch are named per zone below). You build and run with `setup.ps1` as usual.
2. **Reseed the DB.** All zone content is seed SQL under `db/resources/`. After you pull or switch branch, reload the play database from `db/database.sql`. The `sgw_harset` database on `:5433` is the coordinator's test DB, not yours.
3. **Postgres.** The shared Postgres on `:5433` crashed at 10:24 on 2026-09-18 and was restarted at about 11:20 (`pg_ctl -D server/pgdata -o "-p 5433"`); no data was lost. If a live-DB test run fails oddly, check it is up.
4. **GM console.** Placement and travel helpers are the `.` console commands documented in [commands.md](../commands.md).
5. **Relog at every step.** Most of the defects these campaigns have found are "correct until you relog". Each checklist below says where.
6. **Report format.** For any failure note: zone, step, what you saw, what you expected, and the server log lines around it. That is enough for a fresh session to start from.

## What Is Testable Now

| Zone | On `main` | Open PRs (testable from branch) | Testable today |
|---|---|---|---|
| Castle Cellblock | C00-C05, C06, C07, C08a, C08b, GC1b-0, GC1 (#655), C06 flank objectives (#671) | none | **Yes: M1-M3 and GC1 on `main`.** The flank check may be unreachable in play (see A12) |
| Castle (World 8, ring platform) | CA00 respawners (#651), CA01 + CA03 mission 701 and the dialog-speaker pin fix (#659), CA10 gate open/cross events (#663), CA04 minigame hardening (#652), CA02 "!" dialog bind (#661), CA05 story actors and point sets (#667), missions 706 and 708 (#668), missions 702-704 (#660) | none (docs closeout PR #669 only) | **Yes, M1-M5 on `main` after a rebuild** (the gate leg into Harset excepted, see M4) |
| Harset | Waves 1 and 2 (#662, #682) | placement PR `harset/placement` (guessed placements; see the ledger) | Travel, combat and population checks (C1, C4, C6) on `main`; placement checks (C5) once the PR merges |

## Suggested Order If Budget Is Tight

Highest value per minute first.

1. **Cellblock M1-M3 on current `main`.** The largest body of already-merged content and it needs nothing else to land first. Its own 24+ scenario guide with a results table is [castle-cellblock-rebuild/uat-guide.md](castle-cellblock-rebuild/uat-guide.md); use that for the full pass and this file for the quick one.
2. **Castle respawn check on `main`.** Five minutes, catches bad coordinates.
3. **Harset M1 checks** once #662 merges (or from its branch). Cheap, and tells us whether ring and door travel is safe.
4. **Castle M1** (mission 701). All its PRs (#659, #652, #661) are on `main`, so this is testable now after a rebuild.
5. **Harset M0 placement session.** The longest job and the critical path for every Harset mission, but it needs the least Claude budget until the final commit of the generated seed SQL, so it can run whenever you have the time.

## Castle Cellblock (Castle_CellBlock)

**Implemented (merged).** GC1 (#655, `627be660`): Marsh escort dialogs, ring-hop and topside follow, and the post-death blurb 5859 on mission 686 completion. Also: Prisoner 329 and Marsh briefing dialogs; hallway controllers; Region8 guard aggro; the Stasis Sickness icon; Frost's Letter (mission 1360); the take-cover objective 2484 with its cover indicator; accept-blurbs for missions 640, 641, 680 and 688; the Straegis camera scene (Matinee, Marsh despawns, dialog 2516).
**Open.** Only a docs PR (#666, ledger and handoff). The 147-test chain-replay suite passed on a fresh DB before the GC1 merge.
**Not built.** GC1c lockdown VFX (needs client evidence), GC2 (unscoped), GC3 XP formula (needs your decision).

Test with a Jaffa character and a Human character, relogging at each step:

| Milestone | Steps | Expected |
|---|---|---|
| M1 | Load in; talk to Prisoner 329 and Marsh; walk the hallway controllers; enter Region8; observe the Stasis Sickness icon, then cure it | Exactly one topic dialog from Prisoner 329 and one Marsh briefing; each controller accepts once; the pistol guard aggros on Region8 entry; the icon shows on load and clears on cure |
| M2 | Check the mission log; pick up the vial; take cover; try the flank objectives (2725, 2731, C06, #671) and step 2144 | Frost's Letter is in the log; the cover indicator shows on vial pickup and hides in cover; step 2144 needs both objectives. The flank check may be unreachable in play (it needs a guard already holding a cover slot, and slots are only reserved while you are out of weapon range with cover data nearby), but the mission still completes on kills either way |
| M3 | Accept each mission; trigger the Straegis scene; relog afterwards | One prompt per accept; the camera plays once, control returns, Marsh is gone, dialog 2516 shows once, then 5859 about 10.6 s after the scene starts; the scene does not replay after relog |
| GC1 (on `main`) | After the ring hop | Marsh follows you topside; dialog 5859 shows about 10.6 s after the Straegis scene starts |

**Watch for:** the open invisible-corpse bug (issue #582: a corpse stays invisible until relog). Look for the same failure on Marsh's spawn after the ring hop. M4 (arriving near Gerschon with mission 1360 active) depends on the Castle campaign.

## Castle (World 8, ring platform)

**Implemented and merged: every implementable Castle packet, CA00-CA10.** CA00 world-8 respawners (#651; all four coordinates provisional, MEDIUM confidence); CA01 + CA03 mission 701 and the dialog-speaker pin fix (#659); CA02 "!" bind (#661); CA04 minigame hardening (#652); CA05 story actors, point sets 2082-2085 and respawn timers (#667; Zuritska is male); CA10 gate events (#663); missions 706 and 708 (#668); missions 702-704 (#660). Hostile Castle mobs respawn after 120 s and region boxes have ceilings.
**PRs (all merged):**

| PR | Content | State |
|---|---|---|
| #652 | CA04 minigame hardening; `start_minigame` difficulty | **merged to `main`** (`5c354b1e`) |
| #659 | CA01 + CA03 mission 701 (Gerschon, Copplemann, Livewire) and the dialog-speaker pin fix | **merged to `main`** (`f930b4cb`) |
| #661 | CA02 flag-only dialog bind (the "!" over Gerschon) | **merged to `main`** (`ddbc873e`) |
| #663 | CA10 stargate open/cross events (6100, 6113, 4 s dial timer); splits `gate_travel.rs` | **merged to `main`** (`a9d0fad7`) |
| #667 | CA05 story actors, point sets 2082-2085, respawn timers | **merged to `main`** (`364e736d`) |
| #668 | Missions 706 and 708 chains (includes four glow-clear removals for shared NPCs) | **merged to `main`** (`bb47f526`) |
| #660 | Missions 702-704 (Zuritska, Romney) | **merged to `main`** (`2a37ba4c`) |

**Still to do:** closeout docs PR #669 and CA16 doc sync. **Design-gated, not started:** CA13, CA14, CA15.

| Milestone | Steps | Expected |
|---|---|---|
| Respawn (`main` now) | Fresh character on Castle; die once at each of the four checkpoints | You land at a real checkpoint, not the origin and not under the floor. Report any bad coordinate |
| M1 (#659, #652 and #661 all merged: testable now) | Arrive on the ring platform with mission 688 complete and 1360 active | "!" over Gerschon before accepting (Human sees indicator 2573, Jaffa 5861); 701 accepts once and a second interaction does not re-accept; Copplemann advances the step; the Livewire win shows 2575 once; 2576 completes 701 and accepts 702 and 703; relog at each step re-shows the right indicator |
| M2/M3 (on `main`) | Find Zuritska (male, at 268.0 / 66.79 / 1042.59) and Romney | Both exist at their positions for every player; freeing Zuritska completes 702 once; killing Romney completes 703 |
| M4 (all Castle and Harset gate PRs merged) | Win the DHD Livewire, dial Harset, walk into the gate | The gate opens about 4 s after the dial; the crossing plays 6113; you arrive in Harset at the gate's raw coordinate (runbook C4). World 57 is `advisory`, so the arrival is accepted unvalidated; the pinned plaza arrival comes with the placement PR | Whether the gate opened and when, whether you arrived, where you stood, and whether you could move |
| M5 | Two players at different steps | Neither disturbs the other's indicators or actors |

**Untested and provisional:** every coordinate (comms room, Armory prefab, and the Op-Core respawner, which could not be located in the map assets, so it is a reconstruction). Issue #657 (`active_objective_ids` lost on relog, affects every mission) is unfixed; Harset packet H50 is a fix in progress.

## Harset (worlds 57, 68, 69, 70)

**Situation.** Harset was an empty shell. Wave 1 (#662) and wave 2 (#682) are on `main`; the placement PR (`harset/placement`) adds best-guess coordinates for everything that used to wait for an in-client placement walk. **Nothing has been seen in a client.** The single ledger of every guessed coordinate, with evidence class and confidence, and the list of things there is no evidence for, is [harset-rebuild/placements/README.md](harset-rebuild/placements/README.md).

| Layer | What you would see |
|---|---|
| Travel (wave 1 and 2) | Gate arrival validated (advisory in world 57); ring switches with timeouts; Command Center door 6006; DHD glyph; dialing refused for an address you do not know; the Castle DHD Livewire grants Harset's address |
| Combat and NPCs (wave 2) | Stationary guards turn to face you; sentries that could never fire now can; surrendered NPCs stop the attack loop; NPCs use a ranged and a melee ability |
| Missions (wave 2 seeds) | Chains for missions 1324, 1326, 1360/567, 1361, 1200 and 742; objective state survives relog |
| Placements (this PR) | Gate-3 arrival pin at (-5.0, -68.99, 33.0); respawners for worlds 57, 69 and 70; return door 6007 open; 15 world-57 spawns and 8 named regions; 10 Command Center spawns and 3 interior regions; 15 encounter-anchor candidates (ledger only, no chains) |

**Not placed (no evidence):** the Market and Storage door pairs, the Bar, the Holding Pens, vendor and trainer props, Lethander's stall, the 1243 and 1362 anchors, the sarcophagus and lab consoles, and ambient props in worlds 69 and 70. The reasons, and what would unblock each, are in the ledger.

**One systemic problem to know about:** `harset.nav` is wrong for the upper quarters of world 57 (no on-mesh point at the real floor height; four of the five authored ring pads are off-mesh even though their rows are correct). World 57 runs in advisory mode, which is what keeps it playable. A rebuilt mesh exists but loses 12 positions real players stood on, so it is not a drop-in yet.

**Untested mission packets:** the mission packets H23-H28, H32-H37 and H42-H46 and H05 are not written yet. Mission chains that exist can only be exercised where their NPCs now stand (see C5).

## Known Cross-Cutting Risks

- **Relog defects.** #657 (objective state lost on relog) is **fixed** by H50 (2026-09-19): per-objective status now round-trips through `sgw_mission`, and rows written before the fix repair themselves on first login, so no DB cleanup is needed. "Resumes correctly after relog" is a meaningful UAT check again. Two caveats: per-objective **counters** are still session-only, and a same-world **respawn** does not re-send the mission log (playtest finding H8) — only a full relog does.
- **Invisible corpse after spawn** (#582). Seen at Castle Cellblock; watch for it on any NPC spawned mid-session.
- **Every coordinate is provisional** in Castle (CA00, comms room, Armory, Op-Core) and unpinned in Harset.
- **Ring timeout values** are judgement, since the 2009 server had none.

## Where Each Session Leaves Off

| Session | Handoff document |
|---|---|
| Cellblock | [castle-cellblock-rebuild/handoffs/session-resume.md](castle-cellblock-rebuild/handoffs/session-resume.md) |
| Castle | [castle-rebuild/handoffs/session-resume.md](castle-rebuild/handoffs/session-resume.md) |
| Harset | [harset-rebuild/handoffs/session-3-resume.md](harset-rebuild/handoffs/session-3-resume.md) (newest; sessions 1 and 2 sit beside it) |

Each coordinator was asked to refresh its handoff before it runs out of budget. Treat this guide as a snapshot: if a PR merged after 2026-09-18, the handoff is more current.

## Step-By-Step UAT Runbook

Work top to bottom inside a zone. A step passes only if the expected result happens **and** it still holds after a relog where the step says so. Coordinates are given only where a session recorded one; where a location is not known, finding the thing is part of the test, and "could not find X" is a valid report.

**Report template (paste this per failure):** `Zone / milestone / step` - `character (Human or Jaffa, level)` - `saw:` - `expected:` - `server log lines around the time` - `did relog fix it (yes/no)`.

### A. Castle Cellblock (build: current `main`; use one Jaffa and one Human character)

This is the quick pass. The Cellblock session's full guide, with 24+ numbered scenarios and a results table, is [castle-cellblock-rebuild/uat-guide.md](castle-cellblock-rebuild/uat-guide.md) (merged in #672).

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| A1 | Cellblock, on entering | Log in fresh; open the mission log | Frost's Letter (1360) is in the log after the loot step | Whether the letter is missing, duplicated, or present but not usable |
| A2 | Prisoner 329 | Talk to him | Exactly one topic dialog | How many dialogs; screenshot of the topics |
| A3 | Marsh | Talk to him | Exactly one briefing | Repeats, or none |
| A4 | Hallway controllers | Use each once, then again | Each accepts once; the second use does not re-accept | Which controller, and what the second use did |
| A5 | Region8 (the pistol guard's room) | Walk in | The pistol guard aggros on entry | Guard did not aggro, or aggroed early |
| A6 | Any time on load | Watch the buff bar, then cure Stasis Sickness | Icon shows on load, clears on cure. **Note:** the Prison Boot lock and the Stasis Sickness effects are no-ops server-side, so this check depends on the client | Icon missing, stuck, or returns after relog |
| A7 | Relog after A2-A6 | Log out and in | Every state above is unchanged | Which step regressed |
| A8 | Vial pickup, then cover | Pick up the vial; take cover | Cover indicator shows on pickup and hides in cover (objective 2484) | Indicator never shows, or never hides |
| A9 | Straegis scene | Accept each mission (640, 641, 680, 688), then trigger the scene | One prompt per accept. Camera plays once, control returns, Marsh is gone, dialog 2516 shows once, then 5859 about 10.6 s after the scene starts | Any repeated prompt, camera replay, Marsh still visible, or 5859 missing or early |
| A10 | Relog right after A9 | Log out and in | The scene does not replay | That it replayed |
| A11 | GC1 (mission 686 escort) | Complete the escort to the ring, take the ring hop | Marsh follows you topside | Marsh missing or invisible after the hop (issue #582 shape: watch for a corpse or actor that only appears after relog) |
| A12 | Flank objectives 2725 and 2731 (scenario T29 in the Cellblock guide) | Try to trigger a flank event | The flank event fires. It may be unreachable in play: it needs a guard already holding a cover slot. The mission still completes on kills either way | Whether you could ever get the event to fire, and how you killed the guards |

### B. Castle (World 8, ring platform; build: current `main`, all of CA00-CA10 merged)

Provisional (reconstructed, MEDIUM confidence) coordinates: the four CA00 respawners, the Op-Core respawner, the Armory prefab, the comms-room placement. Report any bad spot.

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| B1 | Each of the 4 Castle checkpoints | Die once at each | You respawn on real ground, not at the origin, not under the floor | Which checkpoint and where you ended up |
| B2 | Ring platform, mission 688 complete and 1360 active | Look at Gerschon before talking | A "!" over Gerschon. Human sees indicator 2573, Jaffa 5861 | No "!", wrong indicator, or the wrong one for your faction |
| B3 | Gerschon | Accept 701; interact again | 701 accepts exactly once; the second interaction does not re-accept | Double accept |
| B4 | Copplemann | Follow the step | Step advances with no wave spawning | A wave appeared, or the step is stuck |
| B5 | Livewire terminal | Win the minigame | The win prompt shows 2575 once; then 2576 completes 701 and accepts 702 (and 703) | Prompt shows twice, or no accept |
| B6 | Any point in B2-B5 | Relog | The right indicator re-shows for the current step | Which step lost or duplicated its indicator |
| B7 | `Castle_Zuritska_Cell` at 268.0 / 66.79 / 1042.59 | Look for Zuritska (male) and Romney | Both exist for every player | Missing, wrong gender, wrong position |
| B8 | Zuritska | Free her | Completes 702 once | Not complete, or completes twice |
| B9 | Romney | Kill him | Completes 703 | Not complete |
| B10 | Any hostile Castle mob | Kill it, wait 120 s | It respawns after about 120 s | Respawn time, or it never respawns |
| B11 | Interrogation Block | Walk in | Fires mission 702 step 2402. Region boxes now have ceilings, so watch for a box that does not fire | The exact spot where you crossed and nothing happened |
| B12 | Communications room (Level 5) | Take Zuritska there, or enter the region | Zuritska follows to the room, or the step advances on region entry | Neither happened |
| B13 | Communications terminal | Win its Livewire | Grants 5029 exactly once; delivering it starts 706 | Missing or granted twice |
| B14 | ThroneRoom | Walk in | Advances step 2411 | Did not advance |
| B15 | Access Panel | Use it | Completes 706 | Not complete |
| B16 | Surrender or panel diagnosis | Trigger it | The crystal is revealed | Not revealed |
| B17 | Bravo, or Muelbach (the bunker above Bravo) | Pick up the crystal | Grants 2790 exactly once | Twice or never |
| B18 | Report-in | Human reports to Marsh; Jaffa reports to Moh'katan | Only your faction's NPC accepts it, never both | Wrong NPC accepted it, or both did |
| B19 | DHD | Do the DHD Livewire | Wins. **Harset appears in the DHD's destination list the moment you win, without a relog** (Harset H55: the victory chain grants gate address 3 and sends `updateStargateAddress`). Then dial it: the gate opens about 4 s later | Harset is missing or greyed out in the list; a dial that is refused with a feedback message; the gate did not open, opened at once, or took much longer than 4 s |
| B19a | DHD, after B19 | Log out, log back in, open the DHD | Harset is still listed. The address is persisted, not session state | Harset is gone after the relog — that means the database append failed; the server log will carry a `reason = "grant_address_*"` line |
| B20 | Two players | Put two players on different steps of 701-708 | Neither disturbs the other's indicators, actors or steps. A second player on step 2417 can still click Marsh after the first reports in | What one player saw change because of the other |

### C. Harset (worlds 57, 68, 69, 70)

**C0. Which build.** C1, C4 and C6 work on current `main` (waves 1 and 2 are merged). C5 needs the placement PR (`harset/placement`) merged, or that branch built. Reload the play DB from `db/database.sql` after switching, since every placement is seed data.

**C1. Checks that need no placement (from #662):**

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| C1.1 | Harset (world 57), on login | Watch the log, dialogs and objectives | Nothing from the Cellblock fires (no stray icons, dialogs or objectives) | What fired and its id |
| C1.2 | A ring switch (chains 6001-6005) | Right-click it | A list of four destinations. It does **not** teleport by itself, by design | No list, or fewer than four |
| C1.3 | The list from C1.2 | Pick a destination and wait | You arrive on that pad within about 90 s | It hung; how long you waited; and any `arrival_unrecoverable` or off-mesh line in the server log. An off-mesh pad is expected to abort and leave you where you stood, not freeze you |
| C1.4 | The door to the Command Center (chain 6006) | Walk into the transition | You arrive in Harset_CmdCenter | Nothing happened, or you arrived somewhere wrong. Note: the return door (6007) is disabled until M0, so use GM travel to come back |
| C1.5 | The Harset DHD | Click it | The DHD window opens with the local world's point-of-origin glyph | No window, or a nonsense glyph |
| C1.6 | A seeded guard or mob (not a `.spawn` copy) | Kill it; wait | It respawns in about 30 s. Guards, lieutenants, Petbe and Anat stand still | Respawn time; a guard that walks; or it never returns |

**C4. The Castle-to-Harset gate check (this is the M4 gate-arrival check).** What is tested: the server validates the destination exactly once per placement, and never puts you on an unusable point.

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| C4.0 | **Read before C4.1-C4.3** | Finish mission 708's DHD minigame (check B19) first | Every row below assumes you hold Harset's address. Before Harset H55 nothing in the game granted it: a created character's address book is empty, the dial UI only lists known addresses, and after Harset H06 the server refuses a dial to an address you do not hold, with a feedback message and no gate. A GM can still top their own book up with `gmDHD`, which does not persist | If you cannot select Harset in the DHD at all, the grant did not land — that is B19, not a gate-arrival failure |
| C4.1 | Before dialing | Make sure Harset's address is known: win the Castle DHD Livewire (chain 1357, packet H55). Dialing an address you do not know is refused with error 180 | The DHD Livewire win grants Harset's address; an unknown-address dial is refused with a client-visible error | Whether the win granted the address, and what error a premature dial gave |
| C4.2 | Castle gate, on current `main` (after #662 and #682) | Dial Harset from the Castle DHD; wait about 4 s for the gate to open; walk into it | You arrive in Harset at the gate's raw coordinate (-0.08, -67.27, 38.01): at the gate itself, about 2 m above the plaza floor, so you may drop onto the floor. World 57 is `advisory`, so the server accepts this arrival unvalidated and does not snap you back. The server debug log says the arrival was accepted unvalidated | Whether you arrived at all, where you stood, whether you could move afterwards, any fall damage or stuck state, and whether the gate crossing animation played |
| C4.2a | Castle gate, **after** H53 and **before** Harset M0 | Dial Harset and walk into the gate | The transfer now **goes through**. World 57 is `navmesh_mode = 'advisory'`, so the destination is `Unvalidated` rather than off-mesh, and you arrive on gate 3's authored coordinate. The server log has one `reason = "no_navmesh"` arrival line, not `arrival_unrecoverable`. **You should be able to walk away from wherever you land** — that is the whole check | If you cannot move, are inside geometry, or are falling. Give the coordinate you landed on. This is the residual risk of arriving on an unpinned prop transform, and it is what M0 closes |
| C4.3 | After the placement PR (`harset/placement`) merges | Dial and cross again | You land on the plaza in front of the gate at the pinned arrival point (PL-A-01), facing into the plaza, and can move | Where you landed and which way you faced |

**C5. Placement playtest: check the guesses and send corrections (replaces the old M0 walk).** Every row below is an estimate from map data, so a mismatch is expected and useful. Row ids (`PL-A-nn`, `PL-B-nn`, `PL-C-nn`) point into the [placement ledger](harset-rebuild/placements/README.md), which names the seed row to change. Correct the LOW rows first (the ledger lists them).

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| C5.1 | Castle gate, to Harset (PL-A-01) | Dial and cross (C4) | You land on the plaza at about (-5.0, -68.99, 33.0) facing down the plaza (away from the gate), standing on the floor | Where you actually landed, which way you faced, whether you could move |
| C5.2 | Harset plaza (PL-A-02) | Die once | You respawn at about (-8.0, -68.99, 34.0), on the plaza floor | Where you respawned |
| C5.3 | Market and Storage Room (PL-A-03, PL-A-04) | Die once in each | Market: about (48.0, 3.61, 78.0) on the interior floor (MEDIUM, no navmesh in world 69). Storage: about (50.0, 0.0, 44.0) | Where you respawned, especially if you fell or landed inside a wall |
| C5.4 | Command Center north door, going back to Harset (chain 6007, PL-A-06) | Walk back through the door | You arrive in Harset at the north door (about (0, -67.6, -231)) and are **not** bounced straight back into the Command Center | A bounce or ping-pong, or an arrival that is not standable |
| C5.5 | The five ring pads (PL-A-05) | Use each ring switch and arrive | All five arrive on a real platform. The rows are believed correct; the navmesh is what is wrong | Any pad that dumps you off the platform |
| C5.6 | Command Center, world 68 (PL-C-01 to PL-C-10) | Find Ba'al (north hall head), the Royal Guard and symbiote tank (near Anat), Moh'katan, Marsh, Copplemann, Blackstock, Nerus (lab), Opheltes, Athena | Each stands on the floor facing a sensible way, not inside a wall or pillar. **Blackstock, Opheltes and Athena are LOW**; Ba'al's spot is MEDIUM and the alternative is beside Anat | Each NPC's real vs. expected position and facing |
| C5.7 | World 57 (PL-B-01 to PL-B-15) | Find Hansen, Jacobs, Lo'rak, the two Former-Ra Jaffa, the Suspicious Jaffa, the SecondBug and ThirdBug baskets, the three shield towers and Shield Controls, the Bank anchor, the Storage Lo'taur, and the Petbe quarters search object | Each is where its landmark is (Jaffa Zone west, OP-CORE east, bazaar lower level, Bank northwest) and is clickable. **Shield Controls is the weakest row**; Petbe's quarters is LOW-MEDIUM | Which ones are missing, misplaced, unreachable on foot, or floating |
| C5.8 | Named regions (PL-B-16 to PL-B-23 and the PL-C regions) | Walk into each: Jaffa Zone, OP-CORE Zone, Bank, Petbe quarters, three shield towers, Shield Controls, and the Lab, Marketplace and Storage interiors | Entering fires the region (for example mission 1241's Lab scan) | A region that does not fire, or fires from the wrong place |
| C5.9 | Any spot you want corrected | Stand on the right spot on foot and type `.bug pin <what it is>` (for an NPC, next to it) | The server records your position, `on_navmesh`, `regions_inside`, and nearby entities' position and `yaw_byte` | Send the recorded pins keyed by row id. A spot you cannot walk to is not a valid pin (Castle's Romney was placed into a sealed wing) |
| C5.10 | **The two door pairs (not placed: no evidence)** | Stand in the Harset doorway that loads the Market and type `.bug pin market door`; repeat from inside the Market and for the Storage Room both ways | Two doorway coordinates close both packets | If you cannot tell which doorway is which, that is itself the answer |

For a heading, face the way a visitor arrives before pinning (`.lookat`), because a heading of exactly 0 on a placed NPC is almost always the Castle "faces a wall" mistake.

**C6. Castle playtest findings applied to Harset, and the non-GM walk (packets H51 and H53, on `content/harset-wave2`; needs no placement).** C6.1-C6.3 are the 2026-09-18 Castle defects that would have repeated here. C6.4-C6.5 are H53, and they are the ones that must be run from an **ordinary account**: every containment gate in the server is warn-only for a GM, so a GM cannot observe the defect H53 fixes.

| # | Where / command | Do | Expect | If it fails, report |
|---|---|---|---|---|
| C6.1 | Any plaza guard (they stand still by design) | Shoot it from its side or back, from beyond about 30 units, then strafe around it | It turns to face you within about 2 s (the AI tick) and keeps turning as you move | `.bug guard not turning` next to it: `wire_facing_vs_caller_deg` should be near 0 |
| C6.2 | The gate-side sentries near (4.7, -58.7, -188.2) and (-4.4, -67.6, -231.1) | Aggro one from inside 30 units with a clear view | It shoots back. Before H51 these nine sentries could never fire, because `harset.nav` does not cover where they stand | `testLOS <guard> <you>` from the GM console: `BLOCKED` with a clear view is the bug; `UNKNOWN` is expected there and is treated as clear |
| C6.3 | Anywhere in Harset | Die, respawn, then use a ring switch and walk onto the pad; then walk into the Command Center door | Both still work after the respawn | This is the Castle "Throne Room doesn't recognise me" bug. The server log should show `respawn: re-registered client-hinted regions after reanchor` and then `region_hint` lines again. If the friction detector logs `no_region_hints_since_respawn`, the fix did not take in the client |
| C6.4 | **Harset, on an ordinary (non-GM) account** — packet H53. This check is worthless from a GM: the navmesh gate has always been warn-only for GMs, which is exactly why nobody caught this | Start on the gate plaza and **walk** (do not fly, `ghost` or `.gotoxyz`) east across the plaza, then the whole way to the Command Center door | You cross the plaza and reach the door on foot without ever being yanked backwards. The stretch from about Z -200 to Z -228 at X -24..0 is the one the mesh does not cover at all; before H53 an ordinary player was snapped back there every packet | Exactly where you were snapped back, from the client's point of view. In the server log, `movement.validation` rows with `reason = "navmesh"`, and the boot line `reason = "navmesh_mode_summary"` for world 57 — it must read `navmesh_mode = advisory`. If it reads `enforce`, the seed did not load and nothing else in this row will work |
| C6.5 | Same non-GM character, same session | Ring out and back (C1.2/C1.3), then die and respawn | Rings still arrive and the respawn still places you somewhere you can walk away from | H53 relaxed only the navmesh gate. If you end up frozen, inside geometry or falling, that is a **different** defect — report the coordinate and any `arrival_unrecoverable`, `CorrectionSuppressed` or `OutOfBounds` line. Out-of-bounds and teleport-sized moves are still rejected in Harset by design |
