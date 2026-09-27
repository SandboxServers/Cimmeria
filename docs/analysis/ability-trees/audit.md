# Ability Trees: Compatibility Delta Against `main`

> Type: reference. Audience: the coordinator and packet workers.
> Updated: 2026-09-25, against `main` @ `acbcc22e` (the same commit the FINAL v1 handoff was built on). Companions: [campaign README](README.md), [work packets](work-packets.md), [source data](source/README.md), [Phase 0 gap report](../sgw-handoff-pack-v1.2/phase0-gap-report.md), [trainer runtime audit](../sgw-handoff-pack-v1.2/audits/audit-trainer-runtime.md).

Each row is checked against the code, the seed or the client files, not taken from either handoff. IDs (`A-nn`) are cited by the work packets.

## 1. Handoff claims that hold

| ID | Claim | Evidence |
|---|---|---|
| A-01 | The trainer loop exists end to end: `trainAbility` → cell validation → `CellToBaseMsg::TrainAbility` → atomic base debit → `AbilityGranted` → `onKnownAbilitiesUpdate` → trainer re-send. | `cell/cell_methods/player/vendor/train.rs`, `base/world_entry/methods/progression/mod.rs:492-626`, `cell/service/base_messages/ability_granted.rs` |
| A-02 | The seed tree is a stub: Soldier and Commando only, all level 1, no prerequisites. | `db/resources/Archetypes/Seed/archetype_ability_tree.sql`. Only 113 of its 150 distinct ability ids appear in the FINAL Soldier/Commando trees. |
| A-03 | A hand-copied Soldier/Commando tree lives in Rust. | `mercury/world_data/stats.rs:42-88`. It is the fallback when the DB query returns nothing (`player_load/core/player_data.rs:173-182`), and it does **not** match the seed (for example, the Rust Commando tree 0 starts at 700, while the seed's Commando rows start at 597 in tree 1). The primary path already reads the DB. |
| A-04 | No branch-point, per-node cost, root or capstone column. | `db/resources/Archetypes/Tables/archetype_ability_tree.sql`: six columns plus `tree_index_sanity CHECK (0..2)`. There is no primary key on the table. |
| A-05 | The base debits a hard-coded 1 point. | `training_points = training_points - 1` in the `UPDATE` at `progression/mod.rs:552-560`. It is replay-safe through `NOT (abilities @> ARRAY[$1])`. |
| A-06 | Trainer authority hole: a forged `trainAbility` trains from anywhere. | `train.rs` never reads `last_interaction_target`. The interact path already has the distance gate (`interactions::interact_target_in_range`, `MAX_INTERACT_DISTANCE = 5.0`) and pins the target only after that gate passes (`cell_methods/player/interaction/interact.rs:170-209`), so a train gate can rely on the pin. |
| A-07 | The client's point counter goes stale after a purchase. | `AbilityGranted` carries `training_points_remaining`, but `ability_granted.rs` only logs it. The level-up bundle already has the right wire shape to copy: `ON_ENTITY_PROPERTY(GENERICPROPERTY_TrainingPoints = 1, value)` in `build_grant_xp_bundle`. |
| A-08 | `MAX_LEVEL = 20`. | `crates/game/src/player.rs:6`, `sgw_player.level_sanity CHECK (level <= 20)` in `db/sgw/Players/Tables/sgw_player.sql:59`, and a second XP table `LEVEL_EXP` in `stats.rs:90-103` that duplicates `LEVEL_XP`. |
| A-09 | All FINAL ability ids exist. | 439 of 439 node ids and every prerequisite id resolve in `db/resources/Abilities/Seed/abilities.sql` (1,886 abilities). |
| A-10 | Counts. | 439 nodes, 419 unique ids, 21 branches, 3 per archetype. Per archetype: Soldier 72, Commando 64, Scientist 59, Archaeologist 65, Asgard 66, Free Jaffa/Shol'va 51, Goa'uld 62. 59 nodes have raw `training_cost = 0`; 243 unlock above level 20. |
| A-11 | Data invariants. | Every branch has contiguous node orders from 1, exactly one root and one level-50 capstone, and unlock levels only from {1, 5, …, 50}. `required_branch_points` rises 0, 2, …, 20 with the tier. No ability repeats inside one archetype, so `(archetype, ability_id)` can be unique. Every prerequisite is in the same branch. There are no self-references or cycles. 70 prerequisites sit in the same tier as their dependant, which is fine because the graph is acyclic. |

## 2. Findings the handoff did not have

| ID | Finding | Consequence |
|---|---|---|
| A-20 | **The FINAL v1 branch-point gate cannot be met per branch.** Each branch has one tier-0 node (its root, 1 point) but tier 1 needs 2 points. A purchase simulation at level 50 with unlimited points reaches only the 21 roots per branch. Counted across the archetype, it reaches all 439. Two alternative per-branch readings were also simulated: `rbp-1` reaches 403 and `rbp/2` reaches 433. | The owner's v2 workbook settles this: spend is **archetype-wide** (D-AT01). The v1 acceptance line "branch-A points must not satisfy a gate in branch B" is withdrawn. Branch isolation comes from prerequisites instead, because every prerequisite is in the node's own branch (A-11). |
| A-21 | **The in-game trainer UI is the Ability window, not the Trainer window.** `UI/Core/Trainer/Trainer.toc` has `<Enabled>false</Enabled>` (this confirms `docs/client/ui-layout-inventory.md:129`), but the enabled `UI/Core/Ability/Ability.lua` subscribes to `Events.TrainerOpen`, switches to `TrainingMode`, shows the trainer portrait, and calls `buyTrainable(id)` on an enabled button. | AT-06 needs **no client patch**. This clears the Phase 0 report's "trainer layout disabled" UAT blocker. |
| A-22 | **The Ability window draws at most 30 buttons per tab** (`AbilityMod.MAX_BUTTONS = 30`; `Ability.layout` defines `Ability_Button1`-`30`). | The largest FINAL branch has 25 nodes, so it fits. AT-05 adds a seed guard of 30 or fewer per branch. |
| A-23 | **The client enables a node only when the server says it is trainable.** Buttons use `getTrainableInfo(id).trainable` (the `onTrainerOpen` byte) and are otherwise disabled. Nodes missing from the trainer's offered list come back from `getTrainableInfo` with no id and are **hidden**, not greyed. | Two rules follow. (1) The `trainable` byte must be computed by the same predicate as the purchase gate: branch spend, points and cost included, or the player clicks an enabled button and gets nothing. (2) To show locked nodes greyed, the trainer must offer the whole tree, with `trainable = 0` on locked nodes. AT-E1 confirms the native join between `onAbilityTreeInfo` and the trainer list. |
| A-24 | **Every training rejection is silent.** Nine `return`s in `train.rs` log and send nothing. | This breaks the project rule that every button press gets visible feedback. AT-04 sends `onErrorCode` (method 121, `ERRORCODE_SYSTEM_Ability`, instance = ability id) and re-sends `onTrainerOpen` so a stale window corrects itself. |
| A-25 | **The respec button is live but does nothing.** In `TrainingMode` the Ability window shows `AbilityRespecButton`; accepting the prompt calls `respecAbilities()` → cell method 72 `resetMyAbilities`, which is a stub. `onTrainerOpen` already advertises `CostToRespec = 1000`. | AT-08 implements v2's respec rule. Until then, a press greys the button and nothing else happens. |
| A-26 | **Starter collision.** `char_creation_abilities` grants 1646 (Health Heal) to every char def, and 1646 is a Goa'uld Servant Lord node. Because an already-known ability is a silent no-op, a Goa'uld can never buy that node, and it adds no spend. | v2 says to reconcile it before Goa'uld UAT. Goa'uld UAT is out of showcase scope, so the AT-05 seed guard lists it as the one known exception (D-AT09). |
| A-27 | The trainer's `trainable` computation is a second copy of the gates. | `cell/interactions/trainer.rs` re-implements level, prerequisite and known checks separately from `train.rs`. AT-01 folds both into one predicate. |
| A-28 | Ability-tree loading runs twice: the cell caches all archetypes at startup (`cell/spawner/abilities.rs:173`), and the base queries per player load (`player_load/meta.rs:156`). | Both read the same table, so they cannot disagree on content today. They can disagree on ordering and columns once the schema grows. AT-01 introduces one catalog loader and AT-02 moves the base onto it. |

## 3. Stale statements in the older pack

- Phase 0 D4 ("Tau'ri is a fourth Free Jaffa branch, defer it") is superseded. FINAL v1 and v2 have exactly three branches: Heritage, Tau'ri and Tactics.
- Phase 0's "cap stays 20, 2 points per level" is superseded by v2 (D-AT02).
- Phase 0's "trainer layout disabled, UAT may need a `.toc` patch" is resolved by A-21.

The documents under `sgw-handoff-pack-v1.2/` are left as they are, as a historical import. This audit and the [README](README.md) decisions take precedence.
