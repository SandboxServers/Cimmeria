//! The base half of the Black Market GM tools (BM-07): `.bm_seed`,
//! `.bm_expire` and `.bm_list`, so a tester can fill the auction house and
//! trigger a settlement without a second player or a 12-hour wait.
//!
//! The cell's `.`-console gate has already checked the GM's server-side
//! access level and parsed the arguments; [`BmGmActor`] carries the GM's ids
//! from the cell's own entity. Every answer is a feedback line to the GM.
//!
//! - `.bm_seed [count]` ([`seed`]) lists `count` system-seller auctions from
//!   the UAT set, after the same system seller check as the boot seed.
//! - `.bm_expire <auctionId>` ([`expire`]) makes one active auction due now
//!   and runs one expiry sweep pass at once, then says how it settled.
//! - `.bm_list` ([`list`]) shows the newest active auctions with their ids.
//!
//! Telemetry: `bm.gm_action` at INFO for a command that changed or read
//! something, `bm.gm_rejected` at WARN (ERROR for a database failure) with
//! a `reason` for one that did not; every row carries the GM's
//! `account_id` and `player_id`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::send::BmNet;
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::{session_identity, ConnectedClientState};
use crate::cell::messages::BmGmActor;

mod expire;
mod list;
mod seed;

pub use expire::gm_expire;
pub use list::gm_list;
pub use seed::gm_seed;

/// The most listings one `.bm_seed` may add (the cell checks it too).
pub const MAX_SEED_COUNT: u8 = 60;

/// What every GM handler needs: the GM, the database and the network.
#[derive(Clone, Copy)]
pub struct GmCtx<'a> {
    pub actor: BmGmActor,
    pub net: BmNet<'a>,
}

impl<'a> GmCtx<'a> {
    pub fn new(
        actor: BmGmActor,
        transport: &'a Arc<dyn Transport>,
        connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) -> Self {
        Self {
            actor,
            net: BmNet {
                transport,
                connected,
                entity_to_addr,
            },
        }
    }

    /// The GM's session identity, for the names on a log row. Call it inside
    /// the branch that logs: it takes the session locks.
    fn identity(&self) -> cimmeria_entity::cell_entity::PlayerIdentity {
        session_identity::identity_for_entity(
            self.net.connected,
            self.net.entity_to_addr,
            self.actor.entity_id,
        )
    }

    /// One feedback line to the GM.
    async fn tell(&self, text: &str) {
        send_gm_feedback_to_client(
            self.actor.entity_id,
            text,
            self.net.transport,
            self.net.connected,
            self.net.entity_to_addr,
        )
        .await;
    }

    /// Log a refused command (`bm.gm_rejected`) and tell the GM `line`.
    /// `reason = db_error` is ERROR, anything else WARN.
    async fn refuse(&self, command: &'static str, reason: &'static str, line: &str) {
        let a = self.actor;
        let id = self.identity();
        if reason == "db_error" {
            tracing::error!(
                event = "bm.gm_rejected",
                command,
                reason,
                entity_id = a.entity_id,
                entity_name = id.player_name,
                account_id = a.account_id,
                account_name = id.account_name,
                player_id = a.player_id,
                player_name = id.player_name,
                detail = line,
                "Black Market GM command failed"
            );
        } else {
            tracing::warn!(
                event = "bm.gm_rejected",
                command,
                reason,
                entity_id = a.entity_id,
                entity_name = id.player_name,
                account_id = a.account_id,
                account_name = id.account_name,
                player_id = a.player_id,
                player_name = id.player_name,
                "Black Market GM command refused: nothing changed"
            );
        }
        self.tell(line).await;
    }
}

/// The database, or a refusal to the GM when the server has none.
async fn pool_or_refuse<'p>(
    ctx: &GmCtx<'_>,
    command: &'static str,
    db_pool: &'p Option<Arc<PgPool>>,
) -> Option<&'p PgPool> {
    match db_pool.as_deref() {
        Some(pool) => Some(pool),
        None => {
            ctx.refuse(
                command,
                "no_db_pool",
                "The Black Market is unavailable: the server has no database.",
            )
            .await;
            None
        }
    }
}

/// The item's display name, or `item <id>` when it has none.
async fn item_name(pool: &PgPool, item_def_id: i32) -> String {
    sqlx::query_scalar::<_, String>("SELECT name FROM resources.items WHERE item_id = $1")
        .bind(item_def_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| format!("item {item_def_id}"))
}

#[cfg(test)]
mod tests;
