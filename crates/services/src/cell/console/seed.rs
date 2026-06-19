//! Authoring persistence — the single choke point for everything the `.`-console
//! authoring commands write.
//!
//! In this repo the seed files under `db/resources/` are the source of truth —
//! the DB is rebuilt from them, and the standing rule is "edit the seed in
//! `db/resources/`, never a `db/scripts/*.sql` migration." So an authoring
//! command does three things, in order:
//!
//! 1. **Apply in memory** (the caller does this before calling here) so the GM
//!    sees the effect immediately — a freshly-assigned patrol starts walking now.
//! 2. **Write the live DB** via [`CellToBaseMsg::ExecuteAuthoringSql`] so the
//!    change holds across reconnects within the current deploy. This is
//!    transient: the next deploy rebuilds from seeds and wipes it.
//! 3. **Record the seed SQL** as the durable artifact for a human to commit.
//!    Each statement is appended to a per-session on-disk log immediately (so
//!    nothing is lost even without confirmation) and buffered per-GM for
//!    [`confirm`], which groups the statements per seed file.
//!
//! **The raw SQL is never shown in-game** — the client's chat isn't
//! copy-pasteable, so emitting it there is useless. It goes out-of-band: the
//! per-session log file ([`session_log_path`]) and the server's tracing log
//! today, and (once enabled on the colo) a Discord authoring channel. In-game
//! feedback is status-only ("recorded — N pending", "confirmed: M statements").
//!
//! # Future Discord hook
//!
//! [`confirm`] groups the buffer per seed file and emits each block to tracing
//! today. The same per-file payload is exactly what a `cimmeria-discord`
//! `EventKind` (e.g. `SeedAuthored`) would post once the colo integration is on
//! — a sink swap inside [`confirm`], not a redesign. Discord is intentionally
//! NOT wired here yet (it's off in the colo, and adding an event type is its own
//! checklist in `docs/architecture/discord-notifications.md`).

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Record one authored change: send the live DB write to the base, append the
/// statement to the per-session on-disk log, and buffer it per-GM for
/// [`confirm`]. `label` is a short human tag (e.g. `"savespawn"`); `seed_file`
/// is the `db/resources/` path the statement should be committed into.
///
/// In-game feedback is a status line only — the SQL itself goes to the log file
/// and (on confirm) tracing / Discord.
pub(crate) async fn record(
    caller_id: u32,
    seed_file: &str,
    label: &str,
    sql: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // 2. Live DB write (transient — wiped on next deploy). A closed base
    //    channel means the live write is lost; warn so the GM's "recorded"
    //    feedback isn't silently overstating what happened (the seed log + buffer
    //    below still capture the durable artifact).
    if let Err(e) = tx
        .send(CellToBaseMsg::ExecuteAuthoringSql {
            entity_id: caller_id,
            label: label.to_string(),
            sql: sql.to_string(),
        })
        .await
    {
        tracing::warn!(
            entity_id = caller_id,
            label,
            error = %e,
            "authoring live-write enqueue failed (base channel closed); seed still recorded"
        );
    }

    // 3a. Persist to the per-session on-disk log immediately.
    append_session_log(format!("-- [{label}] -> {seed_file}\n{sql}\n")).await;

    // 3b. Buffer per-GM for grouped confirmation.
    space_mgr
        .authoring_changes
        .entry(caller_id)
        .or_default()
        .push((seed_file.to_string(), sql.to_string()));

    let pending = space_mgr
        .authoring_changes
        .get(&caller_id)
        .map_or(0, Vec::len);
    send_gm_feedback(
        caller_id,
        &format!(
            "{label}: recorded ({pending} pending). .seedconfirm to emit, .seedcancel to discard."
        ),
        tx,
    )
    .await;
}

/// Flush the caller's buffered authoring statements: group them per seed file,
/// emit each file's block to the server log (and the per-session on-disk log),
/// then clear the buffer. The developer confirms here, so only reviewed sets
/// reach the out-of-band sinks.
pub(crate) async fn confirm(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let changes = space_mgr
        .authoring_changes
        .remove(&caller_id)
        .unwrap_or_default();
    if changes.is_empty() {
        send_gm_feedback(caller_id, "seedconfirm: no pending changes.", tx).await;
        return;
    }

    // Group by seed file, preserving first-seen file order and per-file
    // statement order.
    let mut files: Vec<String> = Vec::new();
    let mut by_file: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (file, sql) in &changes {
        if !files.contains(file) {
            files.push(file.clone());
        }
        by_file.entry(file.clone()).or_default().push(sql.clone());
    }

    for file in &files {
        let stmts = &by_file[file];
        let block = stmts.join("\n");
        // Out-of-band emission #1: server tracing log. Future Discord sink
        // posts this same (file, block) payload.
        tracing::info!(
            target: "authoring",
            entity_id = caller_id,
            seed_file = %file,
            statements = stmts.len(),
            "seed authoring confirmed:\n{block}"
        );
        // Out-of-band emission #2: per-session on-disk log, marked confirmed.
        append_session_log(format!(
            "-- ===== CONFIRMED by entity {caller_id} -> commit into {file} =====\n{block}\n"
        ))
        .await;
    }

    send_gm_feedback(
        caller_id,
        &format!(
            "seedconfirm: {} statement(s) across {} seed file(s) emitted to the server log + {}",
            changes.len(),
            files.len(),
            session_log_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(log unavailable)".to_string()),
        ),
        tx,
    )
    .await;
}

