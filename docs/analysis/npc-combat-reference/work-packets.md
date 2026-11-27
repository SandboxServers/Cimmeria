# NPC combat reference: work packets

Ledger: [README.md](README.md). This file holds the rules, the contract and
wave 0. Waves 1-2 are in [work-packets-w1-w2.md](work-packets-w1-w2.md) and
waves 3-5 in [work-packets-w3-w5.md](work-packets-w3-w5.md); both use the
rules and contract here. Campaign folder: `docs/analysis/npc-combat-reference/`. Branch
prefix: `npc-combat-reference/`. Worktree names: `nc-<id>` (for example
`nc-01`, `nc-21a`).

## Dispatch rules

- One worktree and one test database (`sgw_<worktree>`) per packet, made by
  `ops`: `pwsh tools/build-lane/mk-worktree.ps1 npc-combat-reference/<NC-xx>-<slug> nc-<xx>`.
- Every packet is `Host: windows`. A worker on another platform returns
  `blocked`.
- Report path: `<worktree>\.claude\reports\npc-combat-reference\<packet>-coder.md`
  (a domain agent uses the same path). The report has these sections:
  - `Summary`;
  - `Files`;
  - `Tests`: each test, and why it fails when the change is reverted;
  - `Lane`: the `status=` line of each check;
  - `Findings`: anything surprising, and anything left over.
- Return exactly one line:
  `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- A check that fails on **current** code in a verify-only packet (NC-11,
  NC-14, NC-23, NC-24) is a finding, not something to fix. Keep the failing
  assertion, mark it `#[ignore = "NC finding: <one line>"]`, and return `done`
  with the finding in the report. The architect turns it into a packet.

## Rules (part of every packet)

- **R1** Work only in your worktree. Start every shell call with
  `Set-Location <worktree>;`. Use PowerShell, never Bash.
- **R2** Every compiling cargo call goes through the lane, in the foreground:
  `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`. Run live-DB tests
  with `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Never call `cargo`
  directly. Read the lane's summary and failures file; never rerun a check
  just to see its output.
- **R3** Edit only the files your packet names. No `git stash`, no
  `git worktree prune`, no push. Commit once, locally, with the subject given
  and the attribution lines from your brief.
- **R4** Game data lives in seeds under `db/resources/` only, never in
  `db/scripts` migrations or one-off UPDATEs. Keep each INSERT on one line, in
  the file's existing column order.
- **R5** Files have a soft cap of 500 lines and a hard cap of 700. If a file
  would pass 500, split it along a natural seam.
- **R6** Files under `docs/` are CRLF. After writing one, normalise it:
  `$p='<absolute path>'; $t=[IO.File]::ReadAllText($p) -replace "\r?\n","\r\n"; [IO.File]::WriteAllText($p,$t)`.
  Never hand-edit a `<!-- gen:NAME -->` block.
- **R7** A behaviour change needs a test that fails when the change is
  reverted. Live-DB tests are gated with `require_db_or_skip!`, have
  `live_db` in the fn or module name, and take their URL from
  `test_support::database_url()`. Sentinel ids fit in `i32` (from
  `0x7000_0000`).
- **R8** A log line that carries an id carries its name next to it (for
  example `template_id` with `template_name`). No IPs, credentials or account
  names anywhere.
- **R9** No packet before NC-51 touches `docs/gap-analysis*` or
  `docs/project-status.md`.
- **R10** You get 5 reruns of any one lane check. After that, stop and return
  `blocked`, with a worknote in your report.

Lane crates: `cimmeria-cell-world` (`crates/cell-world`),
`cimmeria-cell-combat` (`crates/cell-combat`), `cimmeria-cell` (`crates/cell`).
The standard check order is `cargo fmt -p X`, then
`cargo clippy -p X --all-targets -- -D warnings`, then
`cargo nextest run -p X <filter>`, and then the live-DB filter if the packet
has one.

## Contract

### C1 Combat-profile resolver (NC-01; NC-10 adds C2)

Module `crates/cell-world/src/cell/combat/combat_profile/mod.rs`, unit tests
in `combat_profile/tests.rs`, declared `pub mod combat_profile;` in
`crates/cell-world/src/cell/combat/mod.rs`.

```rust
use super::aggression::NPC_DEFAULT_ABILITY; // 592 Pistol Shot

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceholderPolicy { Legacy, RequireWeaponEvidence }

