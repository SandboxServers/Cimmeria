//! BeingAppearance assembly, onEntityTint, and visual resend helpers.
//!
//! Extracted from `world_entry.rs` — these functions build the appearance
//! wire data and handle the post-transaction / post-cinematic resend logic.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx, write_wstring, SKIN_TINTS};

use super::helpers::send_to_witness;
use super::ConnectedClientState;

// ── Appearance data builders ────────────────────────────────────────────────

/// Build the BeingAppearance wire args: `[wstring bodyset][u32 count][wstring comp]*`.
///
/// Used by `handle_map_loaded` to cache for later resend, and by
/// `handle_on_client_ready` / `handle_cancel_movie` to resend.
pub(crate) fn build_appearance_args(bodyset: &str, components: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    write_wstring(&mut buf, bodyset);
    buf.extend_from_slice(&(components.len() as u32).to_le_bytes());
    for comp in components {
        write_wstring(&mut buf, comp);
    }
    buf
}

/// Build the onEntityTint wire args: `[u32 primary=0][u32 secondary=0][u32 skin_tint]`.
///
/// Maps `skin_color_id` (DB index) through the SKIN_TINTS table, matching
/// the C++ `requestCharacterVisuals` flow that sends the mapped tint value.
pub(crate) fn build_tint_args(skin_color_id: i32) -> Vec<u8> {
    let skin_tint = if (skin_color_id as usize) < SKIN_TINTS.len() {
        SKIN_TINTS[skin_color_id as usize]
    } else {
        SKIN_TINTS[0]
    };
    let mut buf = Vec::with_capacity(12);
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&skin_tint.to_le_bytes());
    buf
}

// ── Visual resend handlers ──────────────────────────────────────────────────

/// Finalize world entry after the client sends `SGWPlayer.onClientReady`.
///
/// Also resends BeingAppearance + onEntityTint. The first copy was sent in the
/// mapLoaded bundle but may have been dropped because the entity was still in a
/// "transaction" during bundle processing. The C++ server sends BeingAppearance
/// 3-5 times via createCacheStamp replays; this second send mimics that.
pub(crate) async fn handle_on_client_ready(
    addr: SocketAddr,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pending = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get_mut(&addr)
            .and_then(|c| c.pending_client_ready.take())
    };

    let Some(pending) = pending else {
        tracing::debug!(%addr, "SGWPlayer.onClientReady received with no pending world-entry finalization");
        return Ok(());
    };

    let entity_id = pending.entity_id;

    tracing::info!(
        %addr,
        entity_id,
        player_id = pending.player_id,
        world = %pending.world_name,
        "SGWPlayer.onClientReady received -- finalizing world entry"
    );

    // Query saved missions from DB before sending InitPlayerState
    let saved_missions =
        super::world_entry::methods::query_saved_missions(db_pool, pending.player_id).await;

    // Query player abilities from DB
    let abilities: Vec<i32> = if let Some(pool) = db_pool {
        sqlx::query_scalar("SELECT unnest(abilities) FROM sgw_player WHERE player_id = $1")
            .bind(pending.player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    // Query active bandolier slot and items from DB (Bug #1: don't hardcode empty state).
    // Distinguish DB error from "no row" so a connection blip doesn't silently default
    // a real player to empty bandolier state.
    let (active_bandolier_slot, bandolier_items) = if let Some(pool) = db_pool {
        let slot: i32 = match sqlx::query_scalar::<_, Option<i32>>(
            "SELECT bandolier_slot FROM sgw_player WHERE player_id = $1",
        )
        .bind(pending.player_id)
        .fetch_optional(pool.as_ref())
        .await
        {
            Ok(Some(Some(s))) => s,
            Ok(Some(None)) | Ok(None) => 0,
            Err(e) => {
                tracing::error!(
                    player_id = pending.player_id,
                    "Bandolier slot read failed; defaulting to 0 but logging error: {e}"
                );
                0
            }
        };

        let items = super::world_entry::methods::player_load::meta::query_bandolier_items(
            db_pool,
            pending.player_id,
        )
        .await;

        (slot, items)
    } else {
        (0, Vec::new())
    };

    if let Some(ref tx) = cell_tx {
        let _ = tx.send(BaseToCellMsg::ConnectEntity { entity_id }).await;

        let _ = tx
            .send(BaseToCellMsg::InitPlayerState {
                entity_id,
                player_id: pending.player_id,
                world_name: pending.world_name.clone(),
                saved_missions,
                abilities,
                active_bandolier_slot,
                bandolier_items,
            })
            .await;
    }

    // Resend BeingAppearance + onEntityTint now that the entity is fully ready.
    let appearance_args = pending.appearance_args;
    let tint_args = pending.tint_args;
    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::BEING_APPEARANCE,
                &appearance_args,
            )
        },
    )
    .await;
    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_ENTITY_TINT,
                &tint_args,
            )
        },
    )
    .await;

    tracing::info!(%addr, entity_id, "World entry finalized (BeingAppearance resent)");
    Ok(())
}

