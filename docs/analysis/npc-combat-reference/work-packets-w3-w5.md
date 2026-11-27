# NPC combat reference: work packets, waves 3-5

Same dispatch rules, rules R1-R10 and contract C1-C6 as
[work-packets.md](work-packets.md). A packet here is not self-contained
until you have read that file's § Dispatch rules, § Rules and the contract
sections the packet names.

---

## Wave 3: flip, staff in place, registry, UAT spec, roster spec, NID tooltip (6 packets)

Wave 3 ships as one PR from an integration branch,
`npc-combat-reference/W3-integration` (worktree `nc-w3`). Ops cuts it from
`origin/main` once the W2 PR has merged, or else from the W2 integration
head 2a1887107 and rebases it onto `origin/main` after the W2 merge. Each
packet runs its own review loop and is merged into the integration branch
with `merge-noff.ps1`. NC-22, NC-30, NC-31, NC-33 and NC-34 run in parallel
on disjoint file sets. NC-25 (D-NC8, D-NC11) is cut from the integration
head only after NC-31 and NC-33 have been merged into it, because it
corrects lines in the files they create, and D-NC12 is decided (a). Merge
order into the integration branch: NC-22, NC-31, NC-33, NC-34, NC-25, then
NC-30 last (only after a green
`pwsh tools/build-lane/lane.ps1 --exclusive cargo nextest run --workspace <EXCL>`
on the integration head with all six merged; EXCL as in the lane-build
skill). Retire each packet worktree with `rm-worktree.ps1 --merged-into
<W3 PR>` once the wave PR merges.

**Wave 3 gate.** Every packet `ship` in its own review loop, then one wave
review `ship` over the integration branch, then the green workspace nextest.
W4 starts only when the wave PR has merged, and each lab session still needs
its own go-ahead (D-NC7).

### NC-22 Staff component on the real 159/160

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-12 and NC-20
  merged; D-NC3 decided "in place".
- **Branch:** `npc-combat-reference/NC-22-jaffa-staff-in-place`. **Worktree:** `nc-22`.
- **Commit:** `feat(content): Praxis Jaffa 159/160 carry their staff (NC-22)`
- **Files:**
  - `db/resources/Entities/Seed/entity_templates.sql` (rows 159 and 160:
    insert `WP-Jaffa.WP_Staff_Plasma_4A` into `components`, keeping the
    array sorted the way the 1371 row is; nothing else);
  - `reference/regression_harset.rs` (test 1 now expects the component);
  - lineup seeds, only if the lineup guard fails. A new look needs a lineup
    actor in templates 1571-1599 and spawns 14031-14099; follow the existing
    lineup rows in `docs/content/debug-area.md` § lineup (about line 1371).
- **Tests:** the updated NC-24 test 1, and
  `debug_area_lineup_live_db_every_look_has_an_actor`.
- **Lane:** fmt and clippy `-p cimmeria-cell`;
  `live-db-test.ps1 regression_harset`; `live-db-test.ps1 lineup`.
- **Docs:** the `debug-area.md` lineup table, only if you add an actor.
- **Rules:** R1-R8, R10.

### NC-30 Flip to `RequireWeaponEvidence`

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-10, NC-21a-c,
  NC-11, NC-12, NC-13 and NC-14 merged; D-NC1 decided.
- **Branch:** `npc-combat-reference/NC-30-require-weapon-evidence`. **Worktree:** `nc-30`.
- **Commit:** `feat(npc-ai): no Pistol Shot placeholder without firearm evidence (NC-30)`
- **Files:**
  - `crates/cell-world/src/cell/combat/combat_profile/mod.rs` (the constant
    only);
  - `combat_profile/tests.rs` (test 9 now pins `RequireWeaponEvidence`);
  - `reference/placeholder_policy.rs`;
  - doc-comment updates in `spawn.rs` (the comment above the resolver call)
    and in `ability_select.rs` (the "Why no ammo gate", `npc_is_melee_only`
    and "Effect on single-ability sets" comments that mention the 592
    fallback);
  - `docs/gameplay/npc-ai.md` (the NC-10 subsection: the policy is now
    `RequireWeaponEvidence`).
