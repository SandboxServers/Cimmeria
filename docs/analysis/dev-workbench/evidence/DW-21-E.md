# DW-21-E: dot progression, ability, stat, ammo and pet evidence

This is the source contract for DW-21, not a live-client acceptance result. Its 27 headings are the exact `Spec` name union in `crates/cell-console/src/cell/console/registry/commands/{progression,abilities,stats,ammo,pet}.rs`. The typed palette must preserve the leading dot. A disabled palette action remains available in the raw GM console; it is disabled here because the palette cannot promise a safe, verified outcome. No live lab was used.

## Common entry and readback rules

`crates/cell-console/src/cell/console/chat/mod.rs` and `dispatch.rs::{handle_console_command,resolve_target,exec}` enforce the server-side `access_level >= GameMaster` chat gate, exact command name, whitespace-token arity, a same-space/in-view selected entity when required, and an accepted-command audit row. The parser has no quoted-string grammar. `Target::Player` checks `is_player`; `Target::Being` currently matches **any** entity, without testing for a stat block. `Target::None` passes an in-view selection if there is one, otherwise `None`; each handler below decides whether to use it. The raw `gm*` ammo aliases are **dot commands**; their similarly named native slash commands have no `.def` method and do not reach these handlers (`docs/commands.md`, `gm/{give_ammo,set_infinite_ammo}.rs`). Revalidate identity and authority at execution; a palette preview cannot freeze selection. A Base feedback line after a DB commit proves the DB operation, not that the client's update arrived. A cell-side “queued” line does not prove a Base commit.

`server_entity_get {entity_id}` exposes `level`, `stats`, health/focus and state, but no cash, XP, inventory stacks, known abilities, crafting disciplines, blueprints, pet owner/stance, or infinite-ammo flag (`crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`). `server_ability_state {entity_id}` exposes cooldowns, pulsing effects, ledger entries and stats (`crates/lab-mcp/src/tools/abilities.rs::ability_state`); it is an immediate cell snapshot. `server_db_query` is read-only and row-capped (`crates/lab-mcp/src/tools/db.rs`); query exact `player_id` after a definitive Base result, and after relog when testing durability. `.effects`, `.cooldowns`, `.pet info`, `.pet list`, the stat commands and Base feedback are additional feedback surfaces, each with limits below. `server_entity_get`/`server_ability_state` may return no entity during transition: treat that as unknown, not a failed mutation. A snapshot of current state is not proof of persistence. `db/sgw/Players/Tables/{sgw_player,sgw_player_discipline_expertise,sgw_player_ability_grants}.sql` define the persistent fields.

Several queued grants carry a cell entity ID, not an immutable session generation. The same numeric entity ID may name character A when the cell queues a command and character B by the time Base processes or finishes it. A DB write keyed to A can succeed while a post-commit UI push or GM feedback addressed by that ID reaches B. A database delta for A does not prove delivery or grant attribution to the original live session. Each affected row states its specific boundary; the typed palette must fail closed until the message and post-await sends are fenced to immutable GM and subject owners.

## Progression

### .allcraft

- **Sources:** `registry/commands/progression.rs::SPECS`; `crafting.rs::{dispatch,all_craft}`; `crates/base-crafting/src/base/crafting/allcraft.rs::{handle_gm_all_craft,apply_all_craft}`; `options.rs::enable_craft_anywhere`; `allcraft_tests.rs`; `docs/gameplay/crafting-system.md`.
- **Arguments:** None.
- **Target/scope:** Required selected, in-view player; `player_id` from that entity. This does not fall back to the caller.
- **Preconditions and authority:** Cell GM gate and Base `caller_access >= GM_ACCESS_LEVEL` for the **current** holder of `gm_entity_id`; loaded catalog, paradigms, DB, target crafting row and transaction locks. `GmAllCraft` has no immutable GM/subject generation, and Base does not prove the current target holder still plays the saved `player_id`. Base refusal and DB-failure lines are distinct.
- **Effects and impact:** One transaction persists all catalog disciplines at expertise 100, all blueprints and racial paradigm levels at 7; then attempts a crafting UI push and `enable_craft_anywhere(entity_id, ...)` for a **current** session. The latter returns `bool`, but `handle_gm_all_craft` ignores it and still logs/sends `granted` after a failed session update or push. A reused entity ID can send A's crafting state to B or enable B's craft-anywhere. Persistent grant and transient privilege are separate outcomes.
- **Verification:** Compare exact A `sgw_player.discipline_ids,blueprint_ids,racial_paradigm_levels` and `sgw_player_discipline_expertise` by `player_id` with catalog IDs, then reread after relog. Base `allcraft granted` proves neither target-session craft-anywhere nor UI delivery: inspect `push_failed` and separately read the same owner session's craft-anywhere state/options. A reused GM ID can receive feedback intended for the original caller.
- **Compensation:** No proven inverse of the full prior crafting state; restore a captured DB/seed-backed state manually. Logout only clears craft-anywhere.
- **Availability:** Disabled until an immutable GM/subject owner fence survives the DB awaits and post-commit pushes, `enable_craft_anywhere` failure changes the result, and a complete before-state/destructive-grant preview exists; the raw GM command works.

