//! `ChatCellToBase`: chat traffic from the cell to the base, carried by
//! `CellToBaseMsg::Chat`.
//!
//! One nested enum, so later chat packets add a variant here instead of in
//! `cell_to_base.rs` (the organizations pattern, work-packets.md § Messages).
//! Every variant carries the actor's ids **from the cell's own session
//! state** (`CellEntity`), never from a client payload, and never a
//! privilege bit.

/// Chat messages sent from CellApp to BaseApp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatCellToBase {
    /// A GM broadcast to every online player (SS-C2, D-SS16): `sendGMShout`
    /// (cell method 222) with `isGlobal != 0`, or `.announce <text>`.
    ///
    /// The cell has already passed the GM gate (access level GameMaster or
    /// higher, read from `CellEntity::access_level`) and the D-SS12 text
    /// rules. `args` is the finished `onPlayerCommunication` payload from
    /// `cimmeria_wire::cell::chat::serialize_gm_broadcast`; the base only fans
    /// it out, to every session in the online index that has a player
    /// entity. The space scope never reaches the base: the cell knows its
    /// own space's players.
    GmBroadcast {
        /// The GM's entity id (for the log only; nothing is addressed to it).
        entity_id: u32,
        /// The GM's `sgw_player.player_id`, `None` if the cell has none.
        player_id: Option<i32>,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// Where the broadcast came from: `"native"` (CM 222) or
        /// `"console"` (`.announce`). A fixed label, for the log.
        source: &'static str,
        /// The serialized `onPlayerCommunication` args.
        args: Vec<u8>,
    },
}

impl ChatCellToBase {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            ChatCellToBase::GmBroadcast { .. } => "gm_broadcast",
        }
    }
}
