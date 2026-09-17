//! BaseApp-side executor for `.`-console authoring SQL.
//!
//! The `.`-console spawn/patrol authoring commands (`savespawn`, `path_add`,
//! …) run on the CellService, which has no DB pool. They hand a
//! **server-generated** `INSERT`/`UPDATE`/`DELETE` to the base via
//! [`CellToBaseMsg::ExecuteAuthoringSql`](crate::cell::messages::CellToBaseMsg::ExecuteAuthoringSql);
//! this handler runs it against the live pool and reports the row count back to
//! the GM on the feedback channel.
//!
//! The write is intentionally **transient**: it lets the developer see the
//! change hold across reconnects within the current deploy, but the next deploy
//! rebuilds the DB from the `db/resources/` seeds and wipes it. The durable
//! artifact is the seed SQL the cell records to the per-session authoring log
//! and (later) Discord — see `crate::cell::console::seed`.
//!
//! **Trust model:** the `.`-channel is GM-gated server-side (the cell only
//! forwards a command from an `access_level >= GameMaster` caller), and `sql`
//! is server-generated — numeric values come from cell-parsed `i32`/`f32` and
//! strings are escaped through `crate::cell::console::seed::sql_str`, so there
//! is no raw client text concatenated into the statement. This mirrors the
//! legacy `Atrea.dbQuery` authoring path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::{AssertSqlSafe, PgPool};

use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::ConnectedClientState;

/// Max search hits reported per `.search*` command — keeps a broad query from
/// flooding the feedback channel.
const SEARCH_LIMIT: i64 = 25;

