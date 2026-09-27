//! `ChatCellToBase`: chat traffic from the cell to the base, carried by
//! `CellToBaseMsg::Chat`.
//!
//! One nested enum, so later chat packets add a variant here instead of in
//! `cell_to_base.rs` (the organizations pattern, work-packets.md § Messages).
//! Every variant carries the actor's ids **from the cell's own session
//! state** (`CellEntity`), never from a client payload, and never a
//! privilege bit.

/// Longest GM mute, in minutes (7 days), shared by the cell's `.mute`
/// parser and the base's handler. Project policy, not recovered data: a mute
/// ends at the next restart anyway (D-SS26), and the bound keeps
/// `now + duration` far from `Instant` overflow.
pub const MAX_MUTE_MINUTES: u32 = 7 * 24 * 60;

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
    /// `.mute <name> <minutes> [reason]` (SS-C3, D-SS26). The cell has
    /// passed the GM gate and bounded `minutes` and `reason`; the base
    /// resolves the name among online characters, writes its `MuteTable`
    /// and answers the GM and the player.
    Mute {
        /// The GM's entity id, where the base sends the GM's answer.
        entity_id: u32,
        /// The GM's `sgw_player.player_id`, `None` if the cell has none.
        player_id: Option<i32>,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The character name the GM typed (resolved per D-SS13).
        target_name: String,
        /// 1 to `MAX_MUTE_MINUTES`.
        minutes: u32,
        /// The GM's stated reason, logged only; empty when none was given.
        reason: String,
    },
    /// `.unmute <name>` (SS-C3, D-SS26).
    Unmute {
        /// The GM's entity id, where the base sends the GM's answer.
        entity_id: u32,
        /// The GM's `sgw_player.player_id`, `None` if the cell has none.
        player_id: Option<i32>,
        /// The GM's `account.account_id`, `None` if the cell has none.
        account_id: Option<u32>,
        /// The character name the GM typed (resolved per D-SS13).
        target_name: String,
    },
}

impl ChatCellToBase {
    /// Short stable name for the `kind` log field.
    pub fn kind(&self) -> &'static str {
        match self {
            ChatCellToBase::GmBroadcast { .. } => "gm_broadcast",
            ChatCellToBase::Mute { .. } => "gm_mute",
            ChatCellToBase::Unmute { .. } => "gm_unmute",
        }
    }
}
