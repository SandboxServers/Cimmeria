//! Frame-bracketed captures (C4, D-AX2). The PrintWindow capture stays on
//! the daemon side; the bridge heartbeat before and after it names the
//! client frames the picture spans. A failed heartbeat never fails the
//! capture: it only drops the bracket.

use super::screenshot::CapturedImage;
use super::Supervisor;

/// The client frames a capture spans, `[before, after]`. `Some` only when
/// both heartbeats answered and they are in order.
pub fn bracket(before: Option<u64>, after: Option<u64>) -> Option<[u64; 2]> {
    match (before, after) {
        (Some(b), Some(a)) if a >= b => Some([b, a]),
        _ => None,
    }
}

/// The frame span as the text of a screenshot result, or `""` when unknown.
pub fn frames_text(b: Option<[u64; 2]>) -> String {
    match b {
        Some([from, to]) => format!(", frames {from}..{to}"),
        None => String::new(),
    }
}

impl Supervisor {
    /// Capture the client window, bracketed by heartbeats (see [`bracket`]).
    pub async fn capture_bracketed(&self) -> Result<(CapturedImage, Option<[u64; 2]>), String> {
        let before = self.bridge.heartbeat().await.ok();
        let img = self.capture().await?;
        let after = self.bridge.heartbeat().await.ok();
        Ok((img, bracket(before, after)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_needs_both_in_order() {
        assert_eq!(bracket(Some(5), Some(7)), Some([5, 7]));
        assert_eq!(bracket(None, Some(7)), None);
        assert_eq!(bracket(Some(7), Some(5)), None);
    }

    #[test]
    fn frames_text_format() {
        assert_eq!(frames_text(Some([81230, 81233])), ", frames 81230..81233");
        assert_eq!(frames_text(None), "");
    }
}
