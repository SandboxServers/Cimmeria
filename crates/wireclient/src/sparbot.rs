//! `sparbot`: a second duelist for a tester who is alone (SS-U2).
//!
//! The bot is an in-world [`GameSession`] that accepts every duel challenge
//! addressed to it, stands still, and forfeits after a configurable time.
//! The `sparbot` binary (`src/bin/sparbot.rs`) logs it in; this module holds
//! the parts a test can drive without a network: the duel wire builders,
//! the decoder for the two server messages the bot reacts to, the
//! [`Sparbot`] state machine and the [`run`] loop.
//!
//! # Wire
//!
//! Indices from `docs/protocol/`, pinned against `cimmeria-wire` by the
//! crate's integration tests (`tests/it/sparbot_duel.rs`):
//!
//! - `sendDuelChallenge(WSTRING name, INT8 squad)`: SGWPlayer base method
//!   `0xD9` (base index 25).
//! - `sendDuelResponse(INT8 accept)`: cell method 102; 1 accepts.
//! - `duelForfeit()`: cell method 103, no arguments. The server logs it as
//!   `UNIMPLEMENTED` until SS-D3; the bot sends it anyway and says so.
//! - `onDuelChallenge(INT32 challengerEntityId, ARRAY<INT32> squad)`: client
//!   method 143, sent to the target's own entity.
//! - `onPlayerCommunication(WSTRING speaker, UINT8 flags, UINT8 channel,
//!   WSTRING text)`: client method 28; the duel texts ride it.
//!
//! # Keeping the session alive
//!
//! The server drops a client it has not heard from for 60 s
//! (`base-session` `tick_sync.rs`, `INACTIVITY_TIMEOUT`), and its reliable
//! sends wait for acks that `LoopbackPeer` only piggybacks on the next
//! outbound packet. A bot that only listened would lose its session. The
//! real client sends `AUTHENTICATE` on every tick while idle (about six per
//! second, measured on the colo), so [`run`] sends one every
//! [`HEARTBEAT_EVERY`] as an unreliable packet, which also carries the
//! pending acks.

use std::time::{Duration, Instant};

use crate::bundle::{decode_bundle, S2CMessage};
use crate::session::GameSession;

/// `sendDuelChallenge`, SGWPlayer base method index 25.
pub const SEND_DUEL_CHALLENGE: u8 = 0xD9;
/// `sendDuelResponse`, SGWPlayer cell method.
pub const SEND_DUEL_RESPONSE: u16 = 102;
/// `duelForfeit`, SGWPlayer cell method.
pub const DUEL_FORFEIT: u16 = 103;
/// `onDuelChallenge`, SGWPlayer client method.
pub const ON_DUEL_CHALLENGE: u16 = 143;
/// `onPlayerCommunication`, SGWPlayer client method.
pub const ON_PLAYER_COMMUNICATION: u16 = 28;
/// The server's "Duel aborted" line (moniker 878): the duel is over.
pub const TEXT_DUEL_ABORTED: &str = "Duel aborted";

/// How often [`run`] sends its keep-alive `AUTHENTICATE`.
pub const HEARTBEAT_EVERY: Duration = Duration::from_millis(250);
/// [`run`] gives up when the server has sent nothing for this long. The
/// server's tickSync runs at 10 Hz, so a silence this long means the
/// session is gone; it is the client's own `NetInactivityTimeout`.
pub const SERVER_SILENCE_LIMIT: Duration = Duration::from_secs(15);

/// `sendDuelChallenge(name, squad)` as a base-method message.
pub fn send_duel_challenge(name: &str, squad: bool) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut args = Vec::with_capacity(5 + units.len() * 2);
    args.extend_from_slice(&(units.len() as u32).to_le_bytes());
    for u in units {
        args.extend_from_slice(&u.to_le_bytes());
    }
    args.push(u8::from(squad));
    GameSession::base_method(SEND_DUEL_CHALLENGE, &args)
}

/// `sendDuelResponse(accept)` on the bot's own entity.
pub fn send_duel_response(own_entity_id: u32, accept: bool) -> Vec<u8> {
    GameSession::cell_method(SEND_DUEL_RESPONSE, own_entity_id, &[u8::from(accept)])
}

/// `duelForfeit()` on the bot's own entity.
pub fn duel_forfeit(own_entity_id: u32) -> Vec<u8> {
    GameSession::cell_method(DUEL_FORFEIT, own_entity_id, &[])
}

