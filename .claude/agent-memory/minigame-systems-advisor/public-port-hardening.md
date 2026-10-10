---
name: public-port-hardening
description: SFS port 30000 is internet-facing; why scanner bytes produced empty-field WARNs, the listener limits, send timeout and keepalive chosen (2026-10-10), and which login rejections may WARN
metadata:
  type: project
---

Port `30000/tcp` is published on all interfaces on the colo, so scanners reach it.
Tracked as issue #532 (see `docs/analysis/issue-triage-2026-09-25/findings/sec-b.md`).

**Why the Discord WARN had empty `msg_type`/`body_action`:** a TLS ClientHello
(`16 03 01 00 ..`) or RDP probe (`03 00 ..`) has a `0x00` within its first bytes, which
ends an SFS frame. The codec got a few bytes of binary with no `<msg>` element, so both
fields stayed `""` and `parse_message` WARNed from inside the codec. A plain HTTP request
has no NUL, so it just times out or hits the 4 KiB cap; it never produced that WARN.

**Rule since 2026-10-10:** `protocol::parse_message` returns `Result<_, ParseError>` and
never logs. Pre-login rejects log DEBUG with `reason=non_sfs_preauth` + `peer`; logged-in
sessions WARN with entity/player pairs and the type fields. DEBUG still reaches SigNoz
(`cimmeria_minigame=debug` OTLP row). Only WARN/ERROR reach Discord.

**Limits (`server/limits.rs`, `ListenerLimits::default()`):** 256 total, 8 per source
IP, 30 s accept-to-login, 4096-byte frames (unchanged, C++ parity), 30 min inbound idle.
Idle expiry reports Canceled (0). The original C++ had none of these.

Also 10 s per send (`SEND_TIMEOUT`; a timed-out send ends the connection and skips the
teardown frames) and TCP keepalive 60 s / 10 s / 3 probes on accepted sockets (socket2).

**Ticket one-live-connection:** `authenticate_and_claim` returns `Result<_, ClaimRejection>`
and refuses a session already `connected` (WARN `ticket_already_claimed`, the only
login-phase WARN reachable with a real ticket). Ticket/game mismatches are INFO: entity ids
are guessable, so they were a Discord flood vector. A reconnect is only free once the
server has noticed the old socket is gone (FIN/RST, failed or timed-out send, keepalive
~90 s, idle 30 min). A half-open drop is refused as `ticket_already_claimed` until then.

Still open from #532: Flash policy `domain='*'`, accept-rate limiting, ticket-to-IP binding.

See also [[session-lifecycle-original]].
