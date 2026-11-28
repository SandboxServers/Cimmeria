# NPC combat V2: work packets

Ledger: [README.md](README.md). This file holds the dispatch rules, the rules every packet carries, the contract, wave 0 (foundation), wave 1 (research) and the NR-10 gate. Waves 2-6 are in [work-packets-batches.md](work-packets-batches.md), which uses the rules and contract here.

- Campaign folder: `docs/analysis/npc-combat-v2/`.
- Branch prefix: `npc-combat-v2/`.
- Worktree names: `nr-<id>`.
- Integration branches: `npc-combat-v2/nr-w2`, `nr-w3` and `nr-w4`.

## Dispatch rules

- `ops` makes one worktree and one test database per packet: `pwsh tools/build-lane/mk-worktree.ps1 npc-combat-v2/<NR-xx>-<slug> nr-<xx>`.
- **One worktree per creature:** a creature packet (NR-14, NR-22..25, NR-31) owns only its own creature's rows, files and tests.
- Every packet is `Host: windows`. A worker on another platform returns `blocked`.
- The report path is `<worktree>\.claude\reports\npc-combat-v2\<packet>-coder.md`. The report has these sections:
  - `Summary`;
  - `Files`;
  - `Tests`: each test, and why it fails when the change is reverted;
  - `Lane`: the `status=` line of each check;
  - `Findings`.
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Research packets return `done` with UNRESOLVED rows when the evidence is absent. Absence is a finding, never a reason to guess.
- A verify-only packet (NR-35) that finds a failing check on current code keeps the assertion, marks it `#[ignore = "NR finding: <one line>"]` and returns `done` with the finding.
- Waves 0 and 1 dispatch together. NR-03 waits for V1.1 NX-03 to merge, and NR-08 for V1.1 NX-04 and NX-05.

## Rules (part of every packet)

- **R1** Work only in your worktree. Start every shell call with `Set-Location <worktree>;`. Use PowerShell, never Bash.
- **R2** Every compiling cargo call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`. Run live-DB tests with `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Never call `cargo` directly. Read the lane's summary and failures file; never rerun a check just to see its output.
- **R3** Edit only the files and rows your packet names. No `git stash`, no `git worktree prune`, no push. Commit once, locally, with the subject given and the attribution lines from your brief.
- **R4** Game data lives in seeds under `db/resources/` only, never in `db/scripts` migrations or one-off UPDATEs. Keep each INSERT on one line, in the file's column order. To change a template, edit its INSERT row in place. Use only ids from C2.
- **R5** Files have a soft cap of 500 lines and a hard cap of 700. Split along a natural seam.
- **R6** Files under `docs/` are CRLF. After writing one, normalise it: `$p='<absolute path>'; $nl=[string][char]13+[char]10; $t=[IO.File]::ReadAllText($p) -replace '\r?\n',$nl; [IO.File]::WriteAllText($p,$t)`. Always use absolute paths with `[IO.File]`. Never hand-edit a `<!-- gen:NAME -->` block.
- **R7** A behaviour change needs a test that fails when the change is reverted. Live-DB tests are gated with `require_db_or_skip!`, have `live_db` in the fn or module name, and take their URL from `test_support::database_url()`. Sentinel ids fit in `i32` (from `0x7000_0000`).
- **R8** A log line that carries an id carries its name next to it. No IPs, credentials or account names anywhere. Client paths appear only repo-relative (`..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`).
- **R9** No packet before NR-51 touches `docs/gap-analysis*`, `docs/project-status.md` or `docs/guides/unified-uat.md`. No packet touches `docs/analysis/npc-combat-reference/` or `docs/analysis/npc-combat-v1-1/`.
- **R10** You get 5 reruns of any one lane check. After that, stop and return `blocked`, with a worknote in your report.
- **R11** Label every new fact with one of the six evidence classes (README § Purpose). Never present RECONSTRUCTED or STRONGLY INFERRED as original. Every reconstructed seed row carries the C5 comment.
- **R12** Never copy, rehost or commit client files. Cite them by package and export path.

