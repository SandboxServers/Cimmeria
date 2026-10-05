//! The GM bulk ability commands (ability-mechanics AB-N2):
//! `gmGiveAllAbilities` (154) and `gmResetAbilities` (153).
//!
//! The cell GM-gates the call and sends [`GmAbilityBulk`]; the base writes
//! `sgw_player.abilities` in one guarded `UPDATE` and answers with
//! [`GmAbilitiesChanged`], which the cell mirrors with one
//! `onKnownAbilitiesUpdate` burst. Both commands act on the calling GM only:
//! neither method has a target argument in `SGWGmPlayer.def`.

/// Where a bulk change came from, so the audit trail names the real
/// source (DA-02 review F4): the typed command, or the Debug Area's ability
/// granter / reset NPC (content action `gm_ability_bulk`). Both are GM-gated
/// on the cell; only the label and the player's lines differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmAbilitySource {
    /// `/gmgiveallabilities` or `/gmresetabilities`, typed by the GM.
    Command,
    /// A click on the Debug Area ability granter or ability reset NPC.
    NpcGranter,
}

impl GmAbilitySource {
    /// The `source` value logs carry.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::NpcGranter => "npc_granter",
        }
    }
}

/// Which bulk change a GM asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmAbilityChange {
    /// `gmGiveAllAbilities`: append every id in `ability_ids` the character
    /// does not know yet. Not a trainer purchase, so no points move and
    /// nothing goes into `trained_abilities` (a respec keeps them).
    GrantAll,
    /// `gmResetAbilities`: set the known abilities to the archetype's
    /// character-creation starters, refund `tree_points_spent` into
    /// `training_points` and clear `trained_abilities`, as the trainer
    /// respec does, but with no trainer, no charge, and quest and GM grants
    /// removed too.
    Reset,
}

impl GmAbilityChange {
    /// The client method name, for logs and feedback.
    pub fn command(self) -> &'static str {
        match self {
            Self::GrantAll => "gmGiveAllAbilities",
            Self::Reset => "gmResetAbilities",
        }
    }
}

/// `CellToBaseMsg::GmAbilityBulk`: a GM bulk ability change for the GM's own
/// character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmAbilityBulk {
    /// The GM's cell entity id.
    pub entity_id: u32,
    /// The GM's character, resolved by the cell.
    pub player_id: i32,
    /// The GM's account, for the telemetry (`None` when unknown).
    pub account_id: Option<u32>,
    pub change: GmAbilityChange,
    /// Who asked: the typed command or the NPC granter.
    pub source: GmAbilitySource,
    /// [`GmAbilityChange::GrantAll`]: the archetype tree's ability ids, in
    /// tree order. Empty for [`GmAbilityChange::Reset`]: the base reads the
    /// starters from `resources.char_creation_abilities` itself.
    pub ability_ids: Vec<i32>,
}

/// `BaseToCellMsg::GmAbilitiesChanged`: what the base wrote for one
/// [`GmAbilityBulk`]. The row is final; the cell mirrors it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GmAbilitiesChanged {
    pub entity_id: u32,
    /// The character the base wrote. The cell ignores the message when
    /// `entity_id` now plays another character.
    pub player_id: i32,
    pub change: GmAbilityChange,
    /// Echoed from the request.
    pub source: GmAbilitySource,
    /// Ids now known that were not before, in row order.
    pub added: Vec<i32>,
    /// Ids no longer known, in their old row order.
    pub removed: Vec<i32>,
    /// `training_points` after the `UPDATE` (a reset refunds the spend).
    pub training_points: i32,
}
