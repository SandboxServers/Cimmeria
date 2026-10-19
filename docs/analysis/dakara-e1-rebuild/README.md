# Dakara_E1 Rebuild

> Type: ledger. Audience: the owner, the orchestrator, packet workers and reviewers.
> Opened 2026-10-06 against `main` @ `ddd549797`; re-sliced to the [campaign-packet skill](../../../.claude/skills/campaign-packet/SKILL.md) on 2026-10-18 against `main` @ `d1bba0400`. Packet prefix `DK-`. Companions: [resume card](RESUME.md), [work packets and contract](work-packets.md), [audit](audit.md), [placements](placements/README.md), [Class Start v6 ledger](../class-start-v6/README.md), [Harset rebuild](../harset-rebuild/README.md), [Castle rebuild](../castle-rebuild/README.md), [operator guide](../zone-restoration-operator-guide.md).
>
> **Status (2026-10-18): wave 1 is Done (DK-06a #1854, DK-07 #1853, DK-08 #1855, DK-12a #1856). Wave 2 is Ready: DK-09, DK-09b, DK-10a, DK-10e, DK-10fs; DK-06b waits on D-DK10.** D-DK1 is decided as option F (owner, 2026-10-18), with D-DK17 and D-DK20 ruled: the bridge mission 63100 "Home to Dakara" leads into 1570 (packets DK-10fs and DK-10fc). DK-00 to DK-05 are merged (#1282 to #1288). M1 and M2 UAT are owed; the owner runs them himself, and no agent uses the lab until the campaign ends. The DK-08 review's two calls are ruled (D-DK18, D-DK19). Wave 3 waits on wave 2 per its dependencies. This README is the single status source; [work-packets.md](work-packets.md) holds the rules and packet text, not status.

## Why

Class Start v6 moves the Free Jaffa start (char_defs 8 and 18, archetype 7) from SGC_W1 to Dakara_E1 (PR #1273, merged). That PR's review found the result was a dead end: world 61 had one spawn (the DHD), no hostile, no content chain, and a new character's stargate address book was empty, so a new Free Jaffa had no mission and could not leave. The owner then supplied an external handoff archive with a `WORLDS` directory for the planet and asked whether it strings together enough of what the database already holds to restore the zone.

## The Answer

**No for a whole zone today, yes for the story once positions are chosen, and the archive is not what makes the difference.**

- **What we have, complete:** the text of the whole Dakara_E1 arc. Twelve missions (1570 from its third step, and 1645-1655) with every step and objective, twelve dialog sets with every line, nine mission items, and the client's names for the cast, the interactable objects, six named areas and one respawner. The client map is complete (401 tiles, identical to the archive's last build), the server has a navmesh for it, and the gate, its volume, its Kismet and the DHD are seeded and match the client.
- **What is missing, everywhere:** where anything stands. No position survives for Bra'tac, Moh'katan, Loth'ta, the tents' entrances, the drop sites, the gates, the Naquadah Repository or the Med Tent. No hostile template, encounter, wave or count exists for the missions that are fights. No offer rule, prerequisite or reward exists for any mission. The seed, the client and the archive agree on this: the archive's spawn, encounter and player-start files for Dakara_E1 are empty.
- **What the archive adds:** a file manifest that proves our client is its final build, and actor rows we can read from the same maps with the repo's own tools. Its mission list is missing four of the twelve missions ([audit, claims 2 and 3](audit.md#handoff-claims-used-and-their-verdicts)). Nothing in it needs importing.
- **How much is playable from that:** the dead end is gone (DK-01). Seven of the twelve missions have no fight in their objectives: talk, carry, search, hack and two device steps (1570's Dakara half, 1645, 1646, 1648, 1649, 1650, 1652). They are authorable once positions are estimated from the map and corrected in one playtest pass. In story order the peaceful run is 1570, 1645, 1646; then 1647, a fight, sits in front of the rest. Two missions are fights with no recovered design (1647, 1653) and need the owner's go-ahead to be reconstructed. Three leave for the SGC (1651, 1654, 1655) and wait on a way to dial out of the SGC.
- **One thing the owner should know before deciding anything:** the recovered arc is not a level-1 start. It is level 3 to 5, and its first Dakara scene tells a Jaffa player "Welcome back, my friend" after the SGC attack. In the client's data the Free Jaffa reaches Dakara from the SGC. A level-1 Dakara start is this project's decision, so its first minutes have to be reconstruction or new content whatever we do (D-DK1; option F, recommended, adds one authored level-1 mission that leads into 1570).

The four other Dakara worlds in the archive's `MISSING_UNRECOVERED` folder (`Dakara` 24, `Dakara_E2` 63, `Dakara_E3` 64, `Dakara_Superweapon` 65) have no map in the client and none in the archive. Their folders are name-matched asset lists the archive itself grades `CANDIDATE_NOT_PROVEN`. The fifty missions labelled Dakara E2 and E3 cannot be placed in a world and are outside this campaign.

## Goal And Non-Goals

**Goal.** A Free Jaffa who starts on Dakara_E1 can always leave and come back, has a first mission, and can play the Dakara arc as far as the evidence and the owner's decisions allow, with every reconstructed row labelled as reconstruction.

**Non-goals.**

- Dakara_E2, Dakara_E3, Dakara_Superweapon, Dakara (24) and MissionTestDakara (30): no client map.
- The SGC side of 1570, 1651, 1654 and all of 1655: the SGU route campaign (OD-CS10 in the Class Start v6 ledger) owns them. This campaign owns what happens on worlds 61 and 62.
- Vendors, trainers and ambient population. The merchant tent on the map has no vendor data (Harset's D-H09 applies).
- Mission XP, cash and item rewards. All twelve missions have none seeded; the Cellblock's GC3 gate (formula evidence first) applies unchanged (D-DK7).
- Changing where Free Jaffa start. That is Class Start v6's decision; this ledger records the evidence and offers it as an option (D-DK1, option D).
- New client content: no new dialog rows, no cooked dialog or step-text overrides, no client patch. One exception, under D-DK1 F: the authored mission 63100 enters the client's mission catalog as a server-served `MISSION_ADDITIONS` entry (#1734). The client downloads it in the category-3 resync at character select; nothing is installed on the client.

## Evidence Labels

Seed rows and worknotes in this campaign use the labels the earlier campaigns use:

| Label | Use |
|---|---|
| `ORIGINAL_DATA` | The row restates data that is in the client's own cooked files (a mission step, a dialog id, a name string, a map actor). It never means CME server data; the seed is a reconstruction ([rules-and-gotchas](../../agents/rules-and-gotchas.md)). |
| `RECONSTRUCTION` / `PROJECT_FINAL_RECONSTRUCTION` | Authored by this project to connect recovered pieces: offer rules, step triggers, positions, encounters. |
| `NEW CONTENT` | Text or behaviour with no recovered counterpart (the arrival notice, Teal'c's letter, mission 63100). |
| `PROJECT_FINAL_S2C` | Inherited from Class Start v6: the Dakara start, level 1, start gear. |
| Placement classes | `AUTHORED`, `MAP-LANDMARK`, `MAP-GEOMETRY`, `MAP-MARKER`, `SPEC-DESCRIPTIVE`, `INFERRED`, with HIGH / MEDIUM / LOW / NO-IDEA confidence, exactly as in the [Harset placement method](../harset-rebuild/placements/METHOD.md). |

Handoff claims carry CONFIRMED / CONTRADICTED / UNVERIFIABLE in the [audit](audit.md#handoff-claims-used-and-their-verdicts).

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | The whole arc's text survives: 12 missions, every step, objective and dialog line, 9 items, cast and prop names. | [audit](audit.md) | DK-00 (Done) |
| F2 | No position survives for any story actor, object or area; DK-02's map estimates are mostly LOW. | [placements](placements/README.md) | DK-02 (Done), M2 correction pass |
| F3 | No hostile template, encounter, wave or count exists for 1647 or 1653. | audit | GDK1, DK-20 (D-DK5) |
| F4 | The recovered arc is level 3 to 5 and opens with "Welcome back, my friend"; nothing recovered is level 1 or 2. No server code reads a mission's level (`missions.level` and `dialog_set_maps.min_level` have no reader under `crates/`), so the level is cosmetic today. | dialogs 5810/5811; grep of `crates/` | D-DK1, DK-10a |
| F5 | The SGC cannot be left: worlds 58 and 86 have no DHD or dial chain and gate 27's row is under the floor. | audit | DK-30, DK-33 (B-DK4) |
| F6 | The radio beats (dialogs 5815, 5816 and later 4916, 4921, 4937) mix speaker 2609 Hammond and speaker 0, with no NPC present. `display_dialog` from a non-interact chain shows only all-speaker-0 dialogs. | dialog rows; Harset guardrails | DK-06a, DK-06b (D-DK10) |
| F7 | `complete_objective` on a step's last required objective completes the **whole mission**, not the step (`crates/cell-content/src/cell/missions/progression.rs`, the `all_required_complete` block). A `complete_objective` on an objective outside the current step is a no-op with a WARN. Any-order multi-objective steps (1570 step 4902 and 4901, 1646 step 4908) therefore need a pattern. | progression.rs | DK-08 (D-DK12) |
| F8 | The three SG-18 remains sites (PL-DK-A-07 to A-09) are LOW and off the plaza's navmesh component (279): A-07 in component 619, A-08 and A-09 in 324. | [A-world61 placements](placements/A-world61-story-placements.md) | DK-09 (D-DK11) |
| F9 | The five drop locations for 1646 are unplaced. The DK-03 prop is one template (449), and one template holds one dialog bind and one glow, so five drops on one template cannot carry five binds. | placements README "No idea" list; DK-03 | DK-12a, DK-09 (D-DK13, D-DK16) |
| F10 | The handoff archive has nothing to import; its Dakara spawn and encounter files are empty. | audit | Done |
| F11 | The merged id blocks differ from the 2026-10-06 plan (spawns 8000+, point sets 2123+, templates 440+, respawner 611), and the chain block 8001+ is not registered in the four-zone id allocation. | seed files; [id-allocation](../four-zone-planning/id-allocation.md) | DK-07 (D-DK8, D-DK9) |
| F12 | Lab evidence from Class Start v6 CS-08 already shows a new Free Jaffa entering on Dakara (SGU_FREE_JAFFA PASS). It covers part of DK-U1, not the rest of M1. | [CS-08 worknote](../class-start-v6/worknotes/CS-08.md) | M1 |
| F13 | The interact-tag linter (`crates/content-engine/tests/it/interact_tag_linter.rs`) needs an allowlist entry per chain that binds a dialog set from a non-interact trigger. It accepts ranges and has no stale-entry check, so ranges can be added before the chains exist. | linter source | DK-10a |
| F14 | Every chain replay test in this crate loads the real seed (`build_engine(Some(&pool))`), so mission packets run them through `live-db-test.ps1`, not plain nextest. | `chain_replay_tests/mission_1324.rs` | all mission packets |
| F15 | No chain on `main` touches mission 1570, and no SGU route campaign exists yet to own its SGC steps 4641 and 4642. | grep of `db/resources/Content/Seed/` | DK-10a (D-DK6) |
| F16 | Two stale remote branches remain from merged packets: `dakara/dk01-dead-end-fix`, `dakara/dk03-cast-templates`. | `git branch -r` | DK-99 |
| F17 | Objective 5937 (step 4904) is seeded `is_optional = false` though its text reads "(Optional) Follow the path...". Completing 5928 alone does not finish the step. | `missions` seed | DK-10b (leave 4904 by `advance_step`) |
| F18 | Bra'tac (59) and Moh'katan (54) are templates shared with Harset; binds on them are per player, so Dakara binds do not leak, but a template edit would. | DK-03 worknote | all mission packets |
| F19 | The arrival notice (chain 8002, subject "Dakara: no orders yet") says the Free Jaffa command "is not staffed yet". Under D-DK1 B or C that is false the moment wave 3 merges. The mail is once per character, so characters that already have it keep the old text. | `dakara_e1_space_chains.sql:122`, `chain_replay_tests/dakara_e1_space.rs:63` | DK-10e (D-DK17) |
| F20 | The linter's region check (`every_chain_region_key_matches_a_seeded_point_set`) reads only `db/resources/Events/Seed/point_sets.sql`, so any `enter_region` key from `point_sets_dakara_e1.sql` (2123-2130) fails it. The linter file is also 753 lines, over the 700 hard cap. | `crates/content-engine/tests/it/interact_tag_linter.rs:467` | DK-09b |
| F21 | The server does not re-check that a player who reports a client-hinted region stands in it. For 4902 a forged report only advances the sender's own step. | `event_dispatch/region.rs` | Accepted (work-packets C5 rule 8) |
| F22 | DK-08's 4902 and 4901 rows need 12 chain ids; C3 gave DK-10c 10 and DK-10d sat directly after it. | [DK-08 review](worknotes/DK-08.md), finding 8 | D-DK19 |
| F23 | **The server can ship a new mission to the client without a patch** (owner, 2026-10-18; option E's "cannot add a mission" was wrong). #1734 built the path: a `MissionAddition { mission_id, xml }` entry in `crates/resources/src/base/mission_additions.rs` is appended to the served mission category (3), the metadata version bumps, and the client's `versionInfoRequest` at character select pulls the whole category again (the PAK is 749 KB, about 2-3 s, once per client per build). The journal then shows the mission's name, level, step and objective text from that XML. Production `MISSION_ADDITIONS` is **empty**: 63100 would be the project's first live entry, so client parsing, journal, progression and relog are unproven (`docs/architecture/mission-pak-overrides.md` keeps them UATPending). Ids 63100-63103 are free in the stock catalog, `db/resources`, `crates` and every open campaign. | `mission_additions.rs`, `resources/mod.rs` (`load_all_impl`), `tests/mission_additions.rs`, `mission-pak-overrides.md` | DK-10fs (D-DK1 F, D-DK20) |
| F24 | Two existing tests break when the first production addition lands, and the bridge packets must fix them exactly: `tests/mission_additions.rs` asserts `MISSION_ADDITIONS.is_empty()` and builds its baseline with `load_all(DIR)` (which now includes 63100), and `chain_replay_tests/dakara_e1_space.rs` asserts the exact chain set on a Free Jaffa arrival (it gains 8007; the world-gated 8075 is expected not to fire there because that harness stamps no world id, which DK-10fc proves by running it). | test sources | DK-10fs, DK-10e, DK-10fc |

## Decisions

Older files, worknotes and seed comments say `OD-DKn`; that is the same decision as `D-DKn` here. Statuses use the skill vocabulary; a `BlockedDecision` row waits on the owner and the orchestrator asks it.

| Id | Decision | Status | Reason and consequence |
|---|---|---|---|
| D-DK1 | **What a level-1 Free Jaffa is given first.** **A.** No mission: an address, a notice, a respawn. **B.** After A, accept 1570 on arrival, advance it past its two SGC steps and play its Jaffa dialogs; the log then shows two completed SGC steps the character did not play. **C.** After A, skip 1570 and start at 1645. **D.** Return the Free Jaffa start to SGC_W1 (a v6 decision). **E.** B plus a bridge for the first minutes: E1 a letter, E2 a letter and a hail, E3 the newcomer dialogs (superseded). **F.** An authored level-1 mission, 63100 "Home to Dakara", accepted on arrival; talking to Rak'nor completes it and starts 1570 at step 4906, with Teal'c's letter (E1) for the backstory ([§ D-DK1 option F](#d-dk1-option-f-an-authored-level-1-bridge-mission)). | Decided: option F (owner, 2026-10-18) | Recommended **F** (2026-10-18). It replaces E: the owner overruled option E's claim that the server cannot add a mission (F23). A level-1 Free Jaffa gets a level-1 mission in the journal that names the first person to talk to, the letter tells the backstory the shipped dialogs assume, and 1570 then runs as in B. Adds DK-10fs (wave 2) and DK-10fc (wave 3); amends DK-10a (8040 triggers on 63100's completion), DK-10b (drops 8057) and DK-10e (chain 8007, the letter). E2's hail and DK-10f are dropped. It was the blocker for DK-10a to DK-10d, DK-10fs, DK-10fc and DK-11; now lifted. Fallback if the client rejects 63100: B with E1 ([§ option F, fallback](#fallback)). Under C, DK-10a to DK-10d drop and DK-11's offer chain is gated `archetype eq 7` and 1645 `not_active` instead of 1570 `completed`. Under D the campaign stops after wave 2's DK-09 and DK-06b. |
| D-DK2 | The Dakara start grants **Omega Site (gate 5)** only; the SGC address waits for DK-30. | Decided (shipped in DK-01, #1284); owner confirmation pending, non-blocking | Omega Site has a DHD, so the trip is two-way. Changing it is one seed row. |
| D-DK3 | Positions are **map estimates** (DK-02), corrected by the owner in one pass after the M2 playtest. | Decided (2026-10-07, for DK-04) | Harset method. MEDIUM: the mission texts' "Eastern" and "Western" are not tied to a map axis. |
| D-DK4 | **One tent interior**, world 62 (instanced), for both tents. | Decided (2026-10-07, for DK-04) | It is what the four flap names and the single map imply; the instance gives per-player scenes for 1650 (Harset D-H03). |
| D-DK5 | Fights with no recovered design (1647, 1653, guards on 1570's sites and 1652's plazas): **reconstruct** small encounters at the player's band, after the M2 playtest, behind GDK1. | **BlockedDecision** | Owner go-ahead needed. Blocks wave 4 onward. Until then the arc stops at 1646. |
| D-DK6 | DK-01 shipped as its own PR here. This campaign authors world 61/62 steps; the SGU route campaign authors SGC steps, meeting at step ids. Until that campaign exists, DK-10a advances past 4641 and 4642 itself. | Decided | OD-CS10 keeps SGU route authoring out of v6. |
| D-DK7 | **Rewards inherit GC3**: no XP, cash or items until there is formula evidence. | Decided | No reward is seeded for any of the twelve missions. |
| D-DK8 | Register chain block **8001-8499** for Dakara_E1 in the four-zone id allocation. | Decided; DK-07 does it | F11: the block is in use (8001-8006) but unregistered. |
| D-DK9 | **The merged ids are the contract**: spawns 8000-8099, point sets 2123-2199, templates 440-479, respawners 611-619, chains 8001-8499. The 2026-10-06 plan's spawn block 600-699 and point-set block 2200-2279 are void. | Decided | Every merged row already uses the new blocks. |
| D-DK10 | How a radio beat is shown (pin a speaker without an NPC, a speaker-0 fallback, or something else). | Open: ruled by the architect after DK-06a. A client-patch outcome becomes BlockedDecision for the owner. | Blocks DK-06b, hence DK-10d step 4905 and the later radio steps. |
| D-DK11 | Seed the SG-18 remains at A-07 to A-09 anyway, each with an on-mesh guard only (no plaza-connectivity guard). | Decided | They are search sites, reached across rough ground; the M2 playtest corrects them. |
| D-DK12 | The pattern for any-order multi-objective steps (F7). Candidates: sibling "last / not last" chains per objective; a counter rebuilt on `player_loaded`; a region-enter step; an engine change so that all-required-complete on a non-final step advances the step (needs a Castle and Harset impact check). | Decided by DK-08 ([worknote](worknotes/DK-08.md)), seed-only, plus D-DK18 for 4902's triggers. | Sibling "not last / last" chains on objective status for 4902; a counter plus one visited marker per remains for 4901; a counter rebuilt on `player_loaded` for 4908. The rows are copied into DK-10c and DK-12c. No engine change. |
| D-DK13 | Place the five drop locations by map estimate inside component 279 (the walled city). | Decided; DK-12a does it | The texts say "drop location" in Moh'katan's scouts' area; no position survives. |
| D-DK14 | **One seed file per packet**; DK-10a creates all seven mission seed files with headers only, their `\ir` lines, the linter range entries and the empty test submodules. | Decided | Lets wave-3 packets run in parallel without touching shared files. |
| D-DK15 | Archetype gates: Jaffa dialogs `archetype gte 7`, Tau'ri dialogs `archetype lte 6`, the Free Jaffa entry `archetype eq 7`. | Decided | Praxis Jaffa (8) visiting gets the Jaffa lines; the dsm rows ship both variants. |
| D-DK16 | The five drop props get **distinct templates**: 449 plus clones 458 to 461, so each holds its own bind and glow. Hostiles move to 462 and up. | Decided; DK-09 does it | F9. |
| D-DK17 | Reword the arrival notice to point at the first mission giver, using the proposed B wording, and send Teal'c's letter (chain 8007) as written ([DK-10e](packets/wave-2.md#dk-10e-reword-the-arrival-notice-and-send-tealcs-letter)). | Decided: proposed wording and Teal'c letter as written (owner, 2026-10-18) | F19. DK-10e runs once D-DK1 is confirmed. Under A or D the notice stays and DK-10e is dropped. |
| D-DK18 | **Step 4902's three objectives are client-hinted regions on the SG-18 remains sites.** Point sets 2128 `Dakara_E1.SG18Louis` (5632), 2129 `Dakara_E1.SG18Nguyn` (5630), 2130 `Dakara_E1.SG18Waters` (5631, optional): world 61, `Cylinder`, radius 8, height 10, flags 1, centred on the remains spawns' x/z, 3 m below their y. The chain that advances to 4901 shows 5814 ("weak radio signal") and binds the three remains. No 4902 bind, no new template, dsm 6730 unused. | Decided (architect, 2026-10-18, DK-08 review finding 7) | The step text says "search the locations"; the remains are those locations. A region needs no template (the block 462 up is DK-20's) and no bind or restore. A region at the outpost lead was rejected: Loth'ta stands there, so the H52 step replay would fire it on the 4904 click. 5814 is speaker 0, so it shows from a region chain (F6). The M2 position pass moves each region with its remains. DK-09 seeds them; DK-09b makes the linter see them (F20). |
| D-DK19 | **Chain ranges:** DK-10c gets 8060-8074 (12 used, 3 spare); DK-10d moves to 8160-8179; later missions start at 8180. | Decided (architect, 2026-10-18, DK-08 review finding 8) | F22. Moving DK-10d keeps every other range and gives the radio step room before D-DK10 is ruled. DK-07's id-allocation row and DK-10a's allowlist and constants carry the new ranges. |
| D-DK20 | **63100 is the project's first production `MISSION_ADDITIONS` entry**, so the owner's M2 run is also the first live proof of the #1734 path (client parsing, journal, progression, relog). Cellblock (CB-N05) and Castle (D-CA16) also plan authored missions. Does Dakara go first, and does a pass at M2 count as the proof for them? Consequence either way: every client re-downloads the mission category once after the deploy that ships it. | Decided: accept being first user of the mission-catalog route (owner, 2026-10-18) | F23. Recommended: Dakara goes first, since 63100 is the smallest possible entry (one step, one objective, one task, the shape of the fixture #1734 already tests). If the owner wants another zone first, DK-10fs waits for it and only its id row and test move. |

### D-DK1 option E: a bridge for the first minutes

> **Superseded by [option F](#d-dk1-option-f-an-authored-level-1-bridge-mission) (2026-10-18).** The owner overruled this section's first "cannot" bullet: "We can ship new missions to the client just fine" (F23). The analysis of the gap and of the letter still holds and option F reuses it; E2's hail, E3 and DK-10f are dropped.

The owner asked (2026-10-18) whether this is a place to author new content that bridges a level-1 Free Jaffa to an arc that is level 3 to 5 and opens with "Welcome back, my friend". This section answers that. It changes no other decision. D-DK17 is answered: the notice text proposed in DK-10e is used as written, and every option below keeps it.

**What the gap is.** The client's own dialogs say who the player is. In 6110 the player tells Rak'nor "Ba'al's forces invaded as I was delivering my warning". In 5811 they tell Bra'tac that the warning "came too late" and that "Teal'c has asked me to search for SG-18". In that story the Free Jaffa is Bra'tac's messenger, coming home from the SGC through the gate. A character created on Dakara has seen none of this. So the gap is backstory, not mechanics. 1570, 1645 and 1646 have no fight, nothing reads their level (F4) and they pay no XP (D-DK7), so a level-1 character can play all three as they are.

**What the server can author within the Scope** (no client patch, no new dialog row, no cooked-data override):

| Tool | What it can do | Label |
|---|---|---|
| System mail (`send_system_mail`, as chain 8002 uses it) | Free text: a sender and a subject of up to 128 characters each, a body of up to 1,000. Sent once per character with the quiet cooldown. The only channel for free text. | `NEW CONTENT` |
| Chat bark (`npc_bark`) | Says the text of an existing `dialog_screens` row in the player's chat window under any speaker name. It opens no window and goes to the triggering player only. It reuses client text and adds no row. | `ORIGINAL_DATA` text, `RECONSTRUCTION` chain |
| Dialog choice | Which shipped dialog an NPC shows, on which step, for which archetype (dialog-set binds). | `RECONSTRUCTION` |
| Order and gates | Which mission is accepted when, and which steps advance (chains). | `RECONSTRUCTION` |

**What it cannot do.**

- ~~Add a mission or an objective.~~ **Wrong** (owner, 2026-10-18; F23): the server serves the mission catalog, and a `MISSION_ADDITIONS` entry reaches the client at character select without a patch. Option F uses it.
- Give an NPC a new line or a new dialog. Either one is a new dialog row.
- Re-level 1570 to 1646. The journal's level comes from the cooked data, and the server never sends `missions.level` (F4).
- Stage a level-1 practice fight. There is no hostile design (D-DK5).
- Borrow a Dakara E2 or E3 mission. Their texts name other places and assume the later arc.
- Use `system_message`. Its wire format is unknown.

Raising the character to level 3 on arrival with `grant_xp` works on the server alone, but it reverses Class Start v6's level-1 start (`PROJECT_FINAL_S2C`). It is not offered here.

| Option | What the player sees in the first minutes | NEW CONTENT | Packets and size | Risk | How it feeds 1570 and 1645 |
|---|---|---|---|---|---|
| **E1. Teal'c's letter** | They land on the plaza, and chat shows two mail lines. The letter gives the backstory; the approved notice sends them to Bra'tac. The journal shows 1570 "SG-18" at step 4906 with the two gate steps done, which is now the trip the letter describes. | The letter (sender, subject, body), chain 8007 in `dakara_e1_space_chains.sql` (block 8007-8039, space). Labelled `NEW CONTENT` in its seed comment. | Folded into DK-10e, which grows from about 15k to about 22k tokens: one chain, one more chain-replay assertion, one more line on DK-U2. | Low. The player may talk to an NPC before opening the mail. A letter signed by a canon character is an owner call; a neutral sender also works. | Chain 8040 is unchanged. 5811's "Welcome back" and "Teal'c has asked me" become true. 1645's offer (5807) and 1646's (5819) still follow 1570 and are unchanged. |
| **E2. Letter and hail** (recommended) | E1, plus a hail. Rak'nor (spawn 8006, a few metres away on the plaza) speaks his own shipped line 102197 in chat: "Welcome back to Dakara. Did the Tau'ri listen to Master Bra'tac's warning?". The first thing on screen is then a named Jaffa asking for the news. Clicking him plays 6110, which sends the player to the command tent. | The letter, as in E1. The hail is client text; only its chain is new (`RECONSTRUCTION`). | DK-10e as in E1, plus a new **DK-10f** in wave 3 (after DK-10a and DK-10b, about 15k tokens). It adds two chains in `dakara_e1_m1570_entry_chains.sql`, both `archetype eq 7`: 8041 on first arrival, gated 1570 `not_active` (conditions are read before 8040 runs), and 8042 on a relog, gated step 4906 active and objective 5936 not completed. Chain-replay tests run through `live-db-test.ps1`, and DK-U rows are added. | Low to medium. The hail repeats at each login until the player talks to Rak'nor; objective 5936 bounds it. Whether a bark sent at `player_loaded` shows before the world is drawn is a lab UAT item. The line repeats 6110's first screen. | As in E1. Rak'nor's optional 5936 becomes the natural first click, then Bra'tac's 5811. |
| **E3. Newcomer dialogs** | No letter. The approved notice sends them to Bra'tac, who greets them with his other shipped variant, 5810: "Welcome to Dakara. You bring news about the warning we sent?". Rak'nor's 6110 is not bound for them. Loth'ta's 5813 ("Welcome, Jaffa! Are you the first of my reinforcements?") already fits a newcomer. | None. | DK-10b's rows 8045, 8051 and 8052 get new gates; its size is unchanged. Needs an exception to D-DK15 for step 4906. | Medium, and it is a story risk. 5810 drops Moh'katan's "The alliance with earth becomes a stone around our necks" (screen 97888), the first hint of her betrayal in 1649 and 1650. The player still answers with SGC news they never saw. Praxis Jaffa visitors need their own split. | 1570 runs with 5810. 1645 and 1646 are unchanged, but 1649 and 1650 lose their setup. |

Under C, none of the three works well. 1645's offer (5807, "Master Bra'tac, I need to speak with Loth'ta already") and 1646's (5819, "The death of SG-18 is very serious") both assume 1570 was played. Option E is therefore B plus a bridge.

~~**Recommendation: B with E2.**~~ Superseded by option F.

**Letter** (`NEW CONTENT`; accepted by the owner as written, 2026-10-18; kept under F as chain 8007). Sender `Teal'c`, subject `Dakara: find SG-18`, body: `You carried Master Bra'tac's warning to Stargate Command, but Ba'al's Jaffa reached the base first. They came through with SG-18's iris code, and SG-18 has not reported from Dakara since. Now that you are home, tell Master Bra'tac what you saw at the SGC, and find SG-18.` E2's extra sentence is dropped: under F the journal names Rak'nor.

### D-DK1 option F: an authored level-1 bridge mission

**The design.** One new mission, the smallest shape the client is known to accept: the #1734 fixture's one step, one objective and one task, with new names and text. A Free Jaffa arriving on Dakara for the first time gets it; talking to Rak'nor finishes it; finishing it starts 1570 at its first Dakara step. 1570 keeps its own level 3 in the journal (nothing reads it, F4).

| Id | Row | Text (`NEW CONTENT`) |
|---|---|---|
| 63100 | Mission, `Level="1"`, `MissionLabel="General"`, `IsAStory="true"`, `CanAbandon="false"`, no reward (D-DK7) | `MissionDefn` and `HistoryText`: `Home to Dakara` |
| 63101 | Step | `Speak with Rak'nor at the Stargate.` |
| 63102 | Objective (required) | `Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command.` |
| 63103 | Task, `TaskType="1"` | none |

**How it reaches the client, with no patch.** DK-10fs adds one `MissionAddition` (mission 63100, a frozen 1206-byte XML) to production `MISSION_ADDITIONS`, the seed rows for the server's loaders in a new `db/resources/Missions/Seed/missions_dakara_e1.sql`, and the tests. The served mission category gains one entry and its version bumps; at character select every client sees the new version and downloads the category again, once. The journal's title, level, step and objective text come from that XML. No dialog row is new: Rak'nor's click plays the shipped 6110, whose first line already fits ("Welcome back to Dakara. Did the Tau'ri listen to Master Bra'tac's warning?"). The other new text is mail: the reworded notice (8002) and Teal'c's letter (8007), both in DK-10e.

**The flow** (chains in `dakara_e1_bridge_chains.sql`, block 8075-8079, DK-10fc; 8040 in DK-10a):

| Chain | Trigger | Conditions | Actions |
|---|---|---|---|
| 8075 | `player_loaded Dakara_E1` | world 61; `archetype eq 7`; 63100, 1570 and 1645 all `not_active` (so never again once any of them is active or completed) | `accept_mission 63100`; bind Rak'nor (spawn 8006, tag `Dakara_E1_Raknor`) to dialog set 6890 at 441 |
| 8076 | `player_loaded Dakara_E1` | world 61; `archetype eq 7`; step 63101 active | the same bind (relog restore; disjoint from 8075) |
| 8077 | interact `Dakara_E1_Raknor` | world 61; `archetype eq 7`; step 63101 active | `display_dialog 6110`; unbind 6890; `complete_mission 63100` |
| 8040 | `mission_completed 63100` | world 61; `archetype eq 7`; 1570 `not_active` | `accept_mission 1570`; advance 4641 to 4642 to 4906; `complete_objective 5936` (Rak'nor's optional objective, already done in this conversation) |
| 8002, 8007 | `player_loaded Dakara_E1` (space chains, once per character) | `archetype eq 7` | the notice and the letter (DK-10e) |

Chain-completed missions fire `mission_completed` inline (`executor/mission.rs`, `event_dispatch::fire_mission_completed`), so 8040 runs in the same click. 5936 is optional and not the step's last required objective, so it completes nothing else (F7), and DK-10b's 4906 chains then take over at Bra'tac.

**What the player sees.** They land on the plaza; the journal shows "Home to Dakara", level 1, "Speak with Rak'nor at the Stargate."; chat shows two mail lines (the notice and Teal'c's letter). Rak'nor stands a few metres away. Clicking him plays 6110, which ends by sending them to Bra'tac in the command tent; the journal moves "Home to Dakara" to completed and shows 1570 "SG-18" at "Speak to Bra'tac at his command tent.", with its two SGC steps marked done (the trip the letter describes) and "(Optional) Speak with Rak'nor to determine the location of Bra'tac's command tent." already ticked.

**Risks.**

- First live use of the #1734 path (D-DK20). The client might reject or misread the entry; the owner's M2 run is the proof (DK-U11 to DK-U16).
- Every client re-downloads the mission category once (about 2-3 s at character select) after the deploy.
- An apostrophe in `Rak'nor` is sent raw in the XML, as stock dialog text does; DK-U12 checks the journal shows it.
- `CanAbandon="false"`: the mission cannot be dropped, so a player who never clicks Rak'nor keeps it. Rak'nor never moves (F18 does not apply: template 441 is Dakara's own).

#### Fallback

If DK-U11 or DK-U12 fails and the cause is not quick to fix, fall back to **B with E1**: revert DK-10fs and DK-10fc (one PR each, so one revert each), set 8040's trigger back to `player_loaded Dakara_E1` with 1570 and 1645 `not_active` conditions and drop its `complete_objective 5936`, and restore DK-10b's 8057 bind. DK-10e's notice and letter stay. The DK-10a packet text keeps both variants so this is a seed edit, not a redesign.

## Blockers

| Id | Dependency | Status | Folded into |
|---|---|---|---|
| B-DK1 | PR #1273 (CS-02): a Free Jaffa starts on Dakara, respawner 610. | **Resolved**: #1273 merged. M1 is runnable. | M1 |
| B-DK2 | No coordinate for any story actor, object or area. | **Lifted** by DK-02's map estimates; the five drop locations remain | DK-12a (D-DK13) |
| B-DK3 | No hostile design for Dakara. | Open | D-DK5, GDK1 |
| B-DK4 | The SGC cannot be left. | Open, outside this campaign (SGU route campaign) | DK-30, DK-33 |
| B-DK5 | Radio beats need an NPC speaker with no NPC present. | Open | DK-06a, DK-06b, D-DK10 |
| B-DK6 | No client map for worlds 24, 63, 64, 65. | Permanent | Non-goal |

## Matrices

Legend, as in the Class Start v6 ledger: **BUILD** can be implemented now; **BUILD-INFRA** needs this campaign's own infrastructure packets first; **BLOCKED** waits on something outside the packet (a blocker or an owner decision). A row reaches PASS only after implementation, tests and the lab UAT.

### Missions

| Seq | Mission | Where | Status | Packets |
|---|---|---|---|---|
| 00 | 63100 Home to Dakara (`NEW CONTENT`, D-DK1 F) | Dakara | BLOCKED (D-DK1, D-DK20) | DK-10fs, DK-10fc |
| 01 | 1570 SG-18, steps 4641-4642 | SGC | BLOCKED (route campaign) | DK-10a advances past them (D-DK6) |
| 01 | 1570 SG-18, steps 4906-4903 | Dakara | BLOCKED (D-DK1) | DK-10a, DK-10b, DK-10c (DK-08, D-DK18), DK-10d (DK-06b) |
| 02 | 1645 Withdrawal Orders | Dakara | BLOCKED (D-DK1) | DK-11 |
| 03 | 1646 Moh'katan's Scouts | Dakara | BUILD-INFRA | DK-12a, DK-09, DK-12b, DK-12c |
| 04 | 1647 Enemy at the Gates | Dakara | BLOCKED (D-DK5) | GDK1, DK-20, DK-21; DK-06b for step 4916 |
| 05 | 1648 Loth'ta's Withdrawal | Dakara | BUILD-INFRA | DK-13; follows 1647 in the story |
| 06 | 1649 Moh'katan | Dakara | BUILD-INFRA | DK-14a, DK-14b; DK-06b for step 4921 |
| 07 | 1650 Confront Moh'katan | Dakara, tent interior | BUILD-INFRA | DK-15a, DK-15b |
| 08 | 1651 Renewed Attack | Dakara, SGC, Dakara | BLOCKED (B-DK4) | DK-30 |
| 09 | 1652 Fireball | Dakara | BUILD-INFRA | DK-31a, DK-31b; guards are D-DK5 |
| 10 | 1653 Superweapon | Dakara | BLOCKED (D-DK5) | DK-32; DK-06b for step 4937 |
| 11 | 1654 Aftermath | Dakara to SGC | BLOCKED (B-DK4) | DK-33 |
| 12 | 1655 Shutdown | SGC, Omega Site | BLOCKED | Outside this campaign |

### Systems

| System | Status | Packet |
|---|---|---|
| Free Jaffa start point and respawner 610 | Merged (#1273) | CS-02 |
| First stargate address, gate 25 arrival pin, arrival notice | Integrated (#1284); M1 UAT pending | DK-01 |
| Landmark census and placement ledger | Done (#1283) | DK-02 |
| Cast templates and interactable props | Integrated (#1285); M2 UAT pending | DK-03 |
| Named areas, tent travel, Med Tent respawner | Integrated (#1287); M2 UAT pending | DK-04 |
| Static population | Integrated (#1288); M2 UAT pending | DK-05 |
| Chain block registered | Ready | DK-07 |
| Any-order objective pattern | Ready (ruling) | DK-08 |
| Radio dialog | Ready (evidence), then BlockedDependency | DK-06a, DK-06b |
| SG-18 remains, their search regions and drop-location props placed | BlockedDependency | DK-12a, DK-09, DK-09b |
| Hostile roster, encounters, world-61 cover sets | BlockedDecision (D-DK5) | GDK1, DK-20 |
| Ha'tak show and explode sequences (event sets 1194, 1195) | BUILD-INFRA | DK-31a, DK-31b |
| A way out of the SGC | BLOCKED (B-DK4) | SGU route campaign |
| Vendors, trainers, ambient population | BLOCKED (no data) | none |
| Mission rewards | BLOCKED (GC3, D-DK7) | none |
| Navmesh containment for world 61 (`advisory` to `enforce`) | Not in this campaign | none |
| Dakara_E2, E3, Superweapon, Dakara (24) | BLOCKED (B-DK6) | none |

## Packets

This table is the only packet status. Rules, contract and packet text: [work-packets.md](work-packets.md) and [packets/](packets/). **Current wave: 2.**

| Packet | Wave | Scope | Depends on | Role | Status |
|---|---|---|---|---|---|
| DK-00 | - | Audit, ledger, packets | none | architect | Done (#1282) |
| DK-01 | - | Dead end removed (record split DK-01a/b/c) | #1273 | - | Integrated (#1284), UATPending (M1); [worknote](worknotes/DK-01.md) |
| DK-02 | - | Landmark census, placement ledger | none | - | Done (#1283); [worknote](worknotes/DK-02.md) |
| DK-03 | - | Templates 440-457 | none | - | Integrated (#1285), UATPending (M2); [worknote](worknotes/DK-03.md) |
| DK-04 | - | Named areas, tent flaps, Med Tent respawner | DK-02, DK-03 | - | Integrated (#1287), UATPending (M2); [worknote](worknotes/DK-04.md) |
| DK-05 | - | Static cast spawns 8004-8007, Repository point set 2127 | DK-02, DK-03 | - | Integrated (#1288), UATPending (M2); [worknote](worknotes/DK-05.md) |
| DK-06a | **1** | Radio evidence: how a speaker-2609 dialog can show with no NPC | none | `game-archaeology-specialist` | Done (#1854) |
| DK-07 | **1** | Register chain block 8001-8499 in the four-zone id allocation | none | packet-coder | Done (#1853) |
| DK-08 | **1** | Any-order objective ruling with exact rows (D-DK12) | none | `mission-systems-advisor` | Done (#1855) |
| DK-12a | **1** | Five drop-location estimates PL-DK-A-17 to A-21 | none | `movement-teleport-advisor` | Done (#1856) |
| DK-06b | 2 | Radio display, per the D-DK10 ruling | DK-06a, D-DK10 | packet-coder or `rust-gameserver-dev` (per ruling) | BlockedDecision (D-DK10) |
| DK-09 | 2 | Remains spawns, drop templates 458-461 and spawns, SG-18 search regions 2128-2130 (D-DK18), on-mesh guards | DK-08, DK-12a | packet-coder | Ready |
| DK-09b | 2 | Linter: region check moved to its own file, reads every `point_sets*.sql` (F20) | none | packet-coder | Ready |
| DK-10a | 2 | 1570 entry chain 8040 (under F: on 63100's completion), eight seed-file headers incl. the bridge file, test scaffolding, linter ranges | D-DK1 (F, decided) | packet-coder | Ready |
| DK-10e | 2 | Reword the arrival notice (8002) and send Teal'c's letter (8007) | D-DK1 (F), D-DK17 (decided) | packet-coder | Ready |
| DK-10fs | 2 | Mission 63100 "Home to Dakara": `MISSION_ADDITIONS` entry, seed rows, tests ([packet](packets/wave-2-dk-10fs.md)) | D-DK1 (F), D-DK20 (decided) | packet-coder | Ready |
| DK-10b | 3 | 1570 steps 4906 and 4904: Rak'nor, Bra'tac, Loth'ta (under F without the 8057 bind) | DK-10a | packet-coder | BlockedDependency (via DK-10a) |
| DK-10fc | 3 | Bridge chains 8075-8077: accept 63100, Rak'nor bind and restore, complete on his click | DK-10a, DK-10fs | packet-coder | BlockedDependency (via DK-10a, DK-10fs) |
| DK-10c | 3 | 1570 steps 4902 (search regions) and 4901 (SG-18 remains), chains 8060-8074 | DK-10a, DK-08, DK-09, DK-09b | packet-coder | BlockedDependency (via DK-10a, DK-09, DK-09b) |
| DK-10d | 3 | 1570 steps 4905 and 4903: radio report, Bra'tac turn-in, chains 8160-8179 (D-DK19) | DK-10a, DK-06b | packet-coder | BlockedDependency (via DK-10a, DK-06b) |
| DK-11 | 3 | Mission 1645 | DK-10a (gate per D-DK1) | packet-coder | BlockedDependency (via DK-10a) |
| DK-12b | 3 | 1646 offer, accept and turn-in | DK-10a, DK-11 contract | packet-coder | BlockedDependency |
| DK-12c | 3 | 1646 five drops | DK-10a, DK-08, DK-09 | packet-coder | BlockedDependency |
| GDK1 | 4 | Encounter design gate for 1647, 1652, 1653 | D-DK5, M2 playtest | `npc-ai-spawn-advisor` | BlockedDecision (D-DK5) |
| DK-20 | 4 | Hostile roster, cover sets | GDK1 | packet-coder | BlockedDecision (D-DK5) |
| DK-21 | 4 | Mission 1647 | DK-20, DK-06b, DK-12c | packet-coder | BlockedDecision (D-DK5) |
| DK-13 | 4 | Mission 1648 | DK-21 | packet-coder | BlockedDependency |
| DK-14a | 4 | Mission 1649 offer and radio | DK-13, DK-06b | packet-coder | BlockedDependency |
| DK-14b | 4 | Mission 1649 hack, drops, decode, turn-in | DK-14a | packet-coder | BlockedDependency |
| DK-15a | 5 | Mission 1650 confrontation | DK-14b | packet-coder | BlockedDependency |
| DK-15b | 5 | Mission 1650 terminal search, turn-in | DK-15a | packet-coder | BlockedDependency |
| DK-30 | 5 | Mission 1651 Dakara steps, SGC address grant | DK-15b, B-DK4 | packet-coder | BlockedDependency |
| DK-31a | 5 | Mission 1652 ring-control evidence (`play_sequence` id kind) | DK-30 | `rust-gameserver-dev` | BlockedDependency |
| DK-31b | 5 | Mission 1652 chains | DK-31a | packet-coder | BlockedDependency |
| DK-32 | 5 | Mission 1653 | DK-31b, DK-20 | packet-coder | BlockedDecision (D-DK5) |
| DK-33 | 6 | Mission 1654 exit boundary | DK-32, B-DK4 | packet-coder | BlockedDependency |
| DK-99 | 6 | Close-out | all | `documentation-writer`, ops | BlockedDependency |

Wave 1 has no internal dependencies: dispatch all four at once. Wave 2's DK-09 needs both DK-08 and DK-12a merged; DK-10a and DK-10e need only D-DK1; DK-10fs needs D-DK1 (F) and D-DK20; DK-09b needs nothing. Wave 3's DK-10fc needs DK-10a and DK-10fs merged. Every wave is reviewed by one read-only `packet-reviewer`; a `packet-coder` (Haiku) applies its findings. No `packet-review-fixer` (owner, 2026-10-18).

## The Dead-End Fix, Exactly

DK-01 is the whole minimum. With #1273 merged, a level-1 Free Jaffa who lands on the gate plaza:

1. **Can leave and return.** A world-61 chain on `player_loaded Dakara_E1`, gated `archetype eq 7`, runs `grant_stargate_address 5`. The executor arm is idempotent, updates the open client with `updateStargateAddress` and persists to `sgw_player.known_stargates`. The DHD (spawn 38) is 7.6 m from the start point and then lists Omega Site. At Omega Site the arrival unlock teaches gate 25, Omega's DHD (spawn 41) lists Dakara E1, and gate 25 gets an `arrival_*` pin on the plaza so the return does not land inside the gate volume.
2. **Has a working respawn.** Respawner 610 and the profile start point come from #1273; DK-01 adds nothing and its UAT checks it.
3. **Is told the truth.** One system mail, once per character, from a project-authored sender, saying the Free Jaffa command is not staffed yet and the gate is open to Omega Site. It is `NEW CONTENT` and says so in its seed comment. The mail action needed one small change so a repeat firing is silent ([DK-01](work-packets.md#dk-01)); the address and the pin do not depend on it. There is no first mission in the minimum (D-DK1, option A).

**How this sits with OD-CS10.** OD-CS10 keeps SGU route authoring out of v6 and names 1645-1653, 1654 and 1655 as a follow-up campaign. DK-01 authors no mission, binds no dialog and touches none of those ids; it is "start infrastructure that later world transitions can use", which OD-CS10 lists as v6's own. This campaign is that follow-up for worlds 61 and 62; the first mission arrives in DK-10a to DK-10d (1570's Dakara half), then DK-11 (1645) and DK-12b and DK-12c (1646). Missions 1645-1655 keep their seeded level 3 to 5 and the start level stays 1 (F4).

## Where Confidence Is Low Or A Guess

- Every position. One recovered line helps: Rak'nor says the command tent is "just to the east of the Stargate, next to the healing tent" (dialog 6110). The SG-18 sites are off the plaza mesh (F8) and the drop locations are unplaced (F9).
- Whether both tents were meant to share the one interior map.
- How "use your radio" was triggered. No radio item, ability or object is named in the mission data (DK-06a).
- What a client does on a gate arrival at world 61, and whether the mail window is a good place for the notice. Both are lab UAT items.
- Whether 1570's search sites and 1652's plazas were guarded. The dialog says the areas "have seen heavy fighting"; no objective asks for a kill.
- The offer rules. Order comes from the name strings' sequence numbers; nothing says what each mission required.

## Architecture Guardrails

The [Harset guardrails](../harset-rebuild/README.md#architecture-guardrails) apply unchanged. The authoring rules every mission packet carries are in [work-packets.md, C5](work-packets.md#c5-authoring-rules-every-chain-packet-follows). The ones this zone hits first:

- One event fires every matching chain; sibling chains on one trigger must have disjoint conditions. The Tau'ri and Jaffa branches of 1570 are siblings split on `archetype` (D-DK15).
- `complete_objective` on a step's last required objective completes the whole mission (F7).
- A dialog-set bind is per player and in memory; every chain that binds one needs a `player_loaded Dakara_E1` (or `Dakara_E1_StoryRm`) restore chain gated on the active step.
- `display_dialog` from `dialog_choice`, `player_loaded` or a deferred action only shows an all-speaker-0 dialog unless an interaction pinned a speaker.
- `OnRegionEnter` does not filter by world; region keys are byte-exact `point_sets.name` values.
- `cross_world_teleport` matches `spaces.xml` world names exactly and cannot set facing.
- Shared NPCs on world 61 are never hidden, destroyed, moved or made hostile by mission content. Per-player scenes go in the instanced tent (D-DK4) or use mission-scoped `spawn_entity` tags.
- Bra'tac (59) and Moh'katan (54) are shared templates; Harset spawns Moh'katan. A Dakara change to either template is a change to Harset.
- No new dialog rows and no cooked-data override: the client holds every Dakara dialog already. The one exception is DK-10fs's `MISSION_ADDITIONS` entry for mission 63100 (D-DK1 F), whose XML is frozen and pinned by a test.

## Validation And UAT Gates

Tests follow [TESTING.md](../../../TESTING.md): a chain-replay test (type 6, live-DB here, F14) per chain packet that fails when the seed rows are removed, a live-DB guard (type 3) for seed relationships, a navmesh guard for every coordinate, and a wire-format test (type 2) for anything that emits a new message. **No lab use until the campaign ends (owner, 2026-10-18):** the owner runs every UAT below himself; no agent or packet worker starts a lab session or a client.

| Milestone | Packets | Acceptance in the lab (all pending) |
|---|---|---|
| M1 Dead end removed | DK-01 (with #1273) | New Free Jaffa: the notice arrives once; the DHD lists Omega Site; dial, cross, arrive standing at Omega Site; Omega's DHD lists Dakara E1; return lands on the plaza outside the gate; relog keeps both addresses; a death respawns at the plaza. A Human visitor gets no grant from the chain. Runnable now (B-DK1 resolved); CS-08 covers the arrival only (F12). |
| M2 Arrival story | DK-02 to DK-05, waves 1 to 3 | The bridge mission 63100 shows and completes ([M2 bridge checklist](#m2-bridge-checklist-mission-63100), DK-U11 to DK-U16); Bra'tac, Moh'katan and Loth'ta are where the ledger says and survive a relog; tent flaps work both ways; 1570's Dakara half, 1645 and 1646 complete in order with a relog at every step; the owner corrects positions in one pass; the owner answers D-DK5. |
| M3 Betrayal | wave 4, DK-15a, DK-15b | The gate defence is winnable at the player's level; radio beats show; the terminal hack, the search and the decode work; the tent confrontation plays for one player without changing what another sees. |
| M4 Climax | DK-30 to DK-32 | SGC round trip with the generators; both Ha'taks appear and explode for the acting player; courtyard objectives complete. |
| M5 Exit | DK-33, DK-99 | 1654 hands off to the SGC once; final PASS / BLOCKED matrix. |

### M1 checklist: dead end removed

DK-01's lab steps, mirrored in the [unified UAT guide](../../guides/unified-uat.md#dakara_e1) under the same ids. All pending. #1273 is merged, so a new Free Jaffa starts on Dakara; CS-08's lab run saw that arrival (DK-U1's first sentence) but none of the steps below.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| DK-U1 | Create a new Free Jaffa (SGU, Jaffa body) and enter the world | You stand on the Dakara gate plaza. Within a few seconds chat shows "Dakara Gate Watch sent you mail N. Open your mail to read it." and the new-mail notice | `.gotolocation Dakara_E1` on an existing Free Jaffa fires the same chains |
| DK-U2 | Open your mail | One mail from **Dakara Gate Watch**, subject "Dakara: no orders yet", three sentences (no orders yet; the DHD dials Omega Site; Omega's DHD brings you back), no cash, no item | Whether the mail window is the right place for the notice is an open question: say what you think. |
| DK-U3 | Right-click the DHD in front of the gate | The dialling window lists **Omega Site** with its name and glyphs | A row reading "Unknown" is a fail: note whether reopening the DHD fixes it |
| DK-U4 | Dial Omega Site and walk into the gate | You arrive at Omega Site, standing, able to walk | |
| DK-U5 | At Omega Site, right-click the DHD | It lists **Dakara E1** | The arrival taught it. Omega Site has no respawner: do not die there |
| DK-U6 | Dial Dakara E1 and walk into the gate | You land on the plaza about 10 m in front of the gate, past the foot of its steps, facing away from the gate toward the DHD, and can walk at once | A landing inside the gate ring, on the steps, in the air or under the floor is a fail: say where you stood. The point is `(96.87, -16.75, 243.23)` |
| DK-U7 | Log out and back in on Dakara | No second mail and no "has already sent you mail" line. The DHD still lists Omega Site | The repeat is silent on purpose |
| DK-U8 | Delete the notice, then relog | Still no second mail | Once per character |
| DK-U9 | Die on Dakara and respawn | You respawn on the gate plaza | Respawner 610 comes from #1273. Nothing on Dakara is hostile yet, so it needs a GM kill; if you cannot die, record the step as not run |
| DK-U10 | As a Human or a Praxis Jaffa (a GM visitor), `.gotolocation Dakara_E1` | The reply ends in `[stargate arrival]` and you stand on the DK-U6 point. No mail arrives, and your DHD list has nothing new from this visit | The chains are for archetype 7 only |

### M2 bridge checklist: mission 63100

Runs after waves 2 and 3 are deployed (DK-10a, DK-10e, DK-10fs, DK-10fc), on a new Free Jaffa. All pending; the owner runs it (D-DK1 F, D-DK20). DK-99 copies it to the unified UAT guide.

| # | Do | Expect | Notes / known issues |
|---|---|---|---|
| DK-U11 | Start the client and reach character select after the deploy | Character select takes a few seconds longer once (the mission catalog downloads again); no error, no crash. The next login is as fast as before | The first live use of a `MISSION_ADDITIONS` entry (D-DK20). A crash or a hang here: stop and note the time |
| DK-U12 | Create a new Free Jaffa, enter the world, open the journal | "Home to Dakara", level 1, step "Speak with Rak'nor at the Stargate.", objective "Rak'nor keeps watch over the Stargate plaza. Tell him what you saw at Stargate Command." Both apostrophes show as `'`. 1570 "SG-18" is not in the journal yet | A blank title, a missing step or a garbled apostrophe is a fail: screenshot the journal |
| DK-U13 | Open your mail | Two mails: **Dakara Gate Watch**, "Dakara: report to Bra'tac" (Bra'tac in the command tent; the DHD dials Omega Site; Omega's DHD brings you back) and **Teal'c**, "Dakara: find SG-18" (the warning reached the SGC too late; SG-18 is missing; tell Bra'tac) | Say whether the letter reads well before or after the Rak'nor click |
| DK-U14 | Log out and back in before talking to Rak'nor, then click Rak'nor | "Home to Dakara" is still active and no third mail arrives. Rak'nor plays "Welcome back to Dakara. Did the Tau'ri listen to Master Bra'tac's warning?" (6110) to its end | Checks the relog restore (chain 8076) |
| DK-U15 | After 6110 closes, open the journal | "Home to Dakara" is completed. 1570 "SG-18" is active at "Speak to Bra'tac at his command tent.", its two SGC steps marked done and "(Optional) Speak with Rak'nor to determine the location of Bra'tac's command tent." ticked. Clicking Rak'nor again plays nothing new from this mission | 1570 then runs as in the rest of M2 |
| DK-U16 | Relog. Then, as a Praxis Jaffa or a Human (a GM visitor), `.gotolocation Dakara_E1` and click Rak'nor | The Free Jaffa gets no second "Home to Dakara". The visitor gets no "Home to Dakara", no mail and no 1570 from arrival | Chains 8075-8077 are for archetype 7 only |

## Handoff Validation Record

| Check | Recorded outcome |
|---|---|
| Archive | `00_README.md`, the three `HANDOFFS/` files, `REPORTS/00_FINAL_SUMMARY.txt`, `10_WORLD_CONTENT_SUMMARY.csv`, `12_MISSING_UNRECOVERED_WORLDS.csv`, both Dakara_E1 world folders (17 files each) and the four missing-world evidence notes read. Nine of Dakara_E1's CSVs and fourteen of the StoryRm's are 10-byte `EMPTY` files. Nothing from the archive is committed. |
| Client files | Read-only: map directory listing and SHA-256 of all Dakara tiles; `extract_actors.py` and `kismet_extractor.py --survey` on both maps (with the reader's `lzallright` dependency in a throwaway environment); five cooked-data PAK entries opened. No client, launcher or lab tool was started. |
| Seed and repo | `db/resources/**` queried from a recent load and cited by file; `entities/spaces.xml`, `cell_spaces.xml`, `data/spaces/README.md`; the CS-02 branch read with `git show`; the earlier handoff pack's three Dakara workbooks. 2026-10-18 re-slice: seed files, `progression.rs`, `actions.rs`, the interact-tag linter and the 1324 replay helpers read on `main` @ `d1bba0400`. |
| Not done | No build, no test, no Rust extractor, no client session. Listed in the [audit](audit.md#what-could-not-be-checked). |
