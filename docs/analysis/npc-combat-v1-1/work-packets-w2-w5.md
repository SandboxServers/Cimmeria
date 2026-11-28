# NPC combat V1.1: work packets, waves 2-5

Ledger: [README.md](README.md). The dispatch rules, rules R1-R12 and the contract C1-C7 are in [work-packets.md](work-packets.md); every brief below inlines the rules it names. Packet sections in wave 2 are **provisional**. NX-10 (the kit spec) confirms or amends them in `kit-spec.md`, and the orchestrator dispatches no wave-2 packet before NX-10 lands and its owner decisions are answered.

## Gate: NX-10 Titan kit spec (re-slice checkpoint)

- **Agent:** architect. **Host:** windows. **Depends:** NX-04..09 merged. D-NX3, D-NX5 and D-NX6 are answered, or the spec marks what depends on them.
- **Branch:** `npc-combat-v1-1/NX-10-kit-spec`. **Worktree:** `nx-10`.
- **Commit:** `docs(npc-combat): Straegis Titan kit spec (NX-10)`
- **Files:** new `docs/analysis/npc-combat-v1-1/kit-spec.md`; README (the decisions, findings and packet rows); this file (amended packet sections, if needed); RESUME.md.
- **Do:**
  1. Read the four NX-04..09 findings docs and the runtime trace.
  2. Settle the kit table in README § Provisional Titan kit, row by row, with a class for each row.
  3. Fix the exact values the seed packets need:
     - per ability: name, range, cooldown, cast time, target type, effect script (`MeleePhysicalDamage`, `RangedPhysicalDamage` or an area script) and damage NVPs, RECONSTRUCTED for level 40-42 against the V1 effect 736 and 5310 scale;
     - per sequence: id, event id, KIS name and NVP list (C3 shape);
     - event sets and the ability-to-event-set link.
  4. Decide F11 (a) or (b) from NX-04 and NX-07. Under (a), NX-14 becomes a test-only packet.
  5. Re-slice wave 2 if a packet would pass 40k working tokens, and keep every file owned by one packet.
- **Lane:** none. Run R6 on every file.

---

## Wave 2: Titan implementation (7 packets, one integration PR)

- **Ships as one PR** from the integration branch `npc-combat-v1-1/nx-w2` (worktree `nx-w2`, cut from `origin/main` when the wave starts). This is D-NX12: a Titan set never lands without its presentation.
- `ops` merges each packet branch into `nx-w2` with `merge-noff.ps1`, after that packet's own review loop returns `ship`. **Merge order:** NX-11, NX-12a, NX-12b, NX-13, NX-14, NX-15, NX-16.
- **Dispatch:** NX-11, NX-12a, NX-14 and NX-15 start together. NX-12b starts from `nx-w2` once NX-12a is merged into it. NX-13 starts once NX-11 is in, and NX-16 once NX-13 is in.
- The wave review runs on `nx-w2`. Its fix notes go to one fresh packet-coder in `nx-w2`. After merge, `ops` retires the packet worktrees with `rm-worktree.ps1 --merged-into <PR> nx-11 nx-12a nx-12b nx-13 nx-14 nx-15 nx-16`.
- **File ownership (no two packets share a file):**

| Packet | Owns |
|---|---|
| NX-11 | `Abilities/Seed/abilities_straegis_titan.sql`, `Effects/Seed/effects_straegis_titan.sql`, `Effects/Seed/effect_nvps_straegis_titan.sql`, their `\ir` lines, `creatures/titan_kit.rs` |
| NX-12a | `Events/Seed/straegis_titan_events.sql`, its `\ir` line, `creatures/titan_presentation.rs` |
| NX-12b | `crates/resources/src/base/sequence_overrides.rs` |
| NX-13 | `Abilities/Seed/ability_sets_straegis_titan.sql`, its `\ir` line, the template 79 row in `Entities/Seed/entity_templates.sql`, `Loot/Seed/loot_debug_area_creatures.sql` and its `\ir` line, spawn 14320's `loot_table_id` in `spawnlist_debug_area_creatures.sql`, `creatures/titan_set.rs` |
| NX-14 | the stance or phase code NX-10 names, `creatures/stance.rs` |
| NX-15 | the tasker code NX-10 names, `creatures/tasker.rs` |
| NX-16 | `creatures/titan.rs`, the registry's Titan runtime rows, `docs/content/debug-area.md` Z13 rows |

