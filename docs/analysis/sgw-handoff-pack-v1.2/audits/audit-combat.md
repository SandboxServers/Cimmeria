# Combat pipeline audit — handoff pack v1.2 "Final v1" vs. existing Cimmeria implementation

**Scope:** read-only audit. No repo file was modified. Sources read: the pack's
`docs/SOURCE_POLICY.md`, `docs/COMBAT_SPEC.md`, `docs/COMBAT_ACCURACY_COVER_FINAL_V1.md`,
`docs/COMBAT_SYSTEM_FINAL_V1.md`, `docs/QA_TESTS.md`; the repo's
`docs/reverse-engineering/findings/combat-damage-analysis.md` and `cover-system.md`;
the three in-flight evidence-ledger branches (read via `git show`, never checked out);
and the live Rust combat surface.

---

## Existing combat surface

| Block | What Cimmeria does today | File:line | Evidence label |
|---|---|---|---|
| (a) Hit roll | **QR score, not hit-chance %.** `coordination×0.05` (ranged) or `engagement×0.05` (melee) − `perception×0.05` + `accuracy×0.01` − `defense×0.01` ± `qrMod` + `awareness×0.01`. Hard-coded. Sampled via `Beta(1.4, 1.4+qr×2.0)`, banded at 0.07/0.20/0.80/0.93 → Miss/Glancing/Hit/Crit/DoubleCrit | `crates/services/src/cell/combat/damage/qr.rs:50-73`, `:21-30`, `:105-110` | **Units CONFIRMED** (`entities/defs/alias.xml:193-206`, 100 pts = 1 QR). **Distribution + thresholds RECONSTRUCTION** — from `deprecated/python/common/Config.py:38-41`, corroborated by nothing in shipped data |
| (a′) Miss gate | **None.** `result_code` is forwarded to the client only; `calculate_damage` runs unconditionally with `qr_rand` as a damage multiplier, so a MISS still deals up to 14% of base | `crates/services/src/cell/abilities/damage_apply/mod.rs:100,157,250`; `crates/services/src/cell/combat/damage/pipeline.rs:44` | Defect |
| (b) Cover tiers | Full geometry/AI system: loader, spatial grid, scoring, slot reservation, player detection. Two-axis `ECoverHeight` Low/Mid/High/LOS × `ECoverQuality` Good/Better/Best/None. **Directional arc already implemented**: `orient ± π/2` with 5° hysteresis. 5 m player proximity radius | `crates/services/src/cell/cover/scoring.rs:19,90-97`; `crates/services/src/cell/cover/detection.rs:32`; `crates/services/src/cell/cover/types.rs:114` | Node data + enums **CONFIRMED**; weights RECONSTRUCTION |
| (b′) Cover→combat | **Zero coupling.** `calculate_qr` reads no cover or crouch term. `COVER_QR_MODIFIER`(48), `COVER_ACCURACY`(66), `COVER_DEFENSE`(67), `CROUCHING_ACCURACY`(68), `CROUCHING_DEFENSE`(69) all exist with **max = 0**, so any buff is silently a no-op. `BSF_CROUCHING` (bit 2) is broadcast-only | `crates/services/src/cell/combat/damage/qr.rs:47`; `crates/entity/src/stats/stat_list.rs:68,96-99`; `crates/services/src/cell/cell_methods/combatant.rs:41-43` | Defect |
| (c) Cover penetration | Nothing | — | — |
| (d) Armor | Per-damage-type Armor Factor + Density, **no slot weights**. `af_mitigation = round(AF × max(0, mitigation − penetration)/100)`, then flat subtraction from post-QR damage. No cap | `crates/services/src/cell/combat/damage/pipeline.rs:57-59,64,181-189` | Penetration/mitigation **semantics CONFIRMED** (`alias.xml:225,243`); combination RECONSTRUCTION |
| (e) Resistance | `HEALTH: fortitude×0.01 + healthRes×0.01`; `FOCUS: intelligence×0.01 + mentalRes×0.01`; applied `×(1−r)`. **Uncapped** — can exceed 1.0 and flip sign. `KINETIC_RES`(29) never read anywhere in the codebase | `crates/services/src/cell/combat/damage/pipeline.rs:62,167-179` | Units CONFIRMED (`alias.xml:211-213`); uncapped is a defect |
| (f) Focus | No Focus→accuracy term at all. Focus→Health exists as **overflow spillover**, not a probability: `(overflow×100/focusDmg)×focusDmg/300` added to health damage | `crates/services/src/cell/effects/scripts.rs:251-300` | RECONSTRUCTION; traces to an Atrea `.script` graph, the `/300` unsourced |
| (g) Status resist | Nothing | — | — |
| (h) AoE | Cone + radius target collection only. No blast exposure, no LoS occlusion, no cover interaction | `crates/services/src/cell/abilities/cone_aoe/geometry.rs` | Bands from `enumerations.xml:11-32` |
| (i) TechComp/quality | Nothing in combat. `tech_comp`/`quality_id` read only for vendor, visuals, Livewire difficulty | `crates/services/src/base/world_entry/methods/vendor/recharge.rs:238` | — |
| (j) Ammo types | `cur_ammo_type` tracked in the bandolier, never affects damage or penetration | `crates/services/src/cell/abilities/dispatch.rs:438-440` | — |
| (k) Stacking | **Ref-counted state flags present and correct** (`state_flag_counts`). No stack caps, no DR, no immunity timers — matching the client. Absorption pools drain properly, elemental-specific before generic | `crates/entity/src/cell_entity/state_flags.rs:41-98`; `crates/services/src/cell/combat/damage/pipeline.rs:126-159` | CONFIRMED absence of DR/caps |

