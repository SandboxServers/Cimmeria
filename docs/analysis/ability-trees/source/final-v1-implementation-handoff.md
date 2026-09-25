# Cimmeria / Stargate Worlds — ALL CLASS ABILITY TREES FINAL v1
## Claude Code implementation handoff

### 0. Read this first

This packet is a **server implementation handoff** for the current project-final Stargate Worlds class progression.

The authoritative source for tree content is:

`source/SGW_All_Classes_Progression_Final_v1.xlsx`

SHA-256 is recorded in `MANIFEST.json`.

The machine-readable exports in `data/` are generated directly from that workbook. Do not hand-edit the CSV/JSON exports and then claim they are canonical; if data changes are needed, change the canonical workbook/project decision first and regenerate the exports.

### Authority rule

For **class-tree membership, branch structure, node order, unlock levels, project prerequisites, branch-point gates and project-v1 cost**, this workbook is the current PROJECT FINAL v1 authority.

The repo already contains an older `docs/analysis/sgw-handoff-pack-v1.2/` audit. Keep its code/schema/runtime findings where still true, but **do not let older handoff tree-data decisions override this newer workbook**.

Known example: the old Phase-0 report calls Free Jaffa `Tau'ri` a fourth branch and recommends deferring it. That is stale relative to this workbook. FINAL v1 has exactly three Free Jaffa/Shol'va branches:

1. Heritage
2. Tau'ri
3. Tactics

Do not create a fourth branch.

---

# 1. Canonical FINAL v1 dataset

- 7 archetypes
- 21 branches
- 439 purchasable tree nodes
- 419 unique ability IDs
- 19 ability IDs intentionally appear in more than one archetype/tree
- 280 reserve/review candidates are preserved but are NOT part of the purchasable FINAL v1 tree
- historical/project target level cap: 50
- project unlock breakpoints: 1 / 5 / 10 / 15 / 20 / 25 / 30 / 35 / 40 / 45 / 50
- project-v1 node cost: 1 skill point per node
- 59 promoted nodes have raw `Training Cost = 0`; preserve that source fact and warn/flag it, do not silently rewrite the source field
- one capstone per branch at level 50
- branch-point gates: 0 -> 20 by tier
- exact unlock breakpoints, branch-point gates, skill-point price, node ordering and prerequisite graph are PROJECT FINAL v1 unless a row explicitly carries stronger source evidence

## Branch -> current `tree_index` mapping

Map the three workbook branches to the server's existing `tree_index 0..2` in workbook order.

| Archetype | tree_index 0 | tree_index 1 | tree_index 2 |
|---|---|---|---|
| Soldier | Automatic Weapons | Heavy Weapons | Command |
| Commando | Demolitions | Stealth / Infiltration | Precision / Marksmanship |
| Scientist | Medical | Support | Robotics |
| Archaeologist | Anthropology | Sociology | Archaeology |
| Asgard | Attack Programs | Defensive Programs | Scientific Programs |
| Free Jaffa / Shol'va | Heritage | Tau'ri | Tactics |
| Goa'uld | Ashrak | Battle Lord | Servant Lord |

`data/branch_index_map.csv` is the machine-readable form.

### Branch authority

Do not flatten the evidence labels:

- Soldier: all three branches source-backed.
- Commando: Demolitions source-backed; Stealth/Infiltration and Precision/Marksmanship are strong reconstruction.
- Scientist: Medical and Support source-backed; Robotics secondary-evidence strong.
- Archaeologist: Anthropology and Sociology source-backed; Archaeology secondary-evidence strong.
- Asgard: all three source-backed.
- Free Jaffa/Shol'va: branch names survive in secondary trainer material; membership is reconstructed, Tau'ri is revision-heavy.
- Goa'uld: branch names are secondary but strongly corroborated by mechanics/historical material; membership/order remains project reconstruction where the row says so.

Do not present PROJECT FINAL values as recovered retail facts.

---

# 2. Current Cimmeria runtime — do not rebuild what already exists

This handoff was built against repository main:

`acbcc22e7adc002ebb1a857a94fc64431adf6a4e`

