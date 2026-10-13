---
name: Lab client process uptime
description: Source and status semantics for lab_client_status process uptime.
type: reference
---

On Windows, `lab_client_status.uptime_ms` comes from `GetProcessTimes` on the
same handle used for the liveness check. The full creation FILETIME ticks are
saved in this instance's `lab-instance.json` at launch and compared on every
status read. Matching identity is converted to Unix milliseconds for uptime;
a reused PID reports zero with `uptime_state: "pid_reused"`.

At supervisor startup, only a registry entry with the current bridge port
and matching live creation ticks is adopted. Older registry files without
creation identity still work for peer discovery, but startup cannot safely
adopt their process; stop the old client and relaunch to write a verified
entry. An exited or unqueryable process has zero uptime, as does a live
process whose creation time cannot be read. Non-Windows builds return an
unavailable process observation and zero uptime.
