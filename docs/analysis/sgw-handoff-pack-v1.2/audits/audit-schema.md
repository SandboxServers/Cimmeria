# Audit: SGW handoff pack v1.2 progression model vs existing Cimmeria persistence

Read-only audit. No files modified. All repo paths absolute-from-root `C:\Users\Steve\source\projects\Cimmeria\`.

Pack root: `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\sgw-handoff-pack-v1.2\pack\`

---

## 1. Existing surface

| Concern | Where it lives | Notes |
|---|---|---|
| Archetype catalog | `db\resources\Archetypes\Tables\archetypes.sql:6-19`, seed `db\resources\Archetypes\Seed\archetypes.sql` | 9 `EArchetype` values including `ARCHETYPE_Any`, `ARCHETYPE_Sholva`, `ARCHETYPE_Jaffa`. PK on `archetype`. Enum type at `db\resources\Archetypes\Types\EArchetype.sql`. |
| Skill tree | `db\resources\Archetypes\Tables\archetype_ability_tree.sql:6-14` | Columns `(archetype "EArchetype", ability_index integer, ability_id integer, tree_index integer, level integer DEFAULT 1, prerequisite_abilities integer[] DEFAULT '{}')`. PK `(archetype, tree_index, ability_index)` at `db\resources\_primary_keys.sql:68`. `CONSTRAINT tree_index_sanity CHECK (tree_index >= 0 AND tree_index <= 2)`. FK `ability_id -> abilities(ability_id)` at `db\resources\_foreign_keys.sql:46`. |
| Ability definitions | `db\resources\Abilities\Tables\abilities.sql:6-33` | 1,886 seed rows in `db\resources\Abilities\Seed\abilities.sql`. Carries `training_cost integer NOT NULL` (line 24), `cooldown real`, `warmup real`, `min_range`/`max_range`, `required_ammo`, `effect_ids integer[]`, `flags`, `target_collection_method`, `threat_level_id`. |
| Trainer NPC offering | `db\resources\Abilities\Tables\trainer_abilities.sql:6-10` `(list_id, archetype, ability_id)`, PK at `_primary_keys.sql:612`; `db\resources\Abilities\Tables\trainer_ability_lists.sql:6-9`; `db\resources\Entities\Tables\entity_templates.sql:35` (`trainer_ability_list_id integer`) | Exactly one seeded list — `list_id=1`, "Debug ability list", Commando-only rows. Template 25 ("Interaction Debug NPC") is the only trainer. |
| NPC ability sets | `db\resources\Abilities\Tables\ability_sets.sql:6-9`, `ability_set_abilities.sql:6-9` | NPC-side, unrelated to player progression. 3 sets seeded. **Latent bug:** `ability_set_abilities_pkey PRIMARY KEY (ability_set_id)` at `db\resources\_primary_keys.sql:44` — should be composite `(ability_set_id, ability_id)`; as written a set can hold only one ability. |
| Starter abilities | `db\resources\Archetypes\Tables\char_creation_abilities.sql:6-9` (PK `(char_def_id, ability_id)` at `_primary_keys.sql:116`) | 115 seed rows, keyed by `char_def_id` not archetype. Backfilled for 15 previously-empty char defs by `db\scripts\seed_starter_abilities_all_archetypes.sql`. |
| **Per-character learned abilities** | `db\sgw\Players\Tables\sgw_player.sql:28` — `abilities integer[] DEFAULT '{}' NOT NULL` | Denormalised array on the character row. No per-ability source, no learned-at level, no timestamp. This is the only store of "which abilities does this character have". |
| **Skill points** | `db\sgw\Players\Tables\sgw_player.sql:33` — `training_points integer DEFAULT 0 NOT NULL` | Single unspent counter. No spent counter. No per-branch counter. |
| Character level / XP | `db\sgw\Players\Tables\sgw_player.sql:9` (`level integer DEFAULT 1`), `:23` (`exp integer DEFAULT 0`) | `CONSTRAINT level_sanity CHECK (level >= 0 AND level <= 20)` at line 59. |
| Crafting progression (the normalised precedent) | `db\sgw\Players\Tables\sgw_player_discipline_expertise.sql:25-31` | `(player_id, discipline_id, expertise)` PK `(player_id, discipline_id)`, `ON DELETE CASCADE` FK at `db\sgw\_foreign_keys.sql:75`. The file header explains exactly why a `{id -> value}` map was split out of `sgw_player` instead of stored as a parallel array. This is the template to copy. |
| Crafting tree analogue | `db\resources\Archetypes\Tables\disciplines.sql:6-17` | Has `row`, `column`, `required_discipline_ids integer[]`, `racial_paradigm_level`, `tech_competency`. Shows the repo already models a gated tree with an array prereq column. |
| Other progression columns on `sgw_player` | `sgw_player.sql:34-37` | `discipline_ids integer[]`, `racial_paradigm_levels integer[]`, `applied_science_points integer`, `blueprint_ids integer[]`. Same array-on-row pattern as `abilities`. |
| Train handler — validation (cell) | `crates\services\src\cell\cell_methods\player\vendor\train.rs:31-199` | Six gates, in order: (1) `ability_id` in `space_mgr.ability_defs`; (2) entity has a `player_id`; (3) not already known (silent no-op); (4) ability present in the player's `archetype_ability_tree`; (5) `player_level >= tree_entry.level`; (6) every `prerequisite_abilities` entry known. Then sends `CellToBaseMsg::TrainAbility`. |
| Train handler — persist + debit (base) | `crates\services\src\base\world_entry\methods\progression\mod.rs:478-610`, SQL at `:534-546` | `UPDATE sgw_player SET abilities = abilities \|\| $1::integer, training_points = training_points - 1 WHERE player_id = $2 AND training_points > 0 AND NOT (abilities @> ARRAY[$1::integer]) RETURNING training_points`. Single-statement atomic: the `training_points > 0` guard and the `NOT (@>)` guard together make double-click and stale-cache double-debit impossible. Replies `BaseToCellMsg::AbilityGranted`. |
| XP / TP award | `crates\services\src\base\world_entry\methods\progression\mod.rs:70-192`, SQL at `:115-124` | `UPDATE sgw_player SET exp = $1, level = $2, training_points = $3 WHERE player_id = $4`. Persist-before-mutate ordering is deliberate (comment at `:64-69`). |
| Level / TP constants | `crates\game\src\player.rs:6` `MAX_LEVEL = 20`; `:17` `TRAINING_POINTS_PER_LEVEL = 2`; `:10` `LEVEL_XP: [u64; 21]` | Compile-time assert at `progression\mod.rs:24-27` pins `LEVEL_XP.len() == MAX_LEVEL + 1`. |
| Trainer open | `crates\services\src\cell\interactions\trainer.rs` (483 lines) | `onTrainerOpen`, flat client method 113. Wire: `INT32 TrainerID, UINT32 count, [N x (INT32 abilityID + UINT8 trainable)], INT32 CostToRespec`. `DEFAULT_RESPEC_COST = 1000` (preserved Python placeholder). Emits `trainer_empty_offering` and `trainer_offered_unbound` negative logs. |
| Respec | `crates\services\src\cell\cell_methods\player\combat\mod.rs:124-125` | `resetMyAbilities` (CM 72, `cell_methods\player\constants.rs:10`) logs `UNIMPLEMENTED`. Confirmed player-facing, not GM-gated (`cell\dispatch\gm_gate.rs:304-327`). |
| Tree load into cell | `crates\services\src\cell\service\startup.rs:237-239` -> `spawner::load_archetype_ability_trees(pool)`; cache at `crates\services\src\cell\space_manager\mod.rs:165-170` (`archetype_ability_trees: HashMap<i32, Vec<ArchetypeAbilityTreeEntry>>`) | DB-driven. |
| **Duplicate tree source (hazard)** | `crates\services\src\mercury\world_data\stats.rs:45-88` | Soldier and Commando trees **hardcoded in Rust** for the wire `createPlayer`/`AbilityTreeData` payload, entirely separate from the DB load above. Counts match the seed today (Soldier 29/28/27, Commando 28/30/27) but only by hand. `stats.rs:90-96` also hardcodes a second XP table `LEVEL_EXP` distinct from `crates\game\src\player.rs` `LEVEL_XP`. |
| Character load | `crates\services\src\base\world_entry\methods\player_load\core\player_data.rs:61-66` | `SELECT ... abilities, training_points, applied_science_points, blueprint_ids ... FROM sgw_player WHERE player_id = $1 AND account_id = $2`. Account-ownership check present. |
| Character create | `crates\services\src\base\character_create.rs:351-413` | `SELECT ability_id FROM resources.char_creation_abilities WHERE char_def_id = $1`, then a single `INSERT INTO sgw_player (... abilities, access_level)`. |
| In-memory session mirror | `crates\services\src\base\mod.rs:218` `player_training_points: Option<u32>`; reset at `base\dispatch\session.rs:120`; hydrated at `base\world_entry\play_character.rs:171` | Cache only; DB row is authoritative. |

---

## 2. Proposed ↔ existing mapping

| Pack table (`docs\SERVER_SCHEMA_PROPOSAL.md`) | Equivalent exists? | Field-level mapping | Missing | Recommendation |
|---|---|---|---|---|
| **`character_skill_state`** (character_id, archetype, available_skill_points, spent_skill_points, version) | **Partial — adapt existing** | `character_id` -> `sgw_player.player_id`; `archetype` -> `sgw_player.archetype` (integer 0-8, not the `EArchetype` enum used in `resources`); `available_skill_points` -> `sgw_player.training_points` | `spent_skill_points`, `version` | **Adapt.** Add `training_points_spent integer NOT NULL DEFAULT 0` and `progression_version smallint NOT NULL DEFAULT 1` to `sgw_player`. A separate table would duplicate the per-character identity 1:1 and, worse, would split the atomic debit at `progression\mod.rs:534-546` across two rows — turning a single guarded `UPDATE` into a transaction. Not worth it. Note the archetype representation split: `sgw_player.archetype` is `integer` while `resources.archetype_ability_tree.archetype` is `"EArchetype"`; any join needs an explicit int-to-enum mapping (today the Rust side keys the in-memory cache by `archetype_id: i32`). |
| **`character_learned_ability`** (character_id, ability_id, learned_at_level, source, learned_at) | **Partial — needs a new table** | `character_id` + `ability_id` -> elements of `sgw_player.abilities integer[]` | `learned_at_level`, `source` (`starter`/`trainer`/`mission`/`system`), `learned_at` — **none of the three is representable in an `integer[]`** | **New table.** `sgw_player_ability`, modelled directly on `sgw_player_discipline_expertise` (same reasoning, same `ON DELETE CASCADE`, same composite PK shape). Keep `abilities integer[]` populated in the same transaction during rollout: it is the hot read path on login and the source for `onKnownAbilitiesUpdate`, and rolling deploys need old code to keep working. Flip the read path to the table in a later phase. |
| **`skill_tree_node`** (archetype, branch, node_order, ability_id, unlock_level, required_branch_points, skill_point_cost, primary_prereq_ability_id, is_branch_root, is_capstone, evidence_status, project_version) | **Partial — adapt existing** | `archetype` -> `archetype`; `branch` -> `tree_index` (integer 0-2, **unnamed**); `node_order` -> `ability_index`; `ability_id` -> `ability_id`; `unlock_level` -> `level`; `skill_point_cost` -> `abilities.training_cost` (lives on the ability, not the node); `primary_prereq_ability_id` -> first element of `prerequisite_abilities` | `required_branch_points`, `is_branch_root`, `is_capstone`, `evidence_status`, `project_version`, and a branch **name** | **Adapt `archetype_ability_tree`.** Its PK is already `(archetype, tree_index, ability_index)`, i.e. exactly `(archetype, branch, node_order)`. Both the cell tree cache and the wire tree payload key off that shape, so replacing it would touch `startup.rs`, `space_manager`, `train.rs`, `trainer.rs` and `stats.rs`. Add the five missing columns plus branch naming. |
| **`skill_tree_node_prereq`** (archetype, branch, ability_id, prereq_ability_id) | **Exists, denormalised** | `prerequisite_abilities integer[]` on the same node row covers primary and additional prereqs together | Nothing functionally missing | **Keep the array.** It is read whole on every trainer open (`trainer.rs:160+`) and every train attempt (`train.rs:139-149`), and is never joined or filtered. A child table buys referential integrity at the cost of an extra fetch per node in a per-interaction hot path. The sibling table `resources.disciplines.required_discipline_ids` uses the same array-prereq pattern, so this is house style. If a distinguished primary is needed, add `primary_prereq_ability_id integer` as a nullable scalar alongside the array rather than normalising. |

**Net verdict on "prefer adapting the existing server model":** three of four proposed tables map onto existing structures and should be adapted in place. Only `character_learned_ability` genuinely needs new storage, and only because `learned_at_level` / `source` / `learned_at` cannot live in an `integer[]`.

---

## 3. Data comparison (computed counts)

Method: `data\trainer_server_export.json` parsed with `ConvertFrom-Json` (439 records under `.records`); `db\resources\Abilities\Seed\abilities.sql` and `db\resources\Archetypes\Seed\archetype_ability_tree.sql` parsed by regex over the `INSERT` statements.

### Coverage

| Metric | Value |
|---|---|
| Export records | 439 |
| Export distinct `ability_id` | 419 |
| Export `ability_id` present in our `abilities` seed | **419 of 419 (100%)** |
| Export `ability_id` absent from our `abilities` seed | **0** |
| Our `abilities` seed rows | 1,886 — exactly matches the pack README's "recovered ability/effect master for 1,886 abilities" |
| Our `archetype_ability_tree` rows | 169 (Soldier 84, Commando 85; **no rows for any other archetype**) |
| Our tree distinct `ability_id` | 150 |
| Our tree abilities **absent** from the export | **28** |
| Our tree **rows** whose ability is absent from the export | **33 of 169** |

Absent ability IDs (present in our seed tree, no node in the export): `592, 594, 597, 653, 660, 696, 697, 728, 745, 772, 775, 780, 810, 812, 879, 1233, 1249, 1361, 1880, 1884, 1886, 1888, 1892, 1959, 2420, 2421, 2422, 2910`.

### Archetype agreement

Restricting to the 136 seed rows whose ability also appears in the export:

| | Count |
|---|---|
| Seed archetype agrees with an export archetype for the same ability | **116** |
| Seed archetype disagrees with every export archetype for that ability | **20** |

The 20 disagreements (`ability seed=X export=Y`):

```
1005 seed=Commando  export=Scientist
1005 seed=Soldier   export=Scientist
1481 seed=Soldier   export=Free Jaffa / Shol'va, Goa'uld
1482 seed=Soldier   export=Free Jaffa / Shol'va, Goa'uld
1619 seed=Commando  export=Soldier
1638 seed=Soldier   export=Archaeologist
1724 seed=Commando  export=Free Jaffa / Shol'va
2104 seed=Soldier   export=Free Jaffa / Shol'va
 654 seed=Commando  export=Archaeologist
 654 seed=Soldier   export=Archaeologist
 656 seed=Commando  export=Soldier
 706 seed=Commando  export=Archaeologist
 716 seed=Commando  export=Soldier
 722 seed=Commando  export=Soldier
 774 seed=Soldier   export=Goa'uld
 847 seed=Soldier   export=Commando
 856 seed=Commando  export=Soldier
 867 seed=Commando  export=Soldier
 868 seed=Commando  export=Soldier
 891 seed=Commando  export=Soldier, Scientist