Before implementing, re-open the current versions of the paths below because main may have advanced.

The existing trainer runtime already has most of the transport/purchase loop:

- client `trainAbility(INT32 ability_id)`
- cell-side tree / level / prerequisite validation
- Cell -> Base training request
- atomic training-point debit + persistence
- Base -> Cell `AbilityGranted`
- `onKnownAbilitiesUpdate`
- trainer reopen/refresh
- `onTrainerOpen`
- `onAbilityTreeInfo`
- `sgw_player.training_points`

Relevant current code/doc areas:

- `crates/services/src/cell/cell_methods/player/vendor/train.rs`
- `crates/services/src/cell/interactions/trainer.rs`
- `crates/services/src/base/world_entry/methods/progression/`
- `crates/services/src/cell/service/base_messages/ability_granted.rs`
- `crates/services/src/cell/spawner/abilities.rs`
- `crates/services/src/mercury/world_data/stats.rs`
- `db/resources/Archetypes/Tables/archetype_ability_tree.sql`
- `db/resources/Archetypes/Seed/archetype_ability_tree.sql`
- `db/resources/Abilities/Seed/trainer_abilities.sql`
- `docs/analysis/sgw-handoff-pack-v1.2/audits/audit-trainer-runtime.md`
- `docs/analysis/sgw-handoff-pack-v1.2/phase0-gap-report.md`

Do **not** create a parallel skill-tree subsystem.

Adapt the existing `archetype_ability_tree` + trainer path.

---

# 3. Current gaps that this packet is intended to close

At the repo head used to build this handoff, the major trainer/tree gaps are:

## 3.1 Tree data is still a stub

Current seed:
- only Soldier + Commando;
- 169 rows;
- all unlock level 1;
- prerequisite arrays empty.

FINAL v1:
- 439 nodes;
- seven archetypes;
- real project unlock levels and prerequisite graph.

Import the FINAL v1 data into the existing model after schema support lands.

## 3.2 Client ability tree is hard-coded separately

`mercury/world_data/stats.rs` currently has a literal Soldier/Commando ability-tree array.

This is dangerous.

The client UI and server purchase validation must be generated from the **same authoritative loaded tree data**.

Delete/retire the hand-copied duplicate after the data-driven replacement is proven.

Acceptance:
- one source of truth feeds both trainer validation and `onAbilityTreeInfo`;
- all supported archetypes receive exactly 3 branches;
- node order matches FINAL v1.

## 3.3 Required branch points are not representable

FINAL v1 uses:
`required_branch_points = 0,2,4,...,20`

The current tree has no such field and the player has no authoritative per-branch spend count.

Implement this server-authoritatively.

Do not infer branch points from every currently-known ability because known abilities can come from starter/mission/system/weapon paths.

Preferred direction:
- persist trainer-spent branch points per player + archetype + tree index, OR
- persist learned-ability provenance strongly enough that branch spend can be reconstructed safely.

The debit + ability grant + branch-point increment must be atomic.

Do not spend a training point successfully and then fail to record branch progression.

## 3.4 Per-node project cost is not represented

FINAL v1 `skill_point_cost = 1` for every current node.

Add node-level `skill_point_cost` to the tree model rather than permanently relying on the current hard-coded `-1`.

For this version it will always be 1, but the validator and atomic debit should consume the node value.

Preserve `abilities.training_cost` as source data.

When a purchased FINAL-v1 node has raw/source `training_cost == 0`, emit a structured WARN/metadata signal because 59 promoted nodes are explicitly marked VERIFY.

Do NOT mutate source `training_cost 0 -> 1` just to make the project economy work.

## 3.5 Trainer interaction authority hole

A forged `trainAbility` packet must not allow training from anywhere.

Before purchase:
- verify `last_interaction_target`;
- verify it is a real trainer template;
- verify trainer list offers the ability for the player's archetype;
- verify same space / sensible interaction distance using existing interaction policy.

Do not rely only on tree membership.

## 3.6 Training-points UI becomes stale after purchase

After Base confirms the atomic debit, push the updated TrainingPoints entity property to the client.

