---
name: project-lab-chaos-ch04-review
description: CH-04 lab-chaos registry (2026-10-11): addr-keyed state outlives its session, module trusts caller's expires_at, knob effects outlive TTL
metadata:
  type: project
---

CH-04 (`crates/base-session/src/base/lab_chaos/mod.rs`, commit 43b042536) reviewed
2026-10-11. Gates (feature + DEVELOPER_MODE + lab token) are sound, so nothing here
reaches a player server; colo runs DEVELOPER_MODE=false and never builds the feature.

Findings handed back:
- **State outlives the session.** Keyed by full `SocketAddr`, no owner identity, no
  clear in `teardown_session` (helpers/session_teardown.rs). A relaunch/takeover on
  the same addr (see [[exploit-address-reuse-session-takeover]]) inherits up to 30 min
  of chaos. D-CH3 says "one session by entity id"; the code gives "one address for TTL".
- **TTL cap not enforced in the module.** `set` takes a raw `Instant`; `TtlTooLong`
  is unreachable here; the cap lives only in CH-05.
- **Knob effects outlive TTL** (CH-06/07 consumers): `Delay { ms: u32 }` sleep and
  queued latency are captured at read time; clear cannot cancel them.

Full-addr keying (not IP) is correct: `connected` is also addr-keyed, so two live
sessions never share a key. Poison handling (`into_inner`) and expire-on-read are
atomic and sound.

**How to apply:** on CH-05/06/07 reviews, check the teardown clear and owner check
landed, knobs are clamped, and status's addr->entity_id label is resolved under
[[pattern-deferred-name-resolution]] rules.
