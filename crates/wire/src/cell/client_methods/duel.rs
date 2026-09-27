//! Duel client methods and the duel feedback texts.
//!
//! - `onDuelChallenge` [143]: the target's Yes/No prompt (SS-D1).
//! - `onDuelEntitiesSet` [151] with both duelists at the engage, and
//!   `onDuelEntitiesClear` [153] at every end (SS-D2). SS-E1 D-Q5 showed
//!   they only edit a client-side set that the interactability check never
//!   reads, so they are safe beside AoI's use of 152 (audit A-41).
//! - The PvP flag, `onEntityProperty(GENERICPROPERTY_PvPFlag = 4, v)`. The
//!   client UI reads `Property.PVPFlag` from the same generic-property table
//!   that `onEntityProperty` fills (`UnitFrames.lua`, `Squad.lua`); the
//!   dedicated `pvpFlag` CELL_PUBLIC property is ghost-only and reaches no
//!   client (`duel-wire-formats.md`, SS-D2 section).
//! - The countdown, `onTimerUpdate` with `Type = DuelTimer (14)`. The
//!   client's type-14 handler (`0x00dec9e0`) turns `BigWorldTimeComplete`
//!   into seconds remaining and raises `Event_UI_DuelTimerStart`, which the
//!   Lua shows as the 5, 4, 3, 2, 1 splash.
//!
//! # Feedback texts
//!
//! The strings are the client's own duel monikers from `texts.sql`
//! (872-878), sent as literal `onPlayerCommunication` feedback lines. SS-E1
//! found no client path that renders a duel moniker by id (`onErrorCode`
//! is ruled out), so the text travels, not the id
//! (`docs/reverse-engineering/findings/duel-wire-formats.md`, D-Q6). The
//! lines with no client moniker are Cimmeria's own wording.

pub use super::being::ON_TIMER_UPDATE;
pub use super::player::{ON_DUEL_CHALLENGE, ON_DUEL_ENTITIES_CLEAR, ON_DUEL_ENTITIES_SET};
pub use super::spawnable_entity::ON_ENTITY_PROPERTY;

/// `EEntityPropertyType::GENERICPROPERTY_PvPFlag` (`enumerations.xml`).
pub const GENERICPROPERTY_PVP_FLAG: i32 = 4;

/// `ETimerUpdateType::DuelTimer` (`enumerations.xml`).
pub const TIMER_DUEL: i8 = 14;

/// Moniker 872.
pub const TEXT_CHALLENGE_SELF: &str = "You can not challenge yourself to a duel";
/// Moniker 873: the challenger is already in a duel or a pending challenge.
pub const TEXT_ALREADY_IN_DUEL: &str = "You are already involved in a duel";
/// Moniker 877: different space, or beyond the challenge range (D-SS19).
pub const TEXT_NOT_CLOSE_ENOUGH: &str = "You are not close enough to send a duel request";
/// Moniker 878: a declined or expired challenge, told to both sides.
pub const TEXT_DUEL_ABORTED: &str = "Duel aborted";

/// Squad duels are refused. Moniker 874 ("You cannot start a squad duel
/// when not in a squad") would be false for a squad member, so the line is
/// Cimmeria's own.
pub const TEXT_SQUAD_DUEL_UNSUPPORTED: &str = "Squad duels are not available.";
/// The typed name matched no online character.
pub const TEXT_TARGET_NOT_ONLINE: &str = "That player is not online.";
/// The typed name matched more than one online character (D-SS13).
pub const TEXT_TARGET_AMBIGUOUS: &str =
    "More than one player matches that name. Type the exact name.";
/// The target ignores the challenger (D-SS15).
pub const TEXT_TARGET_IGNORING: &str = "That player is not accepting your duel challenges.";
/// The challenger is not yet client-ready (world entry or gate travel).
pub const TEXT_CHALLENGER_LOADING: &str =
    "You cannot send a duel challenge while entering the world.";