/// The policy production runs. `Legacy` until D-NC1 / NC-30.
pub const PLACEHOLDER_POLICY: PlaceholderPolicy = PlaceholderPolicy::Legacy;

/// A component whose package starts with this is a weapon mesh.
pub const WEAPON_COMPONENT_PREFIX: &str = "WP-";
/// 592 Pistol Shot is a human firearm: under RequireWeaponEvidence only a
/// component starting with this may earn it.
pub const HUMAN_FIREARM_PREFIX: &str = "WP-Human.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatProfileSource { ExplicitSet, LegacyPlaceholder, Unresolved }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WeaponEvidence { None, Component(String) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcCombatProfile {
    pub abilities: Vec<i32>,
    pub source: CombatProfileSource,
    pub weapon_evidence: WeaponEvidence,
}

/// First component starting with WEAPON_COMPONENT_PREFIX, cloned.
pub fn weapon_evidence(components: Option<&[String]>) -> WeaponEvidence;

pub fn resolve_npc_combat_profile(
    ability_ids: &[i32],
    components: Option<&[String]>,
    policy: PlaceholderPolicy,
) -> NpcCombatProfile;

/// The AI-time fallback for an NPC that knows no ability:
/// Legacy → Some(NPC_DEFAULT_ABILITY); RequireWeaponEvidence → None.
pub fn placeholder_ability(policy: PlaceholderPolicy) -> Option<i32>;

/// Process-wide once-per-template throttle: true the first time a
/// template id is seen, false after. Static LazyLock<Mutex<HashSet<i32>>>,
/// the same pattern as INERT_LOG in
/// crates/cell-combat/src/cell/abilities/effect_plan.rs:57.
pub fn first_report(template_id: i32) -> bool;
```

The resolution table (`resolve_npc_combat_profile`). `weapon_evidence` is
always filled in from the components.

| `ability_ids` | Policy | Components | `abilities` | `source` |
|---|---|---|---|---|
| non-empty | any | any | `ability_ids.to_vec()`, order kept | `ExplicitSet` |
| empty | `Legacy` | any | `[592]` | `LegacyPlaceholder` |
| empty | `RequireWeaponEvidence` | one starting with `WP-Human.` | `[592]` | `LegacyPlaceholder` |
| empty | `RequireWeaponEvidence` | otherwise (none, `WP-Jaffa.*`, ...) | `[]` | `Unresolved` |

### C2 Diagnostics (NC-10)

`combat_profile/report.rs`:
`pub fn report_profile(template_id: i32, template_name: &str, body_set: &str, profile: &NpcCombatProfile)`.

- `ExplicitSet`: logs nothing.
- `LegacyPlaceholder`: when `first_report(template_id)`, log
  `tracing::info!(target: "npc.combat_profile", event = "combat_profile_placeholder", template_id, template_name, body_set, weapon_component, ability_id = 592, ...)`.
- `Unresolved`: when `first_report(template_id)`, log
  `tracing::warn!(target: "npc.combat_profile", event = "combat_profile_unresolved", template_id, template_name, body_set, weapon_component, ...)`.

Set `weapon_component` to the component string, or `"none"`. The message
text reads "NPC template has no ability set; using the Pistol Shot
placeholder" for a placeholder, and "NPC template has no ability set and no
weapon evidence; it holds fire" for an unresolved profile. The event names
are a telemetry contract that NC-44 greps for.

### C3 Id blocks and tags

| What | Ids | Owner |
|---|---|---|
| Debug Area templates | 1380 Jaffa Guard clone, 1381 Jaffa Lieutenant clone (1382-1389 reserved) | NC-03 |
| Debug Area spawns | 14110-14129 (14110-14114 used); the footer floor is 14129 | NC-03 |
| Spawn tags | `DebugArea_Ref_NID`, `DebugArea_Ref_PRU`, `DebugArea_Ref_ScavDog`, `DebugArea_Ref_JaffaGuard`, `DebugArea_Ref_JaffaLieutenant` | NC-03 |
| Ability set | 6 = `ScavDog: Mob Strike (RECONSTRUCTED, NC-13)` holding 1433 | NC-13 |
| Hand effect NVPs | 470-479 (470/471 for effect 736, 472/473 for effect 1717) | NC-12 |

These blocks were checked free on 2026-11-27: templates 1378-1399, NVPs
463-489 and spawns from 14110 hold no rows. The seed-sequence
guard floors (templates 1409, spawns 23199) already cover these blocks.

### C4 Test layout

Tests live under
`crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/reference/`.
NC-03 creates `mod.rs` and **every** file below as a stub holding a
`//! <packet> fills this.` doc line, so later packets never share a file.

