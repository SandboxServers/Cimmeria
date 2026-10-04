//! Authoring persistence — the single choke point for everything the `.`-console
//! authoring commands write.
//!
//! In this repo the seed files under `db/resources/` are the source of truth —
//! the DB is rebuilt from them, and the standing rule is "edit the seed in
//! `db/resources/`, never a `db/scripts/*.sql` migration." So an authoring
//! command goes through two stages:
//!
//! 1. **Queue** ([`record`] / [`record_spawn`]). The caller has already applied
//!    the effect in memory, so the GM sees it immediately — a moved NPC stands
//!    in its new spot, a freshly-assigned patrol starts walking. The generated
//!    SQL is appended to a per-session on-disk log (so nothing is lost) and
//!    buffered per-GM. **No database is touched yet**; `.seedcancel` drops the
//!    buffer and nothing was written.
//! 2. **Confirm** ([`confirm`], `.seedconfirm`). Each queued statement is sent
//!    to the live DB via [`CellToBaseMsg::ExecuteAuthoringSql`] so the change
//!    holds across reconnects within the current deploy (transient: the next
//!    deploy rebuilds from seeds and wipes it), and emitted to telemetry as the
//!    durable artifact a developer merges into the seed.
//!
//! **The raw SQL is never shown in-game** — the client's chat isn't
//! copy-pasteable, so emitting it there is useless. It goes out-of-band: the
//! per-session log file ([`session_log_path`]) and the `authoring` tracing
//! target, which the OTLP exporter ships to SigNoz. In-game feedback is
//! status-only ("queued — N pending", "saved N change(s)").
//!
//! # What reaches SigNoz on confirm
//!
//! Every confirm stamps one `batch` id on all its events:
//!
//! - one `seed spawn confirmed` event per `.savespawn` / `.delspawn`, carrying
//!   every spawnlist column as its own field (`op`, `spawn_id`, `world`,
//!   `world_id`, `template_id`, `x`/`y`/`z`, `heading`, `tag`) plus the exact
//!   `sql`, so the row can be rebuilt without parsing the statement;
//! - one `seed authoring confirmed` event per seed file, whose body is the
//!   statement block to append to that file.
//!
//! How a developer turns a batch into a seed commit is in
//! `docs/guides/placing-npcs-and-objects.md`.
//!
//! # Future Discord hook
//!
//! The per-file block [`confirm`] emits is exactly what a `cimmeria-discord`
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
use crate::cell::space_manager::{AuthoringChange, SpaceManager, SpawnRowChange, SpawnRowOp};

/// Queue one authored change for [`confirm`]. `label` is a short human tag
/// (e.g. `"path_add"`); `seed_file` is the `db/resources/` path the statement
/// should be committed into. Touches no database.
pub(crate) async fn record(
    caller_id: u32,
    seed_file: &str,
    label: &str,
    sql: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    queue(
        caller_id,
        change(seed_file, label, sql, None),
        tx,
        space_mgr,
    )
    .await;
}

/// [`record`] for a spawnlist change, carrying the structured row that
/// [`confirm`] emits to telemetry.
///
/// A GM re-saving the same NPC before confirming replaces the queued change
/// rather than adding a second one, so "move it, save, nudge it, save again"
/// confirms exactly one row — and never a second `INSERT` for one NPC.
pub(crate) async fn record_spawn(
    caller_id: u32,
    seed_file: &str,
    label: &str,
    sql: &str,
    row: SpawnRowChange,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    queue(
        caller_id,
        change(seed_file, label, sql, Some(row)),
        tx,
        space_mgr,
    )
    .await;
}

fn change(
    seed_file: &str,
    label: &str,
    sql: &str,
    spawn: Option<SpawnRowChange>,
) -> AuthoringChange {
    AuthoringChange {
        seed_file: seed_file.to_string(),
        label: label.to_string(),
        sql: sql.to_string(),
        spawn,
    }
}

async fn queue(
    caller_id: u32,
    change: AuthoringChange,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    append_session_log(format!(
        "-- [{}] queued (not yet confirmed) -> {}\n{}\n",
        change.label, change.seed_file, change.sql
    ))
    .await;

    let label = change.label.clone();
    let buffer = space_mgr.authoring_changes.entry(caller_id).or_default();
    let replaced = match change.spawn.as_ref().map(|s| s.entity_id) {
        Some(entity_id) => {
            let before = buffer.len();
            buffer.retain(|c| c.spawn.as_ref().map(|s| s.entity_id) != Some(entity_id));
            before != buffer.len()
        }
        None => false,
    };
    buffer.push(change);
    let pending = buffer.len();

    let note = if replaced {
        " (replaced your earlier save of this entity)"
    } else {
        ""
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "{label}: queued{note} -- {pending} pending. .seedconfirm to save, .seedcancel to discard."
        ),
        tx,
    )
    .await;
}

