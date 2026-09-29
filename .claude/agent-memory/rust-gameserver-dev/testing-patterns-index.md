---
name: testing-patterns-index
description: Sub-index of the rust-gameserver-dev testing-pattern notes (nextest vs cargo test, revert proofs, live-DB races and ports, chain replay, encrypted test sessions, LogCapture)
metadata:
  type: reference
---

# Testing patterns notes

Moved out of MEMORY.md on 2026-09-28 (AM-12 compaction) to keep the index under its read limit. One line per topic file.

- [cargo-test-vs-nextest-flakiness.md](cargo-test-vs-nextest-flakiness.md) — `cargo test -p cimmeria-services` has order-dependent failures; validate with nextest.
- [db-test-revert-verification.md](db-test-revert-verification.md) — split DB code into a pure helper + shell; revert seed guards in place.
- [bincode-persisted-cache-format.md](bincode-persisted-cache-format.md) — bincode 2 needs `config::legacy()`; the wrong config decodes silently.
- [live-db-scratch-cluster.md](live-db-scratch-cluster.md) — `db.bat init` loads nothing; scratch Postgres recipe on :5544.
- [chain-replay-executor-guards.md](chain-replay-executor-guards.md) — run `execute_actions`, not just `resolve_event`; `0x7000_5000` reserved.
- [local-postgres-port.md](local-postgres-port.md) — probe port and DB name first; on a wrong one live-DB tests skip green.
- [test-file-split-without-touching-mod-rs.md](test-file-split-without-touching-mod-rs.md) — `tests.rs` -> `tests/mod.rs` needs no parent edit.
- [revert-test-restore-crlf-trap.md](revert-test-restore-crlf-trap.md) — restore with `git checkout HEAD -- <file>` between revert tests.
- [revert-verification-checkout-wipes-uncommitted.md](revert-verification-checkout-wipes-uncommitted.md) — scope restores to one file; checkpoint per packet.
- [revert-verification-loses-uncommitted-fmt.md](revert-verification-loses-uncommitted-fmt.md) — run `cargo fmt` before a WIP checkpoint.
- [revert-proof-mutation-must-be-confirmed.md](revert-proof-mutation-must-be-confirmed.md) — a failed scripted mutation reports every guard "ok"; confirm it applied, never split on `=>`.
- [vacuous-guard-and-sentinel-collision-review.md](vacuous-guard-and-sentinel-collision-review.md) — review checklist: vacuous guards, fixtures that fail two rules, `0x7000_xxxx` collisions.
- [interact-range-and-logcapture-traps.md](interact-range-and-logcapture-traps.md) — `get_entity` spans all spaces, so proximity gates need a space check; LogCapture cargo-test flake fixed in #891.
- [test-session-packets-are-encrypted.md](test-session-packets-are-encrypted.md) — TestTransport packets are encrypted (zero key) and feedback lines need player_entity_id; decrypt before grepping text.
- [wireclient-passive-session-dies.md](wireclient-passive-session-dies.md) — a listen-only `GameSession` is reaped at 60 s; send an unreliable AUTHENTICATE heartbeat, as `sparbot::run` does.
- [live-db-lock-race-tests.md](live-db-lock-race-tests.md) — a lock-race test must see the waiter blocked first.
- [forced-db-race-share-lock.md](forced-db-race-share-lock.md) — deterministic type-5 live-DB race with no code hook: hold `LOCK TABLE ... IN SHARE MODE`, release once `pg_stat_activity` shows N lock waiters.
