//! Session kind (TG-NET-16, owner decision D-TG13): whether a login is a lab
//! run or a real player, taken from the account's `test_account` flag alone.
//!
//! The game client has no way to declare its kind on the wire, so the server
//! never asks it. Phase 1 reads `account.test_account` with the credentials,
//! the flag rides the SID record and the Phase 2 ticket (`PendingLogin`), and
//! Phase 3 stores [`session_kind`] on the session as `session_kind`.
//! Telemetry stamps it as `cimmeria.session_kind`, the same key the ingest
//! puts on client and launcher rows.

/// `session_kind` of an account flagged `test_account` (the lab accounts).
pub const SESSION_KIND_LAB: &str = "lab";
/// `session_kind` of every other account.
pub const SESSION_KIND_PLAYER: &str = "player";

/// The `session_kind` for an account with this `test_account` flag.
pub fn session_kind(test_account: bool) -> &'static str {
    if test_account {
        SESSION_KIND_LAB
    } else {
        SESSION_KIND_PLAYER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::credentials::{validate_credentials, ClientCredential};
    use crate::test_support::require_db_or_skip;
    use sha1::{Digest, Sha1};
    use sqlx::PgPool;

    /// Sentinels in the crate's `0x7000_1B00` credential window, past
    /// `credentials.rs` (`+1`..`+7`) and clear of `tls_smoke.rs` (`+0x20`)
    /// and `audit.rs` (`+0x40`).
    const LAB_SENTINEL: i32 = 0x7000_1B08;
    const PLAYER_SENTINEL: i32 = 0x7000_1B09;

    fn upper_sha1(input: &str) -> String {
        Sha1::digest(input.as_bytes())
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect()
    }

    async fn cleanup(pool: &PgPool, account_id: i32) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account(pool: &PgPool, account_id: i32, name: &str, test_account: bool) {
        cleanup(pool, account_id).await;
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password, password_algo, accesslevel, enabled, test_account) \
             VALUES ($1, $2, $3, 1, 0, true, $4)",
        )
        .bind(account_id)
        .bind(name)
        .bind(upper_sha1(name))
        .bind(test_account)
        .execute(pool)
        .await
        .expect("insert sentinel account");
    }

    async fn kind_at_login(pool: &PgPool, name: &str) -> &'static str {
        let hash = upper_sha1(name);
        let acct = validate_credentials(pool, name, ClientCredential::LegacySha1Hex(&hash))
            .await
            .unwrap_or_else(|_| panic!("sentinel {name} must log in"));
        session_kind(acct.test_account)
    }

    #[test]
    fn session_kind_maps_flag_to_lab_or_player() {
        assert_eq!(session_kind(true), "lab");
        assert_eq!(session_kind(false), "player");
    }

    /// A flagged account logs in as `lab`, an unflagged one as `player`, and
    /// the seed flags exactly the lab accounts. Reverting the `test_account`
    /// column read in `validate_credentials` (or the seed flags) fails it.
    #[tokio::test]
    async fn live_db_lab_account_is_test_account() {
        let pool = require_db_or_skip!();

        // Seed: the flagged seeded accounts are the five lab accounts
        // (ids 10-14 in `db/sgw/Accounts/Seed/account.sql`). Sentinel rows
        // from other tests sit far above this range.
        let flagged: Vec<i32> = sqlx::query_scalar(
            "SELECT account_id FROM account \
             WHERE test_account AND account_id < $1 ORDER BY account_id",
        )
        .bind(0x7000_0000_i32)
        .fetch_all(&pool)
        .await
        .expect("read seeded test accounts");
        assert_eq!(
            flagged,
            vec![10, 11, 12, 13, 14],
            "the seed must flag exactly the lab accounts as test accounts"
        );

        let lab_name = format!("tg16-lab-{LAB_SENTINEL}");
        let player_name = format!("tg16-player-{PLAYER_SENTINEL}");
        insert_account(&pool, LAB_SENTINEL, &lab_name, true).await;
        insert_account(&pool, PLAYER_SENTINEL, &player_name, false).await;

        let lab_kind = kind_at_login(&pool, &lab_name).await;
        let player_kind = kind_at_login(&pool, &player_name).await;

        cleanup(&pool, LAB_SENTINEL).await;
        cleanup(&pool, PLAYER_SENTINEL).await;

        assert_eq!(lab_kind, "lab", "a test account must resolve to lab");
        assert_eq!(
            player_kind, "player",
            "an unflagged account must resolve to player"
        );
    }
}
