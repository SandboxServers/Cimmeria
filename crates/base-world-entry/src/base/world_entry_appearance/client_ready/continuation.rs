//! World-entry continuation after `SGWPlayer.onClientReady`: the burst bundle,
//! the post-onClientReady callees, the first-login cinematic and the deferred
//! AoI flush.
//!
//! The continuation runs after the post-handoff session check in `mod.rs`.
//! Each stage re-checks the session before its side effect, so a session
//! replaced while an earlier await was in flight ends the old continuation
//! instead of sending its packets under the new session's key.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_base_session::base::cinematic_aoi_hold::CinematicAoiHold;
use cimmeria_base_session::base::plugin::{
    BaseCtx, BasePlugins, WorldEntryCall, WorldEntryHookPoint,
};
use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::base::{ConnectedClientState, PendingClientReadyInfo};
use crate::cell::messages::BaseToCellMsg;

use super::super::super::helpers::send_bundle_to_witness_reliable;
use super::super::super::world_entry_chat::build_welcome_message_args;
use super::super::builders::build_on_client_ready_burst_bundle;
use super::super::cinematic::send_cinematic;
use super::super::cinematic_aoi_hold;
use super::cell_handoff::SessionSnapshot;

/// One stage boundary: the test pause and mark hooks, and the session check
/// that ends the continuation before the stage's side effect. `$current` is
/// the `session.is_current(..)` closure; `$name` is the stage name.
macro_rules! stage {
    ($addr:expr, $current:expr, $name:literal) => {
        #[cfg(test)]
        super::session_token_tests::pause_after($addr, $name).await;
        if !$current() {
            return Ok(());
        }
        #[cfg(test)]
        super::continuation_tests::mark($addr, $name);
        #[cfg(test)]
        super::session_token_tests::pause_after($addr, concat!($name, "_late")).await;
    };
}

/// Everything the continuation reads from the session it is finalizing.
pub(super) struct ContinuationCtx<'a> {
    pub(super) addr: SocketAddr,
    pub(super) entity_id: u32,
    pub(super) pending: PendingClientReadyInfo,
    pub(super) player_name: Option<String>,
    pub(super) account_id: u32,
    pub(super) aoi_hold: Option<CinematicAoiHold>,
    pub(super) plugins: &'a BasePlugins,
    pub(super) transport: &'a Arc<dyn Transport>,
    pub(super) connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub(super) entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub(super) db_pool: &'a Option<Arc<sqlx::PgPool>>,
    pub(super) cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub(super) session: &'a SessionSnapshot,
}