### .learndiscipline

- **Sources:** `registry/commands/progression.rs::SPECS`; `crafting.rs::learn`; `crates/base-crafting/src/base/crafting/handlers.rs::{handle_grant_expertise,grant_expertise_in_db}`; `docs/gameplay/crafting-system.md`.
- **Arguments:** Positive `i32 disciplineId`; optional positive `i32 expertise` delta, default 1. Large positive values saturate then clamp to 100.
- **Target/scope:** Required selected player and its `player_id`.
- **Preconditions and authority:** Cell GM gate and positive values. Base handler receives `GmGrantExpertise {entity_id,player_id,...}` without a GM access check, catalog-discipline validation or target session-generation fence; DB/persistence failure drops the grant.
- **Effects and impact:** Registers the ID in `discipline_ids` if absent and persists expertise raised by delta, clamped 0–100, then pushes method 136 by `entity_id`. The cell's immediate `+N` feedback is only enqueue intent. Base `gmGiveExpertise: ... now N` is addressed to the **subject entity**, not the GM; after A's DB commit a reused ID can route both feedback and update to B.
- **Verification:** Read A's `sgw_player.discipline_ids` and `sgw_player_discipline_expertise` by exact `player_id,discipline_id` before/after and after relog. A cell `+N` line and a method 136 on current B are insufficient; require immutable owner evidence for the recipient. No arbitrary numeric ID until catalog validation is enforced.
- **Compensation:** `.forgetdiscipline` drives expertise to zero but keeps the discipline known; restoring exact prior expertise requires a recorded before value and a separate plan.
- **Availability:** Disabled: arbitrary positive IDs can enter persistent state, Base does not fence the target session after persistence, and feedback is neither an unambiguous GM receipt nor safe against entity-ID reuse.

### .forgetdiscipline

- **Sources:** `registry/commands/progression.rs::SPECS`; `crafting.rs::forget`; `crates/base-crafting/src/base/crafting/handlers.rs::grant_expertise_in_db`; `docs/gameplay/crafting-system.md`.
- **Arguments:** Positive `i32 disciplineId`.
- **Target/scope:** Required selected player.
- **Preconditions and authority:** Cell GM gate and parse/bound check; Base path lacks a second GM/catalog check or target session-generation fence. Missing DB prevents mutation.
- **Effects and impact:** Sends delta `-100`; Base clamps expertise to zero, registers an absent ID as known and persists it. **It does not delete the discipline row or remove the known ID.** Cell `-> expertise 0` feedback precedes persistence; Base result goes to the subject entity.
- **Verification:** Compare A's exact `sgw_player.discipline_ids` and `sgw_player_discipline_expertise` row before/after and after relog. Base may push A's zero to reused entity B; its feedback or update alone does not prove the live recipient. Zero expertise is not proof the discipline was forgotten.
- **Compensation:** Add the captured previous expertise through the grant path if possible; an initially absent discipline requires manual removal of both known ID and row. No automatic inverse.
- **Availability:** Disabled: the name implies removal while implementation preserves or creates a known discipline; Base also lacks a post-commit target-owner fence and caller-bound receipt.

### .craftkit

- **Sources:** `registry/commands/progression.rs::SPECS`; `crafting.rs::craft_kit`; `crates/base-crafting/src/base/crafting/gm_grant/{mod.rs,craftkit.rs}::{handle_gm_craft_grant,handle_craftkit,kit_grants}`; `gm_grant/tests/craftkit.rs`.
- **Arguments:** `i32 blueprintId`; optional `i32 count` default 1, Base validates `1..=10`; component set 1 only.
- **Target/scope:** Required selected player and its carried eligible bags; item placement follows component `container_sets`.
- **Preconditions and authority:** Cell GM gate; Base checks GM access, catalog blueprint/set, count, DB and bag capacity. Missing components, full bag or failed transaction refuse before any kit grant.
- **Effects and impact:** Atomic crafting inventory transaction grants `count` times each component's quantity, sends inventory update and inventory events. This is a persistent item grant, not a blueprint unlock; success line lists applied stacks.
- **Verification:** Wait for Base `craftkit [...] items granted` line; compare `sgw_inventory` rows by target `character_id`, including `item_id,type_id,stack_size,container_id,slot_id`, before/after and after relog. The line and inventory update alone do not prove client receipt. `server_entity_get` has no inventory list.
- **Compensation:** No proven inverse: consuming/removing the granted stacks risks removing preexisting merged rounds/components. Manual reconciliation from captured instance/quantity diff only.
- **Availability:** Disabled until immutable GM and subject owners are fenced across Base catalog/DB awaits and post-commit inventory/feedback sends, and the palette has exact inventory readback plus a multi-item preview. A current GM holding a reused `gm_entity_id` can authorize an old message; a reused target ID can receive A's inventory update.

