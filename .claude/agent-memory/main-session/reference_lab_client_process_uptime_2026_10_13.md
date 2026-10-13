---
name: Lab client process uptime
description: Source and status semantics for lab_client_status process uptime.
type: reference
---

On Windows, `lab_client_status.uptime_ms` is derived from the live PID's
process creation time via `GetProcessTimes`, converted from FILETIME to Unix
milliseconds. The PID must still be live. An exited process reports zero with
`uptime_state: "exited"`; a live process whose creation time cannot be read
reports zero with `uptime_state: "creation_time_unavailable"`. This avoids
using `SupervisorState.started_at`, which resets when the supervisor is
recreated and can otherwise describe a stale PID. The process-time accessor is
unavailable on non-Windows platforms, where uptime consequently remains zero.
