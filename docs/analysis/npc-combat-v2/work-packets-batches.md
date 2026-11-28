# NPC combat V2: work packets, waves 2-6

Ledger: [README.md](README.md). The dispatch rules, R1-R12 and the contract C1-C6 are in [work-packets.md](work-packets.md); every brief inlines the rules it names. These sections are **provisional**: NR-10 (`batch-spec.md`) gives each packet its exact rows, and no packet here is dispatched before NR-10 lands.

Every batch brief inlines R1-R8, R10 and R11, C3 (the batch packet shape) and these seed rules:

- Load the `seed-change` skill.
- Copy the column order from an existing row.
- Edit only the rows `batch-spec.md` assigns to you, and put the C5 comment above each.
- Reload the worktree DB through `pwsh tools/build-lane/live-db-test.ps1 <filter>`.

The standard lane for a batch packet:

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 roster`
4. `pwsh tools/build-lane/live-db-test.ps1 seed_sequences`

Integration (D-NR12):

- Each of waves 2, 3 and 4 ships as **one PR** from its integration worktree (`nr-w2`, `nr-w3`, `nr-w4`), cut from `origin/main` when the wave starts.
- `ops` merges each packet branch into it with `merge-noff.ps1` after the packet's review returns `ship`.
- The wave review runs on the integration tree, and its fix notes go to one fresh packet-coder there.
- After the wave PR merges, `ops` retires the packet worktrees with `rm-worktree.ps1 --merged-into <PR> <names>`.
- A wave starts only once the previous wave's PR has merged, because the waves edit the same seed files.

---

## Wave 2: best evidence first (5 packets, integration `nr-w2`)

**Merge order:** NR-11, NR-12, NR-13, NR-14, NR-15. NR-11, NR-13 and NR-14 start together. NR-12 starts from `nr-w2` once NR-11 is in, and NR-15 once NR-12 is in.

### NR-11 Weapon auto-attack sets

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-10.
- **Branch:** `npc-combat-v2/NR-11-weapon-sets`. **Worktree:** `nr-11`.
- **Commit:** `feat(seeds): NPC weapon auto-attack sets (NR-11)`
- **Files:**
  - new `db/resources/Abilities/Seed/ability_sets_npc_v2_weapons.sql`, and its `\ir` line after `ability_sets.sql`;
  - `roster/firearms.rs`, the set test only (NR-12 adds the template test below it, in a separate `mod`).
- **Do:** add sets 365-372 from `weapon-map.md`, one per weapon class, each with its ranged and melee auto-attack (V1 set 3 is the NID shape). Each set's description names its class, and the C5 comment is STRONGLY INFERRED.
- **Tests:** live-DB `live_db_weapon_sets_hold_their_auto_attacks`: a table of (set, [abilities]) from weapon-map.md. No set contains 592.
- **Revert proof:** dropping the file fails the test.

### NR-12 B1 firearm humans pick their weapon set

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-11 merged into `nr-w2`.
- **Branch:** `npc-combat-v2/NR-12-firearm-humans` (from `nr-w2`). **Worktree:** `nr-12`.
- **Commit:** `feat(seeds): firearm humans fire their own weapon (NR-12)`
- **Files:** the template rows `batch-spec.md` § B1 lists (expected in `entity_templates.sql` and the area template files), and `roster/firearms.rs` (a new `mod templates`).
- **Do:** set each B1 template's `ability_set_id` to the set for its weapon component's class.
- **Tests:**
  1. live-DB `live_db_firearm_humans_resolve_their_weapon_set`, table-driven over every B1 row: each resolves `ExplicitSet` with its set, not the placeholder.
  2. live-DB `live_db_firearm_human_fights_with_its_weapon`, using the B1 Debug Area representative: it casts only its set's abilities and never 592.
- **Revert proof:** reverting one row fails test 1.

### NR-13 B3 mechanical

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-10 (from NR-06).
- **Branch:** `npc-combat-v2/NR-13-mechanical`. **Worktree:** `nr-13`.
- **Commit:** `feat(seeds): drones and walkers fight with the drone kit (NR-13)`
- **Files:**
  - new `Abilities/Seed/ability_sets_npc_v2_mechanical.sql` (sets 373 and up), and its `\ir` line, placed after NR-11's line;
  - the template rows for 69, 81 and 70 (only if NR-06 found its evidence), and lineup 1563-1566 and 1569 per D-NR4;
  - `roster/mechanical.rs`.
- **Do:** follow `batch-spec.md` § B3, using the drone abilities NR-06 paired (README R6).
- **Tests:**
  1. live-DB, table-driven: each template resolves its set.
  2. A fight test on the Z14 drone pen (spawn from NR-03): it casts only drone abilities and never 592.
  3. Lineup rows stay passive.
- **Revert proof:** reverting a template row fails test 1.

### NR-14 B4a AMBRat

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-10, D-NR2.
- **Branch:** `npc-combat-v2/NR-14-ambrat`. **Worktree:** `nr-14`.
- **Commit:** `feat(seeds): the AMBRat bites back (NR-14)`
- **Files:**
  - new `Abilities/Seed/ability_sets_npc_v2_ambrat.sql` (set 380), and its `\ir` line;
  - the template 74 row and lineup 1554;
  - `roster/ambrat.rs`;
  - the AMBRat table in `evidence-registry.md`;
  - one line in `docs/analysis/hub-pass-uat/README.md`, saying the AMBRat attacks again, with its set.
- **Do:** set 380 with the ability `batch-spec.md` names (from NR-05). If that is 983 Bite and V1.1 has not yet given effect 1092 its script, NR-14 owns that edit (C2) and NR-22 reuses it.
- **Tests:**
  1. live-DB `live_db_ambrat_fights_with_set_380`, in the ScavDog shape: casts only set 380 abilities, never 592.
  2. `live_db_ambrat_lineup_twin_passive`.
- **Revert proof:** reverting the template row fails test 1.

### NR-15 Retire the 592 placeholder

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-12 merged into `nr-w2`, D-NR5 = yes.
- **Branch:** `npc-combat-v2/NR-15-retire-592` (from `nr-w2`). **Worktree:** `nr-15`.
- **Commit:** `feat(cell-world): no NPC receives 592 Pistol Shot implicitly (NR-15)`
- **Files:**
  - `crates/cell-world/src/cell/combat/combat_profile/mod.rs` (the `PLACEHOLDER_POLICY` constant, and the doc comment explaining why);
  - its unit tests;
  - `roster/placeholder_retired.rs`.
- **Do:**
  1. Add the variant `PlaceholderPolicy::Never`, under which an empty set always resolves `Unresolved`.
  2. Set `PLACEHOLDER_POLICY = Never`. Keep `Legacy` and `RequireWeaponEvidence` and their tests (V1 C1).
  3. `HUMAN_FIREARM_PREFIXES` stays, now used only by those tests and NR-01's classification.
  4. Log no new event: an unmapped firearm human now logs `combat_profile_unresolved`.
- **Tests:**
  - unit `never_policy_gives_no_placeholder_to_a_firearm_human`;
  - live-DB `live_db_no_template_resolves_to_the_placeholder`: over every seeded template, no profile is `Placeholder`.
- **Revert proof:** restoring the old constant fails the live-DB test, as long as any unmapped firearm template is left.
- **Lane:**
  1. fmt and clippy `-p cimmeria-cell-world`
  2. nextest `-p cimmeria-cell-world combat_profile`
  3. the standard batch lane
- If D-NR5 says no, the orchestrator marks NR-15 Done (not needed).

---

## Wave 3: Jaffa and the organic creatures (6 packets, integration `nr-w3`)

**Merge order:** NR-21, NR-22, NR-23, NR-24, NR-25, NR-26. All but NR-23 and NR-26 start together. NR-23 starts once NR-22 is in (it may reuse NR-22's effect 1092 edit), and NR-26 once the rest are in.

### NR-21 B2 Jaffa with weapons

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-10.
- **Branch:** `npc-combat-v2/NR-21-jaffa`. **Worktree:** `nr-21` (`nr-21a` and `nr-21b` if NR-10 splits it by file).
- **Commit:** `feat(seeds): armed Jaffa fight with their staff or zat (NR-21)`
- **Files:** the B2 template rows; `roster/jaffa.rs`.
- **Do:** give staff carriers set 4 (584 and 710, the V1 NC-41 path), and zat carriers the zat set from NR-11, if `weapon-map.md` has one.
- **Tests:**
  1. live-DB, table-driven: each B2 row resolves its set.
  2. A fight test with the B2 representative: casts only 584 and 710 (or the zat pair), never 592.
- **Revert proof:** reverting a row fails test 1.

### NR-22 B4b Carnosaur

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-10, D-NR2.
- **Branch:** `npc-combat-v2/NR-22-carnosaur`. **Worktree:** `nr-22`.
- **Commit:** `feat(seeds): the Carnosaur fights with its own set (NR-22)`
- **Files:**
  - new `Abilities/Seed/ability_sets_npc_v2_carnosaur.sql` (set 381), and its `\ir` line;
  - template rows 71 and 1559;
  - effect 1092 and NVPs from 490, only if C2 makes NR-22 the owner;
  - `roster/carnosaur.rs`;
  - the registry table.
- **Do:** follow the NC-34 packet shape (`npc-combat-reference/roster-followup.md` § Packet shape) with `batch-spec.md`'s ability.
- **Tests:** a fight test in the ScavDog shape, and the lineup twin stays passive.
- **Revert proof:** reverting the template row fails the fight test.

### NR-23 B4b Rhinolion

- As NR-22, for templates 75 and 1560, set 382, `roster/rhinolion.rs`.
- **Branch:** `npc-combat-v2/NR-23-rhinolion`. **Worktree:** `nr-23`.
- **Commit:** `feat(seeds): the Rhinolion fights with its own set (NR-23)`
- **Depends:** NR-22 merged into `nr-w3` (effect 1092), NR-10, D-NR2.

### NR-24 B4b Lenny

- As NR-22, for templates 73 and 1556, set 383, `roster/lenny.rs`. The baby (1557) follows D-NR3; the default is to leave it NULL and pin it in `lenny.rs`.
- **Branch:** `npc-combat-v2/NR-24-lenny`. **Worktree:** `nr-24`.
- **Commit:** `feat(seeds): Lenny fights with its own set (NR-24)`
- **Depends:** NR-10, D-NR2, D-NR3.

### NR-25 B4b Twilla Tree

- **Under D-NR10 with attack evidence:** as NR-22, for templates 80 and 1561, set 384.
- **With no evidence:** no seed change; a hold-fire pin in `roster/twilla.rs` and the registry gate.
- **Branch:** `npc-combat-v2/NR-25-twilla`. **Worktree:** `nr-25`.
- **Commit:** `feat(seeds): Twilla Tree disposition (NR-25)`
- **Depends:** NR-06, D-NR10.

### NR-26 Lineup rows (D-NR4)

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-21..25 merged into `nr-w3`.
- **Branch:** `npc-combat-v2/NR-26-lineup` (from `nr-w3`). **Worktree:** `nr-26`.
- **Commit:** `feat(seeds): lineup actors carry their world template's set (NR-26)`
- **Files:** only the lineup rows in `entity_templates_debug_area_lineup.sql` that no earlier packet touched, and `roster/lineup.rs`.
- **Do:** give each lineup row whose world template now has a set that same set. Leave every other lineup row NULL.
- **Tests:**
  - live-DB: each lineup row matches its world twin's set;
  - every lineup actor stays passive (copy the V1 lineup guard).
