//! DHD (Dial Home Device) interaction — `onDisplayDHD` (flat index 120).
//!
//! Reference: `deprecated/python/cell/interactions/DHD.py`. The 2009 handler
//! looks up the stargate belonging to the player's current world and calls
//! `player.client.onDisplayDHD(gate.addressOrigin)`; if the world has no
//! gate it sends an error instead.
//!
//! The known-address list the dialling UI filters against is **not** sent
//! here — it rides `setupStargateInfo` at world entry. The wire signature is
//! a single byte (`entities/defs/SGWPlayer.def:1236-1238`,
//! `docs/protocol/client-method-dispatch-table.md:267`).

use std::ops::RangeInclusive;

use tokio::sync::mpsc;

use cimmeria_entity::interaction_flags::INT_DHD;

use crate::cell::client_methods::player::ON_DISPLAY_DHD;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Legal values of `stargates.address_origin` — the point-of-origin glyph
/// the client renders on the DHD ring.
///
/// There are 38 authored glyphs. The wire slot is a `UINT8`
/// (`entities/defs/SGWPlayer.def:1236-1238`), so the type alone admits
/// 0..=255; the column is `INT32`, so it admits anything. Neither is the
/// domain, and a value outside this range is a seed defect that would
/// otherwise reach the client as a missing symbol.
const ADDRESS_ORIGIN_RANGE: RangeInclusive<u8> = 1..=38;