```

### Archetype naming

Export uses 7 display names; our enum has 9 values.

| Export name | Node count | Our `EArchetype` | Mapping |
|---|---|---|---|
| `Soldier` | 72 | `ARCHETYPE_Soldier` | identity |
| `Asgard` | 66 | `ARCHETYPE_Asgard` | identity |
| `Archaeologist` | 65 | `ARCHETYPE_Archeologist` | **spelling differs** (`ae` vs `e`) |
| `Commando` | 64 | `ARCHETYPE_Commando` | identity |
| `Goa'uld` | 62 | `ARCHETYPE_Goauld` | punctuation differs; seed `name` column already stores `Goa''uld` |
| `Scientist` | 59 | `ARCHETYPE_Scientist` | identity |
| `Free Jaffa / Shol'va` | 51 | `ARCHETYPE_Sholva` **and** `ARCHETYPE_Jaffa` | **one-to-two collision — unresolvable without new evidence** |
| — | — | `ARCHETYPE_Any` | no export counterpart (NPC/wildcard sentinel) |

Our `archetypes` seed `name` column already holds the display strings `Soldier`, `Commando`, `Scientist`, `Archeologist`, `Asgard`, `Goa'uld`, `Shol'va`, `Jaffa`, and `''` for `_Any` — so a name-based join is possible for 6 of 7 and fails only on `Archaeologist`/`Archeologist` and the Free Jaffa / Shol'va merge.