| File | Owner |
|---|---|
| `mod.rs` | NC-03 |
| `nid.rs` | NC-11 |
| `pru.rs` | NC-14 |
| `jaffa.rs` | NC-12 |
| `scavdog.rs` | NC-13 |
| `melee_preference.rs` | NC-20 |
| `regression_tutorial.rs` | NC-23 |
| `regression_harset.rs` | NC-24 |
| `placeholder_policy.rs` | NC-30 |

NC-03 makes these `pub(super)` in `debug_area_combat/live_db.rs`:
`fight_space`, `spawn`, `pass`, `set_pools` and `with_prefix`. Reference code
reaches them as `super::super::live_db::*`. `reference/mod.rs` exports:

```rust
pub(super) const REF_SPAWNS: std::ops::RangeInclusive<i32> = 14110..=14129;
/// The one world-1300 row tagged `tag`, from `super::world_records()`.
pub(super) fn ref_row<'a>(records: &'a [SpawnRecord], tag: &str) -> &'a SpawnRecord;
```

Every `reference/*.rs` file declares its submodules with `mod nid;` and so on
in `reference/mod.rs`, written once by NC-03. Live-DB tests sit in a
`mod live_db { ... }` inside the owner's file.

### C5 Seed comment for reconstructed rows

Put this line directly above each changed or added row:

```sql
-- RECONSTRUCTED (NC-12): effect 736 desc '-100F -10H' -> HealthDamage 10, FocusDamage 100; script per #493 (effect 656)
```

### C6 Evidence registry row (NC-02 defines the file; NC-31 fills it)

One table per reference NPC, with these columns:

```text
| Field | Value | Class | Source (file:line, export, or PR) | Packet |
```

The fields are the brief's §8 list: template id; body set; weapon item and
component ids; ability and effect ids; event-set and sequence ids; animation
package and export; audio; client build; server path; runtime test result.
The class is one of the six in README § Evidence classifications.

---

## Wave 0: foundation and research (7 packets, all parallel)

The file sets are disjoint. Each packet ships as its own PR. Merge order:
NC-01, NC-03, NC-07, NC-04, NC-02, NC-05, NC-06.

### NC-01 Combat-profile resolver (pure)

