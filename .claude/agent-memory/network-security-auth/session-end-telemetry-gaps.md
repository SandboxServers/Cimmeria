---
name: session-end-telemetry-gaps
description: 2026-10-10 telemetry-gaps review of auth/session end - which session ends emit session.end, admin API rows never reach SigNoz (tower=off matches tower_http)
metadata:
  type: project
---

Verified 2026-10-10 against colo SigNoz and the code (review: `docs/analysis/telemetry-gaps/reviews/auth-network.md`, packets TG-NET-01..15).

- `session.end` (`base-session/.../session_teardown.rs`) fires only when the teardown still sees `player_entity_id`. logOff to character select (`base/src/base/dispatch/session.rs`) clears it first, so it never emits one. The client never sends DISCONNECT or logOff(1) (0 in 7 days), so every real quit, crash and lab kill ends as `inactivity_timeout`.
- `session.start` fires on every `InitPlayerState`, including gate travel, so starts and ends don't pair.
- Duplicate-login evictions (~40/week) are mostly crash-then-relaunch inside the 60 s reap, not account sharing. The WARN carries no old-session idle time.
- The `tower=off` directive in `OTEL_FILTER` prefix-matches `tower_http`, so admin API request rows never reach SigNoz (0 rows in 30 days).
- The S1 spec label: the code emits lowercase `logoff`, and only for the account-level Mercury logOff.

**Why:** answers "how did this session end" questions without re-deriving.
**How to apply:** check whether TG-NET packets have merged before trusting these facts again.
