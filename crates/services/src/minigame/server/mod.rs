//! SmartFoxServer-compatible TCP server for minigame connections.
//!
//! Each client connection runs in its own tokio task with an independent
//! game instance and tick timer. This module owns the connection lifecycle;
//! the siblings hold the pieces it leans on:
//!
//! - [`framing`] — null-terminated message framing over the socket.
//! - [`handshake`] — the pre-game version check and ticket login.
//! - [`result_dispatch`] — the single seam every outcome leaves through.
//!
//! Reference: `deprecated/cpp/src/baseapp/minigame_connection.cpp`.

mod framing;
mod handshake;
mod result_dispatch;

#[cfg(test)]
mod tests;

use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use self::framing::{read_null_terminated, send_null_terminated, MAX_MESSAGE_LEN};
use self::handshake::{read_and_handle_login, read_and_handle_version};
use self::result_dispatch::{send_minigame_result, RESULT_CANCELED, RESULT_DEFEAT, RESULT_VICTORY};
use super::game::{GameOutput, MinigameInstance};
use super::protocol::{self, SfsMessage};
use super::session::{MinigameSession, SessionRegistry, PENDING_SESSION_TTL, SWEEP_INTERVAL};
use crate::cell::messages::CellToBaseMsg;

/// Start the minigame TCP server.
pub async fn run(
    addr: &str,
    port: u16,
    external_port: u16,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
) {
    let listen_addr = format!("{addr}:{port}");
    let listener = match TcpListener::bind(&listen_addr).await {
        Ok(l) => {
            tracing::info!(addr = %listen_addr, "Minigame server listening");
            l
        }
        Err(e) => {
            tracing::error!(addr = %listen_addr, error = %e, "Failed to bind minigame server");
            return;
        }
    };

    // Defect B4: a session whose SWF never connects has no connection task
    // to clean it up, so without this sweep it pins its entity id in the
    // registry until the player relogs.
    registry.spawn_sweep(PENDING_SESSION_TTL, SWEEP_INTERVAL);

    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tracing::debug!(peer = %peer, "Minigame connection accepted");
                let reg = registry.clone();
                let tx = result_tx.clone();
                tokio::spawn(handle_connection(stream, reg, tx, external_port));
            }
            Err(e) => {
                tracing::warn!(error = %e, "Minigame accept error");
            }
        }
    }
}

/// Handle a single minigame connection through the full lifecycle.
async fn handle_connection(
    mut stream: TcpStream,
    registry: SessionRegistry,
    result_tx: mpsc::Sender<CellToBaseMsg>,
    external_port: u16,
) {
    let mut buf = vec![0u8; MAX_MESSAGE_LEN];
    let mut buf_len = 0usize;

    // Phase 1: Version check
    let api_version =
        match read_and_handle_version(&mut stream, &mut buf, &mut buf_len, external_port).await {
            Some(v) => v,
            None => return,
        };

    // Phase 2: Login
    let (session, game) =
        match read_and_handle_login(&mut stream, &mut buf, &mut buf_len, api_version, &registry)
            .await
        {
            Some(pair) => pair,
            None => return,
        };

    let entity_id = session.entity_id;

    // From here the session belongs to this task: the sweep must stop
    // considering it, and *every* exit path below has to unregister it or
    // the player can never start another minigame without relogging
    // (defect B4).
    registry.mark_connected(entity_id).await;
    let game_name = session.game_name.clone();

    run_session(stream, &registry, &result_tx, session, game, buf, buf_len).await;

    registry.remove(entity_id).await;
    tracing::info!(entity_id, game = %game_name, "Minigame session ended");
}

