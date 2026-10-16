# DW-03S binding design handoff

> Type: campaign worknote. Audience: coordinator, packet coders and reviewers. Date: 2026-10-16. This records a design, not an implemented or live-verified binding.

## Why the addendum exists

The DW-03 dossier accepts an optional `CaptureContext` but its live reader currently supplies `client_context: None`, so server fields show `server_session_unbound`. That is correct fail-closed behavior. A roster entity number, player name or account name could identify the wrong session on another shard or after relog. The supervisor's PID plus Windows process creation ticks identify its process, not a BaseApp session. The current telemetry `session_id` belongs to the launcher telemetry stream and is not a Mercury session ID or authority. `server_sessions` exposes a BaseApp socket peer and player entity, while the DLL receive detour can read a client-side Mercury peer and encrypted datagram fingerprint. Client destination and server source endpoints differ under NAT and may be reused. Issue #1703 records that the receive hook has not supplied live proof of its endpoint or events yet.

The source seams used to frame the packets are `crates/lab/src/supervisor/{process,status,events/store}.rs`, `crates/client-telemetry/src/hooks/iat_hooks/mod.rs::recvfrom_detour`, `crates/client-telemetry/src/hooks/mercury_recv/wire.rs::{stash_received,recv_event}`, `crates/lab-mcp/src/tools/sessions.rs`, `crates/base/src/base/service.rs::online_players`, `crates/base/src/base/login/mod.rs`, `crates/base/src/base/connect_loop/mod.rs`, and `crates/base-session/src/base/mod.rs::ConnectedClientState::connected_at`. `crates/base-session/src/base/helpers/reliable_send.rs` logs encrypted fingerprints for reliable datagrams, but that log alone is not a complete send observer. These are source observations; the exact complete server transmit seam and live DLL hook outcome remain proof tasks in DW-03S-E and DW-35.

## Decision and packet slicing

Five packets, each 30–45k working tokens, have independent ownership: DW-03S-E maps and fixtures the path (docs), DW-03S-C makes the process-bound DLL receive observation reliable (client-telemetry), DW-03S-S produces generation-scoped server wire proofs (base), DW-03S-X exposes those proofs through the existing privileged lab MCP endpoint (lab-mcp), and DW-03S-B joins them in the supervisor/dossier (lab). The dependency chain and exact tests/stop conditions are in `work-packets.md`. No runtime packet owns all three client, server and supervisor crates. One reviewer/fixer can cover the five-packet wave after each packet's own adversarial/domain review.

The join requires two different encrypted datagram fingerprints attributed to the same current server generation and an exact process/bridge epoch, plus a unique fresh roster row coupling that generation to player entity and world and a before/after client Unit.Player slot that agrees. Server boot change, relog, PID reuse, gate travel, duplicate candidate, cursor drop and stale target all produce an explicit unknown. Fingerprints are diagnostic correlation, not credentials; GM execution must still derive and check actor authorization at the server. The existing packet tap remains singly owned and is not consumed to establish this binding.

## Unresolved gates

1. **DW-03S-E static proof:** identify a common successful encrypted-send seam and its generation lifecycle. If no complete seam exists inside the scoped BaseApp crate, revise the contract and packet ownership before coding; do not quietly settle for reliable-only evidence.
2. **DLL observation:** issue #1703's missing live hook events and client destination peer remain unverified. Offline tests can validate parser/counters but cannot claim the hook fires in the actual SGW process.
3. **Live permission:** D-DW12 and existing D-DW9 require the owner's separate explicit approval before taking the lab lease or starting/restarting the client or changing its server. No approval has been given for this probe. DW-35 performs the live proof after approval.
4. **Authority:** the binder is a read correlation only. DW-30 still must derive the GM actor from the authenticated server-side session and recheck authority; neither a browser-supplied actor nor `BoundSession` is a credential.

Until those gates pass, the dossier and five-layer truth must keep unproven server fields `unknown` with a gap; packet PRs must not label live behavior verified.