/// Resend BeingAppearance + onEntityTint after the first-login cinematic finishes.
///
/// The client sends `cancelMovie` (exposed cell method index 108) when the intro
/// cinematic ends. By this point both previous BeingAppearance sends (in the
/// mapLoaded bundle and after onClientReady) may have been lost because the
/// cinematic was rendering full-screen. This third send ensures the model loads.
pub(crate) async fn handle_cancel_movie(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let cached = {
        let clients = connected.lock().unwrap();
        clients
            .get(&addr)
            .and_then(|c| match (&c.cached_appearance_args, &c.cached_tint_args) {
                (Some(a), Some(t)) => Some((a.clone(), t.clone())),
                _ => None,
            })
    };

    let Some((appearance_args, tint_args)) = cached else {
        tracing::debug!(%addr, entity_id, "cancelMovie: no cached appearance data -- skipping resend");
        return;
    };

    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::BEING_APPEARANCE,
                &appearance_args,
            )
        },
    )
    .await;
    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_ENTITY_TINT,
                &tint_args,
            )
        },
    )
    .await;

    tracing::info!(%addr, entity_id, "cancelMovie: BeingAppearance + onEntityTint resent after cinematic");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mercury::SKIN_TINTS;

    /// `build_appearance_args` wire layout:
    /// `[wstring bodyset] [u32 LE component_count] [wstring component]*`
    /// where each wstring is `[u32 LE char_count] [UTF-16LE chars]`.
    /// Asserts the COMPLETE byte vector against a hand-computed
    /// expected slice — partial spot-checks would let a broken
    /// implementation that emits the right first byte but wrong
    /// subsequent ones still pass.
    #[test]
    fn build_appearance_args_emits_bodyset_count_components_layout() {
        let buf = build_appearance_args("Body", &["A".to_string(), "BB".to_string()]);
        let expected: &[u8] = &[
            // bodyset wstring: count=4, then 'B' 0 'o' 0 'd' 0 'y' 0
            4, 0, 0, 0, b'B', 0, b'o', 0, b'd', 0, b'y', 0, // component_count = 2
            2, 0, 0, 0, // component "A": count=1, 'A' 0
            1, 0, 0, 0, b'A', 0, // component "BB": count=2, 'B' 0 'B' 0
            2, 0, 0, 0, b'B', 0, b'B', 0,
        ];
        assert_eq!(buf, expected, "byte-exact wire layout");
    }

    /// Non-ASCII regression guard: write_wstring must emit
    /// UTF-16-LE code units (with a UTF-16 char_count), not a
    /// UTF-8 byte sequence (with a UTF-8 byte length). A drift to
    /// UTF-8 would produce identical bytes for ASCII but corrupt
    /// the wire payload for any character above 0x7F.
    ///
    /// "café" in UTF-16: c (0x0063), a (0x0061), f (0x0066),
    /// é (0x00E9) → char_count = 4, byte_count = 8.
    /// In UTF-8 the same string would be 5 bytes (é is two bytes).
    /// Pinning byte-exact UTF-16-LE catches the drift.
    #[test]
    fn build_appearance_args_emits_utf16_for_non_ascii_components() {
        let buf = build_appearance_args("Body", &["café".to_string()]);
        let expected: &[u8] = &[
            // bodyset wstring: "Body"
            4, 0, 0, 0, b'B', 0, b'o', 0, b'd', 0, b'y', 0, // component_count = 1
            1, 0, 0, 0, // component "café": char_count=4, then UTF-16-LE code units
            4, 0, 0, 0, // c (0x0063), a (0x0061), f (0x0066), é (0x00E9)
            0x63, 0x00, 0x61, 0x00, 0x66, 0x00, 0xE9, 0x00,
        ];
        assert_eq!(
            buf, expected,
            "non-ASCII string must serialize as UTF-16-LE, not UTF-8"
        );
    }

    #[test]
    fn build_appearance_args_with_no_components_emits_zero_count() {
        let buf = build_appearance_args("X", &[]);
        // bodyset "X": 4 + 2 = 6 bytes; then count=0 (4 bytes).
        assert_eq!(buf.len(), 6 + 4);
        assert_eq!(buf[0..4], [1, 0, 0, 0]);
        assert_eq!(buf[6..10], [0, 0, 0, 0]);
    }

    /// `build_tint_args` wire layout: `[u32 0][u32 0][u32 LE skin_tint]`.
    /// The first two slots are reserved for primary/secondary tints and
    /// must always be zero.
    #[test]
    fn build_tint_args_layout_with_valid_skin_color_id() {
        let buf = build_tint_args(3);
        assert_eq!(buf.len(), 12);
        assert_eq!(buf[0..4], [0, 0, 0, 0], "primary tint must be 0");
        assert_eq!(buf[4..8], [0, 0, 0, 0], "secondary tint must be 0");
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[3]);
    }

    /// Out-of-range skin_color_id falls back to SKIN_TINTS[0]. Pin so a
    /// future regression that panics on the index path can't crash the
    /// world-entry flow.
    #[test]
    fn build_tint_args_clamps_oob_skin_color_id_to_zero() {
        let buf = build_tint_args(999);
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[0]);
    }

    /// Negative skin_color_id casts to a huge usize and falls into the
    /// fallback branch. Pin so the cast doesn't accidentally succeed
    /// after a refactor (which would index into garbage).
    #[test]
    fn build_tint_args_negative_id_falls_back() {
        let buf = build_tint_args(-1);
        let tint = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
        assert_eq!(tint, SKIN_TINTS[0]);
    }
}
