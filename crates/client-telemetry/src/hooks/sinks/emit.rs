//! How a sink hands an event on: one call that queues it for the uploader
//! and, in a lab build, mirrors it into the bridge's local ring.
//!
//! The event-registry and CEGUI hooks each carry their own copy of these
//! few lines; the sinks share this one so the eight of them cannot drift.
//!
//! In a unit test there is no producer (`boot` has not run), so `emit`
//! records the event in a thread-local list instead. That lets a test drive
//! a detour and read back exactly what it would have sent.

use serde_json::Value;

/// The fields of one event, in emit order.
pub type Fields = Vec<(&'static str, Value)>;

/// Queue an event under `target` at `level`, and (lab builds) mirror it to
/// the bridge ring as `bridge_kind`.
///
/// Best-effort and non-blocking: a full queue drops the event and counts
/// it, like every other producer in the DLL.
#[cfg(not(test))]
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(crate) fn emit(
    target: &'static str,
    level: &'static str,
    bridge_kind: &'static str,
    fields: Fields,
) {
    #[cfg(feature = "lab-bridge")]
    crate::bridge::events::push(
        bridge_kind,
        crate::bridge::crash::now_ms(),
        Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
        ),
    );
    #[cfg(not(feature = "lab-bridge"))]
    let _ = bridge_kind;

    #[cfg(windows)]
    if let Some(p) = crate::boot::producer() {
        let mut b = crate::events::ClientNativeEvent::builder(target, level);
        for (k, v) in fields {
            b = b.field(k, v);
        }
        p.try_emit(b);
    }
    #[cfg(not(windows))]
    let _ = (target, level, fields);
}

/// Test build: record instead of sending.
#[cfg(test)]
pub(crate) fn emit(
    target: &'static str,
    level: &'static str,
    bridge_kind: &'static str,
    fields: Fields,
) {
    CAPTURED.with(|c| {
        c.borrow_mut().push(Captured {
            target,
            level,
            bridge_kind,
            fields,
        })
    });
}

/// One event a test build recorded.
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct Captured {
    pub target: &'static str,
    pub level: &'static str,
    pub bridge_kind: &'static str,
    pub fields: Fields,
}

#[cfg(test)]
impl Captured {
    /// The value of field `key`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
}

#[cfg(test)]
thread_local! {
    static CAPTURED: std::cell::RefCell<Vec<Captured>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Take every event recorded on this thread so far.
#[cfg(test)]
pub(crate) fn take_captured() -> Vec<Captured> {
    CAPTURED.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_build_records_what_would_have_been_sent() {
        let _ = take_captured();
        emit(
            "client.test.thing",
            "info",
            "test.thing",
            vec![("a", serde_json::json!(1)), ("b", serde_json::json!("x"))],
        );
        let got = take_captured();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].target, "client.test.thing");
        assert_eq!(got[0].level, "info");
        assert_eq!(got[0].bridge_kind, "test.thing");
        assert_eq!(got[0].get("a"), Some(&serde_json::json!(1)));
        assert_eq!(got[0].get("missing"), None);
        assert!(take_captured().is_empty(), "taking drains the list");
    }
}
