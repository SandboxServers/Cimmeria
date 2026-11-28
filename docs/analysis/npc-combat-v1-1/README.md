# NPC combat V1.1: Horden and Straegis Titan

Type: campaign ledger. Audience: the orchestrator, packet agents and the campaign owner. Last updated: 2026-11-27.
Companions: [work-packets.md](work-packets.md) (rules, contract, waves 0-1), [work-packets-w2-w5.md](work-packets-w2-w5.md) (waves 2-5), [RESUME.md](RESUME.md).
Follows: [npc-combat-reference](../npc-combat-reference/README.md) (V1). Sibling: [npc-combat-v2](../npc-combat-v2/README.md).

## Purpose

Restore two creatures from asset research to live Debug Area validation, reusing the V1 combat infrastructure. Horden is template 72 (`MOB_Horden.upk`). The Straegis Titan is template 79 (`MOB_StraegisTitan.upk` plus `Straegis-VFX.upk`).

The campaign:

- never substitutes a generic attack or an unrelated animation to make a creature look functional;
- labels every row with one of the six V1 evidence classes;
- records genuine external blockers as gates instead of inventing original behaviour;
- stays independent of the CellBlock-to-Tollana audit;
- keeps every validated V1 behaviour (D-NC1 to D-NC13) unchanged.

Packet prefix `NT`, worktrees `nt-<id>`, branch prefix `npc-combat-v1-1/`, decisions `D-NT<n>`.

## Evidence classes

These are the six V1 classes, unchanged ([V1 README § Evidence classifications](../npc-combat-reference/README.md)): ORIGINAL CLIENT VERIFIED, EARLIEST LEGACY DATA, CURRENT CIMMERIA IMPLEMENTATION, STRONGLY INFERRED, RECONSTRUCTED and UNRESOLVED.

- A seed row alone is never ORIGINAL.
- An export read from a client package is ORIGINAL CLIENT VERIFIED for the asset's existence and content only. Pairing that asset with an ability is at best STRONGLY INFERRED.

## Findings (architect research, 2026-11-27)

