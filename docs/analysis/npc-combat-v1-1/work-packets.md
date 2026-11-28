# NPC combat V1.1: work packets

Ledger: [README.md](README.md). This file holds the dispatch rules, the rules every packet carries, the contract, wave 0 (foundation) and wave 1 (research). Waves 2-5 are in [work-packets-w2-w5.md](work-packets-w2-w5.md), which uses the rules and contract here.

- Campaign folder: `docs/analysis/npc-combat-v1-1/`.
- Branch prefix: `npc-combat-v1-1/`.
- Worktree names: `nx-<id>` (for example `nx-02`, `nx-12a`).
- Integration branch for wave 2: `npc-combat-v1-1/nx-w2`, worktree `nx-w2`.

## Dispatch rules

- `ops` makes one worktree and one test database (`sgw_<worktree>`) per packet: `pwsh tools/build-lane/mk-worktree.ps1 npc-combat-v1-1/<NX-xx>-<slug> nx-<xx>`.
- Every packet is `Host: windows`. A worker on another platform returns `blocked`.
- The report path is `<worktree>\.claude\reports\npc-combat-v1-1\<packet>-coder.md`; a domain agent uses the same path. The report has these sections:
  - `Summary`;
  - `Files`;
  - `Tests`: each test, and why it fails when the change is reverted;
  - `Lane`: the `status=` line of each check;
  - `Findings`: anything surprising, and anything left over.
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Research packets (wave 1) return `done` with UNRESOLVED rows when the evidence is not there. Absence is a finding, never a reason to guess.
- A verify-only packet (NX-20) that finds a failing check on current code keeps the assertion, marks it `#[ignore = "NX finding: <one line>"]` and returns `done` with the finding. The architect turns it into a packet.
- Waves 0 and 1 dispatch **together** (9 agents). Their file sets are disjoint.

## Rules (part of every packet)

- **R1** Work only in your worktree. Start every shell call with `Set-Location <worktree>;`. Use PowerShell, never Bash.
- **R2** Every compiling cargo call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`. Run live-DB tests with `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Never call `cargo` directly. Read the lane's summary and failures file; never rerun a check just to see its output.
- **R3** Edit only the files your packet names. No `git stash`, no `git worktree prune`, no push. Commit once, locally, with the subject given and the attribution lines from your brief.
- **R4** Game data lives in seeds under `db/resources/` only, never in `db/scripts` migrations or one-off UPDATEs. Keep each INSERT on one line, in the file's existing column order. Use only ids from C2.
- **R5** Files have a soft cap of 500 lines and a hard cap of 700. If a file would pass 500, split it along a natural seam.
- **R6** Files under `docs/` are CRLF. After writing one, normalise it: `$p='<absolute path>'; $nl=[string][char]13+[char]10; $t=[IO.File]::ReadAllText($p) -replace '\r?\n',$nl; [IO.File]::WriteAllText($p,$t)`. Always use absolute paths with `[IO.File]`. Never hand-edit a `<!-- gen:NAME -->` block.
- **R7** A behaviour change needs a test that fails when the change is reverted. Live-DB tests are gated with `require_db_or_skip!`, have `live_db` in the fn or module name, and take their URL from `test_support::database_url()`. Sentinel ids fit in `i32` (from `0x7000_0000`).
- **R8** A log line that carries an id carries its name next to it (for example `template_id` with `template_name`). No IPs, credentials or account names anywhere. Client paths appear only in the repo-relative form `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`.
- **R9** No packet before NX-41 touches `docs/gap-analysis*`, `docs/project-status.md` or `docs/guides/unified-uat.md`. No packet touches `docs/analysis/npc-combat-reference/` (V1 is closed).
- **R10** You get 5 reruns of any one lane check. After that, stop and return `blocked`, with a worknote in your report.
- **R11** Evidence: label every new fact with one of the six classes (README § Evidence classes). Never present RECONSTRUCTED as original. Every RECONSTRUCTED seed row carries the C5 comment.
- **R12** Never copy, rehost or commit client files (packages, paks, extracted assets). Cite them by package and export path.

Lane crates:

