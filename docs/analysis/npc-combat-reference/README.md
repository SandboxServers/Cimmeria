# NPC combat reference campaign (NC)

Ledger for "SGW Reconstruction Campaign 01: original NPC combat, weapons,
creature attacks and animation restoration" (brief from Lomiada, 2026-11).
Packets: [work-packets.md](work-packets.md) (rules, contract, wave 0),
[work-packets-w1-w2.md](work-packets-w1-w2.md) and
[work-packets-w3-w5.md](work-packets-w3-w5.md). Resume card: [RESUME.md](RESUME.md). W4 lab results: [w4-lab-results.md](w4-lab-results.md) (2026-11-27; open: blank PRU/ScavDog attacker name in the client combat log (known cosmetic, templates 4 and 76 ship no name, see unified-uat K26), NID respawn and T2 Z9 squad not exercised, Jaffa damage tuning, GM health commands fail).

## Purpose

Give four reference NPCs combat that is data-driven, visually appropriate and
independently tested, and stop creatures from inheriting the humanoid Pistol
Shot (592) placeholder. The four references are:

- **NID Guard** (template 24): human infantry with an SMG;
- **Praxis Jaffa Guard and Lieutenant** (160 and 159): staff, ranged and melee;
- **ScavDog** (76): an organic creature with melee;
- **Prisoner Retrieval Unit** (4): a mechanical construct with an energy weapon.

The whole campaign is testable in the **Debug Area (world 1300)**, in a new
zone Z12 "Combat reference range". It is isolated from CellBlock, Praxis and
mission content. Only the regression packets look at those worlds, and only
to prove they are unchanged.

## Verdict: feasible with re-scoping

The server and seed work is feasible and mostly small. The brief overstates
how much is broken, because several defects it lists are already fixed (F3).
It also understates the blast radius of removing the 592 fallback outright
(F2).

Presentation evidence needs the owner's local client copy: UPK animations,
notifies, FEV ownership, sockets and emitters. That work is isolated in NC-05,
NC-06 and the wave-4 lab UAT, and every other packet runs without the client.

Nothing needs a client patch, a new opcode or client UI. Every change is a
seed row or server-side selection logic, and the client already plays the
presentation that the existing event-set ids drive.

Re-scoped from the brief:

- "SQL seed changes **or migrations**": seeds only (repo rule).
- "Cross-reference every CookedData*.pak, historical SourceCache, interviews":
  only what NC-05 and NC-06 can reach in the owner's QA client, plus what the
  repo already holds. Other builds are UNRESOLVED until someone supplies them.
- "Cooked-data modifications": none are planned.
- The placeholder removal is staged. The resolver lands first with no
  behaviour change (`Legacy`). The flip to `RequireWeaponEvidence` landed in NC-30
  (D-NC1) after the tests were made explicit.
- Elite Jaffa specials and the wider roster are out of scope. NC-34 writes
  only the follow-up spec.

## Evidence classifications

Every registry row and every reconstructed seed comment uses exactly one of:

| Class | Meaning here |
|---|---|
| ORIGINAL CLIENT VERIFIED | Read from the owner's cooked client (UPK, pak, FEV) or SGW.exe RE, with the file and export cited |
| EARLIEST LEGACY DATA | The `db/resources` seeds (the Project Giza extraction) or the 2009 Python server, not checked against the client |
| CURRENT CIMMERIA IMPLEMENTATION | What the Rust server does today, cited by file and line |
| STRONGLY INFERRED | Indirect client evidence, such as a matching body family plus a generic KIS set, with no direct link |
| RECONSTRUCTED | Behaviour or data Cimmeria chose by judgement, in this campaign or an earlier PR, such as a damage payload taken from the ability's own description |
| UNRESOLVED | Unknown; listed in the registry's open list |

`db/resources` is Project Giza's approximation, not CME's database
(`.claude/agent-memory` reference note), so a seed row alone is never ORIGINAL.

A seed value is classed by its evidence, not by who wrote it. RECONSTRUCTED
covers any value Cimmeria chose by judgement, in this campaign or an earlier
PR (cite the PR). CURRENT CIMMERIA IMPLEMENTATION is reserved for Rust server
behaviour and is never used for a seed value. For an ability's
`event_set_id` link (wave-0 ruling D1, `worknotes/wave0.md`):