- **Tests** (in `placeholder_policy.rs`):
  1. A creature record (body `MOB_*`, empty set, no `WP-`) spawned through
     `spawn_npc_from_record` knows no ability. One AI pass with a player at
     5 m leaves the player undamaged, and the fight records `no_ability`
     (grep `note_outcome("no_ability")` in `fight.rs` for how to observe it).
  2. A no-set template with `WP-Human.*` still knows [592] and fires.
  3. A no-set template with `WP-Jaffa.*` holds fire and logs one
     `combat_profile_unresolved`.
  - Revert proof: setting the constant back to `Legacy` fails tests 1 and 3.
- **Lane:** fmt and clippy for `-p cimmeria-cell-world`,
  `-p cimmeria-cell-combat` and `-p cimmeria-cell`; then the exclusive
  workspace nextest above; then `live-db-test.ps1 debug_area` and
  `live-db-test.ps1 regression_`. Any failure outside these files is a
  fixture that NC-21 missed. List it and return `blocked`; do not patch it
  here.
- **Docs:** `npc-ai.md`. Run R6.
- **Rules:** R1-R3, R5, R7, R8, R10.

### NC-31 Evidence registry filled

- **Agent:** documentation-writer. **Host:** windows. **Depends:** NC-02,
  NC-04, NC-05, NC-06, NC-07 and NC-11 to NC-14 merged. If NC-05 or NC-06
  is blocked for lack of a client, mark those rows UNRESOLVED and say so.
- **Branch:** `npc-combat-reference/NC-31-registry-fill`. **Worktree:** `nc-31`.
- **Commit:** `docs(npc-combat): fill the reference evidence registry (NC-31)`
- **Files:** `docs/analysis/npc-combat-reference/evidence-registry.md` only.
- **Do:**
  - Fill every C6 row from the NC-04, 05, 06 and 07 docs and the NC-11..14
    test results (from the packet reports, which the orchestrator copies into
    `worknotes/`).
  - Upgrade a row's class only with a cited source. Never call a
    RECONSTRUCTED row original.
  - Fill § Open (UNRESOLVED). This is the brief's deliverable 7, the list of
    remaining unresolved original behaviour.
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8, R9.

### NC-33 Live UAT checklist and debug-area.toml rows

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-03 and NC-11 to
  NC-14 merged.
- **Branch:** `npc-combat-reference/NC-33-uat-checklist`. **Worktree:** `nc-33`.
- **Commit:** `docs(npc-combat): Z12 live UAT checklist (NC-33)`
- **Files:** new `docs/analysis/npc-combat-reference/uat-checklist.md`;
  `docs/guides/uat-specs/debug-area.toml` (append rows in its existing
  format, with ids `NC-U1`..; read the file's header and a few rows first).
- **Do:** write one block per reference (NID, Jaffa, ScavDog, PRU) plus a
  telemetry block. Each row has a step, the expected client presentation
  (weapon model, animation, projectile or beam, sound), the expected server
  evidence (the log line, the damage numbers), and a pass/fail cell. Cover:
  - setup: the Z1 arrival, the Z2 granter and vendors for the weapon and
    ammo, then walking to Z12, standing 4-6 m off the target and clearing
    dialogs first (lab gotchas);
  - the NID: burst fire at range, SMG muzzle and sound, damage numbers,
    cover use, death, loot (table 2) and a 20 s respawn;
  - the Jaffa: staff visible, a ranged bolt at 15 m, a staff swing at 2 m
    (after NC-20), damage on the swing, and the switch back when you step
    out;
  - the ScavDog: no gun presentation ever, a bite only in reach, creature
    attack sound, hit reaction and death;
  - the PRU: energy beam or projectile from its emitter, damage, and drone
    movement;
  - telemetry: no `combat_profile_placeholder` for 24, 4, 76, 1380 or 1381;
    no `effect_inert` for 736 or 1717; `combat_profile_*` lines carry
    template names.

  Mark each row's evidence class target (ORIGINAL CLIENT VERIFIED when it is
  seen in the client).
