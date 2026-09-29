//! `.giveammo` for one player (`CellToBaseMsg::GmGiveAmmo`, ammo campaign
//! AM-06, issue #1026).

/// A GM's `.giveammo`, already GM-gated and validated by the cell's
/// `.`-console: `ammo_type` is a special `EAmmoType` ordinal with a reserve
/// item, and `rounds` is in `1..=` the console's cap.
///
/// The base grants through `AmmoReserve::return_rounds`, so the rounds top
/// up existing stacks to the item's `max_stack_size` before opening new
/// ones, and whatever does not fit in the carried bags is reported back to
/// the GM, never written as an over-cap stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GmGiveAmmo {
    /// The recipient's cell entity id.
    pub entity_id: u32,
    /// The recipient's `sgw_player.player_id`, resolved by the cell.
    pub player_id: i32,
    /// The GM who typed the command; the base sends it the result line.
    pub gm_entity_id: u32,
    /// The GM's character, so the base answers only while that entity still
    /// plays it.
    pub gm_player_id: i32,
    /// The GM's account, for the telemetry (`None` when not threaded in).
    pub gm_account_id: Option<u32>,
    /// `EAmmoType` ordinal (`cimmeria_entity::ammo_type`).
    pub ammo_type: i32,
    /// The reserve item the cell resolved from `ammo_item_types`, for the
    /// feedback and telemetry; the base re-resolves it in SQL.
    pub item_id: i32,
    /// Rounds to grant.
    pub rounds: i32,
}
