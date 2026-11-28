# NPC combat V2: the remaining roster

Type: campaign ledger. Audience: the orchestrator, the owner and packet agents. Last updated: 2026-11-27.
Companions: [work-packets.md](work-packets.md) (rules, contract, waves 0-1), [work-packets-batches.md](work-packets-batches.md) (waves 2-6), [RESUME.md](RESUME.md). Sister campaign: [npc-combat-v1-1](../npc-combat-v1-1/README.md) (Horden and the Straegis Titan).

## Purpose

V1 ([npc-combat-reference](../npc-combat-reference/README.md)) gave five reference NPCs their own attacks and put `PlaceholderPolicy::RequireWeaponEvidence` in force (D-NC1). Every other NPC with no ability set now holds fire. The V1 lab session logged the backlog:

- 21 `combat_profile_placeholder` rows: firearm humans that still fire 592 Pistol Shot;
- 266 `combat_profile_unresolved` rows: NPCs with no set and no weapon evidence.

V2 gives each hostile NPC family an attack backed by evidence. It reuses V1's resolver, reference-test shape, evidence registry and Debug Area pens, and works in batches ordered by how much evidence each family has. An NPC with no evidence keeps holding fire, and its registry row says why. V2 never gives an NPC a generic attack in place of its own.

The six evidence classes are V1's: ORIGINAL CLIENT VERIFIED, EARLIEST LEGACY DATA, CURRENT CIMMERIA IMPLEMENTATION, STRONGLY INFERRED, RECONSTRUCTED and UNRESOLVED.

V2 depends on V1.1 in two places:

- the `SequenceOverride` NVP field (NX-02), used only if a batch needs a new sequence;
- D-NX4, the ability-override category, used only if a batch needs a new ability id.

The plan avoids new ability ids where a seeded ability already fits.

## Findings (architect research, 2026-11-27)