### Branch ↔ `tree_index`

| Archetype | Our seed rows per `tree_index` | Export nodes per branch | Agreement |
|---|---|---|---|
| Soldier | tree0 = 29, tree1 = 28, tree2 = 27 | Automatic Weapons 22, Command 25, Heavy Weapons 25 | **Disagrees.** tree0 -> Automatic Weapons 22 + Command 1; tree1 -> Heavy Weapons 15 + Command 2; tree2 -> Command 15. The export's Heavy Weapons / Command split does not fall on our tree1/tree2 boundary. |
| Commando | tree0 = 28, tree1 = 30, tree2 = 27 | Demolitions 23, Precision / Marksmanship 19, Stealth / Infiltration 22 | **Agrees.** tree0 -> Demolitions (20 matched), tree1 -> Stealth / Infiltration (22), tree2 -> Precision / Marksmanship (19). Clean 1:1 with no cross-contamination. |

### `level` / `unlock_level` / prereqs / cost

| Metric | Our seed | Export |
|---|---|---|
| Distinct `level` / `unlock_level_project_v1` values | **`{1}` only** — every one of 169 rows is level 1 | 11 tiers: 1(21), 5(41), 10(40), 15(47), 20(47), 25(49), 30(46), 35(47), 40(41), 45(39), 50(21) |
| Rows with non-empty prerequisites | **0 of 169** | 418 nodes with `primary_prereq_ability_id`; 22 nodes with `additional_prereq_ability_ids` |
| `required_branch_points` | no column | `{0,2,4,6,8,10,12,14,16,18,20}` |
| `skill_point_cost` | `abilities.training_cost` (per-ability, varies) | flat `1` for all 439 |
| `raw_training_cost = 0` nodes | — | **59** (`purchase_status = "VERIFY RAW COST 0"`); remaining 380 are `"SUPPORTED AS TRAINABLE"` |
| Nodes above our `level_sanity` cap of 20 | — | **243 of 439** |
| Total skill points to buy every node | — | **439** |
| Skill points obtainable in Cimmeria at level 20 | **38** (`(20-1) * TRAINING_POINTS_PER_LEVEL=2`) | — |

