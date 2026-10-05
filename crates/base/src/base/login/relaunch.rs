//! A client relaunched on the address:port of its own live session.
//!
//! The SGW client binds a fixed UDP port (63888 on the colo), so a client
//! that is killed and relaunched comes back on the **same** address:port
//! as the session it left behind. That session stays registered until the
//! 60 s inactivity reap, and every datagram from the address is routed to
//! it. Before this module, the relaunched client's plaintext
//! `baseAppLogin` therefore landed on the old channel, failed to decrypt
//! and was dropped as a `login_retry_on_channel`, while the old session's
//! tick-sync loop kept sending the new client packets encrypted with the
//! old key (the client logs each as "Dropped corrupted incoming packet").
//! The player sat at "Logging in..." until the old channel timed out
//! (colo, release v2026-10-05.1, found by the DA-06 lab run).
//!
//! # Telling a relaunch from a retransmit
//!
//! The client re-sends its `baseAppLogin` every 300 ms until its reply
//! handler finishes (`decrypt_reject` module doc), so a plaintext login on
//! an established channel is usually a retransmit of the login that
//! created the channel. Its ticket was consumed when the channel was
//! registered: tickets are single-use and leave `pending_logins` on first
//! use. A relaunched client went back through SOAP login and carries a
//! **fresh, unconsumed** ticket. So "the ticket is still in
//! `pending_logins`" is the whole test, and a retransmit can never pass it.
//!
//! # What the takeover requires (security)
//!
//! A fresh ticket is routed to [`super::handle_login`], which tears the
//! old session down only when all of these hold:
//!
//! 1. The datagram is a well-formed plaintext `baseAppLogin` that does
//!    **not** decrypt under the live session's key (a real channel packet
//!    is never treated as a login).
//! 2. Its ticket is unconsumed in `pending_logins`, which only the login
//!    server fills, after the account's password check, with a single-use
//!    random 20-character ticket (30 s TTL).
//! 3. The ticket's account is the live session's account
//!    ([`cross_account_refusal`]). A ticket for any other account is
//!    refused, burned, and the live session stays up.
//!
//! Rule 3 is what makes a spoofed source address harmless. Anyone can
//! spoof a live player's address:port, and anyone with an account can get
//! a ticket for **their own** account. Without rule 3 that pair would let
//! any account holder kick any player whose address they know. With it,
//! the attacker needs a fresh ticket for the victim's own account, which
//! takes the victim's password. Holding that already lets them evict the
//! victim from any address through the duplicate-login path
//! (`eviction.rs`), so the takeover adds no new power.
//!
//! The new client cannot prove it holds the new session key before the
//! takeover: Phase 3 is plaintext by design, and the server registers
//! every new channel on the ticket alone. Waiting for a first datagram
//! that decrypts under the new key would add nothing here, because the
//! ticket and the key come from the same SOAP reply, so whoever has one
//! has the other.
//!
//! No extra rate limit: every takeover consumes a ticket, and each ticket
//! costs a full SOAP login. A refused ticket is burned too, so a spoofer
//! replaying one gets a single WARN row, not a row per datagram.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::MercuryEncryption;

use crate::auth::PendingLogin;

use super::super::session_identity;
use super::super::ConnectedClientState;
use super::parse_baseapp_login;

/// Stable `reason` for a fresh ticket from another account refused on an
/// established channel.
pub(crate) const REASON_RELAUNCH_ACCOUNT_MISMATCH: &str = "relaunch_account_mismatch";

/// The `(request_id, ticket)` of a plaintext `baseAppLogin` on an
/// established channel that carries a fresh, unconsumed ticket: a
/// relaunched client, not a retransmit. `None` for anything else,
/// which goes down the normal encrypted path.
///
/// `enc` is the live session's cipher. The datagram must fail to decrypt
/// under it, so a channel packet that happens to parse as a login is
/// never rerouted.
pub(crate) fn fresh_login_on_channel(
    raw: &[u8],
    enc: &MercuryEncryption,
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
) -> Option<(u32, String)> {
    let (request_id, ticket) = parse_baseapp_login(raw).ok()?;
    if enc.decrypt(raw).is_ok() {
        return None;
    }
    let unconsumed = pending_logins.lock().ok()?.contains_key(&ticket);
    unconsumed.then_some((request_id, ticket))
}

/// Why a fresh login may not take over the session at `addr`, or `None`
/// when it may (no session there, or a session of the same account).
///
/// Logs the refusal: one WARN per refused ticket, since the caller burns
/// the ticket.
pub(crate) fn cross_account_refusal(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    login: &PendingLogin,
) -> Option<&'static str> {
    let live = {
        let clients = connected.lock().ok()?;
        let c = clients.get(&addr)?;
        if c.account_id == login.account_id {
            return None;
        }
        session_identity::session_identity(c)
    };
    tracing::warn!(
        %addr,
        account_id = live.account_id,
        account_name = live.account_name,
        player_id = live.player_id,
        player_name = live.player_name,
        ticket_account_id = login.account_id,
        ticket_account_name = %login.account_name,
        reason = REASON_RELAUNCH_ACCOUNT_MISMATCH,
        "baseAppLogin for a different account arrived on an established channel; \
         refusing the takeover and burning the ticket (the live session stays up)"
    );
    Some(REASON_RELAUNCH_ACCOUNT_MISMATCH)
}
