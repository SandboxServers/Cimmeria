# Phase 0 Compatibility and Gap Report: SGW Claude Server Handoff Pack v1.2

**Status:** Phase 0 complete. No runtime, schema, or seed changes were made.
**Date:** 2026-09-18
**Baseline:** `origin/main` at `f1d7ebcb` (Castle packets CA00–CA10 merged).
**Pack:** [`pack/`](pack/) (generated 2026-09-19 by the external reconstruction effort; see [README.md](README.md) for provenance and what was included).
**Method:** five parallel read-only domain audits against the live tree. The unabridged audits are in [`audits/`](audits/) and are the evidence for every claim below.

This report answers the pack's first task verbatim: *inspect the existing server project, map its current character, ability, trainer, inventory, combat and world tables/classes to this handoff, do not implement changes, produce a compatibility/gap report and a proposed Phase 1 patch plan for trainer + learned abilities.*

Evidence labels follow the pack's `SOURCE_POLICY.md`: **CONFIRMED / SOURCE-BACKED**, **USER-CONFIRMED**, **RECONSTRUCTION / INFERENCE**, **PARTIAL / UNRESOLVED**. Every pack progression and combat constant is RECONSTRUCTION by the pack's own declaration. Every repo code fact cited here is CONFIRMED by direct file read at the cited line.

---

## 1. Headline verdicts

| Pack phase | Cimmeria today | Verdict |
|---|---|---|
| 1. Trainer + learned abilities | Trainer open, purchase handler, six server-side gates, atomic debit and persistence all exist and are unit-tested. Seed tree is a level-1, no-prerequisite, two-archetype stub. | **Adapt, not build.** Gap is data plus three gates the schema cannot express (branch points, per-node cost, trainer proximity). |
| 2. Ability runtime | Cooldown, range, ammo cost, effect dispatch are DB-driven. Warmup is animation-only. Weapon-family enforcement has zero code. | **~70% present.** |
| 3. Combat resolver | QR score sampled from a beta distribution into five CONFIRMED result bands. Cover subsystem complete but not coupled to the hit roll. | **Pack model conflicts on roll shape and units.** Land pack blocks as config seams on QR, never as a replacement. |
| 4. Weapons / items | Auto-attack binding, 4-slot bandolier, reload, ammo-type selection, DB clip sizes all live. | **3 of 4 done.** Missing: ammo-mode toggle effects, TechComp scaling. |
| 5. Character starts | Two start rows keyed on alignment: SGU → `SGC_W1`, Praxis → `Castle_CellBlock`. Space registry silently falls back to `Castle_CellBlock` for any unknown world. | **Conflict.** Free Jaffa and Asgard sit on the exact placeholder the pack forbids. Blocked on a registry fix and on content authoring, not on seed data. |
| 6. World / mission content | We hold the complete original mission tables (1,041 / 3,480 / 4,037 / 4,358 rows) the pack calls "missing server data". Region data covers three worlds; `spawn_points` and `paths` are empty. | **Pack's mission-table gap does not apply.** Region authoring is the long pole. |
| 7. NPC / loot | 159 templates, 176 spawn rows, 8 `mission_rewards` rows. | **Reward data is the hole.** |
| Castle CellBlock scope rule | All five QA regression items pass on `origin/main`. The pack's "Drones → Ambernol → Ring minigame → Armory" ordering is wrong (drone is an objective inside Find Ambernol). | **Compatible.** |

---

## 2. Provenance finding: the pack was built from this repo's seed

The pack calls its SQL extracts "legacy / fan-dev SQL" and ranks them as secondary evidence. They are this repo's own `db/resources/` seed, and the pack's larger data files were derived from the same source:

| Pack claim | Our seed | Result |
|---|---|---|
| 6,059 cooked items | `Items/Seed/items.sql` | exact |
| 1,886 abilities, 3,216 effects | `Abilities/Seed/abilities.sql`, `Effects/Seed/effects.sql` | exact |
| 1,278 weapon / related item IDs | `items.item_id` | 1,278 / 1,278 present |
| "Legacy clip size / ammo types / default ammo" | `items.clip_size`, `.ammo_types`, `.default_ammo_type` | 1,278 / 1,278 identical |
| "Legacy Event6 / Event7 ability IDs" | `items_event_sets` (event 6 = melee, event 7 = ranged) | 1,274 / 1,274 identical |
| 419 trainer-export ability IDs | `abilities.ability_id` | 419 / 419 resolve |
| 91 worlds, all IDs in `MASTER_SOURCE` §7 | `Worlds/Seed/worlds.sql` | 38 / 38 checked match |
| 28 stargates | `Worlds/Seed/stargates.sql` | 28 / 28 ids, names, worlds match |

Consequence for the source policy: a "match" between pack and seed confirms **provenance**, not **retail correctness**. Neither side independently validates the other. Where they disagree (section 5), the seed does not automatically win, and neither does the reconstruction; both must be preserved as a conflict.