/// The target is entering the world (world entry or gate travel).
pub const TEXT_TARGET_LOADING: &str = "That player is entering the world. Try again in a moment.";
/// The prompt to the target could not be queued; the challenge is dropped.
pub const TEXT_CHALLENGE_UNDELIVERED: &str = "Your duel challenge could not be delivered.";
/// The target is already in a duel or a pending challenge (D-SS21).
pub const TEXT_TARGET_BUSY: &str = "That player is already involved in a duel.";
/// The per-pair cooldown after a decline or expiry is running (D-SS21).
pub const TEXT_PAIR_COOLDOWN: &str = "You cannot challenge that player again yet.";
/// The challenge went out: the challenger's acknowledgement.
pub const TEXT_CHALLENGE_SENT: &str = "Duel challenge sent.";
/// A response with no pending challenge addressed to the caller.
pub const TEXT_NO_PENDING_CHALLENGE: &str = "You have no duel challenge to answer.";
/// Accept: both sides, before the countdown.
pub const TEXT_DUEL_ACCEPTED: &str = "Duel accepted. The duel starts in 5 seconds.";
/// The countdown ran out and the duel is engaged: both sides.
pub const TEXT_DUEL_ENGAGED: &str = "The duel has begun.";

/// `onEntityProperty(GENERICPROPERTY_PvPFlag, flag)`: two `INT32`s, the
/// property id then the value (0 or 1).
pub fn build_pvp_flag(flagged: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    out.extend_from_slice(&GENERICPROPERTY_PVP_FLAG.to_le_bytes());
    out.extend_from_slice(&i32::from(flagged).to_le_bytes());
    out
}

/// `onDuelEntitiesSet(ARRAY<INT32> aEntityList)` [151]: a `u32` count, then
/// the entity ids.
pub fn build_on_duel_entities_set(entity_ids: &[i32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + entity_ids.len() * 4);
    let n = u32::try_from(entity_ids.len()).expect("ARRAY element count exceeds u32");
    out.extend_from_slice(&n.to_le_bytes());
    for id in entity_ids {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out
}

/// `onTimerUpdate(ID, Type = DuelTimer, SourceID, SecondaryId = 0,
/// TotalTime, BigWorldTimeComplete)`: 21 bytes in `SGWBeing.def` order.
/// `complete` is absolute game time (`game_time_secs() + total`); the
/// client shows `complete` minus its own clock, clamped at 0.
pub fn build_duel_timer(
    timer_id: i32,
    source_entity_id: i32,
    total: f32,
    complete: f32,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(21);
    out.extend_from_slice(&timer_id.to_le_bytes());
    out.push(TIMER_DUEL as u8);
    out.extend_from_slice(&source_entity_id.to_le_bytes());
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&total.to_le_bytes());
    out.extend_from_slice(&complete.to_le_bytes());
    out
}