Also live, and worth flagging:

- A temporary 2× player-damage multiplier at `crates/services/src/cell/abilities/damage_apply/mod.rs:137`.
- Damage type hard-coded to `DT_PHYSICAL` for both the health and focus components at
  `crates/services/src/cell/abilities/damage_apply/mod.rs:160,172`, so per-type Armor Factor
  never exercises the energy/hazmat/psionic branches in the auto-attack path.

---

## Pack ↔ existing, per block

| Block | Verdict | Note |
|---|---|---|
| (a) Accuracy vs Defense hit roll | **Conflict (roll shape), seam (modifiers)** | A binary hit-chance model cannot emit Glancing/Crit/DoubleCrit, which are CONFIRMED wire bands (`enumerations.xml:252-262`, string table `0x01e6ce00`, Kismet events 2002-2006). There is also a unit clash: original is 100 points = 1 QR (`alias.xml:204-205`); the pack's `/1000` makes 100 points = 10 percentage points. Keep QR as the roll; take the pack's accumulation structure as the QR delta |
| (b) Directional cover tiers + crouch | **Seam, numbers conflict** | Original cover is a **QR modifier**, not a Defense addend (`alias.xml:216` coverQRModifier "+1 attack and defend QR per point"; `:235` coverDefense −0.01 QR/pt). Pack has one tier axis; original has two (height × quality). The pack's tier table needs remapping before it can be seeded. Our arc resolution already exists and is the piece the pack asks for |
| (c) Cover penetration | **Seam, with rename** | "Cover Penetration" **is** `coverAccuracy` — ability 1450 "+100 Cover Penetration" → effect 1741 "+100 CoverAccuracy"; `EStats` has `coverAccuracy`(66) and `coverDefense`(67) and no `coverPenetration`. Adopt the `max(0, coverDefense − coverAccuracy)` shape, do not add a new stat |
| (d) Armor slot-weighted mitigation + penetration | **Seam, layered above us** | No slot weights exist in shipped data (`resources.items` has 23 columns, none a combat stat). But the pack solves a real problem: four matching 30% pieces must not sum to 120%. Put the 15/35/10/25/15 weights in "equipment → `MITIGATION` stat", upstream of `calculate_armor_factor`, rather than replacing it. Our flat subtraction vs the pack's multiplier must be reconciled; the multiplier caps more cleanly |
| (e) Explicit damage resistance | **Compatible** | The pack's `clamp(-50%, +60%)` fixes a live uncapped bug at `pipeline.rs:62`. Adopt the clamp, keep our stat sourcing, and wire `KINETIC_RES` while there |
| (f) Focus→accuracy and Focus→Health | **Conflict with shipped data** | The game-data ledger is explicit: nothing anywhere links Focus to accuracy or QR, and no Focus→Health bleed rule exists in shipped data. What the data does hold is **discrete gates at 25% and 50%** (effects 1410, 1608, 2577) plus 10:1 Focus:Health authoring in 7 of 8 original `effect_nvps` rows. The pack's continuous ramps (`-200 × (1−ratio)`, `10% + 80% × (1−ratio)`) contradict that shape. Seam, default off; keep the authored gates |
| (g) Status resist roll | **Seam, restructure first** | Original resist rolls are **co-sequenced QR rolls** sharing an `effect_sequence` step with the effect they gate (79 of 79 cases), typed kinetic/mental/health, carrying `EF_CalculateQRFromTarget` with `EF_DontUseQR` clear. Build the sequencing (confirmed), then the chance curve as config. Immunity is a moniker, not a timer |
| (h) AoE blast exposure / LoS | **Compatible as new** | We have nothing here. Caution: zero LoS rows exist across all 3,216 effects; LoS is a client-side precondition string only (`ErrorStrings.pak _39`). The pack's cone-uses-normal-rules split matches the distribution `TCM_AECone` 100 vs `AERadius` 300 |
| (i) TechComp / quality item scaling | **Compatible as new** | No base weapon damage exists in the data at all, and `clip_size`/`max_ranged_range` do not vary with `tech_comp` (duplicate weapon names at tc 1/3/5 are otherwise identical). Pure config; the `1 + (TC−1)×0.0125` curve is invention |
| (j) Ammo fallbacks (Hollow Point / AP) | **Compatible as new, one unit problem** | `+150 Penetration` under the confirmed `alias.xml:225` reading means −150% of the target's mitigation, which overshoots badly. Needs unit reconciliation before seeding. The pack is right to limit itself to two entries: the Hollow Point description text is copy-pasted verbatim onto EMP (1445), Explosive (1446) and five Dart abilities, all claiming Physical |
| (k) Stacking rules and caps | **Mostly compatible** | "One stance per stance family" matches confirmed data exactly — every stance ability opens with "Remove Effect of moniker EFFECT_Stance" (effects 3055, 3056). Implement via that moniker, not a hardcoded family rule. "Same flat moniker: strongest magnitude, duration refresh" is compatible with the confirmed `SecondaryId`-keyed refresh. All caps are invention; config them. We already have the ref-count model the pack does not mention |