/// If `target_entity_id` is a DHD, emit `onDisplayDHD` and return `true`.
///
/// Mirrors [`super::trainer::try_open_trainer`]'s shape: a fast flag test so
/// the 99% of interactions that aren't DHDs cost one bitwise AND, and a
/// `bool` so the caller can fall through to the generic dispatch.
///
/// The caller has already distance-checked the target.
pub(crate) async fn try_open_dhd(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let is_dhd = space_mgr
        .get_entity(target_entity_id)
        .is_some_and(|t| t.interaction_type_flags & INT_DHD != 0);
    if !is_dhd {
        return false;
    }

    let world_name = match space_mgr.get_entity_world_name(entity_id) {
        Some(w) => w,
        None => {
            tracing::warn!(
                entity_id,
                target_entity_id,
                reason = "entity_world_unknown",
                "onDisplayDHD: interacting entity is in no space — DHD not opened"
            );
            return true;
        }
    };

    // "The gate on this world." `space_mgr.stargates` is a HashMap, so pick
    // deterministically by the lowest `stargate_id` rather than by iteration
    // order — two worlds carry more than one gate row and an unordered pick
    // would hand the client a different origin glyph across restarts.
    // 2009 picks "the first gate whose world matches" (`DHD.py:22-25`), which
    // in its DefMgr ordering is the same thing.
    let gate = space_mgr
        .stargates
        .iter()
        .filter(|(_, g)| g.world_name == world_name)
        .min_by_key(|(id, _)| **id)
        .map(|(id, g)| (*id, g.address_origin));

    let (stargate_id, address_origin) = match gate {
        Some(g) => g,
        None => {
            // 2009 sends `player.onError("Cannot display DHD: No stargate
            // found on this world")` here. We have no equivalent free-text
            // error channel (`onErrorCode` is an enum-coded surface), so the
            // player sees nothing — log it so the seed gap is greppable.
            tracing::warn!(
                entity_id,
                target_entity_id,
                world_name = %world_name,
                reason = "no_stargate_for_world",
                "onDisplayDHD: a DHD prop is spawned on a world with no \
                 stargates row — the dialling UI cannot open; seed the gate \
                 or remove the prop"
            );
            return true;
        }
    };

    // `address_origin` is a point-of-origin GLYPH, not an identifier: it
    // repeats across rows (value 1 on both `SGC W2` and `SGC`, 13 on both
    // Dakara E2 and E3). Never key `stargates` by it — that map is keyed by
    // `stargate_id`.
    //
    // Validate against the *glyph* range, not just the wire's UINT8 range.
    // `u8::try_from` alone (the pre-review guard) accepted 0 and 39..=255 —
    // every one of which serialises cleanly and renders a DHD the client has
    // no symbol for, which reads as a client bug rather than the seed error
    // it is. 1..=38 is the authored domain; anything else is a data defect
    // and refusing to emit is what makes it findable.
    let origin_byte = match u8::try_from(address_origin) {
        Ok(b) if ADDRESS_ORIGIN_RANGE.contains(&b) => b,
        _ => {
            tracing::warn!(
                entity_id,
                stargate_id,
                address_origin,
                min = *ADDRESS_ORIGIN_RANGE.start(),
                max = *ADDRESS_ORIGIN_RANGE.end(),
                world_name = %world_name,
                reason = "address_origin_out_of_range",
                "onDisplayDHD: stargates.address_origin is outside the 1-38 \
                 point-of-origin glyph range — refusing to emit rather than \
                 rendering a glyph the client has no symbol for; fix the seed row"
            );
            return true;
        }
    };

    // NOTE for packet H06: the 2009 code carries an explicit
    // `# TODO: Check that the player is interacting with a DHD when an
    // onDialGate rpc arrives` (`DHD.py:18`), and this branch is the only
    // place that could record the "DHD is open" fact H06 would enforce
    // against. It is deliberately NOT recorded here: the per-player store it
    // would live in belongs to the dial state machine, which Castle packet
    // CA10 owns (`SpaceManager::pending_gate_dials`). H06 should add a
    // timestamped stamp there rather than a second store.

    tracing::info!(
        entity_id,
        target_entity_id,
        stargate_id,
        address_origin = origin_byte,
        world_name = %world_name,
        "interact: DHD → onDisplayDHD"
    );

    if let Err(e) = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_DISPLAY_DHD,
            args: vec![origin_byte],
        })
        .await
    {
        tracing::warn!(
            entity_id,
            target_entity_id,
            "onDisplayDHD: cell→base send failed -- the dialling UI will not \
             open for the player: {e}"
        );
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::spawner::StargateEntry;
    use crate::test_support::make_space_manager;
    use tokio::sync::mpsc;

    fn gate(world: &str, address_origin: i32) -> StargateEntry {
        StargateEntry {
            world_name: world.to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            address_origin,
            arrival: None,
            event_set_id: None,
        }
    }

    /// Player 1 plus a DHD prop (entity 100) in Agnos.
    fn mgr_with_dhd() -> SpaceManager {
        let mut mgr = make_space_manager();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr.spawn_npc(100, "Agnos", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(prop) = mgr.get_entity_mut(100) {
            prop.interaction_type_flags = INT_DHD;
        }
        mgr
    }

    fn drain_dhd(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Option<Vec<u8>> {
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } = msg
            {
                if method_index == ON_DISPLAY_DHD {
                    return Some(args);
                }
            }
        }
        None
    }

    /// Wire shape: `onDisplayDHD` is flat index 120 with a single
    /// `UINT8 PointOfOrigin` (`entities/defs/SGWPlayer.def:1236-1238`), and
    /// the glyph is the one belonging to the world the player is standing
    /// on — not the gate they are dialling.
    #[tokio::test]
    async fn dhd_interact_emits_120_with_one_byte_the_worlds_origin_glyph() {
        let mut mgr = mgr_with_dhd();
        mgr.stargates.insert(15, gate("Agnos", 15));
        // A gate on another world must not be picked.
        mgr.stargates.insert(3, gate("Harset", 6));

        let (tx, mut rx) = mpsc::channel(16);
        assert!(try_open_dhd(1, 100, &tx, &mut mgr).await);

        let args = drain_dhd(&mut rx).expect("expected onDisplayDHD (120)");
        assert_eq!(args, vec![15u8], "one byte, the local world's origin glyph");
    }

    /// Lowest `stargate_id` wins, so two gates on one world cannot hand the
    /// client a different glyph across restarts (`stargates` is a HashMap).
    ///
    /// Six candidates, not two: with a two-entry map an unordered pick
    /// (`.next()` instead of `.min_by_key`) would still land on the right one
    /// about half the time and the guard would be a coin flip.
    #[tokio::test]
    async fn dhd_picks_the_lowest_stargate_id_on_the_world() {
        let mut mgr = mgr_with_dhd();
        for (id, origin) in [(27, 1), (31, 2), (44, 3), (52, 4), (63, 5), (17, 9)] {
            mgr.stargates.insert(id, gate("Agnos", origin));
        }

        let (tx, mut rx) = mpsc::channel(16);
        assert!(try_open_dhd(1, 100, &tx, &mut mgr).await);
        assert_eq!(drain_dhd(&mut rx), Some(vec![9u8]));
    }

    /// `ON_DISPLAY_DHD` is only ever compared against itself elsewhere in the
    /// suite, so a wrong constant in `client_methods/player.rs` would leave
    /// every test green while the client received a different RPC. Pin the
    /// literal against the def (`entities/defs/SGWPlayer.def:1236`) and the
    /// dispatch table.
    #[test]
    fn on_display_dhd_is_flat_index_120() {
        assert_eq!(ON_DISPLAY_DHD, 120);
    }

    /// `as u8` would wrap 256 to a perfectly in-range-looking 0 and render a
    /// silently wrong DHD. Swapping `u8::try_from` back to `as u8` makes
    /// this emit `[0]` instead of nothing.
    #[tokio::test]
    async fn dhd_refuses_to_emit_an_out_of_range_origin_glyph() {
        let mut mgr = mgr_with_dhd();
        mgr.stargates.insert(15, gate("Agnos", 256));

        let (tx, mut rx) = mpsc::channel(16);
        assert!(
            try_open_dhd(1, 100, &tx, &mut mgr).await,
            "the interaction is still claimed — it just doesn't emit"
        );
        assert_eq!(drain_dhd(&mut rx), None);
    }

    /// PR #662 review, finding 3. The glyph domain is 1-38, but the
    /// pre-review guard was `u8::try_from` alone — so 0, 39 and 255 all
    /// serialised cleanly and reached the client as a DHD with no symbol to
    /// render, which reads as a client bug rather than the seed error it is.
    ///
    /// Reverting the `ADDRESS_ORIGIN_RANGE.contains(&b)` arm makes all three
    /// rejected rows emit.
    #[tokio::test]
    async fn dhd_refuses_an_origin_glyph_outside_the_authored_1_to_38_range() {
        for bad in [0, 39, 255] {
            let mut mgr = mgr_with_dhd();
            mgr.stargates.insert(15, gate("Agnos", bad));

            let (tx, mut rx) = mpsc::channel(16);
            assert!(
                try_open_dhd(1, 100, &tx, &mut mgr).await,
                "the interaction is still claimed — it just doesn't emit"
            );
            assert_eq!(
                drain_dhd(&mut rx),
                None,
                "address_origin {bad} is outside the 1-38 glyph range and must \
                 not reach the client"
            );
        }
    }

    /// Positive control for the guard above: both ends of the authored
    /// range still emit. Without this, tightening the check to something
    /// absurd (`38..=38`) would leave the rejection test green.
    #[tokio::test]
    async fn dhd_emits_both_ends_of_the_authored_glyph_range() {
        for good in [1u8, 38u8] {
            let mut mgr = mgr_with_dhd();
            mgr.stargates.insert(15, gate("Agnos", i32::from(good)));

            let (tx, mut rx) = mpsc::channel(16);
            assert!(try_open_dhd(1, 100, &tx, &mut mgr).await);
            assert_eq!(
                drain_dhd(&mut rx),
                Some(vec![good]),
                "address_origin {good} is a legal glyph and must be emitted"
            );
        }
    }

    /// A DHD prop on a world with no `stargates` row is a seed gap, not a
    /// crash and not a bogus emit.
    #[tokio::test]
    async fn dhd_on_a_world_with_no_gate_emits_nothing() {
        let mut mgr = mgr_with_dhd();
        mgr.stargates.insert(3, gate("Harset", 6));

        let (tx, mut rx) = mpsc::channel(16);
        assert!(try_open_dhd(1, 100, &tx, &mut mgr).await);
        assert_eq!(drain_dhd(&mut rx), None);
    }

    /// Non-DHD targets fall straight through so the generic interaction
    /// dispatch still runs.
    #[tokio::test]
    async fn a_non_dhd_target_is_not_claimed() {
        let mut mgr = mgr_with_dhd();
        mgr.stargates.insert(15, gate("Agnos", 15));
        if let Some(prop) = mgr.get_entity_mut(100) {
            prop.interaction_type_flags = 0;
        }

        let (tx, mut rx) = mpsc::channel(16);
        assert!(!try_open_dhd(1, 100, &tx, &mut mgr).await);
        assert_eq!(drain_dhd(&mut rx), None);
    }
}
