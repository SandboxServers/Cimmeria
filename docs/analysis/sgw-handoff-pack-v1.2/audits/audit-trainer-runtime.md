# Phase 1/2 audit: handoff pack vs existing Cimmeria runtime

Read-only audit. No repo file was modified. Scope: pack Phase 1 (trainer +
learned abilities) and Phase 2 (ability runtime) mapped onto the existing
Rust runtime.

Evidence labels follow `pack/docs/SOURCE_POLICY.md`. Anything sourced from
the pack's `trainer_server_export.json` progression columns is
**RECONSTRUCTION / INFERENCE** (the pack's own `warning` field says so:
"unlock levels, branch-point gates, skill-point cost and prerequisite graph
are PROJECT FINAL v1 unless separately source-backed"). Repo-code findings
below are CONFIRMED by direct file read at the cited line numbers.

**Headline: Phase 1 is already built.** The trainer open, the purchase
handler, cell-side validation, atomic debit and persistence all exist and
are unit-tested. What is missing is not code, it is *data* — the seed tree
is a level-1, no-prereq, two-archetype stub — plus three gates the current
schema cannot express. Phase 2 is roughly 70% there, with weapon-family
enforcement absent entirely.

---

## 1. Current ability grant path

Three grant sources, all live in production code:

1. **Character creation.**
   `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\character_create.rs:352`
   selects `resources.char_creation_abilities` by `char_def_id` and writes
   the ids into `sgw_player.abilities` (an `integer[]`) as part of the
   create INSERT (`character_create.rs:390-397`). Seed file:
   `C:\Users\Steve\source\projects\Cimmeria\db\resources\Archetypes\Seed\char_creation_abilities.sql`
   (168 lines).
   Pack cross-reference: `KNOWN_UNKNOWNS.md` line 33 explicitly says
   "`char_creation_abilities` legacy rows are not authoritative enough to
   define canonical starts" — so the repo's starter grants are
   **PARTIAL / UNRESOLVED** against the pack, not confirmed.

2. **Trainer purchase.** Full round trip, cell → base → cell:
   - Client sends cell method 77 `trainAbility(INT32 AbilityID)`.
   - Dispatch: `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\cell_methods\player\vendor\mod.rs:28`
     (decodes 4 bytes, warns on truncation).
   - Validation: `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\cell_methods\player\vendor\train.rs:31`
     — six guards, detailed in section 5.
   - On pass, sends `CellToBaseMsg::TrainAbility` (`train.rs:176-182`).
   - Base handler: `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\base\world_entry\methods\progression\mod.rs:478`
     — in-memory fast-path check at `:516`, then the atomic debit at
     `:534-546`.
   - Reply `BaseToCellMsg::AbilityGranted` → cell mirror at
     `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\service\base_messages\ability_granted.rs:14`,
     which adds to `entity.abilities` (`:24`), re-sends
     `onKnownAbilitiesUpdate` (`:34`), and re-fires `onTrainerOpen` to
     refresh the trainable flags (`:85`).

3. **Weapon-granted abilities.** Not stored in the known set at all;
   resolved at fire time from `resources.items_event_sets`. The fallback
   lives at
   `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\abilities\use_ability\handle.rs:144`
   (`is_ability_granted_by_active_weapon`). Loader:
   `C:\Users\Steve\source\projects\Cimmeria\crates\services\src\cell\spawner\abilities.rs:317`.

### What the client sends and what it gets back

The client sends a bare **ability id** — not a node index, not a tree
index. Definition: `C:\Users\Steve\source\projects\Cimmeria\entities\defs\SGWPlayer.def:635-638`:

```xml
<trainAbility>
    <Exposed/>
    <Arg>   INT32       <ArgName>AbilityID</ArgName></Arg>
</trainAbility>
```

The server replies **nothing on rejection**. Every failure branch in
`vendor/train.rs` (lines 44, 72, 85, 119, 135, 161) and in
`handle_train_ability` (`progression/mod.rs:493, 501, 524, 557, 567`)
returns after a `tracing` call. No `onErrorCode` (client method 121), no
toast, no feedback. Pack QA test "Trainer / progression #6 — buying the
same node twice is rejected server-side" passes, but silently; the player
sees a dead button.

---

## 2. Client protocol surface

| Method | Index | Direction | Handler path | Status |
|---|---|---|---|---|
| `trainAbility` | cell 77 | C→S | `crates\services\src\cell\cell_methods\player\vendor\train.rs:31` | Implemented; no error reply on any rejection |
| `onTrainerOpen` | client 113 | S→C | `crates\services\src\cell\interactions\trainer.rs` (`try_open_trainer`) | Implemented; wire noted as pcap+python verified in the module doc |
| `onKnownAbilitiesUpdate` | client 101 | S→C | `crates\services\src\mercury\world_data\map_loaded.rs:275-289`; `crates\services\src\cell\service\base_messages\player_init.rs` (`send_known_abilities_update`) | Implemented |
| `onAbilityTreeInfo` | client 141 | S→C | `crates\services\src\mercury\world_data\map_loaded.rs:268-273`, data from `crates\services\src\mercury\world_data\stats.rs:45` | Implemented but **hard-coded in Rust**, see below |
| `onEntityProperty(propId 1 = TrainingPoints)` | client 7 | S→C | `crates\services\src\mercury\world_data\map_loaded.rs:325-337`; `crates\services\src\base\world_entry\methods\progression\mod.rs:333-341` | Partial — sent at world entry and on level-up, **never after a purchase** |
| `giveAbility` | client 118 | S→C | constant only: `crates\services\src\mercury\mod.rs:250`, `crates\services\src\cell\client_methods\player.rs:44` | Defined, never emitted anywhere |
| `resetMyAbilities` | cell 72 | C→S | `crates\services\src\cell\cell_methods\player\combat\mod.rs:124` | Stub — logs `UNIMPLEMENTED: resetMyAbilities` |
| ability respec | — | — | — | **No method exists.** `onTrainerOpen` ships a `CostToRespec` of 1000 naquadah (`interactions\trainer.rs:41`, `DEFAULT_RESPEC_COST`) with nothing to spend it on. Only `respecCrafting` (cell 100) and the GM-side `gmRespec` (155) exist |

Protocol doc rows confirming indices:
`docs/protocol/client-method-dispatch-table.md:248` (101),
`:260` (113), `:265` (118), `:268` (121);
`docs/protocol/cell-method-dispatch-table.md:285` (77 `trainAbility`,
`YES` exposed, `INT32 abilityId`, def line 635), `:286`
(`giveTrainingPoints`, not exposed), `:341` (100 `respecCrafting`).

### The `onAbilityTreeInfo` duplication trap

`C:\Users\Steve\source\projects\Cimmeria\crates\services\src\mercury\world_data\stats.rs:45-90`
returns a **literal Rust array** of ability ids for Soldier (3 trees of
29/28/27) and Commando (28/30/27), and `AbilityTreeData::default()` (empty)
for the other seven archetypes. Its own doc comment at `:42` says it was
transcribed "from
`db/resources/Archetypes/Seed/archetype_ability_tree.sql`" — i.e. it is a
hand-copied duplicate of the DB seed, with no test tying the two together.
Importing the pack tree without rewriting this function means the client's
tree window and the server's purchase validation will disagree.

### Trainer discovery

A trainer is any NPC whose `entity_templates.trainer_ability_list_id` is
non-NULL. Loader:
`crates\services\src\cell\spawner\abilities.rs:270` (`load_template_trainer_lists`).
Offered list is keyed `(list_id, archetype_id)` —
`crates\services\src\cell\spawner\abilities.rs:226` (`load_trainer_abilities`).
Today exactly one trainer template exists: template 25 ("Interaction Debug
NPC"), `list_id = 1`. All 169 rows of
`db/resources/Abilities/Seed/trainer_abilities.sql` are on `list_id = 1`.

`try_open_trainer` computes a per-ability `trainable` byte from the
player's level, known set and the tree's prereqs, and emits the wire shape
`INT32 TrainerID, UINT32 count, [N x (INT32 abilityID + UINT8 trainable)],
INT32 CostToRespec` (`interactions\trainer.rs`, serialisation block at the
5-byte-per-entry loop). Instrumented with a `trainer_opens_total` counter
split by outcome (`entity_no_template`, `not_a_trainer`, `player_missing`,
`no_archetype`, `empty_offering`, `opened`).

---

## 3. Skill points

The counter exists end to end.

- **Column:** `sgw_player.training_points` (integer).
- **Client property:** `trainingPoints INT32 CELL_PRIVATE` at
  `C:\Users\Steve\source\projects\Cimmeria\entities\defs\SGWPlayer.def:67-71`.
  Also present: `numRespecAbility` / `numRespecCrafting` at `:429-438`
  (both unused server-side).
- **Grant on level-up:** `crates\game\src\player.rs:17`
  `TRAINING_POINTS_PER_LEVEL = 2`, applied in the level loop at
  `crates\services\src\base\world_entry\methods\progression\mod.rs:98`,
  persisted at `:115-124`, pushed as `onEntityProperty` propId 1 at
  `:333-341`. The pack's "exact original skill-point gain cadence" is
  listed as a known unknown (`KNOWN_UNKNOWNS.md:19`), so 2/level is
  **RECONSTRUCTION** on our side too — it is not a conflict with the pack,
  just an unverified value on both sides.
- **World-entry push:** `crates\services\src\mercury\world_data\map_loaded.rs:325-337`
  sends propId 2 (AppliedSciencePoints), **1 (TrainingPoints)**, 7
  (AccessLevel), 8 (Gender), 4 (PvPFlag), 3 (AmmoTypeId).
- **Load from DB:** `crates\services\src\base\world_entry\methods\player_load\core\player_data.rs:47`
  and `:204`.
- **In-memory cache:** `ConnectedClientState.player_training_points` at
  `crates\services\src\base\mod.rs:218`.

Two defects:

**(a) Every ability costs exactly 1 point, hard-coded.** The debit is
literally `training_points = training_points - 1`
(`progression/mod.rs:537`). The `abilities.training_cost` column exists
(`db/resources/Abilities/Tables/abilities.sql`) and is never selected or
read anywhere in `crates/`. This happens to match pack v1 by luck: all 439
pack nodes carry `skill_point_cost_project_v1 = 1` (verified by counting
the export). But 59 of those 439 nodes have `raw_training_cost = 0` and
`purchase_status = "VERIFY RAW COST 0"`, which is exactly pack QA test #9
("`Training Cost=0` nodes still show a server log warning/metadata flag
when purchased") — there is no such warning today because the column is
never read.

**(b) The client's point counter goes stale after a purchase.**
`ability_granted.rs:17` receives `training_points_remaining`, logs it at
`:31`, and **never emits `onEntityProperty`**. A full grep of
`ON_ENTITY_PROPERTY` emit sites in `crates/services/src` returns only:
`progression/mod.rs:337` (level-up), three bandolier/ammo sites,
`mercury/aoi/create.rs:170`, and `map_loaded.rs:336`. Nothing on the train
path. So the displayed point total is wrong from the moment of purchase
until the next level-up or relog. This is the cheapest high-value fix in
the audit.

No `spent_skill_points`, no `learned_at_level`, no `source` field.
`sgw_player.abilities` is a flat `integer[]` appended with
`abilities || $1::integer` — there is nowhere to record the pack's
`character_learned_ability.source` (`starter` / `trainer` / `mission` /
`system`) without a new table.

---

## 4. Ability runtime status (Phase 2)

| Mechanic | Where implemented | Data-driven? | Status |
|---|---|---|---|
| Cooldown | `crates\services\src\cell\abilities\use_ability\handle.rs:399-406`; per-moniker grouping in `crates\entity\src\abilities\manager.rs:276` and `:310` | From `abilities.cooldown` (real) | Works per-ability. **Moniker grouping is dead code**: `crates\services\src\cell\spawner\abilities.rs:89` omits `moniker_ids` from the SELECT and `:112` hard-codes `moniker_ids: vec![]`, so the grouping loop never iterates. The column exists (`bigint[]`), and an `ability_moniker_groups` table exists in `db/resources/Abilities/Tables/` |
| Warmup | `crates\services\src\cell\abilities\use_ability\handle.rs:528-556` | From `abilities.warmup` | **Animation only.** `warmup > 0` fires the `Ability_Begin` sequence (event 1000); damage resolution runs in the same call at `:618` with no delay. There is no cast-time gate, no interrupt window, no pending-cast state. Note `docs/gap-analysis.md:239` claims "Ability warmup — IM (implemented)"; that row overstates what the code does |
| Ammo / resource cost | `handle.rs:369-397` (required-ammo check, reload-in-flight gate at `:378`), reload at `crates\services\src\cell\cell_methods\player\world\reload.rs:77` | From `abilities.required_ammo` | Implemented, read through bandolier helpers. `required_ammo > 0` also serves as the de-facto "is a weapon attack" predicate (`handle.rs:294`) |
| **Weapon-family requirement** | — | — | **Absent.** A case-insensitive grep for `weapon_family` / `weaponfamily` / `required_weapon` across `crates/` returns **zero hits**. The nearest schema hook is the unused `abilities.item_monikers bigint[]` column. The pack's `abilities_final_v1.json` carries a `Weapon Family` field (empty on the sampled records). Pack QA "Weapon / ability" tests 1 and 2 cannot pass |
| Linked-effect dispatch | NVP damage at `crates\services\src\cell\abilities\damage_apply\mod.rs:113-134`; script dispatch at `:499-523`; registry at `crates\services\src\cell\effects\registry.rs` | From `abilities.effect_ids` → `resources.effects` + `resources.effect_nvps` | Data-driven in shape. Registry has 9 named scripts (`HealHealth`, `HealFocus`, `MeleeDamage`, `MeleePhysicalDamage`, `AbsorbShield`, `Stun`, `Suppression`, `RangedPhysicalDamage`, `RangedEnergyDamage`). **But 3,200 of 3,216 seeded `effects` rows have `script_name = NULL`** — only 14 rows name a script at all. Nearly every ability therefore resolves through the `HealthDamage` / `FocusDamage` NVP path, not a script |
| Range | `handle.rs:239-256`, default 30.0 when `max_range == 0`; out-of-range emits `onErrorCode(0, ability_id, 42)` at `:262-274` | From `abilities.min_range` / `max_range` | Implemented. Notably this is the **only** ability-path failure that sends the client an error code |
| Target validity | `handle.rs:223-237` — player attackers may only single-target a hostile NPC | Hard-coded faction sentinel | Implemented (#444). TODO at `:216` notes supportive/friendly-target abilities need the inverse gate and that `AbilityDef` has no offensive/supportive flag |
| Damage scaling | `damage_apply/mod.rs:135-141` | No | `health_base_damage * 2` for player attackers, comment reads "Temp: 2x player damage so players can kill NPCs before dying". Unknown-ability fallback is a flat 15 HP (`:133`) |

Effect loading (`crates\services\src\cell\spawner\abilities.rs:360`) has a
noteworthy hazard already documented in-comment at `:363-372`: the
`target_collection_method` PG enum must be cast to TEXT or the entire
`fetch_all` fails and `effect_defs` silently ends up **empty for the
process lifetime**, disabling every effect-resolved ability behind a single
startup WARN.

---

## 5. Purchase-validation coverage (9 pack steps)

Pack source: `pack/docs/SERVER_SCHEMA_PROPOSAL.md` "Purchase validation".

| # | Pack requirement | Status | Path / evidence |
|---|---|---|---|
| 1 | Character archetype matches the tree | **Exists** | `cell\cell_methods\player\vendor\train.rs:100-120` — looks up `archetype_ability_trees[archetype_id]`, rejects on miss with `event = "train_rejected", reason = "not_in_archetype_tree"` |
| 2 | Node exists and is enabled | **Partial** | Id existence: `train.rs:38` (`ability_defs.contains_key`). Tree membership: `:100`. There is **no `enabled` / `is_active` column** on `archetype_ability_tree`, so "enabled" is unrepresentable |
| 3 | Character does not already know the ability | **Exists**, both layers | Cell: `train.rs:77` (silent no-op on duplicate). Base: `progression/mod.rs:540` — `AND NOT (abilities @> ARRAY[$1::integer])` in the same UPDATE, so a double-click cannot double-debit |
| 4 | Character level >= unlock level | **Code exists, data vacuous** | `train.rs:123-136`. But **all 169 seed rows have `level = 1`** (verified by extracting the level column from the seed), so the gate never fires today |
| 5 | Required branch points satisfied | **Missing** | No column on `archetype_ability_tree`, no counter on the entity or in `sgw_player`, no check anywhere. Pack values span 0,2,4,...,20 |
| 6 | Primary and additional prerequisite abilities learned | **Partial** | `train.rs:139-162` iterates `tree_entry.prerequisite_abilities` and rejects on the first missing one. But **all 169 seed rows are `'{}'`**, so the gate never fires. The schema also has no primary-vs-additional distinction (pack splits `primary_prereq_ability_id` from `additional_prereq_ability_ids`) |
| 7 | Skill points >= cost | **Partial** | In-memory fast path `progression/mod.rs:504-525`; authoritative DB gate `AND training_points > 0` at `:539`. Always debits exactly 1; no per-node cost |
| 8 | Any faction / trainer access condition passes | **Missing — and it is a server-authority hole** | `handle_train_ability` never checks that the player is interacting with a trainer. `last_interaction_target` exists on the entity (it is read in `ability_granted.rs:68` for the trainer resend) but is **not** consulted in `train.rs`. A forged `trainAbility` packet trains from anywhere in the world, with no NPC visit, no proximity check, no faction check |
| 9 | Transaction deducts points and persists atomically | **Exists** | `progression/mod.rs:534-546` — one `UPDATE ... RETURNING training_points` guarded by `training_points > 0 AND NOT (abilities @> ARRAY[$1])`. `Ok(None)` (0 rows) is treated as a rejection at `:550-558`. The cell mirror only happens after the DB confirms |

Related: existing cell-side guard tests live in
`vendor\train.rs:202-411` (`mod handle_train_ability_tests`) — six
rejection cases plus one happy path, all asserting on whether
`CellToBaseMsg::TrainAbility` was emitted.

---

## 6. Blockers

### B1 — ID-assignment collision (CONFIRMED; blocks a naive import)

The repo seed partitions abilities by id range: Soldier 597-689 (84 rows),
Commando 700-792 (85 rows). The pack does not partition. Measured against
`trainer_server_export.json`:

| Measurement | Count |
|---|---|
| Pack nodes total | 439 |
| Pack unique ability ids | 419 |
| Pack archetypes | 7 (Soldier 72, Asgard 66, Archaeologist 65, Commando 64, Goa'uld 62, Scientist 59, Free Jaffa/Shol'va 51) |
| Pack Soldier ids also in repo's **Soldier** tree | 55 |
| Pack Soldier ids the repo assigns to **Commando** | 12 |
| Pack Soldier ids in neither repo tree | 16 |
| Pack Commando ids in repo's Commando tree | 61 |
| Pack ability ids absent from the repo tree entirely | 297 of 419 |

The pack is explicit that one ability can belong to two archetypes —
`abilities_final_v1.json` record 523 (Concussive Grenade) carries a
`Final v1 Progression` array with both `Soldier / Command` and
`Commando / Demolitions` entries. `archetype_ability_tree` supports that
(archetype is part of the key), but `world_data\stats.rs:45` hard-codes a
strict partition. Checklist Phase 0's "report collisions and legacy
fan-server assumptions" has a concrete finding here.

### B2 — Level cap 20 vs pack level 50 (CONFIRMED)

`crates\game\src\player.rs:6` sets `MAX_LEVEL: u32 = 20`.
`crates\services\src\base\world_entry\methods\progression\mod.rs:18-28`
defines `LEVEL_XP: [u64; 21]` with a **compile-time assert** tying its
length to `MAX_LEVEL + 1`. A parallel hard-coded `LEVEL_EXP: [i32; 21]`
lives at `crates\services\src\mercury\world_data\stats.rs:93`.

Pack unlock levels are `1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50`, with 21
capstone nodes. Roughly half the pack tree is unreachable until the cap and
both XP tables are extended. Pack QA "Trainer / progression #8" (capstone
requires Level 50) cannot pass today. Note the two XP tables are
independent duplicates — extending one without the other desyncs the
client's max-XP bar from the server's level-up threshold.

### B3 — Client trainer UI may be disabled (PARTIAL / UNRESOLVED)

`docs/client/ui-layout-inventory.md:129` records
`Trainer/Trainer.layout` as **DISABLED — disabled in .toc**, and `:316`
adds "Functionality may have moved to DisciplineTrainer". If that record is
accurate, `onTrainerOpen` reaches a client with no registered handler and
Phase 1 is unverifiable in-game without a client-side `.toc` patch. The
same doc carries a self-warning that its counts are approximate, so this
needs a live check against a client install before Phase 1 work is
scheduled. `Ability/Ability.layout` (the tree/hotbar window) is ACTIVE, so
`onAbilityTreeInfo` and `onKnownAbilitiesUpdate` do have a consumer either
way.

### B4 — What the client can and cannot display

Good news for the pack's gating model. The wire carries **no** level,
prereq or branch-point data:

- `TrainerAbility` is `FIXED_DICT { INT32 abilityID, UINT8 trainable }` —
  `C:\Users\Steve\source\projects\Cimmeria\entities\defs\alias.xml:417-422`.
- `onTrainerOpen` is `INT32 TrainerID, ARRAY<TrainerAbility> Abilities,
  INT32 CostToRespec` — `entities\defs\SGWPlayer.def:1194-1198`.
- `onAbilityTreeInfo` is `ARRAY<ARRAY<INT32>>` — three flat id lists,
  nothing else.

So gray-versus-active is **entirely** the server's `trainable` byte. The
pack's branch / level / prereq gating can therefore be enforced
server-side with **zero client patching**. The only thing the client cannot
do is explain *why* a node is locked — any per-reason messaging would need
a separate `onErrorCode` or chat line, which is also the fix for the silent
rejections in section 1.

### B5 — Silent rejections (CONFIRMED)

Nine distinct rejection paths across `train.rs` and `progression/mod.rs`
return without telling the client anything. The player clicks Train and
nothing happens. Contrast the out-of-range path in `handle.rs:262-274`,
which does send `onErrorCode`. Cheap to fix, and it makes every pack QA
trainer test observable instead of log-only.

---

## 7. Proposed Phase 1 patch plan (NOT applied)

Ordered so each step is independently reviewable. Repo policy: edit seeds
in `db/resources/` directly, never write `db/scripts/*.sql` migrations.

### Schema and seed

1. `db\resources\Archetypes\Tables\archetype_ability_tree.sql` — add
   `required_branch_points integer NOT NULL DEFAULT 0`,
   `skill_point_cost integer NOT NULL DEFAULT 1`,
   `evidence_status text`, `project_version text`. Keep
   `prerequisite_abilities integer[]` as the single prereq carrier: fold
   the pack's `primary_prereq_ability_id` and
   `additional_prereq_ability_ids` into it. Add `primary_prereq_ability_id`
   separately **only** if a future trainer UI needs to distinguish them —
   the wire cannot express it today (B4).
2. `db\resources\Archetypes\Seed\archetype_ability_tree.sql` — regenerate
   from `trainer_server_export.json`, **Soldier only first**, per
   `IMPLEMENTATION_CHECKLIST.md` Phase 1 ("Start with Soldier only... Load
   the remaining six archetypes after Soldier passes"). Resolve the 12-id
   Soldier/Commando conflict from B1 explicitly and record the decision.
   Every level / branch-point / prereq value must be labelled
   **RECONSTRUCTION / INFERENCE** per `SOURCE_POLICY.md`; only the
   branch membership is SOURCE-BACKED.
3. `db\resources\Abilities\Seed\trainer_abilities.sql` — currently one
   debug list (`list_id = 1`, template 25). Real trainer NPCs and per-list
   offerings are Phase 6/7 content, not Phase 1. Phase 1 can be QA'd
   entirely through the debug trainer.

### Code

4. `crates\services\src\cell\spawner\abilities.rs:141-214` — extend
   `ArchetypeAbilityTreeEntry` and the SELECT at `:179` with the new
   columns. Separately, add `moniker_ids` to the ability SELECT at `:89`
   and stop hard-coding `vec![]` at `:112`, which revives the existing
   moniker-group cooldown logic in `crates\entity\src\abilities\manager.rs:276`.
5. `crates\services\src\cell\cell_methods\player\vendor\train.rs` — add:
   - **Step 7, branch points.** Derive the player's spent points in a
     branch by counting known abilities whose tree entry shares the
     requested node's `tree_index`, then compare against
     `required_branch_points`. No new persisted counter needed — it is a
     pure function of the known set plus the tree, which also makes it
     self-healing after a GM grant.
   - **Step 8, trainer access.** Require `last_interaction_target` to
     resolve to a template present in `space_mgr.template_trainer_lists`,
     mirroring the check already written in `ability_granted.rs:70-73`.
     Closes the forged-packet hole.
   - Pass `skill_point_cost` through on `CellToBaseMsg::TrainAbility`
     (new field).
   - Emit `onErrorCode` on each rejection (fixes B5).
6. `crates\services\src\base\world_entry\methods\progression\mod.rs:534` —
   parameterise the debit:
   `SET ... training_points = training_points - $3 WHERE ... AND training_points >= $3`.
   Keep the `NOT (abilities @> ARRAY[...])` clause — it is what makes the
   double-click safe.
7. `crates\services\src\cell\service\base_messages\ability_granted.rs:34` —
   emit `onEntityProperty(propId 1, training_points_remaining)` alongside
   the known-abilities update. Fixes the stale counter from section 3(b).
8. `crates\services\src\mercury\world_data\stats.rs:45` — **delete the
   hard-coded arrays** and build `AbilityTreeData` from
   `space_mgr.archetype_ability_trees`. Not optional: leaving it means the
   client's tree window and the server's validation diverge the moment the
   pack tree lands (B1). This requires threading the loaded trees into the
   `mapLoaded` builder, which today takes only `archetype_id`.
9. Optional, cheap, satisfies pack QA #9: when `abilities.training_cost`
   is 0 on a purchased node, log a WARN with the node's `evidence_status`.
   Requires adding `training_cost` to the ability SELECT at
   `spawner\abilities.rs:89`.

### Tests required (per TESTING.md type picker)

- **Unit** (type 1), extending the existing
  `mod handle_train_ability_tests` in `vendor\train.rs:202`: one rejection
  test per new gate — insufficient branch points, interaction target is not
  a trainer, insufficient points for a multi-cost node. Each must fail when
  its guard is reverted, or it is a happy-path test, not a regression
  guard.
- **Live-DB** (type 3), in `progression\mod.rs` tests behind
  `require_db_or_skip!`: cost-N debit arithmetic and the
  `training_points >= cost` floor (a cost-3 purchase at 2 points must
  affect 0 rows). Sentinel ids must fit `i32`; cleanup deletes by exact
  sentinel. Note from memory: the local dev Postgres is on **5544**, and on
  the wrong port `require_db_or_skip!` self-skips and still reports PASS —
  check the skip count, not just the green.
- **Wire-format** (type 2): byte-exact `onTrainerOpen` for a mixed
  trainable/untrainable list (5 bytes per entry, no marker, `CostToRespec`
  trailing), and a byte-exact `onAbilityTreeInfo` built from DB rows rather
  than the literal — the latter is the guard that keeps step 8 from
  regressing.
- **Seed guard** (unit over the loaded maps): assert every
  `trainer_abilities` row has a matching `archetype_ability_tree` row for
  the same archetype. This converts today's runtime
  `trainer_offered_unbound` WARN (`interactions\trainer.rs`) into a
  build-time failure.

### Explicitly deferred, stated as assumptions

- **Level cap 50.** Separate patch; touches `MAX_LEVEL`, both XP tables,
  and the compile-time assert. Until then, pack nodes above level 20 are
  imported but unreachable — which is correct and safe (the level gate
  rejects them), just incomplete.
- **Ability respec.** No client→server method exists. `CostToRespec` stays
  a placeholder. Would need protocol archaeology to find the real method,
  or a GM-only path via the existing `gmRespec` (155).
- **Weapon-family enforcement.** Phase 2. Needs a schema decision on
  whether `abilities.item_monikers` is the intended carrier before any code
  lands.
- **Warmup as a real cast-time gate.** Phase 2. Today it only triggers an
  animation; making it authoritative means a pending-cast state, an
  interrupt rule, and a decision about what cancels a cast — none of which
  the pack specifies.

---

## Appendix: pack data shape observed

`data/trainer_server_export.json` — `{warning, records}`, 439 records.
Per-record fields: `archetype`, `branch`, `node_order`,
`unlock_level_project_v1`, `required_branch_points_project_v1`,
`skill_point_cost_project_v1`, `ability_id`,
`primary_prereq_ability_id`, `additional_prereq_ability_ids`,
`is_branch_root`, `is_capstone`, `raw_training_cost`, `purchase_status`,
`evidence`, `project_status`. All costs are 1; 21 capstones; 59 rows with
`raw_training_cost = 0`; 3 branches per archetype (Free Jaffa has a fourth,
`Tau'ri`, with 8 nodes — worth noting against the pack's own QA test #1
"a new character sees exactly three branches for its archetype", and
against the repo's `tree_index_sanity CHECK (tree_index BETWEEN 0 AND 2)`,
which **would reject a fourth branch outright**).

`data/abilities_final_v1.json` — `{schema_version 1.0, ability_count 419,
records}`. Rich per-ability rows including `Cooldown s`, `Warmup s`,
`Min/Max Range`, `Effect IDs`, `Moniker IDs`, `Weapon Family`,
`Flags Raw` + `Flags Decoded`, `Ammo Cost Text`, and a
`Final v1 Progression` array (one entry per archetype that can learn it).
The `Weapon Family` field is empty on the sampled records, so B-level
weapon-family enforcement has no data to drive it yet either.

**Extra finding worth escalating:** the Free Jaffa `Tau'ri` fourth branch
collides with the `tree_index_sanity` CHECK constraint on
`archetype_ability_tree`. That is a Phase 0 collision the checklist asks
for, and it is not resolvable by data alone — either the constraint is
wrong or the pack's fourth branch is (pack `KNOWN_UNKNOWNS.md:22` does flag
"Jaffa Tau'ri branch is particularly revision-heavy").
