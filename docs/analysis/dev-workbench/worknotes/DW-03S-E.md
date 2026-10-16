# DW-03S-E worknote

Static proof-path reconnaissance only; no lab lease, client start, network probe, or cargo build.

## Outcome

- Traced client Winsock receive event through bounded DLL ring and both supervisor drain paths. Exact encrypted receive fingerprint exists; injection bridge epoch and ingestion-time process identity do not.
- Traced `Unit.Player` from Lua slot constant to numeric entity through the slot map.
- Traced Phase 3 insertion, shared transport projection, reliable/unreliable/direct/Cell fan-out, and retransmit collection. `mercury.reliable_send` is a debug registration, not complete successful-send coverage.
- Traced current `server_sessions` roster and gate travel's delayed world mutation. The roster can remain in the origin world with unchanged status during an in-progress transfer; the downstream roster/binder needs an authoritative transition epoch/state or must fail closed. No server boot/generation is exposed yet.
- Wrote synthetic fixture cases with source-local time anchors and explicit expected gaps in `evidence/DW-03S-E.md`.

## Blocking finding for next packet

`Transport::send_to(bytes,addr)` in `crates/mercury/src/transport.rs` is the complete post-send byte seam, but its signature has no `SendOrigin`. `BaseService` passes trait-object clones to handlers across `base-session` and `base-world-entry`; retransmit collection returns only bytes. A BaseService-only wrapper that looks up peer after await can relabel a paused old-generation send as a same-peer replacement. **DW-03S-S is BlockedDependency as currently owned.** Revise packet ownership/design to carry an immutable origin ticket through all included sends, or explicitly narrow proof coverage before coding. Do not claim a live hook or binding from this static map.

## Check

`git diff --check` on the two owned docs; no cargo for docs-only work. Independent game-archaeology and network-security reviews remain the coordinator's gate.