/// Drive one authenticated session: room join, game loop, teardown.
///
/// Split out of [`handle_connection`] so the caller owns the single
/// `registry.remove` that every exit path must reach. Before the split the
/// handshake sends each `return`ed straight out of the function, leaking the
/// registry entry on a mid-handshake socket error — the same "entity stuck
/// with a phantom session until relog" shape as defect B4.
///
/// Takes the socket and read buffer by value because nothing after it in
/// [`handle_connection`] needs them.
async fn run_session(
    mut stream: TcpStream,
    registry: &SessionRegistry,
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    session: MinigameSession,
    mut game: Box<dyn MinigameInstance>,
    mut buf: Vec<u8>,
    mut buf_len: usize,
) {
    let entity_id = session.entity_id;
    let room_id = registry.allocate_room_id().await;
    let user_id = entity_id.to_string();

    // Send login success sequence (matching C++ exactly)
    let game_name = session.game_name.clone();
    let on_victory_chains = session.on_victory_chains.clone();

    // rmList
    let rm_list = format!(
        "<msg t='sys'><body action='rmList' r='0'>\
         <rm id='{room_id}' priv='0' temp='0' game='1' ucnt='1' maxu='1' scnt='0' maxs='100'>\
         <n><![CDATA[{game_name}-{room_id}]]></n></rm></body></msg>"
    );
    if send_null_terminated(&mut stream, &rm_list).await.is_err() {
        return;
    }

    // loginSucceeded
    let login_ok = protocol::encode_extension_raw(
        "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginSucceeded</var>",
    );
    if send_null_terminated(&mut stream, &login_ok).await.is_err() {
        return;
    }

    // joinOK with game params
    let join_ok = format!(
        "<msg t='sys'><body action='joinOK' r='{room_id}'>\
         <pid id='1' />\
         <vars>\
         <var n='abilityBitfield' t='n'><![CDATA[{}]]></var>\
         <var n='seed' t='n'><![CDATA[{}]]></var>\
         <var n='difficulty' t='n'><![CDATA[{}]]></var>\
         <var n='techcomp' t='n'><![CDATA[{}]]></var>\
         <var n='pclevel' t='n'><![CDATA[{}]]></var>\
         <var n='intelligence' t='n'><![CDATA[{}]]></var>\
         <var n='instcc' t='n'><![CDATA[-1]]></var>\
         <var n='CA0' t='n'><![CDATA[0]]></var>\
         <var n='CA1' t='n'><![CDATA[0]]></var>\
         <var n='CA2' t='n'><![CDATA[0]]></var>\
         <var n='CA3' t='n'><![CDATA[0]]></var>\
         <var n='CA4' t='n'><![CDATA[0]]></var>\
         </vars>\
         <uLs r='{room_id}'>\
         <u i='999' m='0' s='0' p='1'><n><![CDATA[{user_id}]]></n><vars></vars></u>\
         </uLs>\
         </body></msg>",
        session.abilities_mask,
        session.seed,
        session.difficulty,
        session.tech_competency,
        session.player_level,
        session.intelligence,
    );
    if send_null_terminated(&mut stream, &join_ok).await.is_err() {
        return;
    }

    // uCount
    let u_count =
        format!("<msg t='sys'><body action='uCount' r='{room_id}' u='1' s='0'></body></msg>");
    if send_null_terminated(&mut stream, &u_count).await.is_err() {
        return;
    }

    // Start the game — call started() on the instance
    let outputs = game.started();
    for output in &outputs {
        if let GameOutput::Send(vars) = output {
            let msg = protocol::encode_extension(vars);
            if send_null_terminated(&mut stream, &msg).await.is_err() {
                return;
            }
        }
    }

    // onPlayerJoinGame
    let join_game = protocol::encode_extension_raw(&format!(
        "<var n='_cmd' t='s'>onPlayerJoinGame</var><var n='PlayerId' t='s'>{user_id}</var>"
    ));
    if send_null_terminated(&mut stream, &join_game).await.is_err() {
        return;
    }

    // onGameBegin
    let game_begin = protocol::encode_extension_raw("<var n='_cmd' t='s'>onGameBegin</var>");
    if send_null_terminated(&mut stream, &game_begin)
        .await
        .is_err()
    {
        return;
    }

    tracing::info!(entity_id, game = %game_name, room_id, "Minigame started");

    // Phase 3: Game loop with tick timer
    let tick_interval = if game.needs_tick() {
        Some(tokio::time::interval(Duration::from_millis(250)))
    } else {
        None
    };

    let mut game_complete = false;
    // Whether a `MinigameResult` was dispatched upstream. `game_complete`
    // alone can't tell a finished game from a dropped socket, and only the
    // latter warrants `aborted()`.
    let mut result_reported = false;
    let mut tick_interval = tick_interval;

    loop {
        tokio::select! {
            // Read incoming messages
            result = read_null_terminated(&mut stream, &mut buf, &mut buf_len) => {
                match result {
                    Some(msg) => {
                        let parsed = protocol::parse_message(&msg);
                        match parsed {
                            Some(SfsMessage::ExtensionRequest { cmd, params }) => {
                                let outputs = game.message(&cmd, &params);
                                for output in outputs {
                                    match output {
                                        GameOutput::Send(vars) => {
                                            let encoded = protocol::encode_extension(&vars);
                                            if send_null_terminated(&mut stream, &encoded).await.is_err() {
                                                game_complete = true;
                                                break;
                                            }
                                        }
                                        GameOutput::Victory => {
                                            tracing::info!(entity_id, game = %game_name, "Minigame victory");
                                            game_complete = true;
                                            result_reported = true;
                                            // Fire victory chains
                                            send_minigame_result(
                                                result_tx,
                                                entity_id,
                                                &game_name,
                                                RESULT_VICTORY,
                                                on_victory_chains.clone(),
                                                "victory_message",
                                            )
                                            .await;
                                        }
                                        GameOutput::Failure => {
                                            tracing::info!(entity_id, game = %game_name, "Minigame failure");
                                            game_complete = true;
                                            result_reported = true;
                                            send_minigame_result(
                                                result_tx,
                                                entity_id,
                                                &game_name,
                                                RESULT_DEFEAT,
                                                vec![],
                                                "failure_message",
                                            )
                                            .await;
                                        }
                                    }
                                }
                            }
                            _ => {
                                tracing::warn!(entity_id, "Unexpected message type during game");
                            }
                        }
                    }
                    None => {
                        // Connection closed
                        tracing::debug!(entity_id, "Minigame connection closed");
                        game_complete = true;
                    }
                }
            }

            // Tick timer
            _ = async {
                if let Some(ref mut interval) = tick_interval {
                    interval.tick().await;
                } else {
                    // No tick needed — sleep forever
                    std::future::pending::<()>().await;
                }
            } => {
                let outputs = game.tick();
                for output in outputs {
                    match output {
                        GameOutput::Send(vars) => {
                            let encoded = protocol::encode_extension(&vars);
                            if send_null_terminated(&mut stream, &encoded).await.is_err() {
                                game_complete = true;
                                break;
                            }
                        }
                        GameOutput::Victory => {
                            tracing::info!(entity_id, game = %game_name, "Minigame victory (tick)");
                            game_complete = true;
                            result_reported = true;
                            send_minigame_result(
                                result_tx,
                                entity_id,
                                &game_name,
                                RESULT_VICTORY,
                                on_victory_chains.clone(),
                                "victory_tick",
                            )
                            .await;
                        }
                        GameOutput::Failure => {
                            tracing::info!(entity_id, game = %game_name, "Minigame timeout");
                            game_complete = true;
                            result_reported = true;
                            send_minigame_result(
                                result_tx,
                                entity_id,
                                &game_name,
                                RESULT_DEFEAT,
                                vec![],
                                "failure_tick",
                            )
                            .await;
                        }
                    }
                }
            }
        }

        if game_complete {
            break;
        }
    }

    // Defect B4: the loop can also exit because the SWF closed or the
    // socket dropped, with no Victory/Failure ever produced. Give the
    // instance its documented teardown call — before this, `aborted()` had
    // no call site anywhere in the tree and a half-played Livewire board
    // was simply dropped.
    //
    // Ordering and the upstream report both mirror C++ `Minigame::abort`
    // (`deprecated/cpp/src/baseapp/minigame.cpp`): `aborted()` first, then
    // the close sequence, and `MinigameCanceled` (0) upstream — *not*
    // `Defeat` (2), which the original reserved for a game the player
    // actually lost. `handle_minigame_result` on the cell only acts on
    // victory, so today code 0 is inert there; it matters because the
    // original's cell handler is what cleared `BSF_PlayingMinigame` and
    // released the movement lock. When either of those lands, dropping this
    // report would strand the player.
    if !result_reported {
        tracing::info!(
            entity_id,
            game = %game_name,
            "Minigame aborted -- client closed without reporting a result"
        );
        for output in game.aborted() {
            if let GameOutput::Send(vars) = output {
                // Best-effort: the peer is usually already gone.
                let encoded = protocol::encode_extension(&vars);
                let _ = send_null_terminated(&mut stream, &encoded).await;
            }
        }
        send_minigame_result(
            result_tx,
            entity_id,
            &game_name,
            RESULT_CANCELED,
            vec![],
            "aborted",
        )
        .await;
    }

    // Cleanup: send close sequence
    let leave = protocol::encode_extension_raw(&format!(
        "<var n='_cmd' t='s'>onPlayerLeaveGame</var><var n='PlayerId' t='s'>{user_id}</var>"
    ));
    let _ = send_null_terminated(&mut stream, &leave).await;

    let end = protocol::encode_extension_raw("<var n='_cmd' t='s'>onGameEnd</var>");
    let _ = send_null_terminated(&mut stream, &end).await;

    let room_del =
        format!("<msg t='sys'><body action='roomDel'><rm id='{room_id}' /></body></msg>");
    let _ = send_null_terminated(&mut stream, &room_del).await;
}
