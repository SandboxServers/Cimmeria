---
name: lab-probe-traffic-starves-watchdog
description: cimmeria-lab watchdog killed a healthy client when an agent ran hundreds of tiny client_mem_read calls; heartbeat queues behind probes on the one bridge connection
metadata:
  type: project
---

2026-09-29 live finding: an agent walking the client entity map with one `client_mem_read` per field starved the supervisor's heartbeat (the bridge client is one connection behind one mutex), five misses tripped `MAX_HEARTBEAT_FAILS`, the healthy client was killed, and the relaunch's old Lua autologin failed during the intro movies until the 3-crashes/10-min cap stopped recovery.

**Why:** heartbeat misses were counted as death even while other bridge calls were succeeding.

**How to apply:** in `crates/lab`, bulk memory work reads whole structs (`client_entity_table`: one 0x18-byte read per rb-tree node, one 0x34-byte read per entity) and bypasses the command journal. The watchdog forgives a miss while any bridge call succeeded within `heartbeat::BUSY_GRACE_MS`; crash recovery logs back in with the native `lab_login` flow. Anything new that loops over the bridge should keep reads coarse. See [[injected-dll-unwind-and-lua-error-rules]] for the bridge side.
