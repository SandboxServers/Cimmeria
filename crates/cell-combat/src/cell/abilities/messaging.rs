//! Entity-method routing helpers for the cell-side wire dispatch.
//!
//! Three routing modes live here:
//!
//! - [`send_entity_method`] — entity-aware default. Player → self only; NPC →
//!   witnesses only. Used by paths where the state change is meaningful to
//!   one audience (the owner OR the observers, not both).
//! - [`send_entity_method_to_witnesses`] — strict witness-only fanout. Never
//!   sends to the entity's own client even if it's a player. Use when the
//!   event is for observers regardless of who owns the entity (e.g., a
//!   corpse-loot indicator, an NPC cleanup signal).
//! - [`send_entity_method_to_self_and_witnesses`] — owner + witnesses. Use for
//!   player state changes that must propagate to other players in AoI — the
//!   five cases in [#278](https://github.com/SandboxServers/Cimmeria/issues/278):
//!   `BSF_IN_COMBAT` flip, death/respawn state-field, `BSF_HOLSTER` posture,
//!   `BeingAppearance` equip recomposite, `setMovementType`. For NPC entities
//!   the "self" send is degenerate (NPCs have no client) and this collapses
//!   to the witnesses-only path.
//!
//! Every send goes through one router, [`deliver`], so a send the closed
//! cell-to-base channel refuses is always a WARN (`abilities.wire`,
//! `event = wire_send_failed`), never a silent `let _`. Ability-subsystem
//! callers that also owe an `abilities.wire` row use
//! [`super::wire_ledger::send`], which routes through the same function.
//!
//! Also hosts the dirty-stat flush helper that pushes a queued `onStatUpdate`
//! to the attacker's client after an ammo decrement.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::wire_ledger::{self, WireCtx};

/// `event` of a witness fan-out that found no witnesses (`abilities.wire`,
/// DEBUG).
pub(crate) const EVENT_NO_WITNESSES: &str = "wire_no_witnesses";

/// Which audience one entity-method send goes to. Each of the three public
/// helpers below is one of these; [`wire_ledger::send`] takes it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireRoute {
    /// The entity's own client, unconditionally. The caller has already
    /// checked that the entity is a player.
    SelfOnly,
    /// [`send_entity_method`]: a player's own client, or an NPC's witnesses.
    EntityDefault,
    /// [`send_entity_method_to_witnesses`]: witnesses only, never the owner.
    Witnesses,
    /// [`send_entity_method_to_self_and_witnesses`]: the owner (a player)
    /// and every witness.
    SelfAndWitnesses,
}

impl WireRoute {
    /// The `route` field on the `abilities.wire` rows.
    pub fn label(self) -> &'static str {
        match self {
            Self::SelfOnly => "self",
            Self::EntityDefault => "entity_default",
            Self::Witnesses => "witnesses",
            Self::SelfAndWitnesses => "self_and_witnesses",
        }
    }
}

/// What one routed send reached: the input to the `abilities.wire` row.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The method was queued for the entity's own client.
    pub self_sent: bool,
    /// The witnesses it was queued for.
    pub witness_ids: Vec<u32>,
    /// Witnesses it was addressed to, including those whose send failed.
    pub witnesses_addressed: usize,
    /// Sends that failed because the cell-to-base channel is closed. Each
    /// failure is a WARN (`abilities.wire`, `event = wire_send_failed`).
    pub failed: usize,
}

impl Delivery {
    /// A single successful send to the entity's own client.
    pub fn self_only() -> Self {
        Self {
            self_sent: true,
            ..Self::default()
        }
    }

    /// Clients the method was queued for.
    pub fn delivered(&self) -> usize {
        usize::from(self.self_sent) + self.witness_ids.len()
    }
}

/// Send an entity method call, routing to the entity's client if it's a player,
/// or broadcasting to all witnessing players if it's an NPC (ghost entity).
///
/// In BigWorld, method calls on ghost entities are forwarded to all players who
/// have that entity in their AoI. This is how players see NPC attack animations,
/// health changes, death states, etc.
///
/// This is the **entity-aware default**. If you need a player's state change
/// to also reach other players in AoI, use
/// [`send_entity_method_to_self_and_witnesses`] instead — this function alone
/// does not fan out for players.
///
/// A send the closed cell-to-base channel refuses is a WARN. Ability-subsystem
/// callers that also want an `abilities.wire` row go through
/// [`wire_ledger::send`] instead.
pub async fn send_entity_method(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    deliver(
        entity_id,
        method_index,
        args,
        WireRoute::EntityDefault,
        tx,
        space_mgr,
    )
    .await;
}

