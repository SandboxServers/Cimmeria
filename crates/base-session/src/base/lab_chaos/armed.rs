//! Chaos armed for an account's next world entry (decision D-CH10, CH-14).
//!
//! Chaos set on a session is tagged with an in-world player entity, so it can
//! only be set after that login's first-login AoI flush, which the `flush`
//! profile exists to disturb. An arm is held by account id instead, one per
//! account (arming again replaces it), and is one-shot: the account's next
//! `playCharacter` moves it onto the session, tagged with the new player
//! entity and a new generation, before the client can send `onClientReady`
//! (`promote_armed`). It keeps the TTL it was armed with, so it expires on the
//! same clock whether or not it was promoted.
//!
//! Arms are not session chaos until promoted: the net shim never sees them,
//! and session teardown or a return to character select leaves them be (they
//! are for the account's next world entry, not its current session). A clear
//! of everything, a clear by account and expiry remove them. Logs name the
//! account by id only: the registry is never given the name.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{MutexGuard, PoisonError};
use std::time::Instant;

use super::{
    chaos_seed, check_request, profile_label, ChaosRefused, FlushChaos, LabChaos, NetChaos,
    SessionChaos,
};

/// Chaos waiting for an account's next world entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmedChaos {
    /// The account whose next world entry takes it. Status reports this.
    pub account_id: u32,
    /// As [`SessionChaos::label`].
    pub label: String,
    pub net: Option<NetChaos>,
    pub flush: Option<FlushChaos>,
    /// Fixed when armed; promotion keeps it.
    pub expires_at: Instant,
}

impl ArmedChaos {
    /// The session entry this arm becomes once its world entry is known.
    fn into_session(self, entity_id: u32, generation: u64) -> SessionChaos {
        SessionChaos {
            entity_id,
            label: self.label,
            net: self.net,
            flush: self.flush,
            expires_at: self.expires_at,
            generation,
        }
    }
}

impl LabChaos {
    /// Arms chaos for `account_id`'s next world entry and returns when it
    /// expires. The same checks as [`Self::set`]; a second arm for the
    /// account replaces the first. Expired arms are swept here.
    pub fn arm(
        &self,
        account_id: u32,
        label: String,
        net: Option<NetChaos>,
        flush: Option<FlushChaos>,
        ttl_s: u32,
        now: Instant,
    ) -> Result<Instant, ChaosRefused> {
        let expires_at = check_request(net.as_ref(), flush.as_ref(), ttl_s, now)?;
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_armed",
            profile = profile_label(net.as_ref(), flush.as_ref()),
            level = %label,
            account_id, // nt:id-only the chaos registry is never given the account name
            seed = chaos_seed(net.as_ref(), flush.as_ref()),
            expires_in_s = ttl_s,
            "lab chaos armed for an account's next world entry"
        );
        let armed = ArmedChaos {
            account_id,
            label,
            net,
            flush,
            expires_at,
        };
        let mut map = self.armed();
        sweep_expired_arms(&mut map, now);
        map.insert(account_id, armed);
        Ok(expires_at)
    }

    /// Removes the arm for `account_id`, if any, and returns how many it
    /// removed (0 or 1). Chaos already promoted onto a session is the
    /// entity's: clear that by entity.
    pub fn clear_account(&self, account_id: u32) -> usize {
        let cleared = usize::from(self.armed().remove(&account_id).is_some());
        if cleared > 0 {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "account",
                account_id, // nt:id-only the chaos registry is never given the account name
                cleared,
                "lab chaos arm cleared for an account"
            );
        } else {
            tracing::debug!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "account",
                account_id, // nt:id-only the chaos registry is never given the account name
                cleared,
                "lab chaos arm cleared for an account (nothing armed)"
            );
        }
        cleared
    }

    /// Takes the arm for `account_id` (one-shot) and sets it on the session
    /// at `addr`, tagged with `entity_id`, the player entity this world
    /// entry created. Replaces whatever that address had. Returns whether
    /// it promoted something; an arm past its TTL is dropped instead.
    ///
    /// The caller is world entry, after the new entity id and before the
    /// client can trigger the first flush (CH-14 worknote).
    pub fn promote_armed(
        &self,
        account_id: u32,
        addr: SocketAddr,
        entity_id: u32,
        now: Instant,
    ) -> bool {
        // Taken out under the arms lock alone; the session map is locked
        // after it is released, so the two locks never nest.
        let Some(armed) = self.armed().remove(&account_id) else {
            return false;
        };
        if now >= armed.expires_at {
            log_arm_expired(account_id);
            return false;
        }
        let expires_in_s = armed.expires_at.saturating_duration_since(now).as_secs();
        let chaos = armed.into_session(entity_id, self.next_generation());
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_promoted",
            profile = profile_label(chaos.net.as_ref(), chaos.flush.as_ref()),
            level = %chaos.label,
            account_id, // nt:id-only the chaos registry is never given the account name
            entity_id, // nt:id-only the chaos registry is never given the character name
            addr_hash = %self.addr_hash(addr),
            seed = chaos_seed(chaos.net.as_ref(), chaos.flush.as_ref()),
            expires_in_s,
            "lab chaos armed for the account set on its new session"
        );
        // Through the guard, so `has_net` counts a promoted net profile.
        let mut sessions = self.sessions();
        self.sweep_expired(&mut sessions, now);
        sessions.insert(addr, chaos);
        true
    }

    /// Every live arm, sorted by account id. Expired arms are removed.
    pub fn armed_status(&self, now: Instant) -> Vec<ArmedChaos> {
        let mut map = self.armed();
        sweep_expired_arms(&mut map, now);
        let mut live: Vec<ArmedChaos> = map.values().cloned().collect();
        live.sort_by_key(|a| a.account_id);
        live
    }

    /// As [`Self::sessions`]: every change is one insert or remove, so a
    /// poisoned lock is still safe to use.
    pub(super) fn armed(&self) -> MutexGuard<'_, HashMap<u32, ArmedChaos>> {
        self.armed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn sweep_expired_arms(map: &mut HashMap<u32, ArmedChaos>, now: Instant) {
    map.retain(|&account_id, armed| {
        let live = now < armed.expires_at;
        if !live {
            log_arm_expired(account_id);
        }
        live
    });
}

fn log_arm_expired(account_id: u32) {
    tracing::info!(
        target: "lab.chaos",
        event = "lab_chaos_expired",
        scope = "account",
        account_id, // nt:id-only the chaos registry is never given the account name
        "lab chaos arm expired before the account entered the world"
    );
}