- **Lane:** if the repo has a TOML lint for uat-specs, run it; find it in
  the tools catalog. Run R6 on the md.
- **Rules:** R1, R3, R6, R8, R9.

### NC-34 Roster follow-up spec

- **Agent:** documentation-writer. **Host:** windows. **Depends:** NC-15 and
  NC-31 merged.
- **Branch:** `npc-combat-reference/NC-34-roster-spec`. **Worktree:** `nc-34`.
- **Commit:** `docs(npc-combat): roster follow-up spec (NC-34)`
- **Files:** new `docs/analysis/npc-combat-reference/roster-followup.md`.
- **Do:** propose one follow-up campaign for Carnosaur, Horden, Rhinolion,
  AMBRat, Lenny and Unas. For each: the templates (from
  `placeholder-inventory.md`), the body family, the matching reference
  pipeline (organic → ScavDog, construct → PRU, and so on), candidate
  abilities with the evidence each still needs, and the packet shape. **No
  seed changes.** Assignment waits for the W4 UAT (brief § Definition of
  done).
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R9.

### NC-25 Ability 559 gets its own tooltip effect, 5310 (D-NC11 split)

- **Agent:** packet-coder. **Host:** windows. **Depends:** NC-11 merged
  (#2089); NC-31 and NC-33 merged into the W3 integration branch (ops cuts
  `nc-25` from its head after both); D-NC8 and D-NC11 decided 2026-11-27;
  **D-NC12 decided "(a)"** (README). NC-25 is integrated. If the
  owner picks (b), (c) or (d), the architect re-specs this packet.
- **Branch:** `npc-combat-reference/NC-25-559-tooltip-effect`. **Worktree:** `nc-25`.
- **Commit:** `fix(content): ability 559 uses its own tooltip effect 5310 (NC-25)`
- **Why.** Ability 559 (`Automatic Weapon Auto Attack`) has this tooltip,
  `abilities.sql:5252-5255`, which you **must not change**:
  `Ranged Single Target Attack` / `-100F` / `-10H` / `1 Ammo`. Its one effect,
  621, executes the Giza seed's -200F/-20H. The owner chose the tooltip
  (D-NC8) and chose to keep 621 as it is and give 559 a new effect row
  (D-NC11). The new numbers are RECONSTRUCTED (Cimmeria's judgement from the
  tooltip); never call them original or CME.
- **Blast radius (D-NC12).** An ability's damage resolves only through
  `abilities.effect_ids` (`AbilityDef.effect_ids`); `effects.ability_id` is not
  read for resolution, and 621 has no other caller (grep: `'{621}'` appears
  once in `db/resources/Abilities/Seed`). So repointing 559 changes the damage
  of everything that fires 559: the NID Guard, all 29 templates on set 3
  (the Z9 lethal squad 1376 among them) and the player SMG and assault-rifle
  auto attack (`items_event_sets` event 7). After this packet no ability uses
  621. That is what D-NC12 (a) accepts, and it is why test 2 below re-measures
  Z9. The client never sees 5310: an instant single-target hit sends
  `ability_id` 559 and the cast id in `onEffectResults`, not the effect row id.

  | Tooltip | Where it executes | Before | After |
  |---|---|---|---|
  | `-100F` `-10H` | `abilities.effect_ids` of 559 | `'{621}'` | `'{5310}'` |
  | `-100F` | `effect_nvps` 25521 `FocusDamage` on 5310 | none | `'100'` |
  | `-10H` | `effect_nvps` 25520 `HealthDamage` on 5310 | none | `'10'` |
  | `1 Ammo` | `abilities.required_ammo` (3) | out of scope (D-NC10) | unchanged |