- **Revert proof:** reverting a row fails the first test.

---

## Wave 4: thin evidence and hold-fire pins (5 packets, integration `nr-w4`)

**Merge order:** NR-31, NR-32, NR-33, NR-34, NR-35. All but NR-35 start together, and NR-35 starts once the rest are in.

### NR-31 B5 Straegis Fighter and Beacon

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-08.
- **Branch:** `npc-combat-v2/NR-31-straegis`. **Worktree:** `nr-31`.
- **Commit:** `feat(seeds): hostile Straegis Fighter and Beacon (NR-31)`
- **Files:**
  - new `Abilities/Seed/ability_sets_npc_v2_straegis.sql` (sets 385 and 386), and its `\ir` line;
  - template rows 78, 1567, 77 and 1568;
  - `roster/straegis.rs`;
  - the registry tables.
- **Do:** follow `batch-spec.md` § B5. **Never edit** the pet abilities' event sets: the Fighter pet must keep working.
- **Tests:**
  1. A fight test per template.
  2. live-DB `live_db_straegis_pet_kit_unchanged`: the pet summon's abilities and event sets are as seeded before NR-31.
- **Revert proof:** reverting a row fails test 1.

### NR-32 B6 Goa'uld

- **Agent:** packet-coder. **Host:** windows. **Depends:** NR-07, D-NR7.
- **Branch:** `npc-combat-v2/NR-32-goauld`. **Worktree:** `nr-32`.
- **Commit:** `feat(seeds): armed Goa'uld fight with their hand device or staff (NR-32)`
- **Files:**
  - a new `Abilities/Seed/ability_sets_npc_v2_goauld.sql` (sets from 390) only if no NR-11 set fits;
  - the B6 template rows;
  - `roster/goauld.rs`.
