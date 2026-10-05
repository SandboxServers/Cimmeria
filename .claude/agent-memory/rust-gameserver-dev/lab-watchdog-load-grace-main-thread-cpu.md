---
name: lab-watchdog-load-grace-main-thread-cpu
description: labd watchdog load grace - every bridge call (heartbeat too) is main-thread, so a load is only visible as main-thread CPU read from outside SGW.exe
metadata:
  type: project
---

Every bridge RPC, `heartbeat` and `events_read` included, is dispatched in the client's main-thread Tick drain (`client-telemetry/src/bridge/mod.rs`). While a UE3 map load blocks that thread, nothing can be read from the client: the event ring, the tick counter and Lua are all unreachable, so "last event was a load start" cannot be read mid-stall. The server's `server_sessions` `status: "loading"` (pending_map_loaded / pending_client_ready) is ground truth, but labd reaches lab-mcp only when `CIMMERIA_LAB_MCP_URL`/`_TOKEN` are set, and the colo endpoint answered 403 in 2026-09.

The load grace (2026-10-05, `supervisor/stall_grace.rs` + `supervisor/main_thread.rs`) uses the main thread's CPU time via Toolhelp + `GetThreadTimes` (earliest-created thread of the pid): >= 2 % of the poll interval = busy = load; a busy stall waits up to 120 s (`CIMMERIA_LAB_LOAD_GRACE_SECS`), an idle one dies at the old rules. Busy is sticky per stall (`StallTracker`), and the first two samples after a Tick advance are ignored: they still cover rendered frames, so counting them would make every deadlock look busy.

**Why:** world 1300 loads (19-42 s) were killed by MAX_HEARTBEAT_FAILS = 5.
**How to apply:** for any future "client looks hung" logic in labd, sample from outside the process; a busy-loop hang now dies at the cap, not at ~30 s. Related: [[lab-probe-traffic-starves-watchdog]].