- **`db/database.sql`:** NX-11, NX-12a and NX-13 each add `\ir` lines in a different region, so the merges are clean:
  - NX-11 after line 281 (abilities) and after line 310 (effects; the NVP file goes after the effects file);
  - NX-12a after line 356 (`debug_area_ring_events.sql`);
  - NX-13 after line 284 (`ability_sets.sql`) and after line 377 (`loot_tables.sql`).

  The line numbers are approximate, so match on the neighbouring file name.

Every wave-2 brief inlines R1-R8, R10 and R11, and these seed rules:

- Copy the column order from an existing row of the same table. The patterns are ability 221 Energy Shock with effect 264 (PRU, V1 NC-11), and NC-12's effect 736 NVPs.
- Every new row is RECONSTRUCTED and carries the C5 comment.
- Never edit a row outside C2, except template 79 (NX-13).
- Reload the worktree DB with `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which rebuilds it from the seeds. Load the `seed-change` skill.

### NX-11 Titan abilities, effects and NVPs

- **Agent:** packet-coder. **Host:** windows. **Depends:** NX-10, D-NX3.
- **Branch:** `npc-combat-v1-1/NX-11-titan-abilities`. **Worktree:** `nx-11`.
- **Commit:** `feat(seeds): Straegis Titan abilities and effects, RECONSTRUCTED (NX-11)`
- **Files:**
  - the three new seed files from the ownership table, and their `database.sql` lines;
  - `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/creatures/titan_kit.rs`.
- **Do:**
  1. Add abilities 3500-3503, exactly as in kit-spec.md (name, range, cooldown, target type, event-set link).
  2. Add effects 9170 and up, and effect NVPs 480 and up (HealthDamage, FocusDamage and any area radius), exactly as in kit-spec.md.
- **Tests:** in `titan_kit.rs` `mod live_db`, `live_db_titan_abilities_resolve`. It loads abilities 3500-3503 through the same loader the cell uses (copy from the V1 `reference/` tests), then asserts:
  - each ability exists, with its kit-spec range and cooldown;
  - each damaging ability's effect has the kit-spec script name and NVP values;
  - no Titan ability names effect 5310 or ability 592.
- **Revert proof:** dropping any one seed file fails the test.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell --all-targets -- -D warnings`
  3. `pwsh tools/build-lane/live-db-test.ps1 titan_abilities`
  4. `pwsh tools/build-lane/live-db-test.ps1 seed_sequences`

### NX-12a Titan event sets and sequences

- **Agent:** packet-coder. **Host:** windows. **Depends:** NX-10.
- **Branch:** `npc-combat-v1-1/NX-12a-titan-events`. **Worktree:** `nx-12a`.
- **Commit:** `feat(seeds): Straegis Titan event sets and sequences (NX-12a)`
- **Files:**
  - new `db/resources/Events/Seed/straegis_titan_events.sql`, modelled on `debug_area_ring_events.sql`: `event_sets`, `event_sets_sequences`, `sequences` and `sequences_nvp` rows in one file, in that order;
  - its `database.sql` line;
  - `creatures/titan_presentation.rs`.
- **Do:**
  1. Add event sets 1550 and up, and sequences 3030 and up, exactly as kit-spec.md lists them.
  2. Write each NVP row as a seed `sequences_nvp` row with the same Name and Value that NX-12b puts in the override.
- **Tests:** in `titan_presentation.rs` `mod live_db`, `live_db_titan_event_sets_load`:
  - every Titan ability's event set loads with the kit-spec sequence ids;
  - each sequence's `kismet_script_name` and NVP pairs match kit-spec.md;
  - no Titan sequence id collides with a cooked `CookedDataKismetSeqEvent` key. Read the keys from `data/cache/CookedDataKismetSeqEvent.pak` with the `zip` crate, as `sequence_overrides.rs` tests do.