- **Do:** set only the rows with evidence. A Goa'uld with no weapon evidence stays NULL and goes to NR-34's table.
- **Tests:** table-driven resolution, and a fight test with the B6 representative.
- **Revert proof:** reverting a row fails the table test.

### NR-33 B7 unarmed hostile humanoids

- **Agent:** packet-coder. **Host:** windows. **Depends:** D-NR6 = Fists.
- **Branch:** `npc-combat-v2/NR-33-unarmed`. **Worktree:** `nr-33`.
- **Commit:** `feat(seeds): unarmed hostile humanoids fight with their fists (NR-33)`
- **Files:**
  - a new `Abilities/Seed/ability_sets_npc_v2_unarmed.sql` (one set, 733 Fists Melee AA, id from 390 per NR-10);
  - the B7 hostile rows;
  - `roster/unarmed.rs`.
- **Do:** set only the hostile rows. Friendly, interactive, vendor and trainer NPCs stay NULL.
- **Tests:**
  1. table-driven resolution;
  2. a fight test;
  3. a test that a friendly row next to them stays NULL.
- **Revert proof:** reverting a row fails test 1.
- If D-NR6 says hold fire, the B7 rows go to NR-34 instead, and NR-33 is marked Done (not needed).

### NR-34 B8 hold-fire pins

