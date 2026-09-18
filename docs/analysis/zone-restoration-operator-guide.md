# Zone Restoration: Operator Test Guide

> Type: how-to. Audience: the human operator (game owner) running in-client UAT. Written 2026-09-18 during the weekly-usage crunch, from the three coordinator sessions' own status reports. Nothing below has been run in a client yet; every "expected" line is what the code and tests say should happen, not an observed result.

Companions: [Cellblock handoff](castle-cellblock-rebuild/handoffs/session-resume.md), [Castle handoff](castle-rebuild/handoffs/session-resume.md), [Harset handoff](harset-rebuild/handoffs/session-1-resume.md), and each campaign's `README.md` for its full UAT milestone list.

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
| Castle Cellblock | C00-C05, C07, C08a, C08b, GC1b-0, GC1 (#655) | none | **Yes: M1, M3 and GC1 on `main`, M2 only partly (needs C06)** |
| Castle (World 8, ring platform) | CA00 respawners (#651), CA01 + CA03 mission 701 and the dialog-speaker pin fix (#659), CA10 gate open/cross events (#663), CA04 minigame hardening (#652), CA02 "!" dialog bind (#661), CA05 story actors and point sets (#667), missions 706 and 708 (#668), missions 702-704 (#660) | none (docs closeout PR #669 only) | **Yes, M1-M5 on `main` after a rebuild** (the gate leg into Harset excepted, see M4) |
| Harset | nothing yet | #662 (branch `content/harset-rebuild`, all review findings answered, CI running on the final head, waiting on your merge go-ahead) | Travel and population checks, from the branch now, or from `main` once #662 merges |

## Suggested Order If Budget Is Tight

Highest value per minute first.

1. **Cellblock M1 and M3 on current `main`, and the testable half of M2.** The largest body of already-merged content and it needs nothing else to land first. The flank objectives and step 2144 wait for C06.
2. **Castle respawn check on `main`.** Five minutes, catches bad coordinates.
3. **Harset M1 checks** once #662 merges (or from its branch). Cheap, and tells us whether ring and door travel is safe.
4. **Castle M1** (mission 701). All its PRs (#659, #652, #661) are on `main`, so this is testable now after a rebuild.
5. **Harset M0 placement session.** The longest job and the critical path for every Harset mission, but it needs the least Claude budget until the final commit of the generated seed SQL, so it can run whenever you have the time.

## Castle Cellblock (Castle_CellBlock)

**Implemented (merged).** GC1 (#655, `627be660`): Marsh escort dialogs, ring-hop and topside follow, and the post-death blurb 5859 on mission 686 completion. Also: Prisoner 329 and Marsh briefing dialogs; hallway controllers; Region8 guard aggro; the Stasis Sickness icon; Frost's Letter (mission 1360); the take-cover objective 2484 with its cover indicator; accept-blurbs for missions 640, 641, 680 and 688; the Straegis camera scene (Matinee, Marsh despawns, dialog 2516).
**Open.** Only a docs PR (#666, ledger and handoff). The 147-test chain-replay suite passed on a fresh DB before the GC1 merge.
**Not built.** C06 flank objectives 2725 and 2731 (unblocked, not started), GC1c lockdown VFX (needs client evidence), GC2 (unscoped), GC3 XP formula (needs your decision).

Test with a Jaffa character and a Human character, relogging at each step:

| Milestone | Steps | Expected |
|---|---|---|
| M1 | Load in; talk to Prisoner 329 and Marsh; walk the hallway controllers; enter Region8; observe the Stasis Sickness icon, then cure it | Exactly one topic dialog from Prisoner 329 and one Marsh briefing; each controller accepts once; the pistol guard aggros on Region8 entry; the icon shows on load and clears on cure |
| M2 (partial on `main`) | Check the mission log; pick up the vial; take cover. Step 2144 and the flank objectives (2725, 2731) cannot be tested: they depend on C06, which is not built | Frost's Letter is in the log; the cover indicator shows on vial pickup and hides in cover |
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
| M4 (#663 and #668 merged; the Harset leg waits on Harset #662) | Dial with the DHD Livewire | The gate opens about 4 s after dial; crossing plays 6113 and lands on Harset once. **Do not dial to Harset on current `main`:** the arrival validation (Harset H01, in PR #662) is not on `main` yet, so you would be placed at the gate's raw coordinate inside a navmesh hole and silently frozen |
| M5 | Two players at different steps | Neither disturbs the other's indicators or actors |

**Untested and provisional:** every coordinate (comms room, Armory prefab, and the Op-Core respawner, which could not be located in the map assets, so it is a reconstruction). Issue #657 (`active_objective_ids` lost on relog, affects every mission) is unfixed; Harset packet H50 is a fix in progress.

## Harset (worlds 57, 68, 69, 70)

**Situation.** Harset was an empty shell (23 static spawns, no content chains, a navmesh of 1,939 disconnected islands). Wave 1 is PR #662 (146 files, CI green, review fixes being applied). It delivers travel safety and population, not missions.

**Implemented in #662 (not yet on `main`).**

| Packet | What you would see |
|---|---|
| H01 | Gate arrival is validated against the navmesh with a respawner fallback; DHD interaction shows the point-of-origin glyph |
| H02 | Ring transporters cannot hang forever: every wait state has a timeout (90 s for remote load, a judgement value to tune from a real client) |
| H10 | Ring switches (chains 6001-6005, regions 4-8) and the Harset-to-Command-Center door (6006). The return door 6007 is **disabled** until its coordinate is pinned |
| H11 | 33 new mob and prop templates (200-223, 240-248) and two NPC ability sets (Jaffa staff, Goa'uld ribbon device). Templates only: nothing spawns from them yet |
| H13 | The 14 Harset mob rows respawn after 30 s; guards, lieutenants, Petbe and Anat stand still |
| H03, H04, H07 | Engine primitives for later missions (spawn/despawn actions, health-threshold trigger, world condition). Not player-visible; covered by tests |

**Checks you can do from the #662 branch, no placement needed:**

1. Log in on Harset. No Cellblock dialogs, icons or objectives should fire (they used to leak in).
2. Right-click a ring switch. Expected: a destination list of four (it does not teleport by itself, by design); pick one and arrive on that pad. Watch the server log for `arrival_unrecoverable` or an off-mesh warning; a pad that is off the navmesh is exactly what this is meant to reveal.
3. Walk into the Harset to Command Center door. Expected: you arrive in Harset_CmdCenter. You cannot walk back through the door yet (6007 is disabled); use GM travel.
4. Click the DHD. Expected: the DHD window opens with the local world's glyph.
5. Kill one of the seeded guard or mob spawns (not a `.spawn` copy). Expected: it respawns in about 30 s. Tell us if 30 s feels wrong; it is one number.
6. **Gate dial from Castle to Harset is not yet safe to test.** The gate has no seeded arrival coordinate and world 57 has no respawner row, so after the review fix the transfer is meant to abort cleanly instead of placing you in a navmesh hole. If it teleports you anywhere, note where.

**M0: the placement session (only you can do this).** About 25 story NPCs have no coordinate anywhere, and eight arrival points need pinning. Every Harset mission is blocked until this is done. Procedure, from [the Harset README](harset-rebuild/README.md):

1. Pin the gate arrival, the five ring pads and both Command Center transitions using the map debug HUD. The return door (6007) needs a point roughly 8 units clear of point set 2078 to avoid ping-pong.
2. Place each story NPC and vendor prop with `.spawn <template>`, then `.savespawn`.
3. Run `.seedconfirm`. It emits the seed SQL; it never writes live rows. Hand that SQL to a session to commit.

**In progress, not finished, not testable:** wave-2 work exists as uncommitted changes in worktrees (H06 client-forged region guard, H08 Submit cleanup, H09 multi-ability sets, H50 objective persistence, and seed chains for missions 1324, 1326, 1360/567, 1361, 1200 and 742). None of it has been reviewed or merged, and it is all gated on M0 for positions. Do not spend client time on it.

## Known Cross-Cutting Risks

- **Relog defects.** #657 (objective state lost on relog) affects every mission; H50 is the fix. Until it merges, "resumes correctly after relog" can fail for reasons unrelated to the content under test.
- **Invisible corpse after spawn** (#582). Seen at Castle Cellblock; watch for it on any NPC spawned mid-session.
- **Every coordinate is provisional** in Castle (CA00, comms room, Armory, Op-Core) and unpinned in Harset.
- **Ring timeout values** are judgement, since the 2009 server had none.

## Where Each Session Leaves Off

| Session | Handoff document |
|---|---|
| Cellblock | [castle-cellblock-rebuild/handoffs/session-resume.md](castle-cellblock-rebuild/handoffs/session-resume.md) |
| Castle | [castle-rebuild/handoffs/session-resume.md](castle-rebuild/handoffs/session-resume.md) |
| Harset | [harset-rebuild/handoffs/session-1-resume.md](harset-rebuild/handoffs/session-1-resume.md) |

Each coordinator was asked to refresh its handoff before it runs out of budget. Treat this guide as a snapshot: if a PR merged after 2026-09-18, the handoff is more current.
