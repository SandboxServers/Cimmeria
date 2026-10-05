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
//! # What may take an occupied address over (security)
//!
//! A fresh ticket is routed to [`super::handle_login`], which asks
//! [`address_claim`] what the session already on the address is:
//!
//! 1. The datagram is a well-formed plaintext `baseAppLogin` that does
//!    **not** decrypt under the live session's key (a real channel packet
//!    is never treated as a login).
//! 2. Its ticket is unconsumed in `pending_logins` and younger than
//!    `TICKET_TTL` (30 s). Only the login server fills that map, after the
//!    account's password check, with a single-use random 20-character
//!    ticket.
//! 3. The live session is the **same account**: a relaunch. The old
//!    session is evicted ([`AddressClaim::SameAccount`]).
//! 4. The live session is **another account** whose own ticket was issued
//!    to a different IP than the address it registered from, and the
//!    incoming ticket was issued to this address's IP: a squatter. The
//!    squatter is evicted ([`AddressClaim::Squatter`]).
//! 5. Anything else is refused: the ticket is burned and the live session
//!    stays up ([`AddressClaim::Refused`]).
//!
//! Rule 3 stops a spoofed source address from kicking a **live** player.
//! Anyone can spoof a live player's address:port, and anyone with an
//! account can get a ticket for their own account. Without rule 3 that pair
//! would let any account holder kick any player whose address they know.
//! With it, the attacker needs a fresh ticket for the victim's own account,
//! which takes the victim's password. Holding that already lets them evict
//! the victim from any address through the duplicate-login path
//! (`eviction.rs`), so the takeover adds no new power.
//!
//! Rule 3 alone does not protect a **free** address. An attacker can spoof
//! the victim's address:port while no session holds it (before the
//! victim's login, or after a reap) and register a session there with a
//! ticket for their own account. The SOAP request behind that ticket is
//! TCP, which cannot be spoofed, so the ticket carries the attacker's real
//! IP, and the registration is logged as `ticket_ip_mismatch` (#442, still
//! warn-only). Rule 4 is what keeps such a squatter from locking the victim
//! out: the victim's own ticket was issued to the victim's IP, so the
//! victim's login evicts the squatter instead of being refused. Rule 4 never
//! evicts a session whose ticket matched its address, so it cannot be
//! turned against a player who registered normally. A session behind
//! carrier-grade NAT can register with a mismatched ticket IP legitimately;
//! another account's ticket issued to that same public IP can evict it,
//! which needs a second client behind the same NAT.
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

use cimmeria_base_session::base::plugin::SessionExtensions;
use cimmeria_mercury::encryption::MercuryEncryption;

use crate::auth::PendingLogin;

use super::super::session_identity;
use super::super::ConnectedClientState;
use super::parse_baseapp_login;

/// Stable `reason` for a fresh ticket refused on an established channel.
pub(crate) const REASON_RELAUNCH_ACCOUNT_MISMATCH: &str = "relaunch_account_mismatch";

/// Stable `reason` (and `disconnect_reason`) for a squatter evicted by the
/// address's rightful owner (rule 4 in the module doc).
pub(crate) const REASON_ADDRESS_RECLAIMED: &str = "address_reclaimed";

/// Whether the ticket a session registered with was issued to the IP the
/// session registered from. Stored in the session's extensions at Phase 3;
/// a session without one (no Phase 3 login, such as a test fixture) counts
/// as matched, so rule 4 never evicts it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TicketIpBinding {
    pub(crate) matched: bool,
}

/// The extensions a newly registered session starts with: its
/// [`TicketIpBinding`].
pub(crate) fn binding_extensions(ticket_ip_matched: bool) -> SessionExtensions {
    let mut extensions = SessionExtensions::default();
    extensions.insert(TicketIpBinding {
        matched: ticket_ip_matched,
    });
    extensions
}

/// `true` unless the session registered with a ticket issued to another IP.
fn registered_with_matching_ip(c: &ConnectedClientState) -> bool {
    c.extensions
        .get::<TicketIpBinding>()
        .is_none_or(|b| b.matched)
}

/// What a fresh login finds on its address. See the module doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AddressClaim {
    /// No session on the address.
    Free,
    /// A session of the login's own account: a relaunch.
    SameAccount,
    /// Another account's session that registered with a mismatched ticket
    /// IP, and the login's ticket was issued to this address's IP.
    Squatter,
    /// Another account's session the login may not displace (or the
    /// `connected` lock is poisoned). Already logged.
    Refused,
}

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

/// Classify the session at `addr` for a fresh `login` from `addr`. Logs
/// the refusal and the squatter eviction (one WARN each; the caller burns
/// a refused ticket, so a replay cannot repeat the row).
pub(crate) fn address_claim(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
    login: &PendingLogin,
) -> AddressClaim {
    let (live, live_ip_matched) = {
        let Ok(clients) = connected.lock() else {
            tracing::error!(
                %addr,
                ticket_account_id = login.account_id,
                ticket_account_name = %login.account_name,
                reason = REASON_RELAUNCH_ACCOUNT_MISMATCH,
                "connected lock poisoned: refusing the login rather than risk displacing a live session"
            );
            return AddressClaim::Refused;
        };
        let Some(c) = clients.get(&addr) else {
            return AddressClaim::Free;
        };
        if c.account_id == login.account_id {
            return AddressClaim::SameAccount;
        }
        (
            session_identity::session_identity(c),
            registered_with_matching_ip(c),
        )
    };
    let ticket_ip_matches = crate::auth::client_ips_match(login.client_ip, addr.ip());
    if !live_ip_matched && ticket_ip_matches {
        tracing::warn!(
            %addr,
            account_id = live.account_id,
            account_name = live.account_name,
            player_id = live.player_id,
            player_name = live.player_name,
            ticket_account_id = login.account_id,
            ticket_account_name = %login.account_name,
            reason = REASON_ADDRESS_RECLAIMED,
            "The session on this address registered with a ticket issued to another IP; \
             a login whose ticket was issued to this IP reclaims the address and evicts it"
        );
        return AddressClaim::Squatter;
    }
    tracing::warn!(
        %addr,
        account_id = live.account_id,
        account_name = live.account_name,
        player_id = live.player_id,
        player_name = live.player_name,
        ticket_account_id = login.account_id,
        ticket_account_name = %login.account_name,
        ticket_ip_matches,
        reason = REASON_RELAUNCH_ACCOUNT_MISMATCH,
        "baseAppLogin for a different account arrived on an established channel; \
         refusing the takeover and burning the ticket (the live session stays up)"
    );
    AddressClaim::Refused
}
