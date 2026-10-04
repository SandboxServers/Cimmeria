//! The seam between an interrupt effect and the cast it interrupts
//! (ability-mechanics AB-09c).
//!
//! An effect script is synchronous and lives below combat, so it cannot
//! reach the warmup table's async cancel (`interrupt_pending_cast` in
//! `cimmeria-cell-combat`, which sends the caster's timers and the
//! `Ability_Interrupt` sequence) or cancel a channel. It queues an
//! [`InterruptRequest`] here instead, and combat resolves the queue: right
//! after the script runs (`flush_stat_buff_timers`, which every caller of
//! a script already awaits) and, as a safety net, on the 100 ms stat-buff
//! tick. Combat rolls the target's interrupt resistance, then cancels its
//! warmup and its channels.

use crate::cell::space_manager::SpaceManager;

/// What asks for the interrupt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterruptCause {
    /// An interrupt effect ("Interrupts target", an EMP round): rolled
    /// against the target's interrupt resistance.
    #[default]
    Effect,
    /// A stun or knockdown landed: an incapacitated entity cannot keep
    /// casting, so it is never rolled.
    Incapacitated,
}

/// One queued interrupt: `source_id`'s effect asks to break `target_id`'s
/// cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterruptRequest {
    /// The entity whose effect interrupts (the actor).
    pub source_id: u32,
    /// The entity whose warmup and channels it breaks (the subject).
    pub target_id: u32,
    /// The interrupting effect.
    pub effect_id: i32,
    /// The ability the effect belongs to.
    pub ability_id: i32,
    /// The effect's own chance to interrupt, in percent (100 = certain),
    /// before the target's resistance.
    pub chance_pct: i32,
    /// Why it was queued.
    pub cause: InterruptCause,
    /// Unique per request, set by [`SpaceManager::request_interrupt`]: the
    /// resistance roll's seed includes it, so two attempts on one cast (or
    /// on a channel, which has no warmup instance) roll independently.
    pub nonce: u64,
    /// The interrupting cast (AB-T1): the actor's `cast_id`, stamped from the
    /// cast scope by [`SpaceManager::request_interrupt`]. `None` outside a
    /// cast (a stun from a pulse whose instance had none).
    pub cast_id: Option<i32>,
}

impl SpaceManager {
    /// Queue an interrupt for combat to resolve.
    pub fn request_interrupt(&mut self, mut request: InterruptRequest) {
        self.interrupt_nonce = self.interrupt_nonce.wrapping_add(1);
        request.nonce = self.interrupt_nonce;
        request.cast_id = self.current_cast_id();
        self.pending_interrupts.push(request);
    }

    /// Take the queued interrupts aimed at `target_id`, in queue order.
    pub fn take_interrupt_requests_for(&mut self, target_id: u32) -> Vec<InterruptRequest> {
        if self.pending_interrupts.is_empty() {
            return Vec::new();
        }
        let (mine, rest) = std::mem::take(&mut self.pending_interrupts)
            .into_iter()
            .partition(|r| r.target_id == target_id);
        self.pending_interrupts = rest;
        mine
    }

    /// Take every queued interrupt.
    pub fn take_all_interrupt_requests(&mut self) -> Vec<InterruptRequest> {
        std::mem::take(&mut self.pending_interrupts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(target_id: u32) -> InterruptRequest {
        InterruptRequest {
            source_id: 1,
            target_id,
            effect_id: 723,
            ability_id: 657,
            chance_pct: 100,
            cause: InterruptCause::Effect,
            nonce: 0,
            cast_id: None,
        }
    }

    #[test]
    fn requests_are_taken_per_target_and_then_all() {
        let mut mgr = SpaceManager::new(1);
        mgr.request_interrupt(req(2));
        mgr.request_interrupt(req(3));
        mgr.request_interrupt(req(2));
        let mine = mgr.take_interrupt_requests_for(2);
        assert_eq!(mine.iter().map(|r| r.target_id).collect::<Vec<_>>(), [2, 2]);
        assert_ne!(
            mine[0].nonce, mine[1].nonce,
            "each request has its own nonce"
        );
        let rest = mgr.take_all_interrupt_requests();
        assert_eq!(rest.iter().map(|r| r.target_id).collect::<Vec<_>>(), [3]);
        assert!(mgr.take_all_interrupt_requests().is_empty());
    }
}
