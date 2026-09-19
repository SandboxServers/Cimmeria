---
name: vacuous-guard-and-sentinel-collision-review
description: Four recurring review findings on packet branches — vacuous regression guards, fixture-checks-itself asserts, live-DB-only trigger coverage, and cross-branch sentinel-id collisions under ci-live-db
metadata:
  type: feedback
---

Four failure shapes found reviewing CA10 (PR #663) that recur on any packet branch. Check for all four before declaring a packet done.

**Why:** each one produces a test suite that is green and proves nothing. Three of the four passed CI and the original author's own revert proof.

**How to apply:** when reviewing or finishing someone else's packet, run this checklist before validating.

1. **Vacuous guard — the revert-proof lies.** A test can fail to exercise a precondition, so the revert produces no change either way. CA10's `leaving_the_space_stops_the_pending_make_gate` asserted "no late gate-open after `destroy_entity`" but never expired the 4 s dial deadline, so the tick emitted nothing whether or not the scrub existed. **Check:** does the revert proof name a DIFFERENT test than the one whose name matches the behaviour? That is the tell — the author noticed the wrong test failing and wrote it down instead of investigating.

2. **Fixture checking itself.** `only_6100_and_6113_are_wired` asserted twelve event ids were absent from a `sequence_map` the test fixture itself populated with exactly two entries. Zero signal. **Check:** does the assert's subject come from the fixture or from production code?

3. **Live-DB-only coverage of a pure-value feature.** `require_db_or_skip!` reports PASS when it skips, so a feature covered only by chain-replay guards has *no* signal in CI's no-DB `test` job or in a contributor's first `cargo test`. New content-engine triggers need the no-DB half too: `Trigger::matches()` (keyed / wrong-key / wildcard / missing-param / cross-trigger negative) in `crates/content-engine/src/triggers/tests/` plus `convert_trigger` arms (including near-miss `event_type` spellings) in `loader/tests/trigger_conversion.rs` — `convert_trigger` is private to `loader`, so the two halves must live in separate modules.

4. **Sentinel-id collisions across concurrent packet branches.** Under `ci-live-db` every test shares one database, so two branches claiming the same `0x7000_xxxx` id is a flake that only appears after both land — invisible to either branch's own CI. Two sub-rules: never derive an id inline (`DIALED_CHAIN_ID + 1` is invisible to anyone grepping reservations, and is exactly how CA10 collided with mission 701's `TEST_PLAYER` at `0x7000_6001`); declare every id as a named constant and record the neighbouring claims in the module docs. Known claims as of 2026-09-18: `0x7000_0000`–`0x7000_5000` general `crates/services`, `0x7000_6000`/`_6010` CA04 `livewire_pairs`, `0x7000_6001` mission 701 `TEST_PLAYER`, `0x7000_6200`/`_6210`/`_6220` CA10 stargate triggers, `0x7000_6230`+ free.

Related: [[chain-replay-executor-guards]], [[db-test-revert-verification]], [[cargo-test-vs-nextest-flakiness]], [[local-postgres-port]].
