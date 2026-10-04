//! The dial-side address-book gate (security finding CAT-O-01).
//!
//! `onDialGate` carries `targetAddressId` as a raw client `INT32`. The 2009
//! server refused any address the character did not hold
//! (`deprecated/python/cell/SGWPlayer.py:2060-2064`); Cimmeria loaded
//! `known_stargates` from the DB, serialised it to the client in
//! `setupStargateInfo`, and then read it nowhere on the server, so any
//! client could dial any of the 28 seeded gates and cross-world teleport
//! itself into content it never unlocked.
//!
//! One function, one call site — the top of
//! [`super::handle_dial_gate`]. Deliberately its own file so the
//! authorization surface is a thing you can point at, and so the gate does
//! not drift into the dial state machine next to it. What the player is told
//! on a refusal lives in [`super::dial_feedback`].

use super::dial_feedback::DialRefusal;
use crate::cell::space_manager::SpaceManager;

/// Does this player's address book contain `target_address_id`?
///
/// On refusal returns the [`DialRefusal`] to report; the caller cancels any
/// dial in flight, tells the player (`dial_feedback::send_dial_refusal`) and
/// returns without touching any other state.
///
/// **No GM exemption here, deliberately.** 2009 had none either, and an
/// `access_level` branch would put a second authorization surface on a check
/// whose whole value is having exactly one. `gmDHD` is the one caller that
/// needs an address it may not hold, and it solves that by topping up the
/// caller's in-memory book before dialling — see
/// `cimmeria_cell_console::cell::console::gm::travel`, which is already authorised
/// against the session's access level by the cell-method GM gate.
pub(super) fn player_knows_stargate(
    entity_id: u32,
    target_address_id: i32,
    space_mgr: &SpaceManager,
) -> Result<(), DialRefusal> {
    let Some(entity) = space_mgr.get_entity(entity_id) else {
        // Front-runs `handle_dial_gate`'s own entity-missing branch, which
        // keys on the space binding rather than the entity. Both refuse;
        // this one gets there first because the address book lives on the
        // entity.
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            target_address_id,
            target_address_name = cimmeria_names::book().stargate(target_address_id),
            reason = "dial_entity_missing",
            "onDialGate: no cell entity for the caller — refusing the dial; \
             the client is told to try again"
        );
        return Err(DialRefusal::NotInWorld);
    };
    if entity.known_stargates.contains(&target_address_id) {
        return Ok(());
    }

    // `known_count = 0` has two causes worth telling apart when triaging a
    // "my gate is greyed out" report: a character that genuinely holds no
    // addresses (the column defaults to `'{}'` and `base::character_create`
    // does not seed it), or a dial sent inside the window between
    // `CreateEntity` and `InitPlayerState`, where the entity exists but its
    // address book has not arrived. Both refuse, which is the right
    // direction; the count is in the fields so the log can distinguish them
    // from "holds addresses, just not this one".
    tracing::warn!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        player_id = entity.player_id,
        player_name = entity.identity().player_name,
        target_address_id,
        target_address_name = cimmeria_names::book().stargate(target_address_id),
        known_count = entity.known_stargates.len(),
        reason = "unknown_stargate_address",
        "onDialGate: address is not in the player's known list — refusing the dial; \
         the traveller stays put and the client is told why"
    );
    Err(DialRefusal::UnknownAddress)
}
