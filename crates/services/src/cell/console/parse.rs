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

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> (mpsc::Sender<CellToBaseMsg>, mpsc::Receiver<CellToBaseMsg>) {
        mpsc::channel(8)
    }

    /// `true` iff at least one GM-facing feedback message landed on the channel.
    fn fed_back(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> bool {
        // Exactly-one semantics: a parser emits a single feedback message per
        // failure, so a double-feedback regression panics here instead of
        // passing silently.
        let first = rx.try_recv().is_ok();
        assert!(
            rx.try_recv().is_err(),
            "expected at most one GM-feedback message"
        );
        first
    }

    // ---- parse_i32 ---------------------------------------------------------

    #[tokio::test]
    async fn parse_i32_valid_returns_value_no_feedback() {
        let (tx, mut rx) = channel();
        let got = parse_i32(7, &["-42", "x"], 0, "count", &tx).await;
        assert_eq!(got, Some(-42));
        assert!(!fed_back(&mut rx), "valid input must not feed back");
    }

    #[tokio::test]
    async fn parse_i32_non_numeric_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        let got = parse_i32(7, &["abc"], 0, "count", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx), "malformed input must feed back");
    }

    #[tokio::test]
    async fn parse_i32_missing_arg_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        // idx past the end of the slice exercises the `args.get` -> None arm.
        let got = parse_i32(7, &["1"], 5, "count", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_i32_float_string_is_rejected() {
        let (tx, mut rx) = channel();
        // "1.5" is not a valid i32 literal.
        let got = parse_i32(7, &["1.5"], 0, "count", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    // ---- parse_f32 ---------------------------------------------------------

    #[tokio::test]
    async fn parse_f32_valid_returns_value_no_feedback() {
        let (tx, mut rx) = channel();
        let got = parse_f32(7, &["3.5"], 0, "x", &tx).await;
        assert_eq!(got, Some(3.5));
        assert!(!fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_f32_non_numeric_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        let got = parse_f32(7, &["nope"], 0, "x", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_f32_missing_arg_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        let got = parse_f32(7, &[], 0, "x", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_f32_nan_is_rejected_by_finite_filter() {
        let (tx, mut rx) = channel();
        // "NaN" parses to f32 but fails the `is_finite` filter -> None branch.
        let got = parse_f32(7, &["NaN"], 0, "x", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_f32_inf_is_rejected_by_finite_filter() {
        let (tx, mut rx) = channel();
        let got = parse_f32(7, &["inf"], 0, "x", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    // ---- parse_bool --------------------------------------------------------

    #[tokio::test]
    async fn parse_bool_truthy_values() {
        for s in ["1", "true", "TRUE", "True"] {
            let (tx, mut rx) = channel();
            let got = parse_bool(7, &[s], 0, "flag", &tx).await;
            assert_eq!(got, Some(true), "{s} should parse true");
            assert!(!fed_back(&mut rx), "{s} should not feed back");
        }
    }

    #[tokio::test]
    async fn parse_bool_falsy_values() {
        for s in ["0", "false", "FALSE", "False"] {
            let (tx, mut rx) = channel();
            let got = parse_bool(7, &[s], 0, "flag", &tx).await;
            assert_eq!(got, Some(false), "{s} should parse false");
            assert!(!fed_back(&mut rx), "{s} should not feed back");
        }
    }

    #[tokio::test]
    async fn parse_bool_invalid_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        let got = parse_bool(7, &["maybe"], 0, "flag", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }

    #[tokio::test]
    async fn parse_bool_missing_arg_returns_none_and_feeds_back() {
        let (tx, mut rx) = channel();
        // `args.get(idx)` -> None falls into the catch-all `_` arm.
        let got = parse_bool(7, &[], 0, "flag", &tx).await;
        assert_eq!(got, None);
        assert!(fed_back(&mut rx));
    }
}
