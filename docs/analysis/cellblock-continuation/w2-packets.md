# Cellblock continuation: wave 2 packets

> **Type:** Work packets (canonical, on `content/cellblock-campaign`)
> **Audience:** The Cellblock orchestrator, and the agents it dispatches for CB-N04s, CB-N04m, CB-N04a and W2-INT, and their reviewers
> **Last updated:** 2026-10-17
> **Companions:** [work-packets.md](work-packets.md) (dispatch rules, waves), [README.md](README.md) (status, D-CC decisions), [route-baseline.md](route-baseline.md) (candidate pockets, protected areas), [CB-N04 contract](../four-zone-planning/cellblock-work-packets.md#cb-n04--purposeful-ambient-cast-one-safe-cluster)

Wave 2 covers CB-N04 (the ambient cast) and CB-N07 (one room's encounter response). Only CB-N04 can move now.

- **W1c is not triggered.** CB-N02s2 found no new restore gaps, and CB-N03s found no static fault. Neither sealed a child.
- **CB-N04, cluster 1 only.** The contract allows one room cluster per child. W2 seals and builds one cluster. A second cluster (CB-N04b) is considered only after CB-U08 and CB-U09 show it helps (O14-12: conservative density).
- **CB-N07 gets no packet.** It is BlockedEvidence on CB-U05 (N03 reachability) and CB-U09 (O14-15 pacing), and BlockedDecision on D-CC11 (the owner sets the death, difficulty and time impact). A later architect pass seals CB-N07s once all three land. See [§ CB-N07](#cb-n07--held).

## Dispatch plan

| Step | Packet | Agent | Worktree / branch | Runs after |
|---|---|---|---|---|
| 1 | [CB-N04s](#cb-n04s--seal-the-cluster-1-cast) | `npc-ai-spawn-advisor` | `cellblock-cb-n04s` / `cellblock/cb-n04s-cast-seal` | Worktree on feature SHA `0fca3114f` |
| 2 | [CB-N04m](#cb-n04m--mission-sign-off-on-the-seal) | `mission-systems-advisor` | the same worktree, after CB-N04s commits | CB-N04s returns `done` with verdict `sealed` |
| 3 | [CB-N04a](#cb-n04a--build-the-cluster-1-cast) | `packet-coder` | `cellblock-cb-n04a` / `cellblock/cb-n04-<room>` | CB-N04m returns `done` with verdict `approve` |
| 4 | Review | `packet-reviewer`, read-only | all three commits | CB-N04a returns `done` |
| 5 | Fixes, then re-review | `packet-coder`, then a fresh `packet-reviewer` | the owning worktree | Only on a `fix` verdict; at most 3 rounds (D-CC6) |
| 6 | [W2-INT](#w2-int--integrate-wave-2) | `packet-coder` | `zone-cellblock` | Every packet has `ship` |
| 7 | CB-U08 lab row | `lab-driver` via the orchestrator | lab | **Ask the user first.** Runs on the integrated branch |

Steps 1 to 3 are sequential because each one reads the previous one's output. The wave is small (three packets), so one reviewer takes all of it.

**Stop rules.**

- If CB-N04s returns verdict `no safe pocket`, CB-N04 becomes BlockedEvidence on CB-U08. Skip steps 2 to 6. The orchestrator asks the user for a lab run that measures the candidate pockets live.
- If CB-N04m returns `change`, it has already edited the seal. The orchestrator then dispatches CB-N04a on the changed seal. Do not re-run CB-N04s.
- If CB-N04m returns `reject`, return the seal to the architect.

## Contract

| Item | Value |
|---|---|
| Feature branch | `content/cellblock-campaign` (worktree `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock`) |
| Recorded feature SHA for W2 | **`0fca3114f2c93ed3abdb0e7bab285dfdf9ee92cb`** (W1b integrated, 2026-10-16), or the later architect commit that adds this file. Ops records the exact SHA it branched from. |
| Worktree recipe | 1. Ops: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <wt-name>` (branches from `main`). 2. Ops: `git -C <wt> rebase --onto <base> HEAD <branch>`. `<base>` is the feature SHA for CB-N04s, and **the CB-N04s branch tip after CB-N04m** for CB-N04a, so that the coder's worktree contains the seal. 3. Verify `git -C <wt> rev-parse HEAD` = `<base>` and `git -C <wt> status --porcelain` is empty. Stop on any mismatch; never reset. |
| Report path | `<wt>\.claude\reports\cellblock-continuation\<packet>-<role>.md` |
| Return line | Exactly one line: `<packet> <done\|blocked\|failed> <commit-sha\|-> <report-path>` |
| Measurement | Every report ends with `## Measurement`: working tokens (or `not exposed`), wall-clock start and end (UTC, `Get-Date -AsUTC`), lane runs, review rounds. |
| Claimed IDs (2026-10-17) | Spawns **520, 521, 522** (world 12); holder dialog **60160**; screens **200160, 200161, 200162**; chains **11010, 11011, 11012**. Recorded in [id-allocation.md § CB-N04 claims](../four-zone-planning/id-allocation.md#cb-n04-claims--2026-10-17). A `git grep` of all 740 local branches found none of them. Use the lowest IDs you need; never pick another. |
| Ambient bark shape | `enter_region` trigger on an **existing** point set key (exact string: every Cellblock region is `Castle_Cellblock.RegionN` with a lowercase b, except `Castle_CellBlock.Region8`), scope `player`, **`once` true**. Mission-state conditions decide the phase. One `npc_bark` action: `{"screen_id": N, "speaker": "<name>", "channel": "say"}`. The server reads the text from `dialog_screens` and sends it only to the crossing player's chat (onPlayerCommunication). Precedent: chains 1176-1178. |
| Why `once` true | The H52 step-activation replay re-fires `enter_region` for mission-gated chains when a step activates while the player stands in the region. `once` stops a second line in the same space visit. The mission gate stops it across visits and relogs. The fired-once set is in memory only. |
| Telemetry (owner rule, 2026-10-13) | Progress events, all existing: span `content.npc_bark`, INFO `Content: npc bark` with `chain_id` and `screen_id` (the line was spoken). The executor's WARN with `reason` = `screen_not_cached`, `empty_text` or `actor_not_player` (refused). The spawner's per-entity spawn log line (CB-N04s names it). No new telemetry code. |
| Holder dialog | Seed-only dialog 60160, never displayed, so it needs no `DIALOG_OVERRIDES` entry (new overrides are quarantined). Screens 200160+ hang off it. Precedent: dialog 60102. |

### Repo rules every packet carries

1. **PowerShell only**, no bash. Begin every command with `Set-Location <wt>;` and use absolute paths. `[IO.File]` calls take absolute paths only.
2. **Never run `cargo` directly.** Every fmt, compile, test or clippy goes through `pwsh tools/build-lane/lane.ps1 cargo ...`. Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<wt>` database from seeds first. Run lanes in the foreground and read their logs under `logs/<worktree-name>/`.
3. **No `git stash`, no `git worktree prune`, no `git reset`, no push.** Commit locally once the checks pass.
4. **Seeds are the source of truth.** Edit `db/resources/` only. Never add `db/scripts/*.sql`.
5. **File caps** apply to Rust and to new docs: soft 500 lines, hard 700. Check `(Get-Content <file>).Count` before and after. Seed SQL files and the existing `docs/content/mission-chains.md` (1956 lines) are exempt: append to them, never split them.
6. **Line endings.** Docs under `docs/` are CRLF. After writing one, run `$p='<abs path>'; $t=[IO.File]::ReadAllText($p) -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($p,$t)`; after `git add`, `git -C <wt> ls-files --eol <path>` must show `w/crlf`. `castle_cellblock_chains.sql` is stored CRLF (`i/crlf`). `spawnlist.sql`, `dialogs.sql` and `dialog_screens.sql` are `i/lf w/crlf`. For every seed file, `git -C <wt> diff --stat <path>` must count only your added lines; a whole-file count means the endings broke. Rust files are `i/lf`; the Edit tool keeps them.
7. **Edit only the files your packet lists.** Any other file needed means `blocked`. Never edit the seed's top-of-file chain-range header or `id-allocation.md` (D-CC8: W2-INT owns them).
8. **IDs come only from the contract's claimed list.** Never pick a free ID yourself.
9. Run `git -C <wt> diff --check` before committing.
10. If a lane check fails twice for the same reason, stop and report `failed` with the log path. Do not widen the change to make it pass.

---

## CB-N04s — Seal the cluster 1 cast

| Field | Value |
|---|---|
| Agent | `npc-ai-spawn-advisor` |
| Size | About 40k working tokens. Static, plus `nav_inspect` runs through the lane. No lab, no Ghidra, no code. |
| Branch / worktree | `cellblock/cb-n04s-cast-seal` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04s` |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md` (new) only |
| Commit | `docs(cellblock): seal the cluster 1 ambient cast` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04s\.claude\reports\cellblock-continuation\CB-N04s-npc-ai-spawn-advisor.md` |

**Why.** The [CB-N04 contract](../four-zone-planning/cellblock-work-packets.md#cb-n04--purposeful-ambient-cast-one-safe-cluster) needs a seal before any coder runs. The seal lists each new tag, template, world-12 position and heading, phase, bark text and suppression rule. [route-baseline.md](route-baseline.md) lists candidate pockets but leaves every one UNSEALED. O14-12 lets the coordinator and the domain reviewers choose a conservative cast and a safe placement. You are the NPC reviewer.

**Read (Grep with context; never read a whole seed).**

1. The CB-N04 contract section linked above, and the `CB-N04` rows of [decision-log.md](decision-log.md) (CB-D03).
2. [route-baseline.md](route-baseline.md): the world-12 spawn census, the candidate pockets (Stasis side and cells, Prisoner 329 cell edge, Preparation side wall, Barracks after clearance, Armory side space), the protected areas (wake-up pod, med station, Mess Hall, Hallway05, ring pads) and row CB-U08.
3. [worknotes/cb-n05-seal.md](worknotes/cb-n05-seal.md): the surfaces CB-N05 uses (pods, Prisoner 329, Marsh). The cast must not sit on them or speak about them.
4. `db/resources/Entities/Seed/entity_templates.sql`: Grep `VALUES \((17[4-9]|180|24),` for templates 174-180 (friendly Op-CORE prisoners, faction 1, no ability set; 175 wanders, 176 is unarmed) and 24 (the NID guard).
5. `db/resources/Worlds/Seed/spawnlist.sql`: Grep `, 12, [0-9]+, '` for the world-12 rows. Spawns 400-490 and the tags `DebugHub_*`, `CraftHub_*` and `BlackMarket_Auctioneer` are the debug hub: no ambience among them.
6. `db/resources/Events/Seed/point_sets.sql` (sets 2032-2048) and the points that bound them (Grep `set_id` = the set, in the points seed in the same folder).
7. `db/resources/Content/Seed/castle_cellblock_chains.sql`: Grep `'enter_region'` with `-B 4 -A 14`. This shows which regions already carry chains in which phase.
8. The hostile scan: Grep `aggro_radius|aggro_range|hostile_scan|is_hostile` under `crates/cell-combat/src/` and `crates/cell/src/cell/`. Find the radius a template-24 guard uses, and whether a hostile NPC targets a friendly-faction NPC.
9. `.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`, and Grep `-i cellblock` in `.claude/agent-memory/npc-ai-spawn-advisor/`.

**Do.**

1. **Pick one cluster** from the candidate pockets. It must be on the route on first approach and outside every protected area. Name its room in one word; that word is `<room>` in CB-N04a's branch and commit (for example `stasis`).
2. **Pick the cast:** 1 to 3 static NPCs from templates 174, 176 or 180. Never use 175, because it wanders. For each template, check `interaction_type` and every `add_dialog_set` aimed at it in the chains seed. A template that a tutorial chain binds to a dialog is excluded, because clicking it would open that dialog out of order. Use `entity_templates.sql` only if no existing template can present the role. If so, say so, and CB-N04a is then `blocked` for an architect pass.
3. **Place each NPC:** exact `x, y, z, heading` (BigWorld, y up). Probe every position with one route anchor in the same room. Write a probes file under the scratch folder and run:
   `Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/castle_cellblock.nav --probes <abs probes file>`
   Exit 0 is required. Exit 2 means a probe has no polygon, so move it. Exit 3 means the probes are split across components, so move it. Paste the output into the seal.
4. **Clearance.** For each cast position, compute the distance to every world-12 template-24 spawn, and cite the guard's aggro radius from step 8 of Read. The minimum must be at least the radius plus 5 m. Do this even if hostiles ignore friendly NPCs, so that a player who stops to look never pulls a guard. Also compute the distance to each protected anchor: the wake-up pod, the med station, each ring pad and the Hallway05 and Mess Hall guard spawns. Give each anchor's coordinates and source row. Then set `GUARD_CLEARANCE_M` and `PROTECTED_CLEARANCE_M`, the values the test will enforce: the measured minimum rounded down to a whole metre.
5. **Choose the trigger:** one existing region the player crosses on first approach to the cluster, and its mission gate (the phase). Use 1 bark, or 2 at most. Check step 7's list. The bark must not share a crossing and phase with a tutorial prompt, a Marsh bark (1176-1178) or a CB-N05 chain (11000-11005). Write each chain's happy-path params and its two adjacent wrong states (the phase not reached and the phase passed), as `mission_<m>_status` or `mission_<m>_step_<s>_status` keys with the values `active`, `completed` or `not_active`.
6. **Write the copy:** one line per bark, at most 200 characters, in the prisoners' voice. It reacts to the escape (the player is up and moving). No instructions, no tutorial hints, no promise of a mission, no pods, no new named characters. The speaker string is `Op-CORE Soldier` (moniker 7552) unless you show evidence for another.
7. **Check the IDs:** use the lowest claimed IDs you need. Re-run the collision check: `Set-Location <wt>; $br=@(git for-each-ref --format='%(refname:short)' refs/heads); for($i=0;$i -lt $br.Count;$i+=80){ git grep -n -E 'VALUES \((52[0-2]|60160|1101[0-2]), |, 20016[0-2], ' @($br[$i..([Math]::Min($i+79,$br.Count-1))]) -- db/resources/Worlds db/resources/Dialogs db/resources/Content }`. Zero hits is required. A hit means `blocked`.
8. **Name the telemetry** for CB-U08: the spawner's per-entity log line (find it in `crates/cell-catalog/src/cell/spawner/`) plus the contract's bark events.
9. **Write the CB-U08 steps** for this cluster: approach path, camera position, what to watch (cast visible, no aggro, no blocked passage, the bark once, nothing on relog), and the SigNoz query.

**Seal layout** (`worknotes/cb-n04-seal.md`, CRLF, under 300 lines):

- `## Cluster`: room, why it, what was excluded and why.
- `## Values`: one table that CB-N04a copies into its test constants. Rows: `<room>`; `SPEAKER`; each cast row (tag `Cellblock_Ambient_<Name>`, spawn id, template id, x, y, z, heading); each bark (chain id, region key, screen id, mission id for `scope_id`, happy params, refused cases); `GUARD_CLEARANCE_M`; `PROTECTED_CLEARANCE_M`; each protected anchor (name, x, y, z).
- `## SQL`: the [CB-N04a templates](#seed-templates) with every `<...>` filled. These are the exact blocks CB-N04a pastes.
- `## Evidence`: nav_inspect output, the distance table, the radius citation, the template checks and the collision check.
- `## CB-U08 steps`.
- `## Verdict`: `sealed`, or `no safe pocket` with the reason.
- `## Measurement`.

**Checks.** `git -C <wt> diff --check`; rule 6 CRLF; `pwsh tools/lint-md.ps1 docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md` (warnings are OK, errors are not).

**Not in scope:** editing seeds or code, lab runs, `crates/cell-chatter/`, a second cluster, CB-N07.

**Repo rules:** 1, 2, 3, 6, 7, 8 and 9.

---

## CB-N04m — Mission sign-off on the seal

| Field | Value |
|---|---|
| Agent | `mission-systems-advisor` |
| Size | About 25k working tokens. Static. |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04s` (after CB-N04s's commit) |
| Writes | `docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md` only (appends a section, and edits `## Values` or `## SQL` only on `change`) |
| Commit | `docs(cellblock): mission sign-off on the cluster 1 cast seal` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04s\.claude\reports\cellblock-continuation\CB-N04m-mission-systems-advisor.md` |

Read the seal, then check:

1. **Gate.** The happy params put the player in the intended phase on first approach. The two refused cases are the adjacent states. Confirm against the mission and step seeds and the chains on that region (Grep the region key in `castle_cellblock_chains.sql`, `-B 4 -A 14`).
2. **No overlap.** No tutorial prompt, Marsh bark (1176-1178) or CB-N05 chain (11000-11005, [cb-n05-seal.md](worknotes/cb-n05-seal.md)) fires on the same crossing in the same phase.
3. **Suppression.** `once` is true. The gate closes after the phase, so a relog or a space change cannot replay the line in a later phase. No bark sits on `player_loaded`.
4. **Cast.** No chain in the seed targets the chosen templates or `Cellblock_Ambient_*` tags with `add_dialog_set`, `set_interaction_type`, `move_entity` or `set_aggression`.
5. **Copy.** The copy is lore-safe for the escape phase and does not pre-empt the CB-N05 story.

Append `## Mission sign-off` with one row per check, then the verdict: `approve`, `change` (edit `## Values` and `## SQL` in place and list each change), or `reject` (with the reason). Checks and rules as CB-N04s. Return the verdict word in the report's first line.

---

## CB-N04a — Build the cluster 1 cast

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 40k working tokens: four seed blocks, one test file, one module line, two docs |
| Branch / worktree | `cellblock/cb-n04-<room>` / `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04a` (based on the CB-N04s tip, so it contains the seal) |
| IDs | From the seal's `## Values` only; all of them are in the contract's claimed list |
| Commit | `feat(cellblock): add <room> escape ambience` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04a\.claude\reports\cellblock-continuation\CB-N04a-coder.md` |
| UAT row | CB-U08 (two-client AoI, camera, no aggro, bark once): UATPending, run by the orchestrator on the integrated branch |

**Files (only these):**

- `db/resources/Worlds/Seed/spawnlist.sql`
- `db/resources/Dialogs/Seed/dialogs.sql`
- `db/resources/Dialogs/Seed/dialog_screens.sql`
- `db/resources/Content/Seed/castle_cellblock_chains.sql`
- `crates/cell-content/src/cell/content/chain_replay_tests/cellblock_ambience.rs` (new, under 260 lines)
- `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` (one line)
- `docs/content/mission-chains.md`
- `docs/analysis/cellblock-continuation/worknotes/cb-n04a.md` (new)

**Step 0.** Read `docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md`, sections `## Values`, `## SQL` and `## Mission sign-off`. If the verdict is not `sealed`, or the sign-off is not `approve` or `change`, stop and report `blocked`. Grep the four seed files for each ID in `## Values`. Any hit means `blocked`.

**Seed changes.** Paste the seal's `## SQL` blocks exactly:

1. `spawnlist.sql`: the cast block goes directly after the last `INSERT INTO spawnlist` row with `world_id` 12 and a spawn id below 400. Leave one blank line before and after it.
2. `dialogs.sql`: the holder row goes directly after the dialog 60110 row and its blank line, before the `--` / `-- TOC entry` block.
3. `dialog_screens.sql`: the screen rows go directly after the screen 200010 row and its blank line. Then change `SELECT pg_catalog.setval('dialog_screens_screen_id_seq', 200010, true);` so it names the highest screen id you added. This follows the bank precedent of setting the sequence to the highest authored screen.
4. `castle_cellblock_chains.sql`: append the chain block at the end of the file, after chain 5030's `content_actions` statement, with one blank line between. Do not touch the header (rule 7).

### Seed templates

The seal fills these in. They are shown here so that the reviewer can diff the result against them.

```sql
-- CB-N04a (2026-10-17): Cellblock ambient cast, <room>. Friendly Op-CORE
-- prisoners (faction 1, no ability set), static. Placement, nav probe and
-- guard clearance: docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md.
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name) VALUES (520, <x>, <y>, <z>, <heading>, 12, <template>, 'Cellblock_Ambient_<Name>', NULL);
```

```sql
-- Cellblock continuation CB-N04a: holder for the <room> ambient barks. Never
-- displayed; npc_bark reads its screens' text. Seed-only, no override.
INSERT INTO dialogs (dialog_id, dialog_flags, event_set_id, ui_screen_type, tags, accepts_mission_id, name) VALUES (60160, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Cellblock - ambient cast bark holder (never displayed)');
```

```sql
-- Cellblock continuation CB-N04a: <room> ambient bark lines (CIMMERIA_AUTHORING).
-- Speaker 0: npc_bark supplies the chat prefix from its own params.
INSERT INTO dialog_screens (dialog_id, screen_id, text, speaker_id, index) VALUES (60160, 200160, '<line>', 0, 0);
```

```sql
-- =====================================================================
-- CB-N04a (2026-10-17): Cellblock ambient cast, <room>
-- =====================================================================
-- <N> friendly Op-CORE prisoners (spawns 520..) show the escape is under
-- way. Each chain speaks one line into the crossing player's chat on the
-- first entry to <region> during <phase>. once = true stops the H52
-- step-activation replay speaking twice in a visit; the mission gate stops
-- it across visits and relogs. Seal: docs/analysis/cellblock-continuation/
-- worknotes/cb-n04-seal.md. Holder dialog 60160 is never displayed.

-- Chain 11010: <purpose>
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (11010, 'CB-N04a - <room> ambience: enter <region> during <phase> -> Op-CORE prisoner bark', 'mission', <m>, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (11010, 'enter_region', '<region key>', 'player', true, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (11010, 'mission_status', <m>, NULL, 'eq', 'active', 0),
  (11010, 'mission_status', <m2>, NULL, 'eq', 'not_active', 1);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (11010, 'npc_bark', NULL, NULL,
 '{"screen_id": 200160, "speaker": "Op-CORE Soldier", "channel": "say"}', 0, 0);
```

A step gate uses `('step_status', <m>, '<step>', 'eq', 'active', n)` instead of a `mission_status` row.

**Test.** New file `chain_replay_tests/cellblock_ambience.rs`. Register it in `chain_replay_tests/mod.rs` by inserting `mod cellblock_ambience;` between `mod castle_loot_containers;` and `mod cellblock_dialog_closes;`. Test type: live-DB chain replay at resolve level, plus a live-DB seed guard (D-CC4). This packet changes no executor code; say so in the report. Fill every `<...>` from the seal's `## Values`, and give `BARKS` and `CAST` one entry per seal row.

```rust
//! CB-N04a — the Cellblock ambient cast (<room>): spawn rows tagged
//! `Cellblock_Ambient_*` and bark chains 11010.. in
//! `castle_cellblock_chains.sql`. Placement, gates and copy are sealed in
//! `docs/analysis/cellblock-continuation/worknotes/cb-n04-seal.md`.
//!
//! - The bark tests pin each chain's phase, the two adjacent wrong states
//!   its gate must refuse, and `once` (the H52 step-activation replay
//!   re-fires mission-gated `enter_region` chains; `once` stops a second
//!   line in one visit).
//! - The placement test pins world, template and tag for every cast row,
//!   and the static clearances the seal measured from every world-12 NID
//!   guard (template 24) and every protected anchor.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;
use crate::test_support::require_db_or_skip;

type Params = &'static [(&'static str, &'static str)];

struct Bark {
    chain_id: i32,
    region_key: &'static str,
    screen_id: i32,
    fires: Params,
    refuses: &'static [(&'static str, Params)],
}

/// Seal § Values. Region keys are byte-identical to `point_sets.sql`.
const BARKS: &[Bark] = &[Bark {
    chain_id: 11010,
    region_key: "<region key>",
    screen_id: 200160,
    fires: &[<happy params>],
    refuses: &[
        ("the phase is not reached", &[<params>]),
        ("the phase has passed", &[<params>]),
    ],
}];

const SPEAKER: &str = "Op-CORE Soldier";
/// `CHAN_say`; the loader accepts no other channel.
const CHAN_SAY: u8 = 0;

/// Seal § Values: (tag, spawn id, template id).
const CAST: &[(&str, i32, i32)] = &[("Cellblock_Ambient_<Name>", 520, <template>)];
const GUARD_CLEARANCE_M: f64 = <g>;
const PROTECTED_CLEARANCE_M: f64 = <p>;
const PROTECTED: &[(&str, [f64; 3])] = &[("<anchor>", [<x>, <y>, <z>])];

async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist and assemble"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn resolve_region_enter(engine: &ChainEngine, region_key: &str, params: Params) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

#[tokio::test]
async fn live_db_cellblock_ambience_barks_fire_once_in_their_phase() {
    let pool = require_db_or_skip!();
    for bark in BARKS {
        let id = i64::from(bark.chain_id);
        let engine = engine_for(&pool, bark.chain_id).await;
        assert!(engine.is_once(id), "chain {id} must be once = true (H52 replay)");
        let resolved = resolve_region_enter(&engine, bark.region_key, bark.fires);
        let barks: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(c, a)| *c == id && matches!(a, Action::NpcBark { .. }))
            .map(|(_, a)| a)
            .collect();
        assert_eq!(barks.len(), 1, "chain {id} must resolve one NpcBark: {:?}", resolved.actions);
        match barks[0] {
            Action::NpcBark { screen_id, speaker, channel } => {
                assert_eq!(*screen_id, bark.screen_id, "chain {id} speaks the wrong screen");
                assert_eq!(speaker, SPEAKER, "chain {id} has the wrong chat prefix");
                assert_eq!(*channel, CHAN_SAY, "chain {id} must ride CHAN_say");
            }
            other => panic!("filtered for NpcBark but got {other:?}"),
        }
        assert_no_deferred_actions(&resolved, id);
    }
}

#[tokio::test]
async fn live_db_cellblock_ambience_barks_refuse_adjacent_states() {
    let pool = require_db_or_skip!();
    for bark in BARKS {
        let id = i64::from(bark.chain_id);
        let engine = engine_for(&pool, bark.chain_id).await;
        for (why, params) in bark.refuses {
            let resolved = resolve_region_enter(&engine, bark.region_key, params);
            assert!(
                !resolved.actions.iter().any(|(c, _)| *c == id),
                "chain {id} must not resolve when {why}: {:?}",
                resolved.actions
            );
        }
    }
}

#[tokio::test]
async fn live_db_cellblock_ambience_cast_rows_are_placed_safely() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String, i32, i32, f64, f64, f64)> = sqlx::query_as(
        "SELECT spawn_id::int4, tag, template_id::int4, world_id::int4, \
         x::float8, y::float8, z::float8 FROM resources.spawnlist \
         WHERE tag LIKE 'Cellblock\\_Ambient\\_%' ORDER BY spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist scan must succeed");
    assert_eq!(rows.len(), CAST.len(), "exactly the sealed cast rows: {rows:?}");

    let guards: Vec<(f64, f64, f64)> = sqlx::query_as(
        "SELECT x::float8, y::float8, z::float8 FROM resources.spawnlist \
         WHERE world_id = 12 AND template_id = 24",
    )
    .fetch_all(&pool)
    .await
    .expect("guard scan must succeed");
    assert!(!guards.is_empty(), "no world-12 NID guards: the seed did not load");

    let dist = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    for ((spawn_id, tag, template, world, x, y, z), (want_tag, want_id, want_tpl)) in
        rows.iter().zip(CAST)
    {
        assert_eq!((spawn_id, tag.as_str(), template, world), (want_id, *want_tag, want_tpl, &12));
        let at = [*x, *y, *z];
        for g in &guards {
            let d = dist(at, [g.0, g.1, g.2]);
            assert!(d >= GUARD_CLEARANCE_M, "{tag} is {d:.1} m from an NID guard");
        }
        for (name, anchor) in PROTECTED {
            let d = dist(at, *anchor);
            assert!(d >= PROTECTED_CLEARANCE_M, "{tag} is {d:.1} m from {name}");
        }
    }
}
```

`spawnlist.tag` may be `NULL` on other rows. The `LIKE` filter returns only cast rows, so `String` decodes. If `cargo clippy` objects to the tuple comparison's mixed references, compare the fields one by one. Keep each assertion message.

**Why the tests fail on revert.** Without the chain rows, `engine_for` panics. Without the spawn rows, the count is 0 and `assert_eq!` fails. A wrong gate either fails the happy path or resolves on a refused case. The existing guard `marsh_barks::seed_shape::live_db_every_seeded_bark_names_a_real_screen_and_a_named_speaker` now also covers the new barks: a missing or blank screen fails it. `live_db_player_loaded_into_castle_cellblock_resolves_no_bark` fails if a bark lands on `player_loaded`.

**Mutation proof (required).** In one edit, change chain 11010's first condition value `'active'` to `'completed'` and spawn 520's `world_id` from `12` to `13`. Run `live-db-test.ps1 cellblock_ambience`. The fire test and the placement test must fail. Undo both edits with the Edit tool (no git restore), rerun, and confirm green. Record both runs in the report.

**Docs.**

- `docs/content/mission-chains.md`: add `#### Cellblock ambient cast (CB-N04a)` directly before `### Castle (Persistent Zone)`. Add one table, `| Chain | Trigger | Conditions | Actions |`, in the shape of the 1006-1009 rows, with one row per chain. Add one sentence naming the spawn ids, tags and templates, and one linking the seal.
- `worknotes/cb-n04a.md` (CRLF, under 60 lines): what was added, which tests and why they fail on revert, the mutation proof, the telemetry events from the contract, and the status (CB-U08 UATPending).

**Lane checks, in order** (each prefixed `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\cellblock-cb-n04a;`):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`
3. `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-content --filter-expr 'test(cellblock_ambience) | test(marsh_barks)'`
4. `pwsh tools/build-lane/live-db-test.ps1 cellblock_ambience`. All three tests must pass, not skip.
5. `pwsh tools/build-lane/live-db-test.ps1 marsh_barks`. The all-bark guards must stay green.
6. `git -C <wt> diff --check`, plus rule 6 for every seed and doc.

**Not in scope:** the seed header, `id-allocation.md`, `crates/cell-chatter/`, `entity_templates.sql`, executor code, a lab run, and any ID not in the seal.

**Repo rules:** 1 to 10.

---

## W2-INT — Integrate wave 2

| Field | Value |
|---|---|
| Agent | `packet-coder` |
| Size | About 25k working tokens |
| Worktree | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock` (branch `content/cellblock-campaign`) |
| Depends on | CB-N04s, CB-N04m and CB-N04a each have reviewer verdict `ship` |
| Report | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-cellblock\.claude\reports\cellblock-continuation\W2-INT-coder.md` |

1. `git -C <wt> status --porcelain` must be empty, and HEAD must be the commit the brief names. Otherwise `blocked`.
2. Cherry-pick, in this order, every commit the brief lists: CB-N04s (seal), CB-N04m (sign-off), CB-N04a (packet commit, then its fix commits). Use `git -C <wt> cherry-pick <sha>`. On a conflict, run `git -C <wt> cherry-pick --abort` and report `blocked` with the paths. Never resolve by hand.
3. Bookkeeping commit `docs(cellblock): record W2 ambient cast chains and IDs`. It touches only these files:
   - `castle_cellblock_chains.sql` header: directly before the line starting `--   (next free inside 1001-1111:`, add `--   Cellblock ambient cast (CB-N04a, 2026-10-17): chain(s) <used ids>, individual claims in the 11000-11499 Cellblock authored block (id-allocation.md, CB-N04 claims).`
   - `docs/analysis/four-zone-planning/id-allocation.md`, "Chain IDs in use today" Cellblock row: add the number of chains used to the count `125`, and append `, and 11010–<last>` after `the single outlier 5030`. In the "CB-N04 claims — 2026-10-17" table, mark each used ID `In use (W2-INT)` and each unused one `Released (W2-INT)`.
   - `docs/analysis/cellblock-continuation/README.md`: the CB-N04s, CB-N04m and CB-N04a rows become `Integrated` with their SHAs (CB-N04a also `UATPending` on CB-U08).
4. Lane checks (prefix `Set-Location <wt>;`): `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`, then `pwsh tools/build-lane/live-db-test.ps1 cellblock_ambience`, then `pwsh tools/build-lane/live-db-test.ps1 marsh_barks`. All must pass, not skip.
5. Report the integrated SHAs. No push.

Repo rules 1, 2, 3, 6, 9 and 10.

---

## CB-U08 — Lab row for the cast (orchestrator)

This row is not a coder packet. Under D-CC3 and O14-08 the orchestrator **asks the user first**, waits until the lab is free, and then dispatches `lab-driver` with the steps from the seal's `## CB-U08 steps`, on the integrated branch. If it fails, a fix child moves the spawn or changes the gate; that child needs a new architect pass. If the lab is unavailable, the row stays UATPending, and CB-N08 records it (O14-09). The contract assigns two-client AoI and the visual check to CB-N08, so CB-U08 can also run there.

---

## CB-N07 — Held

There is no packet in W2. The [contract](../four-zone-planning/cellblock-work-packets.md#cb-n07--one-rooms-authored-encounter-response-conditional) needs three things first:

- **CB-U05:** live flank and cover evidence. CB-N03s found no static fault, so CB-N03 cannot resolve without it.
- **CB-U09:** a valid 30-45 minute core-run measurement that shows room inside the 10-minute optional budget (O14-14, O14-15).
- **D-CC11 (BlockedDecision):** the owner sets the death, difficulty and time impact acceptable for a one-room encounter.

When all three land, an architect pass writes CB-N07s, an advisor seal by NPC, combat and mission, with one room (med bay, mess hall, or barracks and armory). It uses existing verbs only and has the 40k budget. The coder packet follows from that seal.