So there is **zero agreement** on `level`: our seed is uniformly 1, the export is an 11-tier ladder. And **zero agreement** on prerequisites: our seed has none, the export has 418+22. These are not conflicting values — our seed simply never populated the fields, which makes the export additive rather than contradictory for those two columns.

### ID collisions

- **No collision** between export ability IDs and our `abilities` table — all 419 resolve.
- **19 export `ability_id`s appear in two different nodes**, always across different archetypes:

```
 523: Soldier/Command#4              ; Commando/Demolitions#2
 598: Soldier/Automatic Weapons#1    ; Scientist/Support#4 ; Archaeologist/Sociology#16
 646: Commando/Stealth/Infiltration#1; Goa'uld/Ashrak#2
 649: Commando/Stealth/Infiltration#2; Free Jaffa / Shol'va/Tactics#11
 713: Soldier/Automatic Weapons#11   ; Scientist/Support#6
 718: Soldier/Automatic Weapons#9    ; Archaeologist/Sociology#17
 861: Soldier/Command#13             ; Commando/Demolitions#7
 865: Commando/Stealth/Infiltration#17; Goa'uld/Ashrak#12
 866: Commando/Stealth/Infiltration#21; Goa'uld/Ashrak#18
 874: Commando/Stealth/Infiltration#7; Free Jaffa / Shol'va/Tactics#12
 891: Soldier/Automatic Weapons#8    ; Scientist/Support#12
1364: Soldier/Command#20             ; Commando/Demolitions#13
1451: Soldier/Command#18             ; Archaeologist/Sociology#13
1474: Soldier/Automatic Weapons#14   ; Scientist/Support#14
1481: Free Jaffa / Shol'va/Heritage#4; Goa'uld/Battle Lord#6
1482: Free Jaffa / Shol'va/Heritage#5; Goa'uld/Battle Lord#7
1518: Commando/Stealth/Infiltration#5; Goa'uld/Ashrak#7
1626: Free Jaffa / Shol'va/Tactics#8 ; Goa'uld/Battle Lord#8
2864: Soldier/Command#23             ; Commando/Demolitions#17
```