/// Finish world entry for a session that passed the post-handoff check.
pub(super) async fn run_continuation(
    ctx: ContinuationCtx<'_>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ContinuationCtx {
        addr,
        entity_id,
        pending,
        player_name,
        account_id,
        aoi_hold,
        plugins,
        transport,
        connected,
        entity_to_addr,
        db_pool,
        cell_tx,
        session,
    } = ctx;
    let player_id = pending.player_id;
    let current = || session.is_current(connected, addr, entity_id, player_id);
    let name = player_name.as_deref();

    // Bundle the post-onClientReady burst: BeingAppearance resend +
    // onEntityTint resend + onPlayerCommunication welcome. No onChatJoined:
    // the client hardcodes every built-in channel (SS-C4, D-ORG14; see the
    // `world_entry_chat` module doc).
    //
    // **Transaction-state audit** (see
    // [docs/architecture/mercury-bundle.md](../../../docs/architecture/mercury-bundle.md)):
    // every message below targets the player's own `entity_id`, which was
    // created in `handle_map_loaded`'s prior bundle. The CREATE_BASE_PLAYER
    // transaction released at that bundle's end-of-frame, so same-entity
    // messages in THIS bundle bind to the now-live entity and are NOT
    // HOLD-FOR-TRANSACTION dropped. No CREATE_ENTITY / CELL_PLAYER fires
    // inside this handler, so the bundle is exclusively post-transaction
    // property/method updates — the canonical "safe to combine" case.
    //
    // `speaker` resolution mirrors the pre-bundle path: prefer the session's
    // `player_name`, fall back to "Server" with a WARN so the unexpected
    // missing-name case stays visible.
    let appearance_args = pending.appearance_args;
    let tint_args = pending.tint_args;
    let speaker = player_name.as_deref().unwrap_or_else(|| {
        tracing::warn!(
            %addr,
            entity_id,
            entity_name = name,
            "onClientReady: player_name not set on connected state — \
             sending welcome with generic Server speaker"
        );
        "Server"
    });
    let welcome_args = build_welcome_message_args(speaker, entity_id);

    stage!(addr, current, "burst");
    let bundle =
        build_on_client_ready_burst_bundle(entity_id, &appearance_args, &tint_args, &welcome_args);
    send_bundle_to_witness_reliable(transport, connected, entity_to_addr, entity_id, bundle).await;

    // A non-GM refused a GM-only world (D-DA4) arrives here at their
    // faction's start: tell them why, now that the world has loaded.
    stage!(addr, current, "gm_redirect");
    if let Some(line) = cimmeria_base_session::base::world_entry::gm_only_worlds::take_redirect_line(
        pending.player_id,
    ) {
        cimmeria_base_session::base::gm_feedback::send_gm_feedback_to_client(
            entity_id,
            line,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    // Push contact lists (Friends / Ignore + any custom lists) to the client.
    // Runs after the burst bundle so the entity is fully live on the client
    // before we start sending list-method packets. Also creates the system
    // lists on first login (idempotent via ensure_system_lists).
    stage!(addr, current, "contact_lists");
    super::super::super::contact_list::handlers::push_contact_lists_on_login(
        entity_id,
        pending.player_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    // Teams and Commands (ORG-06): each organization's header, ranks and
    // roster, then its online members marked Online; and the other online
    // members told this character is online. Every world entry, so gate
    // travel refreshes the entity id their rosters hold.
    stage!(addr, current, "org_restore");
    cimmeria_base_session::base::organization::handlers::restore_on_login(
        &cimmeria_base_session::base::organization::handlers::OrgCtx {
            db_pool,
            transport,
            connected,
            entity_to_addr,
            cell_tx,
        },
        &cimmeria_base_session::base::organization::handlers::OrgPlayer {
            account_id: Some(account_id),
            player_id: pending.player_id,
            entity_id,
        },
    )
    .await;

    // The base plugins' world-entry work (#962 step 5). Crafting sends its
    // state here: disciplines, paradigm levels, blueprints, the ASP total and
    // the crafting options (140: the window's machine and tool), owner-only,
    // in one bundle. After the burst for the same reason as the contact
    // lists: the entity is live and the crafting UI has loaded. Runs on every
    // onClientReady, so a world change resends it too.
    stage!(addr, current, "plugin_hook");
    plugins
        .run_world_entry_hook(
            WorldEntryHookPoint::ClientReadyAfterOrgRestore,
            WorldEntryCall {
                addr,
                entity_id,
                player_id: pending.player_id,
                ctx: BaseCtx {
                    db_pool,
                    cell_tx,
                    transport,
                    connected,
                    entity_to_addr,
                },
            },
        )
        .await;

    // Cache this character's Ignore list on the session (tells) and push it
    // to the cell entity (spatial chat), D-SS15. Here, after InitPlayerState,
    // so every world entry re-seeds it: gate travel creates a fresh cell
    // entity whose set would otherwise be empty.
    stage!(addr, current, "ignore_resync");
    super::super::super::contact_list::ignore::resync_ignore_cache(
        super::super::super::contact_list::ignore::IgnoreSyncCtx {
            db_pool,
            connected,
            cell_tx,
        },
        addr,
        pending.player_id,
        entity_id,
        "world_entry",
    )
    .await;

    // Gate-mail expiry for this player's own mailbox (SS-M4, D-SS04), so
    // mail that expired while they were away has taken its path before
    // they can open the mailbox.
    stage!(addr, current, "mail_sweep");
    if let Some(pool) = db_pool {
        super::super::super::world_entry::methods::mail::expiry::spawn_login_sweep(
            Arc::clone(pool),
            Arc::clone(transport),
            Arc::clone(connected),
            pending.player_id,
        );
    }

    // Fan out online status (CM 89, eventId=LoggedInStatus, data=1) to all
    // online players who have this character in any of their contact lists.
    // Runs after the list-push so the player is fully set up before watchers
    // are notified. Fire-and-forget via tokio::spawn — the login burst is
    // already complete and we must not block it on per-watcher DB + sends.
    stage!(addr, current, "login_fanout");
    if let Some(ref name) = player_name {
        let name_owned = name.clone();
        let player_id_owned = pending.player_id;
        let pool_c = db_pool.clone();
        let transport_c = transport.clone();
        let connected_c = connected.clone();
        let entity_to_addr_c = entity_to_addr.clone();
        tokio::spawn(async move {
            super::super::super::contact_list::handlers::fanout_login_status(
                &name_owned,
                player_id_owned,
                true, // online
                &pool_c,
                &transport_c,
                &connected_c,
                &entity_to_addr_c,
            )
            .await;
        });
    }

    // First-login cinematic — fires AFTER appearance is bound to the now-live
    // possessed pawn. Sending it inside the mapLoaded bundle (before this
    // gate) lets the cinematic-exit CollectGarbage reclaim the in-flight
    // appearance asset and produces a "dev cube" flash. Issue #288.
    if pending.first_login != 0 {
        stage!(addr, current, "cinematic");
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
            match sqlx::query("UPDATE sgw_player SET first_login = 0 WHERE player_id = $1")
                .bind(pending.player_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(r) if r.rows_affected() == 0 => {
                    // silent rows_affected==0 was the
                    // original-Python ghost bug — UPDATE succeeds but
                    // touches nothing, flag stays set, cinematic
                    // re-fires every login. error! so a single ops query
                    // (rows_affected != expected) surfaces it.
                    tracing::error!(
                        player_id = pending.player_id,
                        player_name = name,
                        rows_affected = 0,
                        expected = 1,
                        "first_login flag NOT cleared — cinematic will re-fire on next login (no matching player row?)"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        player_id = pending.player_id,
                        player_name = name,
                        error = %e,
                        "Failed to clear first_login flag after cinematic dispatch; player will see intro again next login",
                    );
                }
            }
        }

        tracing::info!(
            %addr,
            entity_id,
            entity_name = name,
            "First-login cinematic dispatched after onClientReady gate"
        );
    }

    // Flush any AoI messages the cell tried to dispatch while the client
    // was still loading terrain. The client is now ready to receive
    // entity-state traffic; the Channel's deferred-send queue absorbs
    // any overflow past the 32-slot TX window.
    //
    // On first login only the player-self method calls go now. Entity
    // introductions stay buffered until the movie is cancelled or runs out.
    // A hold with no cinematic (lab chaos `Delay`) is released here, and the
    // release keeps it up for the delay from a spawned timer.
    stage!(addr, current, "aoi_flush");
    if let Some(hold) = aoi_hold {
        cinematic_aoi_hold::on_client_ready(
            hold,
            pending.first_login != 0,
            entity_id,
            addr,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    } else {
        super::super::super::world_entry::cell_dispatch::flush_deferred_aoi(
            entity_id,
            addr,
            "on_client_ready",
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    tracing::info!(
        %addr,
        entity_id,
        entity_name = name,
        "World entry finalized (BeingAppearance resent)"
    );
    Ok(())
}