/// Refuse a witness fanout of a method the client never binds on a non-player
/// entity (today only `onTimerUpdate`; see [`super::timer_update`]). Every
/// such send is a silent drop on the witness's client, so reaching this is a
/// caller that bypassed [`super::timer_update::send_timer_update`]: WARN.
fn refuse_unbound_npc_method(entity_id: u32, method_index: u16, via: &'static str) -> bool {
    if !super::timer_update::unbound_on_non_player(method_index) {
        return false;
    }
    tracing::warn!(
        target: "abilities.wire",
        event = "wire_npc_method_unbound",
        entity_id,
        method_index,
        method_name = cimmeria_wire::names::any_entity_client_method(method_index),
        via,
        reason = "no_client_binding",
        "NPC method not fanned out to witnesses: the client binds it on SGWPlayer only and would drop it"
    );
    true
}

/// Fan out an entity-method call to all AoI witnesses of `entity_id`.
///
/// Strict witness-only: never sends to `entity_id`'s own client even if it's
/// a player. Returns the witness count actually addressed so callers can
/// `tracing::debug!` the fanout shape without re-querying the space manager.
///
/// An empty result (no AoI witnesses) is a debug-level signal, not a warning —
/// many state changes legitimately have no observers (player alone in a space,
/// NPC outside any player's AoI, etc.). Use [`send_entity_method`] if you want
/// the "NPC with no witnesses" warning — that signal indicates a routing bug
/// for ghost entities, which doesn't apply when the caller has explicitly
/// asked for witness-only fanout.
pub async fn send_entity_method_to_witnesses(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> usize {
    deliver(
        entity_id,
        method_index,
        args,
        WireRoute::Witnesses,
        tx,
        space_mgr,
    )
    .await
    .witnesses_addressed
}

/// Emit an entity-method call to `entity_id`'s own client AND fan out to all
/// AoI witnesses.
///
/// This is the helper [#278](https://github.com/SandboxServers/Cimmeria/issues/278)
/// names — use it for player state changes that must also propagate to other
/// players who can see them (BSF_IN_COMBAT, death/respawn flips, holster pose,
/// equip BeingAppearance, movement-type transitions).
///
/// For an NPC entity (`is_player == false`), the "self" path degenerates —
/// NPCs don't have a client — and this collapses to a witness-only fanout,
/// equivalent to [`send_entity_method_to_witnesses`]. That keeps the helper
/// callable from paths that don't statically know whether the entity is a
/// player without forcing a branch at every callsite.
///
/// Returns the witness count actually addressed (excludes the self send).
pub async fn send_entity_method_to_self_and_witnesses(
    entity_id: u32,
    method_index: u16,
    args: Vec<u8>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> usize {
    deliver(
        entity_id,
        method_index,
        args,
        WireRoute::SelfAndWitnesses,
        tx,
        space_mgr,
    )
    .await
    .witnesses_addressed
}

/// The witnesses `route` addresses for `entity_id`, after the unbound-method
/// refusal and the empty-audience logging each route has always had.
fn witness_audience(
    entity_id: u32,
    method_index: u16,
    route: WireRoute,
    is_player: bool,
    space_mgr: &SpaceManager,
) -> Vec<u32> {
    match route {
        WireRoute::SelfOnly => Vec::new(),
        WireRoute::EntityDefault if is_player => Vec::new(),
        WireRoute::EntityDefault => {
            if refuse_unbound_npc_method(entity_id, method_index, "send_entity_method") {
                return Vec::new();
            }
            let witnesses = space_mgr.get_witnesses_of(entity_id);
            if witnesses.is_empty() {
                tracing::warn!(
                    target: "abilities.wire",
                    event = "wire_npc_no_witnesses",
                    entity_id,
                    method_index,
                    method_name = space_mgr.client_method_name(entity_id, method_index),
                    "send_entity_method: NPC has no witnesses, method dropped"
                );
            }
            witnesses
        }
        WireRoute::Witnesses | WireRoute::SelfAndWitnesses => {
            let witnesses = space_mgr.get_witnesses_of(entity_id);
            if witnesses.is_empty() {
                // Routine for a player alone in AoI: the self send (if the
                // route has one) still goes out and the `wire_sent` row
                // counts it. The row names the cast so a forensics query on
                // `cast_id` reads it in sequence (2026-10-04 colo smoke test:
                // it was the one row of a Heal Focus cast with no `event`).
                let who = space_mgr.player_identity(entity_id);
                tracing::debug!(
                    target: "abilities.wire",
                    event = EVENT_NO_WITNESSES,
                    stage = "wire",
                    method = wire_ledger::method_name(method_index),
                    method_index,
                    entity_id,
                    method_name = space_mgr.client_method_name(entity_id, method_index),
                    account_id = who.account_id,
                    player_id = who.player_id,
                    cast_id = space_mgr.current_cast_id(),
                    route = route.label(),
                    self_send = is_player && route == WireRoute::SelfAndWitnesses,
                    "witness fan-out skipped: no witnesses in AoI (the owner's own send, \
                     when the route has one, still goes out; see self_send)"
                );
                return witnesses;
            }
            if !is_player
                && refuse_unbound_npc_method(
                    entity_id,
                    method_index,
                    "send_entity_method_to_witnesses",
                )
            {
                return Vec::new();
            }
            witnesses
        }
    }
}

/// Route one entity-method call and report what it reached. Every send in
/// this module goes through here, so a failed send is always a WARN.
pub(crate) async fn deliver(
    entity_id: u32,
    method_index: u16,
    mut args: Vec<u8>,
    route: WireRoute,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) -> Delivery {
    let is_player = space_mgr.get_entity(entity_id).is_some_and(|e| e.is_player);
    let to_self = match route {
        WireRoute::SelfOnly => true,
        WireRoute::EntityDefault | WireRoute::SelfAndWitnesses => is_player,
        WireRoute::Witnesses => false,
    };
    let witnesses = witness_audience(entity_id, method_index, route, is_player, space_mgr);
    let mut out = Delivery {
        witnesses_addressed: witnesses.len(),
        ..Delivery::default()
    };

    if to_self {
        // The last send takes the buffer; only a fan-out behind it clones.
        let self_args = if witnesses.is_empty() {
            std::mem::take(&mut args)
        } else {
            args.clone()
        };
        let sent = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args: self_args,
            })
            .await;
        if sent.is_ok() {
            out.self_sent = true;
        } else {
            out.failed += 1;
            let who = space_mgr.player_identity(entity_id);
            crate::cell::abilities::metrics::wire_send_failed_in(
                space_mgr,
                entity_id,
                crate::cell::abilities::metrics::WireMessage::from_method(method_index),
            );
            tracing::warn!(
                target: "abilities.wire",
                event = "wire_send_failed",
                method = wire_ledger::method_name(method_index),
                method_index,
                method_name = space_mgr.client_method_name(entity_id, method_index),
                entity_id,
                recipient_id = entity_id,
                account_id = who.account_id,
                player_id = who.player_id,
                route = route.label(),
                reason = "cell_to_base_closed",
                "entity method not queued for the owner's client: the cell-to-base channel is \
                 closed, so the player never sees it"
            );
        }
    }

    let mut witness_failed = 0usize;
    for witness_id in witnesses {
        if route == WireRoute::EntityDefault {
            // TRACE: one row per witness per send. The fan-out's one
            // `wire_sent` row (AB-T4) carries the witness count at DEBUG.
            tracing::trace!(
                target: "abilities.wire",
                event = "wire_witness_routed",
                witness_id,
                entity_id,
                method_index,
                method_name = space_mgr.client_method_name(entity_id, method_index),
                "send_entity_method: routing NPC method to witness"
            );
        }
        let sent = tx
            .send(CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args: args.clone(),
                // Drives the idbase selection at wire-encode time; the same
                // for every witness of this entity.
                entity_is_player: is_player,
            })
            .await;
        if sent.is_ok() {
            out.witness_ids.push(witness_id);
        } else {
            witness_failed += 1;
        }
    }
    if witness_failed > 0 {
        // One row per fan-out, not per witness: a closed channel refuses
        // every witness in the loop the same way.
        out.failed += witness_failed;
        crate::cell::abilities::metrics::wire_send_failed_in(
            space_mgr,
            entity_id,
            crate::cell::abilities::metrics::WireMessage::from_method(method_index),
        );
        tracing::warn!(
            target: "abilities.wire",
            event = "wire_send_failed",
            method = wire_ledger::method_name(method_index),
            method_index,
            method_name = space_mgr.client_method_name(entity_id, method_index),
            entity_id,
            route = route.label(),
            witness_count = out.witnesses_addressed,
            failed_count = witness_failed,
            reason = "cell_to_base_closed",
            "entity method not queued for its witnesses: the cell-to-base channel is closed, so \
             the observers never see it"
        );
    }
    if matches!(route, WireRoute::Witnesses | WireRoute::SelfAndWitnesses)
        && out.witnesses_addressed > 0
    {
        let who = space_mgr.player_identity(entity_id);
        tracing::debug!(
            target: "abilities.wire",
            event = "wire_fanned_out",
            stage = "wire",
            method = wire_ledger::method_name(method_index),
            entity_id,
            account_id = who.account_id,
            player_id = who.player_id,
            cast_id = space_mgr.current_cast_id(),
            route = route.label(),
            method_index,
            method_name = space_mgr.client_method_name(entity_id, method_index),
            witness_count = out.witnesses_addressed,
            "send_entity_method_to_witnesses: fanned out"
        );
    }
    out
}

