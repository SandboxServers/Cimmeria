# DW-03S send-origin and travel-epoch redesign

> Type: design handoff. Date: 2026-10-16. Scope: packet contracts and ledger only. Static source review, no build or live client run.

## Evidence and decision

`evidence/DW-03S-E.md` (PR #1750, unmerged when this revision was written) traced `crates/mercury/src/transport.rs::Transport::send_to`: only encrypted bytes and peer cross the awaited transport. `crates/base-session/src/base/helpers/mod.rs::collect_pending_retransmits` returns `Vec<Bytes>` without a channel origin. `BaseService::start` projects the same bidirectional transport into the receive loop, Cell fan-out and sweepers. A BaseService-only wrapper that looks up the peer after await can therefore misattribute a paused A send to same-peer B. The sampled `mercury.packet` event and reliable-window registration do not constitute a complete post-send ring.

The replacement contract adds `Transport::send_to_with_origin` as an opt-in, object-safe method with an immutable `SendOrigin`. Phase 3 stores only the fixed generation seed in `ConnectedClientState::extensions`; each sender combines it with the current player entity while the originating session supplies key/sequence/ACK or cached retransmit ciphertext. The Base proof wrapper records only full successful sends and counts untagged/failed/partial/dropped paths as coverage gaps. P builds the seam, G builds boot/generation and observer, H preserves origins through session helpers and retransmit, D covers direct world-entry/Base sends, F audits Cell fan-out and all remaining production send sites, and S publishes only a fail-closed snapshot. Historical A proof is filtered from current B roster; no post-await address lookup is allowed.

GateTravel's Cell producer may enqueue a transition and tear down the origin before Base consumes the message. `ConnectedClientState.world_name`, `world_entry_sent` and displayed roster status can remain unchanged in that interval. The common authoritative source is `SpaceManager::destroy_entity`; its player identity is read before removal. T-C scopes a bounded per-persistent-player epoch and `InProgress` state there, exposed through the existing Cell `LabQuery` channel. T-R settles that exact epoch after destination `onClientReady` and checks the current session/generation/player; X requires a matching stable Cell/Base epoch, and B compares it before/after every joined read. A same-world round trip increments the epoch. Missing/evicted state fails closed.

## Packet sizing and merge order

| Review wave | Packets | Budgets (k working tokens, tests/docs/review included) |
|---|---|---|
| 1B-a | E, C, P, G, H, D | 30, 40, 35, 45, 45, 45 |
| 1B-b | F, T-C, T-R, S | 40, 40, 40, 35 |
| 1B-c | X, I, B | 35, 30, 45 |

Seven prerequisite packets were added (P/G/H/D/F/T-C/T-R); the old S scope was reduced to snapshot/eligibility. Every packet is at most 45k. Merge dependencies before dispatch; C/I may proceed independently of the server chain once E/C merge, but shared files cannot have parallel owners. The seven extra packets total 290k estimated working tokens; all thirteen wave 1B packets total 505k. These are packet budgets, not one agent context.

## Verification and unresolved implementation facts

- Source files/functions in the evidence table were inspected. The `SessionExtensions` slot exists and avoids editing roughly 29 `ConnectedClientState` fixture literals. `LabState::lab_query` already forwards a typed read to Cell. This revision changes no runtime code and runs no cargo or lab tool.
- P/G's wrapper must preserve chaos/lossy transport semantics and firehose sampling. `ChaosTransport::send_to` can return `Ok(len)` for a deliberate drop, and its `SendShaper` can queue/delay delivery with only bytes/peer. P/G therefore mark any active lab-chaos shim or chaos-testing override as incomplete and suppress eligible proof until a separately reviewed origin-aware actual-socket seam exists. An `Ok` from that shaper and a default trait implementation never count as observed wire proof.
- T-C must verify every GateTravel producer reaches central `destroy_entity`; if any bypasses it, the packet stops with that exact path and X stays blocked. T-R must establish a first-world-entry baseline and recheck the current session after Cell's settle reply; a stale A reply cannot acknowledge B.
- F must produce an exhaustive production send-site census, including sweeper and Cell fan-out clones. Any unowned or untagged path keeps `coverage_complete=false`; downstream packets do not call the binding complete.
- The real DLL hook hit and client/server fingerprint pair are unproven until the separately approved DW-35 live lab step. D-DW9 and D-DW12 remain `BlockedDecision`; no lab lease, client start/restart or server switch is authorized by these docs.
