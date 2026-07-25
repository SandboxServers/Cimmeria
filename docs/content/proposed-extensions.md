---
title: "Proposed content engine extensions"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Proposed content engine extensions

> **Last updated**: 2026-07-25
> **Audience**: Tech lead deciding what mission/content features to invest in next. Engineers picking up a chunk of the engine work.
> **Diátaxis type**: Explanation. Roadmap with rationale, not a how-to or reference.

This document is the **justified roadmap** for engine extensions. It draws from:

- The recent direction on `main` (last 7 days of content-engine commits — counters, multi-trigger, `ChangeStat`, `StatBelowMax`, instance_id plumbing).
- Concrete missions in the SGW data set that the current engine can't express.
- Capabilities from the legacy SGW server (see [serverEd-comparison.md](serverEd-comparison.md)) that map cleanly to data.

Every entry below has a **why** — either it's tied to content that exists in the database, or it's wiring a variant that's already defined but inert. Speculative additions that would be nice in some hypothetical future are out.

The recent direction is informative: each addition has shipped with seed data, executor wiring, and replay tests in the same PR. The list below preserves that discipline — each entry says what would have to ship together.

---

## Tier 1 — wire what's already defined

These variants exist in the `Action` / `Condition` enum and are accepted by the loader, but [executor/mod.rs](../../crates/services/src/cell/content/executor/mod.rs) has no match arm — they fall through to `debug!("Unhandled")` no-ops. Wiring them is mostly executor work, not engine work.

**Four of these are not roadmap items but live bugs**: `move_entity` (5 seeded rows), `launch_ability` (3), `qr_combat_damage` (2), and `fail_objective` (1) are already authored into the shipped seed and silently do nothing every time they resolve. See §1.4.

### 1.1 `Action::ApplyEffect` / `Action::RemoveEffect`

