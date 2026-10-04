//! Native consumables: using an item applies the ability that
//! `resources.items_event_sets` binds to it under event 5
//! (`EVENT_ITEM_USE_ABILITY`), with no content chain.
//!
//! # Which items
//!
//! [`classify`] decides from seed data alone. An item is a native
//! consumable when all of these hold:
//!
//! 1. It has an event-5 binding in `items_event_sets`.
//! 2. The bound ability is not 597 "Heal Focus"
//!    ([`PLACEHOLDER_ITEM_USE_ABILITY`]). The reconstructed seed binds 597
//!    to 158 unrelated mission items (Opheltes's Injection, a Banged-up
//!    Radio, a DHD Bypass Card, ...) as a filler; its effect 659 really does
//!    heal 35 % Focus, so firing it would hand out free heals from quest
//!    props.
//! 3. Every effect of the ability runs a script this path understands:
//!    `HealHealth`, `HealFocus` (a pool heal) or `StatBuff` (a timed
//!    attribute buff). The other ~100 bindings have no script. Mission
//!    items (scanners, detonators, disguise pieces; `container_sets`
//!    `{2}`) keep their old behaviour: a chain decides, or nothing happens.
//!    A bag consumable (`{1,17}`: the Stealth, Energy and Disguise boosts
//!    and the antidotes) with no chain is refused with "This item has no
//!    effect yet." and kept, so the press is never silent.
//! 4. No content chain triggers on `item_use` for it
//!    (`ChainEngine::has_item_use_chain`). A hand-authored chain owns its
//!    item outright: the Ambernol vial (item 19, chain 1034) keeps its
//!    mission gate and its own `launch_ability(1374)` + `remove_item`, and
//!    the native path stands aside, so one use can never both run a chain
//!    and apply the item's ability.
//!
//! # The use, and why the base consumes first
//!
//! [`try_native_use`] runs from `fire_item_use` when `ItemUsed` reaches the
//! cell. It refuses a use that would do nothing, with feedback the player
//! sees (`onErrorCode` plus a `CHAN_FEEDBACK` line, the pattern of the
//! owner-pet refusals): the user is dead, or every pool the item heals is
//! already full. A stat buff always goes ahead (a second one refreshes it).
//! A refused use consumes nothing.
//!
//! An allowed use sends `CellToBaseMsg::ConsumeItemForUse`. The base takes
//! one unit off the clicked stack under a row lock and only on commit
//! answers `BaseToCellMsg::ItemUseConsumed`, which lands in
//! [`apply_consumed_item`] and applies the ability to the user. The effect
//! is paid for before it happens: a double-click on the last unit, or an
//! `ItemUsed` the outbox redelivers, finds no row the second time and gets
//! no effect. (A chain's `change_stat` then `remove_item` applies first and
//! pays later, so the same double-click healed twice for one item.)
//!
//! # Reachability is the security boundary
//!
//! This module applies abilities through `effect_apply`, which bypasses the
//! combat gates (no cooldown, no warmup, no hostility check), so the same
//! three properties that guard `effect_apply` hold here:
//!
//! 1. **The module is private** (`mod consumable_use;` in `content/mod.rs`),
//!    and its two public entry points, re-exported through `fire_item_use`
//!    and `apply_consumed_item`, take an item design id, never an ability
//!    or effect id.
//! 2. **The ability id is never client-supplied.** It comes from
//!    `items_event_sets`, keyed by the `type_id` the base read from the
//!    player's own `sgw_inventory` row: `ItemUsed` after an ownership
//!    lookup, `ItemUseConsumed` from the row it just consumed under lock.
//!    The client names only an inventory instance id.
//! 3. **The target is always the user.** No wire field picks it.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_entity::stats::{FOCUS, HEALTH};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use crate::cell::messages::{CellToBaseMsg, ConsumeItemForUse, ItemUseConsumed};
use crate::cell::space_manager::{vault_access, SpaceManager};
use crate::cell::spawner::EVENT_ITEM_USE_ABILITY;

use super::effect_apply::apply_ability_effects;

/// Ability 597 "Heal Focus": the filler the seed binds to 158 mission items
/// under event 5. Never fired from an item.
pub(super) const PLACEHOLDER_ITEM_USE_ABILITY: i32 = 597;

/// `ERRORCODE_SYSTEM_Ability`, the only `EErrorCodeSystem` value.
const ERRORCODE_SYSTEM_ABILITY: u8 = 0;
/// `CONDITION_FEEDBACK_StatValueGreaterThanOrEqual`: the pool is at max.
pub(super) const FEEDBACK_STAT_AT_MAX: u16 = 32;
/// `CONDITION_FEEDBACK_NotLiving`: the user is dead.
pub(super) const FEEDBACK_NOT_LIVING: u16 = 14;