### .learnblueprint

- **Sources:** `registry/commands/progression.rs::SPECS`; `crafting.rs::learn_blueprint`; `crates/base-crafting/src/base/crafting/gm_grant/{mod.rs,learn_blueprint.rs}::{handle_gm_craft_grant,handle_learn_blueprint,learn}`; `gm_grant/tests/learn_blueprint.rs`.
- **Arguments:** One parseable `i32 blueprintId`; Base catalog validates existence.
- **Target/scope:** Required selected player.
- **Preconditions and authority:** Cell GM gate and Base `caller_is_gm` check on the **current holder** of `gm_entity_id`; the queued `GmCraftGrant` carries no GM or subject session generation. Unknown/already-known ID or failed DB write refuses, but an A→B reuse can make B's access level authorize A's old request.
- **Effects and impact:** Under player-wide lock, persists sorted `sgw_player.blueprint_ids` and pushes full known-crafts list (client method 139); no component items are granted.
- **Verification:** Query A's `sgw_player.blueprint_ids` by exact `player_id` before/after and after relog. `handle_learn_blueprint` never proves `ids.entity_id` still plays `ids.player_id` before or after its DB awaits; `push_known_crafts` and `gm_line` address numeric IDs that may now belong to B. A success line or B's client list is not owner-bound proof.
- **Compensation:** No console unlearn operation; manual restore of captured list is required.
- **Availability:** Disabled until the queued grant carries immutable GM and subject session owners, Base fences both before the write and after awaits before sends, and an A→B ID-reuse replay test proves no cross-session authority or delivery. Catalog-backed ID and DB readback alone are insufficient.

### .missionfail

- **Sources:** `registry/commands/progression.rs::SPECS`; `mission.rs::fail`; `crates/cell-content/src/cell/missions/state_lock.rs::refuse_if_state_unknown`; `tests/{locked_missionfail.rs,hidden_missionfail.rs}`; `docs/gameplay/mission-system.md`.
- **Arguments:** One parseable `i32 missionId` (registry help calls it design ID); no positive bound in handler.
- **Target/scope:** Required selected player and that player's active mission record.
- **Preconditions and authority:** Cell GM gate; mission tracker must be loaded and mission status `MISSION_ACTIVE`. Unknown/loading state or no active mission gets a refusal without mutation.
- **Effects and impact:** Mutates cell mission to `FAILED` and increments repeats through `m.fail()`, sends visible mission update unless hidden, emits gameplay event and immediate GM feedback. **This handler sends no `CellToBaseMsg::MissionUpdate`**, so it does not persist the forced failure; later unrelated mission writes could affect the saved row, but are not this command's success path.
- **Verification:** Observe `missionfail [...] -> FAILED` and the target's immediate cell/client state. Compare exact `sgw_mission` row by `player_id,mission_id` before/after; without a `MissionUpdate` this command has no durable completion receipt. Relog may restore the prior saved status; do not claim persistence from the chat line or frame.
- **Compensation:** No proven inverse that restores repeats, step/objectives and content events.
- **Availability:** Disabled until exact mission before/after and persist/relog verification can be bound; irreversible forced failure needs preview.

### .missionrewards

- **Sources:** `registry/commands/progression.rs::SPECS`; `mission.rs::rewards`; `docs/architecture/dev-console-channel.md`.
- **Arguments:** One parseable `i32 missionId`.
- **Target/scope:** Required selected player's cell mission tracker.
- **Preconditions and authority:** Cell GM gate; target and integer. No catalog/DB lookup **or mission-state lock check** is made. `crates/cell-content/src/cell/missions/state_lock.rs::is_state_unknown` can be true while the saved tracker is loading or unavailable.
- **Effects and impact:** Read-only chat line of current status and step, or “no mission”; **it does not display the reward set**, despite its name/help. An empty locked tracker can produce a false “no mission” line.
- **Verification:** The returned `status`/`step` is only a cell snapshot, not reward data. An absent line must be treated as **unknown** until the mission state is proven loaded and an exact `sgw_mission` read by `player_id,mission_id` agrees; no relog wait is needed for a loaded-state read.
- **Compensation:** None needed.
- **Availability:** Disabled until the handler checks loaded mission state before returning absence and labels unknown/unavailable distinctly; a later reward catalog reader is required for the command's named purpose.

### .givecash

