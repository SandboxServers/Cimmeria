# Dakara_E1 Rebuild

> Type: ledger. Audience: the owner, the coordinator, packet workers and reviewers.
> Opened 2026-10-06 against `main` @ `ddd549797`. Packet prefix `DK-`. Companions: [audit](audit.md), [work packets](work-packets.md), [Class Start v6 ledger](../class-start-v6/README.md), [Harset rebuild](../harset-rebuild/README.md), [Castle rebuild](../castle-rebuild/README.md), [operator guide](../zone-restoration-operator-guide.md).
>
> **Campaign status (2026-10-06): planned. DK-00 (this plan) is written; nothing is implemented, built, tested or run in a client.** Every decision below is PROPOSED until the owner answers it.

## Why

Class Start v6 moves the Free Jaffa start (char_defs 8 and 18, archetype 7) from SGC_W1 to Dakara_E1 in PR #1273. That PR's review found the result is a dead end: world 61 has one spawn (the DHD), no hostile, no content chain, and a new character's stargate address book is empty, so a new Free Jaffa has no mission and cannot leave. The owner then supplied an external handoff archive with a `WORLDS` directory for the planet and asked whether it strings together enough of what the database already holds to restore the zone.

## The Answer

**No for a whole zone today, yes for the story once positions are chosen, and the archive is not what makes the difference.**

