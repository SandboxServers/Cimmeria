//! `.bankdump [player]` (bank-vault BV-04): list a character's personal
//! vault, container 17, to the GM who asked. Read-only.
//!
//! The cell owns the console and the GM gate but has no DB pool, so it
//! forwards [`BankCellToBase::GmDump`] and the base answers here, the same
//! round trip `.searchitem` takes. A named character is matched exactly
//! against `sgw_player.player_name`, so an offline character's vault can be
//! read too (a support question is usually about someone who has logged
//! off).
//!
//! Every outcome is one `gm_action action=bankdump` event on the `bank`
//! target, joined to the GM by `account_id`, `player_id` and `entity_id`:
//! `result=ok` with the vault's `item_count` and `bank_slots`, or
//! `result=refused` with a stable `reason`. A refusal the GM caused (a name
//! that matches nobody) is INFO like the success; a
//! refusal the server caused (no pool, a failed query) is WARN.
//!
//! [`BankCellToBase::GmDump`]: cimmeria_wire::cell::messages::BankCellToBase::GmDump

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::INV_BANK;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::messages::BankSubject;
use sqlx::{PgPool, Row};

use super::gm_feedback::send_gm_feedback_to_client;
use super::ConnectedClientState;

/// At most this many item lines per dump. A personal vault holds at most
/// 100 slots (`bank_slots_sanity`), so only stray rows past the declared
/// size can reach it; the summary line still counts every row.
pub const MAX_ITEM_LINES: usize = 100;

/// The GM who asked, from the cell's session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DumpCaller {
    pub entity_id: u32,
    pub account_id: Option<u32>,
    pub player_id: Option<i32>,
}

/// One row of container 17.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultRow {
    pub slot_id: i32,
    pub item_id: i32,
    pub type_id: i32,
    pub stack_size: i32,
    /// `resources.items.name`, `None` for a type the resources do not know.
    pub name: Option<String>,
}

/// A character's vault as the dump reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultDump {
    pub player_id: i32,
    pub player_name: String,
    pub bank_slots: i16,
    /// Ordered by slot.
    pub rows: Vec<VaultRow>,
}

/// Why no vault was read. [`DumpRefusal::reason`] is the `reason=` label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpRefusal {
    /// No character has that id or name.
    TargetNotFound,
    /// The base has no database pool.
    DbUnavailable,
    /// A query failed; the error text is in the log.
    QueryFailed(String),
}

impl DumpRefusal {
    /// The stable `reason=` string.
    pub fn reason(&self) -> &'static str {
        match self {
            DumpRefusal::TargetNotFound => "target_not_found",
            DumpRefusal::DbUnavailable => "db_unavailable",
            DumpRefusal::QueryFailed(_) => "query_failed",
        }
    }
}

