---
name: discord-noise-and-teardown-race
description: How Discord errors-channel noise was triaged (2026-09-29) — SIGNOZ_ONLY_EVENTS table, the logOff witness-send race, and which colo warns are real faults
metadata:
  type: project
---

Colo Discord only has `lifecycle` + `errors` channels (docker/compose.discord.yml),
so every harvested WARN/ERROR lands in the one errors channel; `auth` is not
configured there, so "Duplicate login" WARNs are the only Discord trace of a
relog.

- **Filter mechanism:** `crates/discord/src/layer/mod.rs` has target-level drops
  (`movement.validation`, `CLIENT_REPLAY_TARGETS`) and `SIGNOZ_ONLY_EVENTS`
  `(target, event)` pairs matched on the structured `event` field. Add a row only
  for data (content gaps, per-deploy repeats), never for a fault.
- **Teardown race:** logOff / `destroy_client_entities` unmap the player from
  `entity_to_addr` before the cell sees `DisconnectEntity`; the cell's queued
  `EntityMoved` relays then miss (20+ WARNs in <1 ms, `entity_count_in_map` 0).
  Fixed at source: `unmap_departed_witness` + `log_addr_miss` in
  `base-session/src/base/helpers/departed_witnesses.rs` (DEBUG
  `witness_session_ended` for 30 s). Any new unmap site must use that helper.
- **spawn_off_mesh:** detector now skips props (`static_mesh`) as well as
  `is_stationary`; every stasis-room (debug hub) spawn is stationary, guarded by
  `live_db_debug_hub_stationary`.
- **Real faults left posting (owner calls):** `login_retry_on_channel`
  (14-row train per stuck login, reply_outstanding=false), Duplicate login,
  rmcp disallowed Host header (lab MCP misconfig), lab.auth, dev-session quota.

**Why:** owner asked for a quiet errors channel without losing SigNoz data.
**How to apply:** before adding a Discord filter, check SigNoz for whether the
source is a race or data gap that can be fixed where it is emitted.
