//! The cast being resolved, for telemetry correlation (ability-mechanics
//! AB-T1).
//!
//! A cast's `cast_id` is the `effect_seq` its launch minted. The launch and
//! fire code hold it as a local, but everything a cast sets off below them
//! (an effect script, the timed effect ledger, a pulsing instance, an
//! interrupt request) only sees the `SpaceManager`. Threading an id through
//! every script signature would touch every effect, so the fire opens a
//! scope instead: [`SpaceManager::enter_cast_scope`] before it resolves,
//! [`SpaceManager::exit_cast_scope`] after, and whatever is created in
//! between stamps [`SpaceManager::current_cast_id`] on itself. Rows emitted
//! later, outside any scope (a pulse, an expiry), read the stamp.
//!
//! Scopes nest: a content chain fired from inside a cast can launch another
//! entity's cast, which enters its own scope and restores the outer one on
//! exit. The cell runs on one task with the `SpaceManager` borrowed `&mut`
//! across every await, so no other code can observe a scope mid-cast.
//!
//! A `cast_id` is unique per caster (it is the caster's own counter), so the
//! join key is `(caster entity_id, cast_id)`.

use cimmeria_entity::cell_entity::PlayerIdentity;

use crate::cell::space_manager::SpaceManager;

impl SpaceManager {
    /// Make `cast_id` the resolving cast and return the one it replaces,
    /// which the caller hands back to [`SpaceManager::exit_cast_scope`].
    /// `None` resolves outside any cast (a pulse whose instance has no cast).
    pub fn enter_cast_scope(&mut self, cast_id: Option<i32>) -> Option<i32> {
        std::mem::replace(&mut self.current_cast_id, cast_id)
    }

    /// Restore the scope [`SpaceManager::enter_cast_scope`] replaced.
    pub fn exit_cast_scope(&mut self, previous: Option<i32>) {
        self.current_cast_id = previous;
    }

    /// The cast being resolved, if any.
    pub fn current_cast_id(&self) -> Option<i32> {
        self.current_cast_id
    }

    /// Resolve a deferred effect (a pulse, a natural end, a channel cancel)
    /// under the cast and invoker identity its instance snapshotted. The
    /// cast scope of the launch is long closed by then, and the invoker may
    /// have disconnected and its entity id been recycled, so the effect rows
    /// read these snapshots, never a lookup. Returns the scope it replaces,
    /// for [`SpaceManager::exit_effect_scope`].
    pub fn enter_effect_scope(
        &mut self,
        cast_id: Option<i32>,
        invoker_id: u32,
        invoker_identity: PlayerIdentity,
    ) -> EffectScope {
        EffectScope {
            cast_id: std::mem::replace(&mut self.current_cast_id, cast_id),
            invoker: self.current_invoker.replace((invoker_id, invoker_identity)),
        }
    }

    /// Restore the scope [`SpaceManager::enter_effect_scope`] replaced.
    pub fn exit_effect_scope(&mut self, previous: EffectScope) {
        self.current_cast_id = previous.cast_id;
        self.current_invoker = previous.invoker;
    }

    /// The identity a row should name for the effect's caster `entity_id`:
    /// the deferred effect's snapshot when it is that effect's invoker,
    /// otherwise the live lookup.
    pub fn caster_identity(&self, entity_id: u32) -> PlayerIdentity {
        match self.current_invoker {
            Some((id, who)) if id == entity_id => who,
            _ => self.player_identity(entity_id),
        }
    }
}

/// What [`SpaceManager::enter_effect_scope`] replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectScope {
    cast_id: Option<i32>,
    invoker: Option<(u32, PlayerIdentity)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes_nest_and_restore() {
        let mut mgr = SpaceManager::new(1);
        assert_eq!(mgr.current_cast_id(), None);
        let outer = mgr.enter_cast_scope(Some(7));
        let inner = mgr.enter_cast_scope(Some(3));
        assert_eq!(mgr.current_cast_id(), Some(3));
        mgr.exit_cast_scope(inner);
        assert_eq!(mgr.current_cast_id(), Some(7), "the outer cast is back");
        mgr.exit_cast_scope(outer);
        assert_eq!(mgr.current_cast_id(), None);
    }

    /// A deferred effect names its snapshotted cast and invoker; the entity
    /// id it names is gone (or recycled), so a lookup would find nobody, or
    /// someone else. Exiting restores the outer scope.
    #[test]
    fn an_effect_scope_names_the_snapshot_not_the_live_entity() {
        let mut mgr = SpaceManager::new(1);
        let snap = PlayerIdentity::new(Some(10), Some(71));
        assert_eq!(mgr.caster_identity(2), PlayerIdentity::UNKNOWN);
        let outer = mgr.enter_cast_scope(Some(9));
        let prev = mgr.enter_effect_scope(Some(1), 2, snap);
        assert_eq!(mgr.current_cast_id(), Some(1));
        assert_eq!(mgr.caster_identity(2), snap);
        assert_eq!(
            mgr.caster_identity(3),
            PlayerIdentity::UNKNOWN,
            "only the invoker reads the snapshot"
        );
        mgr.exit_effect_scope(prev);
        assert_eq!(mgr.current_cast_id(), Some(9));
        assert_eq!(mgr.caster_identity(2), PlayerIdentity::UNKNOWN);
        mgr.exit_cast_scope(outer);
    }
}
