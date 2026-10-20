//! One authenticated session: room join, game loop and teardown.
//!
//! [`run_session`] runs after [`super::handle_connection`] has finished the
//! handshake and claimed the registry entry, and hands back the
//! [`SessionEnd`] that the closing row reports.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use super::framing::{
    self, read_null_terminated, send_null_terminated, ReadError, MAX_MESSAGE_LEN,
};
use super::result_dispatch::{
    send_minigame_result, RESULT_CANCELED, RESULT_DEFEAT, RESULT_VICTORY,
};
use super::{EndReason, ReadState, SessionEnd};
use crate::minigame::game::{GameOutput, MinigameInstance};
use crate::minigame::games;
use crate::minigame::protocol::{self, SfsMessage, SfsValue};
use crate::minigame::session::{MinigameSession, SessionRegistry};

/// A session's socket, which stops writing after the first failed send.
///
/// A send fails on an error or after [`framing::SEND_TIMEOUT`]. Either way
/// the stream may hold half a frame, and the peer is gone or not reading,
/// so the teardown frames are skipped rather than each waiting out its own
/// timeout.
struct Outbound {
    stream: TcpStream,
    broken: bool,
}

impl Outbound {
    fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            broken: false,
        }
    }

    async fn send(&mut self, msg: &str) -> Result<(), ()> {
        if self.broken {
            return Err(());
        }
        let sent = send_null_terminated(&mut self.stream, msg).await;
        self.broken = sent.is_err();
        sent
    }
}

/// Keep the lab tap small and free of login credentials or arbitrary SWF data.
fn tap_params(vars: &HashMap<String, SfsValue>) -> BTreeMap<String, serde_json::Value> {
    [
        "wirename",
        "timeRemaining",
        "timerState",
        "countdownRate",
        "PlayerId",
    ]
    .into_iter()
    .filter_map(|key| {
        let value = match vars.get(key)? {
            SfsValue::String(s) => serde_json::Value::String(s.chars().take(64).collect()),
            SfsValue::Number(n) => serde_json::Number::from_f64(*n)?.into(),
            SfsValue::Bool(b) => serde_json::Value::Bool(*b),
            SfsValue::Object(_) => return None,
        };
        Some((key.to_owned(), value))
    })
    .collect()
}

async fn send_extension(
    out: &mut Outbound,
    registry: &SessionRegistry,
    session: &MinigameSession,
    vars: &HashMap<String, SfsValue>,
) -> Result<(), ()> {
    out.send(&protocol::encode_extension(vars)).await?;
    let cmd = vars
        .get("_cmd")
        .and_then(SfsValue::as_str)
        .unwrap_or("extension");
    registry
        .record_traffic(
            session.entity_id,
            &session.ticket,
            "out",
            cmd,
            tap_params(vars),
        )
        .await;
    Ok(())
}

async fn send_named(
    out: &mut Outbound,
    registry: &SessionRegistry,
    session: &MinigameSession,
    msg: &str,
    cmd: &str,
) -> Result<(), ()> {
    out.send(msg).await?;
    registry
        .record_traffic(
            session.entity_id,
            &session.ticket,
            "out",
            cmd,
            BTreeMap::new(),
        )
        .await;
    Ok(())
}