- **Agent:** packet-coder. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-01-combat-profile`. **Worktree:** `nc-01`.
- **Commit:** `feat(cell-world): NPC combat-profile resolver (NC-01)`
- **Files:** new `crates/cell-world/src/cell/combat/combat_profile/mod.rs` and
  `combat_profile/tests.rs`; `crates/cell-world/src/cell/combat/mod.rs` (add
  `pub mod combat_profile;` beside `pub mod aggression;`; nothing else).
- **Do:** implement C1 exactly: the types, the constants, the four functions
  and the resolution table. There are no callers yet, so production behaviour
  is unchanged. Doc-comment the module: it says why 592 is a Giza/Python
  placeholder (cite `spawn.rs:314-317`), and that D-NC1 is the decision that
  flips `PLACEHOLDER_POLICY`.
- **Tests** (unit, `combat_profile/tests.rs`, wired with `#[cfg(test)] mod tests;`):
  1. An explicit set `[710, 584]` is kept in that order and is `ExplicitSet`
     under both policies.
  2. An empty set under `Legacy` gives `[592]` and `LegacyPlaceholder`, with
     no components and with `WP-Jaffa.WP_Staff_Plasma_4A`.
  3. An empty set under `RequireWeaponEvidence` with `WP-Human.WP_SMG_1A`
     gives `[592]` and `LegacyPlaceholder`.
  4. An empty set under `RequireWeaponEvidence` with
     `WP-Jaffa.WP_Staff_Plasma_4A` gives `[]` and `Unresolved`, and the
     evidence is `Component("WP-Jaffa.WP_Staff_Plasma_4A")`.
  5. An empty set under `RequireWeaponEvidence` with components `None`, and
     with `Some(&[])`, gives `Unresolved` with evidence `None`.
  6. `weapon_evidence` skips `BS_*` and `AR_*` and returns the first `WP-`
     component.
  7. `placeholder_ability` returns `Some(592)` and `None` for the two
     policies.
  8. `first_report(0x7000_0101)` is true and then false; a second id is true
     independently. Use unique sentinel ids per test, because the set is
     process-wide.
  9. `PLACEHOLDER_POLICY == PlaceholderPolicy::Legacy`. This pins D-NC1:
     NC-30 is the only packet that changes it.
- **Revert proof:** tests 2-5 pin the table, so swapping the arms or dropping
  the `WP-Human.` check fails them.
- **Lane:**
  1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
  2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
  3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world combat_profile`
- **Docs:** none. There is no runtime change; NC-10 documents the behaviour.
- **Rules:** R1-R3, R5, R7, R10.

### NC-02 Evidence-registry skeleton

- **Agent:** packet-coder (docs only). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-02-evidence-registry`. **Worktree:** `nc-02`.
- **Commit:** `docs(npc-combat): evidence registry skeleton (NC-02)`
- **Files:** new `docs/analysis/npc-combat-reference/evidence-registry.md`.
- **Do:**
  1. Copy the six-class legend from README § Evidence classifications.
  2. Add one C6 table per reference: NID 24; Jaffa 160 and 159 (one table
     with two value columns); ScavDog 76; PRU 4.
  3. Fill only what the seeds and the current code say, with file:line from
     `db/resources/...` and `crates/...`:
     - the body set, components, ability set and abilities, effects, event
       sets, `weapon_item_id`, faction, level and loot;
     - the class: seeds are EARLIEST LEGACY DATA and code is CURRENT CIMMERIA
       IMPLEMENTATION;
     - everything else is UNRESOLVED, with the packet that will settle it
       (NC-04, 05, 06 or 07).
  4. Look up monikers 1470900795 and 1383013887 (`ITEM_Staff`) and
     3175425141 (`ITEM_Automatic_Weapon`) in
     `db/resources/Entities/Seed/monikers.sql`, and cite them.
  5. Add an empty `## Open (UNRESOLVED)` list and a `## Runtime results`
     table (reference, test name, packet, result), which NC-31 fills.
- **Facts to copy** (verify each against the seed and cite it): the four
  reference tables in README F4-F7, plus effect 621 (desc '-200F -20H', NVP
  20H/200F, event set 289). Note the 559 tooltip discrepancy as D-NC8.
- **Lane:** none (docs). Run R6 on the file.
- **Docs:** this file is the docs row.
- **Rules:** R1, R3, R6, R8, R9.

