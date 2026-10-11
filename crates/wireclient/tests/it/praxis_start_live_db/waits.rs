//! The Praxis-start script's own waits and failure reporting: sends, log
//! scans that leave the cursor alone, the `sgw_mission` poll, and the
//! failure message every wait ends in (D-WC9).

use std::time::{Duration, Instant};

use sqlx::PgPool;

use cimmeria_wireclient::script::{Observed, ScriptSession};
use cimmeria_wireclient::semantic::InvItem;
use cimmeria_wireclient::Error;

/// How long one pump runs while a helper below waits on the log or the DB.
const PUMP_SLICE: Duration = Duration::from_millis(50);
const DB_POLL: Duration = Duration::from_millis(100);
/// How many entries a failure prints (D-WC9).
const TRAIL: usize = 30;
/// At most this many zero-wait pumps in a final drain: each takes one
/// bundle off the inbox, and a wait that has run out must not spin forever
/// on a server that keeps sending.
const FINAL_DRAIN_PUMPS: usize = 256;

/// Unwrap a script result, or [`fail_err`] with its error.
pub(super) fn check<T>(script: &ScriptSession, result: cimmeria_wireclient::Result<T>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => fail_err(script, err),
    }
}

/// Send one client call as its own reliable bundle.
pub(super) async fn send(script: &mut ScriptSession, msg: Vec<u8>) {
    let result = script.send(&[msg]).await;
    check(script, result);
}

/// Pump until a log entry at or after `from` matches `pred`, without moving
/// the script's cursor. For answers whose order against the cursor's last
/// match is not fixed (two senders, the base and the cell).
pub(super) async fn wait_since(
    script: &mut ScriptSession,
    from: usize,
    what: &str,
    timeout: Duration,
    mut pred: impl FnMut(&Observed) -> bool,
) -> Observed {
    let deadline = Instant::now() + timeout;
    let mut scanned = from;
    loop {
        if let Some(o) = script.log()[scanned..].iter().find(|o| pred(o)) {
            return o.clone();
        }
        scanned = script.log().len();
        let now = Instant::now();
        if now >= deadline {
            // Whatever already sits in the inbox counts, as in
            // `ScriptSession::wait_until`: the last slice may have ended
            // with the answer queued but not yet taken.
            final_drain(script).await;
            if let Some(o) = script.log()[scanned..].iter().find(|o| pred(o)) {
                return o.clone();
            }
            fail(script, format!("waited {timeout:?} for {what}"));
        }
        let result = script.pump((deadline - now).min(PUMP_SLICE)).await;
        check(script, result);
    }
}

/// Take what is already in the inbox without waiting: zero-wait pumps until
/// one adds nothing (or [`FINAL_DRAIN_PUMPS`]).
async fn final_drain(script: &mut ScriptSession) {
    for _ in 0..FINAL_DRAIN_PUMPS {
        let result = script.pump(Duration::ZERO).await;
        if check(script, result) == 0 {
            return;
        }
    }
}

/// Poll `sgw_mission` every [`DB_POLL`] for up to `timeout` until the
/// mission's status is `want`. The script pumps between polls. The last poll
/// runs at or after the deadline, so a status written just in time counts.
pub(super) async fn wait_mission_status(
    script: &mut ScriptSession,
    pool: &PgPool,
    player_id: i32,
    mission_id: i32,
    want: i32,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let past_deadline = Instant::now() >= deadline;
        let status: Option<i32> = sqlx::query_scalar(
            "SELECT status FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(mission_id)
        .fetch_optional(pool)
        .await
        .unwrap_or_else(|e| fail(script, format!("read sgw_mission {mission_id}: {e}")));
        if status == Some(want) {
            return;
        }
        if past_deadline {
            fail(
                script,
                format!(
                    "sgw_mission {mission_id} status is {status:?} after {timeout:?}, want {want}"
                ),
            );
        }
        let result = script.pump(DB_POLL).await;
        check(script, result);
    }
}

/// Fail with a script error. A [`Error::ScriptTimeout`] already carries the
/// raw trail (`trail(30)`), so only the decoded events are added to it.
pub(super) fn fail_err(script: &ScriptSession, err: Error) -> ! {
    if matches!(err, Error::ScriptTimeout { .. }) {
        panic!(
            "{err}\nlast decoded events:\n{}",
            events_trail(script, TRAIL)
        );
    }
    fail(script, err)
}

/// Fail with `why`, the raw trail (the last [`TRAIL`] messages) and the last
/// [`TRAIL`] decoded events. NPC avatar updates (`0x10`) can fill the whole
/// raw trail while the script waits; the decoded list still shows what the
/// server answered.
pub(super) fn fail(script: &ScriptSession, why: impl std::fmt::Display) -> ! {
    panic!(
        "{why}\nlast messages:\n{}\nlast decoded events:\n{}",
        script.trail(TRAIL),
        events_trail(script, TRAIL)
    )
}

/// The last `n` log entries that decoded to an event, one line each.
fn events_trail(script: &ScriptSession, n: usize) -> String {
    let mut lines: Vec<String> = script
        .log()
        .iter()
        .rev()
        .filter_map(|o| {
            let event = o.event.as_ref()?;
            let mut text = format!("{event:?}");
            if text.len() > 200 {
                text.truncate(text.floor_char_boundary(200));
                text.push_str("...");
            }
            Some(format!(
                "+{:.3}s entity {} {text}",
                o.at.as_secs_f64(),
                o.msg
                    .entity_id
                    .map_or_else(|| "-".to_owned(), |id| id.to_string())
            ))
        })
        .take(n)
        .collect();
    if lines.is_empty() {
        return "(none)".to_owned();
    }
    lines.reverse();
    lines.join("\n")
}

/// The one inventory item matching `pred` in the mirror.
pub(super) fn only_item(script: &ScriptSession, pred: impl Fn(&InvItem) -> bool) -> InvItem {
    let found: Vec<&InvItem> = script
        .mirror
        .inventory()
        .values()
        .filter(|i| pred(i))
        .collect();
    match found.as_slice() {
        [only] => (*only).clone(),
        other => fail(
            script,
            format!("want exactly one matching inventory item, found {other:?}"),
        ),
    }
}