/// Send a `CellToBaseMsg::RefreshAppearance` for a player entity, reading
/// the player's current `weapon_holstered` state off the cell entity.
///
/// Called from the combat enter/exit broadcast sites after
/// `onStateFieldUpdate` so a draw or holster reaches the wire in the same
/// dispatch burst as the BSF_InCombat change. No-op (with a debug log)
/// for non-player entities or for players whose `player_id` (DB id) hasn't
/// been populated yet — both happen during transient world-entry races
/// and we'd rather drop the rebroadcast than send junk.
pub async fn request_appearance_refresh(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let (player_id, holstered) = match space_mgr.get_entity(entity_id) {
        Some(e) if e.is_player => match e.player_id {
            Some(pid) => (pid, e.weapon_holstered),
            None => {
                tracing::debug!(
                    target: "abilities.wire",
                    event = "appearance_refresh_skipped",
                    entity_id,
                reason = "no_player_id",
                    "request_appearance_refresh: player entity has no DB player_id (pre-load?), skipping"
                );
                return;
            }
        },
        Some(_) => {
            tracing::debug!(
                target: "abilities.wire",
                event = "appearance_refresh_skipped",
                entity_id,
                reason = "not_player",
                "request_appearance_refresh: entity is not a player, skipping"
            );
            return;
        }
        None => {
            tracing::debug!(
                target: "abilities.wire",
                event = "appearance_refresh_skipped",
                entity_id,
                reason = "entity_missing",
                "request_appearance_refresh: entity not found in space_mgr, skipping"
            );
            return;
        }
    };
    if tx
        .send(CellToBaseMsg::RefreshAppearance {
            entity_id,
            player_id,
            holstered,
        })
        .await
        .is_err()
    {
        crate::cell::abilities::metrics::wire_send_failed_in(
            space_mgr,
            entity_id,
            crate::cell::abilities::metrics::WireMessage::RefreshAppearance,
        );
        tracing::warn!(
            target: "abilities.wire",
            event = "wire_send_failed",
            method = "RefreshAppearance",
            entity_id,
            account_id = space_mgr.player_identity(entity_id).account_id,
            player_id,
            holstered,
            reason = "cell_to_base_closed",
            "appearance refresh not queued: the cell-to-base channel is closed, so the weapon              draw or holster never reaches the player or their witnesses"
        );
    }
}

/// Drain the attacker's dirty stats and push `onStatUpdate` (method 20) to its
/// client. Used by `handle_use_ability` after a successful ammo consume — and
/// crucially before any early-return that follows the consume — so the client
/// always sees the AmmoSlot{N} decrement, even when downstream lookups fail.
pub(super) async fn flush_attacker_ammo_stat(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let payload = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => {
            let p = e.stats.serialize_dirty();
            e.stats.clear_dirty();
            p
        }
        None => Vec::new(),
    };
    if !payload.is_empty() {
        wire_ledger::send(
            entity_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            payload,
            WireRoute::EntityDefault,
            WireCtx::new("ammo_flush"),
            tx,
            space_mgr,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "messaging_tests.rs"]
mod tests;
