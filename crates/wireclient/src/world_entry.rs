//! Character select and world entry on a connected [`GameSession`], in the
//! order the real client sends it.
//!
//! Lifted from the NA37 integration-test driver (`tests/it/support`) so the
//! `sparbot` binary and the tests share one sequence. The tests keep their
//! panicking wrappers; this returns [`Error::WorldEntry`] instead.

use std::time::Duration;

use bytes::Bytes;

use crate::bundle::decode_bundle;
use crate::error::{Error, Result};
use crate::session::GameSession;

/// `tickSync` (0x0D): the server's 10 Hz unreliable heartbeat.
const MSG_TICK_SYNC: u8 = 0x0D;
/// `createBasePlayer` (0x05): carries the player's own entity id.
const MSG_CREATE_BASE_PLAYER: u8 = 0x05;
/// `onCharacterList` (0x82): the account's reply once a character exists.
const MSG_ON_CHARACTER_LIST: u8 = 0x82;
/// `onCharacterCreateFailed` (0x83): the account's reply to a refusal.
const MSG_ON_CHARACTER_CREATE_FAILED: u8 = 0x83;

impl GameSession {
    /// `AUTHENTICATE` + `ENABLE_ENTITIES`, then the character-list bundle.
    /// `recv_timeout` bounds the wait for that bundle.
    pub async fn open_character_list(&mut self, recv_timeout: Duration) -> Result<()> {
        let mut post_handshake = GameSession::authenticate();
        post_handshake.extend_from_slice(&GameSession::enable_entities());
        self.send_bundle(&post_handshake, true).await?;
        self.expect_bundles(1, recv_timeout, "the character list")
            .await?;
        Ok(())
    }

    /// `createCharacter` (0xC3), reliable. Returns `Ok` on `onCharacterList`
    /// (0x82) and [`Error::CharacterCreateFailed`] on `onCharacterCreateFailed`
    /// (0x83), so a refusal cannot read as success. The wait is one bundle at
    /// a time, bounded by `recv_timeout` overall.
    pub async fn create_character_on_wire(
        &mut self,
        name: &str,
        char_def_id: i32,
        visual_choices: &[(i32, i32)],
        recv_timeout: Duration,
    ) -> Result<()> {
        // The builder is `GameSession::create_character` (calls.rs); this
        // method has a different name because Rust allows one inherent fn per
        // name.
        let body = GameSession::create_character(name, "", char_def_id, visual_choices, 0);
        self.send_bundle(&body, true).await?;

        let start = tokio::time::Instant::now();
        loop {
            let remaining = recv_timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                break;
            }
            for bundle in self.recv_meaningful_bundles(1, remaining).await {
                let outcome = create_outcome(&bundle);
                self.entry_bundles.push(bundle);
                match outcome {
                    Some(Ok(())) => return Ok(()),
                    Some(Err(code)) => return Err(Error::CharacterCreateFailed(code)),
                    None => {}
                }
            }
        }
        Err(Error::WorldEntry(format!(
            "no onCharacterList after createCharacter within {recv_timeout:?}"
        )))
    }

    /// `playCharacter` (0xC4) through `onClientReady` (0xD8). Sets
    /// [`GameSession::player_entity_id`] from the `createBasePlayer` reply.
    /// `recv_timeout` bounds each step's wait.
    pub async fn play(&mut self, player_id: i32, recv_timeout: Duration) -> Result<()> {
        self.send_bundle(&GameSession::play_character(player_id), true)
            .await?;
        self.expect_bundles(1, recv_timeout, "RESET_ENTITIES")
            .await?;

        self.send_bundle(&GameSession::enable_entities(), true)
            .await?;
        let create_player = self
            .expect_bundles(1, recv_timeout, "CREATE_BASE_PLAYER")
            .await?;
        let own_id = decode_bundle(&create_player[0])
            .into_iter()
            .find(|m| m.msg_id == MSG_CREATE_BASE_PLAYER)
            .and_then(|m| m.entity_id)
            .ok_or_else(|| {
                Error::WorldEntry("CREATE_BASE_PLAYER bundle carried no entity id".into())
            })?;
        self.player_entity_id = Some(own_id);

        self.send_bundle(&GameSession::map_loaded(own_id), true)
            .await?;
        self.expect_bundles(
            2,
            recv_timeout,
            "VIEWPORT+CELL+FORCED_POSITION and the entity data",
        )
        .await?;

        self.send_bundle(&GameSession::on_client_ready(), true)
            .await?;
        Ok(())
    }

    /// Drive character select and world entry: [`GameSession::open_character_list`]
    /// then [`GameSession::play`]. `recv_timeout` bounds each step's wait.
    pub async fn enter_world(&mut self, player_id: i32, recv_timeout: Duration) -> Result<()> {
        self.open_character_list(recv_timeout).await?;
        self.play(player_id, recv_timeout).await
    }

    /// Wait for `count` bundles that are not just `tickSync`, or fail
    /// naming `what`. Under injected latency a periodic tickSync can land
    /// ahead of the reply a step is waiting for, so heartbeats are skipped
    /// rather than counted.
    pub async fn recv_meaningful_bundles(&self, count: usize, timeout: Duration) -> Vec<Bytes> {
        let start = tokio::time::Instant::now();
        let mut out = Vec::with_capacity(count);
        while out.len() < count {
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                break;
            }
            let bundles = self.recv_bundles(1, timeout - elapsed).await;
            if bundles.is_empty() {
                break;
            }
            for b in bundles {
                let msgs = decode_bundle(&b);
                let only_tick_sync =
                    !msgs.is_empty() && msgs.iter().all(|m| m.msg_id == MSG_TICK_SYNC);
                if !only_tick_sync {
                    out.push(b);
                    if out.len() >= count {
                        break;
                    }
                }
            }
        }
        out
    }

    async fn expect_bundles(
        &mut self,
        count: usize,
        timeout: Duration,
        what: &str,
    ) -> Result<Vec<Bytes>> {
        let got = self.recv_meaningful_bundles(count, timeout).await;
        if got.len() < count {
            return Err(Error::WorldEntry(format!(
                "expected {count} bundle(s) for {what}, got {} within {timeout:?}",
                got.len()
            )));
        }
        self.entry_bundles.extend(got.iter().cloned());
        Ok(got)
    }
}

/// The createCharacter verdict a bundle carries, if any: `Some(Ok(()))` for
/// `onCharacterList`, `Some(Err(code))` for `onCharacterCreateFailed`. The
/// code is the first `i32` of that message's args (the server writes
/// `[entityID][errorCode]`, `wire/mercury/protocol/character.rs`). A refusal
/// too short to carry one reads as code 0 rather than as success.
fn create_outcome(bundle: &Bytes) -> Option<std::result::Result<(), i32>> {
    decode_bundle(bundle)
        .into_iter()
        .find_map(|m| match m.msg_id {
            MSG_ON_CHARACTER_LIST => Some(Ok(())),
            MSG_ON_CHARACTER_CREATE_FAILED => {
                let code = m
                    .args()
                    .and_then(|a| a.get(..4))
                    .map_or(0, |b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]));
                Some(Err(code))
            }
            _ => None,
        })
}
