//! BeingAppearance assembly, onEntityTint, and visual resend helpers.
//!
//! Extracted from `world_entry.rs` — these functions build the appearance
//! wire data and handle the post-transaction / post-cinematic resend logic.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx, write_wstring, SKIN_TINTS};

use super::helpers::send_to_witness_reliable;
use super::world_entry::handle_map_loaded;
use super::world_entry_chat::{
    build_chat_joined_args, build_welcome_message_args, DEFAULT_CHAT_CHANNELS,
};
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
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<sqlx::PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Cross-world transition fallback: the SGW client sends `mapLoaded`
    // (cell method 0x99) on initial logins and same-instance respawns,
    // but skips it on cross-world transitions via `gate_travel`
    // (RESET_ENTITIES → ENABLE_ENTITIES → CREATE_BASE_PLAYER + onClientMapLoad).
    // The client jumps straight to `onClientReady` instead. If the
    // transition is mid-flight (`pending_map_loaded` still set, meaning
    // the `mapLoaded` cell-method handler never ran), drive the
    // enter-world bundle from here so the destination world finishes
    // setup. After this synth call, `pending_client_ready` will be
    // populated by `handle_map_loaded` and the rest of this function
    // proceeds normally.
    let pending_map_loaded_present = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get(&addr)
            .is_some_and(|c| c.pending_map_loaded.is_some())
    };
    if pending_map_loaded_present {
        tracing::info!(
            %addr,
            "onClientReady arrived with pending_map_loaded still set — \
             synthesising mapLoaded for cross-world transition"
        );
        if let Err(e) = handle_map_loaded(
            transport,
            addr,
            key,
            connected,
            cell_tx,
            entity_to_addr,
            db_pool,
        )
        .await
        {
            tracing::error!(%addr, error = %e, "Synth mapLoaded failed during cross-world onClientReady");
            // The synth `handle_map_loaded` consumed `pending_map_loaded`
            // before failing, so a retry would no longer enter this
            // branch and `pending_client_ready` will never be populated
            // by the normal map_loaded path. If we just returned here,
            // `pending_destination_ring_id` would be stranded on the
            // client state forever (disconnect cleanup doesn't clear it
            // either), and the destination ring's FSM would sit in
            // `RemoteLoadWait` indefinitely.
            //
            // Drop the pending ring id so state is consistent. The
            // destination ring still won't receive an
            // `AdvanceRingDestination` (no entity_id is reachable on
            // the dropped session anyway), but at least the
            // per-client state isn't lying about an in-flight transfer
            // that's never going to land.
            if let Ok(mut clients) = connected.lock() {
                if let Some(c) = clients.get_mut(&addr) {
                    if c.pending_destination_ring_id.take().is_some() {
                        tracing::warn!(
                            %addr,
                            "Cleared stranded pending_destination_ring_id after \
                             synth mapLoaded failure"
                        );
                    }
                }
            }
            return Ok(());
        }
    }

    // Take the pending finalization AND copy out the player_name in the
    // same lock so the welcome-message send below doesn't need a second
    // round-trip. `player_name` is set during `playCharacter` and stays
    // for the session, so reading it here is safe.
    let (pending, player_name) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let entry = clients.get_mut(&addr);
        match entry {
            Some(c) => (c.pending_client_ready.take(), c.player_name.clone()),
            None => (None, None),
        }
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

    // Query archetype id from DB. Carried through to the cell so the
    // `Item_*` event-set lookup (reload-sequence path) can
    // resolve. Defaults to 0 on error / missing row — the cell
    // `archetype_item_event_set` falls through to "no animation" rather
    // than crashing, which matches the python behavior.
    let archetype_id: i32 = if let Some(pool) = db_pool {
        match sqlx::query_scalar::<_, Option<i32>>(
            "SELECT archetype FROM sgw_player WHERE player_id = $1",
        )
        .bind(pending.player_id)
        .fetch_optional(pool.as_ref())
        .await
        {
            Ok(Some(Some(a))) => a,
            Ok(Some(None)) | Ok(None) => 0,
            Err(e) => {
                tracing::error!(
                    player_id = pending.player_id,
                    "Archetype read failed; defaulting to 0 but logging error: {e}"
                );
                0
            }
        }
    } else {
        0
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

    // Cross-world ring transport carry-through: take the pending ring id
    // BEFORE we drop the connected lock so a concurrent disconnect can't
    // strand it. Set in `gate_travel::handle_gate_travel` only when
    // `Effect::TeleportCrossWorld` produced this hop — stargate dial
    // travel leaves it None.
    let advance_ring_destination_id: Option<i32> = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients
            .get_mut(&addr)
            .and_then(|c| c.pending_destination_ring_id.take())
    };

    if let Some(ref tx) = cell_tx {
        let _ = tx.send(BaseToCellMsg::ConnectEntity { entity_id }).await;

        let _ = tx
            .send(BaseToCellMsg::InitPlayerState {
                entity_id,
                player_id: pending.player_id,
                world_name: pending.world_name.clone(),
                archetype_id,
                saved_missions,
                abilities,
                active_bandolier_slot,
                bandolier_items,
            })
            .await;

        if let Some(region_id) = advance_ring_destination_id {
            // Wake the destination ring's FSM. Sent AFTER InitPlayerState
            // so the cell-side handler runs against the fully-initialised
            // entity (player_id, missions, etc.) — `mark_player_loaded`
            // doesn't need that state directly, but downstream chain
            // events fired by the unlock cascade do.
            let _ = tx
                .send(BaseToCellMsg::AdvanceRingDestination {
                    entity_id,
                    region_id,
                })
                .await;
        }
    }

    // Resend BeingAppearance + onEntityTint now that the entity is fully ready.
    let appearance_args = pending.appearance_args;
    let tint_args = pending.tint_args;
    send_to_witness_reliable(
        transport,
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
    send_to_witness_reliable(
        transport,
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

    // Chat-channel joins + welcome message. Original C++ path is
    // `python/base/SGWPlayer.py onClientReady -> ChannelManager.playerLoggedIn`,
    // so onClientReady (here) is the canonical fire-point. These used to
    // live in the mapLoaded bundle and padded ~311 B onto the worst-case
    // fragment burst before being moved out. Routed via
    // `send_to_witness_reliable`: the player is a witness of their own
    // entity, so this targets the connected client and keeps the bytes
    // on the reliable channel.
    //
    // Arg-building is split into pure helpers (`build_chat_joined_args` /
    // `build_welcome_message_args`) so they're independently unit-tested;
    // this loop is the wire side that the test suite can't reach without
    // a full async harness.
    for &(channel_name, channel_id) in DEFAULT_CHAT_CHANNELS {
        let args = build_chat_joined_args(channel_name, channel_id);
        send_to_witness_reliable(
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
                    method_idx::ON_CHAT_JOINED,
                    &args,
                )
            },
        )
        .await;
    }

    // Welcome message — `onPlayerCommunication(speaker, flags, channel, text)`
    // on CHAN_TELL (9). Matches python `SGWPlayer.py:541`. Cosmetic-only;
    // dropping it has no correctness impact, but it's the moment-of-arrival
    // visible signal that the channel registration above actually landed.
    //
    // `player_name` is normally set during `playCharacter` and stays for
    // the session; falling back to a generic "Server" speaker keeps the
    // welcome line visible even when the name didn't propagate (race
    // during a synthesised cross-world `onClientReady`, or a future
    // refactor that defers the name read). The warn log surfaces the
    // unexpected case without dropping the message on the floor.
    let speaker = player_name.as_deref().unwrap_or_else(|| {
        tracing::warn!(
            %addr,
            entity_id,
            "onClientReady: player_name not set on connected state — \
             sending welcome with generic Server speaker"
        );
        "Server"
    });
    {
        let args = build_welcome_message_args(speaker, entity_id);
        send_to_witness_reliable(
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
                    method_idx::ON_PLAYER_COMMUNICATION,
                    &args,
                )
            },
        )
        .await;
    }

    // First-login cinematic — fires AFTER appearance is bound to the now-live
    // possessed pawn. Sending it inside the mapLoaded bundle (before this
    // gate) lets the cinematic-exit CollectGarbage reclaim the in-flight
    // appearance asset and produces a "dev cube" flash. Issue #288.
    if pending.first_login != 0 {
        send_cinematic(
            transport,
            addr,
            entity_id,
            "Cine-SGWLogo.SGWLogo",
            true, // fullscreen
            connected,
            entity_to_addr,
        )
        .await;

        if let Some(pool) = db_pool {
            if let Err(e) =
                sqlx::query("UPDATE sgw_player SET first_login = 0 WHERE player_id = $1")
                    .bind(pending.player_id)
                    .execute(pool.as_ref())
                    .await
            {
                tracing::warn!(
                    player_id = pending.player_id,
                    error = %e,
                    "Failed to clear first_login flag after cinematic dispatch; player will see intro again next login",
                );
            }
        }

        tracing::info!(%addr, entity_id, "First-login cinematic dispatched after onClientReady gate");
    }

    // Flush any AoI messages the cell tried to dispatch while the client
    // was still loading terrain. The client is now ready to receive
    // entity-state traffic; the Channel's deferred-send queue absorbs
    // any overflow past the 32-slot TX window.
    super::world_entry::cell_dispatch::flush_deferred_aoi(
        entity_id,
        addr,
        socket,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::info!(%addr, entity_id, "World entry finalized (BeingAppearance resent)");
    Ok(())
}

