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

impl GameSession {
    /// Drive character select and world entry: `AUTHENTICATE` +
    /// `ENABLE_ENTITIES` (character list), `playCharacter`,
    /// `ENABLE_ENTITIES` (create player), `mapLoaded`, `onClientReady`.
    /// Sets [`GameSession::player_entity_id`] from the `createBasePlayer`
    /// reply. `recv_timeout` bounds each step's wait.
    pub async fn enter_world(&mut self, player_id: i32, recv_timeout: Duration) -> Result<()> {
        let mut post_handshake = GameSession::authenticate();
        post_handshake.extend_from_slice(&GameSession::enable_entities());
        self.send_bundle(&post_handshake, true).await?;
        self.expect_bundles(1, recv_timeout, "the character list")
            .await?;

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
        &self,
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
        Ok(got)
    }
}
