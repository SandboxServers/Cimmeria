---
name: relaunch-takeover-gate
description: How a relaunched SGW client (fixed UDP port 63888, same addr:port) takes over its dead session; the three-part auth gate and why key-proof is not required. Read before touching handle_datagram routing, handle_login eviction or last_recv.
metadata:
  type: project
---

Fixed 2026-10-04 (branch fix/relaunch-login-on-established-channel). The SGW client binds a fixed UDP port, so a crash+relaunch arrives on the same addr:port as the still-registered session; `handle_datagram` routes every datagram from a registered addr to the encrypted path, so the new plaintext baseAppLogin used to be dropped as `login_retry_on_channel` for up to 60 s.

Takeover gate (`crates/base/src/base/login/relaunch.rs`): (1) parses as plaintext baseAppLogin AND fails to decrypt under the live key; (2) ticket still unconsumed in `pending_logins` (tickets are single-use, so a retransmit of the channel's own login can never pass); (3) ticket account == live session account, else refused + ticket burned (`relaunch_account_mismatch`). Rule 3 is the anti-spoof control: any account holder can mint a ticket for their own account and spoof a victim's addr:port.

**Why no key-proof before teardown:** Phase 3 is plaintext and every channel registers on the ticket alone; ticket and key arrive in the same SOAP reply, so proving the key adds nothing over holding the ticket. Same-account ticket already lets you evict via the duplicate-login path from any address.

**How to apply:** eviction lives in `login/eviction.rs` (`evict_prior_sessions`: other addr = LOGGED_OFF + `duplicate_login`; same addr = no LOGGED_OFF + `relaunch_takeover`). Tick loop teardown is owner-checked (`destroy_owned_client_entities`, ptr_eq on the session's `cancelled` Arc) so a stale loop cannot destroy the replacement session. `last_recv` refreshes only after a successful decrypt (`touch_last_recv` in `connect_loop/encrypted/mod.rs`). Any new teardown path keyed by addr must consider that the addr may now hold a different session. See [[security-audit-2026-05-31]].