- the 35 links the Giza seed already held (non-NULL before PR #822,
  `f153138b5`) are EARLIEST LEGACY DATA (559, 584, 710 and 221 are among them);
- PR #822 Tier 1 (the weapon-family rule) and Tier 2 (what the weapon's items
  fire) are STRONGLY INFERRED;
- PR #822 Tier 3 (judgement calls, including 1433 and 983 to 300) is
  RECONSTRUCTED;
- ORIGINAL CLIENT VERIFIED needs a cooked client record naming that ability,
  such as an `<ItemEventSet AbilityID=…>` row; the client has no ability-keyed
  lookup (F11), so most links can never earn it.

The event set's own sequences and effect links stay EARLIEST LEGACY DATA,
because PR #822 changed only `abilities.event_set_id`.

## Findings

| # | Finding | Class | Packet |
|---|---|---|---|
| F1 | `NPC_DEFAULT_ABILITY = 592` is applied at spawn (`crates/cell-world/src/cell/space_manager/spawn.rs:314-325`, when `record.ability_ids` is empty) and at AI time (`crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:63-66`, when `known_count() == 0`). The code comment cites "the Python convention", so it is a Giza/Python placeholder, not CME behaviour. The raw `spawn_npc` primitive (`spawn.rs:37-39`) also adds it; it is mostly a test primitive | EARLIEST LEGACY DATA | NC-01, NC-10, NC-30 |
| F2 | About 75 templates have no `ability_set_id` and are hostile or spawned in mission worlds. Removing the fallback outright would make them stop attacking, mission mobs included. About 40 test files use 592 as their fixture | CURRENT CIMMERIA IMPLEMENTATION | NC-15, NC-21a-c, NC-30 |
| F3 | Already fixed, so no packet: the player-side 592 redirect (CS-07, OD-CS11); melee abilities firing at 30 m (`effective_max_range` uses `NPC_MELEE_RANGE`); multi-ability sets (H09); Jaffa sets 4 and 5 (H11); the ability-to-event-set links (#822, `docs/reverse-engineering/findings/ability-animation-links.md`; the 1433 and 983 links are RECONSTRUCTED Tier-3 calls, ruling D1) | CURRENT CIMMERIA IMPLEMENTATION | none |
| F4 | NID 24: set 3 is 559 (event set 15, effect 5310 since NC-25, `RangedPhysicalDamage`), with the `WP-Human.WP_SMG_1A` component, loot 2, `use_cover` true and assist 26. The 559 tooltip says -100F/-10H and effect 5310 executes 10H/100F (D-NC8, D-NC11, RECONSTRUCTED); Giza effect 621 (20H/200F) is kept unchanged and unused. `weapon_item_id` 21 (SGHC 6 SMG) has no server consumer (D-NC6). NPCs have no ammo gate and no reload | CURRENT CIMMERIA IMPLEMENTATION | NC-11 |
| F5 | Jaffa 159/160: set 4 (584 ranged, 710 melee), level 50, faction 1. NC-22 added the `WP-Jaffa.WP_Staff_Plasma_4A` staff component in place (D-NC3, RECONSTRUCTED), so the client shows the staff. Effect 736 (ability 710) had no script and no NVPs; NC-12 gave it `MeleePhysicalDamage` and NVPs (RECONSTRUCTED). Inside `NPC_MELEE_RANGE` an NPC now takes its off-cooldown melee swing first (D-NC4, NC-20), so 710 fires at melee reach and 584 elsewhere. Faction 1 means players cannot fight them | CURRENT CIMMERIA IMPLEMENTATION | NC-12, NC-20, NC-22 |
| F6 | ScavDog 76 had no ability set and fired 592 Pistol Shot, a humanoid firearm. NC-13 gives it set 6 = 1433 Mob Strike (RECONSTRUCTED, D-NC5), and NC-12 gave effect 1717 its damage (RECONSTRUCTED). 983 Bite (effect 1092, no event set and no script) stays unassigned. The 1433 and 983 → event set 300 links are PR #822 Tier-3 judgement calls (RECONSTRUCTED). `mob_scav.fev` has five variants of one `attack` event, and no notify in the package references them (NC-05) | EARLIEST LEGACY DATA / RECONSTRUCTED / UNRESOLVED | NC-05, NC-12, NC-13 |
| F7 | PRU 4: set 2 is 221 Energy Shock (effect 264, `RangedEnergyDamage`, 16H/80F, event sets 802/803). The damage type may be forced to physical (NC-07) | CURRENT CIMMERIA IMPLEMENTATION | NC-07, NC-14 |
| F8 | Templates 34 and 35 show a staff mesh but have no set, so they fired a pistol under `Legacy` and idle since NC-30. Other no-set templates with weapon components exist | CURRENT CIMMERIA IMPLEMENTATION | NC-15 |
| F9 | The Debug Area gallery (Z7) already holds 4, 24 and 76, but all of them are NEUTRAL (override 3) and there is no Jaffa. DA-04 template 1371 is a Praxis Jaffa with set 4 and the `WP-Jaffa.WP_Staff_Plasma_4A` staff, but it is faction 3, which players do not fight. No aggressive reference range exists yet | CURRENT CIMMERIA IMPLEMENTATION | NC-03 |
| F10 | Runtime suspects to re-verify: NPC mitigation 0/0, damage type forced to `DT_PHYSICAL`, DoT death firing no `entity_death`, and NPC LOS and target validation | UNRESOLVED | NC-07 |
| F11 | The client has no ability-keyed animation lookup, and the KIS packages are generic per mechanic. Presentation follows the event set (`event_sets_sequences` → `sequences.kismet_script_name`), so a correct event set is the server's whole lever | ORIGINAL CLIENT VERIFIED (ability-animation-links.md) | NC-04, NC-06 |
| F12 | No open PR overlaps (#2075 /gmkilltarget is adjacent). Related issues: #1667 (pistol on the guard: no fire animation or damage), #1681 (pets), #62 (respawn lifecycle), #330 (aggression override) | n/a | NC-24, NC-31 |

## Debug Area test bed (owner requirement)

NC-03 stages Z12 "Combat reference range" in world 1300 from seeds only:

- **Templates 1380 and 1381** clone the hostile Jaffa Guard and Lieutenant.
  They copy DA-04 1371's look column for column (body, components with the
  staff, colours, tint, mesh), so the lineup guard needs no new actor. They
  set faction 10, set 4 and name ids 8917 and 8918.
- **Spawns 14300-14319** (D-NC9), tagged `DebugArea_Ref_*`, with aggressive defaults
  (`aggression_override` NULL) and 20 s respawns. They place the **real**
  templates 24, 4 and 76 plus the clones.
- **Player fixtures** reuse what Z2 already has: the ability granter (template
  1300), the Basic Equipment Quartermaster (300) and the munitions vendor
  (1302). There are no new character seeds.

Every reference test, UAT row and telemetry check runs there. The regression
packets (NC-23 for tutorial and CellBlock, NC-24 for Harset and pets) are the
only ones that read other worlds.

## Client-asset needs

| Needs the owner's client copy | Runs without it |
|---|---|
| NC-05 (ScavDog UPK, animations, notifies, FEV), NC-06 (staff socket, projectile, PRU emitter, NID muzzle), and W4 live UAT NC-40..44 (lab client) | Every other packet: W0 NC-01..04 and NC-07, all of W1-W3, and NC-50/51 |

NC-13 (ScavDog gets 1433) runs without the client as RECONSTRUCTED if D-NC5
says yes. NC-05 upgrades or contradicts it later.

## Decisions

| Id | Question | Recommendation | Status |
|---|---|---|---|
| D-NC1 | What does an NPC with no ability set do? | Staged. `Legacy` (592) ships first with once-per-template diagnostics. Then `RequireWeaponEvidence`: 592 only with a `WP-Human.*` component; otherwise hold fire and log a WARN. Flip it only after the NC-21 test fixups and the four references pass | Decided (owner, 2026-11-27): an NPC with no ability set idles and logs a warning, with no 592 fallback. NC-30 flipped the policy (code kept 592 only for a template with a `WP-Human.*` component; D-NC13 then narrowed that to human firearms) |
| D-NC13 | Which `WP-Human.*` components earn 592? | Narrow the D-NC1 test to human firearms: a whitelist `HUMAN_FIREARM_PREFIXES` (pistol, SMG, AR, LMG, HMG, rifle, shotgun). A melee human (`WP_HandToHand_1A`, templates 28, 92, 1415, 1465) and a deployable part idle | Decided (owner): NC-50b narrows the test. Only firearm templates keep 592; the four melee humans and template 400 now hold fire (the gallery count is 74 idle) |
| D-NC2 | How to give 736 and 1717 their damage | Hand NVP rows 470-473 plus `MeleePhysicalDamage`, the #493 precedent (effect 656), labelled RECONSTRUCTED from each effect's own description. Do not widen the generator's player-only scope | Decided (owner, 2026-11-27): derive from the ability text, RECONSTRUCTED. Merged in #2089 |
| D-NC3 | Staff on the real 159/160 | Clones (1380/1381) in the Debug Area now. Add `WP-Jaffa.WP_Staff_Plasma_4A` to 159/160 in place after the Jaffa reference passes, with lineup rows if their look changes. Keep their faction 1 | Decided (owner, 2026-11-27): yes, staff on 159/160, RECONSTRUCTED. NC-22 implements |
| D-NC4 | Melee preference in reach | NPCs only: within `NPC_MELEE_RANGE`, an off-cooldown melee ability wins over ranged. Pets keep lowest-id | Decided (owner, 2026-11-27): yes, NPCs only |
| D-NC5 | Give ScavDog 1433 Mob Strike before the client evidence? | Yes, as RECONSTRUCTED: new set 6 = {1433}. 983 Bite stays unassigned until NC-05 finds evidence | Decided (owner, 2026-11-27): yes, as RECONSTRUCTED |
| D-NC6 | Make `weapon_item_id` load-bearing? | No. The `WP-*` visual component is the weapon evidence; `weapon_item_id` stays informational | Decided (owner, 2026-11-27): `weapon_item_id` stays informational |
| D-NC7 | Live-client UAT in the lab | Ask first, per run. Use Haiku lab-drivers at Z12, verified against the server log | Decided (owner, 2026-11-27): lab approved in principle, ask before each run |
| D-NC8 | 559's tooltip (-100F/-10H) against effect 621 (-200F/-20H) | No change. The effect row executes; record it as UNRESOLVED in the registry | Decided (owner, 2026-11-27): follow the 559 tooltip, RECONSTRUCTED. Implemented by the D-NC11 split: ability 559 uses new effect 5310 (10H/100F); effect 621 is not rewritten and stays unchanged (NC-25) |
| D-NC9 | NC-03 blocked: C3's spawns 14110-14129 belong to the DR-03b Armory (14110-14299, rows 14110-14116). Where does Z12 go? | Spawns 14300-14319 (14300-14304 used), footer floor 14319; templates 1380/1381 stand. Checked against `origin/main` 3a20a96a0: no spawnlist row from 14117 to 15799 and no doc reserves 14300+. The coder's 13700-13719 was rejected: it lies in DA-04's block, and `debug_area/reach.rs` `the_reach_guards_see_da04_rows` counts every 13600-13799 row against `spawnlist_debug_area_combat.sql` alone, so a separate reference file breaks it. C3, C4 and NC-03 are re-specced; re-dispatch NC-03 | Decided (architect ruling, 2026-11-27) |
| D-NC10 | The 559 tooltip says `1 Ammo`, but the ability row has `required_ammo` 3 | No change in this campaign. NPCs have no ammo gate, so only players see it; leave it to an items or ammo campaign | Record only (2026-11-27): no change here; does not gate NC-25 |
| D-NC11 | How does 559 follow its tooltip (D-NC8): rewrite effect 621 in place, or give 559 its own effect row? | Rewrite 621 in place: it is the same row and the same tooltip | Decided (owner, 2026-11-27): split. Ability 559 gets its own new effect row, 5310, with the tooltip's 10H/100F, RECONSTRUCTED; effect 621 stays unchanged. This replaces D-NC8's "effect 621 is rewritten". NC-25 implemented it (D-NC12 (a)). |
| D-NC12 | The split does not isolate the NID. 559's damage resolves only through `abilities.effect_ids`, and no other ability uses 621, so repointing 559 to 5310 changes the NID Guard, all 29 templates on set 3 (the Z9 lethal squad 1376 among them) and the player SMG and assault-rifle auto attack (`items_event_sets` event 7) alike, and leaves 621 unused. Which outcome? | (a) Accept it: everything that fires 559 follows its own tooltip, which is also the tooltip players read on their weapon; NC-25 as written, with the Z9 re-measure. (b) NID only, through a new server-only ability on a new set for template 24: the new ability id reaches the client in `onEffectResults`, and unknown cooked ids have crashed it before (#938), so it needs a lab probe and a re-spec. (c) NID only, through a new server-side per-set effect override (559 unchanged on the wire): a schema column and resolver code, about two packets. (d) No change: 621 executes for everyone and O1 stays UNRESOLVED. Recommend (a) | Decided (a) (owner, 2026-11-27): everything that fires 559 follows its own tooltip, so repointing 559 to 5310 also changes the 29 set-3 templates, the Z9 squad and the player SMG and assault-rifle auto attack; NC-25 implements it with the Z9 re-measure (9 ticks). |

## Packets

| Id | Wave | Agent | What | Depends | Status |
|---|---|---|---|---|---|
| NC-01 | 0 | packet-coder | Combat-profile resolver module (pure) | none | Integrated |
| NC-02 | 0 | packet-coder | Evidence-registry skeleton | none | Integrated |
| NC-03 | 0 | rust-gameserver-dev | Debug Area Z12 reference range (seeds, tests, doc) | none | Integrated |
| NC-04 | 0 | packet-coder | Seed presentation-chain trace | none | Integrated |
| NC-05 | 0 | game-archaeology-specialist | ScavDog client assets (**client**) | owner client | Integrated |
| NC-06 | 0 | game-archaeology-specialist | Staff, PRU and NID presentation assets (**client**) | owner client | Integrated |
| NC-07 | 0 | combat-systems-advisor | Runtime blockers re-verified | none | Integrated |
| NC-10 | 1 | packet-coder | Wire the resolver and its diagnostics into spawn and AI | NC-01 | Integrated (#2089) |
| NC-11 | 1 | packet-coder | NID 24 reference tests | NC-01, NC-03 | Integrated (#2089) |
| NC-12 | 1 | packet-coder | Damage payloads for 736 and 1717 | NC-03, D-NC2 | Integrated (#2089) |
| NC-14 | 1 | packet-coder | PRU 4 reference tests | NC-01, NC-03 | Integrated (#2089) |
| NC-15 | 1 | packet-coder | Placeholder inventory (templates, callers, tests) | none | Integrated (#2089) |
| NC-13 | 2 | packet-coder | ScavDog set 6 = 1433 | NC-12, D-NC5 | Integrated (W2 PR) |
| NC-20 | 2 | packet-coder | NPC melee preference inside reach | NC-03, D-NC4 | Integrated (W2 PR) |
| NC-21a | 2 | packet-coder | Make 592 explicit in tests, cell-world | NC-15 | Integrated (W2 PR) |
| NC-21b | 2 | packet-coder | Make 592 explicit in tests, cell-combat | NC-15 | Integrated (W2 PR) |
| NC-21c | 2 | packet-coder | Make 592 explicit in tests, other crates | NC-15 | Integrated (W2 PR) |
| NC-23 | 2 | packet-coder | Regression: tutorial and CellBlock combat unchanged | NC-10, NC-12 | Integrated (W2 PR) |
| NC-24 | 2 | packet-coder | Regression: Harset Jaffa and Jaffa pets | NC-12 | Integrated (W2 PR) |
| NC-22 | 3 | packet-coder | Staff component on 159/160 in place | NC-12, NC-20, D-NC3 | Integrated (W3 PR) |
| NC-30 | 3 | packet-coder | Flip to `RequireWeaponEvidence` | NC-10, NC-21a-c, NC-11..14, D-NC1 | Integrated (W3 PR) |
| NC-31 | 3 | documentation-writer | Fill the evidence registry and the unresolved list | NC-02, NC-04..07, NC-11..14 | Integrated (W3 PR) |
| NC-33 | 3 | packet-coder | Live UAT checklist and debug-area.toml rows | NC-03, NC-11..14 | Integrated (W3 PR) |
| NC-34 | 3 | documentation-writer | Roster follow-up spec (Carnosaur, Horden, Rhinolion, AMBRat, Lenny, Unas) | NC-15, NC-31 | Integrated (W3 PR) |
| NC-25 | 3 | packet-coder | Ability 559 uses its own tooltip effect 5310 (10H/100F, RECONSTRUCTED); effect 621 kept | NC-11, NC-31, NC-33, D-NC8, D-NC11, D-NC12 | Integrated (W3 PR) |
| NC-40 | 4 | orchestrator + lab-driver | Live UAT: NID at Z12 | W3, D-NC7 | Lab run done (2026-11-27, [w4-lab-results.md](w4-lab-results.md)) |
| NC-41 | 4 | orchestrator + lab-driver | Live UAT: Jaffa at Z12 | W3, D-NC7 | Lab run done (2026-11-27, [w4-lab-results.md](w4-lab-results.md)) |
| NC-42 | 4 | orchestrator + lab-driver | Live UAT: ScavDog at Z12 | W3, D-NC7 | Lab run done (2026-11-27, [w4-lab-results.md](w4-lab-results.md)) |
| NC-43 | 4 | orchestrator + lab-driver | Live UAT: PRU at Z12 | W3, D-NC7 | Lab run done (2026-11-27, [w4-lab-results.md](w4-lab-results.md)) |
| NC-44 | 4 | orchestrator | Telemetry check: diagnostics and no 592 on creatures | W3, D-NC7 | Lab run done (2026-11-27, [w4-lab-results.md](w4-lab-results.md)) |
| NC-32 | 4 | documentation-writer | NPC combat reference docs page (no lab) | NC-30, NC-31 | Integrated (#2092) |
| NC-50 | 5 | packet-reviewer loop | Campaign review | W4 | In progress |
| NC-51 | 5 | documentation-writer | Close-out: status docs, unified-uat, retire | NC-50 | BlockedDependency |

## Dependency graph

```text
W0  NC-01 ──┬─────────────► NC-10 ─────────────┬─► NC-23 ─┐
            │                                   └─► NC-30 ◄─ NC-21a/b/c ◄─ NC-15
    NC-03 ──┼─► NC-11 ─┐                                ▲
            ├─► NC-14 ─┤                                │ (D-NC1)
            ├─► NC-12 ─┼─► NC-13 (D-NC5)                │
            │          ├─► NC-24                        │
            │          └─► NC-22 (D-NC3) ◄─ NC-20 (D-NC4)
            └─► NC-20
    NC-02, NC-04, NC-05*, NC-06*, NC-07 ──► NC-31 ──► NC-34
    NC-11..14 ──► NC-33          NC-30 + NC-31 ──► NC-32 (W4, no lab)
    NC-11 + NC-31 + NC-33 ──► NC-25 (D-NC8, D-NC11, D-NC12)
    W3 done ──► W4 NC-40..44 (D-NC7, lab, *client) ──► NC-50 ──► NC-51
    (* needs the owner's client copy)
```

## Waves and shipping

| Wave | Packets | Ships as | Merge order |
|---|---|---|---|
| 0 | 7 (widest fan-out, with W2) | One PR per packet; file sets are disjoint | NC-01, NC-03, NC-07, NC-04, NC-02, NC-05, NC-06 |
| 1 | 5 | One PR per packet | NC-10, NC-15, NC-11, NC-14, NC-12 |
| 2 | 7 | One PR per packet | NC-21a, NC-21b, NC-21c, NC-13, NC-20, NC-23, NC-24 |
| 3 | 6 | One PR from the W3 integration branch (`nc-w3`); each packet merges into it with `merge-noff`; NC-25 is cut from it after NC-31 and NC-33 | NC-22, NC-31, NC-33, NC-34, NC-25, NC-30 last (after a green workspace nextest) |
| 4 | 5 lab sessions + NC-32 | The lab sessions have no code; their rows are attested by one docs PR (`nc-uat`). NC-32 is its own PR | NC-32, then the attestation PR |
| 5 | 2 | The campaign-review fix PR, then the close-out PR | NC-50, NC-51 |

Status docs (`docs/gap-analysis*`, `docs/project-status.md`) change only in
NC-51.
