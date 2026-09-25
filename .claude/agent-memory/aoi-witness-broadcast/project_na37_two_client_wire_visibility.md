---
name: na37-two-client-wire-visibility
description: NA37 (2026-09-25) built a real two-wire-client Castle visibility E2E test; confirmed AoI introduction works over the real wire in both directions/orders/GM-status, so the owner's report is still unreproduced at two layers
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