Lane crates:

- `cimmeria-cell-world` (`crates/cell-world`)
- `cimmeria-cell-combat` (`crates/cell-combat`)
- `cimmeria-cell` (`crates/cell`)
- `cimmeria-resources` (`crates/resources`)

The standard check order is:

1. `cargo fmt -p X`
2. `cargo clippy -p X --all-targets -- -D warnings`
3. `cargo nextest run -p X <filter>`
4. the live-DB filter

## Contract

### C1 V1 combat profile (inherited; only NR-15 may change it)

`crates/cell-world/src/cell/combat/combat_profile/mod.rs`:

```rust
pub const PLACEHOLDER_POLICY: PlaceholderPolicy = PlaceholderPolicy::RequireWeaponEvidence;
pub const HUMAN_FIREARM_PREFIXES: &[&str] = &["WP-Human.WP_Pistol_", "WP-Human.WP_SMG_", "WP-Human.WP_AR_", /* ... */];
pub fn resolve_npc_combat_profile(ability_ids: &[i32], components: Option<&[String]>, policy: PlaceholderPolicy) -> NpcCombatProfile;
pub fn report_profile(template_id: i32, template_name: &str, body_set: &str, profile: &NpcCombatProfile);
```

- A template with a non-empty set resolves `ExplicitSet` and logs nothing.
- A NULL-set firearm human resolves to the 592 placeholder and logs `combat_profile_placeholder`.
- Every other NULL-set template resolves `Unresolved` and logs `combat_profile_unresolved`, on target `npc.combat_profile`, once per template.

A batch packet makes its templates resolve `ExplicitSet`, which is how the backlog shrinks.

### C2 Id blocks and tags

