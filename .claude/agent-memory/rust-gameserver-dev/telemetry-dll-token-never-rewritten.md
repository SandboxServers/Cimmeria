---
name: telemetry-dll-token-never-rewritten
description: No launcher rewrites current-session.json on token refresh; the telemetry DLL's upload gate re-reads it but in practice uploads stop at the first token's expiry
metadata:
  type: project
---

As of 2026-10-12, nothing rewrites `current-session.json` while the client runs:
launcher-core `Telemetry::replace_session` and the desktop engine transport keep a
refreshed token in memory only; the lab supervisor writes the file once per launch
(cached mint, reused only with 30+ min left). The DLL's `upload_gate` re-reads the
file on 401/403 and at each backoff deadline; past `expires_at_ms` (by the client's
clock, which may be ahead) it sends one probe at once and then one per 10 min, so a
DLL session goes nearly quiet at its first token's expiry.

**Why:** a lab client retried an expired token every flush for 4.5 h (colo log and
Discord flood); the gate fixes the flood, not the lost uploads.

**How to apply:** if someone asks why DLL events stop after ~8 h, the fix is on the
launcher side (write the refreshed token into the file with `atomic_write`), not in
the DLL. The DLL already picks up a different token from the file.
