---
name: na37-two-client-wire-visibility
description: NA37 round 1 (2026-09-25) confirmed AoI introduction works over a lossless real wire; round 2 added network chaos and found the real defect -- receive_packet's RX-window ordering gate is dead code, so a lost+retransmitted CREATE_ENTITY lets appearance/movement for that entity arrive first
metadata:
  type: project
---

NA37 added a real end-to-end two-client shared-world visibility test:
`crates/wireclient/src/session.rs` (`GameSession` — a real UDP socket
driving `cimmeria_mercury::test_harness::LoopbackPeer` as the client-side
Channel against a real `BaseService`, not a paired loopback peer),
`crates/wireclient/src/bundle.rs` (structural S2C bundle decoder: msg_id /
entity_id / class_id / method_index, ported from
`tools/pcap_dissect.py`'s `SERVER_MSG_FORMAT`), and
`crates/wireclient/tests/two_client_castle_visibility.rs` (live-DB, spawns
a real `Orchestrator`).

**Result: full pass, both directions, both arrival orders, GM and
non-GM.** This is one layer deeper than NA34's in-process
`two_player_visibility` test (same conclusion: no server-side bug found).
Two independent test layers now agree the AoI introduction mechanism
works; the owner's report (players in Castle/Harset not reliably seeing
each other) remains unreproduced server-side. Next suspicion:
client-side ghost-slot handling, or a network condition (loss/reorder)
neither test replicates (both run over lossless localhost).

**Two real gotchas found and fixed along the way (test-infra, not AoI
bugs) — worth remembering for any future test that spins up a real
`Orchestrator`/`CellService`:**

1. **`entities_dir` CWD trap.** `CellService::start()`
   (`crates/services/src/cell/service/mod.rs`) loads
   `entities/spaces.xml` from the literal relative path `"entities"` —
   no `ServerConfig` field or setter overrides it. `cargo test`'s CWD
   for an integration-test binary is the *package* directory
   (`crates/wireclient/`), not the repo root, so this silently fails
   ("Unknown world: X") and the entity is never registered in any real
   `SpaceManager` space — meaning it can NEVER appear in anyone's AoI,
   symptomatically identical to a real AoI bug. Fix: `chdir` the test
   process to the repo root (`env!("CARGO_MANIFEST_DIR")` + `../..`)
   before calling `Orchestrator::start_all()`. See
   `chdir_to_repo_root_for_entities_xml` in the test file.
2. **Ghost class-flattening confirmed, not a bug.** Every AoI-introduced
   player ghost's `CREATE_ENTITY` carries `class_id = 0x02` (SGWPlayer),
   even when the observee is a GM (`class_id = 0x03` only appears in the
   *owning* client's own `CREATE_BASE_PLAYER` at world entry). This
   matches [[player-ghost-aoi-cascade]] (not in this memory dir, see
   `docs/architecture/player-ghost-aoi-cascade.md` Known Gaps —
   `connect_entity` stamps `class_id = 0x02` for every player
   unconditionally). A test asserting the ghost's class byte should
   equal the real GM class is wrong, not the server.

**Reusable pattern:** `LoopbackPeer::from_socket` (Tier 2 harness) works
fine as a one-sided real-client Channel driver against a real server
socket — no second reliable-delivery/reassembly/ACK implementation is
needed for future wireclient-style E2E tests. Requires
`cimmeria-mercury`'s `test-harness` feature.

## Round 2 (2026-09-25): lossy-network chaos found the real defect

The coordinator asked for a lossy-network variant since round 1 (like
NA34) only ran over lossless localhost, while the owner plays over the
internet. Added `crates/wireclient/tests/two_client_castle_visibility_chaos.rs`
plumbing a real `LossyTransport` into a real `BaseService` socket via a
new `chaos-testing` Cargo feature +
`BaseService::set_transport_override` seam (`crates/services/src/base/service.rs`).

**Confirmed defect:** `Channel::receive_packet`'s in-order RX-window
delivery gate (`crates/mercury/src/channel/channel_core.rs`) is fully
implemented and unit-tested but is **dead code** — grep finds exactly
one call site, its own unit test. Neither the live server receive path
nor `LoopbackPeer`'s recv pump ever calls it; a non-fragmented packet's
body is returned immediately with no ordering check. Reproduced with a
deterministic burst-drop test (`LossyTransport::drop_next_sends_to(n,
addr, min_len)`, added this round for exactly this) targeting a
witness's `CREATE_ENTITY` for a peer: once retransmitted, dozens of the
peer's appearance/stat cascade and movement messages arrive **before**
the retransmit — an update for an entity the witness was never told
exists. This is the strongest candidate yet for the owner's original
report, since it only manifests under real packet loss.

**Fix attempted and reverted:** made the AoI cascade
(`entered_aoi` in `crates/services/src/base/world_entry/cell_dispatch/aoi.rs`)
wait for `CREATE_ENTITY`'s ACK (polling the witness's TX window) before
sending the cascade. Closed the cascade-specific hole but Mercury only
piggybacks ACKs on the peer's own next outbound send, so an idle
witness doesn't ACK promptly even at zero loss — the wait stalled
*every* entity introduction on every login, turning a rare packet-loss
bug into a universal latency regression. **Do not ship this pattern.**
A real fix needs either genuine in-order delivery wired end to end
(server recv path + `LoopbackPeer`'s pump both bypass `receive_packet`
today) or a reactive hold keyed on `TxEntry::retransmit_count > 0` (only
engages once a real retransmit happens, never on the happy path) — a
protocol-layer change needing its own design review, out of scope here.
The repro is preserved as `#[ignore]`d
(`burst_drop_of_peer_create_entity_recovers_via_retransmit`) rather than
shipped failing.

**Gotcha for future `LossyTransport` work:** a reorder buffer shared
across destinations breaks the Mercury phase-3 handshake (the first two
packets of *any* fresh connection are parsed positionally) — key it
per-`SocketAddr` (`HashMap<SocketAddr, Vec<Vec<u8>>>`), and even then
avoid combining reorder with a lossless-handshake-sensitive scenario;
document the limitation in the test rather than force it.

**Post-rebase gotcha:** a new hand-named `tracing::debug!(target:
"...")` call trips `crates/server/src/logging/target_scan_tests.rs`'s
SigNoz parity guard (`every_source_target_reaches_signoz_at_its_level`)
even when it's behind a test-only feature — the scanner reads source
text, not compiled output. Any new custom log target needs an entry in
`crates/server/src/logging/filters.rs`'s `OTEL_FILTER`.
