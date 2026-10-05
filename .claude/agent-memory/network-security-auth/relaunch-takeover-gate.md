---
name: relaunch-takeover-gate
description: How a relaunched SGW client (fixed UDP port 63888, same addr:port) takes over its dead session; the address-claim rules (same account / squatter reclaim / refuse), liveness = fresh traffic only, and the addr-keyed late-writer rule. Read before touching handle_datagram routing, handle_login eviction, last_recv or any task that resolves addr then awaits.
metadata:
  type: project
---

Fixed 2026-10-04 (PR #1246). The SGW client binds a fixed UDP port, so a crash+relaunch arrives on the same addr:port as the still-registered session; `handle_datagram` routes every datagram from a registered addr to the encrypted path, so the new plaintext baseAppLogin used to be dropped as `login_retry_on_channel` for up to 60 s.

Routing (`crates/base/src/base/login/relaunch.rs`): plaintext baseAppLogin that fails to decrypt under the live key AND whose ticket is unconsumed (single-use, so a retransmit never passes) goes to `handle_login`. There: ticket age < `TICKET_TTL` (30 s, now `pub` in cimmeria-auth), then `address_claim`: same account = relaunch (evict); other account whose `TicketIpBinding.matched == false` while the new ticket's IP matches addr = squatter (evict, `address_reclaimed`); else refuse + burn (`relaunch_account_mismatch`); poisoned lock refuses. `TicketIpBinding` lives in `ConnectedClientState.extensions` (type map) to avoid touching ~20 struct literals.

**Why same-account is the anti-spoof control:** any account holder can mint a ticket for their own account and spoof a live victim's addr:port. Rule 3 does NOT protect a free addr (squat): that is what the IP-binding reclaim is for. No key-proof before teardown: Phase 3 is plaintext, ticket and key arrive in the same SOAP reply.

**How to apply:**
- Liveness (`connect_loop/encrypted/liveness.rs`): `last_recv` refreshes only for gate-new reliable (InOrder/Buffered), ACKs that retire TX entries, or an unreliable seq advancing past the session's high-water mark (client uses a separate unreliable counter). HMAC-valid != fresh; replays must not keep sessions alive.
- Any task that resolves `addr` (or eid->addr), awaits, then touches `connected[addr]` must re-verify ownership (`cancelled` Arc ptr_eq, or `player_entity_id == Some(eid)`). Done for: tick loop teardown (`destroy_owned_client_entities`) + its retransmit scan, gate travel (`gate_travel/session_owner.rs`), cinematic loop (teardown sets `cinematic_spam_cancel`).
- Eviction in `login/eviction.rs`: other addr = LOGGED_OFF + `duplicate_login`; same addr = no LOGGED_OFF + `relaunch_takeover` / `address_reclaimed`.
See [[security-audit-2026-05-31]] and the reviewer's [[exploit-address-reuse-session-takeover]].
