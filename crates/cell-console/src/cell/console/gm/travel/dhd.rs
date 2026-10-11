//! `gmDHD` — stargate dial by numeric address.

use tokio::sync::mpsc;

use super::super::feedback::send_gm_feedback;
use crate::cell::gate_travel::handle_dial_gate;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `gmDHD(INT8 aGateAddress)` — dial a stargate by numeric address.
///
/// Address `0` is the def's "request the address list" sentinel; listing needs
/// a client feedback channel that isn't wired here yet, so a `0` address is a
/// no-op + warn. Any other address reuses [`handle_dial_gate`], which validates
/// the address against the cached stargate table and routes the cross-world
/// `GateTravel`.
pub(in crate::cell::console::gm) async fn handle_dhd(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) -> bool {
    let gate_addr = match args.first() {
        Some(&b) => b as i8, // INT8 (signed)
        None => {
            tracing::warn!(
                entity_id,
                entity_name = space_mgr.entity_label(entity_id),
                "gmDHD: truncated args (need INT8 aGateAddress)"
            );
            send_gm_feedback(entity_id, "gmDHD: missing INT8 aGateAddress", tx).await;
            return true;
        }
    };
    if gate_addr <= 0 {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            gate_addr,
            "gmDHD: address <= 0 (list request) unsupported — needs a client feedback channel"
        );
        send_gm_feedback(
            entity_id,
            "gmDHD: address must be positive (list request unsupported)",
            tx,
        )
        .await;
        return true;
    }
    tracing::info!(
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        gate_addr,
        "gmDHD: dialing stargate"
    );

    // `handle_dial_gate` enforces the player's address book (CAT-O-01), and a
    // GM debugging a world they have never visited will not hold its address.
    // Top the caller's live book up instead of giving the dial primitive a
    // bypass parameter: that would put a second authorization surface on a
    // check whose whole value is having exactly one, and this arm has already
    // been authorised by the cell-method GM gate against the session's
    // `access_level`. In-memory only — a debug dial should not rewrite the
    // character's persisted address book, and the grant is audit-logged.
    // Mirrors 2009's `giveaddress` console command
    // (`deprecated/python/cell/commands/Player.py:74`).
    //
    // Never for an outbound-only dial hub (the Debug Area gate,
    // `debug_dial_hub`): nobody may hold its address, GMs included. The dial
    // below is refused for it anyway, by the address-book check; skipping the
    // grant keeps the id out of the book so nothing else can mistake it for a
    // known destination.
    let addr = i32::from(gate_addr);
    let is_hub = space_mgr
        .stargates
        .get(&addr)
        .is_some_and(|g| g.debug_dial_hub);
    if is_hub {
        tracing::warn!(
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            gate_addr,
            stargate_name = cimmeria_names::book().stargate(addr),
            reason = "gm_address_grant_dial_hub_skipped",
            "gmDHD: the address is an outbound-only dial hub — not granting it; the dial will be refused"
        );
    } else if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        if !entity.known_stargates.contains(&addr) {
            tracing::warn!(
                entity_id,
                gate_addr,
                entity_name = entity.identity().player_name,
                access_level = entity.access_level,
                reason = "gm_address_grant",
                "gmDHD: caller does not hold this stargate address — granting it for \
                 the rest of this session so the GM dial can proceed"
            );
            entity.known_stargates.push(addr);
        }
    }

    // source address is unused by the primitive.
    let dialed = handle_dial_gate(entity_id, i32::from(gate_addr), 0, tx, space_mgr, engine).await;
    // Report the actual outcome. This used to say "dialing" unconditionally,
    // so a refused dial (unknown address, same world, or — since the PR #662
    // review — a destination with no standable arrival) looked identical to a
    // successful one and the GM was left waiting for a load screen that was
    // never coming.
    let feedback = if dialed {
        format!("gmDHD: dialing gate address {gate_addr}")
    } else {
        format!(
            "gmDHD: gate address {gate_addr} refused — unknown address, same \
             world, or no standable arrival on the destination (see server log)"
        )
    };
    send_gm_feedback(entity_id, &feedback, tx).await;
    true
}