/// Send an `onPlayMovie` cinematic and arm the post-cinematic appearance guard.
///
/// The cinematic-exit `CollectGarbage` reclaims the player's appearance asset
/// regardless of how the cinematic exits (natural end or Esc / Lua
/// `cancelMovie`), leaving the player's pawn rendering as a dev-cube
/// placeholder until the appearance is rebound. The race window varies — on
/// natural end it's ~one frame to several seconds; on Esc it can be a single
/// frame depending on network latency.
///
/// To eliminate the cube regardless of cinematic-exit mode:
///
/// 1. Send the `onPlayMovie` packet.
/// 2. Reset `cinematic_spam_cancel` to `false` on the connected state and
///    spawn a tokio task that resends BeingAppearance + onEntityTint every
///    `RESEND_INTERVAL` for up to `RESEND_DURATION`.
/// 3. The spam loop polls `cinematic_spam_cancel` each iteration. When the
///    client emits a real `cancelMovie` (Esc / Lua), `handle_cancel_movie`
///    flips the flag and the loop exits early — saving the remaining
///    bandwidth for the user's session.
///
/// **Callers** must ensure `cached_appearance_args` + `cached_tint_args` are
/// populated on the connected state before invoking (the spam reads from
/// those). That's done during `handle_map_loaded`.
///
/// **Future cinematics** (mission cutscenes, gate transitions, dialog
/// overlays, etc.) should go through this function rather than emitting
/// `onPlayMovie` directly — the GC race is a property of every cinematic,
/// not just the first-login intro. Issue #288.
pub(crate) async fn send_cinematic(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    entity_id: u32,
    cinematic_asset: &str,
    fullscreen: bool,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // 1. Send the cinematic packet. Reliable — `onPlayMovie` is one-shot
    // state-change; loss leaves the cinematic un-triggered with no
    // self-correcting follow-up.
    let mut movie_args = Vec::with_capacity(32);
    write_wstring(&mut movie_args, cinematic_asset);
    movie_args.push(if fullscreen { 1u8 } else { 0u8 });
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_PLAY_MOVIE,
                &movie_args,
            )
        },
    )
    .await;

    // 2. Arm the appearance-spam guard. Tunable via the two constants below.
    //    100 ms × 200 iters = 20 s; cinematic-exit cancellation short-circuits.
    //
    // 20 s comfortably covers `Cine-SGWLogo` (314 frames @ 23.976 fps =
    // 13.10 s natural end) plus a ~7 s post-cinematic safety buffer for GC
    // and asset-rebind latency. Every BIK cinematic's duration is knowable:
    // parse the embedded BIK header out of the corresponding
    // `game/sgw/.../CookedPC/Packages/Cine-*.upk` — the `BIKi` magic is
    // followed by `file_size`, `num_frames`, then `fps_dividend /
    // fps_divisor` at offset +24 / +28. If a future caller needs a tighter
    // window per cinematic, promote `RESEND_DURATION` to a `send_cinematic`
    // parameter and look it up from a build-time-generated table.
    const RESEND_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);
    const RESEND_DURATION: std::time::Duration = std::time::Duration::from_secs(20);
    let resend_count = (RESEND_DURATION.as_millis() / RESEND_INTERVAL.as_millis()).max(1) as usize;

    // Reset the cancel flag (could be left `true` by a previous cinematic in
    // the same session — e.g. mission cutscene after first-login intro) and
    // clone the Arc so the spawned task can poll it without re-locking.
    let cancel_flag = {
        let clients = connected.lock().unwrap();
        let Some(c) = clients.get(&addr) else {
            tracing::warn!(
                %addr,
                entity_id,
                "send_cinematic: client state vanished before spam armed -- skipping guard"
            );
            return;
        };
        c.cinematic_spam_cancel.store(false, Ordering::Relaxed);
        Arc::clone(&c.cinematic_spam_cancel)
    };

    let resend_socket = Arc::clone(transport);
    let resend_connected = Arc::clone(connected);
    let resend_entity_to_addr = Arc::clone(entity_to_addr);
    let cinematic_label = cinematic_asset.to_string();
    tokio::spawn(async move {
        tracing::info!(
            entity_id,
            cinematic = %cinematic_label,
            resend_count,
            interval_ms = RESEND_INTERVAL.as_millis() as u64,
            "Cinematic-guard appearance spam: starting"
        );
        for i in 0..resend_count {
            tokio::time::sleep(RESEND_INTERVAL).await;
            if cancel_flag.load(Ordering::Relaxed) {
                tracing::info!(
                    entity_id,
                    cinematic = %cinematic_label,
                    sent = i,
                    skipped = resend_count - i,
                    "Cinematic-guard appearance spam: cancelMovie received -- stopping early"
                );
                return;
            }
            resend_appearance_after_cinematic(
                &resend_socket,
                addr,
                entity_id,
                &resend_connected,
                &resend_entity_to_addr,
            )
            .await;
        }
        tracing::info!(
            entity_id,
            cinematic = %cinematic_label,
            sent = resend_count,
            "Cinematic-guard appearance spam: complete (full duration elapsed)"
        );
    });
}

