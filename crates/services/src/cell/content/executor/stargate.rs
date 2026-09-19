//! Stargate-address grants — the content-engine port of the 2009 Atrea
//! authoring node `Act_StargateAddress`.
//!
//! In 2009 stargate addresses were **authored content**.
//! `SGWPlayer.addStargateAddress` (`deprecated/python/cell/SGWPlayer.py:609`)
//! had exactly two callers: this node
//! (`deprecated/entities-editor/editor/Nodes.xml:2428`) and the GM console
//! command `giveaddress` (`deprecated/python/cell/commands/Player.py:74`).
//! The node was never ported, so no chain in Cimmeria could unlock a
//! destination — and with H06's dial gate enforced, a character who has
//! never travelled can dial nothing at all.
//!
//! ## Three legs, and why they all live on the cell
//!
//! A grant has to reach three places (H06 worknote, integration request 6):
//!
//! 1. `CellEntity::known_stargates` — what `handle_dial_gate` enforces
//!    against. Written here, in memory, immediately.
//! 2. The client's address book — the DHD only offers addresses it was
//!    told about, and it is only ever handed the full list once, by
//!    `setupStargateInfo` at map load. A mid-session grant is invisible
//!    without `updateStargateAddress` (client method 66).
//! 3. `sgw_player.known_stargates` — so it survives a relog.
//!
//! Legs 1 and 2 are emitted together, from here, *before* leg 3 is
//! confirmed. That ordering is deliberate: the cell's copy is the thing
//! the dial gate reads, so making the client's copy wait on a database
//! round trip would open exactly the divergence H06 closed on the arrival
//! path — the server accepting a dial the client's UI does not offer. A
//! lost leg-3 write costs the address at next login and warns loudly;
//! a lost leg-2 send makes a granted address undialable with no error.

use tokio::sync::mpsc;

use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::player_journal;
use crate::cell::space_manager::SpaceManager;

/// `updateStargateAddress`'s `hasAddress` argument. 1 = the player now
/// holds it; 2009's `removeStargateAddress` sent 0 here, and this action
/// is grant-only.
const HAS_ADDRESS: u8 = 1;

/// `updateStargateAddress`'s `hidden` argument. Always 0: Cimmeria has no
/// hidden address list — no column, no wire slot, and
/// `mercury::world_data::map_loaded` hardcodes an empty hidden array — so
/// the only list an address can land in is the known one.
const NOT_HIDDEN: u8 = 0;

/// `Action::GrantStargateAddress` — teach the acting player one address.
///
/// Idempotent by construction: a player who already holds the address
/// gets no in-memory write, no client method and no persistence message,
/// so a chain that fires twice (a re-run victory chain, a re-accepted
/// mission) cannot double-append. The DB append is independently
/// idempotent as well, because these two are not the same lock.
pub(super) async fn grant_stargate_address(
    stargate_id: i32,
    entity_id: u32,
    player_id: i32,
    chain_id: i64,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Validate the id against the same `resources.stargates` cache the dial
    // handler resolves destinations from. A grant for an id with no gate
    // would sit in the player's book and in their DHD forever, dialable and
    // guaranteed to be refused one layer down — an authoring typo that
    // looks like a server bug in play.
    if !space_mgr.stargates.contains_key(&stargate_id) {
        tracing::warn!(
            entity_id,
            stargate_id,
            chain_id,
            known_gate_count = space_mgr.stargates.len(),
            reason = "grant_unknown_stargate",
            "grant_stargate_address: no resources.stargates row for this id -- \
             nothing granted; check the seed row's target_id against \
             db/resources/Worlds/Seed/stargates.sql"
        );
        return;
    }

    // Only a player has an address book. Every dispatcher derives the
    // executor's `player_id` as `entity.player_id.unwrap_or(0)`, so a
    // non-positive value means the actor carries no DB character — an NPC,
    // a prop, or a player whose row never loaded. Checked before the entity
    // lookup because it is the cheaper and more specific refusal: an NPC
    // reaching here is an authoring bug (the chain fired on the wrong
    // actor), not a lifecycle race.
    if player_id <= 0 {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            chain_id,
            reason = "grant_non_player_actor",
            "grant_stargate_address: acting entity carries no DB player id -- address \
             not granted; the chain fired against an NPC, a prop, or an unloaded player"
        );
        return;
    }

    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            chain_id,
            reason = "grant_entity_missing",
            "grant_stargate_address: no cell entity for the actor -- address not granted"
        );
        return;
    };

    if entity.known_stargates.contains(&stargate_id) {
        tracing::debug!(
            entity_id,
            player_id,
            stargate_id,
            chain_id,
            "grant_stargate_address: already known -- no-op"
        );
        return;
    }

    entity.known_stargates.push(stargate_id);
    let known_count = entity.known_stargates.len();

    tracing::info!(
        entity_id,
        player_id,
        stargate_id,
        chain_id,
        known_count,
        "Content: granting stargate address"
    );
    // The player learned a destination. A `.bug` bookmark should be able to
    // answer "could they dial it yet?" without a database read — the
    // 2026-09-18 Castle playtest lesson, where "the door does nothing" and
    // "the server refused it" were indistinguishable in-game.
    player_journal::note(
        entity_id,
        player_journal::kinds::STARGATE_ADDRESS,
        format!("granted={stargate_id} chain={chain_id}"),
    );

    // updateStargateAddress(INT32 addressId, UINT8 hasAddress, UINT8 hidden)
    // — `entities/defs/interfaces/GateTravel.def`. 2009 emitted exactly
    // this on every add (`SGWPlayer.py:626`,
    // `updateStargateAddress(addressId, 1, 1 if isHidden else 0)`).
    let mut args = Vec::with_capacity(6);
    args.extend_from_slice(&stargate_id.to_le_bytes());
    args.push(HAS_ADDRESS);
    args.push(NOT_HIDDEN);
    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: UPDATE_STARGATE_ADDRESS,
            args,
        })
        .await
    {
        tracing::warn!(
            entity_id,
            player_id,
            stargate_id,
            chain_id,
            reason = "grant_notify_send_failed",
            "grant_stargate_address: updateStargateAddress could not be enqueued ({e}) -- \
             the server will accept the dial but the client's DHD will not offer \
             the address until the next map load"
        );
    }

    if let Err(e) = tx
        .send(CellToBaseMsg::GrantStargateAddress {
            entity_id,
            player_id,
            stargate_id,
        })
        .await
    {
        tracing::error!(
            entity_id,
            player_id,
            stargate_id,
            chain_id,
            reason = "grant_persist_send_failed",
            "grant_stargate_address: cell->base send failed ({e}) -- the address works \
             for this session only and is lost on relog"
        );
    }
}
