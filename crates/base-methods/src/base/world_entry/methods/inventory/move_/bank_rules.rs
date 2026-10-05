//! The personal vault's rules for `moveItem` (bank-vault BV-03; D-BV05,
//! D-BV08), their refusal reasons and feedback lines, and the
//! `move_accepted` event.
//!
//! The vault session lives in the cell, so the cell attaches its verdict
//! ([`VaultAccess`]) to every forwarded move, taken fresh for that move
//! (session open, same space, pinned Banker within the interact distance;
//! a GM `.bank` session skips proximity). The base applies it here, inside
//! the move transaction, together with the two rules only the database can
//! answer: the player's `bank_slots` and whether the item is a mission item.

use cimmeria_entity::inventory::{INV_BANK, INV_MISSION};
use cimmeria_entity::known_names;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::{Postgres, Transaction};

use super::container_policy::MoveEnd;

/// Why a move is refused. [`MoveRefusal::reason`] is the stable
/// `move_rejected reason=` label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum MoveRefusal {
    /// A container the player never moves (D-BV07, BV-01).
    NotPlayerMovable(MoveEnd),
    /// The vault (17) without an open, in-range session. `reason` is the
    /// cell's verdict label (`no_vault_session`, `banker_out_of_range`, ...).
    VaultSession { end: MoveEnd, reason: &'static str },
    /// A vault slot at or beyond the player's `bank_slots`.
    BankSlotLocked { bank_slots: i32 },
    /// A mission item bound for the vault (D-BV08).
    MissionItem,
    /// A vault move whose item (`Target`: the dragged item into the target
    /// container; `Source`: a swap occupant into the source container) is
    /// not allowed there by its `container_sets`.
    ItemNotAllowed { end: MoveEnd },
    /// A vault move that splits a stack onto an occupied slot.
    SplitOntoOccupied,
}

impl MoveRefusal {
    /// The stable `reason` field.
    pub(super) fn reason(self) -> &'static str {
        match self {
            MoveRefusal::NotPlayerMovable(MoveEnd::Source) => "source_container_not_player_movable",
            MoveRefusal::NotPlayerMovable(MoveEnd::Target) => "target_container_not_player_movable",
            MoveRefusal::VaultSession { reason, .. } => reason,
            MoveRefusal::BankSlotLocked { .. } => "target_slot_beyond_bank_slots",
            MoveRefusal::MissionItem => "mission_item_not_bankable",
            MoveRefusal::ItemNotAllowed { .. } => "item_not_allowed_in_container",
            MoveRefusal::SplitOntoOccupied => "split_onto_occupied_slot",
        }
    }

    /// Which end of the move touched the vault, for the log. `None` for the
    /// BV-01 allowlist refusals, whose reason already names the end.
    pub(super) fn vault_end(self) -> Option<&'static str> {
        match self {
            MoveRefusal::VaultSession { end, .. } => Some(end.label()),
            MoveRefusal::ItemNotAllowed { end } => Some(end.label()),
            MoveRefusal::BankSlotLocked { .. }
            | MoveRefusal::MissionItem
            | MoveRefusal::SplitOntoOccupied => Some("target"),
            MoveRefusal::NotPlayerMovable(_) => None,
        }
    }

    /// The player's `bank_slots`, when the refusal is about them.
    pub(super) fn bank_slots(self) -> Option<i32> {
        match self {
            MoveRefusal::BankSlotLocked { bank_slots } => Some(bank_slots),
            _ => None,
        }
    }

    /// The chat line the player sees before the item snaps back. The BV-01
    /// allowlist refusals (buyback, org vaults) have none: the snap-back is
    /// their whole feedback, as in the legacy server.
    pub(super) fn feedback(self) -> Option<String> {
        let text = match self {
            MoveRefusal::NotPlayerMovable(_) => return None,
            MoveRefusal::VaultSession { reason, .. } => match reason {
                "banker_out_of_range" | "banker_other_space" | "vault_session_other_space" => {
                    "You are too far from the Banker. Return to the Banker to use your vault."
                }
                "banker_gone" => "The Banker has left. Visit a Banker to use your vault.",
                _ => "Your vault is closed. Visit a Banker to use your vault.",
            },
            MoveRefusal::BankSlotLocked { bank_slots } => {
                return Some(format!(
                    "That vault slot is locked. Your vault has {bank_slots} slots."
                ));
            }
            MoveRefusal::MissionItem => "Mission items cannot be stored in the vault.",
            MoveRefusal::ItemNotAllowed { .. } => "That item cannot be placed there.",
            MoveRefusal::SplitOntoOccupied => "Split a stack onto an empty slot.",
        };
        Some(text.to_owned())
    }
}