- **What we have, complete:** the text of the whole Dakara_E1 arc. Twelve missions (1570 from its third step, and 1645-1655) with every step and objective, twelve dialog sets with every line, nine mission items, and the client's names for the cast, the interactable objects, six named areas and one respawner. The client map is complete (401 tiles, identical to the archive's last build), the server has a navmesh for it, and the gate, its volume, its Kismet and the DHD are seeded and match the client.
- **What is missing, everywhere:** where anything stands. No position exists for Bra'tac, Moh'katan, Loth'ta, the tents' entrances, the drop sites, the gates, the Naquadah Repository or the Med Tent. No hostile template, encounter, wave or count exists for the missions that are fights. No offer rule, prerequisite or reward exists for any mission. The seed, the client and the archive agree on this: the archive's spawn, encounter and player-start files for Dakara_E1 are empty.
- **What the archive adds:** a file manifest that proves our client is its final build, and actor rows we can read from the same maps with the repo's own tools. Its mission list is missing four of the twelve missions ([audit, claims 2 and 3](audit.md#handoff-claims-used-and-their-verdicts)). Nothing in it needs importing.
- **How much is playable from that:** the dead end goes away with one small packet and no new evidence (DK-01). Seven of the twelve missions have no fight in their objectives: talk, carry, search, hack and two device steps (1570's Dakara half, 1645, 1646, 1648, 1649, 1650, 1652). They are authorable once positions are estimated from the map and corrected in one playtest pass. In story order the peaceful run is 1570, 1645, 1646; then 1647, a fight, sits in front of the rest. Two missions are fights with no recovered design (1647, 1653) and need the owner's go-ahead to be reconstructed. Three leave for the SGC (1651, 1654, 1655) and wait on a way to dial out of the SGC.
- **One thing the owner should know before deciding anything:** the recovered arc is not a level-1 start. It is level 3 to 5, and its first Dakara scene tells a Jaffa player "Welcome back, my friend" after the SGC attack. In the client's data the Free Jaffa reaches Dakara from the SGC. A level-1 Dakara start is this project's decision, so its first minutes have to be reconstruction or new content whatever we do (OD-DK01).

The four other Dakara worlds in the archive's `MISSING_UNRECOVERED` folder (`Dakara` 24, `Dakara_E2` 63, `Dakara_E3` 64, `Dakara_Superweapon` 65) have no map in the client and none in the archive. Their folders are name-matched asset lists the archive itself grades `CANDIDATE_NOT_PROVEN`. The fifty missions labelled Dakara E2 and E3 cannot be placed in a world and are outside this campaign.

## Goal And Non-Goals

**Goal.** A Free Jaffa who starts on Dakara_E1 can always leave and come back, has a first mission, and can play the Dakara arc as far as the evidence and the owner's decisions allow, with every reconstructed row labelled as reconstruction.

**Non-goals.**

- Dakara_E2, Dakara_E3, Dakara_Superweapon, Dakara (24) and MissionTestDakara (30): no client map.
- The SGC side of 1570, 1651, 1654 and all of 1655: the SGU route campaign (OD-CS10) owns them. This campaign owns what happens on worlds 61 and 62.
- Vendors, trainers and ambient population. The merchant tent on the map has no vendor data (Harset's D-H09 applies).
- Mission XP, cash and item rewards. All twelve missions have none seeded; the Cellblock's GC3 gate (formula evidence first) applies unchanged.
- Changing where Free Jaffa start. That is Class Start v6's decision; this ledger records the evidence and offers it as an option (OD-DK01, option D).
- New client content: no new dialog rows, no cooked-data overrides, no client patch.

## Evidence Labels

Seed rows and worknotes in this campaign use the labels the earlier campaigns use:

| Label | Use |
|---|---|
| `ORIGINAL_DATA` | The row restates data that is in the client's own cooked files (a mission step, a dialog id, a name string, a map actor). It never means CME server data; the seed is a reconstruction ([rules-and-gotchas](../../agents/rules-and-gotchas.md)). |
| `RECONSTRUCTION` / `PROJECT_FINAL_RECONSTRUCTION` | Authored by this project to connect recovered pieces: offer rules, step triggers, positions, encounters. |
| `NEW CONTENT` | Text or behaviour with no recovered counterpart (the arrival notice). |
| `PROJECT_FINAL_S2C` | Inherited from Class Start v6: the Dakara start, level 1, start gear. |
| Placement classes | `AUTHORED`, `MAP-LANDMARK`, `MAP-GEOMETRY`, `MAP-MARKER`, `SPEC-DESCRIPTIVE`, `INFERRED`, with HIGH / MEDIUM / LOW / NO-IDEA confidence, exactly as in the [Harset placement method](../harset-rebuild/placements/METHOD.md). |

Handoff claims carry CONFIRMED / CONTRADICTED / UNVERIFIABLE in the [audit](audit.md#handoff-claims-used-and-their-verdicts).

## Owner Decisions

All PROPOSED. Each has the options and a recommendation; confidence is in the evidence, not the effort.

| Id | Decision | Options | Recommendation |
|---|---|---|---|
| OD-DK01 | **What a level-1 Free Jaffa is given first.** The first recovered Dakara beat is mission 1570 step 4906 (level 3, "Welcome back"); nothing recovered is level 1 or 2. | **A.** No mission in the minimum: an address, a notice, a respawn (DK-01). **B.** After A, mission 1570 accepted on arrival and advanced past its two SGC steps, played with its Jaffa dialogs (DK-10). The log shows two completed SGC steps and Bra'tac's lines mention an SGC attack the character did not play. **C.** After A, skip 1570 and start at 1645; its offer and 1646's both refer to SG-18. **D.** Return the Free Jaffa start to SGC_W1 (the client's order) and treat Dakara as the level-3 zone it was written as. A v6 decision, not this campaign's. | **A now, then B.** A needs no placement and removes the dead end. B is the client's own first Dakara mission, all of its Dakara steps are peaceful, and it sets up 1645 and 1646. Label it `PROJECT_FINAL_RECONSTRUCTION`. HIGH on the facts. |
| OD-DK02 | **Which address the Dakara start grants.** | **Omega Site (gate 5):** has a DHD, so the trip is two-way; the SGU hub; mission 1655 is where the client's story hands it out, so granting it at the start is early. **SGC W1 (gate 27):** the address the story assumes by 1651, but the SGC has no DHD and the gate row is under the floor, so today it is a one-way trip. **Both.** **None.** | **Omega Site only**, in DK-01. Add the SGC address in DK-30, when the SGC can dial out. HIGH. |
| OD-DK03 | **How positions are chosen.** No coordinate survives for any story actor; one dialog line places the command tent "just to the east of the Stargate". | **Map estimate:** place from prefab landmarks, floor and navmesh, label each row, keep one ledger, correct in one pass after a playtest (the Harset method, 2026-09-19). **Owner placement session:** the owner places each actor in a client with `.spawn` / `.savespawn` (Harset D-H15). | **Map estimate first** (DK-02), owner corrects after M2's playtest. MEDIUM: Harset's estimates needed corrections, and the mission texts' "Eastern" and "Western" are not tied to a map axis. |
| OD-DK04 | **Tent interiors.** The step text says to enter the command tent by its entrance flap; the client names four flaps (to and from the command tent and Moh'katan's tent) and ships one tent interior, world 62, instanced. | **One interior for both tents**, populated by mission state. **Command tent only** inside; Moh'katan's scenes outside her tent on world 61. **No interior:** everyone stands outside. | **One interior for both.** It is what the names and the single map imply, and an instanced room gives per-player scenes for 1650 without touching shared NPCs (Harset D-H03). MEDIUM. |
| OD-DK05 | **Fights with no recovered design** (1647 gate defence, 1653 courtyard, and whether 1570's search sites and 1652's plazas are guarded), and their level. A Free Jaffa arrives at level 1; the missions say 3 to 5; no reward gives XP. | **Reconstruct** small encounters at the player's level band from template 35's family, labelled `RECONSTRUCTION`. **Hold** the two combat missions and stop the arc at 1646. **Skip** them with a narrated step. | **Reconstruct**, after M2 is playtested, behind design gate GDK1. Until then the arc stops at 1646. LOW on any particular encounter shape. |
| OD-DK06 | **Where DK-01 lives and who owns the shared missions.** | DK-01 as its own PR in this ledger, merged with or right after #1273; or folded into CS-06. For the SGC legs: this campaign authors world 61/62 steps and the SGU route campaign authors SGC steps, meeting at step ids. | **Own PR, this ledger**, and the Free Jaffa Dakara start is not un-held without it. Step-id split as stated. HIGH. |
| OD-DK07 | **Rewards.** | Inherit GC3 (no XP, cash or items until there is formula evidence); or author provisional rewards. | **Inherit GC3.** HIGH. |

## Blockers

| Id | Dependency | Rows blocked |
|---|---|---|
| B-DK1 | PR #1273 (CS-02) is not merged: on `main` no character starts on Dakara and world 61 has no respawner. | DK-01's UAT (its authoring is not blocked) |
| B-DK2 | No coordinate for any story actor, object or area. **Lifted 2026-10-06 by DK-02** as labelled map estimates ([placements](placements/README.md): 23 rows, mostly LOW; 12 items in its No idea list, notably the Repository, Loth'ta's camp and the five drop locations). | DK-04, DK-05 and the mission packets can start; the Repository, Loth'ta and the drop locations still need the owner |
| B-DK3 | No hostile template, encounter, wave or count for Dakara. | 1647, 1653; DK-20, DK-21, DK-32 (OD-DK05, GDK1) |
| B-DK4 | The SGC cannot be left: worlds 58 and 86 have no DHD and no Harriman dial chain, and gate 27's row is under the floor. | 1651's SGC steps, 1654, 1655; DK-30, DK-33 |
| B-DK5 | Radio beats are NPC-speaker dialogs with no NPC present; `display_dialog` cannot show them from a non-interact chain. | 1570 step 4905, 1647 step 4916, 1649 step 4921, 1653 step 4937; DK-06 |
| B-DK6 | No client map for worlds 24, 63, 64, 65. | All Dakara E2 / E3 / Superweapon content (non-goal) |

## Matrices

Legend, as in the Class Start v6 ledger: **BUILD** can be implemented now; **BUILD-INFRA** needs this campaign's own infrastructure packets first; **BLOCKED** waits on something outside the packet (a blocker above or an owner decision). A row reaches PASS only after implementation, tests and the lab UAT.

### Missions

| Seq | Mission | Where | Status | Needs |
|---|---|---|---|---|
| 01 | 1570 SG-18, steps 4641-4642 | SGC | BLOCKED | SGU route campaign (OD-CS10) |
| 01 | 1570 SG-18, steps 4906-4903 | Dakara | BUILD-INFRA | DK-02 to DK-05; DK-06 for step 4905; OD-DK01 |
| 02 | 1645 Withdrawal Orders | Dakara | BUILD-INFRA | DK-02 to DK-05 |
| 03 | 1646 Moh'katan's Scouts | Dakara | BUILD-INFRA | DK-02 to DK-05 |
| 04 | 1647 Enemy at the Gates | Dakara | BLOCKED | B-DK3 (OD-DK05, GDK1), then DK-20; DK-06 for step 4916 |
| 05 | 1648 Loth'ta's Withdrawal | Dakara | BUILD-INFRA | DK-02 to DK-05; follows 1647 in the story |
| 06 | 1649 Moh'katan | Dakara | BUILD-INFRA | DK-06 for step 4921; Livewire exists |
| 07 | 1650 Confront Moh'katan | Dakara, tent interior | BUILD-INFRA | OD-DK04 |
| 08 | 1651 Renewed Attack | Dakara, SGC, Dakara | BLOCKED | B-DK4 for steps 4930-4929 |
| 09 | 1652 Fireball | Dakara | BUILD-INFRA | Ring-control props, event sets 1194/1195; guarded or not is OD-DK05 |
| 10 | 1653 Superweapon | Dakara | BLOCKED | B-DK3; DK-06 for step 4937 |
| 11 | 1654 Aftermath | Dakara to SGC | BLOCKED | B-DK4 |
| 12 | 1655 Shutdown | SGC, Omega Site | BLOCKED | Outside this campaign (OD-CS10) |

### Systems

| System | Status | Packet |
|---|---|---|
| Free Jaffa start point and respawner 610 | Class Start v6 (#1273, in review) | CS-02 |
| First stargate address for the Dakara start | BUILD | DK-01 |
| Gate 25 arrival pin outside the gate volume | BUILD | DK-01 |
| Arrival notice | BUILD | DK-01 |
| Landmark census and placement ledger | BUILD | DK-02 |
| Cast templates and interactable props | BUILD | DK-03 |
| Named areas, tent travel, Med Tent respawner | BUILD-INFRA | DK-04 |
| Static population and relog restores | BUILD-INFRA | DK-05 |
| Radio dialog | BUILD-INFRA (evidence step first) | DK-06 |
| Hostile roster, encounters, world-61 cover sets | BLOCKED (OD-DK05, GDK1) | DK-20 |
| Ha'tak show and explode sequences (event sets 1194, 1195) | BUILD-INFRA | DK-31 |
| A way out of the SGC | BLOCKED (B-DK4) | SGU route campaign |
| Vendors, trainers, ambient population | BLOCKED (no data) | GDK3, not opened |
| Mission rewards | BLOCKED (GC3) | none |
| Navmesh containment for world 61 (`advisory` to `enforce`) | Not in this campaign | none |
| Dakara_E2, E3, Superweapon, Dakara (24) | BLOCKED (B-DK6) | none |

## Packets

Details, inputs, tests and UAT are in [work-packets.md](work-packets.md).

| Packet | Scope | Depends on | Status |
|---|---|---|---|
| DK-00 | This plan: audit, ledger, packets. | none | Written 2026-10-06 |
| DK-01 | Dead end removed: `grant_stargate_address` on arrival, gate 25 arrival pin, one-time arrival notice, round-trip guards. Seed plus one small mail-action change. | #1273 for UAT; OD-DK02 default | Ready |
| DK-02 | Zone evidence pack: landmark census, floor and navmesh checks, placement ledger. No seed change. | none | Complete 2026-10-06 (#1283): [placements](placements/README.md), [worknote](worknotes/DK-02.md) |
| DK-03 | Cast templates and props (Loth'ta, Rak'nor, Jaffa Captain, tent flaps, drop location, SG-18 remains, terminals). | none | Review 2026-10-06 (#1285; [worknote](worknotes/DK-03.md)) |
| DK-04 | Named areas, tent flaps to and from world 62, Med Tent respawner. | DK-02, DK-03, OD-DK03, OD-DK04 | BlockedDependency |
| DK-05 | Static population on worlds 61 and 62, with relog restores. | DK-02, DK-03 | BlockedDependency |
| DK-06 | Radio dialog: evidence, then the smallest engine change that shows it. | none for the evidence step | Ready (evidence step) |
| DK-10 | Mission 1570, Dakara half, Jaffa and Human dialogs. | DK-04, DK-05, OD-DK01; DK-06 for step 4905 | BlockedDependency |
| DK-11 | Mission 1645. | DK-05, DK-10 | BlockedDependency |
| DK-12 | Mission 1646. | DK-11 | BlockedDependency |
| DK-20 | Hostile roster, cover sets, encounter kit. | GDK1, OD-DK05 | BlockedDecision |
| DK-21 | Mission 1647. | DK-20, DK-06, DK-12 | BlockedDecision |
| DK-13 | Mission 1648. | DK-21 (story order) | BlockedDependency |
| DK-14 | Mission 1649. | DK-13, DK-06 | BlockedDependency |
| DK-15 | Mission 1650. | DK-14, OD-DK04 | BlockedDependency |
| DK-30 | Mission 1651: Dakara steps here, SGC steps with the route campaign; SGC address grant. | DK-15, B-DK4 | BlockedDependency |
| DK-31 | Mission 1652: ring controls, generators, Ha'tak sequences. | DK-30 | BlockedDependency |
| DK-32 | Mission 1653. | DK-31, DK-20, DK-06 | BlockedDecision |
| DK-33 | Mission 1654 exit boundary. | DK-32, B-DK4 | BlockedDependency |
| GDK1 | Design gate: encounter shapes and level band for 1647, 1652, 1653. | OD-DK05, M2 playtest | BlockedDecision |
| DK-99 | Close-out: status docs once, unified UAT section, final matrix. | all | Planned |

**Ready with no owner decision:** DK-02, DK-03 and DK-06's evidence step. **Ready under a recommended default that is one seed row to change:** DK-01 (OD-DK02).

## The Dead-End Fix, Exactly

DK-01 is the whole minimum. After it, with #1273 merged, a level-1 Free Jaffa who lands on the gate plaza:

1. **Can leave and return.** A world-61 chain on `player_loaded Dakara_E1`, gated `archetype eq 7`, runs `grant_stargate_address 5`. The executor arm is idempotent, updates the open client with `updateStargateAddress` and persists to `sgw_player.known_stargates`. The DHD (spawn 38) is 7.6 m from the start point and then lists Omega Site. At Omega Site the arrival unlock teaches gate 25, Omega's DHD (spawn 41) lists Dakara E1, and gate 25 gets an `arrival_*` pin on the plaza so the return does not land inside the gate volume.
2. **Has a working respawn.** Respawner 610 and the profile start point come from #1273; DK-01 adds nothing and its UAT checks it.
3. **Is told the truth.** One system mail, once per character, from a project-authored sender, saying the Free Jaffa command is not staffed yet and the gate is open to Omega Site. It is `NEW CONTENT` and says so in its seed comment. The mail action needs one small change so a repeat firing is silent ([DK-01](work-packets.md#dk-01)); the address and the pin do not depend on it. There is no first mission in the minimum (OD-DK01, option A).

**How this sits with OD-CS10.** OD-CS10 keeps SGU route authoring out of v6 and names 1645-1653, 1654 and 1655 as a follow-up campaign. DK-01 authors no mission, binds no dialog and touches none of those ids; it is "start infrastructure that later world transitions can use", which OD-CS10 lists as v6's own. This campaign is that follow-up for worlds 61 and 62, and the first mission arrives in DK-10 (1570's Dakara half), then DK-11 (1645) and DK-12 (1646). Missions 1645-1655 keep their seeded level 3 to 5 and the start level stays 1: no server code was found that reads a mission's level (`missions.level` and `dialog_set_maps.min_level` have no reader under `crates/`).

## Where Confidence Is Low Or A Guess

- Every position. One recovered line helps: Rak'nor says the command tent is "just to the east of the Stargate, next to the healing tent" (dialog 6110). The tent volumes in the [audit](audit.md#regions-and-triggers) are all about 200 m or more from the gate, so the command tent is probably a prefab the Python reader did not name; where the Repository and the two gates are is unknown until DK-02 reads prefab names and floors.
- Whether both tents were meant to share the one interior map.
- How "use your radio" was triggered. No radio item, ability or object is named in the mission data.
- What a client does on a gate arrival at world 61, and whether the mail window is a good place for the notice. Both are lab UAT items.
- Whether 1570's search sites and 1652's plazas were guarded. The dialog says the areas "have seen heavy fighting"; no objective asks for a kill.
- The offer rules. Order comes from the name strings' sequence numbers; nothing says what each mission required.

## Architecture Guardrails

The [Harset guardrails](../harset-rebuild/README.md#architecture-guardrails) apply unchanged. The ones this zone will hit first:

- One event fires every matching chain; sibling chains on one trigger must have disjoint conditions. The Human and Jaffa branches of 1570 are siblings and are split on `archetype`.
- A dialog-set bind is per player and in memory; every chain that binds one needs a `player_loaded Dakara_E1` (or `Dakara_E1_StoryRm`) restore chain gated on the active step.
- `display_dialog` from `dialog_choice`, `player_loaded` or a deferred action only shows an all-speaker-0 dialog unless an interaction pinned a speaker.
- `OnRegionEnter` does not filter by world; region keys are byte-exact `point_sets.name` values.
- `cross_world_teleport` matches `spaces.xml` world names exactly and cannot set facing.
- Shared NPCs on world 61 are never hidden, destroyed, moved or made hostile by mission content. Per-player scenes go in the instanced tent (OD-DK04) or use mission-scoped `spawn_entity` tags.
- Bra'tac (59) and Moh'katan (54) are shared templates; Harset spawns Moh'katan. A Dakara change to either template is a change to Harset.
- No new dialog rows and no cooked-data override: the client holds every Dakara dialog already.

## Validation And UAT Gates

Tests follow [TESTING.md](../../../TESTING.md): a chain-replay test (type 6) per chain packet that fails when the seed rows are removed, a live-DB guard (type 3) for seed relationships, a navmesh guard for every coordinate, and a wire-format test (type 2) for anything that emits a new message. UAT is in the live lab, by the owner or the coordinator's lab session; no packet worker launches a client.

| Milestone | Packets | Acceptance in the lab (all pending) |
|---|---|---|
| M1 Dead end removed | DK-01 (with #1273) | New Free Jaffa: the notice arrives once; the DHD lists Omega Site; dial, cross, arrive standing at Omega Site; Omega's DHD lists Dakara E1; return lands on the plaza outside the gate; relog keeps both addresses; a death respawns at the plaza. A Human visitor gets no grant from the chain. |
| M2 Arrival story | DK-02 to DK-05, DK-10 to DK-12 | Bra'tac, Moh'katan and Loth'ta are where the ledger says and survive a relog; tent flaps work both ways; 1570's Dakara half, 1645 and 1646 complete in order with a relog at every step; the owner corrects positions in one pass. |
| M3 Betrayal | GDK1, DK-06, DK-20, DK-21, DK-13 to DK-15 | The gate defence is winnable at the player's level; radio beats show; the terminal hack, the search and the decode work; the tent confrontation plays for one player without changing what another sees. |
| M4 Climax | DK-30 to DK-32 | SGC round trip with the generators; both Ha'taks appear and explode for the acting player; courtyard objectives complete. |
| M5 Exit | DK-33, DK-99 | 1654 hands off to the SGC once; final PASS / BLOCKED matrix. |

## Handoff Validation Record

| Check | Recorded outcome |
|---|---|
| Archive | `00_README.md`, the three `HANDOFFS/` files, `REPORTS/00_FINAL_SUMMARY.txt`, `10_WORLD_CONTENT_SUMMARY.csv`, `12_MISSING_UNRECOVERED_WORLDS.csv`, both Dakara_E1 world folders (17 files each) and the four missing-world evidence notes read. Nine of Dakara_E1's CSVs and fourteen of the StoryRm's are 10-byte `EMPTY` files. Nothing from the archive is committed. |
| Client files | Read-only: map directory listing and SHA-256 of all Dakara tiles; `extract_actors.py` and `kismet_extractor.py --survey` on both maps (with the reader's `lzallright` dependency in a throwaway environment); five cooked-data PAK entries opened. No client, launcher or lab tool was started. |
| Seed and repo | `db/resources/**` queried from a recent load and cited by file; `entities/spaces.xml`, `cell_spaces.xml`, `data/spaces/README.md`; the unmerged CS-02 branch read with `git show`; the earlier handoff pack's three Dakara workbooks. |
| Not done | No build, no test, no Rust extractor, no client session. Listed in the [audit](audit.md#what-could-not-be-checked). |