| # | Finding | Class | Source | Settles / acts |
|---|---|---|---|---|
| F1 | There is effectively **one client build** locally. The three local trees (`..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`, the `Client\game` copy and the editor download) hold byte-identical `MOB_Horden.upk` (530,673 B) and `MOB_StraegisTitan.upk` (4,283,273 B), with the same SHA-256. The QA and editor trees also hold `Straegis.upk` (13.6 MB), the 2008 "super build" StrAegis extra. The external SGW_FINAL bundles are indexes only (agent memory `project_final_re_bundles`). | ORIGINAL CLIENT VERIFIED (file hashes) | local trees, hashed by the architect | NT-06 |
| F2 | `MOB_Horden.upk` (engine 3004, licensee 8, uncompressed) has **10 exports**: BodyComponent `MOB_Horden00`, BodySet `BS_MOB_Horden`, MIC `Tol-Horden00_MI`, SkeletalMesh `MOB_Horden`, 4 textures (`Tol-Horden00_D/N/S/TM`) and 2 redirectors. It has **no AnimSet, AnimTree, AnimSequence or PhysicsAsset**, its imports are Core and Engine classes only, and the seed gives the mesh no sockets. The bone names (`b_Bubble*`, `b_Squish*`, `b_Mouth*`, `b_arm*`) describe a blob creature. No other cooked package contains the string "Horden". | ORIGINAL CLIENT VERIFIED | `tools/upk_parser.py` export and name tables; byte search over the QA CookedPC | gate G1; D-NT1, D-NT2 |
| F3 | Neither creature has an audio bank. `Content\audio\mobs` holds only `mob_caDroneFlyer` and `mob_scav`. | ORIGINAL CLIENT VERIFIED | QA tree listing | gate G2 |
| F4 | Horden has no ability, effect, sequence, `name_id` or mission text in the seeds or in any cooked pak (the cooked abilities, effects, Kismet, missions and items paks were searched; zero hits). Its original attack is therefore UNRESOLVED, with no evidence to recover. | EARLIEST LEGACY DATA (absence) | `entity_templates.sql:115`; cooked pak search | D-NT2 |
| F5 | `MOB_StraegisTitan.upk` has **46 exports**. AnimSet `StraegisTitan_Anim` holds **12 AnimSequences** (exports 17-28). The name table gives `CreateTasker`, `DentonateTasker_FINAL` (the asset spells it that way), `DIdle1`, `DMelee_FINAL`, `DRAttack_FINAL`, `DRunF`, `DTransition`, `sHit`, `sIdle1`, `sRunF` and `sWalkF`; NT-04 confirms the twelfth, probably a death. The package also has an AnimTree `MOB_StraegisTitan_Animtree` (BlendBySpeed, BlendList and three AnimNodeSequence nodes named Idle, Walking and Running), the PhysicsAsset `Mesh_StraegisTitan_Physics`, SkeletalMesh `Mesh_StraegisTitan` with one socket, the BodySet `BS_MOB_StraegisTitan` (2,547 B) and the BodyComponent `MOB_StraegisTitan00`. BodyParticles, BodySounds and KismetData appear in the name table. | ORIGINAL CLIENT VERIFIED | upk_parser dump | NT-04 |
| F6 | `Straegis-VFX.upk` (35,241 exports) holds exactly **17 `Str-Titan*` particle systems**: `Str-TitanBase`, `-TitanCrackingLegs`, `-TitanCreateTasker`, `-TitanDeath`, `-TitanDeathGround`, `-TitanDeathHead`, `-TitanDeathHeart`, `-TitanDeathLegs`, `-TitanDetonateTasker`, `-TitanHeart`, `-TitanHeartD`, `-TitanHit`, `-TitanIdle`, `-TitanIdleD`, `-TitanMelee`, `-TitanRangedAttack` and `-TitanTransition`. "CreateTasker" appears only in the Titan package and in this one; "StraegisTitan" also appears in `GLB_Components.upk`. | ORIGINAL CLIENT VERIFIED | upk_parser dump; byte search | NT-05 |
| F7 | **No seed and no cooked pak links a Titan animation or a `Str-Titan*` system to anything.** No ability names the Titan, and the cooked abilities, effects and Kismet paks contain no "Titan". The Titan appears in cooked data only as interaction topics (dialog sets 167 and 231), as mission steps ("Recover a Titan Theme crystal", missions 97 and 265, `A04_Anm_D_20KillATitan` "A Different Drummer") and as item 2041 "Theme crystal from a Straegis Titan". The Titan was a designed boss encounter whose combat data never reached this build's cooked catalogue, so **any Titan ability kit is RECONSTRUCTED**. | EARLIEST LEGACY DATA (cooked absence); ORIGINAL CLIENT VERIFIED (topics, steps, item) | cooked pak search; `name_id` 7135 `DN_Mb_Straegis_Titan_Boss_Anima_40-42` | D-NT3; gate G4 |
| F8 | The server side of "motionless": template 79 has `ability_set_id` NULL, so under `RequireWeaponEvidence` (D-NC1) it resolves `Unresolved`, holds fire and logs `combat_profile_unresolved`. Its spawns are gallery 13306 (Z7, neutral, "idles, warns"), lineup 1570 (spawn 14030) and nothing else. Body scale is 35.15 in `body_sets.sql` (Horden 1.68), and the 35 m Titan clips the gallery walls (`docs/content/debug-area.md` about line 798). Whether its **idle** also fails to animate (bind pose) is UNRESOLVED; NT-04 and NT-07 settle it. | CURRENT CIMMERIA IMPLEMENTATION | `entity_templates.sql:160`; V1 placeholder inventory | NT-07, NT-09 |
| F9 | A presentation route that needs **no client patch** exists. A cooked `CookedDataKismetSeqEvent` entry pairs a generic KIS with NVPs: `_2822` uses `KIS-abilities_human.KIS-SA_Effect_Target` with `pfx1=Straegis-VFX.Str-AuraChemical`, `pfx1Socket=Heart`, `cAnimOneShot`/`sAnimOneShot=sEmitAura` and `useAnims=1`. The server sends only the sequence id (`NameValuePairs` count 0, `use_ability/sequence.rs:74`), and the client reads the NVPs from its own catalogue. `crates/resources/src/base/sequence_overrides.rs` adds catalogue entries at startup and resyncs clients (#840), but `SequenceOverride` has **no NVP field** today. NT-02 adds it. Whether `sAnimOneShot` can play a Titan `D*` sequence through its AnimTree (no slot node was seen) is UNRESOLVED; NT-07 settles it. | ORIGINAL CLIENT VERIFIED (`_2822`, `_2846`); CURRENT CIMMERIA IMPLEMENTATION (overrides) | `data/cache/CookedDataKismetSeqEvent.pak`; `sequence_overrides.rs:33-41,136-154` | NT-02, NT-07, NT-12b |
| F10 | The client's cooked abilities catalogue ends at id 3498, the same as the seed. A new Titan ability id (3500+) is unknown to the client, and the repo has no ability-override category (`crates/resources/src/base/` has dialog, item, mission, sequence, stargate and world-info overrides). Effect ids are server-side: NID's effect 5310 is past the cooked maximum of 5309 and works (NC-40). What the client shows for an NPC ability id it has never seen is UNRESOLVED; it may relate to the V1 blank-name anomaly K26. | ORIGINAL CLIENT VERIFIED (catalogue keys); UNRESOLVED (display) | cooked pak key scan | NT-08; D-NT4 |
| F11 | The Titan's sequence names split into an `s` set (`sIdle1`, `sWalkF`, `sRunF`, `sHit`) and a `D` set (`DIdle1`, `DRunF`, `DMelee_FINAL`, `DRAttack_FINAL`), joined by `DTransition`, with particle pairs `TitanIdle`/`TitanIdleD` and `TitanHeart`/`TitanHeartD`. **Both attacks exist only in the `D` set.** Two hypotheses remain: (a) `D` is the client's combat stance (selected by `BSF_InCombat`, `npc-attack-presentation.md`), which needs no server mechanic; (b) `D` is a damage phase, which needs a health-threshold mechanic. | STRONGLY INFERRED (names); UNRESOLVED (meaning) | F5 | NT-04, NT-07; D-NT6 |
| F12 | Free id blocks, checked 2026-11-27 against every seed and the cooked catalogue keys, are in [work-packets.md § C2](work-packets.md#c2-id-blocks-and-tags). | CURRENT CIMMERIA IMPLEMENTATION | id scan of `db/resources` and the cooked paks | NT-03 onward |

### Provisional Titan kit (RECONSTRUCTED proposal; NT-10 confirms or amends it)

This is a pairing by matching names only, which is STRONGLY INFERRED at best. It is not an original kit.

| Role | Titan AnimSequence (F5) | Titan particle system (F6) | Server ability |
|---|---|---|---|
| Idle, out of combat | `sIdle1` | `Str-TitanIdle`, `Str-TitanHeart`, `Str-TitanBase` | none (AnimTree, BodyParticles) |
| Move | `sWalkF`, `sRunF`; `DRunF` in combat | none | none (BlendBySpeed) |
| Enter combat | `DTransition` | `Str-TitanTransition`, `Str-TitanCrackingLegs` | none, or a phase per D-NT6 |
| Combat idle | `DIdle1` | `Str-TitanIdleD`, `Str-TitanHeartD` | none |
| Melee | `DMelee_FINAL` | `Str-TitanMelee` | new, id 3500 |
| Ranged | `DRAttack_FINAL` | `Str-TitanRangedAttack` | new, id 3501 |
| Create tasker | `CreateTasker` | `Str-TitanCreateTasker` | new, id 3502, per D-NT5 |
| Detonate tasker | `DentonateTasker_FINAL` | `Str-TitanDetonateTasker` | new, id 3503, per D-NT5 |
| Hit reaction | `sHit` | `Str-TitanHit` | none (KismetData hit hook, NT-04) |
| Death | twelfth sequence (NT-04) | `Str-TitanDeath`, `-DeathGround`, `-DeathHead`, `-DeathHeart`, `-DeathLegs` | death event set (NT-12a) |

## External gates

| Gate | What blocks | Why it is external | Effect on "done" |
|---|---|---|---|
| G1 | Horden animation, AnimTree and physics | Absent from every local build (F1, F2). Recovery needs another historical SGW client build that the project does not hold. Never rehost client files. | Horden can be no more than a static, correctly textured mesh unless D-NT1 changes this |
| G2 | Horden and Titan sound | Neither creature has an FMOD bank (F3). Adding one would be a client patch. | Every sound row is UNRESOLVED |
| G3 | Sound verification in the lab | The lab has no audio device (V1 W4 results) | Sound rows stay NOT VERIFIED even where a bank exists |
| G4 | An original Titan combat kit | Never cooked in this build (F7) | The Titan kit is RECONSTRUCTED at best, and is labelled so everywhere |
| G5 | Every research packet | The client tree is not in git; packets read the owner's local copy | A packet without the tree returns `blocked - no client tree` |

## Decisions

| ID | Decision | Status | Recommendation and reason |
|---|---|---|---|
| D-NT1 | Horden animation source | **BlockedDecision** | Record G1 and close Horden animation as an external blocker. Ask the owner whether any other SGW client build (pre-QA, press or editor) exists that ships a `MOB_Horden` AnimSet. Authoring a new AnimSet would be RECONSTRUCTED and need a client patch plus a UE3 engine-3004 toolchain, so it belongs to a separate campaign, if any. |
| D-NT2 | Horden combat | **BlockedDecision** | **Hold fire** (D-NC1 as today), pinned by a test, with the gate written in the registry. There is no attack evidence at all (F4), and with no animation any attack is an invisible generic substitute, which the brief forbids. Alternative: a RECONSTRUCTED creature melee using 983 Bite (the NC-34 shape), labelled RECONSTRUCTED in the seed, registry and docs. |
| D-NT3 | Titan kit source | **BlockedDecision** | New **Titan-only** ability rows 3500-3503, RECONSTRUCTED. Their presentation uses the Titan's own sequences and `Str-Titan*` systems (original assets; the pairing is STRONGLY INFERRED). Damage is RECONSTRUCTED for a level 40-42 boss (`name_id` 7135). Do not reuse the Straegis Fighter abilities 1156, 2847 or 1240: their event sets drive the Fighter pet's presentation (`sEmitAura`), which the Titan lacks, so repointing them would break the pet. |
| D-NT4 | Client display of a new ability id | **BlockedDecision** (asked once NT-08 lands) | If NT-08 shows the client blanks or drops an ability id outside its catalogue, add an `ability_overrides` cooked-catalogue category with the same startup resync as `sequence_overrides` and `item_overrides`. That is server-side and needs no client patch. If the client shows the server's name, nothing is needed. |
| D-NT5 | The tasker mechanic | **BlockedDecision** (asked once NT-05 lands) | Implement only what the assets show. Default if NT-05 finds no minion mesh or template: `CreateTasker` is a Titan cast that plays its animation and `Str-TitanCreateTasker` and marks a ground point; `DentonateTasker` detonates that point as a delayed area hit with `Str-TitanDetonateTasker`. No minion entity, RECONSTRUCTED. Use a minion only if NT-05 finds one. |
| D-NT6 | The meaning of the `s`/`D` stance | Pending NT-04 and NT-07; owner only for (b) | (a) If `D` is the combat stance, no server mechanic is needed beyond `BSF_InCombat`. (b) If it is a damage phase, switch at 50% health (RECONSTRUCTED), which needs the owner's OK. |
| D-NT7 | How animation and VFX are verified in the lab | **BlockedDecision** | The Haiku lab driver takes `lab_screenshot` with `image: true` at fixed beats, and the owner or user watches one short session to sign off animation and VFX sync. Sound stays NOT VERIFIED (G3). V1 verified no presentation at all, because its driver could not see screenshots. |
| D-NT8 | Titan loot | **BlockedDecision** | Give the Debug Area Titan spawn only (the spawn's `loot_table_id`) a new loot table holding item 2041 "Titan Theme Crystal" (a RECONSTRUCTED link, from the item text). World template 79 and mission A04 are unchanged; a mission campaign owns them. Alternative: no loot change. |
| D-NT9 | Titan scale and placement | Decided (architect) | Keep `body_sets` scale 35.15 (NT-04 compares it with the client BodySet). Place the Titan in a new open-field Z13 pen, away from every wall, at least 120 m from every other spawn, respawner and ring station. |
| D-NT10 | Lab use | Decided (inherits D-NC7) | Approved in principle. Each run asks "may I use the lab now?" first and waits for a yes. |
| D-NT11 | World placement | Decided (architect) | No world spawns change. Template 79 spawns only in the Debug Area today (F8), and the gallery spawn 13306 must stay neutral (NT-13 test). |
| D-NT12 | Release shape | Decided (architect) | Wave 2 ships as one PR from the integration branch `nt-w2`, so a Titan set never lands without its presentation. Every other wave ships one PR per packet. |

## Packets

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done.

| Packet | Title | Wave | Agent | Depends | Status |
|---|---|---|---|---|---|
| NT-01 | V1.1 evidence registry skeleton | 0 | packet-coder | none | Ready |
| NT-02 | `SequenceOverride` NVPs | 0 | packet-coder | none | Ready |
| NT-03 | Debug Area Z13 creature range and test stubs | 0 | rust-gameserver-dev | none | Ready |
| NT-04 | Titan package assets | 1 | game-archaeology-specialist | G5 | Ready |
| NT-05 | Titan VFX, KIS and the tasker | 1 | game-archaeology-specialist | G5 | Ready |
| NT-06 | Horden cross-build search and reconstruction options | 1 | game-archaeology-specialist | G5 | Ready |
| NT-07 | Client creature animation path (Ghidra) | 1 | game-archaeology-specialist | none | Ready |
| NT-08 | Client handling of unknown NPC ability ids (Ghidra) | 1 | game-archaeology-specialist | none | Ready |
| NT-09 | Server runtime trace for templates 72 and 79 | 1 | npc-ai-spawn-advisor | none | Ready |
| NT-10 | Titan kit spec (re-slice checkpoint) | gate | architect | NT-04..09 | BlockedDependency |
| NT-11 | Titan abilities, effects and NVPs (seed) | 2 | packet-coder | NT-10, D-NT3 | BlockedDecision |
| NT-12a | Titan event sets and sequences (seed) | 2 | packet-coder | NT-10 | BlockedDependency |
| NT-12b | Titan sequence overrides | 2 | packet-coder | NT-02, NT-12a | BlockedDependency |
| NT-13 | Titan set, template and loot | 2 | packet-coder | NT-11, D-NT8 | BlockedDecision |
| NT-14 | Titan stance or phase mechanic | 2 | rust-gameserver-dev | NT-10, D-NT6 | BlockedDependency |
| NT-15 | Tasker mechanic | 2 | rust-gameserver-dev | NT-10, D-NT5 | BlockedDecision |
| NT-16 | Titan reference tests | 2 | packet-coder | NT-03, NT-13 | BlockedDependency |
| NT-17 | Horden disposition | 3 | packet-coder | D-NT1, D-NT2 | BlockedDecision |
| NT-18 | Ability overrides (only if D-NT4 says yes) | 3 | rust-gameserver-dev | NT-08, D-NT4 | BlockedDecision |
| NT-19 | Lab prep and UAT checklist | 3 | packet-coder | wave 2 merged | BlockedDependency |
| NT-20 | V1 regression verify | 3 | packet-coder | wave 2 merged | BlockedDependency |
| NT-30 | Lab: Titan | 4 | orchestrator with a lab-driver (lab-uat skill) | NT-19, D-NT7, ask first | BlockedDecision |
| NT-31 | Lab: Horden | 4 | orchestrator with a lab-driver | NT-17, NT-19, ask first | BlockedDecision |
| NT-32 | Lab fix packets (reserved, cut from NT-30/31 findings) | 4 | packet-coder | NT-30, NT-31 | BlockedDependency |
| NT-40 | Campaign review loop | 5 | packet-reviewer | wave 4 | BlockedDependency |
| NT-41 | Close-out | 5 | documentation-writer | NT-40 | BlockedDependency |

## ETA (wall clock, wide fan-out)

| Phase | Parallel width | Wall clock | Gated by |
|---|---|---|---|
| Waves 0 and 1 together | 9 agents | 2-4 h (RE packets are the long pole) | the client tree (G5) |
| NT-10 kit spec, plus owner decisions D-NT1-5, 7 and 8 | 1 agent | 45 min, plus the owner's reply time | the owner |
| Wave 2 (integration branch `nt-w2`) | 6-7 agents | 3-4 h, including packet and wave reviews | NT-10 |
| Wave 3 | 4 agents | 1.5-2 h | wave 2 merged |
| Wave 4 lab | 1 lab session at a time | 2-3 h for two creatures, plus fix packets | the lab being free, and a yes |
| Wave 5 review and close-out | 2 | 1.5-2 h | wave 4 |
| **Total** | | **11-16 h of wall clock**, plus owner and lab waits | |

## Cost-aware note

- The cost drivers are the six RE packets (domain agents on their defined models; the Ghidra work in NT-07 and NT-08 is the heaviest) and the lab sessions.
- Implementation runs on Haiku `packet-coder` at about 40k working tokens per packet. Keep each session fresh: Haiku bills 5x above a 100k prompt.
- Reviews run on Sonnet `packet-reviewer`, about $0.68 a pass (campaign-metrics: $13.58 for 20). Fix rounds on Haiku are cents ($0.72 for 17 rounds).
- To save money: run NT-07 and NT-08 in one Ghidra session only if one agent is free and the other is not (they touch different call paths). Skip NT-18 when D-NT4 says no. Run NT-31 inside the NT-30 lab session (one lease, one login).
- Never escalate to Opus for code. Opus rules only (NT-10, and the review-loop rulings).

## Status docs

`docs/gap-analysis*`, `docs/project-status.md` and `docs/guides/unified-uat.md` change once, in NT-41. Per-packet progress lives in this folder.