### NC-03 Debug Area Z12 "Combat reference range"

- **Agent:** rust-gameserver-dev. Placement iteration against the reach
  guards is too open-ended for packet-coder. **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-03-debug-area-z12`. **Worktree:** `nc-03`.
- **Commit:** `feat(debug-area): Z12 combat reference range (NC-03)`
- **Read first:** `docs/content/debug-area.md`, the zone map, § Enemy
  gallery, § Reach (about line 651) and § Tests (about line 669);
  `db/resources/Entities/Seed/entity_templates_debug_area_combat.sql`
  (template 1371).
- **Files:**
  - new `db/resources/Entities/Seed/entity_templates_debug_area_reference.sql`;
  - new `spawnlist_debug_area_reference.sql`, in the directory that holds
    the other `spawnlist_debug_area_*.sql` files;
  - `db/database.sql`: two `\ir` lines, the templates one after the last
    `entity_templates_debug_area_*` line (about line 330) and the spawnlist
    one after the last `spawnlist_debug_area_*` line (about line 412);
  - `crates/cell/src/cell/service/tests/npc_ai/debug_area_combat/mod.rs`
    (add `mod reference;`);
  - `debug_area_combat/live_db.rs` (C4 visibility only);
  - new `debug_area_combat/reference/` with `mod.rs` and the eight stubs
    from C4;
  - `docs/content/debug-area.md`: a Z12 section and the zone-map row.
- **Templates:** 1380 and 1381 are copies of the 1371 row with only these
  changes:
  - 1380: `faction` 10, `name_id` 8917, `template_name`
    `'DebugArea Ref Praxis Jaffa Guard'`, `level` 5, `ability_set_id` 4,
    `respawn_secs` 20;
  - 1381: the same, with `name_id` 8918 and `template_name`
    `'DebugArea Ref Praxis Jaffa Lieutenant'`.

  Keep body, components (with `WP-Jaffa.WP_Staff_Plasma_4A`), colours, tint
  and mesh byte-identical to 1371, so
  `live_db_debug_area_lineup.rs::debug_area_lineup_live_db_every_look_has_an_actor`
  needs no new actor. Add a footer that keeps the template sequence past
  1389, in the form the `entity_templates_debug_area_combat.sql` footer uses
  (`GREATEST`).
- **Spawns:** 14110 NID (template 24), 14111 PRU (4), 14112 ScavDog (76),
  14113 Jaffa Guard (1380) and 14114 Jaffa Lieutenant (1381):
  - world 1300, tags as in C3, `aggression_override` NULL (template default,
    so aggressive), `respawn_secs` 20, `loot_table_id` NULL (template
    default);
  - copy the column list from the existing Debug Area spawnlist file;
  - add a footer `setval(..., GREATEST(<current>, 14129))` in the same form
    as the file that holds spawns 14100-14105 (grep `14109` under
    `db/resources`).
- **Placement:**
  - Choose an open, flat, navmesh-connected site that no zone Z1-Z11 uses.
    Z11 is planned for props; take a site no doc claims.
  - Give each reference its own pen: NID, PRU, ScavDog, and the Jaffa pair
    together. Pens are at least 35 m apart, and every pen is at least 50 m
    from every other world-1300 spawn, respawners 130 and 131, the ring
    stations and the Z2 plaza.
  - Ground the y values from the navmesh, the way the existing seeds do.
  - Iterate until all of these pass: `npc_ai::debug_area::reach`
    (`no_station_reaches_another`, `no_da03_hostile_reaches_a_respawner`,
    `no_npc_fighter_reaches_a_da03_target`), `debug_area_combat`
    (death_respawn: no NPC pulls a player at respawner 130 or 131), the
    gallery guard (`live_db_debug_area_npcs`), the lineup guard and
    `live_db_seed_sequences`.
  - After 5 failed placement rounds, return `blocked` with the coordinates
    you tried.
- **Tests:**
  - no-DB, in `reference/mod.rs`:
    1. `reference_range_holds_the_four_references`: `world_records()` rows in
       `REF_SPAWNS` are exactly the five tags, with templates
       {24, 4, 76, 1380, 1381}, `respawn_secs` 20 and `aggression_override`
       `None`.
    2. `reference_pens_are_apart`: the pairwise distance between pens is at
       least 35 m (the Jaffa pair counts as one pen).
    3. `reference_profiles_today`: `ability_ids` are 24 → [559], 4 → [221],
       1380/1381 → [584, 710] and 76 → [] (NC-13 changes the last).
  - live-DB, in `reference/mod.rs` `mod live_db`:
    4. `live_db_debug_area_reference_rows_load`: `load_spawns_from_db`
       filtered to `REF_SPAWNS` gives 5 rows; 1380 and 1381 have faction
       `HOSTILE_FACTION`.
  - Revert proof: dropping either seed file or its `\ir` line fails tests 1
    and 4.
- **Lane:**
  1. `fmt -p cimmeria-cell`
  2. `clippy -p cimmeria-cell --all-targets -- -D warnings`
  3. `nextest run -p cimmeria-cell debug_area`
  4. `pwsh tools/build-lane/live-db-test.ps1 debug_area`
  5. `pwsh tools/build-lane/live-db-test.ps1 seed_sequences`
- **Docs:** the `docs/content/debug-area.md` Z12 section (CRLF, R6) has:
  - the purpose (the NC reference range);
  - a table of tag, spawn, template, set, abilities and the expected
    behaviour today;
  - the player fixtures to reuse: the Z2 ability granter (template 1300,
    spawn 13020), the Basic Equipment Quartermaster (300, 13001) and the
    munitions vendor (1302, 13006);
  - how to reach the range from the Z1 arrival;
  - a line saying the range is aggressive and that the gallery (Z7) stays
    neutral.

  Add a § Tests line for `reference/`.
- **Rules:** R1-R8, R10. Seed rule: never reuse an id outside C3.

### NC-04 Seed presentation-chain trace

- **Agent:** packet-coder (docs only, no client). **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-04-presentation-chain`. **Worktree:** `nc-04`.
- **Commit:** `docs(npc-combat): seed presentation chain for the references (NC-04)`
- **Files:** new `docs/analysis/npc-combat-reference/presentation-chain.md`.
- **Do:**
  1. Trace each chain through `db/resources/Abilities/Seed/abilities.sql`
     and `db/resources/Effects/Seed/effects.sql` (the `event_set_id`
     column), then `db/resources/Events/Seed/event_sets.sql`,
     `event_sets_sequences.sql`, `sequences.sql` (`kismet_script_name`) and
     `sequences_nvp.sql`.
     - abilities: 559, 584, 710, 1433, 983 and 221;
     - effects: 621, 646, 736, 1717, 1092 and 264.
  2. Write one table per ability: ability, then its event set, sequences,
     kismet script names and NVPs; then its effects, each with its event set,
     sequences and kismet script names.
  3. Cross-check each link against
     `docs/reverse-engineering/findings/ability-animation-links.md` and keep
     its tier label. A link that exists only in the seed is EARLIEST LEGACY
     DATA.
  4. Close with § Gaps: chains that end with no sequence (1092 has no event
     set), and any `kismet_script_name` that names a creature or weapon
     family different from its NPC. NC-06 checks those in the client.
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R9.