- **Sources:** `registry/commands/progression.rs::SPECS`; `give.rs::give_cash`; `crates/base-methods/src/base/world_entry/methods/progression/mod.rs::handle_grant_cash`; `tests/p05.rs`.
- **Arguments:** Positive signed `i32 amount` (max `i32::MAX`).
- **Target/scope:** Required selected player; cash belongs to target `sgw_player.naquadah`, feedback to caller.
- **Preconditions and authority:** Cell GM gate and positive amount/player ID; Base sink requires an entity address and DB row but does not recheck the recipient's player identity or GM rights from the message. No DB or failed update yields no committed grant.
- **Effects and impact:** Persistent additive currency grant keyed to A's `player_id`; after the SQL `UPDATE`, Base sends target property by numeric `entity_id` and GM feedback by numeric caller ID. If either ID has been reused, B can receive A's total or success line. No automatic cap or undo of a later spend.
- **Verification:** Capture exact A `sgw_player.naquadah` by `player_id`, compare after the SQL write and after relog. Base feedback confirms a row update but is not fenced to the original GM; a property received by current B is not proof of A's client delivery. Do not infer success from Cell dispatch alone.
- **Compensation:** No safe inverse after spending or concurrent grants; manual audited balance reconciliation from captured state.
- **Availability:** Disabled until immutable GM/subject owner fencing covers the queued request and post-commit sends, receipt binds to A's exact DB delta, and the palette previews currency impact; raw GM path works.

### .givexp

- **Sources:** `registry/commands/progression.rs::SPECS`; `give.rs::give_xp`; `crates/base-methods/src/base/world_entry/methods/progression/{mod.rs,asp_earning.rs}::{handle_grant_xp,persist_grant}`; `docs/gameplay/crafting-system.md`.
- **Arguments:** Positive signed `i32 amount`, forwarded as `u64` only after positive check.
- **Target/scope:** Required selected player.
- **Preconditions and authority:** Cell GM gate; `GrantXP` carries `entity_id,xp_amount,gm_feedback_to` but no target `player_id` or session generation. Base reads the **current** `active_player_id` for that entity; after A queues a grant, reuse can credit B. No DB drops grant, and no Base GM-level recheck is made.
- **Effects and impact:** Persistent XP and possibly level, training-point **and Applied Science Point** increases in one row-locked SQL statement. `asp_earning::persist_grant` adds `APPLIED_SCIENCE_POINTS_PER_LEVEL` for each level gained relative to the locked DB row, saturating the ASP total at `i32::MAX`; no level gain earns no ASP. The handler may unlock ability-tree state and sends XP/level updates plus the ASP total when earned. The request amount is not necessarily the final XP delta at cap.
- **Verification:** Compare the intended A and actual current B `sgw_player.exp,level,training_points,applied_science_points` by their distinct `player_id`s before/after and after relog; correlate `crafting.asp_earned` with the credited ID. Base can also overwrite B's in-memory XP/level/points after its DB await because it updates the saved address without a post-await owner fence. A success line or `server_entity_get.level` on B cannot attribute the grant to A.
- **Compensation:** No safe inverse of level, spent training/Applied Science Points, unlocked actions or subsequent crafting spends; if an A→B reuse occurred, audit **both** characters' DB and B's in-memory state before manual repair.
- **Availability:** Disabled until `GrantXP` carries immutable target player/session generation, Base fences ownership before persistence and after awaits before in-memory/wire updates, and an A→B replay test plus multi-field readback prove the intended recipient.

### .giveability

- **Sources:** `registry/commands/progression.rs::SPECS`; `give_ability.rs::{give_ability,plan_grant,send_grant}`; `crates/base-methods/src/base/world_entry/methods/progression/grant_ability.rs::{handle_gm_grant_ability,persist_ability_grant}`; `tests/pt07_giveability.rs`.
- **Arguments:** One parseable signed `i32 abilityId`; exact `resources.abilities` catalog membership, not help text, is checked by cell.
- **Target/scope:** Selected in-view player else caller; a selected non-player falls back to caller with a warning line. Palette must show the resolved character before send.
- **Preconditions and authority:** Cell GM gate, known ability and not already known, caller/subject player IDs; Base checks that the subject's **current** entity holder plays `player_id` before persistence, but carries no immutable session generation and does **not** check the original GM owner, live GM access level or session before writing. Its `gm_player_id` check only guards later feedback. A queued command may commit after the original GM session ends; a same-character relog passes the numeric character-ID checks. Duplicate DB grant is refused.
- **Effects and impact:** Guarded DB transaction adds to `sgw_player.abilities` and a `source_kind='gm'` provenance row, not `trained_abilities`; no training points spent. Respec keeps the ability; `/gmresetabilities` removes GM grants. After the DB await Base sends `GmAbilityGranted {entity_id,player_id,ability_id}` without rechecking subject session generation; a new holder or same-character relog can receive a stale mirror/refresh. Feedback is likewise gated by current `gm_player_id`, not the original GM session generation.
- **Verification:** Compare exact `sgw_player.abilities` and `sgw_player_ability_grants` by `player_id,ability_id,source_kind` before/after Base persistence and after relog. A cell `forwarded` event has `persisted=false`; a DB delta proves the character row changed, not that the original GM remained authorized or the original subject client received the mirror. A “saved to the character” line or client ability-window refresh after same-character relog is not generation-bound proof.
- **Compensation:** `/gmresetabilities` removes **all** GM-grant provenance, not just this ability; no single-grant inverse proven. Manual restore from captured grants may be required.
- **Availability:** Disabled until the queued request carries immutable GM and subject owners, Base checks GM authority and both owners before the write and fences mirror/feedback after awaits, and A→B plus same-character relog replay tests prove stale grants/refreshes are refused. Keep exact catalog ID, resolved-subject display and DB/provenance readback in the eventual form.