One divergence worth recording: three stargate `address6` glyphs differ (Egypt 24, Ihpet Crater SGU 20, Men'fa SGU 22). In each case the raw export has a duplicate address tuple and our seed disambiguated by bumping the sixth glyph. No doc records this. It is a RECONSTRUCTION that needs a label.

---

## 3. Domain findings

### 3.1 Schema and persistence ([audit](audits/audit-schema.md))

Existing surface, all CONFIRMED:

| Concern | Where |
|---|---|
| Skill tree | `resources.archetype_ability_tree (archetype, ability_index, ability_id, tree_index, level, prerequisite_abilities int[])`, PK `(archetype, tree_index, ability_index)`, `CHECK tree_index BETWEEN 0 AND 2` |
| Per-character learned abilities | `sgw_player.abilities integer[]` (no source, no learned-at) |
| Skill points | `sgw_player.training_points integer` (unspent only) |
| Level cap | `sgw_player.level_sanity CHECK (level <= 20)`, `crates/game/src/player.rs:6` `MAX_LEVEL = 20` |
| Trainer offering | `trainer_ability_lists`, `trainer_abilities`, `entity_templates.trainer_ability_list_id`; one debug list, template 25 |
| Normalised precedent | `sgw_player_discipline_expertise` (the template to copy for a learned-ability table) |

Mapping of the pack's proposed tables:

| Pack table | Verdict |
|---|---|
| `character_skill_state` | **Adapt `sgw_player`.** Add `training_points_spent`, `progression_version`. A separate table would split the atomic debit across two rows. |
| `character_learned_ability` | **New table `sgw_player_ability`**, modelled on `sgw_player_discipline_expertise`. `learned_at_level`, `source`, `learned_at` cannot live in an `integer[]`. Keep the array populated in the same transaction during rollout. |
| `skill_tree_node` | **Adapt `archetype_ability_tree`.** PK already is `(archetype, branch, node_order)`. Add `required_branch_points`, `skill_point_cost`, `is_branch_root`, `is_capstone`, `evidence_status`, `project_version`, plus `UNIQUE (archetype, ability_id)`. |
| `skill_tree_node_prereq` | **Keep the array.** Read whole on every trainer open; never joined. House style (`disciplines.required_discipline_ids`). |

Data comparison, seed vs pack trainer export:

| Metric | Seed | Pack |
|---|---|---|
| Tree rows | 169 (Soldier 84, Commando 85, nothing else) | 439 nodes, 7 archetypes |
| Distinct `level` | `{1}` | 11 tiers, 1..50 |
| Rows with prerequisites | 0 | 418 primary + 22 additional |
| Nodes above level 20 | — | 243 of 439 |
| Points to buy everything | — | 439 (38 obtainable at cap 20) |
| Seed abilities absent from pack | 28 | — |
| Seed↔pack archetype disagreements | 20 (12 pack-Soldier ids are Commando in our seed) | — |
| Soldier branch boundary | tree1/tree2 split does not fall on pack's Heavy Weapons / Command split | — |
| Commando branch boundary | clean 1:1 | — |

Latent seed bug found incidentally: `ability_set_abilities_pkey` is `(ability_set_id)` only, so an NPC set can hold one ability.

### 3.2 Trainer and ability runtime ([audit](audits/audit-trainer-runtime.md))

The client sends a bare ability id (`SGWPlayer.def:635`, cell method 77 `trainAbility`). The wire carries **no** level, prerequisite, or branch data; `TrainerAbility` is `{INT32 abilityID, UINT8 trainable}` and `onAbilityTreeInfo` is three flat id lists. So every pack gate can be enforced server-side with zero client patching. The client cannot explain *why* a node is locked.

Purchase-validation coverage against the pack's nine steps:

| # | Requirement | Status |
|---|---|---|
| 1 | Archetype matches tree | exists, `vendor/train.rs:100` |
| 2 | Node exists and enabled | partial; no `enabled` column |
| 3 | Not already known | exists, both cell and DB (`NOT (abilities @> ARRAY[$1])`) |
| 4 | Level ≥ unlock level | code exists, data vacuous (all seed levels are 1) |
| 5 | Branch points | **missing** entirely |
| 6 | Prerequisites learned | code exists, data vacuous (all seed prereqs empty) |
| 7 | Points ≥ cost | partial; debit hard-coded to 1, `abilities.training_cost` never read |
| 8 | Trainer / faction access | **missing, and a server-authority hole**: a forged `trainAbility` trains from anywhere; `last_interaction_target` is not consulted |
| 9 | Atomic deduct + persist | exists, `progression/mod.rs:534-546` |

Defects in the existing path: nine rejection branches return silently (no `onErrorCode`); the client's point counter is never refreshed after a purchase; `mercury/world_data/stats.rs:45-88` hard-codes Soldier and Commando trees in Rust as a hand-copied duplicate of the seed; a second XP table `LEVEL_EXP` duplicates `LEVEL_XP`; `resetMyAbilities` (cell 72) is a stub while `onTrainerOpen` already advertises a respec cost.

Phase 2 status: weapon-family requirement has no code and no data (`abilities_final_v1.json` `Weapon Family` is empty on sampled rows); warmup fires an animation with no cast-time gate; moniker cooldown grouping is dead code because `moniker_ids` is never selected; 3,200 of 3,216 effects have `script_name = NULL`.

The client trainer layout is recorded as disabled in the `.toc` (`docs/client/ui-layout-inventory.md:129`). PARTIAL / UNRESOLVED until checked against a client install; if true, Phase 1 cannot be UAT'd in-game without a client-side `.toc` patch.

### 3.3 Combat ([audit](audits/audit-combat.md))

The repo already holds RE evidence that, by the pack's own hierarchy, outranks every pack formula: `docs/reverse-engineering/findings/combat-formulas-status.md` and the two evidence ledgers (all on `main` as of this baseline). Their conclusion is that the original combination rules are not recoverable from any shipped artifact; what is recoverable is the unit system. The pack's 75% + delta/1000 model is not rebutted there, it is simply absent as a hypothesis.

| Pack block | Verdict |
|---|---|
| Accuracy vs Defense | **Conflict.** Binary hit chance cannot emit Glancing / Crit / DoubleCrit, which are CONFIRMED wire bands. Unit clash: original is 100 points = 1 QR; pack makes 100 points = 10 percentage points. Keep QR as the roll; take the pack's accumulation structure as a QR delta. |
| Directional cover + crouch | **Seam, numbers conflict.** Original cover is a QR modifier, not a Defense addend. Original has two axes (height × quality); pack has one. Our arc resolution already exists. |
| Cover penetration | **Seam with rename.** It is `coverAccuracy` (ability 1450 → effect 1741). No new stat. |
| Armor slot weights | **Seam upstream of `calculate_armor_factor`.** Solves a real problem (four 30% pieces must not sum to 120%). |
| Damage resistance clamp | **Compatible.** Fixes a live uncapped bug. |
| Focus → accuracy / health | **Conflict with shipped data.** Data holds discrete gates at 25% and 50%, not continuous ramps. Seam, default off. |
| Status resist roll | **Seam, restructure first.** Original resist rolls are co-sequenced QR rolls (79 / 79 cases). Build sequencing before curving. |
| AoE exposure / LoS | Compatible as new. |
| TechComp / quality | Compatible as new, pure invention. |
| Ammo fallbacks | Compatible as new, but `+150 penetration` overshoots ten-fold under the confirmed unit reading. |
| Stacking / caps | Mostly compatible. Stance exclusivity is a confirmed moniker rule, not a family rule. |

Two prerequisite defects that make every seam inert until fixed: `DEFENSE`, `QR_MOD`, `MITIGATION`, and all `COVER_*` / `CROUCHING_*` stats have `max = 0` in `stat_list.rs`, so buffs write nothing; and `result_code` never gates damage, so a MISS still deals up to 14% of base. Also `EF_DONT_USE_QR = 32` where the original bit is 16, and the constant is never read.

### 3.4 Weapons, ammo, items ([audit](audits/audit-items.md))

| Phase 4 item | Verdict |
|---|---|
| Import weapon-family / auto-attack mappings | **Exists.** `items_event_sets` is the live table; loaded in `cell/spawner/abilities.rs:317`, consulted in `use_ability/weapon_redirect.rs`. Nothing to import. |
| 4-slot bandolier / active weapon | Exists, `crates/entity/src/cell_entity/bandolier.rs`. |
| Reload + ammo-type toggles | Partial. Reload and `requestAmmoChange` exist. The 17 ammo-mode toggle **abilities** (Hollow Point 715, AP 719, Incendiary 723, darts) have rows but no effect script; effect 747 has `script_name = NULL` in the raw seed. |
| TechComp scaling configurable | Missing; `tech_comp` is read only by vendor recharge, visuals, and Livewire. |

58 of 1,278 pack weapon records are tagged CONFLICT or NO LEGACY EVENT where the moniker-derived ability differs from our `items_event_sets` row (Dart Gun 0 / 25 match). This is pre-existing seed noise the pack surfaced correctly, not pack error. No wire propId errors found in the pack.

### 3.5 Worlds, starters, Castle scope ([audit](audits/audit-worlds.md))

| Profile | Pack target | Cimmeria today | Verdict |
|---|---|---|---|
| SGU Human | Earth SGC | `SGC_W1` (58), the tutorial instance, not hub `SGC` (86) | PARTIAL |
| Free Jaffa / Shol'va | Dakara | `SGC_W1` (58) | **CONFLICT** |
| Asgard | Pertho | `SGC_W1` (58) | **CONFLICT** |
| Praxis Human / Goa'uld / Loyalist Jaffa | unresolved | `Castle_CellBlock` (12) | n/a |

The blocker is `base/world_entry/space_registry.rs:27-37`, which knows three world names and silently falls back to `Castle_CellBlock`, and `respawn.rs:335-336`, which hard-codes the same coordinate. Repointing a chardef at Dakara or Pertho today would dump that character in the Praxis cellblock. Pertho has no point sets, no spawns, and no production UMAP in the pack's own map list.

Castle CellBlock: all five QA regression items pass on `origin/main`. Chain 1109 is the only `cross_world_teleport` out of world 12, gated on mission 688; every Castle-main chain gates on `player_loaded 'Castle'` or a Castle region; Copplemann and Zuritska have no CellBlock spawn rows. One metadata defect: chain 1008 declares `scope_id = 8` (Castle) while its trigger key is `Castle_CellBlock.Region8`; harmless because `scope_id` is never filtered on at runtime.

Progression routes (Tollana → …) have no schema column, no code, and no runtime gate anywhere. The nearest reusable primitive is `ring_transport_regions.required_mission_id` (all 30 rows NULL).

Harset mission 742, the pack's canonical worked example, has zero chains on either branch; the Harset campaign owns it.

---

## 4. Collisions and legacy assumptions (checklist Phase 0 deliverable)

1. **Level cap 20 vs pack level 50.** 243 of 439 nodes are unreachable. `LEVEL_XP: [u64; 21]` has a compile-time assert; a second table lives in `stats.rs:93`.
2. **Free Jaffa / Shol'va is one pack archetype but two enum values** (`ARCHETYPE_Sholva` = 7 SGU, `ARCHETYPE_Jaffa` = 8 Praxis). 51 nodes have no split rule.
3. **Free Jaffa has a fourth branch, Tau'ri (8 nodes).** `tree_index_sanity CHECK (0..2)` rejects it outright, and the pack's own QA test 1 says exactly three branches. One of the two is wrong.
4. **12 pack-Soldier ability ids are Commando in our seed**, plus 8 more cross-archetype disagreements. The seed partitions by id range; the pack does not (19 ids legitimately appear in two archetypes).
5. **28 seed tree abilities are absent from the pack.** Deleting them revokes abilities from live characters with no revocation path.
6. **`archetype` is `integer` in `sgw`, `EArchetype` enum in `resources`.** Joins need an explicit mapping. `Archaeologist` vs `Archeologist` spelling differs.
7. **Skill-point cost has two sources**: `abilities.training_cost` (varies, never read) vs pack's flat 1 (59 nodes flagged `VERIFY RAW COST 0`).
8. **Starter worlds** (section 3.5) and the pack's assumption that `char_creation_abilities` is not authoritative, while our `char_creation_choices` / `char_creation_visgroups` path grants starter items the pack did not audit.
9. **Migration policy.** The pack says keep migrations reversible; `db/README.md` says no migrations, edit the seed. Reconciled in section 6.4.
10. **Pack internal inconsistencies:** README says 439 nodes, `abilities_final_v1.json` says 419 (439 nodes, 419 distinct ids, both right); `combat_config_v1.json` is byte-identical to `SGW_Combat_System_Final_v1.json`; `manifest.json` and `manifest_v1.1.json` are stale.

---

## 5. Decisions required before Phase 1 starts

These are owner calls. Each has a recommended default so Phase 1 can proceed under a stated assumption if no ruling arrives.

| # | Decision | Recommended default |
|---|---|---|
| D1 | Level cap: stay at 20 for Phase 1, or raise to 50 with a reconstructed XP curve the pack does not supply? | Stay at 20. Import all nodes; those above 20 are inert and correctly rejected by the level gate. Cap raise is its own patch. |
| D2 | Skill-point economy: 439 points needed, 38 obtainable at 20, 98 at 50 with 2/level. | Leave `TRAINING_POINTS_PER_LEVEL = 2` for Phase 1 (RECONSTRUCTION on both sides). Revisit with D1. |
| D3 | Free Jaffa / Shol'va → which enum? | Load into `ARCHETYPE_Sholva` (7) only. Leave `ARCHETYPE_Jaffa` (8) untouched. |
| D4 | Tau'ri fourth branch vs three-branch constraint. | Defer the Tau'ri branch entirely; the pack itself flags it revision-heavy. Do not relax the CHECK. |
| D5 | Per-node cost source: `abilities.training_cost` or pack flat 1? | Pack flat 1 via the new `skill_point_cost` column; log a WARN when `training_cost = 0`. |
| D6 | Starter worlds: keep everyone in `SGC_W1` / `Castle_CellBlock` while the Castle and Harset campaigns run, or retarget now? | Keep. Phase 5 is blocked on the space-registry fallback and on Pertho / Dakara content anyway. |

---

## 6. Proposed Phase 1 patch plan: trainer + learned abilities

Scope per the pack: Soldier only, validated end to end, then the other six. Ordered so each step is independently reviewable. All changes additive; old binaries keep working against the new schema.

### 6.1 Files changed

| File | Change |
|---|---|
| `db/resources/Archetypes/Tables/archetype_ability_tree.sql` | Add `required_branch_points`, `skill_point_cost`, `is_branch_root`, `is_capstone`, `evidence_status` (default `LEGACY_SEED`), `project_version`; add `UNIQUE (archetype, ability_id)`. |
| `db/resources/Archetypes/Tables/archetype_branches.sql` (new) | `(archetype, tree_index, name, evidence_status)`; branch names have no home today. |
| `db/resources/Archetypes/Seed/archetype_ability_tree.sql` | Load the 72 Soldier export nodes stamped `RECONSTRUCTION` / `FINAL_V1`. Keep the 28 seed-only rows as `LEGACY_SEED`. Record the Heavy Weapons / Command boundary disagreement in a comment; do not pick a side. |
| `db/sgw/Players/Tables/sgw_player.sql` | Add `training_points_spent`, `progression_version`. |
| `db/sgw/Players/Tables/sgw_player_ability.sql` (new) | `(player_id, ability_id, learned_at_level, source, learned_at)`, FK in `_foreign_keys.sql` with `ON DELETE CASCADE`. |
| `db/sgw/Players/Tables/sgw_player_branch_points.sql` (new) | `(player_id, archetype, tree_index, points_spent)`. Must land before points are spent under branch rules; a flat counter cannot be reconstructed later. |
| `crates/services/src/cell/spawner/abilities.rs` | Select the new tree columns into `ArchetypeAbilityTreeEntry`. Also select `moniker_ids` and `training_cost` (revives dead cooldown grouping; enables the cost-0 WARN). |
| `crates/services/src/cell/cell_methods/player/vendor/train.rs` | Gate 7 branch points; gate 8 trainer proximity via `last_interaction_target` ∈ `template_trainer_lists`; pass `skill_point_cost` on `CellToBaseMsg::TrainAbility`; emit `onErrorCode` on every rejection. |
| `crates/services/src/base/world_entry/methods/progression/mod.rs` | Parameterise the debit by cost; write `sgw_player_ability` and `sgw_player_branch_points` in the same transaction as the array append. The `training_points >= $cost AND NOT (abilities @> …)` guards stay on the `UPDATE` itself. |
| `crates/services/src/cell/service/base_messages/ability_granted.rs` | Emit `onEntityProperty(propId 1, training_points_remaining)`. |
| `crates/services/src/mercury/world_data/stats.rs` | Delete the hard-coded Soldier / Commando arrays; build `AbilityTreeData` from `space_mgr.archetype_ability_trees`. Not optional. |
| `crates/services/src/cell/cell_methods/player/combat/mod.rs` | Minimal `resetMyAbilities`: zero the new counters, reset the array to `char_creation_abilities`, refund spent points. Gives the new counters a reset path. |

### 6.2 Schema changes

As listed above. Full DDL sketches with `DOWN:` header comments are in [audit-schema.md §6](audits/audit-schema.md).

### 6.3 Data migration

- Backfill `sgw_player_ability` from `unnest(sgw_player.abilities)` with `source = 'system'`, `learned_at_level = NULL`, `ON CONFLICT DO NOTHING`, in the setup path so it is idempotent across reloads.
- No rows deleted. No archetype moved.

### 6.4 Migration policy reconciliation

Edit `db/resources/` and `db/sgw/` in place per `db/README.md`. Reversibility is satisfied by the schema being fully reconstructible from the tree at any commit plus `git revert`; each new file carries its `DOWN` SQL in a header comment; the backfill is the one thing a reload cannot do and is guarded idempotent.

### 6.5 Assumptions (stated, per section 5)

Cap stays 20; 2 points per level; Shol'va only; Tau'ri deferred; flat cost 1; starter worlds unchanged; `ARCHETYPE_Any` / `_Jaffa` untouched.

### 6.6 Unresolved blockers

- Client trainer layout possibly disabled in the `.toc` (needs a live client check before UAT is scheduled).
- No client→server respec method exists; `CostToRespec` stays a placeholder.
- Weapon-family enforcement is Phase 2 and has no data carrier (`abilities.item_monikers` is the nearest candidate).

### 6.7 Tests (per TESTING.md type picker)

| Type | Test |
|---|---|
| Unit | Extend `mod handle_train_ability_tests` in `train.rs`: one rejection per new gate (branch points, non-trainer target, multi-cost insufficient points). Each must fail with its guard reverted. |
| Live-DB | Train transaction writes array, `sgw_player_ability`, `sgw_player_branch_points`, and counters atomically and rolls back as a unit; double-click still debits once across the transaction; cost-N floor (cost 3 at 2 points affects 0 rows); backfill idempotent across two `database.sql` loads. `require_db_or_skip!`, serialised, exact-sentinel cleanup, sentinels fit `i32`. |
| Wire-format | Byte-exact `onTrainerOpen` for a mixed trainable list (5 bytes per entry, trailing `CostToRespec`); byte-exact `onAbilityTreeInfo` built from DB rows, the guard that keeps the `stats.rs` deletion from regressing. |
| Seed guard | Every `trainer_abilities` row has a matching `archetype_ability_tree` row for its archetype (turns the runtime `trainer_offered_unbound` WARN into a build-time failure). |

### 6.8 QA mapping

Pack QA "Trainer / progression" items 1–7 and 10 become observable once `onErrorCode` lands. Item 8 (capstone at 50) is blocked on D1. Item 9 (cost-0 WARN) is satisfied by the `training_cost` select.

---

## 7. Defects found incidentally

Not caused by the pack; worth their own issues.

- Forged `trainAbility` trains from anywhere (server-authority hole).
- Nine silent trainer rejections; stale client point counter after purchase.
- Hard-coded duplicate trees and XP table in `stats.rs`.
- `ability_set_abilities` PK allows one ability per set.
- Combat: zero stat ceilings on `DEFENSE` / `QR_MOD` / `MITIGATION` / `COVER_*`; no miss gate; `EF_DONT_USE_QR` bit wrong and unread; uncapped resistance; `KINETIC_RES` never read; temporary 2× player damage; damage type hard-coded `DT_PHYSICAL`.
- Chain 1008 `scope_id` wrong (harmless today).
- 58 weapon rows with generic fallback bindings (Dart Gun 0 / 25).

## 8. Documentation drift

- `docs/project-status.md:140` says 29 stargates; there are 28.
- `docs/content/mission-chains.md:677` still calls 687 the CellBlock end; `:793` calls 702 a dead end. Packet CA16 owns this.
- `docs/gap-analysis.md:239` marks warmup implemented; it is animation-only.
- Stargate `address6` disambiguation has no recorded source label.
- The combat-formula evidence ledgers do not mention the pack's model; a `spec.combat.damage-pipeline` chapter should exist before any adoption decision.

## 9. What happens next

Per the pack's `IMPLEMENTATION_CHECKLIST.md`, Phase 1 follows this report once the six decisions in section 5 are ruled on or the defaults accepted. Section 10 breaks every phase into its steps with status and location. Nothing in this PR changes runtime behaviour.

---

## 10. Per-phase step checklist

Steps are the pack's `IMPLEMENTATION_CHECKLIST.md` items, split where one item is really several. Status key: **DONE** exists and is live at the cited location; **PARTIAL** code or data exists but does not fully satisfy the step; **MISSING** nothing exists; **BLOCKED** cannot proceed without a decision (D1–D6) or an upstream fix. "Where" is the file that does it today, or the file the Phase plan puts it in. Paths are repo-relative; `crates/services/src/` is abbreviated `svc/`.

### Phase 1: learned abilities + trainer

| Step | Status | Where / what remains |
|---|---|---|
| Import `trainer_server_export.json` | MISSING | Seed `db/resources/Archetypes/Seed/archetype_ability_tree.sql` has 169 rows (Soldier, Commando), all level 1, no prereqs. Needs the six new columns (§6.1) first. BLOCKED on D3, D4 for Free Jaffa. |
| Implement / preserve skill points | DONE | `sgw_player.training_points`; granted 2/level at `svc/base/world_entry/methods/progression/mod.rs:98`; pushed as propId 1 at `:333`. |
| Refresh the client's point counter after a purchase | MISSING | `svc/cell/service/base_messages/ability_granted.rs:34` receives the remaining count and never emits `onEntityProperty`. |
| Level validation | PARTIAL | Gate exists at `svc/cell/cell_methods/player/vendor/train.rs:123-136`; never fires because every seed level is 1. Data fix only. |
| Branch-point validation | MISSING | No column on the tree, no per-branch spend counter. Plan: `sgw_player_branch_points` + gate 7 in `train.rs`. |
| Prerequisite validation | PARTIAL | Gate exists at `train.rs:139-162`; never fires because every seed prereq is `{}`. Data fix only. |
| Per-node skill-point cost | PARTIAL | Debit hard-coded to 1 at `progression/mod.rs:537`; `abilities.training_cost` never read. Plan: parameterise, BLOCKED on D5. |
| Trainer proximity / access check | MISSING | `train.rs` never consults `last_interaction_target`. Server-authority hole. Plan: gate 8 mirroring `ability_granted.rs:70-73`. |
| Persist learned abilities | DONE | `sgw_player.abilities integer[]`, atomic `UPDATE` at `progression/mod.rs:534-546`. |
| Persist provenance (source, learned-at level, timestamp) | MISSING | Not representable in an `integer[]`. Plan: `sgw_player_ability` table modelled on `sgw_player_discipline_expertise`. |
| Expose unavailable abilities as unavailable | DONE | `onTrainerOpen` trainable byte, `svc/cell/interactions/trainer.rs`; re-sent after every grant. |
| Tell the client why a purchase was rejected | MISSING | Nine silent return paths in `train.rs` and `progression/mod.rs`. Plan: `onErrorCode` (client 121) on each. |
| Serve the ability tree to the client from the DB | PARTIAL | `onAbilityTreeInfo` works but is a hard-coded Rust copy of the seed at `svc/mercury/world_data/stats.rs:45-88`. Must be rebuilt from `space_mgr.archetype_ability_trees` before any import. |
| Respec | MISSING | `resetMyAbilities` (cell 72) is a stub at `svc/cell/cell_methods/player/combat/mod.rs:124`; `CostToRespec` is already advertised. |
| Cost-0 WARN (pack QA 9) | MISSING | Needs `training_cost` in the ability SELECT at `svc/cell/spawner/abilities.rs:89`. |
| Start with Soldier only, then load six more | BLOCKED | D1 (243 nodes above cap 20), D3, D4. |
| Run the trainer QA suite in-game | BLOCKED | `docs/client/ui-layout-inventory.md:129` records `Trainer.layout` disabled in the `.toc`. Verify against a client install. |

### Phase 2: ability runtime

| Step | Status | Where / what remains |
|---|---|---|
| Bind learned ability IDs to recovered ability data | DONE | `svc/cell/spawner/abilities.rs:89` loads `resources.abilities` into `ability_defs`. |
| Bind abilities to linked effects | PARTIAL | Effects loaded at `spawner/abilities.rs:360`; dispatch at `svc/cell/abilities/damage_apply/mod.rs:113-134` and `:499-523`; registry `svc/cell/effects/registry.rs` has 9 scripts. 3,200 of 3,216 effect rows have `script_name = NULL`, so almost everything resolves via the NVP damage path. |
| Weapon-granted abilities (auto-attack, melee) | DONE | Resolved at fire time from `items_event_sets` at `svc/cell/abilities/use_ability/handle.rs:144`. |
| Enforce "ability requires weapon family X" | MISSING | Zero code; zero data (pack `Weapon Family` field empty on sampled rows). Nearest carrier: `abilities.item_monikers`. |
| Cooldown | DONE | `use_ability/handle.rs:399-406` from `abilities.cooldown`. |
| Cooldown moniker grouping | PARTIAL | Logic exists in `crates/entity/src/abilities/manager.rs:276`; dead because `spawner/abilities.rs:112` hard-codes `moniker_ids: vec![]`. |
| Warmup as a cast-time gate | PARTIAL | `handle.rs:528-556` fires the `Ability_Begin` animation only; damage resolves in the same call. No pending-cast state, no interrupt. |
| Ammo / resource consumption | DONE | `handle.rs:369-397` from `abilities.required_ammo`; reload at `svc/cell/cell_methods/player/world/reload.rs:77`. |
| Range validation | DONE | `handle.rs:239-256`; the only ability path that sends `onErrorCode`. |
| Preserve tooltip↔effect conflicts in logs / config | MISSING | Nothing records the 127 conflicts the pack lists. |

### Phase 3: combat resolver

| Step | Status | Where / what remains |
|---|---|---|
| Raise zero stat ceilings (prerequisite) | MISSING | `crates/entity/src/stats/stat_list.rs:68,96-99`: `DEFENSE`, `QR_MOD`, `MITIGATION`, `COVER_*`, `CROUCHING_*` have `max = 0`. Every seam below is inert until this lands. |
| Gate damage on miss (prerequisite) | MISSING | `svc/cell/abilities/damage_apply/mod.rs:100,157,250`: `result_code` is forwarded but `calculate_damage` runs unconditionally. |
| Hit resolution | PARTIAL | QR score at `svc/cell/combat/damage/qr.rs:50-73`, beta sample `:21-30`, five CONFIRMED bands `:105-110`. Hard-coded. Pack's binary model conflicts; adopt its accumulation as a QR delta only. |
| Unresolved formulas as configuration | MISSING | All constants hard-coded. Plan: `config/combat.toml` with `[qr] [cover] [armor] [resistance] [focus] [status_resist] [aoe] [items] [ammo]`, each defaulting to today's behaviour. |
| Directional cover resolution | DONE | `svc/cell/cover/scoring.rs:19,90-97` (arc `orient ± π/2`, 5° hysteresis), `detection.rs:32`; 9,353 nodes seeded. |
| Cover feeds the hit roll | MISSING | `qr.rs:47` reads no cover term. Plan: attacker-direction lookup → config table → QR delta, in QR units (100 pts = 1 QR). |
| Crouch as a defensive state | PARTIAL | `BSF_CROUCHING` broadcast at `svc/cell/cell_methods/combatant.rs:41-43`; no combat effect. Pack's separate crouch channel is itself unsupported by shipped data. |
| Cover penetration | MISSING | Is `coverAccuracy` (stat 66), not a new stat. Plan: `max(0, coverDefense − coverAccuracy)`. |
| Armor slot weights + cap | MISSING | Current: per-type Armor Factor, flat subtraction, no cap, `combat/damage/pipeline.rs:57-59,181-189`. Plan: weights upstream in equipment → `MITIGATION`. |
| Resistance clamp | MISSING | `pipeline.rs:62,167-179` uncapped, can exceed 1.0; `KINETIC_RES` (29) never read. Adopt −50% / +60% now. |
| Focus → accuracy | MISSING | No term anywhere; shipped data has none either. Seam, default off. |
| Focus → Health exposure | PARTIAL | Overflow spillover at `svc/cell/effects/scripts.rs:251-300`, not a probability. Shipped data holds gates at 25% / 50% (effects 1410, 1608, 2577). |
| Status resist rolls | MISSING | Original is co-sequenced QR rolls (79/79). Build sequencing before any chance curve. |
| AoE blast exposure / LoS | MISSING | Cone + radius collection only at `svc/cell/abilities/cone_aoe/geometry.rs`. |
| Hit-decision logging | NOT VERIFIED | `result_code` reaches the client; whether a reproducible server-side hit log exists was not audited. |
| Fix `EF_DONT_USE_QR` | MISSING | `crates/entity/src/abilities/defs.rs:58` = 32; original bit is 16; constant never read. |

### Phase 4: weapons / items

| Step | Status | Where / what remains |
|---|---|---|
| Weapon-family / auto-attack mappings | DONE | `resources.items_event_sets` (event 6 melee, 7 ranged) loaded at `svc/cell/spawner/abilities.rs:317`, consulted in `svc/cell/abilities/use_ability/weapon_redirect.rs` and `abilities/resolve.rs`. Nothing to import; the pack's table is ours. |
| 4-slot bandolier / active weapon | DONE | `crates/entity/src/cell_entity/bandolier.rs`. |
| Reload | DONE | `svc/cell/cell_methods/player/world/reload.rs`, `use_ability/auto_reload.rs`. |
| Clip sizes from DB | DONE | `BandolierItem.clip_size` from `items.clip_size` at grant, `svc/base/world_entry/methods/inventory/grant/grant_item.rs`. |
| Ammo-type selection | DONE | `requestAmmoChange`, `svc/cell/cell_methods/inventory/bandolier/ammo_change.rs`, persisted item-id-keyed. |
| Ammo-mode toggle abilities (Hollow Point 715, AP 719, Incendiary 723, darts) | MISSING | Rows exist in `abilities.sql`; no effect script in `svc/cell/effects/registry.rs`; effect 747 has `script_name = NULL` in the raw seed. `cur_ammo_type` never affects damage (`abilities/dispatch.rs:438-440`). |
| TechComp / quality scaling | MISSING | `tech_comp` read only in `svc/base/world_entry/methods/vendor/recharge.rs:238`, visuals, Livewire. Pure config once Phase 3's `[items]` block exists. |
| Clean the 58 generic-fallback bindings | PARTIAL | Pre-existing seed noise in `items_event_sets` (Dart Gun 0/25). Per-row review. |

### Phase 5: character starts

| Step | Status | Where / what remains |
|---|---|---|
| Start SGU Human at Earth SGC | PARTIAL | `svc/base/chardef.rs:9-253` + `db/resources/Archetypes/Seed/char_creation.sql`: SGU → `SGC_W1` (58), the tutorial instance, not hub `SGC` (86). |
| Start Free Jaffa at Dakara | BLOCKED | Currently `SGC_W1`. `svc/base/world_entry/space_registry.rs:27-37` knows three worlds and falls back to `Castle_CellBlock` for anything else; `svc/cell/cell_methods/player/combat/respawn.rs:335-336` hard-codes the same coordinate. Dakara has no point sets or spawns. D6. |
| Start Asgard at Pertho | BLOCKED | Same registry blocker; Pertho has no point sets, no spawns, no production UMAP in the pack's own list. D6. |
| Make the registry fallback fail loudly | MISSING | Prerequisite for any start-row change. |
| Do not infer starter inventory | DONE (conflict of authority) | We already grant starter items via `char_creation_choices` / `char_creation_visgroups` and abilities via `char_creation_abilities`, `svc/base/character_create.rs:196-489`. The pack leaves these NULL and calls the ability rows non-authoritative. Preserve both positions. |
| Fix char-creation primary-colour persistence | NOT AUDITED | Open user-reported bug in pack `MASTER_SOURCE` §11; outside this audit's scope. |

### Phase 6: world / mission content

| Step | Status | Where / what remains |
|---|---|---|
| Use the world workbooks | DONE | Rendered under `pack/references/world_content/` (17 workbooks). |
| Mission / step / objective / task tables | DONE | `resources.missions` 1,041, `mission_steps` 3,480, `mission_objectives` 4,037, `mission_tasks` 4,358. The pack's "missing server data" claim does not apply here. |
| CellBlock strict scope ends at 688 | DONE | `db/resources/Content/Seed/castle_cellblock_chains.sql` chain 1109 is the only `cross_world_teleport`, gated on 688. Castle-main chains (`castle_701_chains.sql` etc.) gate on `player_loaded 'Castle'` or Castle regions. |
| Post-688 transition to Castle main | DONE | Chain 1109 → world 8 at (466.365, 70.397, 991.466); arrival caught by chain 1201 in `castle_701_chains.sql:139`. |
| Server-side regions (`point_sets` / `point_set_points`) | PARTIAL | 66 sets / 135 points covering Castle, Castle_CellBlock, SGC_W1 only. Long pole for every other world. |
| Persistent spawns | PARTIAL | `spawnlist` 176 rows, three worlds; `spawn_points` 0 rows; `paths` 0 rows. |
| Faction progression routes (Tollana → …) | MISSING | No column, no code, no gate. Nearest primitive: `ring_transport_regions.required_mission_id` (all NULL). |
| Harset mission 742 (pack's worked example) | MISSING | Row exists in `missions.sql`; no chain seed on any branch. Owned by the Harset campaign (`docs/analysis/harset-rebuild/`). |
| Fix chain 1008 `scope_id` | MISSING | Declares 8 (Castle) for a CellBlock trigger; harmless until tooling trusts `scope_id`. |
| Multi-objective step engine defects (#656, #657) | MISSING | Force hand-split chains in 639 and 688; will recur in every new multi-objective step. |

### Phase 7: NPC / enemy / loot

| Step | Status | Where / what remains |
|---|---|---|
| Spawn / archetype master mapping | PARTIAL | `entity_templates` 159, `spawnlist` 176, concentrated in three worlds. |
| Enemy combat kits from recovered ability IDs | PARTIAL | `ability_sets` / `ability_set_abilities` exist (3 sets). PK `(ability_set_id)` only at `db/resources/_primary_keys.sql:44` allows one ability per set. |
| Loot / rewards | MISSING | `mission_rewards` 8 rows against 1,041 missions. No pack QA reward test can pass. |
| No guessed retail stat scaling | PARTIAL | Nothing scales by level, which is correct. A temporary 2× player-damage multiplier lives at `svc/cell/abilities/damage_apply/mod.rs:137` and damage type is hard-coded `DT_PHYSICAL` at `:160,172`. |

### Phase 8: QA / regression

| Pack QA group | Status | Where / what remains |
|---|---|---|
| Trainer / progression 1–7, 10 | PARTIAL | Pass silently today (log-only). Observable once `onErrorCode` lands. Existing guard tests: `train.rs:202-411`. |
| Trainer / progression 8 (capstone at 50) | BLOCKED | D1. |
| Trainer / progression 9 (cost-0 WARN) | MISSING | Needs `training_cost` selected. |
| Weapon / ability 1–2 (weapon family) | MISSING | No enforcement. |
| Weapon / ability 3–6 | DONE | Auto-attack, ammo, reload, ammo-type toggle exist; mode-buff variant of 6 missing. |
| Cover 1–6 | PARTIAL | 1, 2, 5 satisfiable from `cell/cover/`; 3, 4, 6 need the Phase 3 coupling and config. |
| Focus / Health 1–2 | DONE | Distinct pools; effects damage each independently. |
| Focus / Health 3–4 | MISSING | No configured low-Focus behaviour; no conflict diagnostics. |
| Scientist Robotics, Goa'uld Servant Lord | NOT AUDITED | Pet / turret systems outside this audit. |
| World starts 1–4 | BLOCKED | Phase 5. |
| Castle CellBlock regression 1–5 | DONE | All five pass on `origin/main`; item 2's stated ordering is wrong but the content is present. |
| v1.2 combat formula tests (29) | BLOCKED | Meaningless until Phase 3 config seams exist; several would test invented constants. |
