//! Shared arg-parsing helpers for the `.`-console command handlers.
//!
//! Each parser feeds back a GM-facing error and returns `None` on a malformed
//! value, so call sites can do `let Some(v) = parse_i32(...) else { return };`.

use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::messages::CellToBaseMsg;

/// Parse the `idx`th positional arg as `i32`, feeding back a GM-facing error and
/// returning `None` on a malformed value. Callers `let Some(v) = parse_i32(...)
/// else { return }`.
pub(crate) async fn parse_i32(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<i32> {
    match args.get(idx).and_then(|s| s.parse::<i32>().ok()) {
        Some(v) => Some(v),
        None => {
            send_gm_feedback(caller_id, &format!("{label} must be an integer"), tx).await;
            None
        }
    }
}

/// Parse the `idx`th positional arg as `f32`, feeding back a GM-facing error and
/// returning `None` on a malformed value.
pub(crate) async fn parse_f32(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<f32> {
    // Reject non-finite values: `NaN`/`inf` parse fine but Display as bareword
    // `NaN`/`inf`, which is invalid as a SQL numeric literal — it would fail the
    // live authoring write AND bake a broken statement into the recorded seed.
    match args
        .get(idx)
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| v.is_finite())
    {
        Some(v) => Some(v),
        None => {
            send_gm_feedback(caller_id, &format!("{label} must be a finite number"), tx).await;
            None
        }
    }
}

/// Parse a `0`/`1`/`true`/`false` boolean arg, feeding back on a malformed value.
pub(crate) async fn parse_bool(
    caller_id: u32,
    args: &[&str],
    idx: usize,
    label: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> Option<bool> {
    match args.get(idx).map(|s| s.to_ascii_lowercase()) {
        Some(s) if s == "1" || s == "true" => Some(true),
        Some(s) if s == "0" || s == "false" => Some(false),
        _ => {
            send_gm_feedback(caller_id, &format!("{label} must be 0/1/true/false"), tx).await;
            None
        }
    }
}
