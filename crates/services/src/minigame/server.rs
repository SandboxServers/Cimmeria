//! SmartFoxServer-compatible TCP server for minigame connections.
//!
//! Each client connection runs in its own tokio task with an independent
//! game instance and tick timer.
//!
//! Reference: `src/baseapp/minigame_connection.cpp`

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

use super::game::{create_game, GameOutput, MinigameInstance};
use super::protocol::{self, SfsMessage};
use super::session::{MinigameSession, SessionRegistry};
use crate::cell::messages::CellToBaseMsg;

const API_VERSION: u32 = 154;
const MAX_MESSAGE_LEN: usize = 0x1000;

/// Dispatch a `MinigameResult` upstream and log on send failure.
///
/// Extracted from the inline call sites in [`handle_connection`] so
/// the four (game-driven / tick-driven × victory / failure) variants
/// share one error-handling path. `phase` names which call site fired
/// so the ops log distinguishes them. Historically these were
/// `let _ = result_tx.send(...).await`, silently swallowing the loss
/// of a minigame outcome — chains never fired, quest stalled.
async fn send_minigame_result(
    result_tx: &mpsc::Sender<CellToBaseMsg>,
    entity_id: u32,
    game_name: &str,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    phase: &'static str,
) {
    // Discord gameplay-channel: a minigame finished (on by default — low
    // volume / high signal). The minigame server only holds the entity id,
    // so the character name is best-effort (`entity:<id>`); resolving the
    // display name would require a cross-service round-trip not worth the
    // coupling here. `result_code` 1 = victory, anything else = loss.
    cimmeria_discord::emit_minigame_result(
        game_name,
        format!("entity:{entity_id}"),
        result_code == 1,
    );

    let chain_count = on_victory_chains.len();
    if let Err(e) = result_tx
        .send(CellToBaseMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        })
        .await
    {
        tracing::error!(
            entity_id,
            game = %game_name,
            result_code,
            chain_count,
            phase,
            "Minigame: result delivery failed -- chains will not fire: {e}"
        );
    }
}

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
    let (session, mut game) =
        match read_and_handle_login(&mut stream, &mut buf, &mut buf_len, api_version, &registry)
            .await
        {
            Some(pair) => pair,
            None => return,
        };

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
                                            // Fire victory chains
                                            send_minigame_result(
                                                &result_tx,
                                                entity_id,
                                                &game_name,
                                                1, // Victory
                                                on_victory_chains.clone(),
                                                "victory_message",
                                            )
                                            .await;
                                        }
                                        GameOutput::Failure => {
                                            tracing::info!(entity_id, game = %game_name, "Minigame failure");
                                            game_complete = true;
                                            send_minigame_result(
                                                &result_tx,
                                                entity_id,
                                                &game_name,
                                                2, // Defeat
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
                            send_minigame_result(
                                &result_tx,
                                entity_id,
                                &game_name,
                                1,
                                on_victory_chains.clone(),
                                "victory_tick",
                            )
                            .await;
                        }
                        GameOutput::Failure => {
                            tracing::info!(entity_id, game = %game_name, "Minigame timeout");
                            game_complete = true;
                            send_minigame_result(
                                &result_tx,
                                entity_id,
                                &game_name,
                                2,
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

    // Remove session
    registry.remove(entity_id).await;
    tracing::info!(entity_id, game = %game_name, "Minigame session ended");
}

// ── Handshake helpers ────────────────────────────────────────────────────────

async fn read_and_handle_version(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    external_port: u16,
) -> Option<u32> {
    let msg = read_null_terminated(stream, buf, buf_len).await?;
    let parsed = protocol::parse_message(&msg)?;

    match parsed {
        SfsMessage::VersionCheck { version } => {
            // Cross-domain policy (required by Flash XMLSocket)
            let policy = format!(
                "<cross-domain-policy><allow-access-from domain='*' to-ports='{external_port}' /></cross-domain-policy>"
            );
            send_null_terminated(stream, &policy).await.ok()?;

            // API OK (always send OK even if version mismatches — C++ comment explains why)
            let api_ok = "<msg t='sys'><body action='apiOK' r='0'></body></msg>";
            send_null_terminated(stream, api_ok).await.ok()?;

            Some(version)
        }
        _ => {
            tracing::warn!("Expected verChk, got something else");
            None
        }
    }
}

async fn read_and_handle_login(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
    api_version: u32,
    registry: &SessionRegistry,
) -> Option<(MinigameSession, Box<dyn MinigameInstance>)> {
    let msg = read_null_terminated(stream, buf, buf_len).await?;
    let parsed = protocol::parse_message(&msg)?;

    match parsed {
        SfsMessage::Login {
            zone,
            nick,
            password,
        } => {
            // Check API version
            if api_version != API_VERSION {
                let fail = protocol::encode_extension_raw(
                    "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>",
                );
                let _ = send_null_terminated(stream, &fail).await;
                tracing::warn!(api_version, expected = API_VERSION, "Bad API version");
                return None;
            }

            let entity_id: u32 = nick.parse().ok()?;
            let session = registry.authenticate(entity_id, &password, &zone).await?;

            // Create game instance
            let game = create_game(&session);
            if game.is_none() {
                let fail = protocol::encode_extension_raw(
                    "<var n='id' t='n'>999</var><var n='_cmd' t='s'>loginFailed</var>",
                );
                let _ = send_null_terminated(stream, &fail).await;
                tracing::warn!(entity_id, game = %zone, "Failed to create minigame");
                return None;
            }

            tracing::info!(entity_id, game = %zone, "Minigame login successful");
            Some((session, game.unwrap()))
        }
        _ => {
            tracing::warn!("Expected login, got something else");
            None
        }
    }
}

// ── Low-level I/O ────────────────────────────────────────────────────────────

/// Read a null-terminated message from the stream.
/// Returns None on EOF or error.
async fn read_null_terminated(
    stream: &mut TcpStream,
    buf: &mut [u8],
    buf_len: &mut usize,
) -> Option<String> {
    loop {
        // Check if we already have a complete message in the buffer
        if let Some(null_pos) = buf[..*buf_len].iter().position(|&b| b == 0) {
            let msg = String::from_utf8_lossy(&buf[..null_pos]).to_string();
            // Shift remaining data
            let remaining = *buf_len - null_pos - 1;
            if remaining > 0 {
                buf.copy_within(null_pos + 1..*buf_len, 0);
            }
            *buf_len = remaining;
            return Some(msg);
        }

        // Need more data
        if *buf_len >= buf.len() {
            tracing::warn!("Minigame message too long (>{} bytes)", buf.len());
            return None;
        }

        match stream.read(&mut buf[*buf_len..]).await {
            Ok(0) => return None, // EOF
            Ok(n) => *buf_len += n,
            Err(e) => {
                tracing::debug!(error = %e, "Minigame read error");
                return None;
            }
        }
    }
}

/// Send a null-terminated message to the stream.
async fn send_null_terminated(stream: &mut TcpStream, msg: &str) -> Result<(), ()> {
    let mut data = msg.as_bytes().to_vec();
    data.push(0); // null terminator
    stream.write_all(&data).await.map_err(|e| {
        tracing::debug!(error = %e, "Minigame send error");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    /// `send_minigame_result` is the shared helper extracted
    /// from the four inline call sites in `handle_connection` (game-
    /// driven victory/failure + tick-driven victory/failure). Dropping
    /// the receiver simulates a downed cell↔base channel; the guard
    /// asserts the ERROR fires naming the `phase` so all four sites
    /// stay distinguishable in ops logs.
    #[tokio::test]
    async fn send_minigame_result_logs_error_when_receiver_closed() {
        let capture = LogCapture::install();
        let (tx, rx) = mpsc::channel::<CellToBaseMsg>(1);
        drop(rx);

        send_minigame_result(
            &tx,
            /* entity_id */ 4242,
            /* game_name */ "livewire",
            /* result_code */ 1,
            /* on_victory_chains */ vec![100, 200],
            /* phase */ "victory_message",
        )
        .await;

        let event = capture
            .find_message(Level::ERROR, "Minigame: result delivery failed")
            .expect("negative-logging convention: closed result_tx must emit ERROR");
        assert!(
            event.has_field("phase", "victory_message"),
            "phase field must be carried so all four call sites \
             (victory_message / failure_message / victory_tick / failure_tick) \
             stay distinguishable in ops logs: {:#?}",
            event
        );
        assert!(
            event.has_field("chain_count", "2"),
            "chain_count must reflect the on_victory_chains length so a \
             quest-stall investigation can correlate the count of lost \
             chains: {:#?}",
            event
        );
    }

    /// Happy-path: receiver alive, exactly one `MinigameResult` arrives
    /// with the right shape. Guards against a regression that drops
    /// the result silently when the channel IS healthy (e.g., a future
    /// refactor that adds a "skip-if-empty-chains" branch).
    #[tokio::test]
    async fn send_minigame_result_dispatches_through_open_channel() {
        let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(1);

        send_minigame_result(&tx, 99, "hack", 2, vec![], "failure_tick").await;

        match rx.try_recv() {
            Ok(CellToBaseMsg::MinigameResult {
                entity_id,
                result_code,
                on_victory_chains,
            }) => {
                assert_eq!(entity_id, 99);
                assert_eq!(result_code, 2);
                assert!(on_victory_chains.is_empty());
            }
            other => panic!("expected MinigameResult, got {other:?}"),
        }
        assert!(rx.try_recv().is_err(), "exactly one message");
    }
}