/// Resolve `subject` to one character and read its container-17 rows.
pub async fn load_vault_dump(
    pool: &PgPool,
    subject: &BankSubject,
) -> Result<VaultDump, DumpRefusal> {
    let failed = |e: sqlx::Error| DumpRefusal::QueryFailed(e.to_string());
    // `player_name` is UNIQUE (`sgw_player_player_name_key`), so a name
    // names at most one character.
    let player = match subject {
        BankSubject::Player(player_id) => sqlx::query(
            "SELECT player_id, player_name, bank_slots FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_optional(pool)
        .await
        .map_err(failed)?,
        BankSubject::Name(name) => sqlx::query(
            "SELECT player_id, player_name, bank_slots FROM sgw_player WHERE player_name = $1",
        )
        .bind(name)
        .fetch_optional(pool)
        .await
        .map_err(failed)?,
    };
    let Some(player) = player else {
        return Err(DumpRefusal::TargetNotFound);
    };
    let player_id: i32 = player.try_get("player_id").map_err(failed)?;

    let rows = sqlx::query(
        "SELECT inv.slot_id, inv.item_id, inv.type_id, inv.stack_size, it.name \
           FROM sgw_inventory inv \
           LEFT JOIN resources.items it ON it.item_id = inv.type_id \
          WHERE inv.character_id = $1 AND inv.container_id = $2 \
          ORDER BY inv.slot_id, inv.item_id",
    )
    .bind(player_id)
    .bind(INV_BANK)
    .fetch_all(pool)
    .await
    .map_err(failed)?
    .iter()
    .map(|r| {
        Ok(VaultRow {
            slot_id: r.try_get("slot_id")?,
            item_id: r.try_get("item_id")?,
            type_id: r.try_get("type_id")?,
            stack_size: r.try_get("stack_size")?,
            name: r.try_get("name")?,
        })
    })
    .collect::<Result<Vec<_>, sqlx::Error>>()
    .map_err(failed)?;

    Ok(VaultDump {
        player_id,
        player_name: player.try_get("player_name").map_err(failed)?,
        bank_slots: player.try_get("bank_slots").map_err(failed)?,
        rows,
    })
}

/// The feedback lines for a dump: one summary, then one line per item.
pub fn dump_lines(dump: &VaultDump) -> Vec<String> {
    let who = format!("{} (player {})", dump.player_name, dump.player_id);
    if dump.rows.is_empty() {
        return vec![format!(
            "bankdump: {who}: the vault is empty ({} slots)",
            dump.bank_slots
        )];
    }
    let mut lines = vec![format!(
        "bankdump: {who}: {} item(s) in the vault ({} slots)",
        dump.rows.len(),
        dump.bank_slots
    )];
    for row in dump.rows.iter().take(MAX_ITEM_LINES) {
        let name = row.name.as_deref().unwrap_or("unknown item");
        // A row past the declared size is invisible in the client's vault
        // window; say so, because that is what a GM is usually hunting.
        let beyond = if row.slot_id >= i32::from(dump.bank_slots) {
            " (beyond bank_slots)"
        } else {
            ""
        };
        lines.push(format!(
            "    slot {}: {name} (type {}) x{} [item {}]{beyond}",
            row.slot_id, row.type_id, row.stack_size, row.item_id
        ));
    }
    if dump.rows.len() > MAX_ITEM_LINES {
        lines.push(format!(
            "    ... and {} more",
            dump.rows.len() - MAX_ITEM_LINES
        ));
    }
    lines
}

/// The subject's name for a refusal line.
fn subject_label(subject: &BankSubject) -> String {
    match subject {
        BankSubject::Player(id) => format!("player {id}"),
        BankSubject::Name(name) => name.clone(),
    }
}

/// Run one dump and log its `gm_action`. Returns the lines for the GM.
pub async fn run_gm_dump(
    caller: DumpCaller,
    subject: &BankSubject,
    pool: Option<&PgPool>,
) -> Vec<String> {
    let result = match pool {
        Some(pool) => load_vault_dump(pool, subject).await,
        None => Err(DumpRefusal::DbUnavailable),
    };
    let target_name = match subject {
        BankSubject::Name(name) => Some(name.as_str()),
        BankSubject::Player(_) => None,
    };
    match result {
        Ok(dump) => {
            let player_label = known_names::player_name(caller.player_id);
            tracing::info!(
                target: "bank",
                event = "gm_action",
                action = "bankdump",
                result = "ok",
                account_id = caller.account_id,
                account_name = known_names::account_name(caller.account_id),
                player_id = caller.player_id,
                player_name = player_label,
                entity_id = caller.entity_id,
                entity_name = player_label,
                target_player_id = dump.player_id,
                target_player_name = known_names::player_name(dump.player_id),
                target_name,
                item_count = dump.rows.len(),
                bank_slots = dump.bank_slots,
                "gm_action: bankdump listed a vault"
            );
            dump_lines(&dump)
        }
        Err(refusal) => {
            let target_player_id = match subject {
                BankSubject::Player(id) => Some(*id),
                BankSubject::Name(_) => None,
            };
            let reason = refusal.reason();
            match &refusal {
                DumpRefusal::TargetNotFound => {
                    let player_label = known_names::player_name(caller.player_id);
                    tracing::info!(
                        target: "bank",
                        event = "gm_action",
                        action = "bankdump",
                        result = "refused",
                        reason,
                        account_id = caller.account_id,
                        account_name = known_names::account_name(caller.account_id),
                        player_id = caller.player_id,
                        player_name = player_label,
                        entity_id = caller.entity_id,
                        entity_name = player_label,
                        target_player_id,
                        target_player_name = known_names::player_name(target_player_id),
                        target_name,
                        "gm_action: bankdump refused"
                    );
                }
                DumpRefusal::DbUnavailable | DumpRefusal::QueryFailed(_) => {
                    let error = match &refusal {
                        DumpRefusal::QueryFailed(e) => e.as_str(),
                        _ => "no database pool",
                    };
                    let player_label = known_names::player_name(caller.player_id);
                    tracing::warn!(
                        target: "bank",
                        event = "gm_action",
                        action = "bankdump",
                        result = "refused",
                        reason,
                        account_id = caller.account_id,
                        account_name = known_names::account_name(caller.account_id),
                        player_id = caller.player_id,
                        player_name = player_label,
                        entity_id = caller.entity_id,
                        entity_name = player_label,
                        target_player_id,
                        target_player_name = known_names::player_name(target_player_id),
                        target_name,
                        error,
                        "gm_action: bankdump could not read the vault"
                    )
                }
            }
            let who = subject_label(subject);
            vec![match refusal {
                DumpRefusal::TargetNotFound => {
                    format!("bankdump: no character is named {who} (names are case-sensitive)")
                }
                DumpRefusal::DbUnavailable => "bankdump: no live DB connection".to_string(),
                DumpRefusal::QueryFailed(_) => {
                    format!("bankdump: could not read the vault of {who} (see the server log)")
                }
            }]
        }
    }
}

/// `BankCellToBase::GmDump`: run the dump and send every line to the GM.
#[tracing::instrument(
    name = "bank.gm_dump",
    level = "info",
    skip_all,
    fields(entity_id = caller.entity_id)
)]
pub async fn handle_gm_dump(
    caller: DumpCaller,
    subject: BankSubject,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let lines = run_gm_dump(caller, &subject, db_pool.as_deref()).await;
    for line in lines {
        send_gm_feedback_to_client(
            caller.entity_id,
            &line,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}

#[cfg(test)]
mod tests;