/// Drive one authenticated session: room join, game loop, teardown.
///
/// Split out of [`super::handle_connection`] so the caller owns the single
/// `registry.remove` that every exit path must reach. Before the split the
/// handshake sends each `return`ed straight out of the function, leaking the
/// registry entry on a mid-handshake socket error — the same "entity stuck
/// with a phantom session until relog" shape as defect B4.
///
/// Takes the socket and read buffer by value because nothing after it in
/// [`super::handle_connection`] needs them. `idle_timeout` ends the session as a
/// cancel when the client sends nothing for that long. The returned
/// [`SessionEnd`] says which exit ended the session.
pub(super) async fn run_session(
    stream: TcpStream,
    registry: &SessionRegistry,
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    session: MinigameSession,
    mut game: Box<dyn MinigameInstance>,
    read: ReadState,
    idle_timeout: Duration,
) -> SessionEnd {
    let ReadState {
        mut buf,
        len: mut buf_len,
    } = read;
    let mut out = Outbound::new(stream);
    let entity_id = session.entity_id;
    let room_id = registry.allocate_room_id().await;
    let user_id = entity_id.to_string();
    let mut cancel = session.cancel.subscribe();

    // Send login success sequence (matching C++ exactly)
    let game_name = session.game_name.clone();
    let on_victory_chains = session.on_victory_chains.clone();
    let on_failure_chains = session.on_failure_chains.clone();
    // Who the Discord result names: the character's `player_id` and the
    // name the base handed over at registration.
    let player = session.discord_player();

    // Whether a `MinigameResult` was dispatched upstream. Declared out here
    // rather than beside the game loop because the teardown below is shared
    // by *every* exit path, including a socket that dies mid-handshake.
    let mut result_reported = false;
    // Every exit before the game loop is a failed send, so that is the
    // default; the loop arms below overwrite it with their own reason.
    let mut end_reason = EndReason::SendFailed;

    // Everything up to the end of the game loop lives in this labelled
    // block so a failed send can `break 'session` into the common teardown
    // instead of returning past it. An early `return` here used to mean no
    // `aborted()` and no `RESULT_CANCELED` for a socket that died after
    // `game.started()` had already run.
    'session: {
        // rmList
        let rm_list = format!(
            "<msg t='sys'><body action='rmList' r='0'>\
         <rm id='{room_id}' priv='0' temp='0' game='1' ucnt='1' maxu='1' scnt='0' maxs='100'>\
         <n><![CDATA[{game_name}-{room_id}]]></n></rm></body></msg>"
        );
        if send_named(&mut out, registry, &session, &rm_list, "rmList")
            .await
            .is_err()
        {
            break 'session;
        }

        // loginSucceeded
        let login_ok = protocol::encode_extension_raw(
            "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginSucceeded</var>",
        );
        if send_named(&mut out, registry, &session, &login_ok, "loginSucceeded")
            .await
            .is_err()
        {
            break 'session;
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
        if send_named(&mut out, registry, &session, &join_ok, "joinOK")
            .await
            .is_err()
        {
            break 'session;
        }

        // uCount
        let u_count =
            format!("<msg t='sys'><body action='uCount' r='{room_id}' u='1' s='0'></body></msg>");
        if send_named(&mut out, registry, &session, &u_count, "uCount")
            .await
            .is_err()
        {
            break 'session;
        }

        // Start the game — call started() on the instance
        let outputs = game.started();
        if let Some(board) = game.board_snapshot() {
            registry
                .publish_board(entity_id, &session.ticket, board)
                .await;
        }
        for output in &outputs {
            if let GameOutput::Send(vars) = output {
                if send_extension(&mut out, registry, &session, vars)
                    .await
                    .is_err()
                {
                    break 'session;
                }
            }
        }

        // onPlayerJoinGame
        let join_game = protocol::encode_extension_raw(&format!(
            "<var n='_cmd' t='s'>onPlayerJoinGame</var><var n='PlayerId' t='s'>{user_id}</var>"
        ));
        if send_named(&mut out, registry, &session, &join_game, "onPlayerJoinGame")
            .await
            .is_err()
        {
            break 'session;
        }

        // onGameBegin
        let game_begin = protocol::encode_extension_raw("<var n='_cmd' t='s'>onGameBegin</var>");
        if send_named(&mut out, registry, &session, &game_begin, "onGameBegin")
            .await
            .is_err()
        {
            break 'session;
        }

        tracing::info!(
            entity_id,
            entity_name = player.name(),
            player_id = player.id,
            player_name = player.name(),
            game = %game_name,
            room_id, // nt:id-only a SmartFox room number allocated per session
            "Minigame started"
        );

        // Phase 3: Game loop with tick timer
        let tick_interval = if game.needs_tick() {
            Some(tokio::time::interval(Duration::from_millis(250)))
        } else {
            None
        };

        // `game_complete` alone can't tell a finished game from a dropped
        // socket; only the latter warrants `aborted()`, which is what
        // `result_reported` above discriminates.
        let mut game_complete = false;
        let mut tick_interval = tick_interval;

        // Reset on every parsed request. Junk frames and outbound ticks don't count: a
        // Livewire board keeps sending timer updates to a client that left.
        let idle = tokio::time::sleep(idle_timeout);
        tokio::pin!(idle);

        loop {
            tokio::select! {
                // A window close and the SWF's FIN can arrive together. The
                // close RPC must win so its result cannot end a new board.
                biased;
                _ = async { let _ = cancel.wait_for(|requested| *requested).await; } => {
                    end_reason = EndReason::PlayerCanceled;
                    game_complete = true;
                }
                // Tick timer. Polled before the reads, so a flood of inbound frames cannot starve the deadline.
                _ = async {
                    if let Some(ref mut interval) = tick_interval {
                        interval.tick().await;
                    } else {
                        // No tick needed — sleep forever
                        std::future::pending::<()>().await;
                    }
                } => {
                    let outputs = game.tick();
                    if let Some(board) = game.board_snapshot() {
                        registry.publish_board(entity_id, &session.ticket, board).await;
                    }
                    let mut send_failed = false;
                    for output in outputs {
                        match output {
                            GameOutput::Send(vars) => {
                                // A failed send must not drop the result that follows it in this batch.
                                if !send_failed && send_extension(&mut out, registry, &session, &vars).await.is_err() {
                                    send_failed = true;
                                    if !result_reported {
                                        end_reason = EndReason::SendFailed;
                                    }
                                    game_complete = true;
                                }
                            }
                            GameOutput::Victory => {
                                registry.record_event(entity_id, &session.ticket, "result_victory", None).await;
                                tracing::info!(
                                    entity_id,
                                    entity_name = player.name(),
                                    player_id = player.id,
                                    player_name = player.name(),
                                    game = %game_name,
                                    validation = games::validation(&game_name),
                                    "Minigame victory (tick)",
                                );
                                end_reason = EndReason::Victory;
                                game_complete = true;
                                result_reported = true;
                                send_minigame_result(
                                    result_tx,
                                    entity_id,
                                    &session.ticket,
                                    &game_name,
                                    &player,
                                    RESULT_VICTORY,
                                    on_victory_chains.clone(),
                                    vec![],
                                    "victory_tick",
                                    true,
                                )
                                .await;
                            }
                            GameOutput::Failure => {
                                registry.record_event(entity_id, &session.ticket, "result_defeat", None).await;
                                tracing::info!(
                                    entity_id,
                                    entity_name = player.name(),
                                    player_id = player.id,
                                    player_name = player.name(),
                                    game = %game_name,
                                    validation = games::validation(&game_name),
                                    "Minigame timeout",
                                );
                                end_reason = EndReason::Defeat;
                                game_complete = true;
                                result_reported = true;
                                send_minigame_result(
                                    result_tx,
                                    entity_id,
                                    &session.ticket,
                                    &game_name,
                                    &player,
                                    RESULT_DEFEAT,
                                    vec![],
                                    on_failure_chains.clone(),
                                    "failure_tick",
                                    true,
                                )
                                .await;
                            }
                        }
                    }
                }

                // Read incoming messages
                result = read_null_terminated(&mut out.stream, &mut buf, &mut buf_len) => {
                    match result {
                        Ok(msg) => {
                            let parsed = protocol::parse_message(&msg);
                            match parsed {
                                Ok(SfsMessage::ExtensionRequest { cmd, params }) => {
                                    // Only a parsed request resets the idle deadline; junk frames do not.
                                    idle.as_mut().reset(tokio::time::Instant::now() + idle_timeout);
                                    registry.record_traffic(entity_id, &session.ticket, "in", &cmd, tap_params(&params)).await;
                                    let outputs = game.message(&cmd, &params);
                                    if let Some(board) = game.board_snapshot() {
                                        registry.publish_board(entity_id, &session.ticket, board).await;
                                    }
                                    // A failed send ends the session, but a terminal output after
                                    // it still reports its result (MG-CG3 F4), so no `break` here.
                                    let mut send_failed = false;
                                    for output in outputs {
                                        match output {
                                            GameOutput::Send(vars) => {
                                                // A failed send must not drop the result that follows it in this batch.
                                                if !send_failed && send_extension(&mut out, registry, &session, &vars).await.is_err() {
                                                    send_failed = true;
                                                    if !result_reported {
                                                        end_reason = EndReason::SendFailed;
                                                    }
                                                    game_complete = true;
                                                }
                                            }
                                            GameOutput::Victory => {
                                                registry.record_event(entity_id, &session.ticket, "result_victory", None).await;
                                                tracing::info!(
                                                    entity_id,
                                                    entity_name = player.name(),
                                                    player_id = player.id,
                                                    player_name = player.name(),
                                                    game = %game_name,
                                                    validation = games::validation(&game_name),
                                                    "Minigame victory",
                                                );
                                                end_reason = EndReason::Victory;
                                                game_complete = true;
                                                result_reported = true;
                                                // Fire victory chains
                                                send_minigame_result(
                                                    result_tx,
                                                    entity_id,
                                                    &session.ticket,
                                                    &game_name,
                                                    &player,
                                                    RESULT_VICTORY,
                                                    on_victory_chains.clone(),
                                                    vec![],
                                                    "victory_message",
                                                    true,
                                                )
                                                .await;
                                            }
                                            GameOutput::Failure => {
                                                registry.record_event(entity_id, &session.ticket, "result_defeat", None).await;
                                                tracing::info!(
                                                    entity_id,
                                                    entity_name = player.name(),
                                                    player_id = player.id,
                                                    player_name = player.name(),
                                                    game = %game_name,
                                                    validation = games::validation(&game_name),
                                                    "Minigame failure",
                                                );
                                                end_reason = EndReason::Defeat;
                                                game_complete = true;
                                                result_reported = true;
                                                send_minigame_result(
                                                    result_tx,
                                                    entity_id,
                                                    &session.ticket,
                                                    &game_name,
                                                    &player,
                                                    RESULT_DEFEAT,
                                                    vec![],
                                                    on_failure_chains.clone(),
                                                    "failure_message",
                                                    true,
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                                // The authenticated SWF sent something this
                                // server does not handle: a real protocol
                                // gap, so WARN, naming the session and the
                                // `(t, action)` the client used.
                                Ok(other) => {
                                    let (msg_type, body_action) = other.kind();
                                    tracing::warn!(
                                        entity_id,
                                        entity_name = player.name(),
                                        game = %game_name,
                                        msg_type,
                                        body_action,
                                        reason = "unexpected_sfs_message",
                                        "Unexpected SFS message during game",
                                    );
                                }
                                Err(e) => {
                                    let (msg_type, body_action) = e.kind();
                                    tracing::warn!(
                                        entity_id,
                                        entity_name = player.name(),
                                        game = %game_name,
                                        msg_type,
                                        body_action,
                                        reason = e.reason(),
                                        sample = %framing::frame_sample(&msg),
                                        "Unknown SFS message type",
                                    );
                                }
                            }
                        }
                        Err(ReadError::TooLong) => {
                            tracing::warn!(
                                entity_id,
                                entity_name = player.name(),
                                game = %game_name,
                                limit = MAX_MESSAGE_LEN,
                                reason = "message_too_long",
                                "Minigame frame over the size limit; closing",
                            );
                            end_reason = EndReason::FrameTooLong;
                            game_complete = true;
                        }
                        Err(err @ (ReadError::Closed | ReadError::Io(_))) => {
                            // Connection closed
                            tracing::debug!(
                                entity_id,
                                entity_name = player.name(),
                                "Minigame connection closed",
                            );
                            end_reason = if matches!(err, ReadError::Io(_)) {
                                EndReason::ReadFailed
                            } else {
                                EndReason::ClientClosed
                            };
                            game_complete = true;
                        }
                    }
                }

                // Idle deadline: nothing from the client for `idle_timeout`.
                // Ends like a closed window: `aborted()`, then Canceled.
                () = &mut idle => {
                    tracing::info!(
                        entity_id,
                        entity_name = player.name(),
                        game = %game_name,
                        reason = "idle_timeout",
                        timeout_s = idle_timeout.as_secs(),
                        "Minigame session idle; closing",
                    );
                    end_reason = EndReason::IdleTimeout;
                    game_complete = true;
                }
            }

            if game_complete {
                break;
            }
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
    // actually lost. `handle_minigame_result` on the cell releases
    // `BSF_PlayingMinigame` and the movement lock on every code (D-MG15), so
    // dropping this report strands the player.
    // FIN may have won the read arm just before the close RPC reached the
    // registry. Recheck the watch before reporting an aborted result.
    if !result_reported && *cancel.borrow() {
        end_reason = EndReason::PlayerCanceled;
    }
    if !result_reported {
        tracing::info!(
            entity_id,
            entity_name = player.name(),
            player_id = player.id,
            player_name = player.name(),
            game = %game_name,
            end_reason = end_reason.as_str(),
            "Minigame aborted without a result",
        );
        for output in game.aborted() {
            if let GameOutput::Send(vars) = output {
                // Best-effort: the peer is usually already gone.
                let _ = send_extension(&mut out, registry, &session, &vars).await;
            }
        }
        if let Some(board) = game.board_snapshot() {
            registry
                .publish_board(entity_id, &session.ticket, board)
                .await;
        }
        registry
            .record_event(entity_id, &session.ticket, "result_canceled", None)
            .await;
        send_minigame_result(
            result_tx,
            entity_id,
            &session.ticket,
            &game_name,
            &player,
            RESULT_CANCELED,
            vec![],
            vec![],
            "aborted",
            end_reason != EndReason::PlayerCanceled,
        )
        .await;
    }

    // Cleanup: send close sequence
    let leave = protocol::encode_extension_raw(&format!(
        "<var n='_cmd' t='s'>onPlayerLeaveGame</var><var n='PlayerId' t='s'>{user_id}</var>"
    ));
    let _ = send_named(&mut out, registry, &session, &leave, "onPlayerLeaveGame").await;

    let end = protocol::encode_extension_raw("<var n='_cmd' t='s'>onGameEnd</var>");
    let _ = send_named(&mut out, registry, &session, &end, "onGameEnd").await;

    let room_del =
        format!("<msg t='sys'><body action='roomDel'><rm id='{room_id}' /></body></msg>");
    let _ = send_named(&mut out, registry, &session, &room_del, "roomDel").await;

    SessionEnd {
        reason: end_reason,
        room_id,
    }
}