- `cimmeria-cell-world` (`crates/cell-world`)
- `cimmeria-cell-combat` (`crates/cell-combat`)
- `cimmeria-cell` (`crates/cell`)
- `cimmeria-resources` (`crates/resources`)

The standard check order is:

1. `cargo fmt -p X`
2. `cargo clippy -p X --all-targets -- -D warnings`
3. `cargo nextest run -p X <filter>`
4. the live-DB filter, if the packet has one

## Contract

### C1 V1 combat profile (inherited, unchanged)

The resolver is `crates/cell-world/src/cell/combat/combat_profile/mod.rs`, from V1 C1. V1.1 must not change it.

- `PLACEHOLDER_POLICY` is `PlaceholderPolicy::RequireWeaponEvidence` (NC-30).
- 592 Pistol Shot is reserved for the `HUMAN_FIREARM_PREFIXES` whitelist (D-NC13).
- A template with a non-empty ability set resolves `ExplicitSet` and logs nothing.
- An empty set without firearm evidence resolves `Unresolved` and logs `event = "combat_profile_unresolved"` once per template, on target `npc.combat_profile`.

```rust
pub fn resolve_npc_combat_profile(ability_ids: &[i32], components: Option<&[String]>, policy: PlaceholderPolicy) -> NpcCombatProfile;
pub fn report_profile(template_id: i32, template_name: &str, body_set: &str, profile: &NpcCombatProfile);
```

### C2 Id blocks and tags

