# Dakara_E1 Rebuild Work Packets

> Type: how-to. Audience: the orchestrator, ops, packet-coder and domain agents, the packet-reviewer.
> Re-sliced 2026-10-18 to the [campaign-packet skill](../../../.claude/skills/campaign-packet/SKILL.md) against `main` @ `d1bba0400`. First written 2026-10-06 (DK-00, #1282). Companions: [ledger, findings and decisions](README.md), [resume card](RESUME.md), [audit](audit.md), [placements](placements/README.md), [testing playbook](../../../TESTING.md).

The packet status lives in **one place**: the [README packet table](README.md#packets). This file holds the rules, the contract and the packet text. Packet text for waves 1 to 6 is in [packets/](packets/). Waves 4 to 6 (missions 1648 to 1654, written 2026-10-19 by the architect, autonomous) build without the fights; wave 7 (the fights: GDK1, DK-20, DK-21, DK-32b) is blocked on D-DK5 and kept as stubs in [packets/wave-7-blocked.md](packets/wave-7-blocked.md).

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
| Content chains | 8001-8499 | 8001-8006 (`dakara_e1_space_chains.sql`) | 8007 | 8007-8039 space and travel; per-mission files in C3; 8180-8449 missions 1648 to 1654 (D-DK22, C3); 8390-8429 reserved for DK-21 (1647), 8450-8479 for DK-32b; 8480-8499 spare |
| Entity templates | 440-479 | 440-457 | 458 | 458-461 are the four extra Drop Location props (D-DK16, DK-09); 462 and 463 the second ring control and power supply (DK-41, D-DK29); hostiles 464-479 (DK-20) |
| Spawnlist | 8000-8099 | 8000-8007 | 8008 | 8008-8015 are DK-09's; 8016-8021 DK-41's six story props; DK-20 from 8022; 8098 DK-32c's Vocuum (wave 6c) |
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
| `dakara_e1_m1570_report_chains.sql` | DK-10d-B (8164-8165), then DK-10d-A (8160-8163, 8166-8171) | 8160-8179 (moved, D-DK19); 8172-8179 spare |
| `dakara_e1_m1645_chains.sql` | DK-11 | 8080-8119 |
| `dakara_e1_m1646_offer_chains.sql` | DK-12b | 8120-8139 |
| `dakara_e1_m1646_drop_chains.sql` | DK-12c | 8140-8159 |
| `dakara_e1_m1648_chains.sql` | DK-13 | 8180-8199 |
| `dakara_e1_m1649_offer_chains.sql` | DK-14a | 8200-8219 |
| `dakara_e1_m1649_terminal_chains.sql` | DK-14b, then DK-14c (edits 8223, adds victory chain 8231) | 8220-8249 |
| `dakara_e1_m1650_chains.sql` | DK-15, then DK-15b (edits 8258, adds victory chain 8261) | 8250-8289 |
| `dakara_e1_m1651_chains.sql` | DK-30a | 8290-8309 |
| `dakara_e1_m1651_gate_chains.sql` | DK-30b, then DK-30b-r (8310 becomes `stargate_crossed`, 8311 and 8312 deleted, grant chain 8313 added) | 8310-8319 |
| `dakara_e1_m1652_chains.sql` | DK-31b | 8320-8349 |
| `dakara_e1_m1653_chains.sql` | DK-32a; then DK-32c (edits 8351, 8355, 8357; adds 8364, 8365) | 8350-8389 |
| `dakara_e1_m1654_chains.sql` | DK-33, then DK-33-r (8432/8433 grant gate 27, 8434 becomes `stargate_crossed`, 8435 deleted) | 8430-8449 |

DK-40 (wave 4) creates the nine files from `dakara_e1_m1648_chains.sql` down, header only, with their `\ir` lines after `dakara_e1_m1646_drop_chains.sql`, their linter ranges and their test submodules (D-DK21). 8390-8429 (DK-21, 1647) and 8450-8479 (DK-32b) are reserved and get no file until wave 7. DK-10d's range sits after DK-12c's because 4902 and 4901 need 12 ids (D-DK19); the `\ir` order stays the order of this table.

Shared seed files and their single owner per wave: `entity_templates_dakara_e1.sql` and `spawnlist_dakara_e1.sql` (DK-09 in wave 2; in wave 6c DK-32c edits template 457's mesh and appends spawn 8098), `point_sets_dakara_e1.sql` and `point_set_points_dakara_e1.sql` (DK-09: the three SG-18 search regions, D-DK18), `crates/content-engine/tests/it/interact_tag_linter.rs` (DK-10a: `fn allowlist` only; DK-09b: moves the region check to `interact_tag_linter/regions.rs`; the two hunks do not overlap), `dakara_e1_space_chains.sql` (DK-10a edits only its header comment; DK-10e edits only chain 8002's mail text and adds chain 8007 after it). The mission seed `db/resources/Missions/Seed/missions_dakara_e1.sql` and its `\ir` line after `missions.sql` are DK-10fs's alone.

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
| Actions | `accept_mission` (target = mission), `advance_step` (target = mission, key = step text), `complete_objective` (target = mission, key = objective text), `complete_mission`, `display_dialog` (target = dialog), `display_radio_dialog` (target = dialog; sends `EntityId` 0, no NPC; from DK-06b, D-DK10), `add_dialog_set` / `remove_dialog_set` (target = dialog_set_map id, params `{"slot": <template_id>}`), `add_item` (target = item, params `{"container": 0, "qty": 1}`; container 0 means the item's own container set), `remove_item` (target = item, params `{"qty": 1}`), `increment_counter` (target NULL, key = name, params `{"amount": 1}`), `reset_counter` (target NULL, key = name, params `{}`) |
| Actions from wave 5 on | `spawn_entity` (target = template, key = a unique tag, params `{"x", "y", "z", "heading", "is_stationary"}`; refused on world 61 without `"allow_shared": true`, one-shot, no relog restore), `despawn_entity` (target NULL, key = tag; looks in the actor's space), `cross_world_teleport` (target NULL, key = world name, params `{"x", "y", "z"}`; its chain needs a `world` condition). A delayed action (`delay_ms` above 0) does not re-check conditions and is dropped on logout |
| From wave 6c on (owner rulings D-DK24, D-DK31) | `start_minigame` (target NULL, key `'Livewire'`, params `{"on_victory_chains": [V], "difficulty": 1}`): opens the hack; a win fires chain V by id. **Victory chain V** has a `content_chains` row and actions only: no trigger, no conditions (never evaluated), so every gate sits on the launcher (Castle 1356/1357). `grant_stargate_address` (target = gate id, key NULL, params `{}`): idempotent; Dakara grants only gate 27 in 8313, 8432, 8433, never from `player_loaded` (D-DK43). Trigger `stargate_crossed` (key = destination world name, e.g. `SGC_W1`): fires on the origin cell as the player steps through, with mission state and archetype filled (`event_dispatch/stargate.rs`, Castle 1360) |

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
| Story props (DK-41, D-DK29) | 453 `Dakara_E1_CommandTerminal` (61), 454 `Dakara_E1_StoryRm_MohkatanTerminal` (62), 455 `Dakara_E1_RingControl_XMinus` and 462 `_XPlus` (61), 456 `Dakara_E1_PowerSupply1` and 463 `_PowerSupply2` (61); spawns 8016-8021. DK-32c (D-DK45): 457 `Dakara_E1_Vocuum` (61), spawn 8098 | X-minus and supply 1 are west, X-plus and supply 2 east; the Vocuum pad is at the courtyard centre (A-14) |
| Ba'al hologram | template 443, spawned per player by DK-15 as `Dakara_E1_StoryRm_BaalHologram` at B-07 (66.2, -0.12, 34.3) | world 62 only |
| DHD | template 1, tag `Dakara_E1_DHD`, spawnlist 38, world 61 | an `interact_tag` chain runs before the dialling handler and suppresses it when it matches, so **no chain may match it** (wave 6c, D-DK24): the crossings complete on `stargate_crossed SGC_W1` |
| SGC gate | gate 27 `SGC W1`, world 58 `SGC_W1` | granted by 8313 (1651 report at 4932) and 8432/8433 (1654 offer); arrival pinned to P13 (167.00, -10.80, 18.00) on the Gate Room floor by DK-SGC-01 (D-DK44, option C, 2026-10-19); the Gate Control screen (template 37, spawn 68) dials home from DK-SGC-02 (D-DK40, D-DK46) |
| Livewire terminals (D-DK31) | 453 (1649, launcher 8223, victory 8231), 454 (1650, launcher 8258, victory 8261) | difficulty 1; decode (8224) and replay (8225) stay plain clicks (D-DK42) |
| Missions 1648 to 1654 | steps, objectives, dialogs, dsm rows and items per mission | inlined in each packet of [wave 5](packets/wave-5.md) and [wave 6](packets/wave-6.md) |
| Flag carriers (D-DK31) | 6760 (bit 30, no dialog), 6892 (no dialog), 6775 (bit 24) | bound in place of a step's flags-0 dsm so the prop or NPC is clickable |

Dialog set map rows (the `add_dialog_set` target):

| dsm | Dialog | Flags | Bound to | Use |
|---|---|---|---|---|
| 6890 / 6889 | 6110 Jaffa / 6111 Tau'ri | active | 441 Rak'nor | 1570 optional 5936 |
| 6727 / 6726 | 5811 Jaffa / 5810 Tau'ri | active | 59 Bra'tac | 1570 step 4906 |
| 6729 / 6728 | 5813 Jaffa / 5812 Tau'ri | active | 440 Loth'ta | 1570 step 4904 |
| 6730 | 5814 "weak radio signal" (speaker 0 only) | 0 | none: unused (D-DK18) | 5814 is shown by `display 5814` on the 4902 chains that advance to 4901 |
| 6731 / 6732 | 5815 Tau'ri / 5816 Jaffa radio (speaker 2609 Hammond and 0) | 0 | none (no NPC) | 1570 step 4905: shown by `display_radio_dialog` (DK-06b), never bound (DK-10d-A) |
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
| `report.rs` | DK-10d-B, then DK-10d-A (appends) |
| `m1645.rs` | DK-11 |
| `m1646_offer.rs` | DK-12b |
| `m1646_drops.rs` | DK-12c |
| `labels.rs` (unit test of the wave-4 label arms) | DK-40 |
| `m1648.rs`, `m1649_offer.rs`, `m1649_terminal.rs`, `m1650.rs` | DK-13, DK-14a, DK-14b, DK-15 (DK-40 creates them) |
| `m1651.rs`, `m1651_gate.rs`, `m1652.rs`, `m1653.rs`, `m1654.rs` | DK-30a, DK-30b, DK-31b, DK-32a, DK-33 (DK-40 creates them) |

From wave 5 on, `mod.rs` also gives `delays_of` (the `delay_ms` of each action of one chain) and `radio_count`, and `labels_in` labels `spawn_entity(T#tag)`, `despawn_entity(tag)`, `cross_world_teleport(World@x,y,z)` and `play_sequence(N)` (DK-40). From wave 6c, DK-40b adds the labels `start_minigame(Livewire,victory=[V],d=1)` and `grant_stargate_address(27)`, and two helpers: `fire_stargate_crossed(&engine, &ctx, "<destination world>")` and `fire_victory(&pool, V).await` (loads victory chain V alone, asserts it has no conditions, returns its actions as a `ResolvedActions`).

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
8. No new dialog rows, no client patch, no cooked-data override. The one exception is DK-10fs's single `MISSION_ADDITIONS` entry for mission 63100 (D-DK1 F); no other packet adds or edits one. The radio beat needs no client patch either: D-DK10 is ruled server-only (DK-06b's `display_radio_dialog`).
9. No IPs, credentials, tokens or account names in any file.
10. Never launch the game client, the launcher or a lab tool.

## Waves

| Wave | Packets | Gate to start | Reviewer |
|---|---|---|---|
| 1 | DK-06a, DK-07, DK-08, DK-12a | Done (#1853 to #1856) | one `packet-reviewer`, fixes by `packet-coder` |
| 2 | DK-09, DK-09b, DK-10a, DK-10e, DK-10fs | Done (#1867, #1869, #1871, #1875, #1876) | one `packet-reviewer` plus `mission-systems-advisor` read-only; fixes by `packet-coder` |
| 3 | DK-10b, DK-10c, DK-10fc, DK-11, DK-12b, DK-12c | Done (#1881, #1883, #1885, #1887, #1889, #1892) | one `packet-reviewer` plus `mission-systems-advisor` read-only; fixes by `packet-coder` |
| 3b | DK-06b (text in packets/wave-3b-dk-06b.md), DK-10d-B | Done (#1894, #1896) | one `packet-reviewer` for the pair; fixes by `packet-coder` |
| 3c | DK-10d-A (wave-3b review findings folded in) | Done (#1897) | one `packet-reviewer`; fixes by `packet-coder` |
| M2 | owner UAT | waves 3, 3b and 3c merged: **runnable now** | the owner, himself (no agent lab use); DK-U11 to DK-U17 |
| 4 | DK-40 (scaffold), DK-41 (story props) | Done (#1898, #1899) | one `packet-reviewer` for the pair |
| 5 | DK-13, DK-14a, DK-14b, DK-15 | Done (#1903, #1904, #1905, #1906) | one `packet-reviewer` plus `mission-systems-advisor` read-only |
| 6 | DK-30a, DK-30b, DK-31a, DK-31b, DK-32a, DK-33 | DK-31a #1900, DK-30a #1907, DK-31b #1909, DK-32a #1910 merged (docs #1901). **Held:** DK-30b (`34abe129b`, `dk-30b`) after SGC US-30, DK-33 (`41686b50c`, `dk-33`) after SGC US-31; reviewed, PRs not opened | one `packet-reviewer` plus `mission-systems-advisor` read-only |
| 6b | DK-31c (Ha'tak sequences), DK-30b-n, DK-33-n (review nits in the held worktrees) | Done: DK-31c #1913; DK-30b-n `8b1ac4673` and DK-33-n `ffec26d33` committed in the held worktrees | one `packet-reviewer` for the three; fixes by `packet-coder` |
| 6c (current) | DK-40b and DK-32c, then DK-14c, DK-15b, DK-30b-r, DK-33-r ([wave 6c](packets/wave-6c.md)) | DK-40b Done (#1929); DK-32c Ready; the four committed, review pending (`dk-30b` and `dk-33` rebased onto `e7459ddcb`). DK-30b-r and DK-33-r keep the merge holds (US-30, US-31) plus the DK-SGC PR (option C, 2026-10-19) | one `packet-reviewer` for DK-40b and DK-32c; one for the four; fixes by `packet-coder` |
| 6d (current, beside 6c) | DK-SGC-01, then DK-SGC-02 ([plan](sgc-return-plan.md)), owner option C (2026-10-19) | **Ready now**: one new worktree `dk-sgc` from `main`, the two packets in sequence, one PR. Shares no file with 6c | a review loop per packet; DK-SGC-02's pass covers both commits (the wave review); fixes by `packet-coder` |
| 7 | GDK1, DK-20, DK-21, DK-32b | D-DK5 decided A (owner, 2026-10-19). **BlockedDependency**: the owner's M2 playtest, then the architect re-slices | architect expands first |
| close | DK-99a to DK-99e ([wave 99](packets/wave-99.md)) | DK-99a: Done (#1914). DK-99e: DK-14c, DK-15b, DK-32c and DK-SGC merged. DK-99b and DK-99c: every shipping code packet merged (D-DK38). DK-99d: the DK-99 PR merged | one `packet-reviewer` for DK-99e; one for DK-99b and DK-99c |

Waves 5 and 6 are dormant in play until DK-21 ships 1647 (D-DK23): 1648 is offered on `done 1647`. Their packets are built and tested now; build order is not story order. Waves 4 to 6c need no lab and no owner input to build; DK-30b-r and DK-33-r need US-30/US-31 and the DK-SGC PR to merge (owner option C, 2026-10-19).

Packet text: [packets/wave-1.md](packets/wave-1.md), [packets/wave-2.md](packets/wave-2.md) with [packets/wave-2-dk-10fs.md](packets/wave-2-dk-10fs.md), [packets/wave-3.md](packets/wave-3.md), [packets/wave-3b-dk-06b.md](packets/wave-3b-dk-06b.md), [packets/wave-4.md](packets/wave-4.md), [packets/wave-5.md](packets/wave-5.md), [packets/wave-6.md](packets/wave-6.md), [packets/wave-6b.md](packets/wave-6b.md), [packets/wave-6c.md](packets/wave-6c.md), [packets/wave-7-blocked.md](packets/wave-7-blocked.md), [packets/wave-99.md](packets/wave-99.md).

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

## Waves 4 to 7: where the packet text is

The 2026-10-18 stubs were expanded on 2026-10-19 (architect, autonomous). What changed from the stubs, and why:

| Packet | Text | Change from the stub |
|---|---|---|
| DK-40 | [wave 4](packets/wave-4.md) | New: one scaffold packet creates the nine seed files, `\ir` lines, linter ranges, test submodules and label arms, so waves 5 and 6 never share a file (D-DK21) |
| DK-41 | [wave 4](packets/wave-4.md) | New: the six story props the chains click (D-DK29); templates 462 and 463 move the hostile block to 464 |
| DK-13 | [wave 5](packets/wave-5.md) | As stubbed; no longer waits for DK-21 (D-DK23: dormant until 1647 ships) |
| DK-14a | [wave 5](packets/wave-5.md) | The offer is a radio dialog on entering world 61 (D-DK25); the Teal'c call is `display_radio_dialog` |
| DK-14b | [wave 5](packets/wave-5.md) | Shipped as plain terminal clicks through the carrier 6760 (no `item_use`, F32). **Owner ruling 2026-10-19 (D-DK31): the hack is Livewire**, so DK-14c (wave 6c) makes 8223 a launcher |
| DK-15 | [wave 5](packets/wave-5.md) | DK-15a and DK-15b merged into one 40k packet; step 4927 is the tent region (D-DK28). DK-15b (wave 6c) puts Livewire on Moh'katan's terminal (D-DK31) |
| DK-30a, DK-30b | [wave 6](packets/wave-6.md) | DK-30 split: Dakara steps, then crossing C2. DK-30b built a DHD-click teleport; **owner ruling 2026-10-19 (D-DK24): real gate travel**, so DK-30b-r (wave 6c) replaces it and grants gate 27 at Bra'tac's report |
| DK-31a, DK-31b | [wave 6](packets/wave-6.md) | DK-31b ships without sequences (D-DK30); DK-31a is a desk note feeding DK-31c |
| DK-32a | [wave 6](packets/wave-6.md) | The peaceful 1653 path (courtyard, supplies, radio, report); the guards are DK-32b (wave 7) |
| DK-33 | [wave 6](packets/wave-6.md) | Crossing C3 by DHD click; merge held for US-31. DK-33-r (wave 6c) makes it real gate travel and grants gate 27 on the offer (D-DK24) |
| DK-31c, DK-30b-n, DK-33-n | [wave 6b](packets/wave-6b.md) | Expanded 2026-10-19 from DK-31a's note and its map-read addendum: four co-firing chains 8327-8330 (D-DK32 to D-DK36). The two nit packets are the wave-6 review's findings on the held packets |
| DK-40b, DK-32c, DK-14c, DK-15b, DK-30b-r, DK-33-r | [wave 6c](packets/wave-6c.md) | New 2026-10-19 for the owner's rulings D-DK24 (real gate travel, SGC address) and D-DK31 (Livewire required). DK-40b adds the shared labels and helpers first so the four never share a file. DK-32c (added the same day, D-DK45 owner-delegated) builds 1653's Vocuum beside DK-40b; it needs neither |
| GDK1, DK-20, DK-21, DK-32b | [wave 7 (blocked)](packets/wave-7-blocked.md) | D-DK5 decided A (owner, 2026-10-19); BlockedDependency on the owner's M2 playtest before the re-slice |

## DK-99 close-out

Split into DK-99a to DK-99d on 2026-10-19 (D-DK38), plus DK-99e for wave 6c; the packet text is [packets/wave-99.md](packets/wave-99.md). In short: DK-99a put the ledger's DK-U11 to DK-U31 in the unified UAT guide and renamed its placement rows DK-P1 to DK-P9 (D-DK37; merged #1914). DK-99e refreshes the Livewire and gate-travel rows and adds DK-U32 and DK-U33 once DK-14c and DK-15b merge. DK-99b (`mission-chains.md`, `zone-audit.md`) and DK-99c (gap analysis, project status, final ledger, one PR with DK-99b) wait for every shipping code packet, which depends on D-DK5. DK-99d retires the worktrees, deletes the 22 merged `origin/dakara/*` branches (D-DK39), posts the board handoff and has the orchestrator empty `.claude/state/active-campaign`. The owner's UAT never blocks the close-out.