/// A server message the bot reacts to or reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    /// `onDuelChallenge` addressed to the bot.
    DuelChallenge { challenger_entity_id: i32 },
    /// `onPlayerCommunication` to the bot: a feedback or chat line.
    Line {
        speaker: String,
        channel: u8,
        text: String,
    },
}

/// Classify one decoded message. Only entity methods on the bot's own
/// entity count; everything else (AoI traffic, other entities) is `None`.
pub fn classify(msg: &S2CMessage, own_entity_id: u32) -> Option<Incoming> {
    if msg.entity_id != Some(own_entity_id) || msg.msg_id < 0x80 {
        return None;
    }
    // The payload keeps the entity id, and the sub-index byte after it on
    // the extended (0xBD) encoding.
    let skip = if msg.msg_id == 0xBD { 5 } else { 4 };
    let args = msg.payload.get(skip..)?;
    match msg.method_index? {
        ON_DUEL_CHALLENGE => {
            let id = i32::from_le_bytes(args.get(0..4)?.try_into().ok()?);
            Some(Incoming::DuelChallenge {
                challenger_entity_id: id,
            })
        }
        ON_PLAYER_COMMUNICATION => {
            let mut r = args;
            let speaker = read_wstring(&mut r)?;
            let (&[_flags, channel], rest) = r.split_first_chunk::<2>()?;
            r = rest;
            let text = read_wstring(&mut r)?;
            Some(Incoming::Line {
                speaker,
                channel,
                text,
            })
        }
        _ => None,
    }
}

/// `WSTRING`: a `u32` count of UTF-16 units, then the units.
fn read_wstring(r: &mut &[u8]) -> Option<String> {
    let (count, rest) = r.split_first_chunk::<4>()?;
    let n = u32::from_le_bytes(*count) as usize;
    let bytes = rest.get(..n.checked_mul(2)?)?;
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&c| u16::from_le_bytes(c))
        .collect();
    *r = &rest[n * 2..];
    Some(String::from_utf16_lossy(&units))
}

/// How the bot behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparbotConfig {
    /// Forfeit this long after accepting, if the duel is still on. `None`
    /// never forfeits.
    pub forfeit_after: Option<Duration>,
}

/// What the bot did, for the binary's summary and the tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SparbotReport {
    pub challenges_accepted: u32,
    pub forfeits_sent: u32,
    /// Every line the server sent the bot, in order.
    pub lines: Vec<String>,
    /// Why [`run`] returned, once it has.
    pub stopped: Option<StopReason>,
}

/// Why [`run`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The requested run time elapsed.
    RunTimeElapsed,
    /// Nothing from the server for [`SERVER_SILENCE_LIMIT`].
    ServerSilent,
    /// A send failed: the socket is gone.
    SendFailed,
}

/// The duel the bot accepted and has not seen end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Accepted {
    challenger_entity_id: i32,
    at: Instant,
}

/// The bot's decisions, on an explicit clock.
#[derive(Debug)]
pub struct Sparbot {
    own_entity_id: u32,
    config: SparbotConfig,
    accepted: Option<Accepted>,
    pub report: SparbotReport,
}

impl Sparbot {
    pub fn new(own_entity_id: u32, config: SparbotConfig) -> Self {
        Self {
            own_entity_id,
            config,
            accepted: None,
            report: SparbotReport::default(),
        }
    }

    /// React to one message; returns the message to send, if any. A
    /// challenge is always accepted, even over a duel the bot thinks is
    /// still on: the server decides whether the bot is free.
    pub fn on_incoming(&mut self, incoming: Incoming, now: Instant) -> Option<Vec<u8>> {
        match incoming {
            Incoming::DuelChallenge {
                challenger_entity_id,
            } => {
                tracing::info!(
                    target: "sparbot",
                    event = "sparbot.challenge_accepted",
                    entity_id = self.own_entity_id,
                    challenger_entity_id,
                    "duel challenge received; accepting"
                );
                self.accepted = Some(Accepted {
                    challenger_entity_id,
                    at: now,
                });
                self.report.challenges_accepted += 1;
                Some(send_duel_response(self.own_entity_id, true))
            }
            Incoming::Line {
                speaker,
                channel,
                text,
            } => {
                tracing::info!(
                    target: "sparbot",
                    event = "sparbot.server_line",
                    entity_id = self.own_entity_id,
                    speaker,
                    channel,
                    text,
                    "line from the server"
                );
                if text == TEXT_DUEL_ABORTED {
                    self.accepted = None;
                }
                self.report.lines.push(text);
                None
            }
        }
    }

