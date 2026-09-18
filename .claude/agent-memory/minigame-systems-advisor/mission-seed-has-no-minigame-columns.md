---
name: mission-seed-has-no-minigame-columns
description: The mission_steps/objectives/tasks seed tables carry no minigame type or minigame difficulty field — everything comes from the content chain
metadata:
  type: project
---

# Mission seed tables carry NO minigame metadata

Verified against `db/resources/Missions/Seed/` on 2026-09-17. Full column
lists, taken from the INSERT tuples:

- `mission_steps` (line 7): `step_id, mission_id, award_xp, difficulty,
  step_enabled, step_display_log_text, index`
- `mission_objectives` (line 7): `objective_id, step_id, award_xp, difficulty,
  is_enabled, is_hidden, is_optional, display_log_text`
- `mission_tasks` (line 7): `task_id, objective_id, award_xp, difficulty,
  is_enabled, task_type`

Conclusions that keep coming up:

1. **No minigame-type column anywhere.** The game name lives only in a
   `content_actions.target_key` for a `start_minigame` row. If someone asks
   "which minigame does step N use?", the PAK cannot answer it — only a chain
   can, and for most zones no chain exists yet.
2. **`difficulty` on these tables is NOT the minigame difficulty.** It is a
   1/2-valued mission-content rating; the minigame difficulty param is a
   separate hardcoded `1` (see [[chain-wiring-and-gaps]]).
3. **`task_type` is `1` for all 4358 rows** — no discriminator. It encodes
   nothing usable; do not try to read a minigame type out of it.
4. `step_enabled` is `false` on essentially every original-PAK step row; that
   is the canonical-PAK default, not a signal the step is broken.

Only three rows in the whole mission seed mention a minigame, all as free text:
- `mission_objectives.sql:559` — obj 4203 `'Placeholder minigame.'`
- `mission_objectives.sql:1751` — obj 4963 `'to do: add button to dialog for minigame'`
- `mission_steps.sql:5327` — mission 582 step 2307 `'Conversation Minigame With Marsh'`
