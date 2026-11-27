# NPC combat reference: work packets, waves 3-5

Same dispatch rules, rules R1-R10 and contract C1-C6 as
[work-packets.md](work-packets.md). A packet here is not self-contained
until you have read that file's § Dispatch rules, § Rules and the contract
sections the packet names.

---

## Wave 3: flip, staff in place, registry, UAT spec, roster spec (5 packets, parallel)

The file sets are disjoint. Each packet ships as its own PR. Merge order:
NC-22, NC-30 (only after a green
`pwsh tools/build-lane/lane.ps1 --exclusive cargo nextest run --workspace <EXCL>`
on its branch; EXCL as in the lane-build skill), NC-31, NC-33, NC-34.

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
  `live-db-test.ps1 regression_harset`; `live-db-test.ps1 lineup`;
  `live-db-test.ps1 pet_roster`.
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