- **Agent:** packet-coder. **Host:** windows. **Depends:** D-NR1, NR-32 and NR-33 assignments from NR-10.
- **Branch:** `npc-combat-v2/NR-34-hold-fire`. **Worktree:** `nr-34`.
- **Commit:** `test(npc-combat): pin the V2 hold-fire roster with its gates (NR-34)`
- **Files:** `roster/hold_fire.rs`; the hold-fire tables in `evidence-registry.md`.
- **Do:**
  1. Add one table-driven live-DB test over every B8 row: Unas (under D-NR1 hold fire), the Android (A1), the "other" family, and any B6 or B7 row left NULL. Each resolves `Unresolved` and never casts.
  2. Give each row its reason (the gate or decision) in the registry.
- **Revert proof:** giving any pinned row a set fails the test.

### NR-35 Backlog recount and V1/V1.1 regression

- **Agent:** packet-coder (verify only). **Host:** windows. **Depends:** NR-31..34 merged into `nr-w4`.
- **Branch:** `npc-combat-v2/NR-35-recount` (from `nr-w4`). **Worktree:** `nr-35`.
- **Commit:** `test(npc-combat): V2 backlog recount and reference regression (NR-35)`
- **Files:** `roster/backlog.rs`; `docs/analysis/npc-combat-v2/backlog.md` (an "after" column).
- **Do:**
  1. Add live-DB `live_db_v2_backlog_matches_the_ledger`. It counts the seeded templates that resolve `Placeholder` and `Unresolved`, and asserts the counts equal the numbers in `backlog.md` § After: `Placeholder` is 0 under D-NR5, and `Unresolved` is the hold-fire and out-of-scope total.
  2. Run the V1 and V1.1 suites unchanged: `live-db-test.ps1 debug_area_combat`, and nextest `-p cimmeria-cell-world combat_profile`.
  3. A failure on current code follows the verify-only rule.
- **Revert proof:** any batch row reverted changes the count, and the test fails.

---

## Wave 5: lab UAT by batch (ask first)

- **NR-40:** 2-3 lab sessions, each only after the orchestrator asks "may I use the lab now?" and gets a yes (D-NR11). The orchestrator runs the `lab-uat` skill with a Haiku `lab-driver`, briefed with the V1 gotchas:
  - the world name is `DebugArea`;
  - `/gmsethealth` fails, so refill health by death and Release;
  - stand 4-6 m off targets;
  - take `lab_screenshot` with `image: true` at each beat (V1.1 D-NX7).
- **Sessions:**
  1. B1, B3 and the AMBRat.
  2. B2 and B4b.
  3. B5-B7, and a sample of the hold-fire pins.
- Grade every row against `logs/combat.log` and `logs/server.log`.
- **Output:** `docs/analysis/npc-combat-v2/lab-results.md` (CRLF), in the V1 `w4-lab-results.md` shape. Sound is NOT VERIFIED (A3).
- **NR-41:** fix packets, which the architect cuts from failed rows.

## Wave 6: campaign review and close-out

- **NR-50:** the campaign-packet skill § 4 campaign loop on `origin/main`, with fixes as one PR from `npc-combat-v2-review`.
- **NR-51 close-out:**
  - **Agent:** documentation-writer. **Host:** windows. **Depends:** NR-50.
  - **Branch:** `npc-combat-v2/NR-51-close-out`. **Worktree:** `nr-51`.
  - **Commit:** `docs(npc-combat): V2 close-out (NR-51)`
  - **Files:**
    - `docs/gap-analysis*`;
    - `docs/project-status.md`;
    - `docs/guides/unified-uat.md` (the remaining owner UAT);
    - this README and RESUME.
  - **Do:**
    1. State per family what fights, with which evidence class, and what holds fire, with which gate.
    2. Run `python tools/docs-gen/regen.py --check`.
    3. Run R6 on every file.
    4. `ops` retires the worktrees, the orchestrator empties `.claude/state/active-campaign`, and `ops` posts a board handoff.