The UI must show the new remaining point total immediately without relog/level-up.

## 3.7 Level cap mismatch

Current repo:
`MAX_LEVEL = 20`

FINAL v1 contains 243 nodes above level 20.

**Do not invent an XP curve for levels 21-50 in this packet.**

Import all nodes now.
Nodes above the current cap may exist in the tree and remain untrainable through the already-existing level gate.

A level-50 progression patch is a separate decision because:
- historical cap 50 is supported;
- exact 21-50 XP thresholds are not supplied by this workbook;
- changing `MAX_LEVEL`, XP arrays and client world-data level tables touches more than the trainer.

For QA only, a controlled GM/debug path may be used to test >20 tree gating/capstones without declaring the XP curve complete.

## 3.8 Skill-point earn rate

Current Cimmeria gives 2 training points per level.

The workbook does not recover the original point-gain economy.

Leave the current earn rate untouched in this packet.
Label it reconstruction/unresolved.

## 3.9 Free Jaffa / Shol'va enum split

The workbook contains one archetype:
`Free Jaffa / Shol'va`

The repo has:
- `ARCHETYPE_Sholva = 7`
- `ARCHETYPE_Jaffa = 8`

For this implementation campaign, map FINAL-v1 Free Jaffa/Shol'va tree data to **ARCHETYPE_Sholva (7)** only and leave `ARCHETYPE_Jaffa (8)` untouched unless the owner explicitly changes that decision.

Important:
the three branches are exactly:
- Heritage
- Tau'ri
- Tactics

The older handoff statement that Tau'ri is a fourth branch is superseded.

## 3.10 Shared ability IDs are valid

Do not add a global uniqueness constraint on `ability_id` for tree membership.

There are 439 nodes but 419 unique ability IDs.

19 ability IDs are intentionally shared across archetypes.

See:
`data/shared_ability_ids.json`

Tree identity must remain at least:
`(archetype, tree_index, node_order)`

Ability ID alone is not a tree-node primary key.

---

# 4. Data mapping

Use `data/trainer_server_export.json` as the minimal import source.

Fields:

- `archetype`
- `branch`
- `node_order`
- `unlock_level`
- `required_branch_points`
- `skill_point_cost`
- `ability_id`
- `primary_prereq_ability_id`
- `additional_prereq_ability_ids`
- `is_branch_root`
- `is_capstone`
- `raw_training_cost`
- `project_status`

Suggested mapping to existing `resources.archetype_ability_tree`:

- workbook archetype -> `archetype`
- branch -> `tree_index` via `data/branch_index_map.csv`
- node_order -> `ability_index`
- ability_id -> `ability_id`
- unlock_level -> `level`
- primary + additional prereqs -> existing `prerequisite_abilities[]`
- required_branch_points -> new column
- skill_point_cost -> new column
- is_branch_root -> new metadata column
- is_capstone -> new metadata column
- project/evidence status -> new metadata columns if useful for operator tooling
- branch display name -> preserve in DB/data model if it can be done without fighting client-owned naming

Do not create a new normalized prereq table unless current architecture genuinely requires it. The existing prerequisite array is a good fit for runtime checks.

### Additional prerequisite format

When importing:
- `0` / null primary means no primary prerequisite;
- parse `additional_prereq_ability_ids` carefully;
- produce a deterministic deduplicated prerequisite array;
- never include ability id 0;
- fail seed/test if a referenced prerequisite is missing from `resources.abilities`.

---

# 5. Import safety

Before replacing old tree seed data:

1. Verify every FINAL-v1 ability ID exists in `resources.abilities`.
2. Produce the count:
   - 439 nodes
   - 419 unique ability IDs
   - 21 branches
3. Verify exactly three branch indices per archetype.
4. Verify one and only one branch root per branch.
5. Verify one and only one capstone per branch.
6. Verify every capstone is level 50.
7. Verify node orders are unique and contiguous inside each branch.
8. Verify unlock levels are from the allowed FINAL-v1 breakpoint set.
9. Verify no prerequisite points forward to an impossible/missing dependency.
10. Verify a node never requires itself.
11. Verify there is no prerequisite cycle.
12. Verify required branch points never decrease as progression advances where the data intends monotonic tiers.
13. Preserve the 280 reserve candidates separately; do not seed them as purchasable nodes.

