use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::helpers::send_to_witness_reliable;
use super::super::super::ConnectedClientState;
use crate::cell::mail;
use crate::cell::messages::MailOp;
use crate::mercury::{build_entity_method_packet, method_idx};

#[cfg(test)]
mod tests;

/// Handle a mail request from CellService by querying the DB and sending results to the client.
pub async fn handle_mail_request(
    entity_id: u32,
    player_id: i32,
    op: MailOp,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(entity_id, player_id, "Mail request: no DB pool available");
            return;
        }
    };

    // Resolve the player_name only when a request arm actually needs it
    // (RequestBody — the read packet carries the recipient's display
    // name). Headers / Delete / Archive don't read the name, so doing
    // the two-mutex lookup unconditionally would impose avoidable lock
    // contention on the hot list-fetch path.
    let lookup_player_name = || -> Option<String> {
        let addr_guard = match entity_to_addr.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let addr = addr_guard.get(&entity_id).copied();
        drop(addr_guard);
        let addr = addr?;
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        Some(
            clients
                .get(&addr)
                .and_then(|c| c.player_name.clone())
                .unwrap_or_default(),
        )
    };

    match op {
        MailOp::RequestHeaders { b_archive } => {
            tracing::debug!(entity_id, player_id, b_archive, "Mail: querying headers");

            #[derive(sqlx::FromRow)]
            struct MailRow {
                mail_id: i32,
                sender_name: String,
                sender_id: Option<i32>,
                subject: String,
                cash: i64,
                sent_time: i32,
                read_time: i32,
                flags: i32,
            }

            let rows = match sqlx::query_as::<_, MailRow>(
                "SELECT mail_id, sender_name, sender_id, subject, cash, sent_time, read_time, flags \
                 FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id DESC",
            )
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!(entity_id, player_id, "Mail: header query failed: {e}");
                    return;
                }
            };

            let headers: Vec<mail::MailHeader> = rows
                .iter()
                .map(|r| {
                    let cash = i32::try_from(r.cash).unwrap_or_else(|_| {
                        tracing::warn!(
                            mail_id = r.mail_id,
                            db_cash = r.cash,
                            "Mail header cash truncated to i32 range"
                        );
                        r.cash.clamp(i32::MIN as i64, i32::MAX as i64) as i32
                    });
                    mail::MailHeader {
                        id: r.mail_id,
                        from_text: r.sender_name.clone(),
                        from_id: r.sender_id.unwrap_or(0),
                        subject_text: r.subject.clone(),
                        cash,
                        sent_time: r.sent_time as f32,
                        read_time: r.read_time as f32,
                        flags: r.flags,
                    }
                })
                .collect();

            tracing::debug!(
                entity_id,
                count = headers.len(),
                "Mail: sending headers to client"
            );

            let args = mail::serialize_on_mail_header_info(b_archive, &headers);
            send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_MAIL_HEADER_INFO,
                        &args,
                    )
                },
            )
            .await;
        }

        MailOp::RequestBody { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: querying body");

            #[derive(sqlx::FromRow)]
            struct BodyRow {
                message: String,
            }

            let row = match sqlx::query_as::<_, BodyRow>(
                "SELECT message FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(Some(row)) => row,
                // Distinguish "row missing for this character" (legitimate
                // permission boundary or stale client request) from "DB
                // error" (operator-actionable). Folding both into the
                // same warn string would hide connection failures /
                // schema mismatches behind a benign-looking message.
                Ok(None) => {
                    tracing::warn!(
                        entity_id,
                        mail_id,
                        player_id,
                        "Mail body not found for this character_id"
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!(
                        entity_id, mail_id, player_id, error = %e,
                        "Mail body query failed"
                    );
                    return;
                }
            };

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i32;
            if let Err(e) = sqlx::query(
                "UPDATE sgw_gate_mail SET read_time = $1 WHERE mail_id = $2 AND read_time = 0",
            )
            .bind(now)
            .bind(mail_id)
            .execute(pool.as_ref())
            .await
            {
                tracing::warn!(entity_id, mail_id, "Mail: read_time UPDATE failed: {e}");
            }

            let player_name = match lookup_player_name() {
                Some(n) => n,
                None => {
                    tracing::warn!(entity_id, "Mail: no addr for player name lookup");
                    return;
                }
            };
            let args = mail::serialize_on_mail_read(mail_id, &row.message, &player_name);
            send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_MAIL_READ,
                        &args,
                    )
                },
            )
            .await;
        }

        MailOp::Delete { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: deleting");
            match sqlx::query("DELETE FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2")
                .bind(mail_id)
                .bind(player_id)
                .execute(pool.as_ref())
                .await
            {
                Ok(r) if r.rows_affected() == 0 => {
                    tracing::warn!(
                        entity_id,
                        player_id,
                        mail_id,
                        "Mail: Delete affected 0 rows"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(entity_id, player_id, mail_id, "Mail: Delete failed: {e}");
                    return;
                }
            }

            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_MAIL_HEADER_REMOVE,
                        &args,
                    )
                },
            )
            .await;
        }

        MailOp::Archive { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: archiving");
            match sqlx::query(
                "UPDATE sgw_gate_mail SET flags = flags | 1 WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await
            {
                Ok(r) if r.rows_affected() == 0 => {
                    tracing::warn!(entity_id, player_id, mail_id, "Mail: Archive affected 0 rows");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(entity_id, player_id, mail_id, "Mail: Archive failed: {e}");
                    return;
                }
            }

            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_MAIL_HEADER_REMOVE,
                        &args,
                    )
                },
            )
            .await;
        }
    }
}
