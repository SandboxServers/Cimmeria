//! Deployable teardown: the observer-visible despawn and the per-tick
//! verdict that decides when a deployable must go.
//!
//! A deployable lives exactly as long as its lifetime and its owner's hold
//! on it. The verdict, run every AoI tick by the pulse tick
//! (`cimmeria-cell-combat`'s `abilities::deployable_tick`), removes it when:
//!
//! - its last pulse has run (`expired`);
//! - its owner is gone, or the owner id now belongs to someone else
//!   (`owner_gone`: logout, disconnect, gate travel, any destroy);
//! - its owner is in another space (`owner_left_space`: zone change,
//!   respawn elsewhere, space transfer);
//! - its owner is dead (`owner_dead`).
//!
//! A re-cast past the ability's `max_active` removes the owner's oldest one
//! (`replaced`), from the fire. One sweep covers every owner path, so no
//! owner teardown site has to remember deployables, and a deployable never
//! outlives its owner's hold by more than one 100 ms tick. The verdict runs
//! before the pulse in the same tick, so a dead or departed owner's
//! deployable never pulses again.
//!
//! Despawn goes through `despawn_npc`, never bare `destroy_entity`: the
//! former sends `LeftAoI` to every witness and scrubs the witness sets.

use std::time::Instant;

use cimmeria_common::EntityId;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::state_field::BSF_DEAD;
use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::{DespawnOutcome, SpaceManager};

/// Why a deployable was removed: the `reason` of the
/// `deployables.lifecycle` `event = "despawned"` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployableDespawnReason {
    /// Its last pulse ran.
    Expired,
    /// A re-cast by its owner replaced it.
    Replaced,
    /// Its owner no longer exists, or the id belongs to another player.
    OwnerGone,
    /// Its owner is in another space.
    OwnerLeftSpace,
    /// Its owner died.
    OwnerDead,
}

impl DeployableDespawnReason {
    /// Stable `reason` value for logs.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Expired => "expired",
            Self::Replaced => "replaced",
            Self::OwnerGone => "owner_gone",
            Self::OwnerLeftSpace => "owner_left_space",
            Self::OwnerDead => "owner_dead",
        }
    }

    /// Whether the owner's own client view is being torn down on this path
    /// (a traveller gets `RESET_ENTITIES`), so the owner must not be sent a
    /// `LeftAoI` that would reach its next world. Same rule as pets.
    pub fn owner_view_torn_down(self) -> bool {
        matches!(self, Self::OwnerGone | Self::OwnerLeftSpace)
    }
}

/// What the pulse tick should do about one deployable at `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeployableVerdict {
    /// Nothing is due.
    Hold,
    /// A pulse is due.
    Pulse,
    /// The deployable must go.
    Despawn(DeployableDespawnReason),
    /// The entity is already gone; only the registry entry is left.
    Scrub,
}

/// The verdict for `deployable` at `now`. Pure: reads only.
pub fn deployable_verdict(
    space_mgr: &SpaceManager,
    deployable: u32,
    now: Instant,
) -> DeployableVerdict {
    let Some(state) = space_mgr.deployables.get(deployable) else {
        return DeployableVerdict::Hold;
    };
    let Some(space) = space_mgr.get_entity_space_id(deployable) else {
        return DeployableVerdict::Scrub;
    };
    if !owner_holds(space_mgr, state.owner, state.owner_identity) {
        return DeployableVerdict::Despawn(DeployableDespawnReason::OwnerGone);
    }
    if space_mgr.get_entity_space_id(state.owner) != Some(space) {
        return DeployableVerdict::Despawn(DeployableDespawnReason::OwnerLeftSpace);
    }
    // The state bit, not HEALTH: a player corpse can be healed during the
    // Defeat Window.
    if space_mgr
        .get_entity(state.owner)
        .is_some_and(|o| o.state_field & BSF_DEAD != 0)
    {
        return DeployableVerdict::Despawn(DeployableDespawnReason::OwnerDead);
    }
    if state.is_spent() {
        return DeployableVerdict::Despawn(DeployableDespawnReason::Expired);
    }
    if now >= state.next_pulse_at {
        DeployableVerdict::Pulse
    } else {
        DeployableVerdict::Hold
    }
}

/// True when the entity at `owner` is the player that placed the
/// deployable: a player, with the identity captured at placement. Entity
/// ids are reused after a logout, so the id alone is never enough.
fn owner_holds(space_mgr: &SpaceManager, owner: u32, identity: PlayerIdentity) -> bool {
    space_mgr
        .get_entity(owner)
        .is_some_and(|o| o.is_player && (!identity.is_known() || o.identity() == identity))
}