These blocks were checked free on 2026-11-27, in the seeds and in the cooked catalogue keys. V2 owns the blocks after these ([npc-combat-v2 C2](../npc-combat-v2/work-packets.md#c2-id-blocks-and-tags)).

| What | Ids | Owner |
|---|---|---|
| Debug Area templates | 1390 Titan death-drill clone (only if NX-19 needs it); 1391-1394 reserved | NX-19 |
| Debug Area spawns | 14320 Titan (template 79), 14321 Horden (template 72); 14322-14339 reserved; footer floor 14339 | NX-03 |
| Spawn tags | `DebugArea_NT_Titan`, `DebugArea_NT_Horden` | NX-03 |
| Abilities | 3500 Titan Melee, 3501 Titan Ranged Attack, 3502 Create Tasker, 3503 Detonate Tasker; 3504-3509 reserved | NX-11 |
| Effects | 9170-9179 | NX-11 |
| Effect NVPs | 480-487 Titan; 488-489 Horden effect 1092 (only under D-NX2 option b) | NX-11, NX-17 |
| Ability sets | 360 Straegis Titan; 361 Horden (only under D-NX2 option b); 362-364 reserved | NX-13, NX-17 |
| Event sets | 1550-1559 | NX-12a |
| Sequences | 3030-3049 (not in the cooked pak, so each needs an NX-12b override) | NX-12a, NX-12b |
| Loot | loot table 40; loot rows 9160-9169 | NX-13 |

### C3 `SequenceOverride` NVPs (NX-02)

In `crates/resources/src/base/sequence_overrides.rs`:

```rust
pub struct SequenceOverride {
    pub sequence_id: u32,
    pub event_id: u32,
    pub kismet_script_name: &'static str,
    /// (Name, Value) pairs, emitted in this order as
    /// `<KismetScriptNVPs Value="{value}" Name="{name}"></KismetScriptNVPs>`,
    /// both attribute-escaped. Empty for the ring rigs.
    pub nvps: &'static [(&'static str, &'static str)],
}
```

`generate_sequence_xml` writes the NVP children between the opening tag's `>` and `</COOKED_KISMET_EVENT_SEQUENCE>`, Value before Name. That is the byte shape of the shipped `_2822` entry:

```text
...KismetScriptName="KIS-abilities_human.KIS-SA_Effect_Target" EventID="1001" KismetEventSetSeqID="2822"><KismetScriptNVPs Value="Straegis-VFX.Str-AuraChemical" Name="pfx1"></KismetScriptNVPs>...<KismetScriptNVPs Value="1" Name="useAnims"></KismetScriptNVPs></COOKED_KISMET_EVENT_SEQUENCE>
```

### C4 Test layout

The tests live in `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/creatures/`. NX-03 creates `mod.rs` and **every** file below as a stub holding a `//! <packet> fills this.` doc line, so later packets never share a file.

| File | Owner |
|---|---|
| `mod.rs` (`CREATURE_SPAWNS`, `creature_row`) | NX-03 |
| `titan_kit.rs` | NX-11 |
| `titan_presentation.rs` | NX-12a |
| `titan_set.rs` | NX-13 |
| `stance.rs` | NX-14 |
| `tasker.rs` | NX-15 |
| `titan.rs` | NX-16 |
| `horden.rs` | NX-17 |
| `v1_regression.rs` | NX-20 |

Reuse the V1 helpers from `debug_area_combat/live_db.rs` (`fight_space`, `spawn`, `pass`, `set_pools`, `with_prefix`). Live-DB tests sit in `mod live_db { ... }` inside the owner's file.

### C5 Seed comment for reconstructed rows

Put this line directly above each changed or added row:

```sql
-- RECONSTRUCTED (NX-11): Titan Melee; no original kit was cooked (README F7); anim DMelee_FINAL + Str-TitanMelee paired by name (STRONGLY INFERRED); damage per kit-spec.md
```

### C6 Evidence registry row (NX-01 defines the file)

One table per creature, with these columns:

```text
| Field | Value | Class | Source (file:line, package.export, or PR) | Packet |
```

The fields are: template id; body set; skeletal mesh, materials and textures; AnimSet and AnimTree; each animation role (idle, walk, run, combat idle, transition, melee, ranged, hit, death, special); particle systems; audio; ability and effect ids; event-set and sequence ids; collision and physics; loot; respawn; server path; runtime test result; lab result.

### C7 Telemetry (NX-14, NX-15)

Use target `npc.creature`, emitted once per transition, never per tick:

- `event = "creature_stance_changed"`, with `template_id`, `template_name`, `entity_id`, `from`, `to` and `cause`;
- `event = "tasker_created"` and `event = "tasker_detonated"`, with `template_id`, `template_name`, `entity_id`, `ability_id`, `ability_name`, the position, and `targets_hit` (detonate only).

Use `info!` for each. Logs must be readable from SigNoz alone.

---

## Wave 0: foundation (3 packets, all parallel, dispatched with wave 1)

The file sets are disjoint, and each packet ships as its own PR. Merge order: NX-02, NX-03, NX-01. The wave review covers NX-01..03.

### NX-01 V1.1 evidence registry skeleton

- **Agent:** packet-coder (docs only). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v1-1/NX-01-evidence-registry`. **Worktree:** `nx-01`.
- **Commit:** `docs(npc-combat): V1.1 evidence registry skeleton (NX-01)`
- **Files:** new `docs/analysis/npc-combat-v1-1/evidence-registry.md`.
- **Do:**
  1. Copy the six-class legend from README § Evidence classes.
  2. Add one C6 table for Horden (template 72) and one for the Straegis Titan (template 79).
  3. Fill each field only from what the seeds, the code and README F1-F11 say. Cite file:line for every seed fact:
     - `db/resources/Entities/Seed/entity_templates.sql:115` and `:160`;
     - `body_sets.sql` (the scales 1.68 and 35.15);
     - `skeletal_meshes.sql` (the sockets);
     - spawns 13306 and 14030, and lineup templates 1558 and 1570.
  4. Mark every client row the README findings do not settle as UNRESOLVED, naming the packet that settles it (NX-04 to NX-09).
  5. Add `## Gates` (copy G1-G5 from the README), `## Open (UNRESOLVED)` and `## Runtime results` (creature, test name, packet, result). NX-16, NX-17 and NX-30 fill the last two.
- **Lane:** none (docs). Run R6 on the file.
- **Docs:** this file is the docs row.
- **Rules:** R1, R3, R6, R8, R9, R11.

### NX-02 `SequenceOverride` NVPs

- **Agent:** packet-coder. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v1-1/NX-02-sequence-override-nvps`. **Worktree:** `nx-02`.
- **Commit:** `feat(resources): Kismet sequence overrides carry NVPs (NX-02)`
- **Files:** `crates/resources/src/base/sequence_overrides.rs` only, unless the grep in step 2 finds other struct literals.
- **Do:**
  1. Add the `nvps` field exactly as in C3.
  2. Set `nvps: &[]` in `debug_area_ring` (about line 73), in the two literal entries (10187 and 10188), and in every other `SequenceOverride {` literal that `rg "SequenceOverride \{" crates` finds. Edit only the files that grep names.
  3. Change `generate_sequence_xml` (about line 136) to build the string in three parts: the existing opening tag up to `>`, one NVP element per pair (each with `escape_xml_attr` on both value and name), then `</COOKED_KISMET_EVENT_SEQUENCE>`. Keep the existing `\n` after the XML declaration.
  4. Update the module doc to say that overrides can now carry NVPs, citing shipped entry `_2822`.
- **Tests** (unit, in the file's existing `mod tests`):
  1. `generated_xml_with_nvps_matches_shipped_2822_byte_for_byte`. Read the shipped bytes with `python -I -c "import zipfile;print(zipfile.ZipFile(r'<worktree>\data\cache\CookedDataKismetSeqEvent.pak').read('_2822').decode())"` and paste them into a `const` raw string. Build the override with sequence 2822, event 1001, script `KIS-abilities_human.KIS-SA_Effect_Target` and the five NVPs in the shipped order (`pfx1`, `pfx1Socket`, `cAnimOneShot`, `sAnimOneShot`, `useAnims`), then assert byte equality.
  2. `nvp_name_and_value_are_attribute_escaped`: the pair `("A&B", "<x\">")` is emitted as `Value="&lt;x&quot;&gt;" Name="A&amp;B"`.
  3. The existing `generated_xml_matches_a_shipped_entry_byte_for_byte` and `every_override_matches_its_seeded_sequence_row` still pass, unchanged except for `nvps: &[]`.
- **Revert proof:** dropping the NVP loop fails test 1; dropping the escaping fails test 2.
- **Lane:**
  1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-resources`
  2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-resources --all-targets -- -D warnings`
  3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-resources sequence`
- **Docs:** none. NX-12b documents the Titan entries.
- **Rules:** R1-R3, R5, R7, R10.

### NX-03 Debug Area Z13 creature range and test stubs

- **Agent:** rust-gameserver-dev. Placement iteration against the reach guards is too open-ended for packet-coder. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v1-1/NX-03-debug-area-z13`. **Worktree:** `nx-03`.
- **Commit:** `feat(debug-area): Z13 creature range for Horden and the Straegis Titan (NX-03)`
- **Read first:** `docs/content/debug-area.md`, especially § Z12 (about line 657) and the reach and tests sections. Then read `db/resources/Worlds/Seed/spawnlist_debug_area_reference.sql`, which is the shape to copy.
- **Files:**
  - new `db/resources/Worlds/Seed/spawnlist_debug_area_creatures.sql`;
  - `db/database.sql`: one `\ir` line after `spawnlist_debug_area_reference.sql` (about line 417);
  - `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/mod.rs` (add `mod creatures;`);
  - new `debug_area_combat/creatures/` with `mod.rs` and the eight stubs from C4;
  - `docs/content/debug-area.md`: a Z13 section and the zone-map row.
- **Spawns:** 14320 Titan (template 79) and 14321 Horden (template 72). For both rows:
  - world 1300, with the tags from C2;
  - `aggression_override` NULL, so they are aggressive;
  - `respawn_secs` 30;
  - `loot_table_id` NULL (NX-13 may set the Titan's);
  - a footer `setval(..., GREATEST(<current>, 14339))` in the form the reference file uses.
- **Placement:**
  - Use open, flat, navmesh-connected ground that no Z1-Z12 zone uses.
  - Place the Titan at least 120 m from every other world-1300 spawn, respawners 130 and 131, every ring station and the Z2 plaza. It is 35 m tall (scale 35.15), so it needs no wall within 40 m.
  - Place Horden at least 50 m from everything, including the Titan.
  - Ground the y values from the navmesh, as the existing seeds do.
  - Iterate until these pass:
    - `npc_ai::debug_area::reach` (`no_station_reaches_another`, `no_da03_hostile_reaches_a_respawner`, `no_npc_fighter_reaches_a_da03_target`);
    - `debug_area_combat`;
    - the gallery guard `live_db_debug_area_npcs`;
    - the lineup guard;
    - `live_db_seed_sequences`.
  - After 5 failed placement rounds, return `blocked` with the coordinates you tried.
- **Tests:**
  - no-DB, in `creatures/mod.rs`:
    1. `creature_range_holds_titan_and_horden`: the `world_records()` rows in `CREATURE_SPAWNS` are exactly the two tags, with templates {79, 72}, `respawn_secs` 30 and `aggression_override` `None`.
    2. `creature_pens_are_apart`: the Titan pen is at least 120 m from every other world-1300 spawn, and Horden is at least 50 m from everything.
    3. `creature_profiles_today`: `ability_ids` are empty for both templates. NX-13 and NX-17 change this test's expectation, and say so in a comment.
  - live-DB, in `creatures/mod.rs` `mod live_db`:
    1. `live_db_debug_area_creature_rows_load`: `load_spawns_from_db` filtered to `CREATURE_SPAWNS` gives 2 rows.
  - Revert proof: dropping the seed file or its `\ir` line fails tests 1 and 4.
- **Lane:**
  1. `fmt -p cimmeria-cell`
  2. `clippy -p cimmeria-cell --all-targets -- -D warnings`
  3. `nextest run -p cimmeria-cell debug_area`
  4. `pwsh tools/build-lane/live-db-test.ps1 debug_area`
  5. `pwsh tools/build-lane/live-db-test.ps1 seed_sequences`
- **Docs:** the `docs/content/debug-area.md` Z13 section (CRLF, R6) covers:
  - the purpose;
  - a table of tag, spawn, template and today's behaviour (both hold fire under D-NC1);
  - how to reach the range from the Z1 arrival (251, 8, -962);
  - a line saying the range is aggressive while gallery spawn 13306 stays neutral.

  Add a § Tests line for `creatures/`.
- **Rules:** R1-R8, R10. Never reuse an id outside C2.

---

## Wave 1: research (6 packets, all parallel, dispatched with wave 0)

All six are docs only, with disjoint files, and each ships as its own PR in any order. The wave review covers NX-04..09 and checks that their answers agree with each other.

**Shared research rules (inline these in each brief):**

- Load the `re-lookup` skill and use its order: repo docs (`docs/reverse-engineering/findings/`, `docs/protocol/`, `.claude/agent-memory/`), then the Ghidra MCP, then headless Ghidra.
- Use the tools catalog (`tools/README.md#tools-catalog`): `tools/upk_parser.py` (run with `python -I`; `--exports N --imports N --names N`), and the Rust `upk_info`, `extract_kismet`, `inspect-export`, `build-package-index` and `query-index` bins in `crates/upk` and `crates/upk-objects`, run through the lane.
- Write scratch output to your scratchpad, never into the repo.
- Model the findings doc on `docs/reverse-engineering/findings/scavdog-combat-assets.md`: package inventory, sequences by role (length, frames, rate), Kismet hooks, stance, sockets, collision, and a classified answer list.
- Write one agent-memory fact file under `.claude/agent-memory/game-archaeology-specialist/` (or your own agent's folder), plus its index line.
- Rules: R1, R3, R6, R8, R9, R11, R12. The lane applies only to the Rust bins.

### NX-04 Titan package assets

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** G5 (the owner's client tree; without it, return `blocked - no client tree`).
- **Branch:** `npc-combat-v1-1/NX-04-titan-assets`. **Worktree:** `nx-04`.
- **Commit:** `docs(re): Straegis Titan package assets (NX-04)`
- **Files:** new `docs/reverse-engineering/findings/straegis-titan-assets.md`, and the memory fact file.
- **Input:** `MOB_StraegisTitan.upk` (README F5), and `GLB_Components.upk` for its "StraegisTitan" reference.
- **Output must answer:**
  1. All 12 AnimSequence names (confirm the twelfth), each with its length, frame count, rate and AnimNotifies (sound cue names and timing).
  2. The BodySet `BodyStances[]`: each stance's key, `AnimationSet`, `Combat`, `MeleeDistance` and `Weapon`. Is `StraegisTitan_Anim` referenced? Which stance uses `s*` and which `D*`?
  3. The BodyComponent and BodySet `KismetData`, `BodyParticles` and `BodySounds` entries: each entry's particle system, FMOD event, animation and bone.
  4. The AnimTree's nodes, with the AnimName of each AnimNodeSequence (Idle, Walking, Running). Is there an AnimNodeSlot or any one-shot player?
  5. The socket name and bone, the PhysicsAsset bodies, and the approximate collision radius and height at scale 35.15. Also the client BodySet scale, if it carries one.
  6. What `GLB_Components.upk` references the Titan for.
  7. A verdict on F11 (a) versus (b), as far as the package alone shows it.
- **Lane:** only for the Rust bins. Run R6.

### NX-05 Titan VFX, the Straegis KIS and the tasker

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** G5.
- **Branch:** `npc-combat-v1-1/NX-05-titan-vfx-tasker`. **Worktree:** `nx-05`.
- **Commit:** `docs(re): Straegis Titan VFX, KIS and the tasker (NX-05)`
- **Files:** new `docs/reverse-engineering/findings/straegis-titan-vfx-and-tasker.md`, and the memory fact file.
- **Input:**
  - `Straegis-VFX.upk` (the 17 systems in F6);
  - `KIS-abilities_straegis.upk`, whose `KIS-SA_Beam_Source` uses SeqAct_PlayOneShot (`AnimationName`, `bSearchAnimTree`), SeqAct_SpawnEmitter, SeqAct_SetParticleSysParam and SeqAct_ComponentSlotData;
  - `KIS-abilities_human.upk` (`KIS-SA_Effect_Source` and `KIS-SA_Effect_Target`);
  - `Straegis.upk` (the 2008 super build), `Cine-Marsh_Straegis.upk` and every map package that names "Tasker" or "Titan".
- **Output must answer:**
  1. For each of the 17 systems: emitter count, looping or one-shot, duration, any named instance parameters, and the socket or bone it expects.
  2. How `KIS-SA_Effect_Source` and `KIS-SA_Effect_Target` consume `pfx1`, `pfx1Socket`, `cAnimOneShot`, `sAnimOneShot` and `useAnims`. Which actor does each play on (source or target)? What do the `c` and `s` prefixes mean?
  3. How `KIS-SA_Beam_Source` plays an animation (PlayOneShot with `bSearchAnimTree`), and whether that path would play a Titan `D*` sequence.
  4. The tasker: is there a tasker mesh, BodySet, template, Kismet or map object anywhere? What does `Str-TitanCreateTasker` show (a ground crystal, a projectile, a beam)? Is the detonation an area burst?
  5. Any Titan link in `Straegis.upk` or `Cine-Marsh_Straegis.upk` (sequence 1751 is a camera-only Matinee, per agent memory `reference_cellblock_asset_audit`).
- **Lane:** only for the Rust bins. Run R6.

### NX-06 Horden cross-build search and reconstruction options

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** G5.
- **Branch:** `npc-combat-v1-1/NX-06-horden-assets`. **Worktree:** `nx-06`.
- **Commit:** `docs(re): Horden assets, the missing AnimSet and the options (NX-06)`
- **Files:** new `docs/reverse-engineering/findings/horden-assets.md`, and the memory fact file.
- **Do:**
  1. Confirm F2 in every local tree: QA, `Client\game` and the editor download. Also check the UE3 source folders under `..\SGW\Src` and the SGW_FINAL indexes (agent memory `project_final_re_bundles`) for any Horden AnimSet, AnimTree, sound or effect. Absence in a partial build is "unknown", not "never existed".
  2. List the skeleton: bone count, names and hierarchy. Compare it with every other `MOB_*` skeleton in the QA tree for an exact-name match that could share an AnimSet. Expect none, and say so.
  3. State what the client renders for a skeletal mesh with no AnimSet: the reference pose. Mark it STRONGLY INFERRED unless NX-07 confirms it.
  4. Write the options table, with cost and gates for each:
     - (a) a static mesh only, the documented blocker;
     - (b) another build (where it might exist, and who could hold it);
     - (c) an authored AnimSet in an engine-3004 UE3 editor, shipped as a client patch (owner decision, RECONSTRUCTED);
     - (d) any procedural option the engine offers (for example a morph or a physics wobble), only if the package supports it.
  5. List every Horden spawn in the seeds (worlds and counts).
- **Output must answer:** D-NX1's facts (is there any recoverable animation?) and D-NX2's facts (is there any attack evidence?).
- **Lane:** only for the Rust bins. Run R6.

### NX-07 Client creature animation path (Ghidra)

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** none. It uses the Ghidra MCP or headless Ghidra (agent memory `reference_ghidra_headless`), not the live client.
- **Branch:** `npc-combat-v1-1/NX-07-creature-anim-path`. **Worktree:** `nx-07`.
- **Commit:** `docs(re): client creature animation and stance path (NX-07)`
- **Files:** new `docs/reverse-engineering/findings/creature-animation-path.md`, and the memory fact file.
- **Read first:**
  - `docs/reverse-engineering/findings/npc-attack-presentation.md` (BSF_InCombat, SequenceManager);
  - `reference-npc-presentation.md` (the BodyComponent KD model);
  - `scavdog-combat-assets.md` (O15, the stance);
  - `docs/protocol/*-dispatch-table.md`.
- **Output must answer** (cite the function address and name for each):
  1. How the client picks the AnimSet for an NPC from `BodySet.BodyStances[]`. Which server-sent property or flag selects the stance? Does `BSF_InCombat` switch an NPC between stances?
  2. Which node plays `cAnimOneShot` or `sAnimOneShot` from the generic KIS, and does it need an AnimNodeSlot in the AnimTree? Does `SGWAnim_SequenceListPlayer` play `s`-prefixed names?
  3. What a skeletal mesh with no AnimSet renders (this answers NX-06 step 3).
  4. Whether BlendBySpeed's Walking and Running nodes follow the server-driven NPC velocity.
  5. A verdict on README F11 (a) versus (b).
- **Lane:** none. Run R6.

### NX-08 Client handling of unknown NPC ability ids (Ghidra)

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v1-1/NX-08-unknown-ability-id`. **Worktree:** `nx-08`.
- **Commit:** `docs(re): client display of NPC ability ids outside its catalogue (NX-08)`
- **Files:** new `docs/reverse-engineering/findings/npc-unknown-ability-id.md`, and the memory fact file.
- **Output must answer:**
  1. Where the client turns an ability id into the name in the combat log, in the cast bar and in a hit message.
  2. What it does with an id that has no `CookedDataAbilities` entry (3500 and up): a blank name, "Unknown", a skipped message, an assert or a crash?
  3. Whether the cooked-data resync (`versionInfoRequest`, `onVersionInfo(InvalidateAll)`, then `resourceFragment`, #840) covers the abilities category the way it covers category 1. Cite `crates/resources/src/base/resources/apply_overrides.rs` and `dialog_overrides` for the server side.
  4. The cause of V1 anomaly K26 (a blank attacker name for templates 4 and 76): the missing template `name_id`, or something else.
- **Why:** it decides D-NX4 and NX-18.
- **Lane:** none. Run R6.

### NX-09 Server runtime trace for templates 72 and 79

- **Agent:** npc-ai-spawn-advisor. It is read-only on code and writes one doc. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-v1-1/NX-09-runtime-trace`. **Worktree:** `nx-09`.
- **Commit:** `docs(npc-combat): server runtime trace for Horden and the Titan (NX-09)`
- **Files:** new `docs/analysis/npc-combat-v1-1/runtime-trace.md`.
- **Do:** trace, with file:line for each step, what the server does for a spawned template 72 and 79:
  1. Spawn, then the C1 profile (`Unresolved`), then aggro acquisition. Does an `Unresolved` NPC still acquire aggro, chase or leash?
  2. Movement: the navmesh agent radius and step height used for an NPC. Does body scale 35.15 matter to pathing or reach? Does the server have a per-template reach or melee distance?
  3. Which flags and properties the server sends that the client uses for stance and presentation: `BSF_InCombat`, any stance property, `onSequence`.
  4. Whether gallery spawn 13306 (Z7) and lineup template 1570 stay passive when template 79 gains a set (the aggression override and the lineup passive flag).
  5. The code points NX-14 (a stance or phase switch) and NX-15 (a ground-point delayed area hit) would touch, and any existing mechanic each can reuse (deployables, ground-target AoE, `death/side_effects.rs`).
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8, R9, R11.