/// The vault-session refusal for one end of a move into or out of 17, or
/// `None` when the cell's verdict is open.
pub(super) fn vault_session_refusal(end: MoveEnd, vault: &VaultAccess) -> Option<MoveRefusal> {
    vault
        .personal_vault_refusal()
        .map(|reason| MoveRefusal::VaultSession { end, reason })
}

/// The player's row, read inside the move transaction when the move
/// touches the vault.
#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub(super) struct VaultOwner {
    pub account_id: i32,
    pub bank_slots: i16,
}

/// Read `account_id` and `bank_slots` inside the move transaction. A plain
/// read with no row lock: `bank_slots` only grows (BV-05's expansion), so a
/// value read just before a concurrent expansion commits can only refuse a
/// slot the player is about to own, never admit one they do not.
pub(super) async fn read_vault_owner(
    tx: &mut Transaction<'static, Postgres>,
    player_id: i32,
) -> Result<Option<VaultOwner>, sqlx::Error> {
    sqlx::query_as::<_, VaultOwner>(
        "SELECT account_id, bank_slots FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await
}

/// Whether an instance of `type_id` sitting in `container_id` is a mission
/// item (D-BV08): it sits in the mission bag, or its type lists the mission
/// bag in `container_sets`. Items carry no mission flag (`EItemFlag` has
/// none), so the mission bag is the marker.
pub(super) async fn is_mission_item(
    tx: &mut Transaction<'static, Postgres>,
    type_id: i32,
    container_id: i32,
) -> Result<bool, sqlx::Error> {
    if container_id == INV_MISSION {
        return Ok(true);
    }
    let listed: Option<bool> = sqlx::query_scalar(
        "SELECT $2 = ANY(container_sets) FROM resources.items WHERE item_id = $1",
    )
    .bind(type_id)
    .bind(INV_MISSION)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(listed.unwrap_or(false))
}

/// Whether a move from `source_container_id` to `target_container_id`
/// touches the vault.
pub(super) fn touches_vault(source_container_id: i32, target_container_id: i32) -> bool {
    source_container_id == INV_BANK || target_container_id == INV_BANK
}

/// One committed move that touched the vault, for `move_accepted`.
#[derive(Debug, Clone, Copy)]
pub(super) struct AcceptedVaultMove {
    pub owner: VaultOwner,
    pub entity_id: u32,
    pub player_id: i32,
    pub item_id: i32,
    pub type_id: i32,
    pub quantity: i32,
    pub source_container_id: i32,
    pub source_slot_id: i32,
    pub target_container_id: i32,
    pub target_slot_id: i32,
    /// `deposit`, `withdraw`, `split`, `merge` or `swap`, plus `within`
    /// for a move from one vault slot to another.
    pub kind: &'static str,
    pub source_stack_before: i32,
    pub source_stack_after: i32,
    pub target_stack_before: i32,
    pub target_stack_after: i32,
}

/// `move_accepted` (DEBUG, target `bank`), logged after the commit.
pub(super) fn log_move_accepted(m: &AcceptedVaultMove, vault: &VaultAccess) {
    let player_label = known_names::player_name(m.player_id);
    tracing::debug!(
        target: "bank",
        event = "move_accepted",
        account_id = m.owner.account_id,
        account_name = known_names::account_name(m.owner.account_id),
        player_id = m.player_id,
        player_name = player_label,
        entity_id = m.entity_id,
        entity_name = player_label,
        item_id = m.item_id,
        item_type_id = m.type_id,
        item_name = cimmeria_names::book().item(m.type_id),
        quantity = m.quantity,
        kind = m.kind,
        source_container_id = m.source_container_id,
        source_container_name = cimmeria_names::book().container(m.source_container_id),
        source_slot_id = m.source_slot_id, // nt:id-only slot index, unnamed
        target_container_id = m.target_container_id,
        target_container_name = cimmeria_names::book().container(m.target_container_id),
        target_slot_id = m.target_slot_id, // nt:id-only slot index, unnamed
        source_stack_before = m.source_stack_before,
        source_stack_after = m.source_stack_after,
        target_stack_before = m.target_stack_before,
        target_stack_after = m.target_stack_after,
        bank_slots = i32::from(m.owner.bank_slots),
        banker_id = vault.banker_id(), // nt:id-only banker NPC, unnamed on the base
        gm_override = vault.gm_override(),
        distance = vault.distance(),
        "move_accepted: vault move committed"
    );
}

/// The `kind` label of a committed vault move.
pub(super) fn move_kind(
    source_container_id: i32,
    target_container_id: i32,
    shape: MoveShape,
) -> &'static str {
    match shape {
        MoveShape::Split => "split",
        MoveShape::Merge => "merge",
        MoveShape::Swap => "swap",
        MoveShape::Whole if source_container_id == target_container_id => "within",
        MoveShape::Whole if target_container_id == INV_BANK => "deposit",
        MoveShape::Whole => "withdraw",
    }
}

