# Chain-replay tests must *execute* when the change is an executor arm

`crates/services/src/cell/content/chain_replay_tests/` historically stopped at
`ChainEngine::resolve_event` and asserted on the resolved `Action` list. That
shape cannot tell a wired executor arm from `executor/mod.rs`'s `other =>`
catch-all — which is exactly how `move_entity`'s five seeded rows no-opped in
production for as long as they did while the suite stayed green.

If the change is an executor match arm, push the `ResolvedActions` through
`super::super::executor::execute_actions` and assert on the `CellToBaseMsg`
traffic drained off the `mpsc` receiver. `execute_actions` is `pub(super)` in
`executor`, i.e. `pub(in crate::cell::content)` — visible from any
`chain_replay_tests` submodule.

Loading the chain from the DB guards the seed; executing it guards the arm.
Both halves are load-bearing; neither substitutes for the other.

## A verb with zero seed rows still gets a replay

Insert a sentinel chain (`0x7000_xxxx` chain id per TESTING.md's live-DB
rules), load it through `load_single_chain_for_test`, then **delete by exact
id before asserting** — a panic between insert and cleanup would otherwise
leave a live chain registered in the shared test DB for every later test.
Also cleanup-before-insert so a previously-panicking run doesn't collide.

Sentinel reservations are per-module and documented in each module's header.
`0x7000_5000` is taken by `chain_replay_tests/grant_xp.rs`; the
`crates/services` neighbours run `0x7000_1000..0x7000_1B00`, `0x7000_2000`,
`0x7000_3000`, `0x7000_4000`, `0x7000_4242`.

## Gotcha: `world` as both a field name and a module path

`Action::MoveEntity` destructures a field named `world` inside
`executor/mod.rs`, where `world` is also a sibling module. This compiles —
path resolution for `world::move_entity` uses the type namespace, the local
binding lives in the value namespace — but it reads badly enough to trip a
reviewer. It is intentional, not an oversight.
