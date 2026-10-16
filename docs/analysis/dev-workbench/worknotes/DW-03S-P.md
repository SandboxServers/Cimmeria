# DW-03S-P: Mercury send-origin seam

> Status: implemented locally, awaiting adversarial review and integration. No live client or lab action.

`Transport` now accepts an optional caller-captured `SendOrigin` through `send_to_with_origin`, plus a separate pre-Phase-3 send method. Both default implementations delegate to the existing `send_to` without changing bytes or destination and without recording proof. A wrapper test pauses an old same-peer send while a newer origin completes, then asserts the completed wrapper records retain the corresponding origins and the underlying transport sees only the literal datagrams.

The protocol diagnostic paragraph names the new API and its limit. This packet does not attach origins in Base callers, create proof rows, or establish complete binding; those remain DW-03S-G/H/D/F and later packets.

Checks in packet order from this worktree, each through `tools/build-lane/lane.ps1`:

- `cargo fmt --all -- --check`: `status=ok exit=0`.
- `cargo clippy -p cimmeria-mercury --all-targets -- -D warnings`: `status=ok exit=0`.
- `cargo test -p cimmeria-mercury send_origin`: `status=ok exit=0`, 2 passed.

Source and docs changed: `crates/mercury/src/transport.rs`, `crates/mercury/src/transport_origin_tests.rs`, `docs/protocol/mercury-wire-format.md`, this worknote and the campaign ledger row. No new project-memory fact was found beyond the already recorded DW-03S-E evidence and protocol note.