---
name: project-cellblock-681-hidden-2026-10-13
description: "Cellblock controller mission 681 tracker leak and seed fix (#1688)."
metadata:
  type: project
---

- 2026-10-13 lab baseline: Praxis Commando `Blockrun` in Castle Cellblock Mess Hall showed `Mess Hall Controller` / `Kill the Guards` in `client_ui_state.mission_tracker`. The remote server database had `resources.missions.is_hidden = false` for mission 681.
- The other five guard controller missions, 682-686, are seeded hidden. Changing 681 to `is_hidden = true` makes all six use the existing hidden-mission frame suppression. The live-DB regression test loads the seeded mission definitions and asserts the family is hidden; it fails if 681 is reverted.
- Local live-DB test `live_db_cellblock_guard_controllers_are_hidden` passed against a reloaded worktree database. Live client UAT on that database passed: fresh character `Codexfix` was assigned 681 through `/gmmissionassign 681 1`; server logged `mission_accepted` and `mission client frames suppressed` with `reason="hidden_mission"`, while `client_ui_state.mission_tracker` showed only `Arm Yourself!`. God mode was enabled before the check and confirmed by client Feedback.
