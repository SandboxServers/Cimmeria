# lab_uat_run session end: lab_logout fails with "bridge: connect to bridge at 127.0.0.1:8770"

Rows: `armory-abilities-soldier-0`: `_session_end`. Also seen in the canary as GW-3128 BLOCKED (`lab_login` focus, bridge connect failed).
Runs: `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\2026-11-02-unknown-cgbqjljvxcsrgh` (row JSON `rows/armory-abilities-soldier-0/_session_end.json`).

Expected: `session.end disconnect_reason=logoff_character_select` after the logout, then `Client entities cleaned up disconnect_reason=logoff` after the disconnect.

Observed: `step action 0 (lab_logout) failed: lab_logout failed at step 'character_select' after 56486 ms: bridge: connect to bridge at 127.0.0.1:8770`, at 19:37 to 19:39Z, after the last row (Soldier-1477). All 23 spec rows had already run. The next file's run took the lease at 19:39:21Z and drove normally.

Repro: intermittent; any multi-batch `lab_uat_run` that ends with `end_session` left at its default (e.g. `{sections: ["armory-abilities-soldier-0"], specs_dir: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"}`).

Area: lab (client bridge / `lab_logout`), unknown file.
Labels: lab (see docs/agents/).