## Abilities

### .effects

- **Sources:** `registry/commands/abilities.rs::SPECS`; `abilities/{mod.rs,effects.rs}::{dispatch,show}`; `crates/lab-mcp/src/tools/abilities.rs::ability_state`.
- **Arguments:** None.
- **Target/scope:** In-view selected entity else caller; any entity kind, then snapshot may be absent.
- **Preconditions and authority:** Cell GM gate; missing ability state gets an explicit “no ability state” line.
- **Effects and impact:** Read-only concise current warmup, cooldowns, pulses, ledger, flags and stats; no DB mutation.
- **Verification:** Compare line to `server_ability_state {entity_id}` snapshot at the same time; transient timers advance, so exact seconds can differ. No persistence claim or relog wait.
- **Compensation:** None needed.
- **Availability:** Runnable, labelled live cell snapshot.

### .cooldowns

- **Sources:** `registry/commands/abilities.rs::SPECS`; `abilities/cooldowns.rs::{run,list,reset_all,reset_one}`; `crates/cell-combat/src/cell/abilities` cooldown helpers; `docs/commands.md`.
- **Arguments:** No args lists; `reset` clears all; `reset <positive i32 abilityId>` clears one plus its moniker groups. Case-sensitive `reset`; other tokens receive usage. Registry allows up to two words.
- **Target/scope:** **Caller only**, ignoring selection; reset-one sends a zero-length client timer even when server had none.
- **Preconditions and authority:** Cell GM gate, caller ability state; positive ID for one-ability reset. No catalog existence check on reset ID.
- **Effects and impact:** Listing is read-only. Reset mutates caller's transient cooldown maps and sends client timer clears; it does not restore resources or undo a cast.
- **Verification:** `server_ability_state {caller_id}.cooldowns` before/after and `.cooldowns` feedback; hotbar sweep needs separate client observation. No DB/relog proof; cooldowns naturally expire.
- **Compensation:** No exact inverse of timer history; future casts restart cooldowns.
- **Availability:** Runnable with distinct List, Reset all and Reset one forms; Reset all needs a transient-impact confirmation.

### .dummy

- **Sources:** `registry/commands/abilities.rs::SPECS`; `abilities/{dummy.rs,dummy_caster.rs}::{run,spawn_dummy,clear,check_ability}`; `tests/ab_l2/{dummy.rs,dummy_caster.rs}`; `docs/commands.md`.
- **Arguments:** `[]`, `hostile`, `friendly`, `clear`, optional positive `i32 templateId`; `caster <positive i32 abilityId> [intervalSecs]` with interval default 8, range 1–290 and ability warmup/cooldown bounds. Registry permits 0–3 words; unknown form gets usage. No quote syntax.
- **Target/scope:** Caller-owned dummy in caller's space; selection is ignored. `clear` removes only that caller's dummies; each caller has cap 4.
- **Preconditions and authority:** Cell GM gate, player in a space, spawnable template; caster ability must exist in the ability catalog, be active and hostile, reach 3 m and fit the interval. The handler grants it to the new dummy after spawning. Invalid template/cap/ability refuses without a spawn.
- **Effects and impact:** Transient 1,000,000-health NPC 3 m ahead, 10-minute lifetime or logout/clear removal; `friendly` changes faction to 9. `caster` periodically launches the real hostile ability at caller, with combat/effect risk. No persistent spawn row.
- **Verification:** Capture returned dummy entity ID; `server_entity_get` by ID verifies template/space/health/faction, then after clear/expiry confirms absence while guarding against ID reuse. Caster behavior needs combat/ability telemetry; placement alone cannot prove casts. Relog should remove owned dummy.
- **Compensation:** `.dummy clear` removes caller's dummies, but cannot undo damage/effects already caused by caster; `.cleareffects` is only partial cleanup.
- **Availability:** Runnable for static hostile/friendly/clear with exact returned ID and transient label. Caster disabled in palette until target harm and cast readback are explicit.

### .cleareffects