Note 598 (`Soldier/Automatic Weapons` root, `is_branch_root: true`) also appears mid-branch in Scientist and Archaeologist. Cross-archetype reuse is fine for the PK as it stands.

- **Real collision risk:** our PK `(archetype, tree_index, ability_index)` does **not** prevent the same `ability_id` twice within one archetype's tree. The cell-side lookup at `train.rs:100-103` and `trainer.rs` both do a linear `.find(|e| e.ability_id == ability_id)`, which silently resolves to the first match. Loading the export as-is with 19 cross-archetype reuses is safe; loading any future revision that duplicates an ability within one archetype would be a silent wrong-node bug. A `UNIQUE (archetype, ability_id)` constraint closes it.
- **`ability_index` is per-branch, not per-archetype:** our seed uses 1..28 repeated once per `tree_index`, giving 3 rows per `(archetype, ability_index)`. That matches the PK, but means `ability_index` is *not* a stable node identity on its own.

---

## 4. Evidence labels

Using the four labels from `docs\SOURCE_POLICY.md`.

**CONFIRMED / SOURCE-BACKED**
- Our `abilities` seed contents: 419/419 export IDs resolve, and the row count (1,886) matches the pack's independently-derived recovered master exactly. Two independent paths landing on the same figure.
- Every DDL fact, constraint, PK, FK and Rust code path cited in section 1. These are read directly from the repo.
- The existing train flow's atomicity: the `WHERE ... AND training_points > 0 AND NOT (abilities @> ARRAY[$1])` guard at `progression\mod.rs:538-540` is in the tree and commented as deliberate.

**USER-CONFIRMED**
- `important.trainer_tutorial_source_backed`: three skill branches per character; unavailable skills grayed out; skills purchased with skill points; lower-level skills must be purchased before higher-level. Our `CONSTRAINT tree_index_sanity CHECK (tree_index >= 0 AND tree_index <= 2)` independently corroborates the three-branch claim from a second source.

**RECONSTRUCTION / INFERENCE**
- Every `unlock_level_project_v1`, `required_branch_points_project_v1`, `skill_point_cost_project_v1`, `node_order`, `primary_prereq_ability_id` and `additional_prereq_ability_ids` value in the export. The pack's own top-level `warning` field states this verbatim, and `README.md:44-46` repeats it.
- Branch labels for Commando Stealth/Infiltration and Precision/Marksmanship, Scientist Robotics, Archaeologist Archaeology (`KNOWN_UNKNOWNS.md:23-24`).
- Our own `TRAINING_POINTS_PER_LEVEL = 2` and `LEVEL_XP` curve — project choices, not recovered.
- Our `DEFAULT_RESPEC_COST = 1000` (a preserved Python placeholder, flagged as a TODO in the original).

**PARTIAL / UNRESOLVED / MISSING**
- Level cap: ours is 20 (`level_sanity` CHECK, `MAX_LEVEL`), the pack records `historical_level_cap: 50`. Both cannot be right.
- 59 export nodes at `raw_training_cost = 0` / `purchase_status = "VERIFY RAW COST 0"`; `KNOWN_UNKNOWNS.md:20` lists "meaning of all Training Cost = 0 cases" as unresolved.
- The `Free Jaffa / Shol'va` -> two-enum split.
- The 20 archetype disagreements between seed and export.
- The 28 seed abilities the export omits — could be QA-snapshot test content (the pack warns about exactly this at `SOURCE_POLICY.md:25` and `KNOWN_UNKNOWNS.md:43-45`) or export gaps. No basis to choose.
- Soldier's tree1/tree2 boundary vs the export's Heavy Weapons / Command split.
- `char_creation_abilities` as a definition of canonical starts (`KNOWN_UNKNOWNS.md:33`).

