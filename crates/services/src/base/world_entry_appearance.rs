//! BeingAppearance assembly, onEntityTint, and visual resend helpers.
//!
//! Extracted from `world_entry.rs` — these functions build the appearance
//! wire data and handle the post-transaction / post-cinematic resend logic.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx, write_wstring, SKIN_TINTS};

use super::helpers::send_to_witness_reliable;
use super::world_entry::handle_map_loaded;
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

/// Default chat channels registered on every world-entry `onClientReady`.
///
/// Each entry is `(channel_name, channel_id)`. The id values come from
/// `python/common/Constants.py` and the channel set + ordering matches
/// `python/base/SGWPlayer.py onClientReady -> ChannelManager.playerLoggedIn`.
/// Pulled out as a module constant so it's covered by a byte-exact test
/// in [`tests`] and so a future configuration knob (per-archetype
/// channel sets, for example) has a single source to read from.
pub(crate) const DEFAULT_CHAT_CHANNELS: &[(&str, u8)] = &[
    ("say", 0),
    ("emote", 1),
    ("yell", 2),
    ("team", 3),
    ("squad", 4),
    ("command", 5),
    ("server", 7),
    ("tell", 9),
];

/// `onPlayerCommunication`'s "tell" channel id (matches `CHAN_TELL` in
/// `python/common/Constants.py` and `SGWPlayer.py:541`).
pub(crate) const CHAN_TELL: u8 = 9;

/// Build the `onChatJoined(WSTRING channelName, UINT8 channelID)` wire args.
///
/// Wire format: `[u32 char_count][UTF-16LE chars][u8 channel_id]`.
pub(crate) fn build_chat_joined_args(channel_name: &str, channel_id: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8 + channel_name.len() * 2);
    write_wstring(&mut buf, channel_name);
    buf.push(channel_id);
    buf
}