- **Revert proof:** dropping the file fails the test.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell`
  3. `pwsh tools/build-lane/live-db-test.ps1 titan_event_sets`
  4. `pwsh tools/build-lane/live-db-test.ps1 seed_sequences`

### NX-12b Titan sequence overrides

- **Agent:** packet-coder. **Host:** windows. **Depends:** NX-02 merged on main; NX-12a merged into `nx-w2`.
- **Branch:** `npc-combat-v1-1/NX-12b-titan-overrides` (from `nx-w2`). **Worktree:** `nx-12b`.
- **Commit:** `feat(resources): Straegis Titan Kismet sequence overrides (NX-12b)`
- **Files:** `crates/resources/src/base/sequence_overrides.rs` only. If the file passes 500 lines, move the Titan entries to a new `sequence_overrides/straegis_titan.rs` and say so in the report.
- **Do:**
  1. Add one `SequenceOverride` per sequence 3030 and up, with the C3 `nvps` field, copied from kit-spec.md.
  2. Add a module doc line saying the entries are RECONSTRUCTED pairings of original Titan assets (README F9, D-NX3).
- **Tests:** the existing `every_override_matches_its_seeded_sequence_row` and `override_ids_are_unique` now cover the Titan rows. Add `titan_overrides_carry_their_kit_nvps`, which asserts each Titan entry's NVP list equals its `sequences_nvp` seed rows. Use the file's `seeded_sequences` helper, extended to read NVPs if needed.
- **Revert proof:** dropping one NVP pair fails the new test.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-resources --all-targets -- -D warnings`
  3. nextest `-p cimmeria-resources sequence`

### NX-13 Titan set, template and loot

- **Agent:** packet-coder. **Host:** windows. **Depends:** NX-11 merged into `nx-w2`, NX-03, D-NX8.
- **Branch:** `npc-combat-v1-1/NX-13-titan-set` (from `nx-w2`). **Worktree:** `nx-13`.
- **Commit:** `feat(seeds): the Straegis Titan picks its own set (NX-13)`
- **Files:** as in the ownership table.
- **Do:**
  1. Add set 360 "Straegis Titan (RECONSTRUCTED, NX-13)" with abilities 3500-3503 in kit-spec order, in the new file `ability_sets_straegis_titan.sql`. Edit nothing in `ability_sets.sql`.
  2. Set `ability_set_id = 360` on the template 79 row in `entity_templates.sql` (about line 160), with the C5 comment above it.
  3. Set lineup template 1570 too, only if NX-09 shows the lineup flag keeps it passive with a set. Otherwise leave it NULL and say why in the report.
  4. If D-NX8 approves: add loot table 40 "Debug Area Straegis Titan (RECONSTRUCTED)" with rows from 9160 holding item 2041, and set spawn 14320's `loot_table_id = 40`. If D-NX8 refuses, skip this step.
- **Tests:** fill `titan_set.rs`:
  1. live-DB `live_db_titan_picks_set_360`: template 79 resolves `ExplicitSet` with abilities 3500-3503, and the C1 resolver logs no `combat_profile_unresolved` for it.
  2. live-DB `live_db_gallery_titan_stays_neutral`: gallery spawn 13306 keeps its neutral aggression override and never acquires aggro (copy the V1 gallery guard).
  3. live-DB `live_db_titan_loot_table` (only with D-NX8): spawn 14320 rolls from table 40, and the template has no loot of its own.
  4. Update `creatures/mod.rs` `creature_profiles_today` for 79, with a comment naming NX-13.
