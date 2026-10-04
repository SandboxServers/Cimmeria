---
name: ability-telemetry-coverage-gate
description: AB-C7 coverage gate - adding or moving an ability method means touching the script set plus up to four scanned code tables; scanner shapes; the AB-C6 timing join tables are process-global
metadata:
  type: project
---

**The gate (AB-C7, 2026-10-04).** `tools/telemetry-coverage/abilities.py`
owns the ability method set (`ABILITY_METHODS`), the `NOT_IN_SET` list (every
dispatch-table row whose name matches `abilit|effect|combatdebug|healdebug|combatlos`
must be in one or the other) and `EXCEPTIONS` (empty travelling cells, with a
reason; a filled cell with an exception is "stale" and fails). `--check` runs
in CI's build-and-test job and also diffs the committed
`docs/analysis/ability-mechanics/telemetry-coverage.md`.

Scanned tables (regex, so keep their shapes: `generic(N, "x")`, `(N, "x")`
tuples, or struct literals with `index:`/`cell_index:` + `name:`/`method:`):
- server recv: `crates/cell/src/cell/dispatch/ability_receipt.rs` `ABILITY_RECEIPTS`
  (router writes `ability_method_recv` before the GM gate unless the entry names
  a handler-owned event like `use_ability_recv`);
- server send: `cell-combat` `wire_ledger/coverage.rs` `LEDGER_METHODS` (test:
  each must decode to a non-Other/Short `Decoded`);
- client send: `ability_trace/decode.rs` `ALLOWLIST`; client recv:
  `ability_trace/recv_methods.rs` `METHODS`;
- client declaration `ability_trace/coverage.rs` `CLIENT_SENDS`/`CLIENT_RECVS`
  must equal set minus exceptions (script) and resolve via `spec_for` /
  `recv_methods::resolve` from the wire id (Rust test).

**AB-C6 timing.** `ability_trace::timing::with_timing` is one process-global
table shared by the router (send), the network-thread recv hook and the
main-thread applied hooks. Tests that touch it must use ids no other test
uses (negative / 31_999-style). The histogram store
(`governor::ability_timing`) is thread-local under `cfg(test)` so hook tests
cannot leak `client.ability.timing` events into governor tests.

Related: [[method-idx-duplicate-table-drift]], [[client-telemetry-governor-classify-table]].
