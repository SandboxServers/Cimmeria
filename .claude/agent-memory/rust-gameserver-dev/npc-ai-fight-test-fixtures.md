---
name: npc-ai-fight-test-fixtures
description: What the npc_ai test fixtures can and cannot assert — no navmesh means find_path returns None, so a chase must be asserted through the no_path log, not nav_path
metadata:
  type: project
---

`cell/service/tests/npc_ai/mod.rs::make_ai_fixture` builds a non-instanced
"Castle" space with **no navmesh loaded**.

**Why it matters:** `SpaceManager::find_path` early-returns `None` when
`space.navmesh` is `None` (`cell/space_manager/spatial.rs`). So a test that
drives `npc_ai_tick` into the chase branch will find `nav_path` **empty**, and
`assert!(!npc.nav_path.is_empty())` fails even though the NPC took the right
arm.

**How to apply:** assert *which arm ran*, not its side effects.

| Arm | Observable |
|---|---|
| chase (non-stationary, out of range or no LoS) | `tracing::info!` "NPC AI: no path to target" — the `no_path` outcome |
| stationary hold | `tracing::info!` "NPC AI: stationary mob holding fire" + `direction.y` snapped to the target bearing |
| fired | `npc.abilities.is_on_cooldown(id)` flips |
| range-rejected by `use_ability` (not by the AI gate) | `npc.ai_retry_at == Some(_)` — the launch-failure retry |

Both arm logs are INFO and `LogCapture` installs a bare `Registry` with no
level filter, so `capture.find_message(Level::INFO, "...")` sees them.
`LogCapture` needs the default `#[tokio::test]` (current-thread) flavor and
interferes under plain `cargo test` parallelism — use nextest or
`-- --test-threads=1`.

`take_last_outcome()` is consumed by `dispatch` at the end of each tick, so a
test cannot read the outcome back after calling `npc_ai_tick`; use the log.

Fixture gotchas:

- `make_ai_fixture` uses `create_entity`, so the NPC's ability bucket starts
  **empty** — good for testing an explicit set. `make_aggression_fixture` uses
  `spawn_npc`, which seeds `NPC_DEFAULT_ABILITY`.
- Put the target due **west** when asserting facing: yaw `-PI/2` cannot be
  passed by a merely-zeroed yaw the way a `[0, PI]` bearing can.
- Keep the target inside `LEASH_DISTANCE` (50) of the spawn point or the leash
  check fires before anything else.
- `SpawnRecord` has **no** `Default` impl. Clone a prototype out of
  `load_spawn_templates` instead, which also pins the template→ability-set
  wiring for free.

Related: [[npc-range-gate-and-weapon-range-columns]],
[[cargo-test-vs-nextest-flakiness]]