Do not silently delete abilities already known by existing characters.

Existing character compatibility must be handled independently from tree membership migration.

---

# 6. Trainer offering

For the first integration/UAT pass, use the existing trainer architecture.

A single debug trainer/list may offer the full applicable FINAL-v1 tree per archetype if that is the cheapest way to verify the system.

Do not spend showcase budget creating every world trainer before the core tree purchase flow works.

After the system is proven:
- bind real trainer NPCs/lists where source-backed content exists;
- keep offering validation server-authoritative.

---

# 7. Client tree contract

The abilities/trainer UI must show the same data the server validates.

Acceptance for each supported archetype:

- exactly 3 branch arrays;
- ability IDs appear in workbook node order;
- unavailable nodes are present but unavailable/grayed;
- learned abilities are reflected after purchase;
- lower progression/prerequisites gate higher nodes;
- branch-point gate is reflected in server trainability;
- training point count refreshes immediately.

If the client protocol cannot represent a specific metadata field directly, server-side validation still remains authoritative.

Do not alter cooked client assets unless a real client limitation is proven.

---

# 8. Runtime ability mechanics are a separate campaign

This packet is about:
- tree content;
- trainer visibility;
- purchase validation;
- progression persistence.

Do not attempt to rewrite all 439 ability mechanics as part of the tree import.

Every imported ability ID must exist, but "exists in resources.abilities" is not the same as "mechanically perfect in the client."

Keep runtime-mechanics defects as separate, ability-specific work.

For showcase QA, choose a small sample per relevant archetype:
- branch root;
- early prerequisite-dependent node;
- mid-tree node;
- cross-shared ability where useful;
- capstone via controlled debug level if needed.

---

# 9. Showcase priority / Claude-budget rule

We have limited Claude budget.

Implement the common infrastructure once.

Priority for real-client UAT:
1. Soldier
2. Commando
3. Scientist
4. Archaeologist
5. Free Jaffa / Shol'va

Seed Asgard and Goa'uld FINAL-v1 data so the model is complete, but do not burn showcase time on world-specific trainer placement or extensive gameplay UAT for them while their maps/content are outside the current playable showcase scope.

Do not re-audit all 439 abilities before importing the trees.

The canonical workbook already made the project progression selection.

Only investigate a specific node when:
- import validation fails;
- runtime breaks;
- client cannot display/train it;
- a prerequisite or duplicate relationship is ambiguous.

---

# 10. Recommended implementation packets

## AT-01 — Schema + source loader

Adapt `archetype_ability_tree` to express:
- required branch points;
- skill-point cost;
- root/capstone/project metadata as needed.

Create deterministic import/generation from `data/trainer_server_export.json` or convert it into committed SQL seed data following repo conventions.

Acceptance:
- fresh DB load contains 439 FINAL-v1 node rows under the approved archetype mapping;
- all validators pass.

## AT-02 — One source of truth for client + server

Replace the hard-coded Soldier/Commando ability arrays in `world_data/stats.rs`.

Generate `onAbilityTreeInfo` from the same tree cache/data used by training validation.

Acceptance:
- client and server cannot disagree because of duplicate hard-coded lists.

## AT-03 — Branch-point authority

Add persistent per-branch spend tracking.

On successful trainer purchase:
- validate current branch points;
- atomically debit skill points;
- grant ability;
- increment correct branch spend.

Acceptance:
- forged out-of-order capstone purchase is rejected;
- valid progression increments exactly once;
- double-click/replay cannot double-increment.

## AT-04 — Trainer authority + UI refresh

- validate active trainer interaction/offering/proximity;
- push TrainingPoints update after purchase;
- preserve trainer refresh.

Acceptance:
- train packet from across the world rejected;
- legitimate trainer purchase works;
- point display decrements immediately.

## AT-05 — FINAL-v1 seed import

Import all approved nodes.
Do not seed reserve/review candidates.