| | |
|---|---|
| Status today | Loaded at [loader/action.rs:309-318](../../crates/content-engine/src/loader/action.rs#L309-L318). No executor arm. One `apply_effect` and one `remove_effect` row already exist in the seed and no-op today. |
| Effort | Small (S) — bridge into `crate::cell::effects` (verify path) and emit `CellToBaseMsg::ApplyEffect` / `RemoveEffect`. |
| Unlocks | Timed buff/debuff consumables (regen pots, food buffs, focus drinks). HoT/DoT effect chains. The full effect-system surface that `OnEffectInit`/`PulseBegin/End`/`Removed` triggers were built for. |
| Why | The action is already in the loader, the trigger family is already in the engine, and effect application is a per-domain Rust path. The executor arm is the only missing piece. Three of the next likely consumable categories (regen / shield / temp-stat-boost) all need this. |

### 1.2 `Action::StartTimer` / `Action::CancelTimer` + dispatcher for `Trigger::OnTimer`

| | |
|---|---|
| Status today | Action variants and trigger variant defined. No loader arm either — `start_timer` / `cancel_timer` / an `OnTimer` `event_type` cannot be named from seed. No executor arm. No `fire_timer_*` site in [event_dispatch/](../../crates/services/src/cell/content/event_dispatch/). |
| Effort | Medium (M) — needs a per-cell tick loop or per-entity `tokio::time::sleep` task, plus persistence story (timers across logout?). |
| Unlocks | Timed objectives ("defuse in 30s"). Wave-spawn delays. Daily-reset scaffolding (paired with §3.2). Escort-fail-on-pause patterns. Timed buff cleanup if §1.1 doesn't already drive it. |
| Why | Several SGW missions in the bomb-defusal/escape-sequence pattern need this. There's no good Rust-side substitute that an authored chain could call into without reinventing the dispatcher. |

### 1.3 `Action::GrantXP`

| | |
|---|---|
| Status today | Variant defined ([actions.rs:23](../../crates/content-engine/src/actions.rs#L23)). No loader arm. No executor arm. **No mission in the seed data uses XP rewards** — every `reward_xp` field is 0. |
| Effort | Small (S) — once the XP/leveling system lands. |
| Unlocks | Mission XP rewards. Currently every chain that completes a mission fires `CompleteMission` followed by `GrantItem` rewards — `GrantXP` is not authored anywhere. |
| Why | Already on the roadmap via [.claude/plans/2026-03-08-xp-leveling-design.md](../../.claude/plans/2026-03-08-xp-leveling-design.md). The chain-side wiring should land alongside the leveling system. |

### 1.4 Seeded-but-inert actions — `move_entity`, `launch_ability`, `qr_combat_damage`, `fail_objective`

| | |
|---|---|
| Status today | All four have loader arms ([loader/action.rs](../../crates/content-engine/src/loader/action.rs)) and are **used by shipped seed data** — 5 / 3 / 2 / 1 rows respectively. None has an executor arm, so all 11 rows resolve, emit a `debug!`, and do nothing. |
| Effort | Small (S) each. `MoveEntity` can reuse the `MoveWaypoint` handler's position-write path; `LaunchAbility` calls into `crate::cell::abilities`; `QrCombatDamage` calls the existing damage-apply path; `FailObjective` mirrors the `CompleteObjective` handler in [executor/mission.rs](../../crates/services/src/cell/content/executor/mission.rs). |
| Unlocks | Nothing new — it makes already-authored content work. Scripted NPC repositioning, scripted ability fires, scripted damage, and objective-fail branches are all currently silent no-ops in Castle_CellBlock and SGC_W1. |
| Why | This is the highest-value Tier 1 entry because the content authoring is already done and merged. Unlike §1.1–§1.3 it needs no new seed data, no new design, and no dependency on an unshipped system. Each one should ship with a chain-replay guard asserting the action reaches its handler. |

---

## Tier 2 — small additions tied to recent direction

### 2.1 Item-use cooldowns

| | |
|---|---|
| Need | The Health Slappack chain (4001) has no cooldown. With a vendor that sells stacks of 99, the player can chain-heal from 1 HP to full instantly. Every MMO has potion cooldowns. |
| Engine surface | New table `sgw_item_cooldowns (player_id, item_id, ready_at timestamptz)`. New `Condition::ItemReady { item_id }` reading from a populator. New `Action::SetItemCooldown { item_id, duration_secs }`. |
| Effort | Small-Medium (S/M) — schema + populator + condition + action + executor. ~1-2 days. |
| Unlocks | Every consumable category. Generalizes to ability cooldowns if those ever flow through chains (they shouldn't — see §6 of [content-engine.md](content-engine.md) and the §7 "what NOT to add" discussion below). |
| Justified by | The direction PR #220 took. Without this, the Slappack is overpowered the moment a vendor sells it. |

### 2.2 Persistent counters

| | |
|---|---|
| Need | Counters today are in-memory `CellEntity.counters` — lost on logout ([cell_entity/mod.rs:287-290](../../crates/entity/src/cell_entity/mod.rs#L287-L290)). The Mess Hall mission (kill 2 guards) works because it's bounded by one session. Cross-session quotas ("kill 50 Jaffa across the whole game") cannot use this primitive. |
| Engine surface | New table `sgw_player_counters (player_id, counter_name, value, updated_at)`. Hook into the existing `MissionUpdate` outbox path or carve a new `CellToBaseMsg::CounterUpdate`. Honor `content_counters.reset_on` semantics (`mission_complete`, `zone_change`, `never`). |
| Effort | Small (S) — narrow schema, well-bounded write path, no new condition/action shapes. |
| Unlocks | Daily quotas (paired with §3.2). Cross-zone collection counters. Repeatable mission tracking. "Kill X to unlock Y" patterns where X spans hours of play. |
| Justified by | The DB-persistence audit flagged this as the highest-impact persistence gap. The `content_counters.reset_on` column is already in the schema as design intent. |

### 2.3 `Condition::MissionRepeats`

| | |
|---|---|
| Need | `MissionInstance.repeats` is incremented on every completion ([missions.rs:84-88](../../crates/entity/src/missions.rs#L84-L88)) but no condition reads it. Currently every "is this mission available?" gate is `MissionStatus eq not_active`, which becomes false forever after first completion. |
| Engine surface | Add `Condition::MissionRepeats { mission_id, op, value }`. Add `mission_<id>_repeats` to `populate_mission_context`. One loader arm. |
| Effort | Tiny (XS) — ~1 hour of code, ~half a day with tests. |
| Unlocks | Repeatable post-completion missions. "First time you finish this you get a bonus" patterns. Daily content gating (paired with §2.2 and §1.2). |
| Justified by | Existing field on `MissionInstance` that's been incrementing-into-the-void. |

### 2.4 Inventory-count populator (resurrects `Condition::HasItem`)

| | |
|---|---|
| Need | `Condition::HasItem` reads `item_<id>_count` from `ExecutionContext.params` ([conditions.rs:150-151](../../crates/content-engine/src/conditions.rs#L150-L151)) but no populator writes that key. The condition is **dead code today.** |
| Engine surface | New `populate_inventory_context(entity, ctx)` that walks `entity.inventory` and writes `item_<id>_count` for every item. Call it from `fire_dialog_choice`, `fire_interact_tag`, `fire_item_use`, `fire_player_loaded`. |
| Effort | Tiny (XS) — populator + tests. ~half a day. |
| Unlocks | Turn-in NPC patterns ("give me 5 Ambernol vials" → check). Key-locked door patterns ("you must have the keycard"). Quest gates that depend on inventory state, not just possession history. |
| Justified by | Dead-code resurrection. The condition was clearly intended to work; only the populator wire is missing. |

---

## Tier 3 — larger additions justified by content shape

### 3.1 Persistent player flags (branching narratives)

| | |
|---|---|
| Need | Cimmeria has no per-player persistent flag store. "Killed Frost vs. spared Frost" cannot be encoded — the only persistent boolean state is mission status, and that's collapsed into `not_active`/`active`/`completed`. Faction-divergent missions (Harset/SGC_W1) can branch *for one decision point* via archetype gating, but cannot remember a choice across mission boundaries. |
| Engine surface | New table `sgw_player_flags (player_id, flag_name, flag_value)`. New `Action::SetFlag { name, value }` and `Condition::Flag { name, op, value }`. Populator writes `flag_<name>` into `ExecutionContext`. |
| Effort | Medium (M) — schema + populator + condition + action + executor + persistence wiring. ~3-5 days. |
| Unlocks | Branching narratives with permanent choices. Faction-path divergence beyond archetype. "You killed X, the brother NPC remembers" patterns. Reputation-style boolean tracking. |
| Justified by | The mission-shapes audit flagged this as the largest gap blocking real narrative branching. Several SGW missions in the data have decision-point text that the engine cannot honor today. |

### 3.2 Daily / weekly resets

| | |
|---|---|
| Need | No way today to scope state to "this 24-hour window." Quest dailies, daily kill quotas, weekly raid lockouts — none are expressible. |
| Engine surface | A scheduler primitive (paired with §1.2 timer wiring) plus a reset hook on counters (§2.2 honoring `content_counters.reset_on`). Could be as simple as a once-per-tick timer that calls `Action::ResetCounter` on every counter whose `reset_on='daily'` and `updated_at` is from a prior day. |
| Effort | Medium (M) — depends on §1.2 + §2.2 landing first. |
| Unlocks | Daily/weekly content. Reset-able skill point grants. Time-gated repeatable missions. |
| Justified by | Standard MMO surface. Does not exist in the SGW source data prominently — the original game shipped with no daily quests — but the live-service expectation is universal. |

### 3.3 NPC-arrival trigger (escort missions)

| | |
|---|---|
| Need | `Action::MoveWaypoint` does an instant position write today ([executor/world/mod.rs:330](../../crates/services/src/cell/content/executor/world/mod.rs#L330) — no path interpolation; the `speed` field is parsed by the loader and then ignored by the executor). There's no `OnNpcArrived` trigger. So "escort the prisoner to the rings" can only fire on the player's region cross, not on the NPC's actual arrival. |
| Engine surface | First, real path interpolation (Rust gameplay code, not engine). Then `Trigger::OnNpcArrived { entity_tag, region_key }` fired from the path-completion callback. |
| Effort | Large (L) — path interpolation is the real work. The engine surface is small once that lands. |
| Unlocks | Escort missions where the NPC actually escorts (movement events not fake-tied to player position). "Wait for NPC to finish speaking" beats. |
| Justified by | Several SGW missions in the data are escort-shaped. Today they're partially playable because the NPC teleports, which feels broken. |

---

## Things NOT to add to the engine

These sound tempting but belong elsewhere. Documenting them here so the conversation doesn't have to be repeated.

| Tempting addition | Why not |
|---|---|
| **Per-tick AI behavior trees** | Chain dispatch happens in the cell's main event path with a full `ExecutionContext` per matching chain. Running this every game tick per NPC would dwarf the actual content cost. NPC behavior belongs in Rust gameplay code with per-NPC state machines; the engine should consume "NPC-did-X" events from there, not drive them. |
| **Real-time combat math** | Damage formulas, crit rolls, mitigation tables — these need to be hot-path Rust, not JSON-parameterized. The engine's existing `QrCombatDamage` action is a thin call-out, not actual math, and that's the right shape. |
| **Ability cooldowns / GCDs** | Sub-second precision; queryable from input handler before any chain fires. Belongs in `Stat`-backed cooldown timers in the entity, not `Action::StartTimer`. |
| **Pathfinding** | Deterministic Rust running every tick. The engine's `MoveWaypoint` does instant teleport on purpose; if escort movement matters, build the pather Rust-side and surface a single `Trigger::OnNpcArrived`. |
| **Loot-roll RNG** | `Action::RollLootTable` is in the enum but unhandled, and that's fine. Loot tables need server-authoritative randomness with anti-dupe checks; expose only the *result* of a roll back into chains, not the rolling itself. |
| **Inventory layout / equip rules** | `Action::SetActiveSlot` is a thin client poke ([executor/mod.rs:366-397](../../crates/services/src/cell/content/executor/mod.rs#L366-L397)). Resist `Action::SwapInventorySlot`, `Action::EnforceLoadoutRules` — those have invariants that need Rust enforcement. |
| **Reward-selection UI ("pick 1 of 3")** | This is a UI flow with a server callback, not a content-engine primitive. The flow is: player picks reward → client sends choice → server reads it and emits the granted item. Adding a chain trigger for "reward chosen" is reasonable; modeling the picker as engine state is not. |

The litmus test: **if it needs to run at frame rate, hold invariants under concurrent mutation, or do math the engine can't validate, it stays in Rust.** The engine is glue between gameplay-code-defined events and gameplay-code-defined effects. It should be thin, declarative, and slow-path.

---

## Schema-side improvements (independent of variants)

From the persistence audit. Each is small, each pays off as content scales.

| Change | Why |
|---|---|
| Index `content_triggers (event_type, event_key)` | Editor and `load_single_chain_for_test` queries scan it; trivial to add |
| `UNIQUE (chain_id, event_type, event_key)` on `content_triggers` | Prevents accidental duplicate rows from producing double-fire after a flubbed migration |
| Index each `content_*` table on `chain_id` | Editor queries filter by it; PG doesn't auto-index FK columns |
| Filter `enabled = false` chains in the loader (`WHERE enabled = true` in [engine_loader.rs:51](../../crates/services/src/cell/content/engine_loader.rs#L51)) | Nicety, **not** a correctness fix. The boot `SELECT` has no `WHERE` clause, so disabled chains are loaded and registered — but `enabled` is honored in memory at resolve time ([chain.rs:256](../../crates/content-engine/src/chain.rs#L256), and [chain.rs:138](../../crates/content-engine/src/chain.rs#L138) for `fire_event`), so they never match or execute. Filtering in SQL would just stop carrying dead chains in the bucket |
| Add `created_at`/`updated_at` to `content_chains` | Migration drift between environments is currently invisible |
| `content_action_params` JSON Schema registry keyed on `action_type` | Lets the editor validate authoring; lets CI fail on a typo'd `"ammount"` before it ships |
| CHECK constraints on `_type` discriminators (against a curated list) | Surface silent-drop case at INSERT time, not at boot |

---

## How this list was assembled

- **Tier 1** comes from auditing every `Action` variant against the executor's match arms — variants without arms are dead weight that ship with every release.
- **Tier 2** comes from the recent commits on `main` (PRs #214, #220, #237) and a per-condition / per-action audit of populators that don't write the keys the conditions read.
- **Tier 3** comes from the mission-shape audit against the seed data — "are there missions in [db/resources/Missions/Seed/missions.sql](../../db/resources/Missions/Seed/missions.sql) that this engine can't drive?"
- **What NOT to add** is the union of every "but couldn't we just…" question that has a clear architectural answer.

The discipline is: only suggest extensions that are justified by either content already in the database or capabilities already partially in the codebase. No vaporware. No "what would be nice in a hypothetical future."

---

## Related

- [content-engine.md](content-engine.md) — Reference for what already works.
- [extending-the-engine.md](extending-the-engine.md) — How to actually add a variant.
- [serverEd-comparison.md](serverEd-comparison.md) — Where some of the Tier 2 ideas come from.
- [gap-analysis.md](../gap-analysis.md) — Project-wide feature-gap tracking.
- [.claude/plans/](../../.claude/plans/) — Active design docs for in-flight engine work (agentic).