**Policy note:** per `SOURCE_POLICY.md:24`, our seed SQL is a structural hint, not canonical final design. So where seed and export disagree the seed does not automatically win — but neither does the reconstruction. Both sides of each of the 20 archetype disagreements need to be preserved as a conflict, not silently reconciled (`SOURCE_POLICY.md:22`).

---

## 5. Blockers for a Phase 1 "trainer + learned abilities persistence" patch

1. **Level cap 20 vs 50.** `CONSTRAINT level_sanity CHECK (level <= 20)` plus `MAX_LEVEL = 20` makes **243 of 439** export nodes permanently unreachable. Raising the cap is not a one-line change: `LEVEL_XP` is a fixed `[u64; 21]` with a compile-time assert at `progression\mod.rs:24-27`, `stats.rs:90` carries a *second* 21-entry `LEVEL_EXP` table, and the level/XP threshold values go out on the wire. Needs an owner decision: ship Phase 1 at cap 20 with a truncated tree, or raise to 50 with a reconstructed XP curve (which the pack explicitly does not supply).
2. **Skill-point economy.** 439 points to buy every node; 38 obtainable at level 20. Even at cap 50 with 2/level that is 98. `KNOWN_UNKNOWNS.md:19` lists the original cadence as unrecovered. Without a decision, any "buy the whole tree" QA test is unwritable.
3. **Two competing cost sources.** `abilities.training_cost` already exists per-ability and varies; the export asserts a flat `skill_point_cost_project_v1 = 1` per node, with 59 abilities flagged cost-0-and-verify. The debit amount in `progression\mod.rs:537` is currently hardcoded `- 1`. Which source wins must be settled before that line changes.
4. **`Free Jaffa / Shol'va` enum split.** 51 export nodes, no rule to divide them between `ARCHETYPE_Sholva` and `ARCHETYPE_Jaffa`. A seed load cannot proceed for that archetype without a human call.
5. **Branch points have no storage anywhere.** `required_branch_points` needs a per-`(character, archetype, branch)` spend count. Nothing in `sgw_player` or any child table tracks which branch a spent point went into, and the current flat `training_points` decrement discards that information irrecoverably. This must land *before* points are spent under the new rules, or existing characters cannot be reconstructed.
6. **Respec is unimplemented while the wire already advertises it.** `onTrainerOpen` sends `costToRespec` every time (`trainer.rs`), but `resetMyAbilities` (CM 72) is a stub at `combat\mod.rs:124-125`. Adding `training_points_spent` and branch-point tracking without a refund path ships a one-way door: a player who misspends has no recovery and the server has no way to zero the new counters.
7. **Duplicated tree source of truth.** `stats.rs:45-88` hardcodes Soldier and Commando trees in Rust for the wire payload while the cell loads the same data from the DB at `startup.rs:237`. Seeding the export into `archetype_ability_tree` will silently desync the client-facing tree unless `stats.rs` is regenerated in the same change. Same hazard for the two XP tables (`player.rs:10` vs `stats.rs:90`).
8. **The 20 archetype disagreements and 28 missing abilities need a per-row disposition** before a bulk seed load, or the load will either drop working content or move abilities between classes for live characters who already trained them. There is no migration path for a character holding an ability that has moved out of their archetype's tree — `train.rs` gate 4 would then reject re-training it, and nothing currently revokes it.
9. **Archetype type mismatch across schemas.** `sgw_player.archetype` is `integer` with `CHECK (0..8)`; `resources.archetype_ability_tree.archetype` is `"EArchetype"`. Any SQL that joins learned abilities to tree nodes needs an explicit mapping. Today the Rust cache sidesteps this by keying on `i32`.
10. **Migration-vs-seed policy contradiction.** `db\README.md` states plainly: *"the seed data lives in `db/resources/` and is edited directly. Do not add migration scripts under `db/scripts/` — there is no migration framework; the schema is reloaded from source on setup."* Yet `db\scripts\` already contains 12 files, including `seed_starter_abilities_all_archetypes.sql` and `add_player_discipline_expertise.sql`, which are exactly the shape of change Phase 1 needs.

### Reconciling the pack's "keep migrations reversible" rule with repo policy

The pack rule and the repo rule are compatible if reversibility is relocated rather than abandoned:

- **Edit `db\resources\` and `db\sgw\` in place.** New columns go into the `Tables/*.sql` `CREATE TABLE` body; new rows go into `Seed/*.sql`. No new file under `db\scripts\`.
- **The rollback is `psql -f db/database.sql`.** A full reload from source is the repo's revert mechanism, which is why no migration framework exists. Reversibility is satisfied by the schema being fully reconstructible from the tree at any commit, plus `git revert`.
- **Put the `DOWN` SQL in a header comment** in the same file the change lands in, so an operator with live data has an explicit path. `sgw_player_discipline_expertise.sql:1-24` is the precedent for a substantial explanatory header, and `add_player_discipline_expertise.sql` shows what the corresponding one-off looked like.
- **Prove round-trip with a live-DB test.** Per `TESTING.md` and `CLAUDE.md`, a change to a `WHERE` clause or a `rows_affected` invariant needs a live-DB regression guard, not a unit test. Phase 1 changes the train `UPDATE`, so it needs one: fresh `database.sql` load, train an ability, assert the row in `sgw_player_ability` *and* the array element *and* the decremented counters. Use `require_db_or_skip!` and a sentinel `player_id` per `crates\services\src\test_support.rs`; note that a green no-DB run proves nothing here.
- **Backfill is the one thing a reload cannot do.** Existing characters' `abilities` arrays must be projected into the new table. That backfill is a `INSERT ... SELECT unnest(abilities)` with `source = 'system'` and `learned_at_level = NULL`, and it belongs in the seed/setup path guarded by `ON CONFLICT DO NOTHING` so it is idempotent across reloads.

---

## 6. Proposed Phase 1 schema change

Prose plus table sketch. **Not applied.** Additive only — every change is forward and backward compatible with a rolling deploy, so old binaries keep working against the new schema.

### 6.1 `db\sgw\Players\Tables\sgw_player.sql` — two columns

```sql
-- Skill points already spent, for respec refunds and the pack's
-- character_skill_state.spent_skill_points. Kept on the character row
-- rather than in a child table so the existing single-statement atomic
-- debit in progression/mod.rs stays a single statement.
training_points_spent integer NOT NULL DEFAULT 0
    CHECK (training_points_spent >= 0),

-- Serialization/progression format version. Lets a future rules change
-- migrate characters in place instead of guessing which economy a row
-- was written under. 1 = flat 1-point-per-node, cap 20.
progression_version smallint NOT NULL DEFAULT 1
```

The existing debit becomes `SET abilities = abilities || $1, training_points = training_points - $3, training_points_spent = training_points_spent + $3` with the same `WHERE ... AND training_points >= $3 AND NOT (abilities @> ARRAY[$1])` guard shape. Parameterising the cost also unblocks blocker 3 without another schema change.

### 6.2 New `db\sgw\Players\Tables\sgw_player_ability.sql`

```sql
-- One row per (character, ability) the character knows. `sgw_player.abilities
-- integer[]` stays authoritative for reads during rollout and is written in the
-- same transaction; this table adds the provenance an integer[] cannot carry
-- (learned_at_level, source, learned_at). Modelled on
-- sgw_player_discipline_expertise for the same reason: an int[] cannot hold a
-- per-element payload without N coordinated array updates.
--
-- DOWN: DROP TABLE sgw_player_ability;
CREATE TABLE sgw_player_ability (
    player_id        integer     NOT NULL,
    ability_id       integer     NOT NULL,
    learned_at_level integer,                     -- NULL for backfilled rows
    source           text        NOT NULL DEFAULT 'system'
        CHECK (source IN ('starter','trainer','mission','system','gm')),
    learned_at       timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (player_id, ability_id)
);
```

FK `player_id -> sgw_player(player_id) ON UPDATE RESTRICT ON DELETE CASCADE` declared in `db\sgw\_foreign_keys.sql`, not inline — inline would fail load because `sgw_player`'s PK is not established until `_primary_keys.sql` runs. The header on `sgw_player_discipline_expertise.sql:20-25` documents this trap.

No FK to `resources.abilities`: `sgw` and `resources` are separate schemas and the existing `sgw` tables that reference resource IDs do so via `_foreign_keys.sql` (`sgw_inventory.type_id -> resources.items`), so a `RESTRICT` FK here is possible but would block ability-seed churn. Recommend leaving it out for Phase 1 and relying on the cell-side `ability_defs` gate at `train.rs:38`.

The PK is the only index needed: every query is either "all abilities for one player" (login) or "does this player have this ability" (train gate), both covered by the leading `player_id`.

### 6.3 New `db\sgw\Players\Tables\sgw_player_branch_points.sql`

```sql
-- Per-branch spend, satisfying the pack's required_branch_points gate.
-- The flat sgw_player.training_points counter discards which branch a point
-- went into, so this cannot be reconstructed later — it has to land before
-- points are spent under branch-gated rules.
--
-- DOWN: DROP TABLE sgw_player_branch_points;
CREATE TABLE sgw_player_branch_points (
    player_id    integer  NOT NULL,
    archetype    integer  NOT NULL,   -- matches sgw_player.archetype (int, not EArchetype)
    tree_index   smallint NOT NULL CHECK (tree_index >= 0 AND tree_index <= 2),
    points_spent integer  NOT NULL DEFAULT 0 CHECK (points_spent >= 0),
    PRIMARY KEY (player_id, archetype, tree_index)
);
```

`archetype` is carried rather than derived from `sgw_player` so a future respec-and-reclass does not silently reattribute historical spend. Updated with `INSERT ... ON CONFLICT (player_id, archetype, tree_index) DO UPDATE SET points_spent = sgw_player_branch_points.points_spent + 1` in the same transaction as the train.

### 6.4 `db\resources\Archetypes\Tables\archetype_ability_tree.sql` — six columns plus a constraint

```sql
required_branch_points integer NOT NULL DEFAULT 0 CHECK (required_branch_points >= 0),
skill_point_cost       integer NOT NULL DEFAULT 1 CHECK (skill_point_cost >= 0),
is_branch_root         boolean NOT NULL DEFAULT false,
is_capstone            boolean NOT NULL DEFAULT false,
-- SOURCE_POLICY.md label for THIS ROW's level/order/prereq values.
evidence_status        text    NOT NULL DEFAULT 'LEGACY_SEED'
    CHECK (evidence_status IN
        ('CONFIRMED','USER_CONFIRMED','RECONSTRUCTION','LEGACY_SEED','UNRESOLVED')),
project_version        text,   -- e.g. 'FINAL_V1'; NULL for legacy seed rows
-- Closes the silent-wrong-node hole: train.rs and trainer.rs both resolve a
-- node by linear .find() on ability_id within an archetype.
CONSTRAINT archetype_ability_tree_archetype_ability_key UNIQUE (archetype, ability_id)
```

`evidence_status` defaulting to `LEGACY_SEED` means the 169 existing rows self-label correctly with no data change, and every row loaded from the export is explicitly stamped `RECONSTRUCTION` / `FINAL_V1`. That satisfies the pack's non-negotiable rule against presenting a reconstruction as original, at the row level, in the database itself.

### 6.5 Branch naming

The export's branch names have no home. Two options; recommend the lookup table:

```sql
-- db\resources\Archetypes\Tables\archetype_branches.sql
CREATE TABLE archetype_branches (
    archetype   "EArchetype" NOT NULL,
    tree_index  smallint     NOT NULL CHECK (tree_index >= 0 AND tree_index <= 2),
    name        varchar(64)  NOT NULL,
    -- branch label authority is separately unreliable (KNOWN_UNKNOWNS.md:23-24)
    evidence_status text     NOT NULL DEFAULT 'RECONSTRUCTION',
    PRIMARY KEY (archetype, tree_index)
);
```

A separate table rather than a `branch_name` column on every node row, because the name is a property of the branch (21 rows) not the node (439 rows), and because branch-label authority is independently uncertain and wants its own evidence stamp.

### 6.6 Seed load scope

Follow the pack's own recommendation (`README.md:66-68`): **Soldier only** in Phase 1, validated end to end, before bulk-loading the other six.

- Load the 72 Soldier export nodes with `evidence_status = 'RECONSTRUCTION'`, `project_version = 'FINAL_V1'`.
- **Do not delete** the 28 seed-only abilities. Keep those rows with `evidence_status = 'LEGACY_SEED'` and no `project_version`. Deleting them would revoke abilities from live characters with no revocation path.
- Leave Soldier's `tree_index` assignment as-is where seed and export disagree on the Heavy Weapons / Command boundary, and record the conflict in a comment rather than picking a side (`SOURCE_POLICY.md:22`).
- Skip every node with `unlock_level_project_v1 > 20` until blocker 1 is decided, or load them and accept that they are unreachable — loading is the better option, since the rows are inert and the `level` value is the thing being reconstructed.
- Leave `ARCHETYPE_Any`, `ARCHETYPE_Sholva` and `ARCHETYPE_Jaffa` untouched.

### 6.7 What has to change in Rust alongside this

Not part of the schema, but the schema change is inert without it:

- `crates\services\src\cell\spawner\abilities.rs` / `load_archetype_ability_trees` — select the new columns into `ArchetypeAbilityTreeEntry`.
- `crates\services\src\cell\cell_methods\player\vendor\train.rs` — two new gates after gate 6: branch points satisfied, and skill points `>= skill_point_cost`. The pack's purchase-validation list (`SERVER_SCHEMA_PROPOSAL.md:56-67`) is otherwise already fully implemented by gates 1-6 plus the base-side guard.
- `crates\services\src\base\world_entry\methods\progression\mod.rs` — parameterise the debit by cost, write `sgw_player_ability` and `sgw_player_branch_points` in the same transaction as the array append. This turns a single guarded `UPDATE` into an explicit transaction; the `training_points >= cost` and `NOT (abilities @> ...)` guards must stay on the `UPDATE` itself, not move to application code, or the double-click protection is lost.
- `crates\services\src\mercury\world_data\stats.rs:45-88` — regenerate the hardcoded trees, or better, delete them and source the wire payload from the DB cache.
- `crates\services\src\cell\cell_methods\player\combat\mod.rs:124` — `resetMyAbilities` needs at least a minimal implementation (zero the new counters, clear `sgw_player_ability` and the array back to `char_creation_abilities`, refund `training_points_spent`) so the new counters have a reset path.

### 6.8 Test obligations

Per `CLAUDE.md` and `TESTING.md`, this change needs at least: a live-DB regression guard that the train transaction writes all four places atomically and rolls back as a unit; a live-DB guard that a double-click still debits once (the existing invariant, now across a transaction rather than one statement); a live-DB guard that the backfill is idempotent across two `database.sql` loads; and unit tests for the two new cell-side gates in `train.rs`. The live-DB tests run under the `ci-live-db` nextest profile, serialised, with `require_db_or_skip!` and exact-sentinel cleanup.