    /// The forfeit, once `forfeit_after` has passed since the accept.
    pub fn poll(&mut self, now: Instant) -> Option<Vec<u8>> {
        let after = self.config.forfeit_after?;
        let duel = self.accepted?;
        if now.duration_since(duel.at) < after {
            return None;
        }
        self.accepted = None;
        self.report.forfeits_sent += 1;
        tracing::info!(
            target: "sparbot",
            event = "sparbot.forfeit_sent",
            entity_id = self.own_entity_id,
            challenger_entity_id = duel.challenger_entity_id,
            after_secs = after.as_secs_f32(),
            "forfeiting the duel (duelForfeit, CM 103; the server ignores it until SS-D3)"
        );
        Some(duel_forfeit(self.own_entity_id))
    }
}

/// Run `bot` on an in-world `session` for `run_for` (forever when `None`),
/// or until the session is lost. Sets `bot.report.stopped`.
pub async fn run(session: &GameSession, bot: &mut Sparbot, run_for: Option<Duration>) {
    let start = Instant::now();
    let mut last_heartbeat = start - HEARTBEAT_EVERY;
    let mut last_heard = start;
    let reason = loop {
        let now = Instant::now();
        if run_for.is_some_and(|d| now.duration_since(start) >= d) {
            break StopReason::RunTimeElapsed;
        }
        if now.duration_since(last_heard) >= SERVER_SILENCE_LIMIT {
            tracing::warn!(
                target: "sparbot",
                event = "sparbot.session_lost",
                entity_id = bot.own_entity_id,
                reason = "server_silent",
                silent_secs = now.duration_since(last_heard).as_secs(),
                "no traffic from the server; the session is gone"
            );
            break StopReason::ServerSilent;
        }
        let mut out: Vec<(Vec<u8>, bool)> = Vec::new();
        if now.duration_since(last_heartbeat) >= HEARTBEAT_EVERY {
            out.push((GameSession::authenticate(), false));
            last_heartbeat = now;
        }
        if let Some(msg) = bot.poll(now) {
            out.push((msg, true));
        }
        for bundle in session.recv_bundles(1, Duration::from_millis(50)).await {
            last_heard = Instant::now();
            for msg in decode_bundle(&bundle) {
                if let Some(incoming) = classify(&msg, bot.own_entity_id) {
                    if let Some(reply) = bot.on_incoming(incoming, Instant::now()) {
                        out.push((reply, true));
                    }
                }
            }
        }
        for (msg, reliable) in out {
            if let Err(e) = session.send_bundle(&msg, reliable).await {
                tracing::warn!(
                    target: "sparbot",
                    event = "sparbot.session_lost",
                    entity_id = bot.own_entity_id,
                    reason = "send_failed",
                    error = %e,
                    "send to the server failed"
                );
                bot.report.stopped = Some(StopReason::SendFailed);
                return;
            }
        }
    };
    bot.report.stopped = Some(reason);
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    const OWN: u32 = 0x0102_0304;

    /// A decoded entity-method message as the bundle decoder builds it.
    fn method_msg(entity: u32, method_index: u16, args: &[u8]) -> S2CMessage {
        let mut payload = entity.to_le_bytes().to_vec();
        let msg_id = if method_index < 61 {
            0x80 | method_index as u8
        } else {
            payload.push((method_index - 61) as u8);
            0xBD
        };
        payload.extend_from_slice(args);
        S2CMessage {
            msg_id,
            entity_id: Some(entity),
            class_id: None,
            method_index: Some(method_index),
            payload: Bytes::from(payload),
        }
    }

    fn wstring(s: &str) -> Vec<u8> {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut v = (units.len() as u32).to_le_bytes().to_vec();
        for u in units {
            v.extend_from_slice(&u.to_le_bytes());
        }
        v
    }

    /// `sendDuelChallenge("Bo", false)`: `0xD9`, word length 9, the
    /// WSTRING, then the squad byte.
    #[test]
    fn challenge_bytes() {
        assert_eq!(
            send_duel_challenge("Bo", false),
            [0xD9, 9, 0, 2, 0, 0, 0, 0x42, 0, 0x6F, 0, 0]
        );
    }

    /// Accept is CM 102 with one byte 1 (extended, sub-slot 41); forfeit is
    /// CM 103 with no arguments (sub-slot 42).
    #[test]
    fn response_and_forfeit_bytes() {
        assert_eq!(
            send_duel_response(OWN, true),
            [0xBD, 6, 0, 0x04, 0x03, 0x02, 0x01, 41, 1]
        );
        assert_eq!(send_duel_response(OWN, false)[8], 0);
        assert_eq!(duel_forfeit(OWN), [0xBD, 5, 0, 0x04, 0x03, 0x02, 0x01, 42]);
    }

    /// `onDuelChallenge(77, [])` on the bot's entity is a challenge; the
    /// same call on another entity is ignored.
    #[test]
    fn classify_challenge_only_on_own_entity() {
        let args = [77i32.to_le_bytes(), 0u32.to_le_bytes()].concat();
        assert_eq!(
            classify(&method_msg(OWN, ON_DUEL_CHALLENGE, &args), OWN),
            Some(Incoming::DuelChallenge {
                challenger_entity_id: 77
            })
        );
        assert_eq!(
            classify(&method_msg(OWN + 1, ON_DUEL_CHALLENGE, &args), OWN),
            None
        );
    }

    /// A feedback line decodes speaker, channel and text.
    #[test]
    fn classify_feedback_line() {
        let args = [wstring("SYSTEM"), vec![0, 9], wstring("Duel aborted")].concat();
        assert_eq!(
            classify(&method_msg(OWN, ON_PLAYER_COMMUNICATION, &args), OWN),
            Some(Incoming::Line {
                speaker: "SYSTEM".into(),
                channel: 9,
                text: "Duel aborted".into(),
            })
        );
        // Truncated text: not a line, not a panic.
        let short = &args[..args.len() - 3];
        assert_eq!(
            classify(&method_msg(OWN, ON_PLAYER_COMMUNICATION, short), OWN),
            None
        );
    }

    /// Accept at once, forfeit `forfeit_after` later, once.
    #[test]
    fn accepts_then_forfeits_once() {
        let mut bot = Sparbot::new(
            OWN,
            SparbotConfig {
                forfeit_after: Some(Duration::from_secs(10)),
            },
        );
        let t0 = Instant::now();
        let reply = bot.on_incoming(
            Incoming::DuelChallenge {
                challenger_entity_id: 5,
            },
            t0,
        );
        assert_eq!(reply, Some(send_duel_response(OWN, true)));
        assert_eq!(bot.poll(t0 + Duration::from_secs(9)), None);
        assert_eq!(
            bot.poll(t0 + Duration::from_secs(10)),
            Some(duel_forfeit(OWN))
        );
        assert_eq!(bot.poll(t0 + Duration::from_secs(20)), None);
        assert_eq!(bot.report.challenges_accepted, 1);
        assert_eq!(bot.report.forfeits_sent, 1);
    }

    /// "Duel aborted" ends the duel: no forfeit afterwards. Without a
    /// forfeit time the bot never forfeits.
    #[test]
    fn abort_cancels_the_forfeit_and_none_never_forfeits() {
        let t0 = Instant::now();
        let cfg = SparbotConfig {
            forfeit_after: Some(Duration::from_secs(1)),
        };
        let mut bot = Sparbot::new(OWN, cfg);
        bot.on_incoming(
            Incoming::DuelChallenge {
                challenger_entity_id: 5,
            },
            t0,
        );
        bot.on_incoming(
            Incoming::Line {
                speaker: "SYSTEM".into(),
                channel: 9,
                text: TEXT_DUEL_ABORTED.into(),
            },
            t0,
        );
        assert_eq!(bot.poll(t0 + Duration::from_secs(5)), None);
        assert_eq!(bot.report.lines, vec![TEXT_DUEL_ABORTED.to_string()]);

        let mut never = Sparbot::new(
            OWN,
            SparbotConfig {
                forfeit_after: None,
            },
        );
        never.on_incoming(
            Incoming::DuelChallenge {
                challenger_entity_id: 5,
            },
            t0,
        );
        assert_eq!(never.poll(t0 + Duration::from_secs(3600)), None);
    }
}