pub(super) const FULL_HEALTH_TEXT: &str = "You are already at full health.";
pub(super) const FULL_FOCUS_TEXT: &str = "You are already at full focus.";
pub(super) const DEAD_TEXT: &str = "You cannot use that while dead.";
pub(super) const NOT_IMPLEMENTED_TEXT: &str = "This item has no effect yet.";

/// What an item's event-5 ability does, when this path can apply it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConsumablePlan {
    pub(super) ability_id: i32,
    /// The pools its heal effects restore (`HEALTH`, `FOCUS`).
    pub(super) heals: Vec<i32>,
    /// Whether any effect is a `StatBuff`.
    pub(super) buffs: bool,
}

/// How [`classify`] sees an item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Classification {
    /// No event-5 binding.
    NotBound,
    /// Bound to [`PLACEHOLDER_ITEM_USE_ABILITY`].
    Placeholder,
    /// Bound to an ability this path cannot apply (`reason` says why).
    NotNative {
        ability_id: i32,
        reason: &'static str,
    },
    Native(ConsumablePlan),
}

/// Classify `type_id` from the startup caches. Pure; does not look at
/// content chains (see [`try_native_use`]).
pub(super) fn classify(type_id: i32, space_mgr: &SpaceManager) -> Classification {
    let Some(&ability_id) = space_mgr
        .item_event_set_abilities
        .get(&(type_id, EVENT_ITEM_USE_ABILITY))
    else {
        return Classification::NotBound;
    };
    if ability_id == PLACEHOLDER_ITEM_USE_ABILITY {
        return Classification::Placeholder;
    }
    let not_native = |reason| Classification::NotNative { ability_id, reason };
    let Some(def) = space_mgr.ability_defs.get(&ability_id) else {
        return not_native("no_ability_def");
    };
    if def.effect_ids.is_empty() {
        return not_native("no_effects");
    }
    let mut plan = ConsumablePlan {
        ability_id,
        heals: Vec::new(),
        buffs: false,
    };
    for effect_id in &def.effect_ids {
        let script = space_mgr
            .effect_defs
            .get(effect_id)
            .and_then(|e| e.script_name.as_deref());
        match script {
            Some("HealHealth") => plan.heals.push(HEALTH),
            Some("HealFocus") => plan.heals.push(FOCUS),
            Some("StatBuff") => plan.buffs = true,
            _ => return not_native("effect_not_native"),
        }
    }
    Classification::Native(plan)
}

/// Why a use is refused before anything is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    Dead,
    /// Every pool the item heals is full; carries the first one.
    AtMax(i32),
    /// A bag consumable whose event-5 ability this path cannot apply yet
    /// (the Stealth / Energy / Disguise boosts, the antidotes).
    NotImplemented,
}

impl Refusal {
    fn reason(self) -> &'static str {
        match self {
            Self::Dead => "dead",
            Self::AtMax(_) => "already_at_max",
            Self::NotImplemented => "consumable_not_implemented",
        }
    }

    /// The `onErrorCode` code. `None` for [`Self::NotImplemented`]: the
    /// client enum has no "no effect" value, and a wrong condition code
    /// would mislead anyone reading a capture (the code has no Lua
    /// consumer anyway), so that refusal sends only the chat line.
    fn code(self) -> Option<u16> {
        match self {
            Self::Dead => Some(FEEDBACK_NOT_LIVING),
            Self::AtMax(_) => Some(FEEDBACK_STAT_AT_MAX),
            Self::NotImplemented => None,
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Dead => DEAD_TEXT,
            Self::AtMax(FOCUS) => FULL_FOCUS_TEXT,
            Self::AtMax(_) => FULL_HEALTH_TEXT,
            Self::NotImplemented => NOT_IMPLEMENTED_TEXT,
        }
    }
}

/// Whether a use of `type_id` that this path cannot apply must still be
/// answered: the item is a bag consumable (its preferred container is the
/// main bag, `container_sets` `{1,17}`), so the player expects it to do
/// something, and no content chain owns it. A mission item (`{2}`) is left
/// to its chains and stays silent, and the 597 filler never reaches here.
fn is_unimplemented_bag_consumable(
    type_id: i32,
    engine: &ChainEngine,
    space_mgr: &SpaceManager,
) -> bool {
    space_mgr.item_containers.get(&type_id) == Some(&cimmeria_entity::inventory::INV_MAIN)
        && !engine.has_item_use_chain(type_id)
}