---

## Existing RE evidence that outranks the pack

The pack's own source hierarchy puts raw client/runtime evidence above reconstruction. By that
rule the following repo material outranks every formula block in the pack.

1. **`docs/reverse-engineering/findings/combat-formulas-status.md`** — branch
   `docs/combat-formulas-status`, commit `84519817`. Headline verbatim: *"No — the original
   combat formulas are not in anything we have, and no shipped artifact can produce them."*
   What is recoverable is the unit system and vocabulary plus designer-authored per-ability
   numbers — enough to fix units and order of magnitude, **not** enough to reconstruct the
   combination rules (thresholds, curves, ordering, caps). It recommends treating combat as a
   design decision parameterized by verified units, with every constant beyond the units marked
   FAN-GUESS. **It never names the handoff pack and never mentions a 75%/1000 model** — that
   model is absent as a hypothesis rather than rebutted.

2. **`combat-formulas-game-data-evidence.md`** — commit `0820cf63`, 1557 lines. Supplies the
   `entities/defs/alias.xml` unit table, the `EF_DontUseQR` = bit 4 / value **16** finding, the
   cover-penetration linkage 1450 → 1741, the 79/79 co-sequenced resist rolls, the two-axis
   cover enums with 9,353 seeded nodes over 1,353 chunks, and the Hollow-Point copy-paste warning.