Checked free on 2026-11-27. V1.1 owns the blocks before these ([V1.1 C2](../npc-combat-v1-1/work-packets.md#c2-id-blocks-and-tags)).

| What | Ids | Owner |
|---|---|---|
| Debug Area templates | 1395-1399, 1620-1659 (only if a batch needs a Debug Area clone) | NR-10 assigns |
| Debug Area spawns | 14340-14399; footer floor 14399 | NR-03, then batches per NR-10 |
| Spawn tags | `DebugArea_NR_<Family>_<Name>` (for example `DebugArea_NR_Creature_Carnosaur`) | NR-03 |
| Ability sets | 365-372 weapon sets (NR-11); 373-379 mechanical (NR-13); 380-389 creatures (one per creature: 380 AMBRat, 381 Carnosaur, 382 Rhinolion, 383 Lenny, 384 Twilla, 385 Straegis Fighter, 386 Straegis Beacon); 390-399 Goa'uld and unarmed (NR-32, NR-33) | as listed |
| Abilities | 3510-3549, only with V1.1 D-NX4 in force | NR-10 assigns |
| Effects | 9180-9219 | NR-10 assigns |
| Effect NVPs | 490-549 | NR-10 assigns |
| Event sets | 1560-1599 | NR-10 assigns |
| Sequences | 3050-3099 (each needs an override, V1.1 C3) | NR-10 assigns |

Effect 1092 (983 Bite) gets its script and NVPs **once**. The first of V1.1 NX-17 (D-NX2 option b), V2 NR-14 (AMBRat) and V2 NR-22 (Carnosaur) that uses 983 Bite owns that edit. NR-10 checks `origin/main` and tells the other packet to reuse it.

### C3 Batch packet shape

Every batch and creature packet in waves 2-4 has this shape. NR-10 fills in the ids and rows.

1. **Set rows.** A batch with new sets puts its `ability_sets` and `ability_set_abilities` rows in a new file `db/resources/Abilities/Seed/ability_sets_npc_v2_<batch>.sql`, with one `\ir` line in `db/database.sql` after `ability_sets.sql`. A batch that reuses a V1 set (set 3, 4 or 6) adds no set rows.
2. **Template rows.** Edit `ability_set_id` in place on each template row that NR-10's table assigns to the packet. Put the C5 comment above each edited row. Edit no other column, and no row that another packet owns.
3. **Effects.** Reuse the ability's seeded effect. Add a payload only where NR-10 says so (D-NR8), in a new file `Effects/Seed/effect_nvps_npc_v2_<batch>.sql`.
4. **Tests,** in the batch's own file under C4:
   - a table-driven live-DB test that each assigned template resolves `ExplicitSet` with the expected set;
   - one fight test per family in the V1 `reference/` shape: the NPC casts only its set's abilities and never 592.
5. **Registry.** Fill the batch's family table in `docs/analysis/npc-combat-v2/evidence-registry.md` (C6).

Wave integration: two packets in one wave never edit the same row or adjacent rows. NR-10 checks adjacency in each seed file and gives a shared neighbourhood to one packet.

### C4 Test layout

The tests live in `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/roster/`. NR-03 creates `mod.rs` (`ROSTER_SPAWNS`, `roster_row`, and a `assert_set_for_templates(&[(template_id, set_id)])` helper) and every file below as a `//! <packet> fills this.` stub.

| File | Owner |
|---|---|
| `firearms.rs` | NR-12 |
| `mechanical.rs` | NR-13 |
| `ambrat.rs` | NR-14 |
| `placeholder_retired.rs` | NR-15 |
| `jaffa.rs` | NR-21 |
| `carnosaur.rs` | NR-22 |
| `rhinolion.rs` | NR-23 |
| `lenny.rs` | NR-24 |
| `twilla.rs` | NR-25 |
| `lineup.rs` | NR-26 |
| `straegis.rs` | NR-31 |
| `goauld.rs` | NR-32 |
| `unarmed.rs` | NR-33 |
| `hold_fire.rs` | NR-34 |
| `backlog.rs` | NR-35 |

Reuse the V1 helpers in `debug_area_combat/live_db.rs` (`fight_space`, `spawn`, `pass`, `set_pools`, `with_prefix`).

### C5 Seed comment

```sql
-- RECONSTRUCTED (NR-22): Carnosaur set 381 = 983 Bite; no client record pairs Bite with the Carnosaur (registry B4b); MOB_Carnosaur AnimSet present (ORIGINAL CLIENT VERIFIED, NR-05)
```

For a weapon-class mapping, use `-- STRONGLY INFERRED (NR-12): WP-Human.WP_Rifle_* -> set 366 (581 Rifle Auto Attack + 739 Rifle Melee AA), map in weapon-map.md`.

### C6 Evidence registry row (NR-02 defines the file)

One table per family, plus one per named creature, in this form:

```text
| Field | Value | Class | Source (file:line, package.export, or PR) | Packet |
```

The fields are: templates (world and lineup); body set; weapon component; AnimSet; attack animation; particle systems; audio; ability set and abilities; effect; event set; telemetry before and after; runtime test; lab result.

---

## Wave 0: foundation (3 packets, one PR each)

Merge order: NR-02, NR-01, NR-03 (after V1.1 NX-03). The wave review covers all three.

### NR-01 Backlog classification

- **Agent:** packet-coder (docs only; a scratch script is allowed). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v2/NR-01-backlog`. **Worktree:** `nr-01`.
- **Commit:** `docs(npc-combat): V2 backlog classification (NR-01)`
- **Files:** new `docs/analysis/npc-combat-v2/backlog.md`. If it would pass 500 lines, split it into `backlog-humans.md`, `backlog-jaffa.md` and `backlog-other.md`, with `backlog.md` as the index.
- **Do:**
  1. Write a Python script **in your scratchpad** (never in the repo) and run it with `python -I`. It uses the V1 inventory method (`docs/analysis/npc-combat-reference/placeholder-inventory.md` § Method) to parse every `INSERT INTO entity_templates` row in `db/resources/Entities/Seed/entity_templates*.sql` and every `spawnlist*.sql` row.
  2. Keep the templates with a NULL `ability_set_id` and at least one spawn row.
  3. Give each row these columns:
     - template id and name, with file:line;
     - body family (as V1);
     - weapon component (any `WP-*` part, and its class);
     - faction and alignment;
     - tags (interactive, vendor, trainer);
     - worlds and spawns;
     - today's C1 verdict (`placeholder` or `unresolved`);
     - **batch** (B1-B8 per README § Batches, or `out of scope` for friendly, interactive, vendor and trainer NPCs);
     - lineup twin, if any.
  4. Write a summary table of counts per batch. Reconcile them with the lab's 21 + 266 and explain any difference (for example lineup rows, or templates spawned only in worlds the lab never loaded).
  5. List the Jaffa and Goa'uld weapon components (`WP-Jaffa.*`, `WP-Goauld.*`, ribbon device, zat) that NR-10 needs.
- **Lane:** none. Run R6. Do not commit the script.
- **Rules:** R1, R3, R6, R8, R9, R11.

### NR-02 V2 evidence registry skeleton

- **Agent:** packet-coder (docs only). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v2/NR-02-evidence-registry`. **Worktree:** `nr-02`.
- **Commit:** `docs(npc-combat): V2 evidence registry skeleton (NR-02)`
- **Files:** new `docs/analysis/npc-combat-v2/evidence-registry.md`.
- **Do:**
  1. Write the six-class legend.
  2. Add C6 tables for each family: B1 firearm humans, B2 Jaffa, B3 mechanical, B6 Goa'uld, B7 unarmed, B8 hold fire. Add one table per named creature: AMBRat, Carnosaur, Rhinolion, Lenny, Twilla Tree, Straegis Fighter, Straegis Beacon, Android and Unas.
  3. Fill each table from README R1-R10 and the seeds only, citing file:line. Every client row is UNRESOLVED, naming the packet that settles it (NR-04..08).
  4. Add `## Gates` (A1-A5 from the README) and `## Runtime results`.
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8, R9, R11.

### NR-03 Debug Area Z14 roster range and test stubs

- **Agent:** rust-gameserver-dev. **Host:** windows. **Depends:** V1.1 NX-03 merged on main (it creates `spawnlist_debug_area_creatures.sql` and the Z13 docs NR-03 sits beside).
- **Branch:** `npc-combat-v2/NR-03-debug-area-z14`. **Worktree:** `nr-03`.
- **Commit:** `feat(debug-area): Z14 roster range for the V2 NPC families (NR-03)`
- **Files:**
  - new `db/resources/Worlds/Seed/spawnlist_debug_area_roster.sql`, and its `\ir` line after `spawnlist_debug_area_creatures.sql`;
  - `debug_area_combat/mod.rs` (`mod roster;`);
  - new `debug_area_combat/roster/` with `mod.rs` and every C4 stub;
  - `docs/content/debug-area.md` (a Z14 section and the zone-map row).
- **Spawns:** 14340 and up, one pen per family or creature, with the C2 tags:
  - AMBRat 74, Carnosaur 71, Rhinolion 75, Lenny 73, Twilla Tree 80;
  - Straegis Fighter 78 and Beacon 77;
  - Malfunctioning Drone 69, BattleWalker 70, Ancient Drone 81.

  NR-10 adds the human, Jaffa and Goa'uld representatives (from 14360). All are world 1300, aggressive (`aggression_override` NULL), with `respawn_secs` 20.
- **Placement:**
  - Use open navmesh ground that no Z1-Z13 zone uses.
  - Keep each pen at least 40 m from every other pen and spawn, and at least 120 m from the Z13 Titan.
  - Ground the y values from the navmesh.
  - Iterate until these pass: `npc_ai::debug_area::reach`, `debug_area_combat`, the gallery and lineup guards, and `live_db_seed_sequences`.
  - After 5 failed rounds, return `blocked` with the coordinates you tried.
- **Tests,** in `roster/mod.rs`:
  1. no-DB `roster_range_holds_the_v2_creatures`: the tags, templates and `respawn_secs` 20.
  2. no-DB `roster_pens_are_apart`.
  3. live-DB `live_db_debug_area_roster_rows_load`: the row count.
- **Revert proof:** dropping the seed or its `\ir` line fails tests 1 and 3.
- **Lane:**
  1. fmt `-p cimmeria-cell`
  2. clippy `-p cimmeria-cell --all-targets -- -D warnings`
  3. nextest `-p cimmeria-cell debug_area`
  4. `live-db-test.ps1 debug_area`
  5. `live-db-test.ps1 seed_sequences`
- **Docs:** the debug-area.md Z14 section: a tag, spawn, template and today's-behaviour table (all hold fire until their batch lands), and the route from the Z1 arrival. Run R6.
- **Rules:** R1-R8, R10.

---

## Wave 1: research (5 packets, all parallel, one PR each, any order)

**Shared research rules (inline these in each brief):**

- Load the `re-lookup` skill and use its order: repo docs (`docs/reverse-engineering/findings/`, `docs/protocol/`, `.claude/agent-memory/`), then the Ghidra MCP, then headless Ghidra.
- Tools: `python -I tools/upk_parser.py`, and the `crates/upk*` bins through the lane (tools catalog in `tools/README.md`).
- Scratch output goes to your scratchpad.
- Model the findings doc on `docs/reverse-engineering/findings/scavdog-combat-assets.md`: package inventory, AnimSequences by role, Kismet hooks, stance, sockets and collision, a classified answer list.
- Write one agent-memory fact file and its index line.
- Rules: R1, R3, R6, R8, R9, R11, R12.

### NR-04 Weapon class to auto-attack map

- **Agent:** combat-systems-advisor. It is read-only on code and writes one doc. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v2/NR-04-weapon-map`. **Worktree:** `nr-04`.
- **Commit:** `docs(npc-combat): NPC weapon class to auto-attack map (NR-04)`
- **Files:** new `docs/analysis/npc-combat-v2/weapon-map.md`.
- **Do:**
  1. For each `HUMAN_FIREARM_PREFIXES` entry, and each Jaffa and Goa'uld weapon component NR-01 lists, find the weapon item class, from the item seeds (`db/resources/Items/Seed/items.sql`, the weapon type columns) and the player auto-attack path (`crates/cell-combat`: which ability a player with that weapon type auto-attacks with).
  2. Map each class to its ranged and melee auto-attack (README R3): for example Rifle 581 and 739, and the AR and SMG classes to 559 and 595, as the NID Guard does.
  3. Note each ability's effect, range and cooldown, and its event set (presentation).
- **Output:** a table of prefix, weapon class, ranged AA, melee AA, effect, event set, class (STRONGLY INFERRED unless the code maps the weapon type directly, which makes it CURRENT CIMMERIA IMPLEMENTATION) and the proposed set (C2: 365-372).
- **Lane:** none. Run R6.

### NR-05 AMBRat, Carnosaur and Rhinolion assets

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** A4 (the client tree).
- **Branch:** `npc-combat-v2/NR-05-organic-assets`. **Worktree:** `nr-05`.
- **Commit:** `docs(re): AMBRat, Carnosaur and Rhinolion combat assets (NR-05)`
- **Files:** new `docs/reverse-engineering/findings/organic-creature-assets.md`, and the memory fact.
- **Output must answer, per creature:**
  1. The AnimSequence names by role, and which one is the attack (bite, claw, charge, ranged).
  2. The BodySet stances.
  3. KismetData, BodyParticles and BodySounds.
  4. Any cooked ability, effect or Kismet sequence that names the creature.
  5. A candidate seeded ability (983 Bite, 984 Infected Bite, 1433 Mob Strike or another seeded creature ability) whose name and text fit the attack animation, with a class for the pairing.
  6. Whether the AMBRat's hub-pass role (`docs/analysis/hub-pass-uat/README.md`) needs anything more.
- **Lane:** only for the Rust bins. Run R6.

### NR-06 Lenny, Twilla, drones, BattleWalker and Android assets

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** A4.
- **Branch:** `npc-combat-v2/NR-06-other-creature-assets`. **Worktree:** `nr-06`.
- **Commit:** `docs(re): Lenny, Twilla, drone, BattleWalker and Android combat assets (NR-06)`
- **Files:** new `docs/reverse-engineering/findings/mechanical-and-other-creature-assets.md`, and the memory fact.
- **Output must answer:**
  1. As NR-05, for Lenny (and the baby's BodySet), Twilla Tree, `CA_DroneTank`, the templates 69 and 81 drones, BattleWalker 70 and `AN_Android`.
  2. For the drones and BattleWalker: which drone abilities (1174, 1186, 1209, 1216 and 3283) and event sets fit, and does the BattleWalker have its own package?
  3. For Twilla: does it have any attack animation at all (D-NR10)?
  4. For the Android: confirm gate A1.
- **Lane:** only for the Rust bins. Run R6.

### NR-07 Goa'uld and Unas client evidence

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** A4.
- **Branch:** `npc-combat-v2/NR-07-goauld-unas`. **Worktree:** `nr-07`.
- **Commit:** `docs(re): Goa'uld and Unas NPC combat evidence (NR-07)`
- **Files:** new `docs/reverse-engineering/findings/goauld-and-unas-combat-evidence.md`, and the memory fact.
- **Output must answer:**
  1. For the Goa'uld templates NR-01 lists (or the seeds, if NR-01 has not landed): their weapon or hand-device components, and the ribbon device's client assets (KIS, particles, animation on the Goa'uld body).
  2. Any cooked ability, mission text or Kismet that names a Goa'uld NPC attack.
  3. For the Unas: any Unas attack animation, KIS or ability in the client (the `AR_J_Unas.*` parts and the Jaffa body's AnimSets).
  4. Whether the Jaffa body's fists animation exists for an NPC (D-NR6, D-NR1).
- **Lane:** only for the Rust bins. Run R6.

### NR-08 Straegis Fighter and Beacon as hostiles

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** V1.1 NX-04 and NX-05 merged (it reuses their KIS and VFX answers), and A4.
- **Branch:** `npc-combat-v2/NR-08-straegis-hostiles`. **Worktree:** `nr-08`.
- **Commit:** `docs(re): Straegis Fighter and Beacon as hostile NPCs (NR-08)`
- **Files:** new `docs/reverse-engineering/findings/straegis-fighter-beacon-hostile.md`, and the memory fact.
- **Output must answer:**
  1. Which pet abilities (1156, 2847, 1240) and summon or beacon abilities (2826, 3472, 3491-3497) present correctly on an NPC (not a pet) body, and why.
  2. Whether the Fighter and Beacon BodySets carry the `sEmitAura` and beam sequences their event sets name.
  3. A proposed set for each (C2: 385 and 386), with a class for each row.
- **Lane:** only for the Rust bins. Run R6.

---

## Gate: NR-10 Batch spec (re-slice checkpoint)

- **Agent:** architect. **Host:** windows. **Depends:** NR-01 and NR-04..07 merged. NR-08 may follow and amend only the B5 rows.
- **Branch:** `npc-combat-v2/NR-10-batch-spec`. **Worktree:** `nr-10`.
- **Commit:** `docs(npc-combat): V2 batch spec (NR-10)`
- **Files:** new `docs/analysis/npc-combat-v2/batch-spec.md`; README (decisions, findings, packet rows); `work-packets-batches.md` (amended sections); RESUME.md.
- **Do:**
  1. Give each wave 2-4 packet its exact rows: the template ids, with file and line; the set id; the abilities; any new effect; and its test rows.
  2. Check that no two packets in one wave edit the same or adjacent rows (C3).
  3. Cut any packet that would pass 40k working tokens. A batch with more than about 25 template rows splits by seed file.
  4. Assign the human, Jaffa and Goa'uld Debug Area representatives (spawns 14360 and up) to the batch packets that fight-test them.
  5. Settle the effect 1092 owner (C2).
- **Lane:** none. Run R6.