| # | Finding | Class | Source | Acts in |
|---|---|---|---|---|
| R1 | The V1 placeholder inventory counts 566 templates, 506 of them with a NULL `ability_set_id`; 31 keep 592 under the firearm whitelist (D-NC13). The 287 lab-logged rows (21 + 266) are the NULL-set templates that actually spawn. NR-01 refreshes the inventory with a batch column and reconciles the counts. | CURRENT CIMMERIA IMPLEMENTATION | `npc-combat-reference/placeholder-inventory.md`; `w4-lab-results.md` § NC-44 | NR-01 |
| R2 | The NULL-set non-interactive mobs by body family (architect scan; NR-01 confirms the counts): <ul><li>Jaffa body: 154 templates and 176 spawns, including Unas 105-110 and 132-137;</li><li>human body: 69 that hold fire (101 spawns, mostly friendly quest NPCs) and 18 that keep 592;</li><li>Goa'uld body: 39 templates and 29 spawns (bosses such as Ra and Ba'al, plus lineup rows);</li><li>mechanical: 8 (69 Malfunctioning Drone, 70 BattleWalker, 81 Ancient Drone, lineup 1563-1566 and 1569);</li><li>other: 21 (Asgard, Nox children and the like).</li></ul> | CURRENT CIMMERIA IMPLEMENTATION | seed parse | NR-01, NR-10 |
| R3 | The seeds hold a full weapon auto-attack family (EARLIEST LEGACY DATA names): <ul><li>579 Pistol, 581 Rifle, 582 LMG, 580 HMG, 583 Shotgun and 559 Automatic Weapon Auto Attack;</li><li>the melee AAs 708 Pistol, 739 Rifle, 738 LMG, 709 Shotgun and 595 Automatic Weapon;</li><li>584 Staff with 710 Staff Melee AA (V1 set 4), 1414 Zat with 1415 Zat Melee AA;</li><li>712 Ribbon Device with 711 Ribbon Device Melee AA;</li><li>733 Fists Melee AA and 740 Blade Melee AA;</li><li>1216 Drone Ranged AA and 1209 Drone Melee AA.</li></ul>V1 proved the pattern on the NID Guard (set 3, ability 559, effect 5310). | EARLIEST LEGACY DATA (rows); CURRENT CIMMERIA IMPLEMENTATION (the NID path) | `Abilities/Seed/abilities.sql` | NR-04, B1, B2, B6 |
| R4 | The firearm whitelist is `HUMAN_FIREARM_PREFIXES` (`WP-Human.WP_Pistol_`, `_SMG_`, `_AR_`, `_LMG_`, `_HMG_`, `_Rifle_` and others). Each prefix names a weapon class, so a weapon-class-to-auto-attack map retires the 592 placeholder without inventing an attack. | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-world/src/cell/combat/combat_profile/mod.rs:41` | NR-04, NR-11 |
| R5 | These creature packages ship an AnimSet: AMBRat, Carnosaur, Rhinolion, Lenny, TwillaTree, StraegisFighter, StraegisBeacon, ScavDog and CA_DroneTank. `AN_Android` has none, and neither does `MOB_Horden` (V1.1 G1). | ORIGINAL CLIENT VERIFIED (export tables) | `tools/upk_parser.py` over the QA CookedPC | NR-05, NR-06 |
| R6 | The drone ability family (1174 Drone Shot, 1186 Drone Strike, 1216 Drone Ranged AA, 3283 DroneShot) and the `mob_caDroneFlyer` audio bank exist. They make the mechanical batch the best-evidenced one after the firearms. | EARLIEST LEGACY DATA (abilities); ORIGINAL CLIENT VERIFIED (bank) | `abilities.sql`; `Content\audio\mobs` | NR-06, B3 |
| R7 | Creature candidates from NC-34: 983 Bite (effect 1092 `-125F -15H`, no script yet), 984 Infected Bite and 1433 Mob Strike (set 6, the ScavDog set, RECONSTRUCTED). No client record pairs any of them with a specific creature. | EARLIEST LEGACY DATA; RECONSTRUCTED (set 6) | `npc-combat-reference/roster-followup.md` | B4, D-NR2 |
| R8 | The Straegis Fighter has pet abilities 1156, 2847 and 1240 with presentation event sets (`sEmitAura`), and the summon family 2826 and 3491-3497. Hostile Fighter (78) and Beacon (77) templates can reuse the pet abilities, which is evidence the Titan lacks. | EARLIEST LEGACY DATA | `abilities.sql`; V1.1 README F7 | NR-08, B5 |
| R9 | Unas have no ability and no weapon component (NC-34). The pets content inventory records that no Unas ability exists. | EARLIEST LEGACY DATA (absence) | `roster-followup.md` | D-NR1 |
| R10 | The V1 combat anomaly K26 (blank attacker name on templates 4 and 76) applies to any template with no `name_id`. V1.1 NX-08 finds the cause. | CURRENT CIMMERIA IMPLEMENTATION | `w4-lab-results.md` | registry rows |

## Batches (ordered by evidence)

| Batch | Family | Templates (NR-01 confirms) | Attack source | Evidence | Wave |
|---|---|---|---|---|---|
| B1 | Firearm humans | the 31 whitelist templates (21 spawned) | weapon-class auto-attack per R3 and R4, the NID pattern | EARLIEST LEGACY DATA ability, CURRENT CIMMERIA IMPLEMENTATION path | 2 |
| B3 | Mechanical | 69, 81, lineup 1563-1566 and 1569; 70 per NR-06 | drone family (R6), the PRU pattern | EARLIEST LEGACY DATA, audio bank | 2 |
| B4a | AMBRat | 74, lineup 1554 | per NR-05 and D-NR2 | the hub-pass hostile, so it goes first | 2 |
| B2 | Jaffa with a staff or zat component | NR-01 lists them | set 4 (584 and 710), or zat 1414 and 1415 | V1 NC-41 path | 3 |
| B4b | Organic creatures | Carnosaur 71/1559, Rhinolion 75/1560, Lenny 73/1556 (and 1557 per D-NR3), Twilla 80/1561 (per D-NR10) | per NR-05, NR-06 and D-NR2 | AnimSets (R5); attack RECONSTRUCTED | 3 |
| B5 | Straegis Fighter and Beacon | 78/1567, 77/1568 | pet abilities (R8) | EARLIEST LEGACY DATA | 4 |
| B6 | Goa'uld | NR-01 lists them | ribbon device 712/711 or staff, per NR-07 | per NR-07 | 4 |
| B7 | Unarmed hostile humans and Jaffa | NR-01 lists them | 733 Fists Melee AA, per D-NR6 | STRONGLY INFERRED | 4 |
| B8 | Unas, Android and the other family | 105-110, 132-137; 1562; others | none, unless NR-07 finds evidence | absence | 4 (tests pin hold fire) |

Friendly, interactive, vendor and trainer NPCs are out of scope. They keep holding fire, and NR-01 tags them `out of scope`.

## Decisions

| ID | Decision | Status | Recommendation and reason |
|---|---|---|---|
| D-NR1 | Unas | **BlockedDecision** | **Hold fire** until NR-07 finds a client attack for them (R9). Pin the behaviour with a test, and write the gate in the registry. Alternative: 733 Fists Melee AA on the Jaffa body, STRONGLY INFERRED. |
| D-NR2 | Creature sets: one per creature, or shared | **BlockedDecision** | **One set per creature** (the NC-34 default), each RECONSTRUCTED, using 983 Bite or 1433 Mob Strike as NR-10 assigns from the NR-05 and NR-06 findings. Sharing set 6 is quicker, but it shows six creatures with an attack the client never paired with them. |
| D-NR3 | Lenny's baby (1557, lineup only) | **BlockedDecision** | Leave it NULL: it is a lineup-only row with no world template, so a set changes nothing in play. |
| D-NR4 | Lineup rows | Decided (architect) | A lineup template takes its world template's set when V1.1 NX-09 confirms the lineup flag keeps it passive. The V1 Jaffa lineup actors 1380 and 1381 already work this way. |
| D-NR5 | Retiring the 592 placeholder | **BlockedDecision** | **Yes.** Once B1 gives every whitelist template its own weapon set, set `PLACEHOLDER_POLICY` so that no NPC receives 592 implicitly (NR-15), and keep the policy enum for tests. This changes the V1 C1 contract, so the owner approves it. Alternative: keep 592 for unmapped firearm prefixes. |
| D-NR6 | Unarmed hostile humanoids | **BlockedDecision** | Give hostile unarmed human and Jaffa mobs **733 Fists Melee AA**, labelled STRONGLY INFERRED: it is the player's unarmed attack on the same body and animations. Friendly NPCs keep holding fire. Alternative: hold fire. |
| D-NR7 | Goa'uld combat | **BlockedDecision** (asked once NR-07 lands) | Research first (NR-07). Give a Goa'uld the ribbon device (712 and 711) only where its template or client art carries a ribbon device, and the staff where it carries a staff. Any Goa'uld boss special waits for its own evidence. |
| D-NR8 | Damage values | Decided (inherits D-NC2) | Reused abilities keep their seeded effects. A new effect payload is RECONSTRUCTED from the ability or effect text, and the C5 comment says so. |
| D-NR9 | Android | Decided (architect) | Hold fire: `AN_Android` has no AnimSet (R5), which is an external gate like V1.1 G1. |
| D-NR10 | Twilla Tree | **BlockedDecision** (asked once NR-06 lands) | Research first. A tree that attacks needs its own evidence; with none, it holds fire. |
| D-NR11 | Lab use | Decided (inherits D-NC7) | Each run asks "may I use the lab now?" and waits for a yes. |
| D-NR12 | Release shape | Decided (architect) | Waves 2, 3 and 4 each ship as **one PR** from an integration branch (`nr-w2`, `nr-w3`, `nr-w4`), because their packets edit disjoint rows of the same template seed files. Waves 0 and 1 ship one PR per packet. |

## Packets

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Packet | Title | Wave | Agent | Depends | Status |
|---|---|---|---|---|---|
| NR-01 | Backlog classification | 0 | packet-coder | none | Ready |
| NR-02 | V2 evidence registry skeleton | 0 | packet-coder | none | Ready |
| NR-03 | Debug Area Z14 roster range and test stubs | 0 | rust-gameserver-dev | V1.1 NX-03 merged (shared files) | BlockedDependency |
| NR-04 | Weapon class to auto-attack map | 1 | combat-systems-advisor | none | Ready |
| NR-05 | AMBRat, Carnosaur and Rhinolion assets | 1 | game-archaeology-specialist | client tree | Ready |
| NR-06 | Lenny, Twilla, drones, BattleWalker and Android assets | 1 | game-archaeology-specialist | client tree | Ready |
| NR-07 | Goa'uld and Unas client evidence | 1 | game-archaeology-specialist | client tree | Ready |
| NR-08 | Straegis Fighter and Beacon as hostiles | 1 | game-archaeology-specialist | V1.1 NX-04, NX-05 | BlockedDependency |
| NR-10 | Batch spec (re-slice checkpoint) | gate | architect | NR-01, NR-04..07 | BlockedDependency |
| NR-11 | Weapon auto-attack sets | 2 | packet-coder | NR-10 | BlockedDependency |
| NR-12 | B1 firearm humans pick their weapon set | 2 | packet-coder | NR-11 | BlockedDependency |
| NR-13 | B3 mechanical | 2 | packet-coder | NR-10 | BlockedDependency |
| NR-14 | B4a AMBRat | 2 | packet-coder | NR-10, D-NR2 | BlockedDecision |
| NR-15 | Retire the 592 placeholder | 2 | packet-coder | NR-12, D-NR5 | BlockedDecision |
| NR-21 | B2 Jaffa with weapons | 3 | packet-coder | NR-10 | BlockedDependency |
| NR-22 | B4b Carnosaur | 3 | packet-coder | NR-10, D-NR2 | BlockedDecision |
| NR-23 | B4b Rhinolion | 3 | packet-coder | NR-22 | BlockedDecision |
| NR-24 | B4b Lenny | 3 | packet-coder | NR-10, D-NR2, D-NR3 | BlockedDecision |
| NR-25 | B4b Twilla Tree | 3 | packet-coder | D-NR10 | BlockedDecision |
| NR-26 | Lineup rows (D-NR4) | 3 | packet-coder | NR-21..25 | BlockedDependency |
| NR-31 | B5 Straegis Fighter and Beacon | 4 | packet-coder | NR-08 | BlockedDependency |
| NR-32 | B6 Goa'uld | 4 | packet-coder | NR-07, D-NR7 | BlockedDecision |
| NR-33 | B7 unarmed hostile humanoids | 4 | packet-coder | D-NR6 | BlockedDecision |
| NR-34 | B8 hold-fire pins (Unas, Android, other) | 4 | packet-coder | D-NR1 | BlockedDecision |
| NR-35 | Backlog recount and V1/V1.1 regression | 4 | packet-coder | NR-31..34 | BlockedDependency |
| NR-40 | Lab UAT by batch | 5 | orchestrator with a lab-driver | wave 4 merged, ask first | BlockedDecision |
| NR-41 | Lab fix packets (reserved) | 5 | packet-coder | NR-40 | BlockedDependency |
| NR-50 | Campaign review loop | 6 | packet-reviewer | wave 5 | BlockedDependency |
| NR-51 | Close-out | 6 | documentation-writer | NR-50 | BlockedDependency |

NR-09 is not a packet: the lineup-flag question it would answer is in V1.1 NX-09.

## Client-asset gates

| Gate | Blocks | Effect |
|---|---|---|
| A1 | `AN_Android` has no AnimSet | Android holds fire (D-NR9) |
| A2 | No audio bank except `mob_caDroneFlyer` and `mob_scav` | Sound rows UNRESOLVED for every family except drones and ScavDog |
| A3 | The lab has no audio | Sound NOT VERIFIED everywhere |
| A4 | Research needs the owner's client tree (not in git) | NR-05..08 return `blocked - no client tree` without it |
| A5 | Any new ability id needs V1.1 D-NX4 | Batches reuse seeded abilities wherever one fits |

## ETA (wall clock, wide fan-out)

| Phase | Parallel width | Wall clock | Gated by |
|---|---|---|---|
| Waves 0 and 1 | 8 agents (NR-03 and NR-08 start when V1.1 allows) | 3-4 h | client tree; V1.1 NX-03, NX-04 and NX-05 |
| NR-10, and owner decisions D-NR1-3, 5-7 and 10 | 1 agent | 1 h, plus the owner's reply time | the owner |
| Wave 2 (`nr-w2`) | 5 agents | 3-4 h | NR-10 |
| Wave 3 (`nr-w3`) | 6 agents | 3-4 h | wave 2 merged (shared seed files) |
| Wave 4 (`nr-w4`) | 5 agents | 3-5 h | wave 3 merged; NR-07, NR-08 |
| Wave 5 lab | 1 session at a time, 2-3 sessions | 4-6 h, plus fix packets | the lab being free, and a yes |
| Wave 6 review and close-out | 2 | 2 h | wave 5 |
| **Total** | | **19-26 h of wall clock**, plus owner and lab waits | |

## Cost-aware note

- 29 packet rows: 19 Haiku packets (3 foundation, 16 batch) at about 40k working tokens each, 5 research packets on their domain agents, the NR-10 gate, the lab, and the review and close-out.
- At about two Sonnet review passes per packet and $0.68 a pass, packet reviews cost about $30. Add three wave reviews and the campaign review.
- The five research packets and the lab sessions are the main cost.
- To save money:
  - reuse seeded abilities, which avoids D-NX4 and NX-18;
  - batch by family, so one reference-test shape serves a whole batch;
  - pin hold-fire NPCs with one table-driven test (NR-34), not one test each;
  - run the lab by batch, several NPCs per session.
- Opus only rules (NR-10 and review rulings). It never codes.

## Status docs

`docs/gap-analysis*`, `docs/project-status.md` and `docs/guides/unified-uat.md` change once, in NR-51.