/// `onDuelChallenge(INT32 aEntityId, ARRAY<INT32> aSquadList)` [143]
/// (`SGWPlayer.def:1372-1375`): the challenger's entity id, then a `u32`
/// element count and the squad members' entity ids.
pub fn build_on_duel_challenge(challenger_entity_id: i32, squad: &[i32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + squad.len() * 4);
    out.extend_from_slice(&challenger_entity_id.to_le_bytes());
    let n = u32::try_from(squad.len()).expect("ARRAY element count exceeds u32");
    out.extend_from_slice(&n.to_le_bytes());
    for id in squad {
        out.extend_from_slice(&id.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-exact: the entity id, then an empty array (count 0). Eight
    /// bytes; a one-byte or two-byte count would shift nothing here but
    /// would break the client's array read.
    #[test]
    fn on_duel_challenge_empty_squad_is_byte_exact() {
        assert_eq!(
            build_on_duel_challenge(0x0102_0304, &[]),
            vec![0x04, 0x03, 0x02, 0x01, 0, 0, 0, 0]
        );
        assert_eq!(ON_DUEL_CHALLENGE, 143);
    }

    #[test]
    fn on_duel_challenge_with_squad_is_byte_exact() {
        assert_eq!(
            build_on_duel_challenge(7, &[9, -1]),
            vec![7, 0, 0, 0, 2, 0, 0, 0, 9, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    /// Byte-exact: property id 4, then the value, both `INT32` LE.
    #[test]
    fn pvp_flag_is_byte_exact() {
        assert_eq!(build_pvp_flag(true), vec![4, 0, 0, 0, 1, 0, 0, 0]);
        assert_eq!(build_pvp_flag(false), vec![4, 0, 0, 0, 0, 0, 0, 0]);
    }

    /// Byte-exact: a `u32` count, then each id. An empty list is the bare
    /// count.
    #[test]
    fn on_duel_entities_set_is_byte_exact() {
        assert_eq!(
            build_on_duel_entities_set(&[10, 0x0102_0304]),
            vec![2, 0, 0, 0, 10, 0, 0, 0, 0x04, 0x03, 0x02, 0x01]
        );
        assert_eq!(build_on_duel_entities_set(&[]), vec![0, 0, 0, 0]);
    }

    /// Byte-exact, 21 bytes: id, type 14, source, secondary 0, total,
    /// complete.
    #[test]
    fn duel_timer_is_byte_exact() {
        let b = build_duel_timer(7, 10, 5.0, 105.5);
        assert_eq!(b.len(), 21);
        assert_eq!(&b[0..4], &[7, 0, 0, 0]);
        assert_eq!(b[4], 14);
        assert_eq!(&b[5..9], &[10, 0, 0, 0]);
        assert_eq!(&b[9..13], &[0, 0, 0, 0]);
        assert_eq!(&b[13..17], &5.0f32.to_le_bytes());
        assert_eq!(&b[17..21], &105.5f32.to_le_bytes());
    }

    /// The constants are read back from `entities/defs/`, not from copies
    /// of themselves: the generic-property id and the timer type from
    /// `enumerations.xml`, and the method indices from the flattened
    /// SGWPlayer client-method table.
    #[test]
    fn constants_match_the_entity_defs() {
        use super::super::pet_def_tests::{enum_value, flattened_client_methods, index_of};
        assert_eq!(
            enum_value("GENERICPROPERTY_PvPFlag"),
            GENERICPROPERTY_PVP_FLAG as u64
        );
        assert_eq!(enum_value("DuelTimer"), TIMER_DUEL as u64);
        let player = flattened_client_methods("SGWPlayer");
        assert_eq!(index_of(&player, "onDuelChallenge"), ON_DUEL_CHALLENGE);
        assert_eq!(index_of(&player, "onDuelEntitiesSet"), ON_DUEL_ENTITIES_SET);
        assert_eq!(
            index_of(&player, "onDuelEntitiesClear"),
            ON_DUEL_ENTITIES_CLEAR
        );
        assert_eq!(index_of(&player, "onEntityProperty"), ON_ENTITY_PROPERTY);
        assert_eq!(index_of(&player, "onTimerUpdate"), ON_TIMER_UPDATE);
    }

    /// The moniker strings are the client's `texts.sql` rows, verbatim.
    #[test]
    fn moniker_texts_match_the_seed() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../db/resources/Texts/Seed/texts.sql"
        );
        let seed = std::fs::read_to_string(path).expect("read texts.sql");
        for (id, text) in [
            (872, TEXT_CHALLENGE_SELF),
            (873, TEXT_ALREADY_IN_DUEL),
            (877, TEXT_NOT_CLOSE_ENOUGH),
            (878, TEXT_DUEL_ABORTED),
        ] {
            let row = format!("VALUES ({id}, 0, 1033, '{text}', '')");
            assert!(seed.contains(&row), "texts.sql has no row {row}");
        }
    }
}