Free Jaffa mapping:
- Sholva 7 only.
- Heritage/Tau'ri/Tactics = indices 0/1/2.

Acceptance:
- 439 source nodes accounted for in import plan;
- actual row count may differ only if one archetype is intentionally not loaded due the explicit Sholva/Jaffa mapping decision, and that difference must be documented, never silent.

## AT-06 — Real-client UAT

Test at least Soldier first, then the other showcase archetypes.

For one branch:
- buy root;
- attempt gated node too early -> reject;
- satisfy prerequisite -> buy;
- verify branch points;
- verify client known-abilities update;
- verify point count;
- relog;
- reopen trainer;
- learned state and points persist.

Cross-branch:
- branch points in branch A must not satisfy a gate in branch B.

Replay:
- repeat same `trainAbility` packet -> no second debit.

Security:
- train without trainer interaction -> reject.

---

# 11. Required tests

At minimum add automated guards for:

- exact branch count = 3 for each imported archetype;
- exact FINAL-v1 node count by archetype:
  - Soldier 72
  - Commando 64
  - Scientist 59
  - Archaeologist 65
  - Asgard 66
  - Free Jaffa/Shol'va 51 in source
  - Goa'uld 62
- source total = 439;
- unique source ability ids = 419;
- 19 shared IDs accepted;
- one root and one capstone per branch;
- capstone level 50;
- level gate;
- prerequisite gate;
- additional-prerequisite gate;
- branch-point gate;
- skill-point gate;
- archetype gate;
- trainer-offering gate;
- trainer-interaction/proximity gate;
- duplicate/replayed purchase no-op;
- atomic skill debit + ability grant + branch increment;
- TrainingPoints client refresh;
- relog persistence;
- DB tree and `onAbilityTreeInfo` order agree.

---

# 12. Do not do

Do not:

- create a second skill-tree system;
- hard-code 439 nodes into Rust;
- retain a separate hard-coded client tree array;
- assume ability ID is globally unique to one class;
- convert raw `Training Cost 0` to source cost 1;
- invent level 21-50 XP thresholds;
- expand Free Jaffa to four branches;
- map the same Free-Jaffa dataset to both Sholva and Jaffa without owner approval;
- seed reserve/review abilities as normal nodes;
- revoke existing character abilities just because the new tree changed;
- spend this packet rebuilding ability combat formulas;
- spend the limited showcase budget on Asgard/Goa'uld trainer placement/UAT.

---

# 13. Definition of done for this handoff

The tree implementation is done when:

1. the current server loads FINAL-v1 data instead of the two-class level-1 stub;
2. client tree info and server purchase validation use the same data;
3. Soldier, Commando, Scientist, Archaeologist and Free Jaffa/Shol'va can each see three correct branches;
4. all 439 source nodes are accounted for and the supported archetype rows are seeded deterministically;
5. unlock level, prerequisites, branch points and skill-point cost are validated server-side;
6. purchases require a valid trainer interaction;
7. purchase persistence is atomic and replay-safe;
8. training points refresh instantly in the client;
9. learned abilities survive relog;
10. no >20 XP curve was invented just to make level-50 nodes reachable;
11. Asgard/Goa'uld tree data can exist without consuming current showcase UAT budget;
12. every deviation from the workbook is explicitly documented and owner-approved.

---

# 14. Files in this packet

## Canonical
- `source/SGW_All_Classes_Progression_Final_v1.xlsx`

## Minimal server import
- `data/trainer_server_export.csv`
- `data/trainer_server_export.json`
- `data/branch_index_map.csv`

## Full evidence/detail
- `data/all_final_nodes.csv`
- `data/all_final_nodes.json`
- per-class CSVs
- `data/branch_start_end.csv`
- `data/progression_rules.csv`
- `data/sources.csv`

## Safety/reference
- `data/shared_ability_ids.json`
- `data/reserve_review.csv`
- `MANIFEST.json`

When there is a conflict between an older tree-data handoff in the repo and this canonical workbook, stop, cite the conflict, and use this FINAL-v1 workbook unless the owner gives a newer decision.