3. **`combat-formulas-client-evidence.md`** — commit `98030f09`. Confirms the QR stat channel
   and the `ETargetCollectionParams` radius/cone bands; note the two ledgers differ in emphasis
   on whether those bands are real metres or symbolic, and the status page follows the
   game-data reading (symbolic).

4. **`docs/reverse-engineering/findings/combat-damage-analysis.md:9-13`** — SGW.exe contains no
   damage, QR, armor or resistance math. The client applies the server's delta directly with no
   recalculation. Consequence: the pack's formulas can never be validated client-side, and the
   20-entry result-code table at `0x01e6ce00` plus the Kismet IDs are the only hard combat
   contract the client imposes.

5. **`docs/reverse-engineering/findings/cover-system.md`** — cover is server-driven;
   `CoverQRModifier` is a real QR-channel stat surfaced to the HUD, which is what makes the
   pack's "cover adds Defense" framing the wrong channel.

6. **`docs/reverse-engineering/findings/effect-execution-model.md`** — no DR, no immunity
   timers, no stack caps client-side. This independently corroborates the pack's stacking
   section *by absence*, which is the one place the pack and the evidence agree cleanly.

**Two divergences the ledgers name as fixable today against original data:**

- `crates/entity/src/abilities/defs.rs:58` — `EF_DONT_USE_QR = 32`; the original bit is **16**
  (32 is `EF_HasInductionBar`). The constant is also never read, so the QR bypass on 754
  effects is unhonoured and every "+200 Accuracy" buff is rolled as if it could miss.
- `EDamageType` in the same file uses 0-4 where original values are 13/14/15/16/18 (17 skipped),
  and `pipeline.rs:101` sends it as the wire `damage_code`. Both ledgers say verify against a
  client pcap before changing — pcap takes precedence.

---

## Does the server have cover-tier resolution?

Yes geometrically, no in combat.

`crates/services/src/cell/cover/` is a complete subsystem: `loader.rs`, `spatial.rs`,
`scoring.rs`, `reservation.rs`, `detection.rs`, `ai_integration.rs`, `types.rs`. It resolves
height, quality, directional arc, slot occupancy and NPC slot selection, and 9,353 nodes
across 1,353 chunks are seeded. A per-second cell tick
(`crates/services/src/cell/service/ticks/cover.rs`) runs the player proximity sweep.

- **#653 (C05, merged today 2026-09-18)** added a one-off `cover_sets`/`cover_nodes` entry at
  `chunk_id` 1381 carrying 7 hand-placed `SGWSpecCoverNode` world-space positions for the
  Cellblock med-station desk, wired to objective 2484 via `OnPlayerEnteredCover`. It resolved a
  `BlockedEvidence` gap by a UE3 level-extraction pass.
- **#671 (C06)** added the NPC flank trigger (`player_flanked_npc`), which is where the
  `orient ± π/2` arc logic is exercised.

All of it is content-engine triggers and NPC positioning. Nothing feeds QR, damage, or the
`COVER_*` stats. The "1,332 unimplemented Atrea cover nodes" framing is stale — the data is
loaded and queried; only the combat coupling is missing.

---

## Recommendation for Phase 3 config seams

Order matters. The first two are prerequisites, not options — without them every seam below is
inert.

1. **Raise the zero stat ceilings** in `crates/entity/src/stats/stat_list.rs` for `DEFENSE`,
   `QR_MOD`, `MITIGATION`, `COVER_QR_MODIFIER`, `COVER_ACCURACY`, `COVER_DEFENSE`,
   `CROUCHING_ACCURACY`, `CROUCHING_DEFENSE`. All currently have `max = 0`, so a buff writes
   nothing. Any cover, crouch, or accuracy/defense work is a no-op until this lands.
2. **Gate damage on `result_code`.** `RC_MISS` must produce a zero-delta result carrying the
   miss code, not 14% of base. This is a correctness fix independent of which formula wins, and
   it needs a regression guard that fails when reverted.