- **Revert proof:** reverting the template row fails test 1.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell`
  3. `pwsh tools/build-lane/live-db-test.ps1 titan`
  4. `pwsh tools/build-lane/live-db-test.ps1 debug_area`

### NX-14 Titan stance or phase mechanic

- **Agent:** rust-gameserver-dev. **Host:** windows. **Depends:** NX-10, D-NX6.
- **Branch:** `npc-combat-v1-1/NX-14-titan-stance`. **Worktree:** `nx-14`.
- **Commit:** `feat(cell): Straegis Titan stance switch (NX-14)`
- **Files:** the code points that NX-09 and kit-spec.md name, and `creatures/stance.rs`.
- **Do, by the NX-10 verdict:**
  - **(a) D is the combat stance.** The server already sends `BSF_InCombat`. Add no mechanic; add the test only, which pins that the Titan's combat state sets the flag the client reads (NX-07 cites the flag).
  - **(b) D is a damage phase.** At 50% health, switch the phase once, using the mechanism kit-spec.md names. Log `creature_stance_changed` (C7), and pin it with a test.
- **Tests:** in `stance.rs` (no-DB, using the cell's NPC test fixtures):
  - (a) `titan_enters_combat_stance_on_aggro`;
  - (b) `titan_switches_phase_once_at_half_health` and `titan_phase_does_not_repeat`.
- **Revert proof:** removing the flag set or the phase switch fails the test.
- **Lane:**
  1. fmt
  2. clippy for each crate touched
  3. nextest `-p cimmeria-cell stance`, plus the crate the code lives in
- **Rules:** R1-R8, R10, R11, C7.

### NX-15 Tasker mechanic

- **Agent:** rust-gameserver-dev. **Host:** windows. **Depends:** NX-10, D-NX5.
- **Branch:** `npc-combat-v1-1/NX-15-tasker`. **Worktree:** `nx-15`.
- **Commit:** `feat(cell): Straegis Titan tasker, RECONSTRUCTED (NX-15)`
- **Files:** the code points kit-spec.md names (expected near `crates/cell-combat` ground-target area effects, or the deployable path, per NX-09), and `creatures/tasker.rs`.
- **Do (the D-NX5 default; kit-spec.md may amend it):**
  1. Ability 3502 records a ground point under the current target, with a server-side timer (the kit-spec delay) and no entity.
  2. Ability 3503, or the timer, applies the kit-spec area damage to players within the radius. Log `tasker_created` and `tasker_detonated` (C7).
  3. At most one live tasker per Titan. The tasker clears on the Titan's death or leash reset.
  4. Spawn a minion only if kit-spec.md says NX-05 found one.
- **Tests:** in `tasker.rs`, all no-DB:
  1. `tasker_detonates_once_at_its_point`;
  2. `tasker_hits_only_targets_in_radius`;
  3. `tasker_clears_on_titan_death`;
  4. `only_one_tasker_per_titan`.
- **Revert proof:** each test fails when its rule is removed.
- **Lane:**
  1. fmt
  2. clippy for each crate touched
  3. nextest for each crate, with filter `tasker`
- **Rules:** R1-R8, R10, R11, C7.

### NX-16 Titan reference tests and docs

- **Agent:** packet-coder. **Host:** windows. **Depends:** NX-11..15 merged into `nx-w2`.
- **Branch:** `npc-combat-v1-1/NX-16-titan-reference` (from `nx-w2`). **Worktree:** `nx-16`.
- **Commit:** `test(npc-combat): Straegis Titan reference suite (NX-16)`
- **Files:**
  - `creatures/titan.rs`;
  - `docs/analysis/npc-combat-v1-1/evidence-registry.md` (the Titan rows NX-11..15 settled, and § Runtime results);
  - `docs/content/debug-area.md` (the Z13 Titan row: today's behaviour).
- **Tests:** in `titan.rs` `mod live_db`, in the V1 `reference/scavdog.rs` shape:
  1. `live_db_titan_fights_with_its_own_kit`: spawn the Titan with `fight_space`, aggro a test player, step until it casts, and assert each cast's ability id is in 3500-3503 and never 592.
  2. `live_db_titan_melee_in_reach_ranged_outside`: the kit-spec reach rule.
  3. `live_db_titan_sends_its_sequences`: a cast emits the kit-spec sequence id in its `onSequence` (`use_ability/sequence.rs`).
- **Revert proof:** reverting NX-13 fails test 1, and pointing an ability at the wrong event set fails test 3.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell`
  3. `pwsh tools/build-lane/live-db-test.ps1 titan`
  4. Run R6 on both docs.

---

## Wave 3: Horden, client names, lab prep and regression (4 packets, one PR each)

Merge order: NX-17, NX-18, NX-20, NX-19. NX-19 is last, so its checklist cites the merged state. The wave review covers all four.

### NX-17 Horden disposition

