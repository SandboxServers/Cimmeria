use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use crate::cell::mail;
use crate::cell::messages::MailOp;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::super::helpers::send_to_witness;
use super::super::super::ConnectedClientState;

/// Handle a mail request from CellService by querying the DB and sending results to the client.
pub async fn handle_mail_request(
    entity_id: u32,
    player_id: i32,
    op: MailOp,
    socket: &Arc<UdpSocket>,
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

    // Use poison-tolerant lock acquires so a panic in another thread doesn't
    // cascade into a panic here.
    let player_name = {
        let addr_guard = match entity_to_addr.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let addr = match addr_guard.get(&entity_id).copied() {
            Some(a) => a,
            None => {
                drop(addr_guard);
                tracing::warn!(entity_id, "Mail: no addr for player name lookup");
                return;
            }
        };
        drop(addr_guard);
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        clients
            .get(&addr)
            .and_then(|c| c.player_name.clone())
            .unwrap_or_default()
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
                            mail_id = r.mail_id, db_cash = r.cash,
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
            send_to_witness(
                socket,
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

            match sqlx::query_as::<_, BodyRow>(
                "SELECT message FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(Some(row)) => {
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

                    let args = mail::serialize_on_mail_read(mail_id, &row.message, &player_name);
                    send_to_witness(
                        socket,
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
                _ => {
                    tracing::warn!(entity_id, mail_id, "Mail body not found");
                }
            }
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
                    tracing::warn!(entity_id, player_id, mail_id, "Mail: Delete affected 0 rows");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(entity_id, player_id, mail_id, "Mail: Delete failed: {e}");
                    return;
                }
            }

            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket,
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
            send_to_witness(
                socket,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for mail tests. Distinct from prior live-DB sentinels:
    /// grant_cash (+0x100), move_inventory (+0x200), grant_item (+0x300),
    /// missions (+0x400).
    const TEST_BASE: i32 = 0x7000_0500;

    async fn cleanup(pool: &PgPool, account_id: i32) {
        // sgw_gate_mail has no FK to account, so delete its rows by character_id
        // first. The account delete cascades sgw_player rows.
        let _ = sqlx::query(
            "DELETE FROM sgw_gate_mail WHERE character_id IN \
             (SELECT player_id FROM sgw_player WHERE account_id = $1)",
        )
        .bind(account_id).execute(pool).await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id).execute(pool).await;
    }

    async fn insert_account_with_two_chars(
        pool: &PgPool,
        account_id: i32,
        char_a: i32,
        char_b: i32,
    ) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id).bind(format!("mail-test-{account_id}"))
        .execute(pool).await.expect("insert account");

        for player_id in [char_a, char_b] {
            sqlx::query(
                "INSERT INTO sgw_player (\
                    account_id, player_id, level, alignment, archetype, gender, \
                    player_name, extra_name, world_location, bodyset, \
                    pos_x, pos_y, pos_z, skin_color_id, naquadah\
                 ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                           0.0, 0.0, 0.0, 0, 0)",
            )
            .bind(account_id).bind(player_id).bind(format!("test-{player_id}"))
            .execute(pool).await.expect("insert player");
        }
    }

    /// Insert a mail for `character_id`. Returns the auto-generated mail_id.
    async fn insert_mail(pool: &PgPool, character_id: i32, subject: &str) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_gate_mail \
                (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
             VALUES ($1, NULL, $2, 'body', 0, 0, 0, 0) RETURNING mail_id",
        )
        .bind(character_id).bind(subject)
        .fetch_one(pool).await.expect("insert mail")
    }

    fn make_state(entity_id: u32) -> (
        Arc<UdpSocket>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
        std_sock.set_nonblocking(true).unwrap();
        let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(entity_id, fake_addr);
            m
        }));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        (socket, entity_to_addr, connected)
    }

    /// Regression guard: Delete must scope by character_id. Character A
    /// requesting Delete on a mail_id owned by character B (same account)
    /// must affect 0 rows — B's mail stays. The pre-fix bug had the
    /// WHERE clause matching on account-wide criteria, so any character
    /// could delete any sibling's mail.
    #[tokio::test]
    async fn delete_only_affects_target_character_not_account_siblings() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let char_a = TEST_BASE + 1;
        let char_b = TEST_BASE + 2;
        cleanup(&pool, account_id).await;
        insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

        let mail_a = insert_mail(&pool, char_a, "for A").await;
        let mail_b = insert_mail(&pool, char_b, "for B").await;

        let (socket, e2a, conn) = make_state(0x7000_0501);
        let db_pool = Some(Arc::new(pool.clone()));

        // Character A tries to delete character B's mail. Must NOT delete it.
        handle_mail_request(
            0x7000_0501, char_a, MailOp::Delete { mail_id: mail_b },
            &socket, &conn, &e2a, &db_pool,
        ).await;

        let b_still_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)",
        )
        .bind(mail_b).fetch_one(&pool).await.unwrap();
        assert!(b_still_exists,
            "character B's mail must survive when character A issues Delete on it");

        // Sanity: A's own mail is still there too.
        let a_still_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)",
        )
        .bind(mail_a).fetch_one(&pool).await.unwrap();
        assert!(a_still_exists, "A's own mail unaffected");

        // Now A deletes its own mail — must succeed.
        handle_mail_request(
            0x7000_0501, char_a, MailOp::Delete { mail_id: mail_a },
            &socket, &conn, &e2a, &db_pool,
        ).await;
        let a_now_gone: bool = sqlx::query_scalar(
            "SELECT NOT EXISTS(SELECT 1 FROM sgw_gate_mail WHERE mail_id = $1)",
        )
        .bind(mail_a).fetch_one(&pool).await.unwrap();
        assert!(a_now_gone, "A's own mail deletes when A is the requester");

        cleanup(&pool, account_id).await;
    }

    /// Regression guard: Archive must scope by character_id. Same setup as
    /// the Delete test, but verifies the flags column on B's mail stays at 0
    /// when A issues Archive on it.
    #[tokio::test]
    async fn archive_only_affects_target_character_not_account_siblings() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let char_a = TEST_BASE + 101;
        let char_b = TEST_BASE + 102;
        cleanup(&pool, account_id).await;
        insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

        let mail_a = insert_mail(&pool, char_a, "for A").await;
        let mail_b = insert_mail(&pool, char_b, "for B").await;

        let (socket, e2a, conn) = make_state(0x7000_0511);
        let db_pool = Some(Arc::new(pool.clone()));

        // A tries to archive B's mail. Must NOT flip the flags bit.
        handle_mail_request(
            0x7000_0511, char_a, MailOp::Archive { mail_id: mail_b },
            &socket, &conn, &e2a, &db_pool,
        ).await;

        let b_flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_b).fetch_one(&pool).await.unwrap();
        assert_eq!(b_flags, 0,
            "character B's flags must stay 0 when character A issues Archive");

        // A archives its own mail — flags must gain bit 0.
        handle_mail_request(
            0x7000_0511, char_a, MailOp::Archive { mail_id: mail_a },
            &socket, &conn, &e2a, &db_pool,
        ).await;
        let a_flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_a).fetch_one(&pool).await.unwrap();
        assert_eq!(a_flags & 1, 1, "Archive sets bit 0 on the target's flags column");

        cleanup(&pool, account_id).await;
    }

    /// Archive is OR-with-1, so calling it twice on the same mail must leave
    /// flags == 1 (not toggle / increment). Pins the bitwise-OR semantics so
    /// a future regression to `flags = 1` (assignment instead of OR) would
    /// be caught when other flag bits are in play.
    #[tokio::test]
    async fn archive_is_idempotent_via_bitwise_or() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 200;
        let char_a = TEST_BASE + 201;
        let char_b = TEST_BASE + 202; // unused but make_account expects two
        cleanup(&pool, account_id).await;
        insert_account_with_two_chars(&pool, account_id, char_a, char_b).await;

        // Seed mail with flags = 4 (some unrelated bit set).
        let mail_id: i32 = sqlx::query_scalar(
            "INSERT INTO sgw_gate_mail \
                (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
             VALUES ($1, NULL, 'flags-test', 'body', 0, 0, 0, 4) RETURNING mail_id",
        )
        .bind(char_a).fetch_one(&pool).await.unwrap();

        let (socket, e2a, conn) = make_state(0x7000_0521);
        let db_pool = Some(Arc::new(pool.clone()));

        // Archive twice. Both calls must leave flags == 4 | 1 == 5
        // (NOT 1 — that would mean assignment overwrote the unrelated bit).
        for _ in 0..2 {
            handle_mail_request(
                0x7000_0521, char_a, MailOp::Archive { mail_id },
                &socket, &conn, &e2a, &db_pool,
            ).await;
        }

        let flags: i32 = sqlx::query_scalar("SELECT flags FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id).fetch_one(&pool).await.unwrap();
        assert_eq!(
            flags, 5,
            "Archive must OR bit 0 in (flags |= 1) — pre-existing bit 2 must survive. \
             A flags=1 here means a regression to `flags = 1` (assignment)",
        );

        cleanup(&pool, account_id).await;
    }
}