/// Remove `deployable` visibly (`LeftAoI` to every witness), drop it from
/// the registry and log its summary. `path` names the caller (`sweep`,
/// `recast`). Refuses (`NotFound` / `RefusedPlayer`) exactly as
/// `despawn_npc` does; the registry entry is dropped either way.
pub async fn despawn_deployable(
    space_mgr: &mut SpaceManager,
    deployable: u32,
    reason: DeployableDespawnReason,
    path: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> DespawnOutcome {
    let state = space_mgr.deployables.get(deployable).cloned();
    if let Some(s) = &state {
        // A traveller's view is being reset: leave its client out of the
        // fan-out, but only when the id still names the owner. A player
        // given the id is an ordinary witness and must see the object go.
        if reason.owner_view_torn_down() && owner_holds(space_mgr, s.owner, s.owner_identity) {
            if let Some(owner) = space_mgr.get_entity_mut(s.owner) {
                owner.witnesses.remove(&EntityId(deployable as i32));
            }
        }
    }
    // Named before `despawn_npc` removes the entity (Rule 6).
    let deployable_name = space_mgr.entity_names(deployable).entity_name;
    let outcome = space_mgr.despawn_npc(deployable, tx).await;
    space_mgr.deployables.forget(deployable);

    let (owner, id, ability_id, template_id, totals, lifetime_secs) = match &state {
        Some(s) => (
            Some(s.owner),
            s.owner_identity,
            Some(s.ability_id),
            Some(s.template_id),
            s.totals,
            Some(s.spawned_at.elapsed().as_secs_f32()),
        ),
        None => (
            None,
            PlayerIdentity::UNKNOWN,
            None,
            None,
            Default::default(),
            None,
        ),
    };
    let book = cimmeria_names::book();
    let ability_name = ability_id.and_then(|a| book.ability(a));
    let template_name = template_id.and_then(|t| book.template(t));
    match outcome {
        DespawnOutcome::Despawned { witnesses_notified } => tracing::info!(
            target: "deployables.lifecycle",
            decision_outcome = "despawned",
            event = "despawned",
            entity_id = deployable,
            entity_name = deployable_name,
            deployable_id = deployable,
            deployable_name,
            owner_id = owner,
            owner_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            ability_id,
            ability_name,
            template_id,
            template_name,
            reason = reason.reason(),
            path,
            witnesses_notified,
            pulses = totals.pulses,
            hits = totals.hits,
            health_damage = totals.health_damage,
            focus_damage = totals.focus_damage,
            kills = totals.kills,
            lifetime_secs,
            "deployable removed"
        ),
        // A teardown asked to remove an object that is not there: a server
        // bookkeeping error, never client-triggerable.
        other => tracing::warn!(
            target: "deployables.lifecycle",
            decision_outcome = "despawn_failed",
            event = "despawn_failed",
            entity_id = deployable,
            entity_name = deployable_name,
            deployable_id = deployable,
            deployable_name,
            owner_id = owner,
            owner_name = id.player_name,
            account_id = id.account_id,
            account_name = id.account_name,
            player_id = id.player_id,
            player_name = id.player_name,
            ability_id,
            ability_name,
            template_id,
            template_name,
            reason = reason.reason(),
            path,
            outcome = ?other,
            "deployable despawn did not remove an entity"
        ),
    }
    outcome
}

/// Drop the registry entry of a deployable whose entity is already gone
/// (removed by a path that knew nothing about deployables). Logged at WARN:
/// `destroy_entity` and `destroy_space` scrub the registry themselves, so
/// an orphan is a missed teardown path.
pub fn scrub_orphan(space_mgr: &mut SpaceManager, deployable: u32) {
    let Some(state) = space_mgr.deployables.forget(deployable) else {
        return;
    };
    tracing::warn!(
        target: "deployables.lifecycle",
        decision_outcome = "registry_scrubbed",
        event = "registry_scrubbed",
        entity_id = deployable, // nt:id-only the entity is already gone, nothing to name
        deployable_id = deployable, // nt:id-only same entity, already gone
        owner_id = state.owner,
        owner_name = state.owner_identity.player_name,
        account_id = state.owner_identity.account_id,
        account_name = state.owner_identity.account_name,
        player_id = state.owner_identity.player_id,
        player_name = state.owner_identity.player_name,
        ability_id = state.ability_id,
        ability_name = cimmeria_names::book().ability(state.ability_id),
        reason = "entity_gone",
        path = "sweep",
        "deployable registry entry without an entity dropped"
    );
}
