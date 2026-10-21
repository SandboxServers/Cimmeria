//! Test-only tracing capture (launcher-telemetry C9). Thread-local: run async code
//! inside `capture` on a current-thread runtime.
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
    (r, v)
}
