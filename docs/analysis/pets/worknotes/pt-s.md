# PT-S Worknotes

> Type: reference. Audience: pets campaign coordinator (session cimmeria-b5).
> Companions: `README.md`, `work-packets.md` and `audit.md` in this directory (on the
> `docs/pets-campaign-plan` branch while the ledger is unmerged).

## Contract

- **Packet:** PT-S, the pet seed: the summon table, the first pet template and the summon VFX.
- **Scope change (coordinator, owner D-PT03 change):** the first pet is the **Straegis**, not the
  Jaffa. Template 350 is a Straegis Fighter pet summoned by 2826; the event sets go on 2826; the
  Jaffa (1643 -> 351, a clone of 160, name 8087) moves to PT-11.
- **Decisions in force:** D-PT03 as changed above, D-PT02 (owner's level at summon, no pet XP),
  D-PT04 (one active pet), D-PT06 (non-hostile, owner faction applied at spawn).
- **Base:** `origin/main` @ `95366c59`. Branch `pets/pt-s-seed`.
- **Owned paths:**
  - `db/resources/Entities/Tables/pet_summons.sql`, `db/resources/Entities/Seed/pet_summons.sql` (new)
  - `db/database.sql` (two `\ir` lines), `db/resources/_primary_keys.sql`, `db/resources/_foreign_keys.sql`
    (pet_summons blocks only)
  - `db/resources/Entities/Seed/entity_templates.sql` (template 350 only)
  - `db/resources/Abilities/Seed/abilities.sql` (row 2826 only)
  - `db/resources/Abilities/Seed/ability_sets.sql`, `ability_set_abilities.sql` (set 350 only)
  - `crates/cell-catalog/src/cell/spawner/pet_summons.rs` (new), `spawner/mod.rs`,
    `spawner/tests/live_db_pet_summons.rs` (new), `spawner/tests/mod.rs`
  - `crates/cell-world/src/cell/space_manager/mod.rs` (one field), `crates/cell/src/cell/service/startup.rs`
    (one loader call)
- **Read set:** ledger `work-packets.md` (contract, PT-S), `README.md` (D-PT03, D-PT06),
  `audit.md` (A-26, A-40..A-45), `research/content-inventory.md` sections 1, 1e, 3;
  `crates/cell-catalog/src/cell/spawner/{templates,abilities,respawners,mod}.rs`;
  `crates/cell-combat/src/cell/abilities/use_ability/sequence.rs`;
  `crates/resources/src/base/sequence_overrides.rs` and `resources/tests/committed_paks.rs` (#755);
  `data/cache/CookedData{Abilities,KismetSeqEvent,KismetSetEvent}.pak` (entries read with Python `zipfile`).

## Evidence

1. **No original summon binding.** 2826 and its siblings have `effect_ids = {}` in the seed and
   in the cooked ability data (A-26). The table is new seed data.
2. **Event sets are server-only for abilities.** No `COOKED_ABILITY` entry in
   `CookedDataAbilities.pak` carries an event set attribute (0 of 1,886 mention `Event`). The
   server resolves `(event_set_id, event_id) -> sequence_id` itself
   (`spawner/abilities.rs` `load_event_set_sequences`) and sends only the sequence id in
   `onSequence`; the client resolves the id through its own `CookedDataKismetSeqEvent.pak`
   (`sequence_overrides.rs` module doc). Sequences 2292, 2293 and 2904 are all present in the
   shipped PAK with their PFX NVPs, and sets 1121/1122 are in `CookedDataKismetSetEvent.pak`.
   **So no cooked-data override is needed**; setting `abilities.event_set_id = 1121` on 2826 is
   enough.
3. **1121 vs 1122.** An ability row has one `event_set_id`. 1121 "Goauld summon source" holds
   Ability_End (1001) -> 2292 `PFX-GoauldSummon` on the caster's `Buff` socket and
   Ability_Interrupt (1002) -> 2904 interrupt sound. 1122 "Goauld summon target" holds only
   Effect_Init (2000, `entities/defs/enumerations.xml:774`) -> 2293 `PFX-GoauldSummonTarget` on
   the ground. 1122 is a target (effect-level) set and 2826 has no effect, so it cannot sit on the
   ability row. It stays unwired in the seed; PT-03 plays sequence 2293 at the spawned pet
   (source = owner, target = pet). The guard pins that `(1122, 2000) -> 2293` resolves.
4. **Name.** `texts` 28894 `DN_Pet_Straegis_Tier_1` has empty text
   (`db/resources/Texts/Seed/texts.sql:40628`). 27377
   `DN_Mb_Ms_Agnos_Summoned_Straegis_Fighter_Force_43` = "Summoned Straegis Fighter"
   (`texts.sql:37355`) is used. 78's own 8090 "Straegis Fighter" was the other option; 27377 says
   it is summoned, which reads right for a pet.
5. **Kit.** All three Straegis mob abilities deal no damage today: effects 1322, 4138, 1397/1398
   have no damage NVP and no script (`damage_apply/mod.rs:131-152` per combat-systems-advisor).
   - 1240 Straegis Explode: cooldown 0 and event set 1507 "Straegis death ability source"; as the
     lowest-id fallback it would play the death burst on a living pet every AI tick. **Excluded.**
   - 2847 Dissonance: `is_ranged = false` (3 m reach) and target type 1 (self); the AI would cast it
     at its enemy and walk into melee. **Excluded.**
   - 1156 Disengage: `is_ranged = true`, `max_range 0` resolves to the 30 m default, event set
     1499 -> sequence 2824 (Straegis-native `sEmitAura`). Kept as cosmetic filler.
   - **221 Energy Shock** added as the primary (the drone's ability, set 2): ranged, cd 2, effect
     264 `HealthDamage` 16, event set 802 -> sequence 1866 beam; no humanoid animation. Both
     advisors (combat-systems, npc-ai-spawn) recommended exactly this set.
   - New set id **350** (pet sets take 350-369 like the templates, so they cannot collide with
     other campaigns' low ids). `ability_sets2_ability_set_id_seq` stays at 6.
6. **Summon duplicates.** 3491/3493/3495/3496/3497 are "Summon Straegis" copies with warmup 0;
   none is in `archetype_ability_tree` or `trainer_abilities` (2826 is: tree L50 capstone, trainer
   list 1). They get no row.
7. **Template 350 columns.** Clone of 78 (`MOB_StraegisFighter` body set, component, event set 570,
   skin tint -52773120). class `pet`, flags 1032 (`ENTITYFLAG_Pet` 1024 | `NoPetLeveling` 8),
   faction 1 (78 is 10), level 1 (placeholder the summon overwrites; NULL/1 = 250 HP for a GM or
   content spawn, advisor), `move_speed` 0.9 (Col Marsh escort precedent), `use_cover` false,
   `respawn_secs` / `loot_table_id` NULL, `interaction_type` 0. Visual chain exists in the seed
   (`skeletal_meshes.sql:3631`, `body_sets.sql:51` eye height 2.65), but **no `MOB_` template has
   ever been placed by `spawnlist`**, so the render path is unproven.
8. **Unknown class `pet`.** `class_id_for_class` falls through to 0x04 (`crates/wire/src/cell/spawn_record.rs:134`)
   until PT-01 adds the arm. The full live-DB tier (3,548 tests) passes with the row, including
   the NA43 animation linter that scans every template with an ability set.

## Design decisions

- **Loader shape.** `PetSummonCatalog` (a wrapped `HashMap<i32, PetSummon>`) with
  `pet_summon_for(ability_id) -> Option<PetSummon>`, loaded by `load_pet_summons(pool)` and stored
  in `SpaceManager::pet_summons`, the `spawn_templates` pattern. `PetSummon` is `Copy`. A load
  failure logs an ERROR and leaves the catalog empty (every summon then fails as today).
- **Keys.** PK in `_primary_keys.sql`, FKs to `abilities` and `entity_templates` (RESTRICT) in
  `_foreign_keys.sql`, `CHECK (max_active >= 1)` in the table file, per the repo's split-file
  schema convention.
- **Flags.** The advisor flagged that `NoPetLeveling` could read as contradicting D-PT02's
  "owner's level at summon". Kept per the packet: D-PT02 also says pets do not level on their own,
  which is what the flag names. The seed comment records it.
- **`PetUseOwnFaction` (65536)** was not set; D-PT06 is applied in code by PT-01. Coordinator's call
  whether to add it.

## Commands run

All from the worktree root. The B: Dev Drive ran out of space mid-session (another session's
targets), so the later runs exported `CIMMERIA_TARGET_ROOT` to a C: scratch dir.

| Command | Exit | Result |
|---|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell-catalog -p cimmeria-cell-world -p cimmeria-cell --all-targets` | 0 | clean |
| `bash tools/build-lane/live-db-test.sh "::"` (first try) | 101 | B: "no space on device"; not a test result |
| `bash tools/build-lane/live-db-test.sh "::"` (C: target) | 0 | 3548 run, 3548 passed, **0 skipped** |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-cell-catalog -p cimmeria-cell-world -p cimmeria-cell --all-targets -- -D warnings` | 0 | clean |
| `bash tools/build-lane/live-db-test.sh live_db_pet_summons` (seed rows removed) | 100 | 4 run, **4 failed** |
| `bash tools/build-lane/live-db-test.sh live_db_pet_summons` (template 350 placed in spawnlist) | 100 | 4 run, 3 passed, **1 failed** (`[(450, 350)]`) |
| `bash tools/build-lane/reload-db.sh` | 0 | DB restored to the committed seed |

Tests (cimmeria-cell-catalog):

- unit: `cell::spawner::pet_summons::tests::{pet_summon_for_finds_the_row_by_ability_id, pet_summon_for_misses_a_non_summon_ability, empty_catalog_summons_nothing}`
- live-DB: `cell::spawner::tests::live_db_pet_summons::live_db::{every_pet_summon_points_at_a_pet_template, pet_templates_are_never_placed_in_spawnlist, summon_straegis_carries_the_goauld_summon_event_set, straegis_pet_template_is_the_straegis_fighter_body}`

## Regression proof

1. Restored `entity_templates.sql`, `abilities.sql`, `ability_sets.sql` and
   `ability_set_abilities.sql` to their pre-PT-S versions and deleted the `pet_summons` INSERT:
   all four live-DB guards failed (lines 65, 136, 214 and 183 of `live_db_pet_summons.rs`).
2. With the seed restored, appended a `spawnlist` row placing template 350:
   `pet_templates_are_never_placed_in_spawnlist` failed naming `(450, 350)`; the other three passed.
3. Restored with `git checkout HEAD -- <file>` and reloaded the DB.

## Known gaps and hand-offs

- **1122 target VFX:** PT-03 plays sequence 2293 (`(1122, Effect_Init 2000)`) at the new pet.
- **Straegis render unproven:** no `MOB_` body has ever been spawned by our server. Do a GM
  `.spawn 350` (or `.spawn 78`) smoke test in the client before PT-03's UAT. Also check whether
  the Energy Shock beam (sequence 1866, no socket NVP) has a socket to come from on this body,
  and whether the run animation foot-slides at `move_speed` 0.9.
- **The Straegis kit does no special damage:** 1156 is a 0-damage hit and does not knock down
  (nothing reads effect flag 2052 as a knockdown). If PT-05/PT-11 want the real Straegis moves,
  that is effect-script work.
- **Summon Straegis is the L50 Servant Lord capstone** (A-40): UAT needs a level-50 Goa'uld or a
  GM grant of 2826.
- **`entity_templates_template_id_seq`** stays at 304 (no one uses `nextval` for templates; the
  crafting and guild packets touch the same line).

## Integration edits for the coordinator

- `db/resources/Entities/Seed/entity_templates.sql`, `abilities.sql` (row 2826),
  `ability_sets.sql`, `ability_set_abilities.sql`, `db/database.sql`, `_primary_keys.sql` and
  `_foreign_keys.sql` are shared with crafting (310-329) and guilds (330-349): expect textual
  conflicts at the same anchors (end of the templates file, before the sequence `setval`); keep
  both sides.
- `crates/cell-world/src/cell/space_manager/mod.rs`: PT-01 adds `PetRegistry` next to the new
  `pet_summons` field; both are plain field + constructor lines.
- Ledger: record the D-PT03 change (Straegis first, Jaffa to PT-11 as 1643 -> 351), set 350, and
  the name choice (27377, not 28894).
- `docs/gameplay/pet-system.md` still says no pet templates exist; PT-13 should update it.
