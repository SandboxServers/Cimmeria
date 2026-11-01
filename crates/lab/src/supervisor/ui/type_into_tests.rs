use std::sync::Arc;

use serde_json::{json, Value};

use super::{read_text_chunk, typed_evidence};
use crate::proof::{EvVerdict, Why};
use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};
use crate::supervisor::Supervisor;

/// A fake client whose edit box answers `text` to every read; `None` makes
/// the read fail (no window).
async fn edit_box(text: Option<&'static str>) -> Supervisor {
    let responder: Responder = Arc::new(move |method, _: &Value| {
        if method != "lua_eval" {
            return Err(format!("unexpected call: {method}"));
        }
        let v = match text {
            Some(t) => json!({ "done": true, "text": t }),
            None => json!({ "done": false, "text": "" }),
        };
        Ok(lua_ok(&[v.to_string()]))
    });
    fake_bridge::supervisor(responder).await
}

/// The box holds what was typed: confirmed. It holds nothing: denied.
#[tokio::test]
async fn type_into_reads_back() {
    let sup = edit_box(Some("hello")).await;
    let read = sup.read_edit_box("ChatEdit").await;
    let ev = typed_evidence(Some("ChatEdit"), "hello", read.as_deref());
    assert_eq!(ev.verdict, EvVerdict::Confirmed);

    let sup = edit_box(Some("")).await;
    let read = sup.read_edit_box("ChatEdit").await;
    let ev = typed_evidence(Some("ChatEdit"), "hello", read.as_deref());
    assert_eq!(ev.verdict, EvVerdict::Denied);
    assert_eq!(ev.why, Why::NoEffect);
}

/// A box that cannot be read gives no read-back, so nothing is judged.
#[tokio::test]
async fn a_missing_box_reads_as_no_text() {
    let sup = edit_box(None).await;
    assert_eq!(sup.read_edit_box("ChatEdit").await, None);
    let ev = typed_evidence(Some("ChatEdit"), "x", None);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
}

#[test]
fn type_without_into_is_posted_only() {
    let ev = typed_evidence(None, "x", None);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::PostedOnly);
}

#[test]
fn read_back_failure_is_no_post_read() {
    let ev = typed_evidence(Some("ChatEdit"), "x", None);
    assert_eq!(ev.verdict, EvVerdict::Inconclusive);
    assert_eq!(ev.why, Why::NoPostRead);
}

/// The read names the window through `getWindow` as well as the global.
#[test]
fn read_chunk_reads_the_text_through_get_window() {
    let c = read_text_chunk("Inst1Chat/Input");
    assert!(c.contains("getWindow") && c.contains("getText"), "{c}");
}