/// Escape `%`, `_`, and `\` in a `.search*` query so they match themselves
/// literally under `ILIKE ... ESCAPE '\'` instead of acting as a SQL
/// wildcard/escape-introducer. Legacy `Resource.py::searchItem` (and its
/// mission/template siblings) did a plain Python substring search — a GM
/// typing e.g. `50%` or `file_name` expects those characters matched
/// literally, not as "any sequence" / "any single character".
fn escape_ilike_pattern(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        if matches!(c, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// Run the read-only resource-name search for `kind` (`0` items, `1`
/// missions, `2` entity_templates) against `query`, matching `query`
/// case-insensitively and **literally** (via [`escape_ilike_pattern`]).
///
/// Fetches one row past [`SEARCH_LIMIT`] to distinguish "exactly the limit"
/// from "more hits exist" without a second `COUNT(*)` round-trip; the extra
/// row is trimmed before returning. Returns `(label, hits, truncated)` —
/// `label` is `""` for an unrecognized `kind` (mirrors the prior silent
/// `_ => return` in the caller).
async fn run_console_search(
    pool: &PgPool,
    kind: u8,
    query: &str,
) -> Result<(&'static str, Vec<(i32, String)>, bool), sqlx::Error> {
    let (label, sql) = match kind {
        0 => (
            "searchitem",
            "SELECT item_id AS id, name FROM resources.items \
             WHERE name ILIKE $1 ESCAPE '\\' ORDER BY item_id LIMIT $2",
        ),
        1 => (
            "searchmission",
            "SELECT mission_id AS id, mission_defn AS name FROM resources.missions \
             WHERE mission_defn ILIKE $1 ESCAPE '\\' ORDER BY mission_id LIMIT $2",
        ),
        2 => (
            "searchtemplate",
            "SELECT template_id AS id, template_name AS name FROM resources.entity_templates \
             WHERE template_name ILIKE $1 ESCAPE '\\' ORDER BY template_id LIMIT $2",
        ),
        _ => return Ok(("", Vec::new(), false)),
    };

    let pattern = format!("%{}%", escape_ilike_pattern(query));
    let mut hits = sqlx::query_as::<_, (i32, String)>(sql)
        .bind(&pattern)
        .bind(SEARCH_LIMIT + 1)
        .fetch_all(pool)
        .await?;
    let truncated = hits.len() as i64 > SEARCH_LIMIT;
    if truncated {
        hits.truncate(SEARCH_LIMIT as usize);
    }
    Ok((label, hits, truncated))
}

/// Execute one authoring statement and report the outcome to the GM.
#[tracing::instrument(
    name = "console.authoring_sql",
    level = "info",
    skip_all,
    fields(entity_id, label)
)]
pub(crate) async fn handle_execute_authoring_sql(
    entity_id: u32,
    label: &str,
    sql: &str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        tracing::warn!(
            entity_id,
            label,
            "authoring SQL: no DB pool — live write skipped (seed SQL still \
             recorded cell-side for commit)"
        );
        send_gm_feedback_to_client(
            entity_id,
            &format!("{label}: no live DB — change applied in-memory only"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    };

    // `AssertSqlSafe` is required by sqlx 0.9: `query()` now only accepts
    // `&'static str` unless the caller asserts the string was audited for
    // injection. The audit is the "Trust model" note in this module's header —
    // `sql` is server-generated on the GM-gated `.`-channel, numerics come from
    // cell-parsed `i32`/`f32`, and strings are escaped through
    // `crate::cell::console::seed::sql_str`. No raw client text is concatenated.
    match sqlx::query(AssertSqlSafe(sql)).execute(pool.as_ref()).await {
        Ok(res) => {
            let rows = res.rows_affected();
            if rows == 0 {
                // A zero-row result means the statement ran but matched
                // nothing (e.g. an UPDATE/DELETE whose WHERE found no row, or
                // an idempotent re-run). The seed SQL is still recorded, but
                // the live DB is unchanged — report that distinctly rather
                // than as a successful write, or the GM thinks their authoring
                // edit took effect when it didn't.
                tracing::warn!(
                    entity_id,
                    label,
                    "authoring SQL affected 0 rows — live DB unchanged"
                );
                send_gm_feedback_to_client(
                    entity_id,
                    &format!(
                        "{label}: live DB UNCHANGED (0 rows matched) — seed SQL recorded; \
                         check the target exists"
                    ),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            } else {
                tracing::info!(entity_id, label, rows, "authoring SQL executed");
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("{label}: live DB write ok ({rows} row(s); wiped on next deploy)"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::warn!(entity_id, label, error = %e, "authoring SQL failed");
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: live DB write FAILED ({e})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

/// Run a read-only resource-name search and report `id: name` matches to the GM.
///
/// `kind` selects the table: `0` items, `1` missions, `2` entity_templates.
/// The query string is bound as a parameterized, literally-escaped `ILIKE`
/// pattern (see [`run_console_search`]) — never concatenated — so this is
/// injection-safe regardless of the channel gate, and `%`/`_`/`\` in the
/// GM's search text match themselves rather than acting as SQL wildcards.
#[tracing::instrument(
    name = "console.search",
    level = "info",
    skip_all,
    fields(entity_id, kind)
)]
pub(crate) async fn handle_console_search(
    entity_id: u32,
    kind: u8,
    query: &str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let label = match kind {
        0 => "searchitem",
        1 => "searchmission",
        2 => "searchtemplate",
        _ => return,
    };

    let Some(pool) = db_pool else {
        send_gm_feedback_to_client(
            entity_id,
            &format!("{label}: no live DB connection"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    };

    match run_console_search(pool, kind, query).await {
        Ok((_, hits, _)) if hits.is_empty() => {
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: no matches for '{query}'"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        Ok((_, hits, truncated)) => {
            let summary = if truncated {
                format!(
                    "{label} '{query}': {} match(es) (results truncated, refine your search)",
                    hits.len()
                )
            } else {
                format!("{label} '{query}': {} match(es)", hits.len())
            };
            send_gm_feedback_to_client(entity_id, &summary, transport, connected, entity_to_addr)
                .await;
            for (id, name) in hits {
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("    {id}: {name}"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::warn!(entity_id, label, error = %e, "console search query failed");
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: query failed ({e})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests {
    //! `escape_ilike_pattern` gets pure unit coverage; `run_console_search`
    //! (the escaping + truncation-boundary behavior end to end) needs a live
    //! DB since it's a real `ILIKE` query. Sentinel base `0x7000_5000` — one
    //! slot past the highest reserved elsewhere in the crate
    //! (`contact_list::persistence::tests::TEST_BASE = 0x7000_4000`).

    use super::*;
    use crate::test_support::require_db_or_skip;

    const TEST_BASE: i32 = 0x7000_5000;

    #[test]
    fn escape_ilike_pattern_escapes_percent_underscore_backslash() {
        assert_eq!(
            escape_ilike_pattern("50%_off\\sale"),
            "50\\%\\_off\\\\sale"
        );
        assert_eq!(escape_ilike_pattern("plain text"), "plain text");
        assert_eq!(escape_ilike_pattern(""), "");
    }

    async fn insert_item(pool: &PgPool, id: i32, name: &str) {
        sqlx::query(
            "INSERT INTO resources.items (\
                item_id, description, name, quality_id, tech_comp, tier, max_stack_size\
             ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1) \
             ON CONFLICT (item_id) DO UPDATE SET name = EXCLUDED.name",
        )
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert sentinel resources.items row");
    }

    async fn insert_mission(pool: &PgPool, id: i32, defn: &str) {
        sqlx::query(
            "INSERT INTO resources.missions (\
                mission_id, history_text, award_xp, can_abandon, can_fail, can_repeat_on_fail, \
                difficulty, is_a_story, is_enabled, is_hidden, is_override_mission, \
                is_shareable, level, mission_defn, mission_label, num_repeats, \
                show_faction_change_icon, show_instance_icon, show_pvp_icon\
             ) VALUES ($1, '', false, false, false, false, 1, false, true, false, false, \
                       false, 1, $2, 'test', 0, false, false, false) \
             ON CONFLICT (mission_id) DO UPDATE SET mission_defn = EXCLUDED.mission_defn",
        )
        .bind(id)
        .bind(defn)
        .execute(pool)
        .await
        .expect("insert sentinel resources.missions row");
    }

    async fn insert_template(pool: &PgPool, id: i32, name: &str) {
        sqlx::query(
            "INSERT INTO resources.entity_templates \
                (template_id, template_name, class, body_set) \
             VALUES ($1, $2, 'TestClass', 'TestBodySet') \
             ON CONFLICT (template_id) DO UPDATE SET template_name = EXCLUDED.template_name",
        )
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insert sentinel resources.entity_templates row");
    }

    /// Delete by exact sentinel id only (never by range) across all three
    /// search tables — harmless no-op for tables the given id was never
    /// inserted into.
    async fn cleanup(pool: &PgPool, ids: &[i32]) {
        for id in ids {
            let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
                .bind(id)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM resources.missions WHERE mission_id = $1")
                .bind(id)
                .execute(pool)
                .await;
            let _ = sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
                .bind(id)
                .execute(pool)
                .await;
        }
    }

    /// Regression guard: an unescaped `%` in the GM's query would act as a
    /// SQL wildcard and also match a decoy row with unrelated characters in
    /// its place. A literal search must match only the exact-substring row.
    #[tokio::test]
    async fn legacy_p01_search_percent_is_literal_not_wildcard() {
        let pool = require_db_or_skip!();
        let exact = TEST_BASE + 1;
        let decoy = TEST_BASE + 2;
        insert_item(&pool, exact, "legacyp01 abc%def literal").await;
        insert_item(&pool, decoy, "legacyp01 abcXXXdef literal").await;

        let (_, hits, truncated) = run_console_search(&pool, 0, "abc%def")
            .await
            .expect("search must succeed");

        cleanup(&pool, &[exact, decoy]).await;

        assert!(!truncated);
        let ids: Vec<i32> = hits.iter().map(|(id, _)| *id).collect();
        assert!(
            ids.contains(&exact),
            "literal '%' must match the exact-substring row: {ids:?}"
        );
        assert!(
            !ids.contains(&decoy),
            "literal '%' must NOT act as a SQL wildcard and match the decoy row: {ids:?}"
        );
    }

    /// Same as the `%` guard, for `_` (SQL "any single character").
    #[tokio::test]
    async fn legacy_p01_search_underscore_is_literal_not_wildcard() {
        let pool = require_db_or_skip!();
        let exact = TEST_BASE + 3;
        let decoy = TEST_BASE + 4;
        insert_item(&pool, exact, "legacyp01 ab_cd literal").await;
        insert_item(&pool, decoy, "legacyp01 abXcd literal").await;

        let (_, hits, truncated) = run_console_search(&pool, 0, "ab_cd")
            .await
            .expect("search must succeed");

        cleanup(&pool, &[exact, decoy]).await;

        assert!(!truncated);
        let ids: Vec<i32> = hits.iter().map(|(id, _)| *id).collect();
        assert!(
            ids.contains(&exact),
            "literal '_' must match the exact-substring row: {ids:?}"
        );
        assert!(
            !ids.contains(&decoy),
            "literal '_' must NOT act as a SQL single-char wildcard and match the decoy row: {ids:?}"
        );
    }

    /// A literal backslash in the query text must match a name containing a
    /// literal backslash (not be swallowed as an escape-sequence starter).
    #[tokio::test]
    async fn legacy_p01_search_backslash_is_literal() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 5;
        insert_item(&pool, id, "legacyp01 path\\to\\item literal").await;

        let (_, hits, _) = run_console_search(&pool, 0, "path\\to")
            .await
            .expect("search must succeed");

        cleanup(&pool, &[id]).await;

        assert!(
            hits.iter().any(|(hit_id, _)| *hit_id == id),
            "literal backslash in the query must match the literal-backslash row: {hits:?}"
        );
    }

    /// Case-insensitive, multi-word substring match — the base of the search
    /// contract (`.searchitem zatnikatel staff` style two-token queries
    /// arrive here already space-joined by `console::query::search`).
    #[tokio::test]
    async fn legacy_p01_search_is_case_insensitive_and_multi_word() {
        let pool = require_db_or_skip!();
        let id = TEST_BASE + 6;
        insert_item(&pool, id, "Legacyp01 Zatnikatel Staff Mk2").await;

        let (_, upper_hits, _) = run_console_search(&pool, 0, "ZATNIKATEL STAFF")
            .await
            .expect("search must succeed");
        let (_, lower_hits, _) = run_console_search(&pool, 0, "zatnikatel staff")
            .await
            .expect("search must succeed");

        cleanup(&pool, &[id]).await;

        assert!(upper_hits.iter().any(|(hit_id, _)| *hit_id == id));
        assert!(lower_hits.iter().any(|(hit_id, _)| *hit_id == id));
    }

    /// Exactly `SEARCH_LIMIT` (25) matches must report the true, untruncated
    /// count; `SEARCH_LIMIT + 1` (26) matches must truncate to 25 and report
    /// `truncated = true` — the GM must be able to tell "exactly 25 hits"
    /// from "more than 25 hits, refine your search".
    #[tokio::test]
    async fn legacy_p01_search_truncates_at_25_and_reports_exact_count_at_boundary() {
        let pool = require_db_or_skip!();
        let ids: Vec<i32> = (0..26).map(|i| TEST_BASE + 100 + i).collect();
        for &id in &ids {
            insert_item(&pool, id, "legacyp01trunc probe").await;
        }

        let (_, hits_26, truncated_26) = run_console_search(&pool, 0, "legacyp01trunc")
            .await
            .expect("search must succeed");
        assert_eq!(hits_26.len(), 25, "26 matches must be truncated to 25");
        assert!(truncated_26, "26 matches must report truncated = true");

        // Drop one row so exactly 25 remain, and re-query.
        let dropped = ids[0];
        let _ = sqlx::query("DELETE FROM resources.items WHERE item_id = $1")
            .bind(dropped)
            .execute(&pool)
            .await;

        let (_, hits_25, truncated_25) = run_console_search(&pool, 0, "legacyp01trunc")
            .await
            .expect("search must succeed");

        cleanup(&pool, &ids).await;

        assert_eq!(hits_25.len(), 25, "exactly 25 matches must report all 25");
        assert!(
            !truncated_25,
            "exactly 25 matches must report truncated = false"
        );
    }

    /// A query that matches nothing returns an empty, non-truncated result —
    /// the "empty" half of the "empty/error responses" acceptance criterion.
    #[tokio::test]
    async fn legacy_p01_search_no_matches_returns_empty_untruncated() {
        let pool = require_db_or_skip!();
        let (_, hits, truncated) =
            run_console_search(&pool, 0, "zzz-legacyp01-no-such-item-exists-zzz")
                .await
                .expect("search must succeed");
        assert!(hits.is_empty());
        assert!(!truncated);
    }

    /// The escaping/truncation fix applies uniformly across all three search
    /// kinds, not just `searchitem` — sanity-check mission (kind 1) and
    /// template (kind 2) with the same literal-`%` shape as the item guard.
    #[tokio::test]
    async fn legacy_p01_search_literal_percent_applies_to_mission_and_template_kinds() {
        let pool = require_db_or_skip!();
        let mission_exact = TEST_BASE + 7;
        let mission_decoy = TEST_BASE + 8;
        let template_exact = TEST_BASE + 9;
        let template_decoy = TEST_BASE + 10;
        insert_mission(&pool, mission_exact, "legacyp01 mis%sion literal").await;
        insert_mission(&pool, mission_decoy, "legacyp01 misXXsion literal").await;
        insert_template(&pool, template_exact, "legacyp01 tmp%late literal").await;
        insert_template(&pool, template_decoy, "legacyp01 tmpXXlate literal").await;

        let (_, mission_hits, _) = run_console_search(&pool, 1, "mis%sion")
            .await
            .expect("mission search must succeed");
        let (_, template_hits, _) = run_console_search(&pool, 2, "tmp%late")
            .await
            .expect("template search must succeed");

        cleanup(
            &pool,
            &[mission_exact, mission_decoy, template_exact, template_decoy],
        )
        .await;

        let mission_ids: Vec<i32> = mission_hits.iter().map(|(id, _)| *id).collect();
        assert!(mission_ids.contains(&mission_exact));
        assert!(!mission_ids.contains(&mission_decoy));

        let template_ids: Vec<i32> = template_hits.iter().map(|(id, _)| *id).collect();
        assert!(template_ids.contains(&template_exact));
        assert!(!template_ids.contains(&template_decoy));
    }

    /// An unrecognized `kind` must short-circuit without ever running a
    /// query (mirrors the old caller-side `_ => return`).
    #[tokio::test]
    async fn legacy_p01_search_unknown_kind_returns_empty_label() {
        let pool = require_db_or_skip!();
        let (label, hits, truncated) = run_console_search(&pool, 99, "anything")
            .await
            .expect("unknown kind must not error");
        assert_eq!(label, "");
        assert!(hits.is_empty());
        assert!(!truncated);
    }
}