- **Agent:** packet-coder. **Host:** windows. **Depends:** D-NX1 and D-NX2 answered, NX-06.
- **Branch:** `npc-combat-v1-1/NX-17-horden`. **Worktree:** `nx-17`.
- **Commit:** `test(npc-combat): Horden disposition (NX-17)`
- **Files:**
  - `creatures/horden.rs`;
  - `evidence-registry.md` (Horden rows);
  - `docs/content/debug-area.md` (the Z13 Horden row).

  Under D-NX2 option (b) only, also: new `Abilities/Seed/ability_sets_horden.sql` with its `\ir` line, the template 72 row in `entity_templates.sql`, and effect 1092 per NC-34 (roster-followup.md § Packet shape).
- **Do:**
  - **D-NX2 hold fire (recommended):** change no seed. Add live-DB `live_db_horden_holds_fire_with_its_gate`: template 72 resolves `Unresolved`, never casts, and logs `combat_profile_unresolved` once. Write gates G1 and G2 in the registry and on the debug-area row.
  - **D-NX2 option (b):** set 361 with 983 Bite. Edit effect 1092's `script_name` to `MeleePhysicalDamage`, with HealthDamage 15 and FocusDamage 125 as NVP rows 488-489 (C2 reserves them for NX-17; NX-11 stays within 480-487), each with the C5 comment. Add a test in the ScavDog shape (casts 983 only, never 592). Write in the registry that the attack is RECONSTRUCTED and has no animation (G1).
- **Revert proof:** a hold-fire test fails if a set is added; a set test fails if the set is removed.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell`
  3. `pwsh tools/build-lane/live-db-test.ps1 horden`
  4. Run R6 on the docs.

### NX-18 Ability overrides (only if D-NX4 says yes)

- **Agent:** rust-gameserver-dev. **Host:** windows. **Depends:** NX-08, D-NX4 = yes, wave 2 merged.
- **Branch:** `npc-combat-v1-1/NX-18-ability-overrides`. **Worktree:** `nx-18`.
- **Commit:** `feat(resources): cooked ability overrides for server-only ability ids (NX-18)`
- **Files:** new `crates/resources/src/base/ability_overrides.rs`, modelled on `sequence_overrides.rs` and `item_overrides`, and its registration in `apply_overrides.rs`, `metadata_bump.rs` and `committed_paks.rs`. Follow the call sites the sequence overrides use.
- **Do:**
  1. Generate a `CookedDataAbilities` entry for 3500-3503, byte-shaped like a shipped entry (NX-08 cites one), at startup.
  2. Bump the category version so clients resync (#840).
  3. Add a collision guard: an override never shadows a cooked id.
- **Tests:** unit tests:
  1. `generated_ability_xml_matches_a_shipped_entry_byte_for_byte`;
  2. `ability_override_ids_are_not_cooked`;
  3. `every_ability_override_matches_its_seeded_row`.
- **Revert proof:** each test fails without its rule.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-resources`
  3. nextest `-p cimmeria-resources ability_override`
- **Docs:** a line in `crates/resources/README.md` (or the overrides doc it links), CRLF.
- If D-NX4 says no, the orchestrator marks NX-18 Done (not needed) and does not dispatch it.

### NX-19 Lab prep and UAT checklist

- **Agent:** packet-coder (docs only). **Host:** windows. **Depends:** wave 2 merged, NX-17.
- **Branch:** `npc-combat-v1-1/NX-19-lab-prep`. **Worktree:** `nx-19`.
- **Commit:** `docs(npc-combat): V1.1 lab UAT checklist (NX-19)`
- **Files:** new `docs/analysis/npc-combat-v1-1/uat-checklist.md`. If the kill-drill clone is needed, also `entity_templates_debug_area_creatures.sql` (template 1390) and spawn 14322.
- **Do:**
  1. Write graded rows (expected, server evidence, client evidence, class) for:
     - the Titan: idle, aggro, the transition, melee, ranged, the tasker create and detonate, hit, death, respawn, loot;
     - Horden: rendering, then hold fire or Bite.
  2. Give each row its screenshot beat for D-NX7.
  3. Add the lab gotchas from V1 W4 results:
     - the world name is `DebugArea`;
     - `/gmsethealth` fails, so refill health by death and Release;
     - stand 4-6 m off targets;
     - the driver reads chat unless it gets `image: true`.
  4. The Titan kill path: V1 found no GM kill command for the lab character. Look for a kill path through the merged code (a GM command the lab account holds, or a `.`-console command). If none exists, add template 1390, a level-1 clone of 79 with set 360 and a small health pool, as Debug Area spawn 14322 (C2). Pin it with a live-DB test in `titan.rs`, coordinated through the orchestrator, or in a new `creatures/titan_drill.rs`.