### NC-05 ScavDog client assets (needs the owner's client)

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** the
  owner's client tree `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`
  (not in git). If the tree is absent, return `blocked - no client tree`.
- **Branch:** `npc-combat-reference/NC-05-scavdog-assets`. **Worktree:** `nc-05`.
- **Commit:** `docs(re): ScavDog combat assets and audio ownership (NC-05)`
- **Files:** new `docs/reverse-engineering/findings/scavdog-combat-assets.md`
  and `.claude/agent-memory/game-archaeology-specialist/` (one fact file,
  plus its index line).
- **Do:**
  1. Check the tools catalog (`tools/README.md#tools-catalog`) and
     `docs/client/audio-voice-inventory.md` §7 first.
  2. From the `MOB_ScavDog` packages, list the AnimSet and AnimSequence
     exports by role (idle, move, combat idle, attack, hit reaction, death),
     the AnimNotifies on each attack sequence (sound cue names and timing),
     the skeletal mesh's collision or physics asset, and any bone that bounds
     the bite reach.
  3. Establish whether each of the five `mob_scav.fev` attack events is
     referenced by a ScavDog notify. Do not infer it from file names.
  4. Search the cooked ability and effect paks for 1433 and 983 and any
     creature link.
  5. Classify every row with the six classes and cite each by package and
     export path.