/// Whether `plan` would do nothing for `entity_id` right now. A missing
/// entity is not refused here (nothing can be sent to it either).
pub(super) fn refusal(
    plan: &ConsumablePlan,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> Option<Refusal> {
    let entity = space_mgr.get_entity(entity_id)?;
    if crate::cell::combat::is_dead_state(entity.state_field) {
        return Some(Refusal::Dead);
    }
    if plan.buffs || plan.heals.is_empty() {
        return None;
    }
    let full = |stat: &i32| entity.stats.get(*stat).is_some_and(|s| s.cur >= s.max);
    plan.heals
        .iter()
        .all(full)
        .then(|| Refusal::AtMax(plan.heals[0]))
}

/// Run the native path for one `ItemUsed`. Returns `true` when this path
/// owned the use (refused it or asked the base to consume), so
/// `fire_item_use` must not also run chains for it; `false` when the item
/// is not a native consumable and chains decide as before.
pub(super) async fn try_native_use(
    entity_id: u32,
    player_id: i32,
    instance_id: i32,
    type_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let plan = match classify(type_id, space_mgr) {
        Classification::Native(plan) => plan,
        Classification::NotBound => return false,
        Classification::Placeholder => {
            tracing::debug!(
                event = "consumable_skipped",
                reason = "placeholder_ability",
                entity_id,
                player_id,
                type_id,
                ability_id = PLACEHOLDER_ITEM_USE_ABILITY,
                "item use: event-5 binding is the Heal Focus filler; not applied"
            );
            return false;
        }
        Classification::NotNative { ability_id, reason } => {
            if is_unimplemented_bag_consumable(type_id, engine, space_mgr) {
                let id = space_mgr.player_identity(entity_id);
                tracing::warn!(
                    event = "consumable_refused",
                    decision_outcome = "refused",
                    reason = Refusal::NotImplemented.reason(),
                    cause = reason,
                    entity_id,
                    account_id = id.account_id,
                    player_id,
                    item_id = instance_id,
                    instance_id,
                    type_id,
                    ability_id,
                    "item use refused: a bag consumable whose effect is not implemented; \
                     nothing consumed"
                );
                send_refusal(entity_id, id, ability_id, Refusal::NotImplemented, tx).await;
                return true;
            }
            tracing::debug!(
                event = "consumable_skipped",
                reason,
                entity_id,
                player_id,
                type_id,
                ability_id,
                "item use: event-5 ability has an effect this path cannot apply; chains decide"
            );
            return false;
        }
    };
    let id = space_mgr.player_identity(entity_id);
    if engine.has_item_use_chain(type_id) {
        tracing::info!(
            event = "consumable_skipped",
            reason = "chain_owns_item",
            entity_id,
            account_id = id.account_id,
            player_id,
            type_id,
            ability_id = plan.ability_id,
            "item use: an item_use chain owns this item; the native consumable path stands aside"
        );
        return false;
    }
    if let Some(refused) = refusal(&plan, entity_id, space_mgr) {
        let stat = match refused {
            Refusal::AtMax(stat) => Some(stat),
            Refusal::Dead | Refusal::NotImplemented => None,
        };
        let (cur, max) = stat
            .and_then(|s| {
                space_mgr
                    .get_entity(entity_id)?
                    .stats
                    .get(s)
                    .map(|s| (s.cur, s.max))
            })
            .unzip();
        tracing::info!(
            event = "consumable_refused",
            decision_outcome = "refused",
            reason = refused.reason(),
            entity_id,
            account_id = id.account_id,
            player_id,
            instance_id,
            type_id,
            ability_id = plan.ability_id,
            stat_id = stat,
            stat_cur = cur,
            stat_max = max,
            "item use refused: it would do nothing; nothing consumed"
        );
        send_refusal(entity_id, id, plan.ability_id, refused, tx).await;
        return true;
    }
    if instance_id == 0 {
        // Only an `ItemUsed` row written before the instance id existed
        // carries 0; consuming "the first of that type" could take a unit
        // from another stack than the one used.
        tracing::warn!(
            event = "consumable_skipped",
            reason = "no_instance_id",
            entity_id,
            account_id = id.account_id,
            player_id,
            type_id,
            ability_id = plan.ability_id,
            "item use: ItemUsed carries no instance id; nothing consumed or applied"
        );
        return true;
    }
    let msg = CellToBaseMsg::ConsumeItemForUse(ConsumeItemForUse {
        entity_id,
        player_id,
        instance_id,
        type_id,
        vault: vault_access(entity_id, space_mgr),
    });
    if let Err(e) = tx.send(msg).await {
        tracing::error!(
            event = "consumable_consume_send_failed",
            reason = "cell_to_base_closed",
            entity_id,
            account_id = id.account_id,
            player_id,
            instance_id,
            type_id,
            error = %e,
            "item use: ConsumeItemForUse could not be queued; nothing consumed or applied"
        );
        return true;
    }
    tracing::debug!(
        event = "consumable_consume_requested",
        entity_id,
        account_id = id.account_id,
        player_id,
        instance_id,
        type_id,
        ability_id = plan.ability_id,
        "item use: asked the base to consume one unit before applying"
    );
    true
}

/// Apply the ability of a unit the base has consumed. Backs
/// `BaseToCellMsg::ItemUseConsumed`.
#[tracing::instrument(
    name = "inventory.consumable_apply",
    level = "info",
    skip_all,
    fields(entity_id = msg.entity_id, player_id = msg.player_id, type_id = msg.type_id)
)]
pub async fn apply_consumed_item(
    msg: ItemUseConsumed,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let ItemUseConsumed {
        entity_id,
        player_id,
        instance_id,
        type_id,
    } = msg;
    let id = space_mgr.player_identity(entity_id);
    let miss = |reason: &'static str, ability_id: Option<i32>| {
        tracing::warn!(
            event = "consumable_apply_skipped",
            reason,
            entity_id,
            account_id = id.account_id,
            player_id,
            instance_id,
            type_id,
            ability_id,
            "item use: the unit was consumed but its effect was not applied"
        );
    };
    if space_mgr.get_entity(entity_id).and_then(|e| e.player_id) != Some(player_id) {
        miss("entity_gone", None);
        return;
    }
    let plan = match classify(type_id, space_mgr) {
        Classification::Native(plan) => plan,
        _ => {
            miss("no_native_plan", None);
            return;
        }
    };
    if refusal(&plan, entity_id, space_mgr) == Some(Refusal::Dead) {
        miss("dead_at_apply", Some(plan.ability_id));
        return;
    }
    let before = pools(entity_id, space_mgr);
    let registered =
        apply_ability_effects(plan.ability_id, entity_id, entity_id, 0, tx, space_mgr).await;
    let after = pools(entity_id, space_mgr);
    let buffs = space_mgr
        .get_entity(entity_id)
        .map_or(0, |e| e.stat_buffs.entries.len());
    tracing::info!(
        event = "consumable_used",
        decision_outcome = "applied",
        entity_id,
        account_id = id.account_id,
        player_id,
        instance_id,
        type_id,
        ability_id = plan.ability_id,
        heals = ?plan.heals,
        stat_buff = plan.buffs,
        registered,
        health_before = before.0,
        health_after = after.0,
        focus_before = before.1,
        focus_after = after.1,
        active_stat_buffs = buffs,
        "item used: one unit consumed, its ability applied to the user"
    );
}