3. **One `config/combat.toml`**, no new crate. Sections, each defaulting to today's behaviour so
   the change is inert until an operator opts in:
   - `[qr]` — beta parameters and the five band thresholds.
   - `[cover]` — per-height × per-quality QR delta plus a separate crouch delta, expressed in
     **QR units, not percentage points**, so the confirmed 100-points-per-QR conversion holds.
   - `[armor]` — slot weights and the mitigation cap.
   - `[resistance]` — clamp bounds (adopt the pack's −50%/+60% immediately; it fixes a bug).
   - `[focus]` — the 0.25/0.50 gates as the primary mechanism, the pack's continuous ramps
     present but **off by default**.
   - `[status_resist]` — base chance and divisor, applied only after the co-sequencing lands.
   - `[aoe]` — the exposure ladder.
   - `[items]` — TechComp curve and quality multipliers.
   - `[ammo]` — per-type damage and penetration deltas, after the penetration unit is settled.
4. **Feed cover into QR, not Defense.** `cover/scoring.rs` already yields the arc result; add an
   attacker-direction lookup returning `(height, quality, in_arc)` and convert through the
   config table into a QR delta. A defender outside the arc gets no cover term, which is exactly
   the pack's requirement and is already computable.
5. **Sequence status resist rolls before curving them.** The co-sequencing is CONFIRMED; the
   chance formula is not. Building the curve first would bake a guess into the structure.
6. **Label every adopted constant in code.** The pack's own `SOURCE_POLICY.md` requires
   RECONSTRUCTION / INFERENCE labels in code comments and migrations. Applying that consistently
   is what keeps a future authentic formula a config change rather than an archaeology redo.

---

## Open questions

1. **`references/class_and_combat/SGW_Combat_Missing_Formulas.txt`, named in the audit brief, is
   not in the delivered pack.** `references/` contains only `source_extracts/` (11 files, none
   named for formulas). Either the pack is incomplete or the path in the brief is wrong. Worth
   resolving before anyone treats the pack as complete.
2. **Which original cover axis does the pack's None/Low/Medium/High mean?** Original cover is
   two axes: `ECoverHeight` Low 0 / Mid 1 / High 2 / LOS 3, and `ECoverQuality` Good 0 / Better 1
   / Best 2 / None 3. The pack's single tier cannot be seeded without an answer.
3. **Penetration units are a ten-fold apart.** The pack treats penetration as `rating/1000`
   yielding a mitigation *fraction*; `alias.xml:225` says 1 point = 1 percentage point off the
   target's final calculated mitigation. Our `pipeline.rs:57-59` follows the alias reading. The
   pack's `+150` AP value is unusable until this is settled.
4. **No combat chapter exists in `docs/spec/`** (only README, conventions, glossary, how-to-read,
   how-to-write) and none is drafted under `docs/drafts/spec/`. Any adoption decision should land
   as `spec.combat.damage-pipeline` first, or the pack's numbers will exist only in code comments
   with no contract.
5. **Crouch may never have been a separate channel.** `crouchingDefense` appears in no effect
   description at all, and the two tooltip conflicts the pack correctly refuses to reconcile
   (ability 1451 tooltip +200 vs effect 4565 +100; ability 1452 "+100 Crouching Defense" vs its
   only effect 1743 "+200 CoverDefense") both resolve toward cover, not crouch.
6. **Does the beta-distribution sign convention survive contact with real stats?** Positive QR
   yields a *lower* `qr_rand`, compensated by the `(1 + qr)` post-multiply at `pipeline.rs:63`.
   This is preserved from python on purpose, but it means the damage curve and the result-code
   band are driven in opposite directions by the same input. If the pack's accumulation is
   adopted as a QR delta, that interaction needs a numeric sanity pass at realistic stat spreads.

---

## Note on memory

I honoured the read-only constraint and wrote no agent-memory notes. Three durable divergences
are worth recording once writes are permitted: the zero stat ceilings on `DEFENSE`/`QR_MOD`/
`MITIGATION`/`COVER_*`, the missing miss gate in the damage-apply path, and
`EF_DONT_USE_QR = 32` where the original bit value is 16.