- **Lane:** if 1390 is added: fmt, clippy and `live-db-test.ps1 titan`. Run R6.

### NX-20 V1 regression verify

- **Agent:** packet-coder (verify only). **Host:** windows. **Depends:** wave 2 merged.
- **Branch:** `npc-combat-v1-1/NX-20-v1-regression`. **Worktree:** `nx-20`.
- **Commit:** `test(npc-combat): V1 reference suite unchanged by V1.1 (NX-20)`
- **Files:** `creatures/v1_regression.rs` only.
- **Do:**
  1. Run the V1 suites unchanged: `live-db-test.ps1 debug_area_combat`, nextest `-p cimmeria-cell-world combat_profile`, `-p cimmeria-cell-combat` and `-p cimmeria-cell npc_ai`.
  2. Add `live_db_v1_reference_sets_unchanged`, which asserts that templates 4, 24, 76, 159, 160, 1380 and 1381 keep their V1 sets (221, 559, 1433 and set 4) and that no V1.1 id appears in them.
  3. A failure on current code follows the verify-only rule (dispatch rules).
- **Revert proof:** changing any V1 set fails the test.
- **Lane:**
  1. fmt
  2. clippy `-p cimmeria-cell`
  3. the runs in step 1

---

## Wave 4: lab UAT (ask first)

NX-30 and NX-31 run in **one** lab session (one lease, one login), only after the orchestrator asks "may I use the lab now?" and gets a yes (D-NX10). The orchestrator runs the `lab-uat` skill with a Haiku `lab-driver`, and briefs it with the NX-19 checklist and the gotchas. The driver takes `lab_screenshot` with `image: true` at each beat (D-NX7). The orchestrator grades each row against `logs/combat.log` and `logs/server.log`, never on the driver's word.

| Packet | Rows | Pass bar |
|---|---|---|
| NX-30 Titan | NX-19 Titan rows | Every server row PASS. Animation and VFX rows graded from the screenshots and the owner's watch (D-NX7); sound NOT VERIFIED (G3). |
| NX-31 Horden | NX-19 Horden rows | Rendering graded from screenshots; combat per D-NX2 |

- **Output:** `docs/analysis/npc-combat-v1-1/lab-results.md` (CRLF), in the V1 `w4-lab-results.md` shape. NX-30's documentation-writer or packet-coder writes it from the orchestrator's graded notes.
- **NX-32:** fix packets cut by the architect from failed rows, one per finding, in the rules above.

## Wave 5: campaign review and close-out

### NX-40 Campaign review loop

The `campaign-packet` skill § 4 loop runs on `origin/main`, over everything V1.1 merged. It checks:

- the docs against the behaviour;
- the decisions against the code;
- the registry against the lab results;
- the links and the `<!-- gen: -->` blocks.

Fixes ship as one PR from the `npc-combat-v1-1-review` worktree.

### NX-41 Close-out

- **Agent:** documentation-writer. **Host:** windows. **Depends:** NX-40 shipped.
- **Branch:** `npc-combat-v1-1/NX-41-close-out`. **Worktree:** `nx-41`.
- **Commit:** `docs(npc-combat): V1.1 close-out (NX-41)`
- **Files:**
  - `docs/gap-analysis*` (the NPC combat rows);
  - `docs/project-status.md`;
  - `docs/guides/unified-uat.md` (the owner's remaining UAT: the D-NX7 sign-off, and sound if a bank ever appears);
  - this campaign's README and RESUME.
- **Do:** state plainly what works, what was verified in the lab, and what is RECONSTRUCTED or blocked by G1-G4. Run `python tools/docs-gen/regen.py --check` and never hand-edit a gen block. Run R6 on every file.
- `ops` retires every worker worktree, the orchestrator empties `.claude/state/active-campaign`, and `ops` posts a board handoff.