/// Build the `onPlayerCommunication(speaker, speakerFlags, channel, text)`
/// wire args for the post-`onClientReady` welcome message.
///
/// Wire format: `[wstring speaker][u8 speaker_flags=0][u8 channel][wstring text]`.
/// The text is `"Welcome to Stargate Worlds. Your player id is: {entity_id}."`
/// pinned by [`tests`]; matches the python reference at `SGWPlayer.py:541`.
pub(crate) fn build_welcome_message_args(speaker: &str, entity_id: u32) -> Vec<u8> {
    let welcome = format!("Welcome to Stargate Worlds. Your player id is: {entity_id}.");
    let mut buf = Vec::with_capacity(16 + (speaker.len() + welcome.len()) * 2);
    write_wstring(&mut buf, speaker);
    buf.push(0u8); // SpeakerFlags
    buf.push(CHAN_TELL);
    write_wstring(&mut buf, &welcome);
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
    socket: &Arc<UdpSocket>,
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
            socket,
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
            socket,
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
    socket: &Arc<UdpSocket>,
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

    let resend_socket = Arc::clone(socket);
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
        tracing::debug!(%addr, entity_id, "resend_appearance_after_cinematic: no cached appearance data -- skipping");
        return;
    };

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
                method_idx::BEING_APPEARANCE,
                &appearance_args,
            )
        },
    )
    .await;
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
    socket: &Arc<UdpSocket>,
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

    resend_appearance_after_cinematic(socket, addr, entity_id, connected, entity_to_addr).await;

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

    /// `DEFAULT_CHAT_CHANNELS` must match the original python
    /// `SGWPlayer.py onClientReady -> ChannelManager.playerLoggedIn`
    /// channel set exactly — same names, same ids, same order. A
    /// regression that adds, drops, or reorders entries silently
    /// changes the wire-burst shape on every world entry; pinning
    /// the full slice catches it.
    #[test]
    fn default_chat_channels_matches_sgwplayer_channel_set() {
        let expected: &[(&str, u8)] = &[
            ("say", 0),
            ("emote", 1),
            ("yell", 2),
            ("team", 3),
            ("squad", 4),
            ("command", 5),
            ("server", 7),
            ("tell", 9),
        ];
        assert_eq!(
            DEFAULT_CHAT_CHANNELS, expected,
            "DEFAULT_CHAT_CHANNELS drifted from the SGWPlayer.py reference set",
        );
        // CHAN_TELL constant must agree with the "tell" entry in the slice;
        // the welcome message in `handle_on_client_ready` reads from
        // CHAN_TELL while the channel-join loop reads from the slice.
        let tell_entry = DEFAULT_CHAT_CHANNELS
            .iter()
            .find(|(name, _)| *name == "tell")
            .expect("'tell' channel must be in DEFAULT_CHAT_CHANNELS");
        assert_eq!(
            tell_entry.1, CHAN_TELL,
            "CHAN_TELL ({}) must equal the 'tell' entry's id ({}) in DEFAULT_CHAT_CHANNELS",
            CHAN_TELL, tell_entry.1
        );
    }

    /// `build_chat_joined_args` wire layout: `[wstring channel_name][u8
    /// channel_id]`. Byte-exact comparison so a regression that drops the
    /// length prefix, swaps to UTF-8, or appends the id before the
    /// wstring (matching the reverse-order python alias) fails here.
    #[test]
    fn build_chat_joined_args_emits_wstring_then_u8_id() {
        let buf = build_chat_joined_args("say", 0);
        let expected: &[u8] = &[
            3, 0, 0, 0, b's', 0, b'a', 0, b'y', 0, // wstring "say"
            0, // channel_id
        ];
        assert_eq!(buf, expected, "byte-exact wire layout for 'say'/0");

        let buf = build_chat_joined_args("tell", 9);
        let expected: &[u8] = &[
            4, 0, 0, 0, b't', 0, b'e', 0, b'l', 0, b'l', 0, // wstring "tell"
            9, // channel_id
        ];
        assert_eq!(buf, expected, "byte-exact wire layout for 'tell'/9");
    }

    /// All 8 channels in `DEFAULT_CHAT_CHANNELS` must produce well-formed
    /// `onChatJoined` args. Sweeping the actual slice catches a future
    /// channel addition that forgets to update `build_chat_joined_args`
    /// (e.g. introducing a wider id type).
    #[test]
    fn build_chat_joined_args_round_trips_for_all_default_channels() {
        for &(name, id) in DEFAULT_CHAT_CHANNELS {
            let buf = build_chat_joined_args(name, id);
            // Length prefix must equal name.chars().encode_utf16().count()
            // (matches `write_wstring` semantics, not `name.len()`).
            let utf16_units: u32 = name.encode_utf16().count() as u32;
            assert_eq!(
                &buf[0..4],
                &utf16_units.to_le_bytes(),
                "channel {name}: wstring length prefix must be UTF-16 code-unit count"
            );
            let payload_end = 4 + (utf16_units as usize) * 2;
            assert_eq!(
                buf.len(),
                payload_end + 1,
                "channel {name}: total length = 4 (count) + 2*units + 1 (id)",
            );
            assert_eq!(
                buf[payload_end], id,
                "channel {name}: id byte must follow the wstring",
            );
        }
    }

    /// `build_welcome_message_args` wire layout:
    /// `[wstring speaker][u8 0=flags][u8 9=CHAN_TELL][wstring text]`.
    /// Pin the full byte sequence so a regression that drops the
    /// flags byte, picks a different channel, or rewrites the welcome
    /// text fails here.
    #[test]
    fn build_welcome_message_args_emits_speaker_flags_chan_tell_then_text() {
        let buf = build_welcome_message_args("S", 7);
        // Expected text: "Welcome to Stargate Worlds. Your player id is: 7."
        // (50 chars, all BMP, so 50 UTF-16 code units, 100 bytes).
        let mut expected: Vec<u8> = Vec::new();
        // speaker wstring "S"
        expected.extend_from_slice(&1u32.to_le_bytes());
        expected.extend_from_slice(&[b'S', 0]);
        expected.push(0u8); // SpeakerFlags
        expected.push(9u8); // CHAN_TELL
                            // text wstring
        let text = "Welcome to Stargate Worlds. Your player id is: 7.";
        let utf16: Vec<u16> = text.encode_utf16().collect();
        expected.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
        for unit in utf16 {
            expected.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(buf, expected, "byte-exact wire layout for welcome");
    }

    /// Entity id substitution must actually thread into the text. A
    /// regression that hard-codes a placeholder (or formats the wrong
    /// variable) would leave the welcome message lying to the player —
    /// catch it by varying entity_id and asserting the text reflects it.
    #[test]
    fn build_welcome_message_args_includes_entity_id_in_text() {
        let buf = build_welcome_message_args("Tester", 12345);
        // Skip the speaker wstring + flags + channel bytes to reach text.
        let speaker_units = "Tester".encode_utf16().count();
        let text_offset = 4 + speaker_units * 2 + 1 + 1;
        let text_count = u32::from_le_bytes(
            buf[text_offset..text_offset + 4]
                .try_into()
                .expect("4 bytes"),
        ) as usize;
        let text_bytes = &buf[text_offset + 4..text_offset + 4 + text_count * 2];
        let text: String = char::decode_utf16(
            text_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|r| r.unwrap_or('?'))
        .collect();
        assert_eq!(
            text, "Welcome to Stargate Worlds. Your player id is: 12345.",
            "entity_id must be substituted into the welcome message"
        );
    }

    /// Fallback speaker for the player_name=None branch in
    /// `handle_on_client_ready` must produce a valid welcome message,
    /// not panic or omit fields. The fallback string ("Server") is a
    /// real channel name in `DEFAULT_CHAT_CHANNELS`, so a future
    /// refactor that swaps the fallback for an empty string is
    /// distinguishable from the legitimate case.
    #[test]
    fn build_welcome_message_args_handles_fallback_speaker() {
        let buf = build_welcome_message_args("Server", 42);
        // Just verify the speaker wstring is well-formed and reachable.
        let speaker_count = u32::from_le_bytes(buf[0..4].try_into().expect("4 bytes")) as usize;
        assert_eq!(speaker_count, 6, "'Server' has 6 UTF-16 code units");
        let speaker_bytes = &buf[4..4 + speaker_count * 2];
        let speaker: String = char::decode_utf16(
            speaker_bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]])),
        )
        .map(|r| r.unwrap_or('?'))
        .collect();
        assert_eq!(speaker, "Server");
        // SpeakerFlags + Channel bytes are at the fixed offset after the
        // speaker wstring.
        let after_speaker = 4 + speaker_count * 2;
        assert_eq!(buf[after_speaker], 0, "SpeakerFlags must be 0");
        assert_eq!(
            buf[after_speaker + 1],
            CHAN_TELL,
            "Channel must be CHAN_TELL"
        );
    }
}
