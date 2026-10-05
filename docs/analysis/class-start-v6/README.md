# Class Start v6

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-05 against `main` @ `7dc0d5ad6`, which contains the handoff's
> baseline `b28813cc`. Packet prefix `CS-`. Same dispatch rules as the
> [ability mechanics ledger](../ability-mechanics/work-packets.md#dispatch-rules).
>
> **Campaign status (2026-10-05): CS-00 and CS-01 integrated (#1261, #1263, #1264); CS-02 next.** Status
> FINAL_V1_SUBJECT_TO_CHANGE (OD-CS07): a decision here may change after UAT;
> its evidence label does not.

## Why

Lomiada's `SGW_FINAL_V1_S2C_IMPLEMENTATION_HANDOFF_v6` replaces the universal
spawn kit with a per-class start: abilities arrive when the tutorial or gameplay
system that uses them is introduced, each class gets one free signature ability,
and each race starts in its own world. It supersedes the v2-v5 drafts where they
conflict. Two preflight rounds reconciled it against the repo before any code:
the corrections are in [Preflight findings](#preflight-findings) and the
blocked rows in [Blockers](#blockers).

Every row in this campaign is `PROJECT_FINAL_S2C` unless the matrices below say
otherwise. Nothing here is relabelled as retail evidence.

## Owner decisions

| Id | Decision | Replaces |
|---|---|---|
| OD-CS01 | Normal characters do **not** start with the universal loaded pistol. The first-firearm tutorial and mission flow matters. Debug and test profiles may keep a debug-only pistol behind an explicit flag (L2). | D-SA1 for normal progression ([debug-area](../debug-area/README.md#decisions-taken-for-this-campaign)) |
| OD-CS02 | Castle CellBlock mission 687 is a five-way reward split: Soldier, Commando, Scientist, Archaeologist, Loyalist Jaffa. The recovered `Aftermath.script` graph was Human/Jaffa only; that stays the historical record, and the five-way split is never relabelled as retail. | D-CB06 ([castle-cellblock-rebuild](../castle-cellblock-rebuild/README.md)) |
| OD-CS03 | D-AM02 stays: default ammunition is free, no spare magazines are granted, a granted firearm starts loaded, special ammunition stays finite. | none |
| OD-CS04 | Abilities arrive when the relevant tutorial or gameplay system is introduced, not as an unexplained spawn kit. | none |
| OD-CS05 | Every playable profile has Health and Focus (or resource) sustain before sustained solo combat. | none |
| OD-CS06 | Each class gets exactly one free signature stock ability at its class-identity milestone: permanent, survives respec, costs no point, counts as owned and as branch credit. All other tree nodes stay trained. | none |
| OD-CS07 | Status is FINAL_V1_SUBJECT_TO_CHANGE. | none |
| OD-CS13 | Every gun is acquired empty: a newly acquired firearm has 0 rounds and the player reloads once (default reload is free and unlimited, D-AM02). Equip and swap never change the count. Reverses the 'starts loaded' clause of OD-CS03 and rules L4/L5; character-creation kits follow in CS-02. Pending lomiada's acknowledgement. | OD-CS03's "a granted firearm starts loaded"; L4; L5 |

### Locked implementation rules

Set by lomiada in the first preflight answer (2026-10-05).

| Id | Rule | How it lands |
|---|---|---|
| L1 | Normal players do not use the starter-hotbar patch `009`. | 009 applies only to debug-kit profiles. A normal character's bar starts empty; tutorial 5882 tells the player to place Pistol Shot. |
| L2 | The debug kit is an explicit start-profile flag, never derived from access level. | A `debug_kit` column on the start profile (CS-02). No kit choice reads `access_level`. |
| L3 | Unknown start worlds fail closed. | Character creation refuses a profile whose world has no loaded cell space; `space_registry`'s silent Castle_CellBlock fallback becomes a loud refusal (CS-02). |
| L4 | ~~A firearm content grant loads `clip_size` when `clip_size > 0`.~~ | Superseded by OD-CS13. |
| L5 | ~~Staff, ribbon and other zero-clip weapons are unchanged.~~ | Superseded by OD-CS13. |
| L6 | Grant provenance survives reset and respec and stays distinguishable from trained abilities. | `sgw_player_ability_grants` with a `source_kind`; respec touches only `trained_abilities`; the GM / Debug NPC reset rebuilds from provenance (CS-01a). |

### Preflight answers

Lomiada's answers to the second preflight (2026-10-05).

| Id | Decision |
|---|---|
| OD-CS08 | **Goa'uld holding state.** While B4 stands, char_defs 10/19 behave exactly as today: Castle_CellBlock start, the legacy universal kit, the Human M687 / loot-table-10 reward, today's M622/M641 behaviour. None of the canonical Egypt grants apply. The state is labelled `NON_CANONICAL_BLOCKED_LEGACY`, must not shape the canonical PRA_GOAULD design, and is removed as one unit when Egypt becomes playable. |
| OD-CS09 | **Asgard holding state.** While B1-B3 stand, char_def 9 keeps today's runtime literally: SGC_W1 start, the universal kit, pistol 55, 592/594/597/1218/1646, and today's SGC M1559 tutorial. No Pertho or ship grants. Labelled `NON_CANONICAL_BLOCKED_LEGACY`. The SGC starter gates therefore exclude Free Jaffa (`archetype neq 7`), not every non-Human. |
| OD-CS10 | **SGU route authoring stays outside v6.** v6 delivers creation, start, loadout and tutorial behaviour, archetype gates on the SGC starter chains, gateable future Dakara chains, and start infrastructure that later world transitions can use. Missions 1563-1569/1571/1572, 1570, 1645-1653, 1654 and 1655 are a follow-up campaign. Neither SGU route is called gameplay-complete because v6 passes. |
| OD-CS11 | **Player weapon requirement restored globally (CS-07).** An ability with one or more required item monikers fires only if the active weapon carries at least one; no requirement means no change; a refusal gives visible `WrongWeaponType` feedback; NPC casts are unchanged; the 592 → active-weapon redirect is removed, not layered under the check. CS-07 includes an audit of every player ability with a moniker requirement (`ability_id → monikers → valid shipped weapons → PASS/FAIL`). A failure is never fixed by weakening the rule; a bad recovered binding is flagged as its own row. |
| OD-CS12 | **Commando reward family A**: 3347/3359/3372/3387/3401 + 3325. It is what `Aftermath.py:72-74` grants and loot table 10 ships, and it renders. Family B (3345/3358/3371/3386/3400, the Aramid set) has a NULL `visual_component`, a missing icon and a placeholder description. |

D-AT09 (the 1646 starter / Goa'uld tree collision) is resolved for canonical
profiles by OD-CS04, since 1646 leaves the universal kit; the legacy holding
states of OD-CS08/09 still carry it until they are removed.

## Preflight findings

Facts checked against the repo and the local client copy. File paths are from
the repo root.

- **Start level.** Character creation inserts a literal level 1
  (`crates/base/src/base/character_create/mod.rs`); nothing reads
  `missions.level` to set a player's level. v6's "Free Jaffa level 3" came from
  the Dakara missions' seeded levels; the start is **level 1**. CS-02 adds an
  explicit `start_level` and a test that no start level comes from a mission.
- **Start world is defined twice.** `crates/resources/src/base/chardef.rs` is
  authoritative and `resources.char_creation.starting_world` is never read.
  `starting_position()`, `home_for_alignment()` and the console's
  `world_entry_point` also assume only CellBlock and SGC_W1. CS-02 replaces all
  of them with one start profile.
- **Granted firearms arrive empty.** The content grant sets `ammo = charges`
  (0 for 55, 21, 3260); creation loads `clip_size`. Kept as the rule by
  OD-CS13; creation follows in CS-02.
- **The trainer's spend gate is archetype-wide** (`tree_points_spent >=
  required_branch_points`); no per-branch credit exists. Free nodes satisfy
  prerequisites but add no spend.
- **The GM / Debug NPC reset wipes every grant.** `gm_ability_bulk.rs` rebuilds
  from `char_creation_abilities`.
- **No non-GM single-ability content action exists.** Only `gm_ability_bulk`
  and `launch_ability`.
- **Dialog triggers never set `archetype`** (`event_dispatch/dialog.rs`), so an
  archetype condition on a dialog chain reads -1: `eq` never matches,
  `neq`/`lt` always pass. Fixed in CS-01b before any gated dialog chain.
- **No persistent one-time flag.** `once` resets on relog and world change;
  counters are in memory. CS-03 adds persisted tutorial state.
- **SGC chain 3001 would start the Human tutorial for a visiting Free Jaffa**
  (gates: `player_loaded SGC_W1`, 1559 not active). Chains 3017/3018 can pull any
  visitor into M1561. CS-05 gates them `archetype neq 7` (OD-CS09).
- **The Rust server enforces no weapon requirement.** The ability loader never
  selects `abilities.item_monikers`; any known ability fires with any weapon.
  The Python server did (`AbilityManager.py`, WrongWeaponType).
  `docs/gameplay/ability-system.md` marks it DONE. Fixed in CS-07 (OD-CS11).
- **Weapon monikers for the starter rows:** 592 needs ITEM_Pistol (55 has it);
  598 needs ITEM_Automatic_Weapon (21 SGHC 6 has it, **3260 SK37 LMG does not**:
  ITEM_LightMG only); 1984 needs ITEM_Staff (2797 has it); 1639 needs
  ITEM_RibbonDevice (4565 has it). Soldiers use Quick Burst with the SGHC 6 they
  already received.
- **M1559's seeded title is "Welcome to Stargate Command"**; its FirearmBody
  pistol grant (chain 3008) already works. M1571 has no chain.
- **M1562 stops at step 4625** and has no desk interactable; **M1569 has no
  chains**.
- **Staff Swing 1984 is a Shol'va tree node only**; for Loyalist Jaffa
  (archetype 8) it is a plain grant.
- **1646/1647 are not tree roots** (1643 → 1646 → 1647); the free grant skips
  the chain on purpose and keeps its provenance.
- **Doc drift to fix in the packet that touches it:**
  `docs/gameplay/character-creation.md` has a wrong archetype table (starts at
  0, lists Shol'va as System Lords); `docs/gameplay/ability-system.md` claims
  the weapon requirement is enforced.

## Blockers

| Id | Dependency | Rows blocked |
|---|---|---|
| B1 | No Asgard start world: no Pertho or ship map in the client, no navmesh, no server space. | SGU_ASGARD profile; 1554, 1561, 1572, 1533; 3676, 3677 |
| B2 | Asgard Energy unmodelled (D-AB11): 1561 and 1572 have no mechanic. | 1561, 1572 (also B1). v6's named fallback `asgard_energy_fallback` (597+1218) applies once B1 clears. |
| B3 | Missions 1617 Orientation and 1621 Heimdall have no content. | 3676, 3677 (also B1) |
| B4 | No Earth (Egypt Past) world: no Egypt (66), Egypt_StoryRm (67) or Temple (89) map, navmesh, space, spawns or real gate, and no chains for 1527-1539 / 1572-1579. | PRA_GOAULD profile; 1646, 1647, 1639; Goa'uld start gear |

**12 of 47 matrix rows are blocked** (Asgard 7, Goa'uld 5); 35 are buildable.

The Egypt → Harset transition has no surviving original trigger: M1200's
original offer row requires no mission, M1579 ends with "signal the Cargo Ship"
and "Speak to Ba'al", and no recovered script mentions Egypt. When it is built
it is labelled `PROJECT_FINAL_RECONSTRUCTION`. Chain 6121 (accept M1200 on
`player_loaded Harset`) is already our own authoring.

## Matrices

Legend: **BUILD** can be implemented now; **BUILD-INFRA** needs this
campaign's infrastructure; **BLOCKED** has an external dependency
([Blockers](#blockers)). A row reaches PASS only after implementation, tests and
the live UAT (CS-08).

### Profiles

| ProfileID | char_defs | Start | Level | Status |
|---|---|---|---|---|
| PRA_OPCORE_SOLDIER | 1, 11 | Castle_CellBlock | 1 | BUILD-INFRA |
| PRA_OPCORE_COMMANDO | 3, 13 | Castle_CellBlock | 1 | BUILD-INFRA |
| PRA_OPCORE_SCIENTIST | 20, 22 | Castle_CellBlock | 1 | BUILD-INFRA |
| PRA_OPCORE_ARCHAEOLOGIST | 5, 15 | Castle_CellBlock | 1 | BUILD-INFRA |
| PRA_LOYALIST_JAFFA | 7, 17 | Castle_CellBlock | 1 | BUILD-INFRA |
| PRA_GOAULD | 10, 19 | Earth (Egypt Past) | 1 | BLOCKED (B4); holding state OD-CS08 |
| SGU_HUMAN_SOLDIER | 2, 12 | SGC_W1 | 1 | BUILD-INFRA |
| SGU_HUMAN_COMMANDO | 4, 14 | SGC_W1 | 1 | BUILD-INFRA |
| SGU_HUMAN_SCIENTIST | 21, 23 | SGC_W1 | 1 | BUILD-INFRA |
| SGU_HUMAN_ARCHAEOLOGIST | 6, 16 | SGC_W1 | 1 | BUILD-INFRA |
| SGU_FREE_JAFFA | 8, 18 | Dakara_E1 | 1 | BUILD-INFRA |
| SGU_ASGARD | 9 | Starship → Pertho | 1 | BLOCKED (B1, B3); holding state OD-CS09 |

### Abilities

| Scope | Ability | Layer | Status |
|---|---|---|---|
| HUMAN+LOYALIST | 592 Pistol Shot | CORE_TUTORIAL | BUILD-INFRA |
| HUMAN+LOYALIST | 594 Strike | CORE_TUTORIAL | BUILD-INFRA |
| HUMAN+LOYALIST | 597 Heal Focus | CORE_TUTORIAL | BUILD-INFRA |
| HUMAN+LOYALIST | 1218 Recuperation | CORE_TUTORIAL | BUILD-INFRA |
| SGU_FREE_JAFFA | 597 Heal Focus | RACIAL_CORE | BUILD-INFRA |
| SGU_FREE_JAFFA | 1218 Recuperation | RACIAL_CORE | BUILD-INFRA |
| PRA_GOAULD | 1646 Health Heal | RACIAL_CORE | BLOCKED (B4) |
| PRA_GOAULD | 1647 Focus Regeneration | RACIAL_CORE | BLOCKED (B4) |
| SGU_ASGARD | 1554 Synaptic Clarity | RACIAL_CORE | BLOCKED (B1) |
| SGU_ASGARD | 1561 Reroute Energy: Basic | RACIAL_CORE | BLOCKED (B1, B2) |
| SGU_ASGARD | 1572 Convert Energy: Basic | RACIAL_CORE | BLOCKED (B1, B2) |
| SOLDIER_ALL | 598 Quick Burst | SIGNATURE | BUILD-INFRA (fires with 21, refused with 3260) |
| COMMANDO_ALL | 646 Stealth I | SIGNATURE | BUILD-INFRA (stealth effect unmodelled, D-AB11) |
| SCIENTIST_ALL | 948 Battlefield Heal | SIGNATURE | BUILD-INFRA |
| ARCHAEOLOGIST_ALL | 802 Reveal Mini-Games | SIGNATURE | BUILD-INFRA (minimap effect unverified) |
| JAFFA_ALL | 1984 Staff Swing | SIGNATURE | BUILD-INFRA |
| PRA_GOAULD | 1639 Destruction Beam | SIGNATURE | BLOCKED (B4) |
| SGU_ASGARD | 1533 Apply Damage | SIGNATURE | BLOCKED (B1) |

### Gear

| Scope | Trigger | Items | Status |
|---|---|---|---|
| PRA_CELLBLOCK_ALL | M622 | 55 | BUILD (L4) |
| PRA_CELLBLOCK_ALL | M641 | 21 | BUILD (L4) |
| PRA_SOLDIER | M687 | 3260, 7373 | BUILD-INFRA |
| PRA_COMMANDO | M687 | 3347, 3359, 3372, 3387, 3401, 3325 | BUILD |
| PRA_SCIENTIST | M687 | 4444, 7373 | BUILD-INFRA |
| PRA_ARCHAEOLOGIST | M687 | 6843, 7373 | BUILD-INFRA |
| PRA_LOYALIST_JAFFA | M687 | 2797, 4342 | BUILD |
| PRA_GOAULD | Egypt start | 4337, 4338, 4339, 4565 | BLOCKED (B4) |
| SGU_HUMANS | M1559 FirearmBody | 55 | BUILD (L4) |
| SGU_HUMANS | M1562 Carter desk | 21 | BUILD-INFRA |
| SGU_SOLDIER | M1569 | 3260, 7373 | BUILD-INFRA |
| SGU_COMMANDO | M1569 | 3347, 3359, 3372, 3387, 3401, 3325 | BUILD-INFRA |
| SGU_SCIENTIST | M1569 | 4444, 7373 | BUILD-INFRA |
| SGU_ARCHAEOLOGIST | M1569 | 6843, 7373 | BUILD-INFRA |
| SGU_FREE_JAFFA | Dakara start | 2797, 4342 | BUILD-INFRA |
| SGU_ASGARD | M1617 | 3676 | BLOCKED (B1, B3) |
| SGU_ASGARD | M1621 | 3677 | BLOCKED (B1, B3) |

## Packets

| Packet | Scope | Depends on | Status |
|---|---|---|---|
| CS-00 | This ledger: OD-CS01..12, L1-L6, preflight findings, B1-B4, matrices. D-SA1 and D-CB06 marked superseded where recorded. | none | Integrated (#1261) |
| CS-01a | Grant provenance and the content grant action: `sgw_player_ability_grants`; a non-GM `grant_ability` content action (persist, provenance, `onKnownAbilitiesUpdate`, visible chat feedback); branch credit in the trainer spend gate (trained points plus the cost of granted tree nodes; refunds stay trained-only); the GM / Debug NPC reset rebuilds from starters plus provenance; GM grants recorded as `gm`. | none | Integrated (#1264) |
| CS-01b | Dialog triggers set the `archetype` parameter (every player trigger now does; a missing value still reads -1 on purpose, 701's Human-branch fallback). The bandolier ammo counter (`AmmoSlot{N}`) mirrors a granted gun's count. The content executor no longer writes a guessed weapon into the occupied active slot. Guards that a content, mission, loot, GM or vendor acquisition gives a gun 0 rounds, and that equip and swap keep the count (OD-CS13). PR #1263. | none | Integrated (#1263) |
| CS-02 | Data-driven start profile (world, spawn, level, gear, grants, `debug_kit`, holding-state label); fail closed on an unknown world (L3); universal kit removed for canonical profiles; OD-CS08/09 holding states; seeded characters and drift test updated. | CS-01a | Planned |
| CS-03 | Persisted one-time tutorial state; triggers for 5882 and 5883. As built: `sgw_player_tutorials`, the `show_tutorial` action (shown only on the first DB record), the `tutorial_shown` condition, the `player_entered_combat` trigger, and seeded chain 7101 (5883 on the first combat after 5882). 5882 is wired by CS-04/CS-05 per the [`show_tutorial` params](../../content/content-engine-vocabulary.md#show_tutorial-params). Client display of a server-sent type-3 dialog needs lab UAT (CS-08). PR #CS03PR. | CS-01a | In review |
| CS-04 | CellBlock: M622 core grant and tutorials, M641 unchanged, M687 five-way loot tables and signatures. | CS-01a/b, CS-02, CS-03 | Planned |
| CS-05 | SGC: M1559 core grant and tutorials, M1562 Carter desk SMG, M1569 class rewards and signatures; `archetype neq 7` on the SGC starter chains. | CS-01a/b, CS-02, CS-03 | Planned |
| CS-06 | Free Jaffa start on Dakara_E1 at level 1: authored navmesh-validated spawn and respawner, start gear and grants from the profile. | CS-02 | Planned |
| CS-07 | Player weapon-moniker requirement (OD-CS11) with the per-ability audit; remove the 592 redirect; weapon basic-attack transience guards; doc fixes. | none | Planned |
| CS-08 | Live UAT of every buildable profile; final PASS/BLOCKED matrix. | all | Planned |

## Grant provenance contract (CS-01a)

| `source_kind` | Written by | Survives respec | Survives GM / Debug NPC reset |
|---|---|---|---|
| `tutorial` | `grant_ability` content action | yes | yes |
| `racial_core` | start profile (CS-02) or content | yes | yes |
| `signature` | content (M687, M1569) or start profile | yes | yes |
| `mission` | content | yes | yes |
| `gm` | `.giveability`, GM grant-all, Debug NPC granter | yes | **no**: reset removes the ability and its row |

A trained ability is never in this table; it lives in `trained_abilities`.
Existing characters start with no provenance rows (D-AT11).

As built in CS-01a (with the review fixes), rules the table above does not
show:

- **A bought node that content later grants is converted** (OD-CS06, "costs
  no point"), in the grant's own locked transaction: removed from
  `trained_abilities`, its tree node's `skill_point_cost` refunded to
  `training_points` and taken off `tree_points_spent` (both floored at 0), and
  the content row written. A respec afterwards keeps it. The player gets the
  point counter and an "is now yours for free" line.
- **`signature` and `racial_core` grants name their archetypes**
  (`archetypes`, EArchetype ordinals), checked against the player's real
  archetype by the cell and again by the base; a mismatch writes nothing and
  sends no line. `tutorial` and `mission` may omit it.
- **Starters earn no credit.** A grant of one of the archetype's
  character-creation starters writes no row, and the world-entry credit read
  skips starters, so an overlapping grant list gives no free branch credit.

- A content grant of an ability that already has a `gm` row promotes the row
  to its own kind, so the reset keeps it. Any other existing row is kept: the
  first content source wins.
- A GM grant of an ability that already has a content row leaves the row as it
  is, so a GM can never downgrade a signature to `gm`.

The author-facing reference is
[`grant_ability` params](../../content/content-engine-vocabulary.md#grant_ability-params).

## Follow-up campaigns (outside v6)

- **SGU route content** (OD-CS10): 1563-1569/1571/1572, 1570, 1645-1653, 1654,
  1655 with their NPCs, encounters, dialogs, interactables and travel. 1655 needs
  its Human (5854/5856) and Jaffa (5855/5875) branches.
- **EGYPT-00, world-shell feasibility** (lomiada's Egypt reconstruction
  handoff, 2026-10-05): archaeology closure, `Ter-EGT.upk` and
  `Geo_Egt_Prebuild.upk` inventory, the QA UnrealEd create-save-cook-load test,
  and a server-space feasibility report; answers "can we create and load a new
  SGW-native Egypt world shell with the QA editor toolchain?" Checked so far:
  the QA install has `AtreaEditor.bat`, `UnrealEdSGW.xrc`, `UnrealEd.u`,
  `Cine-Intro_Goauld.upk`, `Cine-Transition_Goauld.upk` and `DeS-SkyDome.upk`;
  quest items 4260 Ra Staff Weapon, 8618 Canopic Jar (Empty), 8619 Queen Anat
  and 8620 Cipher Key are seeded (mission bag). `SGW_FINAL_RE.sqlite` and the
  Stage 4U outputs are **not** in the archives we hold (see
  [final RE bundles memory](../../../.claude/agent-memory/main-session/project_final_re_bundles.md)),
  and no loose `Egypt_WorldMap.jpg` is in our client copy. Open question for
  EGYPT-00: the Goa'uld prologue's first twelve missions (1527 Awakening to 1538
  Last Minute Details) carry `DN_ms_A00_Temple_*` names and levels 1-3, so the
  level-1 start may be the Temple world (89) rather than Egypt (66).
- **Asgard start** once a Pertho or ship map exists (B1-B3).
