---
name: reference-signoz-log-mining
description: "How to mine colo logs through the SigNoz MCP without flooding context: results over ~25k tokens land in a tool-results file, so condense with a script; the filters, keys and session anchors that work"
metadata:
  type: reference
---

For agents with the SigNoz MCP connected (setup: `docs/operations/signoz-remote-access.md`). Written 2026-09-20; the three indexes (`cimmeria-server`, `cimmeria-network`, `cimmeria-trace` for TRACE only) are from 2026-09-25.

- **Size:** `signoz_search_logs` returns one large JSON line. Anything over about 25k tokens is written to a tool-results file under the session directory. Read it through a condense script (JSON → `HH:MM:SS.mmm LEVEL scope | body | k=v …`), never by reading the raw file.
- **Never call `signoz_get_field_keys` unfiltered.** The key list is huge.
- **Filters that work:**
  - `body = 'wire_inbound' AND peer = '<ip:port>'`, `msg_name IN (...)`
  - `decoded CONTAINS '"update_id":0,'` (`update_id` itself is not an indexed key)
  - `witness_id = N` on `wire_outbound`
  - `body NOT CONTAINS 'movement.validation_reject'` drops the navmesh-reject noise
- **Wire attributes** live under `attributes_string` (`msg_name`, `peer`, `decoded`).
- **Time:** `start`/`end` are epoch milliseconds (2026-09-20T00:00Z = 1789862400000). Timestamps the owner quotes from Discord are US Central (CDT is UTC-5 in summer).
- **Session anchors:**
  - `World entry: sending RESET_ENTITIES` (login; has addr, entity_id, position)
  - `Gate travel: sending RESET_ENTITIES`
  - `player entered world` (cell side; entity_id only)
  - `player session ended` (has `disconnect_reason`: `logOff`, `duplicate_login`, `inactivity_timeout`)
- **Normal, not faults:**
  - An idle client sends `AUTHENTICATE` about 6/s (DEBUG, not `wire_inbound`) and `perfStats` every 15 s. See [[reference-client-idle-send-cadence]].
  - A fresh client's first `avatarUpdateExplicit` has `update_id` 0 and `vel == pos`.
- **`wire.out` stat values are wrong** until #843 is fixed: the decoder misreads `onStatUpdate` entries.
