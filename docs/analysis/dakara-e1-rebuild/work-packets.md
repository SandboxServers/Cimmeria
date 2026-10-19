# Dakara_E1 Rebuild Work Packets

> Type: how-to. Audience: the orchestrator, ops, packet-coder and domain agents, the packet-reviewer.
> Re-sliced 2026-10-18 to the [campaign-packet skill](../../../.claude/skills/campaign-packet/SKILL.md) against `main` @ `d1bba0400`. First written 2026-10-06 (DK-00, #1282). Companions: [ledger, findings and decisions](README.md), [resume card](RESUME.md), [audit](audit.md), [placements](placements/README.md), [testing playbook](../../../TESTING.md).

The packet status lives in **one place**: the [README packet table](README.md#packets). This file holds the rules, the contract and the packet text. Packet text for waves 1 to 3 is in [packets/](packets/); waves 4 to 6 are stubs below, expanded by the architect when their blockers lift.

## Dispatch rules

1. **Peer check first.** Before any dispatch, ops reports `git log --since="1 hour ago"` on `dakara/*` branches, the board's `dakara-e1-rebuild` subcategory and any `.claude/worktrees/dk-*` folder. A live peer means stand down.
2. **One worktree per packet.** Ops runs `pwsh tools/build-lane/mk-worktree.ps1 dakara/<packet-lower>-<slug> <worktree>`; the worktree name and branch are in each packet's header. Test database: `sgw_<worktree>`. Never two workers in one checkout.
3. **Roles.** `packet-coder` (Haiku) takes seed, test and docs packets that are fully specified here. Evidence, RE and ruling packets go to the named domain agent with the same brief shape. **No `packet-review-fixer` (owner, 2026-10-18).** After all of a wave's packets return, one read-only `packet-reviewer` reviews the wave in one context and writes a finding list per packet. Each packet with verified findings goes back to a fresh `packet-coder` (Haiku) in the same worktree, briefed with the packet section and the finding list; the reviewer then re-checks only those findings. At most 3 review passes per packet.
4. **Brief** (under 25 lines): worktree path, the packet section path, the report path, the commit attribution lines, the return line. The agent reads the packet; the brief never pastes it.
5. **One PR per packet**, opened and merged by ops (`ship.ps1 pr -C <worktree> -m <msg>`, then `ship.ps1 merge <PR> --retire <worktree>`). Packets inside a wave are independent unless the wave table says otherwise. Merge queue: green CI required.
6. **Escalation** per the skill: a context stop leaves `worknotes/<packet>.md` and a fresh coder continues (at most 3 sessions); then one Sonnet implementer (`rust-gameserver-dev` for code and seed, `documentation-writer` for docs); then the architect re-slices. Review: at most 3 passes; a `decision` verdict goes to the architect for a ruling note.
7. **Owner decisions** are `BlockedDecision` in the README. The orchestrator asks them; no packet guesses an answer.
8. **No lab use until the campaign ends (owner, 2026-10-18).** No agent and no packet starts a lab session, the client or the launcher. The owner runs M1, M2 and every later UAT himself; the campaign lists the steps owed and waits for his result.

## Contract

Every packet builds against this section. A packet that cannot honour it stops and returns `blocked`.

### C1. Report and return

- Report file: `<worktree>\.claude\reports\dakara\<packet>-<role>.md`, for example `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dk-07\.claude\reports\dakara\DK-07-packet-coder.md`. Never the root checkout.
- Report body: files changed, each check's final `status=` line (or the command and its exit code for docs packets), the commit hash, every deviation from the packet with its reason.
- Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit once, locally, on the worktree branch, with the subject in the packet header and the attribution lines from the brief. No push, no PR.

### C2. Id blocks (D-DK8, D-DK9; registered by DK-07)

| Kind | Block | In use on `main` | Next free | Notes |
|---|---|---|---|---|
| Content chains | 8001-8499 | 8001-8006 (`dakara_e1_space_chains.sql`) | 8007 | 8007-8039 space and travel; per-mission files in C3; later missions 8180-8499 |
| Entity templates | 440-479 | 440-457 | 458 | 458-461 are the four extra Drop Location props (D-DK16, DK-09); hostiles from 462 |
| Spawnlist | 8000-8099 | 8000-8007 | 8008 | 8008-8015 are DK-09's |
| Point sets | 2123-2199 | 2123-2127 | 2128 | 2128-2130 are the three SG-18 search regions (D-DK18, DK-09), with points 2557-2559; next free after DK-09: 2131 |
| Respawners | 611-619 | 611 | 612 | |
| Authored mission (D-DK1 F) | 63100-63103 | none | 63100 | Mission 63100, step 63101, objective 63102, task 63103 (DK-10fs). Absent from the stock catalog and from `db/resources`; mission ids stay at or below 65535 |

The 2026-10-06 plan's spawn block 600-699 and point-set block 2200-2279 are void: the merged rows use the blocks above.

### C3. Seed files and chain ranges (D-DK14)

One packet owns one seed file. DK-10a creates every file below with only its header and `SET search_path = resources, pg_catalog;`, and adds the eight `\ir resources/Content/Seed/<file>` lines to `db/database.sql` directly after line 407 (`dakara_e1_space_chains.sql`), in the order of this table.

| File (`db/resources/Content/Seed/`) | Owner | Chains |
|---|---|---|
| `dakara_e1_bridge_chains.sql` | DK-10fc (D-DK1 F) | 8075-8079 (8075-8077 used) |
| `dakara_e1_m1570_entry_chains.sql` | DK-10a | 8040-8044 |
| `dakara_e1_m1570_tent_chains.sql` | DK-10b | 8045-8059 |
| `dakara_e1_m1570_search_chains.sql` | DK-10c | 8060-8074 (8060-8071 used, D-DK19) |
| `dakara_e1_m1570_report_chains.sql` | DK-10d | 8160-8179 (moved, D-DK19) |
| `dakara_e1_m1645_chains.sql` | DK-11 | 8080-8119 |
| `dakara_e1_m1646_offer_chains.sql` | DK-12b | 8120-8139 |
| `dakara_e1_m1646_drop_chains.sql` | DK-12c | 8140-8159 |

Later missions use 8180-8499. DK-10d's range sits after DK-12c's because 4902 and 4901 need 12 ids (D-DK19); the `\ir` order stays the order of this table.

Shared seed files and their single owner per wave: `entity_templates_dakara_e1.sql` and `spawnlist_dakara_e1.sql` (DK-09 in wave 2), `point_sets_dakara_e1.sql` and `point_set_points_dakara_e1.sql` (DK-09: the three SG-18 search regions, D-DK18), `crates/content-engine/tests/it/interact_tag_linter.rs` (DK-10a: `fn allowlist` only; DK-09b: moves the region check to `interact_tag_linter/regions.rs`; the two hunks do not overlap), `dakara_e1_space_chains.sql` (DK-10a edits only its header comment; DK-10e edits only chain 8002's mail text and adds chain 8007 after it). The mission seed `db/resources/Missions/Seed/missions_dakara_e1.sql` and its `\ir` line after `missions.sql` are DK-10fs's alone.

### C4. Chain row shape

Copy this shape exactly (single-row `content_chains` and `content_triggers` inserts, multi-row conditions and actions). Every chain has a comment above it naming its evidence label and the objective or bind it serves.

```sql
-- Chain 8051 PROJECT_FINAL_RECONSTRUCTION: 1570 step 4906, Jaffa talks to Bra'tac (dialog 5811), advance to 4904.
INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
VALUES (8051, '1570 - Step 4906: Bra''tac 5811 (Jaffa), advance to 4904', 'mission', 1570, true, 0);

INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
VALUES (8051, 'interact_tag', 'Dakara_E1_StoryRm_Bratac', 'player', false, 0);

INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
VALUES
  (8051, 'world', 62, NULL, 'eq', NULL, 0),
  (8051, 'archetype', NULL, NULL, 'gte', '7', 1),
  (8051, 'step_status', 1570, '4906', 'eq', 'active', 2);

INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES
  (8051, 'display_dialog', 5811, NULL, '{}', 0, 0),
  (8051, 'advance_step', 1570, '4904', '{}', 0, 1),
  (8051, 'remove_dialog_set', 6727, NULL, '{"slot": 59}', 0, 2);
```

Vocabulary used by this campaign (all exist today):

| Kind | Form |
|---|---|
| Triggers | `player_loaded` key = world name (`Dakara_E1`, `Dakara_E1_StoryRm`); `interact_tag` key = spawn tag; `dialog_choice` key = dialog id as text; `mission_completed` key = mission id as text; `enter_region` key = `point_sets.name`, byte-exact (a client-hinted region, `flags` 1; the server fills world, archetype and mission context, `event_dispatch/region.rs`) |
| `world` | `target_id` = world id (61 or 62), operator `eq` |
| `archetype` | `value` text; operators `eq`, `gte`, `lte` |
| `mission_status` | `target_id` = mission; values `not_active`, `active`, `completed` |
| `step_status` | `target_id` = mission, `target_key` = step id text; value `active` |
| `objective_status` | `target_id` = mission, `target_key` = objective id text; value `completed`, use `neq` for "not yet" |
| `counter` | `target_key` = counter name, `value` = integer text, operators `eq`, `lt`, `gte`; reads param `counter_<name>` (`conditions/mod.rs`), a missing counter reads 0. Counters live in memory only: a relog or any world change resets them |
| Actions | `accept_mission` (target = mission), `advance_step` (target = mission, key = step text), `complete_objective` (target = mission, key = objective text), `complete_mission`, `display_dialog` (target = dialog), `add_dialog_set` / `remove_dialog_set` (target = dialog_set_map id, params `{"slot": <template_id>}`), `add_item` (target = item, params `{"container": 0, "qty": 1}`; container 0 means the item's own container set), `remove_item` (target = item, params `{"qty": 1}`), `increment_counter` (target NULL, key = name, params `{"amount": 1}`), `reset_counter` (target NULL, key = name, params `{}`) |

### C5. Authoring rules every chain packet follows

1. **Archetype split (D-DK15).** Jaffa dialogs: `archetype gte 7` (Free Jaffa 7, Praxis Jaffa 8). Tau'ri dialogs: `archetype lte 6`. The Free Jaffa entry: `archetype eq 7`.
2. **Every matching chain fires**, with conditions read before any action runs. Two chains on one trigger either have disjoint conditions or are a declared co-firing pair (rule 6). Priority only orders; it never excludes.
3. **Never let `complete_objective` close a step's last required objective.** It completes the whole mission (`crates/cell-content/src/cell/missions/progression.rs`, the `all_required_complete` block). Leave a step with `advance_step`; finish a mission with `complete_mission`. A `complete_objective` on an objective that is not on the current step is a no-op with a WARN.
4. **Binds are per player and in memory.** Every `add_dialog_set` has a `player_loaded` restore chain for the world the NPC stands in, gated on the same state. A slot keeps every bind added to it (binds are a list per template), so remove the old dsm before adding the next on the same slot, and never let a restore chain and the chain that first adds a bind both fire. A relog or world change drops every bind.
5. **`remove_dialog_set` runs before `complete_mission`** in the same chain: `complete_mission` fires `mission_completed` chains inline.
6. **Step entry binds.** A chain that advances into a step owned by another packet does not add that step's binds. The owning packet adds a co-firing chain on the same trigger with the same conditions, no `display_dialog`, and the next step's `add_dialog_set` actions. `display_dialog_count` on that trigger stays 1. This applies only when the next step's NPC or prop stands in the same world as the trigger. Under D-DK1 F no such pair is left: chain 8040 completes Rak'nor's optional 5936 itself (Rak'nor's 6110 has just played in the bridge mission), so DK-10b has no 8057; under the fallback B, 8057 binds Rak'nor beside 8040. Step 4902 has no binds: its objectives are region enters (D-DK18). The 4902 chains that advance to 4901 add the remains binds themselves, since both steps are DK-10c's. An NPC in the other world gets its bind from its own `player_loaded` restore when the player enters that world, so no co-firing chain is needed.
7. **Linter.** DK-10a adds one range entry per mission seed file to `fn allowlist` in `crates/content-engine/tests/it/interact_tag_linter.rs`. DK-09b moves the region-key check into `interact_tag_linter/regions.rs` and makes it read every `point_sets*.sql` (today it reads only `point_sets.sql`, so a Dakara `enter_region` key would fail it). Wave-3 packets edit neither file.
8. **Regions are trusted.** The server does not re-check that a player who reports a client-hinted region stands inside it. A forged report can only advance the sender's own 4902 step; accepted, noted in F21.

### C6. Ids every mission packet uses

| Thing | Id | Notes |
|---|---|---|
| Worlds | 61 `Dakara_E1`, 62 `Dakara_E1_StoryRm` | 62 is instanced |
| Archetypes | 3 Scientist (Tau'ri test value), 7 Free Jaffa, 8 Praxis Jaffa | `crates/names/src/fixed_tables.rs` |
| Bra'tac | template 59, tag `Dakara_E1_StoryRm_Bratac`, spawn 8004, world 62 | speaker 2323; shared with Harset: never edit the template |
| Moh'katan | template 54, tag `Dakara_E1_StoryRm_Mohkatan`, spawn 8005, world 62 | speaker 945; shared with Harset |
| Rak'nor | template 441, tag `Dakara_E1_Raknor`, spawn 8006, world 61 | speaker 2956 |
| Loth'ta | template 440, tag `Dakara_E1_Lothta`, spawn 8007, world 61 | speaker 781 |
| SG-18 remains | templates 450, 451, 452; spawns 8008-8010 (DK-09) | tags `Dakara_E1_SG18_Louis`, `_Nguyn`, `_Waters` |
| SG-18 search regions | point sets 2128 `Dakara_E1.SG18Louis` (objective 5632), 2129 `Dakara_E1.SG18Nguyn` (5630), 2130 `Dakara_E1.SG18Waters` (5631, optional); DK-09 | `enter_region` keys of step 4902 (D-DK18); one around each remains |
| Drop locations | templates 449, 458, 459, 460, 461; spawns 8011-8015 (DK-09) | tags `Dakara_E1_Drop1` to `Dakara_E1_Drop5` |
| Mission 63100 "Home to Dakara" (D-DK1 F) | step 63101, objective 63102 (required), task 63103 | `NEW CONTENT`, level 1; catalog entry and seed by DK-10fs, chains 8075-8077 by DK-10fc |
| Mission 1570 (Dakara half) | steps 4906, 4904, 4902, 4901, 4905, 4903 | SGC steps 4641, 4642 (objectives 5382, 5383); 4906's optional Rak'nor objective 5936 |
| Mission 1645 | steps 5168 (obj 5938), 4907 (5634 optional, 5633) | item 6779 |
| Mission 1646 | steps 4908 (5635, 5939, 5940, 5941, 5942 all required), 4909 (5943) | item 6780 |

Dialog set map rows (the `add_dialog_set` target):

| dsm | Dialog | Flags | Bound to | Use |
|---|---|---|---|---|
| 6890 / 6889 | 6110 Jaffa / 6111 Tau'ri | active | 441 Rak'nor | 1570 optional 5936 |
| 6727 / 6726 | 5811 Jaffa / 5810 Tau'ri | active | 59 Bra'tac | 1570 step 4906 |
| 6729 / 6728 | 5813 Jaffa / 5812 Tau'ri | active | 440 Loth'ta | 1570 step 4904 |
| 6730 | 5814 "weak radio signal" (speaker 0 only) | 0 | none: unused (D-DK18) | 5814 is shown by `display 5814` on the 4902 chains that advance to 4901 |
| 6731 / 6732 | 5815 Tau'ri / 5816 Jaffa radio (speaker 2609 Hammond and 0) | 0 | none (no NPC) | 1570 step 4905, needs DK-06b |
| 6733 | 5817 Bra'tac turn-in | turn-in | 59 | 1570 step 4903 |
| 7153 / 7154 / 7155 | 6224 / 6225 / 6226 dog tags (speaker 0) | world object | 450 / 451 / 452 | 1570 step 4901 |
| 6736 | 5980 offer blurb (speaker 0) | available | 54 Moh'katan | 1645 offer |
| 6734 | 5807 Moh'katan and Bra'tac | 0 | none | 1645 accept scene |
| 6735 | 5808 Loth'ta turn-in | active | 440 | 1645 step 4907 |
| 6737 | 5981 offer blurb (speaker 0) | available | 54 | 1646 offer |
| 6738 | 5819 Moh'katan briefing | 0 | none | 1646 accept scene |
| 6739-6743 | 5821 "You hide the coded orders" (speaker 0) | world object | 449, 458, 459, 460, 461 | 1646 step 4908 |
| 6744 | 5820 Moh'katan turn-in | turn-in | 54 | 1646 step 4909 |

### C7. Tests every chain packet ships

- **Type:** chain replay (TESTING.md type 6), which is a live-DB test here: it loads the seed through `build_engine(Some(&pool))`.
- **Place:** `crates/cell-content/src/cell/content/chain_replay_tests/dakara_e1_missions/<submodule>.rs`; DK-10a creates the folder, `mod.rs` and every submodule file below as an empty module with a `//!` line, so later packets edit only their own file.

| Submodule | Owner |
|---|---|
| `bridge.rs` | DK-10fc (D-DK1 F) |
| `entry.rs` | DK-10a |
| `tent.rs` | DK-10b |
| `search.rs` | DK-10c |
| `report.rs` | DK-10d |
| `m1645.rs` | DK-11 |
| `m1646_offer.rs` | DK-12b |
| `m1646_drops.rs` | DK-12c |

- **Shape:** `#[tokio::test] async fn live_db_<what>() { let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await; ... }`. Helpers from `super::super::mission_1324` (`ctx_for`, `with_mission`, `with_step`, `fire_player_loaded`, `fire_interact_tag`, `fire_mission_completed`, `fire_dialog_choice`, `display_dialog_count`) and from `super` (`labels_in`, `with_objective`, `with_counter`, `fire_enter_region`, the constants).
- **Each chain gets:** the exact `labels_in(&resolved, <own file range constant>)` list for the matching state (each packet filters to its own file's range from `mod.rs`, so a sibling packet's co-firing chain never breaks it); the empty list for the nearest wrong state (other archetype branch, wrong step, wrong world, mission already completed); `display_dialog_count == 1` on every `interact_tag` that displays.
- **Revert proof:** each test asserts a non-empty label list, so it fails when the packet's seed rows are removed.
- **Run:** `pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_missions` from the worktree root.

### C8. Repo rules every packet carries

1. PowerShell only. No bash, WSL or Git Bash.
2. Every compiling `cargo` call goes through `pwsh tools/build-lane/lane.ps1 cargo ...` run from the worktree root; never call `cargo` directly. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads `sgw_<worktree>`.
3. Never `git stash`, `git worktree prune`, `git push` or `gh pr`. One local commit.
4. Edit only the packet's named files. Files stay under 700 lines (soft cap 500).
5. Markdown under `docs/` is CRLF: keep it CRLF.
6. Seeds change in `db/resources/` only; never add a `db/scripts/` migration.
7. Every seed row carries an evidence-label comment (`ORIGINAL_DATA`, `PROJECT_FINAL_RECONSTRUCTION`, `RECONSTRUCTION`, `NEW CONTENT`); every spawn sets `respawn_secs`; loot stays NULL.
8. No new dialog rows, no client patch, no cooked-data override. The one exception is DK-10fs's single `MISSION_ADDITIONS` entry for mission 63100 (D-DK1 F); no other packet adds or edits one.
9. No IPs, credentials, tokens or account names in any file.
10. Never launch the game client, the launcher or a lab tool.

## Waves

| Wave | Packets | Gate to start | Reviewer |
|---|---|---|---|
| 1 (current) | DK-06a, DK-07, DK-08, DK-12a | none: all Ready | one `packet-reviewer`, fixes by `packet-coder`; DK-07 also takes the D-DK19 id-allocation text fix before its PR |
| 2 | DK-06b, DK-09, DK-09b, DK-10a, DK-10e, DK-10fs | DK-06a, DK-08, DK-12a merged (DK-09b: none); D-DK1 answered (DK-10a, DK-10e; D-DK17 is decided); D-DK1 answered F and D-DK20 answered (DK-10fs); D-DK10 ruled (DK-06b) | one `packet-reviewer` plus `mission-systems-advisor` read-only; fixes by `packet-coder` |
| 3 | DK-10b, DK-10c, DK-10d, DK-10fc, DK-11, DK-12b, DK-12c | DK-10a, DK-09 and DK-09b merged; DK-06b merged (DK-10d only); DK-10fs merged (DK-10fc only) | one `packet-reviewer` plus `mission-systems-advisor` read-only, in two halves if one context cannot hold seven; fixes by `packet-coder` |
| M2 | owner UAT | wave 3 merged | the owner, himself (no agent lab use) |
| 4 | GDK1, DK-20, DK-21, DK-13, DK-14a, DK-14b | D-DK5 answered after the M2 playtest | architect expands first |
| 5 | DK-15a, DK-15b, DK-30, DK-31a, DK-31b, DK-32 | wave 4; B-DK4 for DK-30 | architect expands first |
| 6 | DK-33, DK-99 | wave 5 | `packet-reviewer`, fixes by `packet-coder` |

Packet text: [packets/wave-1.md](packets/wave-1.md), [packets/wave-2.md](packets/wave-2.md) with [packets/wave-2-dk-10fs.md](packets/wave-2-dk-10fs.md), [packets/wave-3.md](packets/wave-3.md).

## Historical packets (shipped; do not re-dispatch)

| Packet | What shipped | PR | Status |
|---|---|---|---|
| DK-00 | Audit, ledger, packets | #1282 | Done |
| DK-01a | Chain 8001 `grant_stargate_address 5` on `player_loaded Dakara_E1` for archetype 7; gate 25 arrival pin on the plaza | #1284 | UATPending (M1) |
| DK-01b | `send_system_mail` quiet-refusal param (Rust, executor and base mail path) with its unit and live-DB tests | #1284 | UATPending (M1) |
| DK-01c | Chain 8002, the one-time arrival notice (`NEW CONTENT`), its replay and executed tests and the live-DB relationship guard | #1284 | UATPending (M1) |
| DK-02 | Landmark census, floor and navmesh checks, placement ledger | #1283 | Done |
| DK-03 | Templates 440-457: five cast, thirteen props, six live-DB guards | #1285 | UATPending (M2) |
| DK-04 | Named areas, tent flaps both ways (chains 8003-8006, spawns 8000-8003), Med Tent respawner 611 | #1287 | UATPending (M2) |
| DK-05 | Static cast spawns 8004-8007, speculative Repository point set 2127 | #1288 | UATPending (M2) |

Worknotes: [DK-01](worknotes/DK-01.md) to [DK-05](worknotes/DK-05.md).

### DK-01

DK-01 shipped as one change in #1284. The a/b/c split above records its three parts so the M1 checklist (DK-U1 to DK-U10 in the [README](README.md#m1-checklist-dead-end-removed)) can name what each step proves: DK-U3 to DK-U6 test DK-01a, DK-U7 and DK-U8 test DK-01b, DK-U1, DK-U2 and DK-U10 test DK-01c. Under today's sizing each part would be its own packet-coder packet; it is not re-dispatched. The `send_system_mail` quiet-refusal param is documented in [content-engine-vocabulary.md](../../content/content-engine-vocabulary.md).

## Stubs: waves 4 to 6

Each stub is expanded by the architect into a full packet (contract excerpt, rules, checks, docs rows, commit) when its gate lifts. Sizes are estimates; anything over about 40k working tokens is split again.

| Packet | Mission or scope | Entries | Depends | Agent |
|---|---|---|---|---|
| GDK1 | Encounter design gate for 1647, 1652, 1653: templates, counts, wave triggers, level band, win and fail. A decision record, nothing seeded. | template 35 and its kit; `spawn_set`, `spawn_entity`, `entity_dead_tag`; cover data in DK-02; the v6 start kit | D-DK5, M2 playtest | `npc-ai-spawn-advisor` with `combat-systems-advisor` |
| DK-20 | Hostile roster from template 462, spawn sets, cover sets | GDK1 record; PL-DK-A-10/A-11; the 279/324 gap warning | GDK1 | packet-coder |
| DK-21 | 1647 Enemy at the Gates: steps 4911, 4912, 4910, 4916, 4915, 4913, 4917, 4914; set 1834; dialogs 5982, 5822-5827, 6106, 6107 | GDK1 record; DK-06b for 4916 | DK-20, DK-06b, DK-12c | packet-coder (split per GDK1) |
| DK-13 | 1648 Loth'ta's Withdrawal: steps 4919, 4918; set 1835; dialogs 5983, 5828-5830; item 6781 | C6 tables | DK-21 (story order) | packet-coder |
| DK-14a | 1649 offer, Bra'tac 5832, Teal'c radio 5833 (DK-06b) | set 1836 | DK-13 | packet-coder |
| DK-14b | 1649 Command Terminal Livewire hack (5834, item 6782), drop sites 5835/5836, decode by `item_use` on 6780 (5837, item 6783), Bra'tac 5838 | Livewire chains 1016/1017, 1060/1061 | DK-14a | packet-coder |
| DK-15a | 1650 confrontation in the player's tent instance (5840: Ba'al, Bra'tac, Moh'katan) with mission-scoped actors | set 1837; template 443 | DK-14b | packet-coder |
| DK-15b | 1650 Moh'katan's Terminal search (item 6787), Bra'tac 5842 completes | set 1837 | DK-15a | packet-coder |
| DK-30 | 1651 Dakara steps 4932, 5171; SGC address grant; SGC steps with the route campaign | set 1838 | DK-15b, B-DK4 | packet-coder |
| DK-31a | 1652 Ring Control props at PL-DK-A-12/A-13; check whether `play_sequence` takes a sequence or event-set id | event sets 1194, 1195 | DK-30 | `rust-gameserver-dev` (evidence) |
| DK-31b | 1652 chains: overload a generator, the matching Ha'tak sequence for the acting player | set 1839; items 6784, 6785 | DK-31a | packet-coder |
| DK-32 | 1653 Superweapon: steps 4936, 4938, 4937, 4935; PL-DK-A-14 to A-16 | GDK1 record; DK-06b for 4937 | DK-31b, DK-20 | packet-coder (split per GDK1) |
| DK-33 | 1654 exit boundary, Dakara half: step 4939, set 1841 | | DK-32, B-DK4 | packet-coder |

## DK-99 close-out

**Role:** `documentation-writer` for the docs, `ops` for the operations. **Gate:** every other packet Done, Integrated with its UAT recorded, or recorded as BlockedDecision with the reason. **Branch / worktree:** `dakara/dk-99-closeout`, `dk-99`. **Commit:** `docs(dakara-e1): close out the Dakara_E1 rebuild campaign (DK-99)`.

Docs (one PR):

1. `docs/gap-analysis.md`, its area file under `docs/gap-analysis/`, and `docs/project-status.md`: once, for the whole campaign.
2. `docs/content/zone-audit.md`: rewrite the Dakara_E1 and Dakara_E1_StoryRm sections from the README matrices.
3. `docs/content/mission-chains.md`: one row per authored mission (63100 if D-DK1 F shipped, 1570 Dakara half, 1645, 1646 and whatever waves 4-6 shipped).
4. `docs/guides/unified-uat.md#dakara_e1`: the M1 and M2 steps still owed, by DK-U id.
5. README: the final PASS / BLOCKED matrix; every packet row Done or the reason it is not.

Operations, each one ops dispatch:

1. `pwsh tools/build-lane/rm-worktree.ps1 --merged` for every `dk-*` worktree.
2. Delete the stale remote branches `dakara/dk01-dead-end-fix` and `dakara/dk03-cast-templates` (both merged under other names).
3. If `.claude/state/active-campaign` reads `dakara-e1-rebuild`, the orchestrator writes it as an empty file. If it names another campaign, leave it.
4. Board: post the handoff in the `dakara-e1-rebuild` subcategory (created at the first dispatch with `board campaign create "dakara-e1-rebuild"`): what shipped, the UAT owed, the open owner decisions.

Checks: `pwsh tools/lint-md.ps1 <each changed file>`; `python tools/docs-gen/regen.py --check` if a generated block is near a changed line.
