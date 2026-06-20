//! Cinematic dispatch + post-cinematic appearance-recovery helpers.
//!
//! Extracted from `world_entry_appearance.rs`. These handle the `onPlayMovie`
//! cinematic send, the appearance-spam guard that heals the cinematic-exit
//! `CollectGarbage` "dev cube" race (issue #288), and the real-client
//! `cancelMovie` recovery path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::mercury::{build_player_entity_method_packet, method_idx, write_wstring};

use super::super::helpers::{send_bundle_to_witness_reliable, send_to_witness_reliable};
use super::super::ConnectedClientState;
use super::builders::build_appearance_resend_bundle;

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
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_PLAY_MOVIE,
                &movie_args,
                version,
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

    // Bundle the BeingAppearance + onEntityTint pair into one fragment.
    // Built via the shared helper so the burst-shape regression guard
    // [`super::builders::tests::appearance_resend_bundle_collapses_to_single_packet`]
    // pins the same composition the production path emits.
    let bundle = build_appearance_resend_bundle(entity_id, &appearance_args, &tint_args);
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;
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
