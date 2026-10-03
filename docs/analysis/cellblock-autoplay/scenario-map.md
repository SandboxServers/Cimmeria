# Castle Cellblock automated walkthrough: scenario map and gap list

> Type: reference (research). Audience: packet workers on the Cellblock autoplay campaign.
> Updated: 2026-09-29. Companions: [README.md](README.md), [work-packets.md](work-packets.md), [livewire-autosolve.md](livewire-autosolve.md) (supersedes §5 LIVEWIRE here).

Researched 2026-09-29 against `main` at d8bff1baa, read-only; no client was driven. The client UI files (bindings, Lua, layouts) were read from the installed QA client at `%LOCALAPPDATA%\Stargate Worlds\Working\SGWGame\Content\UI`. Anything I could not confirm is marked **(unverified)**.

Sources: `docs/guides/automated-uat.md`, `docs/guides/uat-specs/{castle-cellblock,crafting,consumables}.toml`, `docs/analysis/castle-cellblock-rebuild/uat-guide.md` (T01-T32), `docs/analysis/lab-automation/tooling-backlog.md`, `docs/guides/unified-uat.md` (the AMMO rows), `crates/lab/src/uat/{tools,spec,clause}.rs`, `crates/lab/src/uat/runner/{mod,session}.rs`, `crates/lab/src/server/*`, `crates/lab/src/supervisor/{world,ui,combat}/*`, `crates/lab-mcp/src/tools/{mod,db}.rs`, `crates/wire/src/cell/messages/lab.rs`, `crates/wire/src/state_field.rs`, `crates/entity/src/interaction_flags.rs`, `crates/cell/src/cell/service/ticks/{cover,auto_cycle,holster}.rs`, `crates/cell-combat/src/cell/combat/auto_cycle.rs`, `crates/cell-methods/src/cell/cell_methods/{combatant,minigame}.rs`, `crates/cell-content/src/cell/content/consumable_use.rs`, `crates/minigame/src/minigame/games/livewire/*`, and the seed files spawnlist, entity_templates, texts, point_sets/point_set_points, loot, abilities, items_event_sets and castle_cellblock_chains (chain 1034).

---

## 1. The runner: one fresh character across rows

### What works today

- `[section] character = "fresh"` plus `[section.fresh]` creates **one character per section per run** (`Runner.fresh`, `runner/session.rs:140`).
- Later rows reuse that character: `ensure_state` sees `in_world_as == want` and neither relogs nor recreates.
- Rows run in file order. One section of rows W00..W19 therefore carries a single character through the whole walkthrough.
- Relogs inside a row work: a `lab_logout` step followed by `lab_play_character`. The next row's `ensure_state` re-reads `lab_client_status`.

### What breaks

1. **Resume.**
   - `run_dir` reloads the manifest but starts with an empty `fresh` map and `in_world_as = None`.
   - The name is `letters_name(run_id)`, so it is unchanged, and `lab_create_character` fails ("name taken"). The row comes back BLOCKED.
   - A full walkthrough runs 45-90 minutes, far past any MCP call timeout, so it will need to resume.
2. **The intro dialog is closed before T01 asserts it.** After creating a fresh character, `ensure_state` calls `lab_finish_dialog` (`session.rs:167`), which closes dialog 2982.
3. **A failed row does not stop the section** (`runner/mod.rs:9`: "a failed row never stops the section"). Every later walkthrough row then fails for the wrong reason.
4. **Two fresh sections in one run share one name** (`letters_name(run_id)`). You cannot do a Tau'ri pass and a Jaffa pass in the same run.
5. **`capture` reads chat only.** No tool JSON (entity ids, `player_id`, ammo counts) can be stored in a `${var}`.
6. **The clause ops have no delta, bit-test or array-search.** Only eq/ne/contains/matches/gt../len_gte exist. `contains` and `matches` compare the JSON text of arrays, so a regex over `/diff/by_item` works but is brittle.

### Runner changes needed, by priority

| # | Change | Why |
|---|---|---|
| R1 | On resume, adopt an existing character named `letters_name(run_id)` found at character select (or persist `fresh` in run.json) | Lets `run_dir` resume a long walkthrough |
| R2 | `[section] sequential = true`, or a row `depends_on`: once a row is not PASS, NEEDS_HUMAN or UNVERIFIED, the following rows are BLOCKED ("prior row X = FAIL") | One-character chain |
| R3 | `[section.fresh] finish_intro = false` | T01 needs dialog 2982 still open |
| R4 | `capture = "tool"` / `"server"`, with `tool`, `args`, `pointer`, `var` | Instance entity ids, `player_id`, ammo and stack baselines |
| R5 | Automatic `${player_id}` / `${entity_id}` after `ensure_state` (from `server_sessions` by name, or `client_player_state`) | Every server clause and `server_entity_get` needs one |
| R6 | New ops: `delta` against a captured var, `bit_set` / `bit_clear` (for `state_field`, `interaction_type_flags`), `any` (an array element matches a sub-object) | Consumption counts, BSF bits, and "item X is in the Mission container" |
| R7 | `repeat` / `until` on an action (`until` = a clause, plus `max` and `interval_ms`) | Fight until dead; slap pack when HP < N; retry a click |
| R8 | A per-row `timeout_ms`, and partial-row resume | T14/T15 fights, T16 timing |
| R9 | A fresh name per section: `letters_name(run_id + section)` | A second archetype pass in the same run |
| R10 | Fix the capability-table drift (below) and the `consumables.toml` I2 args | Rows that are wrongly BLOCKED today |

---

## 2. Tools: `main` vs the installed supervisor, and redeploying labd

**The installed supervisor predates #1099-#1102 (confirmed).**

- `.mcp.json` launches `%LOCALAPPDATA%\cimmeria-lab\bin\cimmeria-lab.exe` over stdio. The binary is dated 2026-09-29 02:31.
- A `grep -a` of the exe finds only: `client_call_native, client_console, client_cursor_move, client_entity_table, client_events_read, client_hook_{install,list,remove}, client_input_{focus,key,mouse,release,status}, client_lua_eval, client_mem_{read,write}, client_module_info, client_type_text, client_ui_click, client_ui_state, client_wait_for`, plus `lab_*` (characters, client start/stop/restart/status, crash_report, create/delete character, ensure_character_slot, finish_dialog, login, logout, pixel_probe, play_character, screenshot(_region), timeline).
- It contains no `lab_uat_*`.
- It was built from a tree at or before #1094 (merged 01:54), even though it is dated after #1099 merged (02:03).