/// Report how many authoring statements are buffered for the caller, per file.
pub(crate) async fn pending(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let changes = space_mgr.authoring_changes.get(&caller_id);
    match changes.filter(|c| !c.is_empty()) {
        None => send_gm_feedback(caller_id, "seedpending: no pending changes.", tx).await,
        Some(changes) => {
            let mut counts: Vec<(String, usize)> = Vec::new();
            for (file, _) in changes {
                match counts.iter_mut().find(|(f, _)| f == file) {
                    Some((_, n)) => *n += 1,
                    None => counts.push((file.clone(), 1)),
                }
            }
            let summary = counts
                .iter()
                .map(|(f, n)| format!("{f} ({n})"))
                .collect::<Vec<_>>()
                .join(", ");
            send_gm_feedback(
                caller_id,
                &format!("seedpending: {} statement(s) — {summary}", changes.len()),
                tx,
            )
            .await;
        }
    }
}

/// Discard the caller's buffered authoring statements without emitting them.
/// (The per-session on-disk log already holds each statement as it was
/// recorded — this only clears the confirm buffer.)
pub(crate) async fn cancel(
    caller_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let n = space_mgr
        .authoring_changes
        .remove(&caller_id)
        .map_or(0, |c| c.len());
    send_gm_feedback(
        caller_id,
        &format!("seedcancel: discarded {n} pending statement(s)."),
        tx,
    )
    .await;
}

/// Format a SQL string literal: single-quote wrapped with internal quotes
/// doubled, or the bareword `NULL` for `None`. Used when building seed
/// `INSERT`/`UPDATE` statements from authored string fields (tags, names) so no
/// raw client text is concatenated unescaped.
pub(crate) fn sql_str(value: Option<&str>) -> String {
    match value {
        Some(s) => format!("'{}'", s.replace('\'', "''")),
        None => "NULL".to_string(),
    }
}

/// Path of the per-session authoring log, fixed once per server run.
///
/// Directory comes from `CIMMERIA_AUTHORING_LOG_DIR` (default `logs`); the file
/// name is stamped with the process start epoch so each server session gets its
/// own file. `None` if the system clock is unavailable (should never happen).
fn session_log_path() -> Option<PathBuf> {
    static PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir =
            std::env::var("CIMMERIA_AUTHORING_LOG_DIR").unwrap_or_else(|_| "logs".to_string());
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        Some(PathBuf::from(dir).join(format!("seed-authoring-{stamp}.sql")))
    })
    .clone()
}

/// Append a block to the per-session authoring log, best-effort. Failures warn
/// but never abort the command — the live DB write and tracing log still landed.
///
/// The actual file I/O runs on a blocking thread (`spawn_blocking`) so a
/// slow/stalled disk can't block the cell's async worker thread, even though
/// this path is GM-only and rare.
async fn append_session_log(block: String) {
    let _ = tokio::task::spawn_blocking(move || {
        let Some(path) = session_log_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(mut f) => {
                if let Err(e) = f.write_all(block.as_bytes()) {
                    tracing::warn!(path = %path.display(), error = %e, "authoring log append failed");
                }
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "authoring log open failed");
            }
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_str_quotes_and_escapes() {
        assert_eq!(sql_str(None), "NULL");
        assert_eq!(sql_str(Some("plain")), "'plain'");
        assert_eq!(sql_str(Some("O'Neill")), "'O''Neill'");
    }

    #[tokio::test]
    async fn record_buffers_and_sends_live_write() {
        let mut mgr = SpaceManager::new(1);
        let (tx, mut rx) = mpsc::channel(16);
        record(
            7,
            "db/resources/X.sql",
            "savespawn",
            "DELETE FROM x;",
            &tx,
            &mut mgr,
        )
        .await;
        // Buffered for confirm.
        assert_eq!(mgr.authoring_changes.get(&7).map(Vec::len), Some(1));
        // First message is the live-write request; a status feedback follows.
        let mut saw_live = false;
        while let Ok(msg) = rx.try_recv() {
            if matches!(msg, CellToBaseMsg::ExecuteAuthoringSql { .. }) {
                saw_live = true;
            }
        }
        assert!(
            saw_live,
            "record must send ExecuteAuthoringSql for the live write"
        );
    }

    #[tokio::test]
    async fn confirm_clears_buffer() {
        let mut mgr = SpaceManager::new(1);
        let (tx, _rx) = mpsc::channel(16);
        record(
            7,
            "db/resources/X.sql",
            "savespawn",
            "DELETE FROM x;",
            &tx,
            &mut mgr,
        )
        .await;
        confirm(7, &tx, &mut mgr).await;
        assert!(mgr.authoring_changes.get(&7).is_none_or(Vec::is_empty));
    }

    #[tokio::test]
    async fn confirm_with_nothing_pending_is_noop_feedback() {
        let mut mgr = SpaceManager::new(1);
        let (tx, mut rx) = mpsc::channel(16);
        confirm(9, &tx, &mut mgr).await;
        // Exactly one feedback line, and nothing else — no live-write message
        // (ExecuteAuthoringSql) and no second feedback. Assert both the single
        // expected message AND that the channel is then empty, so a future
        // change that emits an extra message in the no-op path trips here.
        assert!(matches!(
            rx.try_recv(),
            Ok(CellToBaseMsg::EntityMethodCall { .. })
        ));
        assert!(
            rx.try_recv().is_err(),
            "no-op confirm must emit exactly one feedback message and nothing else"
        );
    }
}
