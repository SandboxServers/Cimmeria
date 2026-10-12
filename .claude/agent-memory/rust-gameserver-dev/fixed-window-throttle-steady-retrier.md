---
name: fixed-window-throttle-steady-retrier
description: A fixed-window per-key log throttle never bites on a retrier slightly slower than the window; escalate the window and reset on quiet-since-last-seen, not on the window
metadata:
  type: project
---

`RefusalLog` (admin-api telemetry ingest) used a fixed 10 s window per
(uploader, reason). On 2026-10-12 an abandoned lab client retried an expired
token every ~12 s for 4.5 h: one WARN per retry (`suppressed=2` each), and with
`[discord.events] warning = true` 1,363 Discord posts in 6 h.

Fix: the window doubles per written row up to 15 min (`MAX_WINDOW`), and resets
to 10 s only after `RESET_QUIET` (15 min) with no refusal at all. A reset
compared to the *current* window (e.g. "quiet longer than 10 s") resets a 12 s
retrier on every retry, and a "quiet a full window after close" rule still lets
any retrier slower than ~2x the first window through forever.

Credential refusals (`missing_token`, `bad_token`, `token_expired`) now carry
`event = "upload_auth_refused"`, listed in `SIGNOZ_ONLY_EVENTS`; other refusals
carry `upload_refused` and still post.

**Why:** any per-key throttle on a client-driven retry loop has this shape.
**How to apply:** when writing a log throttle for retried client traffic, test
it with a periodic caller slower than the window (inject `Instant`s), not just
a same-instant burst. Related: [[throttle-key-hides-transitions]].