- **Sources:** `registry/commands/abilities.rs::SPECS`; `abilities/clear_effects.rs::{run,strip_pulses,flush_stats}`; `crates/lab-mcp/src/tools/abilities.rs::ability_state`.
- **Arguments:** None; selected entity is not a text argument.
- **Target/scope:** In-view selected entity else caller; affects timed ledger and pulsing effects of that entity, not cooldown/warmup.
- **Preconditions and authority:** Cell GM gate and existing subject. Gone entity refuses with feedback.
- **Effects and impact:** Removes ledger entries with `Cleansed`, runs pulse `on_remove`, restores moved stats/absorb/state refs and sends icon/stat clears. This can interrupt beneficial or hostile effects; transient but gameplay-significant.
- **Verification:** Compare `server_ability_state` ledger/pulses/stats for exact subject before/after; GM line lists removed IDs. Client icons need separate client observation. No DB/relog wait.
- **Compensation:** No safe inverse of timed effects, remaining durations or scripts; recast manually if appropriate.
- **Availability:** Runnable only with selected-subject confirmation and before/after effect snapshot; emphasize cleanse-all impact.

## Stats

All seven read commands below use `registry/commands/stats.rs::SPECS`, `stats.rs::{show,stat_set,format_stat_line}` and a required selected same-space/in-view `Target::Being`. Each accepts **zero** arguments; `Target::Being` does not itself prove the entity has stats. Each reads current cell `StatList`, emits `label: cur/max` or `n/a`, writes no DB, has no compensation and needs no relog. `server_entity_get {entity_id}.stats` is the independent immediate readback; an empty `server_entity_query` stat list is not equivalent because that route intentionally omits full stats.