/// Which write a move made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MoveShape {
    /// The whole stack into an empty slot.
    Whole,
    /// Part of the stack into an empty slot (a new row).
    Split,
    /// Into a stack of the same type with room for it.
    Merge,
    /// The whole stack with the occupant, which takes the source slot.
    Swap,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::cell_entity::VaultScope;

    /// The labels are what SigNoz queries and the `LogCapture` guards
    /// match, so a rename must be deliberate.
    #[test]
    fn refusal_reasons_are_stable() {
        assert_eq!(
            MoveRefusal::NotPlayerMovable(MoveEnd::Source).reason(),
            "source_container_not_player_movable"
        );
        assert_eq!(
            MoveRefusal::NotPlayerMovable(MoveEnd::Target).reason(),
            "target_container_not_player_movable"
        );
        assert_eq!(
            vault_session_refusal(MoveEnd::Target, &VaultAccess::NO_SESSION)
                .map(MoveRefusal::reason),
            Some("no_vault_session")
        );
        assert_eq!(
            MoveRefusal::BankSlotLocked { bank_slots: 40 }.reason(),
            "target_slot_beyond_bank_slots"
        );
        assert_eq!(
            MoveRefusal::MissionItem.reason(),
            "mission_item_not_bankable"
        );
    }

    #[test]
    fn an_open_verdict_refuses_nothing() {
        let open = VaultAccess::Open {
            org_id: None,
            scope: VaultScope::Personal,
            banker_id: Some(7),
            distance: Some(1.0),
        };
        assert_eq!(vault_session_refusal(MoveEnd::Source, &open), None);
    }

    /// Every bank refusal tells the player why; the BV-01 allowlist
    /// refusals keep the bare snap-back.
    #[test]
    fn bank_refusals_carry_a_feedback_line() {
        let out_of_range = MoveRefusal::VaultSession {
            end: MoveEnd::Target,
            reason: "banker_out_of_range",
        };
        assert!(out_of_range.feedback().unwrap().contains("too far"));
        assert!(MoveRefusal::BankSlotLocked { bank_slots: 50 }
            .feedback()
            .unwrap()
            .contains("50 slots"));
        assert!(MoveRefusal::MissionItem.feedback().is_some());
        assert_eq!(
            MoveRefusal::NotPlayerMovable(MoveEnd::Source).feedback(),
            None
        );
    }

    #[test]
    fn move_kinds() {
        assert_eq!(move_kind(1, 17, MoveShape::Whole), "deposit");
        assert_eq!(move_kind(17, 1, MoveShape::Whole), "withdraw");
        assert_eq!(move_kind(17, 17, MoveShape::Whole), "within");
        assert_eq!(move_kind(1, 17, MoveShape::Split), "split");
        assert_eq!(move_kind(17, 1, MoveShape::Merge), "merge");
        assert_eq!(move_kind(17, 1, MoveShape::Swap), "swap");
    }
}
