//! Chaos armed for an account's next world entry (decision D-CH10, CH-14).
//!
//! Chaos set on a session is tagged with an in-world player entity, so it can
//! only be set after that login's first-login AoI flush, which the `flush`
//! profile exists to disturb. An arm is held by account id instead, one per
//! account (arming again replaces it), and is one-shot: the account's next
//! `playCharacter` moves it onto the session, tagged with the new player
//! entity and a new generation, before the client can send `onClientReady`
//! (`promote_armed`). It keeps the TTL it was armed with, so it expires on the
//! same clock whether or not it was promoted. A world entry that fails after
//! the promotion puts the arm back (`rearm`).
//!
//! Arms are not session chaos until promoted: the net shim never sees them,
//! and session teardown or a return to character select leaves them be (they
//! are for the account's next world entry, not its current session). A clear
//! of everything, a clear by account and expiry remove them. Logs name the
//! account by id only: the registry is never given the name.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
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
    /// The net profile, shaping the session's sends once promoted. Promotion
    /// is at `playCharacter`, before RESET_ENTITIES: authentication, the
    /// character list and `playCharacter` itself are never shaped by an arm.
    pub net: Option<NetChaos>,
    /// The flush profile, read by that login's first flush and every later
    /// one on the session until the entry expires or is cleared.
    pub flush: Option<FlushChaos>,
    /// Fixed when armed; promotion keeps it.
    pub expires_at: Instant,
}

/// What [`LabChaos::promote_armed`] took and set, so a world entry that
/// fails after promoting can put the arm back with [`LabChaos::rearm`].
#[derive(Debug, Clone, PartialEq)]
pub struct Promotion {
    /// The session address the entry was set at.
    pub(super) addr: SocketAddr,
    /// The promoted entry's generation: `rearm` removes that entry only.
    pub(super) generation: u64,
    /// The arm as it was taken, same knobs and `expires_at`.
    pub(super) armed: ArmedChaos,
    /// `operator_clears` when it was taken.
    pub(super) clears: u64,
}

impl Promotion {
    /// The account the arm was for.
    pub fn account_id(&self) -> u32 {
        self.armed.account_id
    }
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
        let cleared = {
            let mut armed = self.armed();
            self.operator_clears.fetch_add(1, Ordering::Relaxed);
            usize::from(armed.remove(&account_id).is_some())
        };
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
    /// entry created. Replaces whatever that address had. Returns
    /// what it promoted, for [`Self::rearm`]; an arm past its TTL is dropped
    /// instead.
    ///
    /// The caller is world entry, after the new entity id and before the
    /// client can trigger the first flush, and only while `addr` is still
    /// the session that entry is for (CH-14 worknote).
    pub fn promote_armed(
        &self,
        account_id: u32,
        addr: SocketAddr,
        entity_id: u32,
        now: Instant,
    ) -> Option<Promotion> {
        // Sessions, then arms: the order a clear of everything takes them
        // in. The take and the insert are one step to that clear, which so
        // finds the arm or the promoted entry, never neither. Through the
        // guard, so `has_net` counts a promoted net profile.
        let mut sessions = self.sessions();
        let (armed, clears) = {
            let mut arms = self.armed();
            let armed = arms.remove(&account_id)?;
            (armed, self.operator_clears.load(Ordering::Relaxed))
        };
        if now >= armed.expires_at {
            drop(sessions);
            log_arm_expired(account_id);
            return None;
        }
        let generation = self.next_generation();
        self.sweep_expired(&mut sessions, now);
        sessions.insert(addr, armed.clone().into_session(entity_id, generation));
        drop(sessions);
        tracing::warn!(
            target: "lab.chaos",
            event = "lab_chaos_promoted",
            profile = profile_label(armed.net.as_ref(), armed.flush.as_ref()),
            level = %armed.label,
            account_id, // nt:id-only the chaos registry is never given the account name
            entity_id, // nt:id-only the chaos registry is never given the character name
            addr_hash = %self.addr_hash(addr),
            seed = chaos_seed(armed.net.as_ref(), armed.flush.as_ref()),
            expires_in_s = armed.expires_at.saturating_duration_since(now).as_secs(),
            "lab chaos armed for the account set on its new session"
        );
        Some(Promotion {
            addr,
            generation,
            armed,
            clears,
        })
    }

    /// Undoes `promotion` for a world entry that failed after it: removes
    /// the entry it set (if that entry is still there) and arms the account
    /// again with the same knobs and `expires_at`, so its next real login
    /// still gets the chaos. No arm comes back once the arm has expired,
    /// after an operator clear since the promotion, or when the account was
    /// armed again meanwhile (the newer arm stays). Returns whether it
    /// re-armed.
    pub fn rearm(&self, promotion: Promotion, now: Instant) -> bool {
        let Promotion {
            addr,
            generation,
            armed,
            clears,
        } = promotion;
        let account_id = armed.account_id;
        let skipped = {
            let mut sessions = self.sessions();
            if sessions
                .get(&addr)
                .is_some_and(|c| c.generation == generation)
            {
                sessions.remove(&addr);
            }
            let mut arms = self.armed();
            if self.operator_clears.load(Ordering::Relaxed) != clears {
                Some("cleared_since")
            } else if now >= armed.expires_at {
                Some("expired")
            } else {
                match arms.entry(account_id) {
                    Entry::Occupied(_) => Some("armed_again"),
                    Entry::Vacant(slot) => {
                        slot.insert(armed);
                        None
                    }
                }
            }
        };
        match skipped {
            None => tracing::warn!(
                target: "lab.chaos",
                event = "lab_chaos_rearmed",
                account_id, // nt:id-only the chaos registry is never given the account name
                addr_hash = %self.addr_hash(addr),
                "lab chaos arm put back: the world entry failed after taking it"
            ),
            Some(reason) => tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_rearmed",
                account_id, // nt:id-only the chaos registry is never given the account name
                addr_hash = %self.addr_hash(addr),
                reason,
                "lab chaos arm not put back after a failed world entry"
            ),
        }
        skipped.is_none()
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