- **Output must answer:**
  - Is the 1433 → event set 300 (generic melee) link supported, contradicted
    or unknown?
  - Is 983 Bite tied to ScavDog?
  - What is the physical attack reach in metres, if it is derivable?
- **Lane:** none. Run R6.
- **Rules:** R1, R3, R6, R8 (no local client paths beyond the
  repo-relative form above). Never rehost or copy client files into the repo.

### NC-06 Staff, PRU and NID presentation assets (needs the owner's client)

- **Agent:** game-archaeology-specialist. **Host:** windows. **Depends:** the
  owner's client tree, as in NC-05.
- **Branch:** `npc-combat-reference/NC-06-presentation-assets`. **Worktree:** `nc-06`.
- **Commit:** `docs(re): reference NPC weapon presentation assets (NC-06)`
- **Files:** new `docs/reverse-engineering/findings/reference-npc-presentation.md`.
- **Do:**
  - For `WP-Jaffa.WP_Staff_Plasma_4A`: the mesh sockets (muzzle, grip), the
    projectile or particle system, and the KIS source and target sequences
    behind event sets 3, 300 and 301.
  - For `BS_JaffaMale`: whether the body has melee AnimSequences that set 300
    can drive.
  - For `WP-Human.WP_SMG_1A`: the muzzle socket, flash, tracer and the burst
    sequence for event sets 15 and 289.
  - For `MOB_DroneFlyer00`: the emitter socket, and the beam or projectile
    for event sets 802 and 803.
  - Answer NC-04 § Gaps where you can.
  - Classify and cite every row as in NC-05.
- **Lane:** none. Run R6.
- **Rules:** as NC-05.

### NC-07 Runtime blockers re-verified

- **Agent:** combat-systems-advisor (read-only on code, writes one doc).
  **Host:** windows. **Depends:** none.
- **Branch:** `npc-combat-reference/NC-07-runtime-blockers`. **Worktree:** `nc-07`.
- **Commit:** `docs(npc-combat): re-verify NPC combat runtime blockers (NC-07)`
- **Files:** new `docs/analysis/npc-combat-reference/runtime-blockers.md`.
- **Do:** for each suspect, give `confirmed` / `fixed` / `not a bug`, with
  file:line, and the smallest fix as a proposed packet (one paragraph, with
  files and a test):
  1. NPC mitigation 0/0 against player damage.
  2. Damage type forced to `DT_PHYSICAL`, which would make 221
     `RangedEnergyDamage` land as physical.
  3. A DoT kill fires no `entity_death`.
  4. NPC LOS before firing, and target validation (dead, out of the world,
     friendly).
  5. Cooldown use by NPCs (the 2 s AI cadence against 1.5-3 s cooldowns).
  6. The `effect_inert` diagnostic (`effect_plan.rs:328`): that it fires for
     736 and 1717 today.

  Do not fix anything.
- **Lane:** none. Optionally run one existing test through the lane to
  confirm a claim (R2).
- **Rules:** R1-R3, R6, R9.
