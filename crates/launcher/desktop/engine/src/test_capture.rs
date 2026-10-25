//! Test-only tracing capture (launcher-telemetry C9). Thread-local: run async code
//! inside `capture` on a current-thread runtime.
use crate::game_telemetry::ip_scrub::contains_ip;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::Context, prelude::*, Layer, Registry};

#[derive(Debug, Clone)]
pub(crate) struct Captured {
    pub target: String,
    pub level: tracing::Level,
    pub fields: BTreeMap<String, String>,
}

impl Captured {
    pub(crate) fn field(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    /// LT-40: panics when the message or any field value holds an IP literal.
    /// Only forwarded targets count (`launcher*` and `cimmeria*`, the telemetry
    /// layer's rule): a third-party event such as hyper's loopback pool log is not a
    /// row. The panic names the event and the field key, never the address.
    /// The stage-failure detail targets stay in the local log, so they may hold a URL.
    fn assert_no_ip(&self) {
        if !(self.target.starts_with("launcher") || self.target.starts_with("cimmeria")) {
            return;
        }
        if matches!(
            self.target.as_str(),
            "launcher.install.detail" | "launcher.repair.detail"
        ) {
            return;
        }
        for (key, value) in &self.fields {
            if contains_ip(value) {
                let event = self.field("event").unwrap_or(self.target.as_str());
                panic!("captured event {event} holds an IP literal in field {key}");
            }
        }
    }
}

struct Cap(Arc<Mutex<Vec<Captured>>>);
struct Visit<'a>(&'a mut BTreeMap<String, String>);

impl tracing::field::Visit for Visit<'_> {
    fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
        self.0.insert(f.name().into(), v.to_owned());
    }
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        self.0.insert(f.name().into(), format!("{v:?}"));
    }
}

impl<S: tracing::Subscriber> Layer<S> for Cap {
    fn on_event(&self, e: &tracing::Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        e.record(&mut Visit(&mut fields));
        self.0.lock().unwrap().push(Captured {
            target: e.metadata().target().into(),
            level: *e.metadata().level(),
            fields,
        });
    }
}

pub(crate) fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<Captured>) {
    let store = Arc::new(Mutex::new(Vec::new()));
    let r = tracing::subscriber::with_default(Registry::default().with(Cap(store.clone())), || {
        tracing::callsite::rebuild_interest_cache();
        f()
    });
    let v = store.lock().unwrap().clone();
    for event in &v {
        event.assert_no_ip();
    }
    (r, v)
}

#[cfg(test)]
mod tests {
    #[test]
    #[should_panic(expected = "holds an IP literal")]
    fn capture_panics_on_an_ip_in_a_field() {
        let _ = super::capture(|| {
            tracing::info!(target: "launcher.install", event = "x_finished", x = "198.51.100.7");
        });
    }

    #[test]
    #[should_panic(expected = "holds an IP literal")]
    fn capture_panics_on_an_ip_after_a_key_and_a_port() {
        let _ = super::capture(|| {
            tracing::info!(target: "launcher.install", event = "x_failed", x = "peer:198.51.100.7:8081");
        });
    }

    #[test]
    fn capture_returns_clean_events() {
        let ((), events) = super::capture(|| {
            tracing::info!(target: "launcher.install", event = "x_finished", x = "ok");
            tracing::info!(target: "launcher.install", event = "x_error", x = "reqwest::Error { kind: Request }");
        });
        assert_eq!(events[0].field("x"), Some("ok"));
        assert_eq!(
            events[1].field("x"),
            Some("reqwest::Error { kind: Request }")
        );
    }
}