**On `main`:**

| PR | Tools |
|---|---|
| #1099 (world) | `client_entity_find`, `client_world_click`, `client_target`, `client_move_to`, `client_camera` |
| #1100 (combat) | `client_hotbar`, `client_use_ability`, `client_combat_log`, `client_die_and_respawn`, `client_wait_event` |
| #1101 (UAT) | `lab_uat_run`, `lab_uat_report`, `lab_uat_attest` |
| #1102 (UI) | `client_window_read`, `client_window_click`, `client_chat_log`, `client_inventory`, `client_player_state`, `client_item_action`, `client_drag_drop` |

**Drift on `main`:**

- `crates/lab/src/uat/tools.rs` maps `@window_click_row` to `client_window_click_row`, but #1102 shipped **`client_window_click`**. Any spec using the alias is BLOCKED.
- `docs/guides/uat-specs/consumables.toml` row I2 passes `{type_id, action}` to `@item_action`, but `ItemActionArgs` has no `type_id`: it takes `container`+`slot`, `item_id` (the instance id) or `name`. It also asserts `@inventory` `{type_id}` `/total`, and neither the argument nor that pointer exists.
- `docs/gameplay/combat-system.md` lists holster as STUB, but `requestHolsterWeapon` is implemented (`combatant.rs:60`).

**Redeploying.** The repo has no install script. The documented `.mcp.json.example` points at `<repo>\target\debug\cimmeria-lab.exe`; the AppData copy is a local convention.

1. Build through the lane:
   - `bash tools/build-lane/lane.sh cargo build -p cimmeria-lab`
   - i686 for the client side: `-p cimmeria-client-telemetry --features lab-bridge --target i686-pc-windows-msvc`, `-p cimmeria-client-patches --target i686-pc-windows-msvc`, and `-p cimmeria-start32 --target i686-pc-windows-msvc`.
   - The #1100 and #1102 readers install Lua rings through the bridge, so rebuild the DLL too.