/// Resend BeingAppearance + onEntityTint from `addr`'s cached args.
///
/// Internal helper used by both:
/// - `handle_cancel_movie` (real client cancelMovie — single resend), and
/// - `send_cinematic`'s spam loop (each iteration).
///
/// Pure side-effect (sends two packets); does NOT touch `cinematic_spam_cancel`.
async fn resend_appearance_after_cinematic(
    transport: &Arc<dyn Transport>,
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
        tracing::debug!(%addr, entity_id, "resend_appearance_after_cinematic: no cached appearance data -- skipping");
        return;
    };

    send_to_witness_reliable(
        transport,
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
    send_to_witness_reliable(
        transport,
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
}

/// Handle the client's `cancelMovie` (exposed cell method index 108): the
/// cinematic was dismissed (Esc or Lua-stop). Resends BeingAppearance +
/// onEntityTint to recover from the cinematic-exit GC, and flips
/// `cinematic_spam_cancel` so `send_cinematic`'s spam loop stops early.
pub(crate) async fn handle_cancel_movie(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Stop the in-flight cinematic-guard spam (if any). The client just told
    // us it dismissed the cinematic, so we no longer need to brute-force a
    // post-GC appearance refresh — one resend below handles it.
    {
        let clients = connected.lock().unwrap();
        if let Some(c) = clients.get(&addr) {
            c.cinematic_spam_cancel.store(true, Ordering::Relaxed);
        }
    }

    resend_appearance_after_cinematic(transport, addr, entity_id, connected, entity_to_addr).await;

    tracing::info!(%addr, entity_id, "cancelMovie: BeingAppearance + onEntityTint resent; spam guard signalled to stop");
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

    /// Non-BMP regression guard: write_wstring must use the
    /// UTF-16 code-unit count, NOT `chars().count()`, for the
    /// length prefix. Pick "🌟" (U+1F31F) — a single Unicode
    /// scalar that requires a UTF-16 surrogate PAIR (D83C DF1F),
    /// so:
    ///   - `chars().count()`           = 1
    ///   - `encode_utf16().count()`    = 2
    ///   - UTF-8 byte length           = 4
    ///
    /// All three values are distinct, so the byte-exact assertion
    /// catches both the "drifted to UTF-8" regression and the
    /// "used chars().count() instead of encode_utf16().count()"
    /// regression. (A simpler character like é wouldn't distinguish
    /// the second case because its chars and UTF-16 counts agree.)
    #[test]
    fn build_appearance_args_emits_utf16_for_non_bmp_components() {
        let buf = build_appearance_args("Body", &["🌟".to_string()]);
        let expected: &[u8] = &[
            // bodyset wstring: "Body"
            4, 0, 0, 0, b'B', 0, b'o', 0, b'd', 0, b'y', 0, // component_count = 1
            1, 0, 0, 0,
            // component "🌟": char_count = 2 (UTF-16 code units),
            // then surrogate pair D83C DF1F as little-endian u16s.
            2, 0, 0, 0, 0x3C, 0xD8, 0x1F, 0xDF,
        ];
        assert_eq!(
            buf, expected,
            "non-BMP string must serialize as UTF-16-LE surrogate pair with code-unit count"
        );
    }

    /// Empty-components case must emit the EXACT bodyset wstring
    /// followed by a u32 zero count, with no trailing bytes. Pin
    /// the full byte sequence (not just a length + count slice)
    /// so a regression that drifts the bodyset payload bytes —
    /// e.g. flipping endianness or emitting UTF-8 in the bodyset
    /// while leaving the count zero — still fails this test.
    #[test]
    fn build_appearance_args_with_no_components_emits_zero_count() {
        let buf = build_appearance_args("X", &[]);
        let expected: &[u8] = &[
            // bodyset "X": char_count = 1, then 'X' 0x00
            1, 0, 0, 0, b'X', 0, // component_count = 0
            0, 0, 0, 0,
        ];
        assert_eq!(buf, expected);
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

// Chat-channel registration helpers (`DEFAULT_CHAT_CHANNELS`, `CHAN_TELL`,
// `build_chat_joined_args`, `build_welcome_message_args`) and their
// byte-exact tests live in `super::world_entry_chat`. They're imported
// at the top of this file and used inside `handle_on_client_ready` for
// the post-`onClientReady` `ChannelManager.playerLoggedIn` flow.
