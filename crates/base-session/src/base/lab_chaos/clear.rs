//! The registry's clears: one session, every session and arm, one entity, and
//! the stale-entity check the flush makes. Clearing an arm by account is in
//! `armed.rs`.

use std::net::SocketAddr;
use std::sync::atomic::Ordering;

use super::LabChaos;

impl LabChaos {
    /// Clears one session, or every session and every pending arm (D-CH10)
    /// with `None`. Returns how many entries were removed, which the clear
    /// tool reports. A per-session clear (teardown, character select) leaves
    /// the arms alone: they wait for the account's next world entry. Logs
    /// INFO when it removed something or cleared everything; a per-session
    /// clear that found nothing (every session teardown on a lab server)
    /// logs DEBUG.
    pub fn clear(&self, addr: Option<SocketAddr>) -> usize {
        let cleared = match addr {
            Some(addr) => usize::from(self.sessions().remove(&addr).is_some()),
            None => {
                // Both locks at once, sessions first (the order a promotion
                // takes them in), so a promotion running alongside is wholly
                // before this clear or wholly after it: its arm or its entry
                // is here to clear, never neither.
                let mut sessions = self.sessions();
                let mut armed = self.armed();
                let n = sessions.len() + armed.len();
                sessions.clear();
                armed.clear();
                self.operator_clears.fetch_add(1, Ordering::Relaxed);
                n
            }
        };
        let addr_hash = addr.map(|a| self.addr_hash(a));
        let scope = if addr.is_some() { "session" } else { "all" };
        if cleared > 0 || addr.is_none() {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope,
                addr_hash = addr_hash.as_deref(),
                cleared,
                "lab chaos cleared"
            );
        } else {
            tracing::debug!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope,
                addr_hash = addr_hash.as_deref(),
                cleared,
                "lab chaos cleared (nothing set)"
            );
        }
        cleared
    }

    /// Clears every entry set for `entity_id`, wherever its session is now,
    /// and returns how many it removed. The clear tool's per-entity form: an
    /// entity that went back to character select has no session to resolve,
    /// but its tag is still on the entry status lists.
    pub fn clear_entity(&self, entity_id: u32) -> usize {
        let cleared = {
            let mut sessions = self.sessions();
            let before = sessions.len();
            sessions.retain(|_, chaos| chaos.entity_id != entity_id);
            self.operator_clears.fetch_add(1, Ordering::Relaxed);
            before - sessions.len()
        };
        // Like `clear`'s per-session form: INFO when something went, DEBUG
        // otherwise.
        if cleared > 0 {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "entity",
                cleared,
                "lab chaos cleared for an entity"
            );
        } else {
            tracing::debug!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "entity",
                cleared,
                "lab chaos cleared for an entity (nothing set)"
            );
        }
        cleared
    }

    /// Removes the entry at `addr` when it was set for another entity than
    /// `entity_id`, and says whether it did. A caller that knows which player
    /// entity now owns `addr` uses it to drop chaos a previous owner left
    /// behind (see the CH-04 worknote for CH-05 and CH-07).
    pub fn clear_if_not(&self, addr: SocketAddr, entity_id: u32) -> bool {
        let removed = {
            let mut sessions = self.sessions();
            let stale = sessions
                .get(&addr)
                .is_some_and(|chaos| chaos.entity_id != entity_id);
            if stale {
                sessions.remove(&addr);
            }
            stale
        };
        if removed {
            tracing::info!(
                target: "lab.chaos",
                event = "lab_chaos_cleared",
                scope = "stale_entity",
                addr_hash = %self.addr_hash(addr),
                cleared = 1,
                "lab chaos cleared: set for another entity on this address"
            );
        }
        removed
    }
}