2. `lab_client_stop`, then close the Claude session that holds the stdio child. The exe is locked while it runs.
3. Copy `cimmeria-lab.exe`, `sgw-start32.exe`, `cimmeria_client_telemetry.dll` and `cimmeria_client_patches.dll` into `%LOCALAPPDATA%\cimmeria-lab\bin\`.
4. Restart or reconnect MCP and check that `client_move_to` and `lab_uat_run` are listed.

**Server tools.** `lab-server` defaults to the colo URL, which answered HTTP 403 on 2026-09-29. For the walkthrough, set `CIMMERIA_LAB_MCP_URL` and `CIMMERIA_LAB_MCP_TOKEN` in the cimmeria-lab env to a local server. Otherwise every `server` clause is UNVERIFIED.

---

## 3. Facts every row depends on

### 3.1 Movement

- **`client_move_to` is straight-line closed-loop steering, not pathfinding.**
  - It holds W, turns with mouse-look, and reads the pose every 100 ms.
  - Legs are the `waypoints` in order, then the target.
  - If progress stays under 0.5 m for 2.5 s it tries jump, strafe right and strafe left (3 attempts), then fails "stuck".
  - A jump of more than 30 m in one tick fails the call as a teleport.
  - Timeout defaults to 60 s (max 600 s). Arrival radius is 1.5 m for a point and 2.5 m for an entity. Points are server metres, Y up, by default.
- **Navmesh.** `data/spaces/castle_cellblock.nav` exists but only the server uses it (NPC `find_path`).
  - The spawnlist notes that it covers the stasis room "only in patches".
  - Preparation (Region10) and topside are disconnected components joined only by the rings.
- **Doors** are client Kismet plus UE3 collision: sequence 10000 is the stasis door, 1749 the cell door. `move_to` stops at a closed door as "stuck", which doubles as a negative check.
- **Every multi-room leg needs hand-authored waypoints**, and there are none in the repo. Gap G-NAV1.
- **Room map** (point-set bounding boxes, server metres; use for waypoints and region assertions):

| Region (set) | Room | x range | y | z range |
|---|---|---|---|---|
| Region1 (2032) | stasis room | -347..-319 | 73-79 | -241..-214 |
| **Region8 (2039)** | corridor out of the stasis room (**NID guard ambush**) | -330..-298 | 72-80 | -203..-170 |
| Region2 (2033) | cell block, Prisoner 329 | -311..-242 | 65-75 | -141..-115 |
| Region11 (2042) | med station (Ambernol, drone) | -241..-209 | 65-82 | -134..-109 |
| Region14 (2045) | CellblockRing1 pad | -215.3 | 65.9 | -121.4 |
| Region16 (2047) | CellblockRing2 pad (Preparation) | -192.7 | 55.2 | -154.9 |
| Region10 (2041) | Preparation room | -202..-176 | 55-60 | -159..-114 |
| Region17 (2048) | CellblockRing3 pad (topside) | -89.7 | 45.2 | -161.6 |
| Region9 (2040) | topside corridor | -85..-70 | 45-51 | -170..-153 |
| Region3 (2034) | Mess Hall | -107..-72 | 34-46 | -133..-84 |
| Region4 (2035) | hallway | -109..-86 | 40-45 | -81..-55 |
| Region5 (2036) | Hallway05 | -109..-84 | 25-30 | -54..-34 |
| Region6/12 (2037/2043) | barracks / crate | -149..-115 | 25-32 | -147..-75 |
| Region7/13 (2038/2044) | armory | -89..-36 | 24-32 | -162..-124 |

  The walking order is **Region1 → Region8 → Region2 → Region11**: T26's ambush comes *before* Prisoner 329, not after.

### 3.2 Targeting (client names resolved from `texts`)

| Tag | spawn | template | name_id → client name | pos (server m) | Targeting |
|---|---|---|---|---|---|
| ArmYourself_FrostBody | 19 | 14 | 7031 "Corporal Frost" | (-328.30, 73.47, -210.27) | `name:"Corporal Frost"` |
| ArmYourself_GuardBody | 15 | 21 | NULL → **no name** | (-322.51, 73.47, -209.83) | `point` / `entity_id` |
| ArmYourself_NIDGuard | 20 | 15 | 6961 "Cellblock Guard" (loot 2) | (-289.46, 68.54, -154.28) | `name:"Cellblock Guard"` |
| Prisoner_329 | 6 | 17 | 7030 "Prisoner 329" | (-289.81, 65.47, -113.14) | name |
| 329_CellDoorButton | 22 | 22 | NULL | (-287.29, 67.26, -115.04) | point |
| ArmYourself_AmbernolVial | 12 | 18 | NULL | (-234.04, 66.52, -124.70) | point |
| ArmYourself_PrisonerRetrievalUnit | 10 | 4 | 7599 = **empty string** | (-220.26, 66.74, -121.38) | point / `entity_id` / `hostility:"Hostile"` + distance |
| HackTheRings_Switch | 23 | 3 | NULL | (-218.08, 67.04, -122.72) | point |
| Preparation_ColMarsh | 7 | 10 | 7569 "Colonel Marsh" | (-191, 54.72, -138.59) | name |
| Preparation_SMG1A | 11 | 8 | NULL | (-201.25, 56.08, -131.61) | point |
| Preparation_Terminal | 16 | 19 | NULL | (-187.71, 55.85, -141.50) | point |
| Preparation_RingSwitch | 17 | 3 | NULL | (-193.92, 56.32, -152.16) | point |
| MessHall_Guard1/2, Hallway01-05, Barracks_Guard1-3, Cellblock_ArmoryGuard1 | 29/28, 30/82/31/86/32/33, 25/26/36, 27 | 24 | 7417 "NID Guard" (loot 2) | see the uat-guide | `name:"NID Guard"` gets the nearest; ambiguous for pairs |
| Cellblock_WoodenCrate | 8 | 13 | NULL | (-130.45, 24.67, -92.07) | point |
| Cellblock_TerminalX | 34 | 19 | NULL | (-52.66, 25.76, -151.13) | point |
| Cellblock_ArmoryRingSwitch | 79 | 3 | NULL | (-54.88, 26.08, -163.84) | point |
| Castle_SgtGerschon (world 8) | 112 | 149 | 7034 "Sgt. Gerschon" | (429.64, 70.11, 996.56) | name |

- `client_world_click {point}` clicks a world point with **no mouse-over check**. Unnamed static objects can be clicked by their seed coordinates (the same in every instance), at N1. Whether the click lands on the object's pick volume at that height is **(unverified)**; add a `y` offset if not.
- `client_entity_find {mob_id}` filters on `unitMobId`. Whether that equals the server template id is **(unverified)**.
- Click and move accept only `entity_id`, `name` or `point` (no `mob_id`). Entity ids are the BigWorld ids shared by client and server, but a spec cannot capture them (R4). Gap G-TGT1.

### 3.3 Machine-checkable evidence (verified shapes)

| What | Clause |
|---|---|
| Mission state | `server` `server_db_query {sql:"select m.mission_id, m.current_step_id, m.status, m.completed_objective_ids from sgw_mission m join sgw_player p using (player_id) where (p.player_name='${character}' or p.extra_name='${character}') and m.mission_id=622"}`, pointer `/rows/0/current_step_id` or `/rows/0/status`. Status is 0 active, 1 completed (`crates/entity/src/missions.rs`); a missing row means not accepted. The row persists about one round trip late, so put `wait_ms: 1500` first. In-memory counters (`messhall_kills`) and fired-once chains are not readable (tooling-backlog S2). |
| Entity state | `server_entity_get {entity_id}`: `/ai_state`, `/current_target_id`, `/health_cur`, `/state_field`, `/weapon_holstered`, `/interaction_type_flags`, `/tag`, `/position`. `server_entity_query {template_id, space_id, class_id, radius}` finds ids. |
| BSF bits (`state_field`) | Dead 0x1, AutoCycling 0x2, Crouching 0x4, InCombat 0x8, MovementLock 0x40. The client only dispatches bits 0-7. |
| Interaction bits | RingNetwork 32, MinigameLivewire 256, A-story mission pending/available/active/turn-in 4194304/8388608/16777216/33554432, non-A-story 67108864/134217728/268435456/536870912, MissionWorldObject 1073741824, NormalLoot 4611686018427387904, Attackable 2305843009213693952 |
| Dialog on screen | `tool client_ui_state` `/dialog/text`, or `client_window_read {kind:"dialog"}` `/state/active_text`, `/primary/...` buttons. The client never exposes a dialog id. |
| Greet topics | `client_window_read {kind:"greet"}` `/state/topics` (`[{index,text,level}]`); `len_gte` / `matches` |
| Loot window | `client_window_read {kind:"loot"}` `/state/count`, `/state/items` (`getLootInfo`, 1-based) |
| Inventory | `client_inventory {containers:["Mission"], snapshot:"x"}`, then `{diff_against:"x"}` `/diff/by_item` (`[{name,before,after,delta}]`); `lua` clause with `getNameForSlot` / `getQuantityForSlot` for exact counts (the consumables.toml idiom) |
| Player | `client_player_state`: `/health/current`, `/health/max`, `/ammo/current`, `/ammo/max`, `/ammo/active_slot`, `/ammo/ammo_type`, `/ammo/weapon_item_id`, `/world_id`, `/position`, `/effect_names`, `/in_combat`, `/target/...` |
| Client events | `client_wait_event {kind:"cme.event", name:"*onDialogDisplay*"}`; also `*onMissionUpdate*`, `*onSequence*`, `*onStartMinigameDialog*`, `*onEndMinigame*`. Throttled per name (burst 8, then 4/s), which is fine at tutorial rates, but events carry **no payload** (backlog C5). |
| Chat and barks | `client_chat_log {cursor, speaker, contains}` |
| Combat | `client_combat_log` (`mortal`, hits, source/target is_player) |
| Server logs | `signoz` clauses (PENDING until `lab_uat_attest`) |

---

## 4. The walkthrough

One section: `id = "castle-cellblock-walkthrough"`, `character = "fresh"`, `[section.fresh] alignment = "sgu", archetype = "Commando", gender = "male"` (Tau'ri branch), plus R2 `sequential` and R3 `finish_intro = false`. Rows default to `state = "in_world"`; the `.bug` anchor needs the GM account (the lab account is GM). `required_native = "N1"` unless noted.

Common blocks are defined in §5: **FIGHT** (target, auto-attack, reload, slap-pack policy), **LOOT** (corpse loot) and **LIVEWIRE**.

### W00 · T25: Prison Boot gate

- **Pre:** a new character, first load, in the stasis room at about (-334.23, 73.47, -228.03). `lab_play_character` presses Esc through the intro, which also releases the first-login AoI hold at once.
- **Do:**
  - `client_player_state` (spawn position).
  - `client_move_to {point:{x:-331,y:73.47,z:-226}, timeout_ms:8000}` with `optional: true`: can the player walk?
  - `client_inventory` (equipment).
  - `client_item_action {action:"use", name:"Prison Boots"}` with `optional: true`: can an equipped item be used?
  - If a Livewire opens, run LIVEWIRE.
- **Assert:**
  - `server_db_query` mission 689 status 0.
  - `client_ui_state /mission_tracker` not_contains "Prison Boot".
  - `client_wait_event {name:"*onMissionUpdate*", timeout_ms:3000}` → met:false (#715: no frames for a hidden mission).
  - SigNoz `mission client frames suppressed` mission_id=689 site=accept.
  - Inventory holds 3438 equipped.
  - Walked or not: record only.
  - `server_entity_get {player} /state_field` bit 0x40 (MovementLock), expected clear because the effects are inert.
- **Gaps:** G-MG1 (Livewire). No reader for whether a "use" affordance exists on an equipped slot: the `client_item_action` result is the evidence.

### W01 · T01/T02: Arrival, mission 622, Stasis Sickness

- **Pre:** needs R3.
- **Do:** clauses first, then `lab_finish_dialog` (label `close-2982`).
- **Assert:**
  - `/dialog/text` contains "The last thing you remember".
  - 622 is on step 2113.
  - `client_player_state /effect_names` (K7: no icon expected; record it).
  - SigNoz `launched ability` 1372 chain 1112; dialog 2982 exactly once; 622 accepted once.
  - `client_wait_event {name:"*onDialogDisplay*", count:1, since arm}`.

### W02 · T03/T04: Frost, the Guard, Frost's Letter, the pistol

- **Do:**
  1. `client_move_to {name:"Corporal Frost"}`.
  2. `client_world_click {name:"Corporal Frost", expect:"window"}`.
  3. `client_window_read {kind:"greet"}`, then `client_window_click {root:"GreetWin", text:"Search Cpl. Frost's Corpse"}`.
  4. `lab_finish_dialog` (3995).
  5. Snapshot the inventory. Click Frost again with `expect:"any"`.
  6. `client_world_click {point:{x:-322.51,y:73.47,z:-209.83}, expect:"window"}` (Guard corpse, unnamed), then topic "Search the Guard's Corpse", then `lab_finish_dialog` (3996).
  7. Click the Guard again.
  8. `client_item_action {action:"equip", name:"SI 3 9mm Pistol"}` (label `equip-pistol`).
- **Assert:**
  - Mission inventory gains Frost's Letter (3730), exactly +1.
  - 1360 on step 4037, status 0.
  - 622 goes 2113 → 80623 → 80622 → status 1 after the equip.
  - Main gains item 55, container 1.
  - `/ammo/weapon_item_id` = 55 after the equip; `/ammo/current` = the clip size.
  - The re-clicks produce an empty `/diff/by_item`.
  - SigNoz sequence 10000 once.
- **Looting note:** Frost and the Guard are **dialog-set searches with chain grants, not loot windows** (templates 14 and 21 have no loot table). Assert "no duplicate grant" with the diff.
- **Risk #582/#838:** add the precondition clause `server_witnesses {GuardBody id}` (the player sees it) and `client_entity_find {point-near, rendered_only}` before clicking.

### W03 · T26: Region8 guard ambush

- **Do:**
  - `client_move_to {waypoints:[stasis door threshold ≈ (-338.4,74.4,-213.9) = corner D], point:{x:-314,y:74.2,z:-186.7}}`. The exact door waypoint is **(unverified)**.
  - Then FIGHT against "Cellblock Guard". It has weapon 55, so it drops loot table 2: a guaranteed slap pack plus 80% naquadah.
  - Then LOOT.
- **Assert:**
  - `server_entity_get {guard}` `/current_target_id` = the player and `/ai_state` Fighting, before the player fires.
  - SigNoz `fire_enter_region: matched region_tag=Castle_CellBlock.Region8`; `set aggression` then `generate threat 1000`, in that order.

### W04 · T05/T06: Prisoner 329 (Tau'ri)

- **Do:**
  - `client_move_to {point:{x:-289.8,y:65.47,z:-117}}` (waypoints from Region8 down to Region2 **(unverified)**).
  - `client_world_click {name:"Prisoner 329", expect:"window"}`.
  - Read greet topics. `client_window_click {root:"GreetWin", text:"Free Prisoner 329"}`. `lab_finish_dialog`.
- **Assert:**
  - Topics contain "Free Prisoner 329" exactly once (`matches` over `/state/topics`).
  - `/dialog/text` contains "You feel uncomfotable" (the typo is in the 2009 data).
  - 638 on step 2115.
  - `server_entity_get {329_CellDoorButton} /interaction_type_flags` has bit 256.
  - SigNoz `adding dialog set` exactly once, 2794 chain 1011.
  - `client_chat_log` has no system message 5040.

### W05 · T07: The cell-door Livewire

- **Do:**
  - `client_world_click {point:{x:-287.29,y:67.26,z:-115.04}, expect:"window"}`.
  - LIVEWIRE.
  - Talk to the prisoner: `lab_finish_dialog` (2299, agree), then `lab_finish_dialog` (blurb 2298).
- **Assert:**
  - Sequence 1749 (`*onSequence*` event + SigNoz).
  - Wrench bit 256 cleared.
  - 638 status 1; 639 on step 2117.
  - The greet no longer offers "Free Prisoner 329".
  - SigNoz accept 639 once.

### W06 · T08 order A: Ambernol pickup, the drone, cover

- **Do:**
  1. `client_move_to` into Region11, e.g. (-225, 66.5, -121) (waypoints **(unverified)**). Assert 639 on step 2145 (`fire_enter_region` Region11).
  2. `client_inventory {containers:["Mission"], snapshot:"pre-vial"}`.
  3. `client_world_click {point:{x:-234.04,y:66.52,z:-124.70}, expect:"any"}`, then `lab_finish_dialog` (2297).
  4. **Negative Ambernol check (Ambernol scope):** `client_item_action {action:"use", container:"Mission", name:"Ambernol"}`. Chain 1034 requires step 2343 and the native path stands aside, so nothing happens. Assert the diff is empty, 639 is still on 2144, and SigNoz `item use: an item_use chain owns this item` has `reason=chain_owns_item`. This is a silent no-op: record it against the first-press feedback rule.
  5. **Cover:** `client_move_to {point:{x:-233.0,y:65.46,z:-124.4}, arrival_m:1.0}`, then `wait_ms: 2500`. The server cover tick runs at 1 Hz.
  6. FIGHT against the drone (name is empty: target by `entity_id` via R4, or `client_target {point}` does not exist, so use `client_world_click {point:{x:-220.26,y:66.74,z:-121.38}, button:"left", expect:"target"}`).
- **Assert:**
  - Mission diff shows item 19 +1.
  - The vial is gone: `server_entity_query {template_id:18}` is empty, or `client_entity_find` finds nothing near the point.
  - Drone `current_target_id` = the player.
  - 639 on step 2144 after the pickup. After cover: `completed_objective_ids` contains 2484 and the step is still 2144. After the kill: step 2343.
  - SigNoz `fire_cover_entered: matched cover_set_id=1381`, `complete objective 2484 chain 1132`, sequence 10014 **exactly once**, 10001 once.
  - `human`: the TakeCoverIndicator shows, then hides.
- **Cover, verified in code:**
  - Cover is **server-side proximity only** (`ticks/cover.rs`): a 1 Hz sweep of player positions against cover nodes within `COVER_PROXIMITY_RADIUS = 5.0` m. The player enters it by walking there; **no key or client message is involved**.
  - Crouch (BSF 0x4) is only logged (`crouched=` in the journal and debug line), never required.
  - The server logs target `cover.detection` at DEBUG ("player crossed a cover-set proximity edge", `edge=entered`, `cover_set_id`, `nearest_node_dist`) and writes a player-journal `COVER_EDGE` entry.
  - Whether that DEBUG target reaches SigNoz depends on the OTEL filter **(unverified)**. The info-level `fire_cover_entered: matched` is the safer clause.
  - No client-side cover state exists (CoverIndicator/ holds only the AutoAttack and Reload button layouts).
- **Order B** (kill first) needs a second fresh character (R9) or a separate run, because the objectives fire once.

### W07 · T09: The cure (Ambernol use)

- **Do:**
  - `client_inventory {snapshot:"pre-cure"}`.
  - `client_item_action {action:"use", container:"Mission", name:"Ambernol"}`. N1 is a right-click on the Mission tab; the `/useitem` fallback would be N2.
- **Assert:**
  - `/diff/by_item` Ambernol delta -1 exactly (no double consume: `consumable_use.rs` stands aside for chain-owned item 19).
  - 639 status 1; 640 on step 2120.
  - HackTheRings_Switch flags have bit 256.
  - Blurb `/dialog/text` contains "Hack the Ring Transporter" exactly once (`*onDialogDisplay*` count 1).
  - SigNoz, in order within chain 1034: `launched ability 1374`, then `RemoveInventoryItemByType`, then `completing mission 639`, then `accepting mission 640`.
  - Effect: `/effect_names` unchanged. Effects 1634 and 1636 are inert, so there is nothing to see.
- **Relog sub-check (optional):** `lab_logout`, `lab_play_character`, then SigNoz shows no `ability_id=1372`.

### W08 · T10: Hack the rings; ring 1 → ring 2

- **Do:**
  - `lab_finish_dialog` (2305).
  - `client_world_click {point: HackTheRings_Switch}`, then LIVEWIRE.
  - `client_world_click` the same point, `expect:"nothing"`. The ring fires; do not wrap it in `move_to` (the 30 m teleport check would fail it).
  - `client_wait_for {lua_condition: "local p=unitPosition(Unit.Player) return p and math.abs(p.x+192.66)<6"}` (the lua field names are **(unverified)**; alternatively poll `client_player_state`).
- **Assert:**
  - 640 on 2215 after the hack (ring bit 32 set), then status 1.
  - Position within 5 m of (-192.66, 55.26, -154.84).
  - Marsh flags gain the A-story "available" bit.
  - SigNoz `fire_teleport_in: matched region_id=2`, `completing mission 640` once.

### W09 · T11/T12 + T30: Marsh, the P90 locker, the SMG (Tau'ri)

- **Do:**
  - `client_world_click {name:"Colonel Marsh", expect:"window"}`, greet topic if one appears, then `lab_finish_dialog` (4001), then `lab_finish_dialog` (blurb 4000).
  - `client_world_click {point: SMG1A}`.
  - `client_world_click {name:"Colonel Marsh", expect:"nothing"}` (he must not be talkable).
  - `client_item_action {action:"equip", name:"SGHC 6 SMG"}`.
  - `client_world_click` Marsh, then `lab_finish_dialog` (3999 to Done).
- **Assert:**
  - Steps 2121 → 80641 → 3563 → 3564.
  - The SMG (21) lands in **Main** (`slot_changes` container "Main"), not the Bandolier.
  - Exactly one SMG after a second locker click.
  - After the equip: `/ammo/weapon_item_id` = 21.
  - SigNoz: dialog 4001 once, 5022 zero times; `granting item item_id=21 container_id=1`; `fire_dialog_choice dialog_id=3999 button_id=-1`.
  - `client_window_read {kind:"dialog"}` shows no Accept, Decline or More Info buttons on 2305 and 4000 (T30).
- **Then run the WEAPON block (§5)** here: pistol and SMG are both on the bandolier.

### W10 · T13: The Preparation terminal Livewire

- **Do:** click the terminal point, LIVEWIRE, then `lab_finish_dialog` (3998).
- **Assert:**
  - 641 status 1; 680 on step 2344.
  - RingSwitch flags have bit 32.
  - No blurb on 680 (`*onDialogDisplay*` count 1 only).

### W11 · T27: Marsh's pre-departure line

- **Do:** click "Colonel Marsh".
- **Assert:** `/dialog/text` contains "That's about all we can do from here". Then `lab_finish_dialog`.

### W12 · T28 + T32 line 1: Rings to topside; Marsh follows

- **Do:**
  - `client_chat_log {cursor:"barks", peek:false}` (arms the cursor).
  - `client_world_click {point: Preparation_RingSwitch}`, then wait for the teleport.
  - `client_entity_find {name:"Colonel Marsh", rendered_only:true}`.
- **Assert:**
  - 680 on step 2345.
  - `client_chat_log {cursor:"barks", speaker:"Marsh"}` has one line, "Let's move out!".
  - Marsh is rendered within 5 m.
  - `server_entity_get {Marsh} /position` ≈ (-91.689, 45.188, -161.533).
  - `server_witnesses {Marsh}` includes the player (the #582 shape).
- **Escort:** at each later leg, add a clause `client_entity_find {name:"Colonel Marsh"}` `/matches/0/distance_m` `lte 15`. The pointer path is **(unverified)**.

### W13 · T14 (+T32 line 2): The Mess Hall, the main fight room

- **Do:**
  - `client_move_to` Ring3 → Region9 (-77, 46.2, -161) → Region3. The Mess Hall is one level down; the stair waypoints are **(unverified)**.
  - Check the bark ("I'll draw their fire").
  - Run the full **FIGHT + AMMO + SLAP-PACK** block against MessHall_Guard1, then Guard2.
  - LOOT each corpse.
- **Assert:**
  - On Region9: 680 status 1 before 681 is accepted.
  - 681 **is** visible in the tracker (is_hidden false; record it).
  - After the second kill: 681 status 1 and 682 accepted, not in the tracker.
- **T29 flank:** not exercisable. The Mess Hall and Hallway05 have no cover data, so record "flank not exercisable".

### W14 · T15 (+T32 line 3): Hallways 682-686

- **Do:** per guard, `client_move_to {name:"NID Guard"}` (nearest), then FIGHT, then LOOT. Hallway01 → 02 → Region4 → 03 → 04 → Region5 → Hallway05 Guard1 and Guard2.
- **Assert:**
  - Each kill completes one controller mission and accepts the next: `sgw_mission` rows 682-686, one each.
  - The tracker shows none of them.
  - Bark "Flank their position while I draw their fire!" exactly once on Region5.
  - SigNoz: one accept per mission.
- **T15b/T15c** (out-of-order kill, nearest respawner) are optional sub-rows. Use `client_die_and_respawn {respawn:"release"}` for T15c and assert `/position` near respawner 5.

### W15 · T16/T17 + T31: The Straegis scene

- **Do:**
  - `client_wait_event {arm:true}`.
  - The second Hallway05 kill (label `kill-686`).
  - `client_wait_event {name:"*onSequence*", timeout_ms:3000}`.
  - `client_wait_event {name:"*onDialogDisplay*", count:2, timeout_ms:15000}`.
  - `lab_finish_dialog`.
- **Assert:**
  - Marsh is gone (`client_entity_find` returns empty).
  - 686 status 1; 687 on step 2354; the crate flags gain a highlight.
  - Timing (2516 at about 10.1 s, 5859 at about 10.6 s) through SigNoz `firing deferred action`. The runner cannot yet compare two event timestamps (R6).
  - `human`: the camera.

### W16 · T18/T19: The Aftermath crate (a real loot window) and the barracks

- **Do:**
  - `client_move_to {point: crate}`.
  - `client_world_click {point:{x:-130.45,y:24.67,z:-92.07}, expect:"window"}`.
  - `lab_finish_dialog` (3942).
  - `client_window_read {kind:"loot"}`.
  - `client_item_action {action:"loot_slot", loot_index:1}` (one item), then `client_item_action {action:"loot_all"}`.
  - Click the crate again.
  - FIGHT the three barracks guards, LOOT each.
  - **Armor equip:** `client_item_action {action:"equip", name:"Covert Stealth Helmet"}` plus Vest, Pants, Gloves and Boots; then `client_item_action {action:"equip", name:"Combat Knife"}` (**(unverified)** whether the knife can go on the bandolier).
- **Assert:**
  - `/state/count` = 6; `/state/items` matches 3347, 3359, 3372, 3387, 3401 and 3325.
  - Inventory diff +6.
  - The reopen shows chat "You have already taken everything from this.", or count 0.
  - 687 on step 2355 at the window open; the crate highlight clears.
  - After the third kill: 687 status 1, 688 accepted, blurb 2518 once.
  - The equipment containers show the five pieces.
  - `human`: visible appearance (no appearance reader exists).

### W17 · T20: The Armory and the exit

- **Do:**
  - Move to TerminalX and click it at its point.
  - Optionally FIGHT and LOOT ArmoryGuard1.
  - Click the ArmoryRingSwitch point.
- **Assert:**
  - After the terminal: 688 on step 80688 **and status 0**. This is the auto-complete trap: the mission must not complete at the terminal.
  - After the switch: 688 status 1 and `/world_id` = 8.
  - SigNoz `cross-world teleporting entity` chain 1109.

### W18 · T21/T22: Arrival in Castle

- **Assert:**
  - `/position` within 3 m of (466.365, 70.397, 991.466).
  - `client_entity_find {name:"Sgt. Gerschon"}` distance within interaction range.
  - 1360 status 0 on step 4037; the Mission container still holds 3730.
  - `human`: appearance and colour.

### W19 · T23: Relog sweep (optional rows)

- At each boundary in the uat-guide's restore table: `lab_logout` + `lab_play_character`, then assert the restored bit with `server_entity_get /interaction_type_flags` (256 wrench, 32 ring icon, the A-story bits for Marsh or the locker).
- Assert that no one-shot replays: `*onSequence*` wait met:false for 5 s after login.
- Needs R1 only if the run resumes.

---

## 5. Blocks for the added scope

### FIGHT (targeting, auto-attack, sustained fire)

Verified from the client files `Common/Bindings/Bindings.toc` and `Core/AutoAttack/AutoAttack.lua`.

- **Target:** `client_target {name|entity_id}` (a left-click in the 3D view, N1).
- **Auto-attack toggle:**
  - The **T key** (`AttackTarget`) runs `AutoAttackMod.attackTarget`: `targetNextEnemy()` if nothing is targeted, then `setAutoAttack(not autoEnabled)`.
  - Alternatives: `client_input_key {key:"T"}` (N1), or `client_window_click {target:"AutoAttack_AutoAttackButton"}` (N1).
  - `/toggleAutoCycleAbility` also exists (N2).
- **Server behaviour** (`cell-combat/combat/auto_cycle.rs`):
  - `setAutoCycle(1)` lights `BSF_AUTO_CYCLING` (0x2) immediately. It fires immediately only if the server has both a selected `current_target_id` and a `last_fired_ability_id`; otherwise the first committed manual ability supplies the loop ability. The icon alone does not choose a target. See [auto-cycle-button.md](../../protocol/auto-cycle-button.md#observed-button-failure-and-telemetry-recipe-2026-09-29-colo).
  - It re-fires on each cooldown at the live `current_target_id`.
  - **It clears on target death, despawn, deselect, a manual different ability or `setAutoCycle(0)`, and NOT when the target is out of range** (the loop stays armed so the player can walk back in).
- **Assert:**
  - Lua clause `return AutoAttackMod.autoEnabled` (set by `Events.AutoCycle` from the server bit; N3 read).
  - `server_entity_get /state_field` bit 0x2 set while firing and clear after `mortal` (needs R6 `bit_set`; today `matches` on the decimal is fragile).
  - `client_combat_log` hits accrue with no key presses.
  - After the kill, no new player hits for 3 s.
  - Out of range: walk 40 m away, bit 0x2 **stays set** and hits stop; walk back and hits resume.
- **Sustained fire until dead:** needs R7 `until` `server_entity_get {target} /health_cur lte 0`. Workaround: `client_wait_event {kind:"combat.hit", fields:{mortal:true}, timeout_ms:60000}`.
- **Gap G-CMB1:** there is no `client_fight` composite (target → T → fire once → heal policy → wait until mortal, with a timeout). About 16 fights use it.

### AMMO (reload, clip, bandolier counts)

Verified from `Core/Reload/Reload.lua`, `Core/WeaponBar/WeaponBar.lua` and the bindings.

- **Reload:**
  - The **R key** is `ReloadAmmo`, which calls `reload(UIReloadType.ActiveWeapon)`: `client_input_key {key:"R"}` (N1).
  - Or `client_window_click {target:"Reload_ReloadButton"}` (N1).
  - The server `requestReload` (method 86) refills on a 100 ms completion tick after the warmup.
- **Readers:**
  - `client_player_state /ammo/current`, `/ammo/max`, `/ammo/ammo_type`, `/ammo/active_slot` (from `Stat.AmmoSlotN` and `getCurrentAmmoType`).
  - The weapon-bar text `WeaponBar_TextAmmo` ("cur/max").
  - `client_inventory` Bandolier per-slot ammo type and count.
- **Rows:**
  1. R4 capture `/ammo/current` as `ammo0`.
  2. Fire N shots, counting `client_combat_log` player-sourced records.
  3. Assert `/ammo/current` = `ammo0 - N` (R6).
  4. Press R, `wait_ms 2500`, then assert `/ammo/current` = `/ammo/max`.
  5. Default ammo draws nothing from the bags: the diff is empty (AMMO-10).
  6. SigNoz `reload_draw*` / `reload_refused` with `clip_before/after`.
  7. Press R on a full clip: expect a refusal or no-op; record the feedback (the first-press feedback rule).
- **AMMO rows that fit this run**, with `.giveammo` as G setup:
  - W00: AMMO-19 (debug-hub loot crate in the stasis room; Loot All needs about 18 free slots). Load a special type via the weapon-bar picker (`WeaponBarMod.onAmmoClicked`, which calls `requestAmmoChange`; the picker window names are **(unverified)**; read with `client_window_read {window:"WeaponBarWin"}`).
  - W13 (Mess Hall guards): AMMO-04 (Hollow Point reload draw = clip size), AMMO-05 (partial), AMMO-06/07 (short and empty stack; chat "You have no Armor Piercing rounds left."), AMMO-08 (switch returns rounds), AMMO-10 (default regression), AMMO-12/13 (damage multipliers and burn ticks via SigNoz `ammo_damage_applied`).
  - W06: AMMO-14 against the drone.
  - AMMO-20 needs Castle NIDs (outside the Cellblock), in W18.
  - AMMO-02/03/18 need two clients (`players=2`). AMMO-24 is operator-only. AMMO-11 needs the picker widget names.

### WEAPON (bandolier switch, holster)

- **Slot swap:** **F1-F5** (`ActivateBandolierSlotN`), e.g. `client_input_key {key:"F2"}` (N1).
  - The stock Lua is a **no-op when the cached active slot already equals N** (`client-wire-emit-suppression.md`), so always press the *other* slot.
  - Assert `/ammo/active_slot` and `/ammo/weapon_item_id` change (55 ↔ 21).
  - Assert the ammo stat follows the new slot.
  - SigNoz `bandolier` `active_slot` events (names **(unverified)**).
  - The previous slot's dirty ammo is flushed on the swap (ammo audit A-05).
- **Holster: no client binding exists.** No holster or crouch action is in `Bindings.toc` or the UI Lua. The only UE3 binding, `C` = Crouch, is shadowed by `ToggleCharacter` = C.
  - The server holsters **automatically** `OOC_HOLSTER_DELAY` = 10 s after leaving combat (`ticks/holster.rs`), and unholsters on fire or reload (`UNHOLSTER_DRAW_DURATION` = 1 s).
  - Assert: `server_entity_get /weapon_holstered` is false while fighting and true 10-12 s after the kill (`wait_ms 12000`).
  - A reload while holstered draws first.
  - Visible weapon (BeingAppearance): `human` or screenshot; there is no appearance reader.
  - `requestHolsterWeapon` (implemented) can only be sent by `client_call_native` or a slash command, if one exists **(unverified)**.

### SLAP-PACK policy (consumables)

- **Item:** Health Slappack TC1 = **item 2893**, ability 648 "Slappack Heal TC1" (+500 HP, **cooldown 0**, effect 712).
- **Where it comes from:** loot table 2, the shared guard table used by templates 15 and 24, drops **one per guard corpse, guaranteed** (plus naquadah 5-50 at 80%). The run collects them from W03 on.
- **Setup fallback:** `/gmgiveitem 2893 3` (G).
- **Policy** (needs R7): after each FIGHT tick, `repeat until` `client_player_state /health/current gte 0.5 * /health/max`. The ratio needs a lua clause: `return getUnitStat(Unit.Player, Stat.Health).current / ...` **(binding shape unverified)**. The action is `client_item_action {action:"use", name:"Health Slappack"}` (N1 right-click). Without R7, place one fixed `use` after each fight whose HP read is below threshold (a pre-check clause records it).
- **Assert:**
  - `/diff/by_item` Health Slappack delta -1.
  - `/health/current` rises by min(500, missing).
  - SigNoz `consumable_used decision_outcome=applied type_id=2893 ability_id=648`.
  - With cooldown 0, a second immediate use also works and should consume one more. Assert that, not a refusal.
- **Gap:** the consumables.toml I2 args are wrong (§2).

### LOOT (corpse looting)

- **Where:** every guard corpse (W03 Cellblock Guard; W13 ×2; W14 ×6; W16 ×3; W17 ×1). Each has loot table 2 and gets the `INT_NORMAL_LOOT` bit (4611686018427387904) when something drops; the slappack always drops.
- **Do:**
  - `client_world_click {entity_id:<corpse>, button:"right", expect:"window"}`. Needs the id (R4). By name, `"NID Guard"` returns the nearest, dead or alive; `client_entity_find` has no dead filter (G-TGT2).
  - `client_window_read {kind:"loot"}`.
  - `client_item_action {action:"loot_slot", loot_index:1}`, then `{action:"loot_all"}`.
  - Reopen.
- **Assert:**
  - `/state/count` ≥ 1 and `/state/items` includes the slappack.
  - Inventory diff: Health Slappack +1 and cash_delta 5-50.
  - The reopen shows an empty window or refusal chat, and no second grant.
  - `server_entity_get {corpse} /interaction_type_flags` loses the NormalLoot bit (**(unverified)** that the server clears it on empty).
  - SigNoz `Player looted item`.
- **Frost and the Guard corpse** are chain searches (W02), and the **Aftermath crate** is the only content loot window (W16).

### LIVEWIRE (T07, T10, T13, maybe T25). The flow, verified from the client Lua and `crates/minigame`:

1. `client_world_click {point: host}` (the host carries bit 256). The server sends `onStartMinigameDialog`.
2. **`StartMinigameWin`** opens: `client_window_click {target:"StartMinigame_PlayMinigame"}` (N1), which calls `playMinigame()`.
3. **`MinigameWin`** hosts the Flash SWF in `Minigame_Movie_Area` and talks SmartFox TCP to `crates/minigame`.
4. The game: after `opendoor`, the SWF sends `processmove {wirename}` per wire cut. The server counts wires whose name starts with `g` (goals: 2 at difficulty 1, 4 at 2-3, 6 at 4). A `victory` fires chains when `goal_cut == goal_total`. Cutting `o` (obstacle) or `m` wires speeds the countdown.
5. **No tool can play it (G-MG1).** The wire list and x/y (Flash stage coordinates) exist only in the server's session state.
   - Proposal: a `server_minigame_state {entity_id}` lab-mcp tool returning the live session's wires `{name, x, y, cut}` and stage state, plus a `client_minigame_click {stage_x, stage_y}` supervisor tool that maps stage coordinates into `Minigame_Movie_Area`'s screen rect (`client_window_read {window:"Minigame_Movie_Area", include_nodes:true}`) and clicks with real input (N1).
   - How the SWF's "open door" start is triggered by the player is **(unverified)**; watch one human run.
   - Interim: a GM `.minigame_win` console command that reports victory for the caller's open session (tier G, so these rows grade NATIVE_SHORTFALL). No such command exists today: `/debugminigamecomplete` is "Partly" and the cell's `endCurrentMinigame` / `debugStartMinigame` are `UNIMPLEMENTED` logs.
6. **Assert:** `client_wait_event {name:"*onEndMinigame*"}`, the MinigameWin hides, the chain's step advances, and SigNoz `send_minigame_result` result 1.

---

## 6. Archetype branches

- **The main run is Tau'ri Commando.**
- **The Jaffa pass** (`archetype="Jaffa"`, id 8) needs R9 or a second run. Only W04 (5021), W05 (5020), W09 (5022/5023, where Moh'katan appears on screen 8) and W16 (loot table 11: 3482 jacket and 2797 Serpent Staff) differ.
  - The staff takes no bullets, so skip AMMO. WEAPON becomes pistol ↔ staff.
  - To save time, reach W04 with G setup (`/gmgotoxyz`, `/gmmissionassign 638 1`), and flag the rows.

---

## 7. Gap list, by priority

| P | ID | Gap | Blocks | Fix |
|---|---|---|---|---|
| P0 | G-DEPLOY | The installed labd predates #1099-#1102 (no world, combat, UI or UAT tools) | everything | Rebuild and reinstall (§2) |
| P0 | G-MG1 | **No way to play Livewire** (Flash over SmartFox); no GM win shortcut either | T07, T10, T13 (the walkthrough stops at the cell door) | `server_minigame_state` + `client_minigame_click` (N1), or interim `.minigame_win` (G) |
| P0 | R1/R2/R3 | Resume recreates the character; rows don't stop on failure; the intro dialog is closed | one-character run | runner §1 |
| P0 | R10 | `@window_click_row` → a nonexistent tool; consumables I2 args wrong | rows wrongly BLOCKED | one-line fixes |
| P1 | G-NAV1 | No path planning; no Cellblock waypoints; the navmesh is patchy in the stasis room | every room change | `server_nav_path` (find_path corners), or a `lab_route_record` from one human run |
| P1 | G-TGT1 | Most objects have no client name; click and move take no `mob_id`; no tool-result capture (R4) | corpses, drone, loot | `mob_id` in TargetArg + R4; the `point` click is the interim |
| P1 | G-CMB1 / R7 | No fight loop or repeat-until; slap-pack threshold needs a ratio | ~16 fights | `client_fight` composite and/or R7 |
| P1 | G-SRV | Server clauses need a local lab-mcp (colo 403) and `${player_id}` (R5) | all mission-state asserts | local endpoint + R5 |
| P2 | R6 | No `delta`, `bit_set` or `any` ops | ammo and stack deltas, BSF bits, item presence | clause ops |
| P2 | G-TGT2 | `client_entity_find` has no dead/alive filter | LOOT by name | add a filter |
| P2 | G-DLG1 | No dialog, mission or sequence ids on the client (C5); exactly-once rests on SigNoz attestation | T01, T05, T09, T11, T16 | backlog C5, or S4 `server_journal_tail` |
| P2 | S2 | In-memory counters and fired-once chains not readable; `sgw_mission` lags | T14 counter, T15b | backlog S2 `server_mission_state` |
| P2 | G-WEAP1 | No client holster binding; holster is automatic only | holster-on-demand | assert the auto-holster; drop the manual one |
| P2 | G-AMMO1 | Ammo-picker widget names unknown | AMMO-04..11 type switches | read WeaponBar once |
| P3 | G-TIME | No relative timing between two events | T16/T17 | R6 or SigNoz |
| P3 | G-APP | No appearance (visible weapon or armor) reader | equips, T21 | human / screenshot |
| P3 | G-COVDBG | `cover.detection` is DEBUG; SigNoz export unverified | T08 cover proof | use `fire_cover_entered: matched` + `completed_objective_ids` |
| P3 | G-FLANK | T29 unreachable: no cover data in the Mess Hall or Hallway05 | T29 | content (seed a cover set) |
| P3 | G-2P | AMMO-02/03/18 need a second client | ammo risk rows | the second lab instance exists (#1090) |
| P3 | Doc drift | combat-system.md says holster is STUB; it is implemented | — | fix the doc |
| Note | UX | Using Ambernol before step 2343 is a silent no-op (the "button feedback on first press" rule) | T08/T09 | flag to the owner |