- **Ids (checked 2026-11-27 against the nc-30 tree; re-grep before you
  write).** Effect 5310 is the next id after the seeded maximum 5309
  (`effects.sql` holds 3217 rows, ids 77-5309). It is not a client cooked id
  and does not reach the wire, and no test or tool names 5310 as an effect
  (grep `\b5310\b` in `crates/` and `tools/`; any hit that is an effect id is
  a stop: return `blocked`). NVP ids 25520-25529 are a new hand-authored
  ten-block after the minigames (25500-25509) and debug-armory (25510-25519)
  blocks, outside every `-- ability-mechanics generated` block. No test pins
  the effect or NVP row count. 559 is not in the generator's reachable set
  (`archetype_ability_tree.sql`, `char_creation_abilities.sql`), so
  `effect_nvps_from_desc.py` ignores 5310.
- **Files you own:**
  1. `db/resources/Effects/Seed/effects.sql`. Insert after line 7822 (the
     blank line after the 2907 row, before the `-- TOC entry 3314` trailer),
     so no cited line moves. Four lines and one blank line:
     ```sql
     -- RECONSTRUCTED (NC-25, D-NC8/D-NC11): effect 5310 is ability 559's tooltip '-100F -10H'; Giza effect 621 ('-200F -20H') is kept unchanged
     INSERT INTO effects (effect_id, ability_id, delay, effect_desc, effect_sequence, flags, icon_location, pulse_count, pulse_duration, tcm_param1, tcm_param2, target_collection_method, use_ability_velocity, is_channeled, name, target_collection_id, event_set_id, script_name) VALUES (5310, 559, 0, 'Single Target
     -100F 
     -10H', 0, 0, 'set:CoreWidgets image:IconMissing', 1, 0, NULL, NULL, 'TCM_Single', false, false, 'AW Auto Attack Damage', 0, 289, 'RangedPhysicalDamage');
     ```
     It is the 621 row (lines 7817-7819) with a new id and the tooltip's
     numbers; copy the column list from line 7817 rather than retyping it.
     `-100F ` keeps its trailing space, as 621's `-200F ` does.
  2. `db/resources/Effects/Seed/effect_nvps.sql`. Insert after line 1663
     (nvp 25511), before the blank line and the `-- TOC entry 3313` trailer:
     ```sql

     -- npc-combat-reference hand-authored (25520-25529): NC-25
     -- RECONSTRUCTED (NC-25, D-NC11): effect 5310 desc '-100F -10H' (ability 559's tooltip) -> HealthDamage 10, FocusDamage 100; Giza effect 621 keeps 20/200 (nvps 16, 17)
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25520, 5310, 'HealthDamage', '10');
     INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (25521, 5310, 'FocusDamage', '100');
     ```
  3. `db/resources/Abilities/Seed/abilities.sql`, the 559 row. On line 5255
     change only `'{621}'` to `'{5310}'` (it sits between `100, 0, ` and
     `'{3175425141}'`). Replace the **blank** line 5251 above the row with
     this comment, so no line number moves:
     `-- RECONSTRUCTED (NC-25, D-NC11): effect_ids 621 -> 5310, the effect that follows this tooltip; Giza effect 621 is kept unchanged`
  4. `db/resources/Entities/Seed/entity_templates_debug_area_combat.sql`,
     the comment above template 1376 (lines 51-53) only: replace
     `559 (200 Focus / 20 Health, focus-gated bleed)` with
     `559 (100 Focus / 10 Health since NC-25, focus-gated bleed)` and
     `five AI ticks (about 10 s)` with your measured `m AI ticks (about 2m s)`.
     Keep three comment lines; touch no INSERT.
  5. `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/reference/nid.rs`
     (test 1; the file is 124 lines).
  6. `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/live_db.rs`,
     only `live_db_debug_area_lethal_squad_kills_a_fresh_character_quickly`
     (lines 262-313) and its doc comment (test 2).
  7. Docs: `docs/content/debug-area.md` lines 464-472 (the lethal squad
     paragraph only); `docs/analysis/npc-combat-reference/presentation-chain.md`
     (the 559 header line about 85, the effect paragraph about 97-101, and
     `559/621 (15/289)` about 222); `evidence-registry.md` (row `Ability 559`,
     rows `Effect 621` and `Effect 621 NVPs`, two new rows, and item O1);
     `uat-checklist.md` (Block 1 intro, row N3 and the D-NC8 bullet);
     `docs/guides/uat-specs/debug-area.toml`, row `NC-U1` `notes` only.
     Line numbers in the last three come from the NC-31 and NC-33 branches;
     find each line by Grep for `621` in your worktree.