/// `.seedconfirm` — write the caller's queued changes to the live DB, emit them
/// to telemetry (see the module doc for the event shapes), then clear the
/// buffer.
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

    let gm = space_mgr.player_identity(caller_id);
    let batch = batch_id(caller_id);
    let mut spawns = 0usize;
    let mut live_failed = 0usize;

    for c in &changes {
        if let Err(e) = tx
            .send(CellToBaseMsg::ExecuteAuthoringSql {
                entity_id: caller_id,
                label: c.label.clone(),
                sql: c.sql.clone(),
            })
            .await
        {
            // The telemetry below still carries the change, so a developer can
            // merge it; only the live preview is lost.
            live_failed += 1;
            tracing::warn!(
                target: "authoring",
                entity_id = caller_id,
                entity_name = gm.player_name,
                account_id = gm.account_id,
                account_name = gm.account_name,
                player_id = gm.player_id,
                player_name = gm.player_name,
                batch = %batch,
                label = %c.label,
                error = %e,
                "authoring live write not sent (base channel closed); change still emitted"
            );
        }
        if let Some(s) = &c.spawn {
            spawns += 1;
            if s.op == SpawnRowOp::Insert {
                space_mgr.confirmed_new_spawns.insert(s.entity_id);
            }
            tracing::info!(
                target: "authoring",
                entity_id = caller_id,
                entity_name = gm.player_name,
                account_id = gm.account_id,
                account_name = gm.account_name,
                player_id = gm.player_id,
                player_name = gm.player_name,
                batch = %batch,
                seed_file = %c.seed_file,
                label = %c.label,
                op = s.op.as_str(),
                npc_entity_id = s.entity_id,
                npc_entity_name = space_mgr.entity_label(s.entity_id),
                spawn_id = s.spawn_id, // nt:id-only a spawnlist row, which has no name column
                world = %s.world,
                world_id = s.world_id,
                template_id = s.template_id,
                template_name = cimmeria_names::book().template(s.template_id),
                x = s.x,
                y = s.y,
                z = s.z,
                heading = s.heading,
                heading_deg = s.heading.to_degrees(),
                tag = s.tag.as_deref(),
                sql = %c.sql,
                "seed spawn confirmed",
            );
        }
    }

    // Group by seed file, preserving first-seen file order and per-file
    // statement order.
    let mut files: Vec<&str> = Vec::new();
    for c in &changes {
        if !files.contains(&c.seed_file.as_str()) {
            files.push(&c.seed_file);
        }
    }
    for file in &files {
        let stmts: Vec<&str> = changes
            .iter()
            .filter(|c| c.seed_file == *file)
            .map(|c| c.sql.as_str())
            .collect();
        let block = stmts.join("\n");
        tracing::info!(
            target: "authoring",
            entity_id = caller_id,
            entity_name = gm.player_name,
            account_id = gm.account_id,
            account_name = gm.account_name,
            player_id = gm.player_id,
            player_name = gm.player_name,
            batch = %batch,
            seed_file = %file,
            statements = stmts.len(),
            "seed authoring confirmed:\n{block}"
        );
        append_session_log(format!(
            "-- ===== CONFIRMED batch {batch} by entity {caller_id} -> commit into {file} =====\n{block}\n"
        ))
        .await;
    }

    let live = if live_failed == 0 {
        "live on this server now".to_string()
    } else {
        format!("{live_failed} could NOT be written live (see server log)")
    };
    send_gm_feedback(
        caller_id,
        &format!(
            "seedconfirm: saved {} change(s), {spawns} of them spawn(s); {live}. \
             Sent for merge as batch {batch}.",
            changes.len(),
        ),
        tx,
    )
    .await;
}

/// A short id shared by every event of one confirm, so a developer can pull a
/// whole batch out of SigNoz with one `batch = …` filter.
fn batch_id(caller_id: u32) -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    format!("{caller_id}-{ms}")
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
            for file in changes.iter().map(|c| &c.seed_file) {
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
/// but never abort the command — the queue and the SigNoz events still hold the change.
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
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "authoring log append failed",
                    );
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

    /// Queue-then-confirm: recording writes nothing live; confirming does.
    #[tokio::test]
    async fn record_queues_and_confirm_sends_live_write() {
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
        assert_eq!(mgr.authoring_changes.get(&7).map(Vec::len), Some(1));
        let live_writes = |rx: &mut mpsc::Receiver<CellToBaseMsg>| {
            let mut n = 0;
            while let Ok(msg) = rx.try_recv() {
                if matches!(msg, CellToBaseMsg::ExecuteAuthoringSql { .. }) {
                    n += 1;
                }
            }
            n
        };
        assert_eq!(live_writes(&mut rx), 0, "record must not touch the DB");

        confirm(7, &tx, &mut mgr).await;
        assert_eq!(live_writes(&mut rx), 1, "confirm sends the live write");
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