/// `(HEALTH cur, FOCUS cur)`, for the log row.
fn pools(entity_id: u32, space_mgr: &SpaceManager) -> (Option<i32>, Option<i32>) {
    let stat = |s| {
        space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.stats.get(s))
            .map(|s| s.cur)
    };
    (stat(HEALTH), stat(FOCUS))
}

/// The refusal pair: `onErrorCode(ERRORCODE_SYSTEM_Ability, ability_id,
/// code)`, which the shipped client has no Lua consumer for (kept for
/// parity), and the `CHAN_FEEDBACK` line that actually renders.
async fn send_refusal(
    entity_id: u32,
    id: PlayerIdentity,
    ability_id: i32,
    refused: Refusal,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut calls = Vec::with_capacity(2);
    if let Some(code) = refused.code() {
        calls.push((
            crate::mercury::method_idx::ON_ERROR_CODE,
            error_code_args(ability_id, code),
        ));
    }
    calls.push((
        crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
        serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, refused.text()),
    ));
    for (method_index, args) in calls {
        if tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            })
            .await
            .is_err()
        {
            tracing::warn!(
                event = "consumable_feedback_send_failed",
                reason = "cell_to_base_closed",
                entity_id,
                account_id = id.account_id,
                player_id = id.player_id,
                ability_id,
                method_index,
                "item use refusal feedback could not be queued; the click shows nothing"
            );
        }
    }
}

/// `onErrorCode` args: `ErrorSystem:u8, InstanceID:i32, ErrorCode:u16`.
pub(super) fn error_code_args(ability_id: i32, code: u16) -> Vec<u8> {
    let mut err = Vec::with_capacity(7);
    err.push(ERRORCODE_SYSTEM_ABILITY);
    err.extend_from_slice(&ability_id.to_le_bytes());
    err.extend_from_slice(&code.to_le_bytes());
    err
}