- **Stays unchanged (check each with `git diff` before you commit):** the
  621 row (`effects.sql:7817-7819`); nvps 16 and 17 (`effect_nvps.sql:37`,
  `:39`, 20 and 200); 559's tooltip, `required_ammo` 3, `event_set_id` 15,
  cooldown and monikers; ability set 3 = {559}; every entity template and
  spawn row; `items_event_sets`; `crates/` outside the two test files;
  `docs/reverse-engineering/findings/combat-formulas-game-data-evidence.md`
  (its 621 row stays true).
- **Tests.**
  1. Live-DB, in `nid.rs` `mod live_db`:
     `live_db_reference_nid_559_uses_reconstructed_effect_5310`. Read 559's
     tooltip with
     `sqlx::query_scalar::<_, String>("SELECT description FROM resources.abilities WHERE ability_id = 559")`,
     take the number from the line that is `-<n>F` and from the line that is
     `-<n>H` (trim each line; `strip_prefix('-')`, `strip_suffix`), and
     assert they are `"100"` and `"10"`, so the tooltip itself is pinned.
     Load `crate::cell::spawner::load_ability_defs(&pool)` and
     `load_effect_defs(&pool)` (both return a `HashMap<i32, _>`; see
     `reference/jaffa.rs`) and assert:
     - 559's `effect_ids` is `vec![5310]`;
     - effect 5310: `ability_id` 559, `script_name`
       `Some("RangedPhysicalDamage")`, `event_set_id` 289,
       `param_i32("FocusDamage")` equal to the F number and
       `param_i32("HealthDamage")` equal to the H number;
     - effect 621 is the untouched Giza record: `param_i32("FocusDamage")`
       200, `param_i32("HealthDamage")` 20, script `RangedPhysicalDamage`.
     The doc comment says 5310 is RECONSTRUCTED (D-NC8, D-NC11), that 621 is
     kept on purpose, and gives both revert proofs. **Revert proofs (run
     both):** (i) put `'{621}'` back in `abilities.sql`, run
     `pwsh tools/build-lane/live-db-test.ps1 uses_reconstructed_effect_5310`,
     see it fail on `effect_ids`, restore; (ii) set nvp 17 to `'100'` (a
     rewrite of 621 in place), run it again, see it fail on 621, restore.
     Then check that `git diff` of the seeds shows only your change. The test
     inserts no rows, so it needs no sentinel id.
  2. Recalibrate the Z9 lethal-squad test. Halving 559 makes the four 1376
     operatives slower, so the 7-tick window will probably fail. Temporarily
     raise the loop bound to 30, run
     `pwsh tools/build-lane/live-db-test.ps1 lethal_squad --no-capture`, and
     read `fresh character dead after Some(m)` from the lane log. Set the
     window to `W = ceil(m * 7 / 5)` (the old 5-to-7 headroom), as one
     `const WINDOW: u32` used by the loop and the assert message. Update the
     doc comment ("within W AI ticks (2W s; it takes m when measured, after
     NC-25)"). W stays below 2m, so a squad thinned to two still fails. If m
     is more than 20, stop and return `blocked` with m: Z9's lethal promise
     is then an owner call.
  3. `HAS_MECHANICS_TODAY` stays 291: 559 still has a scripted effect with
     NVPs (5310 instead of 621), and the constant counts abilities, not
     effects. Run `live-db-test.ps1 seeded_has_mechanics_count` and do not
     edit `use_ability/tests/no_mechanics_live_db.rs`.
