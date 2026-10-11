//! The sampler's server reads: one `tools/call` shape, the player lookup,
//! the DB snapshot and the noise filter. All of it goes through
//! [`ServerInvoker::call`]; the only SQL is read-only and keyed by a
//! validated name or the integer `player_id` (D-LR2).

use serde_json::{json, Value};

use super::super::log::{DbSnapshot, SnapItem, SnapMission, SnapPlayer};
use super::snapshot_sql;
use crate::uat::golden::normalise::{glob_match, DEFAULT_IGNORE};
use crate::uat::invoke::ServerInvoker;

/// One call; a failed outcome is `Err("<tool>: <error>")`.
pub(super) async fn call_ok(
    server: &dyn ServerInvoker,
    name: &str,
    args: Value,
) -> Result<Value, String> {
    let out = server.call(name, args).await;
    if out.ok {
        Ok(out.json)
    } else {
        Err(format!("{name}: {}", out.error.unwrap_or_default()))
    }
}

/// `server_db_query`'s rows (each one a JSON object).
async fn db_rows(server: &dyn ServerInvoker, sql: &str) -> Result<Vec<Value>, String> {
    let r = call_ok(server, "server_db_query", json!({ "sql": sql })).await?;
    r.get("rows")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| format!("server_db_query: no rows in {r}"))
}

/// The player's `sgw_player.player_id`. `character` has passed
/// [`super::check_character`], so it holds no quote.
pub(super) async fn lookup_player_id(
    server: &dyn ServerInvoker,
    character: &str,
) -> Result<i32, String> {
    let sql = format!("SELECT player_id FROM sgw_player WHERE player_name = '{character}'");
    let rows = db_rows(server, &sql).await?;
    match rows.as_slice() {
        [one] => one
            .get("player_id")
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok())
            .ok_or_else(|| format!("sgw_player row without a player_id: {one}")),
        [] => Err(format!("no sgw_player named {character:?}")),
        many => Err(format!(
            "{} sgw_player rows named {character:?}",
            many.len()
        )),
    }
}

/// The contract's three snapshot statements. One that fails drops the
/// whole snapshot: a partial one would read as missions disappearing.
pub(super) async fn db_snapshot(
    server: &dyn ServerInvoker,
    player_id: i32,
) -> Result<DbSnapshot, String> {
    let [missions, player, items] = snapshot_sql(player_id);
    let missions = db_rows(server, &missions)
        .await?
        .into_iter()
        .map(serde_json::from_value::<SnapMission>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("sgw_mission row: {e}"))?;
    let player = db_rows(server, &player)
        .await?
        .into_iter()
        .next()
        .map(serde_json::from_value::<SnapPlayer>)
        .transpose()
        .map_err(|e| format!("sgw_player row: {e}"))?;
    // `type_id` and `container_id` are nullable; a row without them names
    // no item a clause could check, so it is skipped.
    let items = db_rows(server, &items)
        .await?
        .into_iter()
        .filter_map(|r| serde_json::from_value::<SnapItem>(r).ok())
        .collect();
    Ok(DbSnapshot {
        missions,
        player,
        items,
    })
}

/// `DEFAULT_IGNORE` from lab-golden: messages whose count is time spent.
pub(super) fn is_noise(m: &Value) -> bool {
    let name = m.get("msg_name").and_then(Value::as_str).unwrap_or("");
    DEFAULT_IGNORE.iter().any(|g| glob_match(g, name))
}