### .stats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("stats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; health, focus and their regeneration stats.
- **Preconditions and authority:** Common GM/selection gate; missing stat prints `n/a`.
- **Effects and impact:** Read-only four current/max pairs.
- **Verification:** Match IDs/values in `server_entity_get.stats`; health/focus summary fields corroborate only two pairs.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .primarystats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("primarystats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; coordination, engagement, fortitude, morale, perception, intelligence.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only six current/max pairs.
- **Verification:** Match exact stat IDs in `server_entity_get.stats`; `n/a` means missing, not zero.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .speedstats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("speedstats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; movement/rotation modifiers and reload/grenade/deploy/attack speed stats.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only six current/max pairs; does not invoke `.speed`.
- **Verification:** Match exact IDs in `server_entity_get.stats`; movement and rotation are two of six.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .armorstats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("armorstats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; four armor and four resistance stats.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only eight current/max pairs.
- **Verification:** Match exact IDs in `server_entity_get.stats`.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .qrstats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("qrstats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; 20 QR combat stats from accuracy through subtlety as enumerated by `stat_set`.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only current/max pairs; this does not force a QR result.
- **Verification:** Match each reported stat ID in `server_entity_get.stats`.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .absorbstats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("absorbstats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; typed and item/energy absorb stats.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only 15 current/max pairs; does not clear shield pools.
- **Verification:** Match exact IDs in `server_entity_get.stats`; `server_ability_state` separately shows active absorb ledger.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .stealthstats

- **Sources:** Shared stat sources above; `stats.rs::stat_set("stealthstats")`.
- **Arguments:** None.
- **Target/scope:** Selected entity; stealth rating/movement, reveal, disguise rating/detection.
- **Preconditions and authority:** Common GM/selection gate.
- **Effects and impact:** Read-only five current/max pairs; does not reveal/disguise an entity.
- **Verification:** Match exact IDs in `server_entity_get.stats`.
- **Compensation:** None needed.
- **Availability:** Runnable as live stat read.

### .speed

- **Sources:** `registry/commands/stats.rs::SPECS`; `stats.rs::{set_speed,SPEED_STATS}`; `tests/p47.rs`; `docs/architecture/python-console.md` legacy comparison.
- **Arguments:** One signed `i32 speed`; handler bounds against **both** live `movementSpeedMod` and `rotationSpeedMod` min/max (100 normal), rejecting rather than clamping.
- **Target/scope:** Required selected same-space/in-view entity. Player client and NPC witnesses receive dirty stat update.
- **Preconditions and authority:** Cell GM gate, entity and both stats present, value in both ranges. Bad input touches neither stat.
- **Effects and impact:** Sets current (not max) of two cell stats, publishes method `ON_STAT_UPDATE`; movement/rotation changes immediately and may be overwritten by later state calculation. No DB write in handler.
- **Verification:** Compare both exact stat IDs' `cur/max` in `server_entity_get.stats` before/after; feedback `Set speed ...` follows mutation but is not persistence. Sample again after movement or relog; no durable speed claim.
- **Compensation:** Set both to captured prior current values only if they were equal; if they differed, `.speed` cannot restore each separately. No automatic inverse.
- **Availability:** Runnable only with live bounds and two-stat before/after, labelled transient. Disabled when prior values differ and a full restore is required.

## Ammo

### .giveammo

- **Sources:** `registry/commands/ammo.rs::SPECS`; `gm/give_ammo.rs::{give_ammo,parse_ammo_type,MAX_ROUNDS}`; `crates/base-methods/src/base/world_entry/methods/inventory/{ammo_gm_give,ammo_reserve}/mod.rs::{handle_gm_give_ammo,grant_rounds,return_rounds}`; `tests/am06_ammo.rs`; `docs/gameplay/weapon-ammo-reload.md`.
- **Arguments:** Two whitespace words: catalog ammo type number/name/alias or reserve item ID, then positive `i32 rounds`; request over 5000 is **clamped** to 5000 with preflight note. Ambiguous `emp`, unknown/default type, no reserve item and nonpositive rounds refuse.
- **Target/scope:** Selected in-view player else caller; selected non-player falls back to caller with warning. Stacks go to target bags.
- **Preconditions and authority:** Cell GM gate, catalog mapping and player IDs; Base confirms the active subject character **before** its DB transaction, then commits inventory. It does not recheck target/GM session generation after awaits or GM access level. Bags may accept only part; zero accepted refuses.
- **Effects and impact:** Persistent special-ammo reserve items in existing/new stacks, possibly partial because bags fill. Base derives the actual reserve item again from `resources.ammo_item_types` inside `return_rounds`; that `out.item_id` can differ from the cell-captured `msg.item_id` if catalog data changed. Base then sends inventory update by numeric target ID after commit, so reused B can receive A's update. GM feedback is checked against `gm_player_id` at send time, but not generation: a new session of the same GM character can receive the old line. No clip fill.
- **Verification:** Wait for Base grant/partial/refusal result and `returned,remainder,stack_before,stack_after` telemetry. Read `server_db_query` rows from `sgw_inventory` for target `character_id`, **only carried reserve bags `container_id IN (1,15)`**, keyed by the actual `resources.ammo_item_types.item_id` for `ammo_type`; require `truncated=false`, sum `stack_size` and compare exact `item_id,type_id,stack_size,container_id,slot_id` row diffs before/after and after relog. Verify actual DB item type equals the Cell-captured `msg.item_id` before enabling. The success line prints `msg.item_id`, not `out.item_id`; `server_entity_get` exposes no bags. Distinguish clamped request from bag remainder and B's UI receipt from A's DB grant.
- **Compensation:** No safe inverse after stacks merge, ammo is used or another grant races; manual audited inventory reconciliation only.
- **Availability:** Disabled until immutable GM/subject owner fencing covers pre/post-DB awaits and sends, actual DB item type matches the Cell-captured type, and bounded carried-bag readback plus partial-grant semantics are shown in the palette.

### .gmgiveammo

- **Sources:** `registry/commands/ammo.rs::SPECS`; `dispatch.rs::exec`; `gm/give_ammo.rs::give_ammo`; same Base handler as `.giveammo`.
- **Arguments:** Same two typed words, bounds, clamping and refusal as `.giveammo`.
- **Target/scope:** Same selected-player-or-caller resolution.
- **Preconditions and authority:** Same GM and Base checks. This **dot alias** does not make native `/gmgiveammo` callable.
- **Effects and impact:** Same persistent, possibly partial reserve-item grant.
- **Verification:** Same carried-bags-only, `truncated=false`, actual-DB-item-identity, exact row-diff and relog recipe as `.giveammo`; label result as dot alias.
- **Compensation:** Same manual-only inventory reconciliation.
- **Availability:** Disabled with `.giveammo` until owner fencing, DB item identity and exact carried-bag readback exist; show as an alias, not a separate action.

### .infiniteammo

- **Sources:** `registry/commands/ammo.rs::SPECS`; `gm/set_infinite_ammo.rs::set_infinite_ammo`; `crates/entity/src/ammo_infinite.rs::{set,is_on}`; `docs/gameplay/weapon-ammo-reload.md`.
- **Arguments:** None reads; optional flag parsed by `ammo_feature::parse_flag` (`on`/`off` documented, parser also defines accepted synonyms). Invalid flag refuses with “nothing changed.”
- **Target/scope:** Selected in-view player else caller; keyed by subject `player_id`, across zones/relogs until server restart.
- **Preconditions and authority:** Cell GM gate and subject player ID; no DB. If finite-special feature is off, feedback says reloads are free regardless of switch.
- **Effects and impact:** No-arg read-only. Flag form mutates process-global per-character switch: special-ammo reserve is not consumed on reload, while clip still empties. This is not a permanent DB grant.
- **Verification:** Immediate feedback includes ON/OFF and subject; issue a no-arg read with same selection to recheck. No existing lab snapshot or DB column exposes this switch; a relog check can confirm process lifetime, but restart clears it. Reload behavior requires separate controlled observation.
- **Compensation:** Set the captured previous ON/OFF value; restart also clears but affects every character's switches.
- **Availability:** Runnable for no-arg read. Disable toggle in one-click palette until selected-subject pinning/readback and server finite-ammo feature state are surfaced.

### .gmsetinfiniteammo

- **Sources:** `registry/commands/ammo.rs::SPECS`; `dispatch.rs::exec`; `gm/set_infinite_ammo.rs::set_infinite_ammo`; `crates/entity/src/ammo_infinite.rs`.
- **Arguments:** Same optional flag as `.infiniteammo`.
- **Target/scope:** Same selected-player-or-caller and per-character process switch.
- **Preconditions and authority:** Same GM gate/player-ID check. This **dot alias** is independent of nonworking native `/gmsetinfiniteammo`.
- **Effects and impact:** Same read or transient toggle, no DB persistence.
- **Verification:** Same immediate ON/OFF feedback plus repeated no-arg read; no direct lab snapshot.
- **Compensation:** Restore captured prior flag with the same command.
- **Availability:** Read-only form runnable; toggle disabled with `.infiniteammo`, display as alias only.

## Pet

### .pet

- **Sources:** `registry/commands/pet.rs::SPECS`; `pet.rs::{dispatch,summon,dismiss,stance,info,list,refuse}`; `crates/cell-world/src/cell/pets` lifecycle; `tests/pt07_pet.rs`; `docs/gameplay/pet-system.md`.
- **Arguments:** Required case-sensitive verb and at most one second word per registry: `summon <i32 templateId|abilityId>`, `dismiss`, `stance <0|1|2>`, `info`, `list`. Summon does not explicitly require a positive ID; catalog lookup rejects unknown values. Dispatcher ignores surplus second word for several verbs, so typed forms must enforce verb-specific arity. Summon ID resolves as ability when `pet_summons` has a row, otherwise template ID.
- **Target/scope:** Caller owns summon/dismiss/stance; `info` uses selected pet else caller's pet; `list` reads all pets in caller's space. Other owners' pets may be inspected, not modified.
- **Preconditions and authority:** Cell GM gate; summon requires player/alive owner and a cached spawn template (even a non-`pet` template is accepted and noted in feedback); stance requires an owned live pet and valid enum. Each refusal emits `pets.command` reason and GM feedback; invalid ID/owner or gone pet must not be treated as success.
- **Effects and impact:** Summon immediately spawns/replaces one per-session pet without normal ability warmup; dismiss removes all caller-owned pets; stance mutates pet AI and queues client stance update. `info`/`list` are read-only. No pet persistence across logout/relog.
- **Verification:** For summon use returned `pet_id`, `server_entity_get {entity_id:pet_id}` for template/space and `.pet info` for owner/stance; dismissal checks the captured IDs absent with ID-reuse guard. Stance is read back by `.pet info` on the exact pet; `server_entity_get` has no owner/stance fields. `list` feedback is space-scoped and may race lifecycle. Client stance update is separate delivery evidence.
- **Compensation:** Dismiss or replace a summon, but neither restores a replaced pet's exact health/state/cooldowns. Reapply prior stance if the same pet still exists. No inverse for effects already caused by pet combat.
- **Availability:** Runnable for `info`/`list` with snapshot scope shown. Mutating forms disabled until exact owner/pet identity and replacement impact can be confirmed before send and verified after; no persistence claim.

## Source-check samples and boundaries

- **Grant:** `.giveability` reaches `give_ability.rs::plan_grant` (catalog/duplicate/subject checks), then `grant_ability.rs::persist_ability_grant` (guarded append and `gm` provenance in one transaction). The cell's `forwarded` log explicitly says `persisted=false`; only the Base result and exact DB rows support a saved grant.
- **Reset:** `.cooldowns reset <abilityId>` in `abilities/cooldowns.rs::reset_one` clears the caller's ability and moniker cooldowns and sends a zero timer even when the server had no timer. `.forgetdiscipline` is instead a persistent expertise-zero operation that leaves the discipline known.
- **Refusal:** `gm_grant/craftkit.rs::kit_grants` rejects count outside 1–10 or missing component set before the transaction. `gm/give_ammo.rs::give_ammo` rejects default ammo and bad quantity before forwarding. Neither preflight refusal grants items.
- **Authority gap:** `GmGrantExpertise`, `GrantCash`, `GrantXP`, `GmGrantAbility` and `GmGiveAmmo` rely on the dot-channel GM gate and message origin rather than all rechecking the GM access level at Base. `GmAllCraft` and `GmCraftGrant` do recheck. The palette must use the authorized console route; it must not call these Base messages directly. A typed form does not strengthen a weak downstream handler.
- **Client proof:** None of this source audit proves stock client feedback, inventory UI, hotbar, pet bar or relog behavior. Those need separately authorized live-client UAT. The missing client prevents such a claim here.