- **Lane, in order:**
  1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell`
  2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell --all-targets -- -D warnings`
  3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell debug_area`
  4. `python tools/ability_mechanics/effect_nvps_from_desc.py --check` (exit 0)
  5. `pwsh tools/build-lane/live-db-test.ps1 uses_reconstructed_effect_5310`,
     then both revert proofs
  6. `pwsh tools/build-lane/live-db-test.ps1 lethal_squad --no-capture`
  7. `pwsh tools/build-lane/live-db-test.ps1 seeded_has_mechanics_count`
  8. `pwsh tools/build-lane/live-db-test.ps1 live_db`, the whole tier. This is
     the blast-radius check for players and the 29 templates. A failure in a
     file you do not own is a finding: list the test and its message and
     return `blocked`. Do not patch it.
  9. `pwsh tools/build-lane/lane.ps1 cargo test -p cimmeria-lab committed_`
     (the uat-specs TOML lint, for the `debug-area.toml` edit)
- **Docs (doc-update map row "Abilities, effects"):**
  `docs/architecture/abilities-and-effects-system.md` needs no change (seed
  values only); say so in the report. Text for each owned docs line:
  - `debug-area.md`: "559, 100 Focus / 10 Health a shot (effect 5310,
    RECONSTRUCTED from its tooltip, D-NC11)", and "dies in m AI ticks, about
    2m s, ... the live-DB smoke allows W", with your measured m and W.
  - `presentation-chain.md`: the header line says effects `{5310}`. The
    effect paragraph says: effect 5310 `effects.sql:7824`, name
    `AW Auto Attack Damage`, desc `Single Target -100F -10H`, script
    `RangedPhysicalDamage`, event set 289, NVPs `HealthDamage` 10 and
    `FocusDamage` 100 (`effect_nvps.sql:1667`, `:1668`), RECONSTRUCTED from the
    559 tooltip (D-NC8, D-NC11, NC-25). Giza effect 621 (`effects.sql:7817`,
    -200F/-20H, NVPs 20/200 at `effect_nvps.sql:37`, `:39`) is kept unchanged
    and no ability uses it. Line 222 says `559/5310 (15/289)`. Cite the line
    numbers you actually wrote.
  - `evidence-registry.md`: row `Ability 559` says effects `{5310}` (was
    `{621}` until NC-25). Rows `Effect 621` and `Effect 621 NVPs` keep class
    EARLIEST LEGACY DATA and their values; drop "To be rewritten ... (D-NC8,
    O1)" and say "kept unchanged; no ability uses it since NC-25 (D-NC11)".
    Add rows `Effect 5310` and `Effect 5310 NVPs` after them, class
    RECONSTRUCTED, packet NC-25, with the values and lines above. O1 keeps
    its id and says "closed by NC-25: ability 559 now executes effect 5310,
    the tooltip's 10H/100F, RECONSTRUCTED (D-NC8, D-NC11); Giza effect 621
    is kept unchanged".
  - `uat-checklist.md` and `debug-area.toml`: the burst is 10 Health and
    100 Focus, executed by effect 5310 since NC-25 (D-NC8, D-NC11,
    RECONSTRUCTED). The Block 1 intro names effect 5310, not 621. Drop every
    "executes 20 Health and 200 Force today" and "record the observed value
    as O1" clause. F means Focus, not Force.
  - Check: Grep the owned docs for
    `200F|20 Health|200 Force|20H/200F|executes 20|\{621\}`. What remains may
    only describe the kept Giza row 621.
- **Shared files.** `debug-area.md` is shared with NC-22, which edits the
  lineup table near lines 1933-1960 only if it adds an actor. Your hunk
  (464-472) is far away and merges cleanly; edit nothing outside it.
  `evidence-registry.md` (NC-31), `uat-checklist.md` and the NC-U1 row of
  `debug-area.toml` (NC-33) are already merged in your base; edit only the
  lines named above. NC-30 touches none of your files: its code is in
  `combat_profile/`, `spawn.rs`, `ability_select.rs`, `fight.rs` and its
  `placeholder_policy.rs` test, and neither `nid.rs` nor `live_db.rs` is
  among them. NC-22's seeds (`entity_templates.sql` rows 159/160 and the
  lineup files) are not yours. Do not touch `reference/mod.rs`, the campaign
  README or RESUME.
- **Rules (inlined):** R1 PowerShell only, `Set-Location <worktree>;` first
  in every call. R2 every cargo call through `lane.ps1` in the foreground;
  live-DB through `live-db-test.ps1`; never plain `cargo`. R3 edit only the
  files above; no `git stash`, no `git worktree prune`, no push; one local
  commit. R4 seeds only (no `db/scripts` migrations or UPDATEs); keep each
  INSERT's columns in place. R5 files stay under 500 lines. R6 the seeds
  and `docs/` files are CRLF: after editing each one, run
  `$p='<absolute path>'; $nl=[string][char]13+[char]10; $t=[IO.File]::ReadAllText($p) -replace '\r?\n',$nl; [IO.File]::WriteAllText($p,$t)`
  and confirm 0 bare LF and a CRLF count equal to the line count. Do **not**
  write `"\r\n"` as a `-replace` replacement: PowerShell writes it as four
  literal characters and puts the whole file on one line. R7 live-DB tests
  use `require_db_or_skip!`, carry `live_db` in the fn or module name and get
  the pool from the macro. R10 5 reruns per check, then `blocked` with a
  worknote.

---

## Wave 4: live-client UAT at Z12 (ask first) and the docs consolidation

These are lab sessions, not packets. Each one needs D-NC7 and a fresh "may I
use the lab now?" to the user. The orchestrator runs them with the `lab-uat`
skill: a Haiku `lab-driver` with a gotcha sheet, and Sonnet verification
against the server log and packet tap. They need a lab client and a server
built from main after W3. Rows from `uat-checklist.md`:

| Session | Scope |
|---|---|
| NC-40 | NID rows |
| NC-41 | Jaffa rows |
| NC-42 | ScavDog rows |
| NC-43 | PRU rows |
| NC-44 | Telemetry rows (server log or SigNoz; no client driving) |

The results are attested into `uat-checklist.md` and the registry's
`## Runtime results` by one docs PR (worktree `nc-uat`). A failing row
becomes a fix packet from the architect; it is not fixed in the session.

### NC-32 NPC combat reference docs

- **Agent:** documentation-writer. **Host:** windows. **Depends:** NC-30 and
  NC-31 merged. It runs alongside the lab sessions, because it touches no
  lab file.
- **Branch:** `npc-combat-reference/NC-32-docs`. **Worktree:** `nc-32`.
- **Commit:** `docs(gameplay): NPC combat reference pipelines (NC-32)`
- **Files:** new `docs/gameplay/npc-combat-references.md`; the "see also"
  line in `docs/gameplay/npc-ai.md` and
  `docs/architecture/abilities-and-effects-system.md`. Walk
  `docs/agents/doc-update-map.md` and list the rows you touched.
- **Do:**
  - Write one page with the four pipelines: data → selection → effect →
    presentation, each with its Z12 test, its evidence class, and links to
    the registry and the RE findings.
  - Add the "how to add a new NPC combat profile" steps that NC-34's
    campaign will follow.
  - Mark every UAT row that is still pending, under "Not yet verified in the
    client".
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8, R9.

---

## Wave 5: campaign review and close-out

- **NC-50 Campaign review** (the campaign-packet skill § 4): packet-reviewer
  passes over everything merged. Fixes ship as one PR from the
  `npc-combat-reference-review` worktree.
- **NC-51 Close-out** (documentation-writer, worktree `nc-51`, commit
  `docs: close out the NPC combat reference campaign (NC-51)`). It updates:
  - `docs/gap-analysis*` and `docs/project-status.md`, once;
  - `docs/guides/unified-uat.md`, with any NC-U rows the owner still owes;
  - the README packet table and RESUME.md.

  Then `ops` retires every `nc-*` worktree and posts the board handoff.
