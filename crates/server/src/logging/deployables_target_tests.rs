//! Deployables (Phase 0) log targets reach SigNoz at the level they emit.
//!
//! `deployables.lifecycle` (INFO spawn and despawn, DEBUG refusals, WARN
//! failures) and `deployables.pulse` (DEBUG per pulse) ride the one
//! `deployables=debug` prefix row in [`OTEL_FILTER`]. Remove the row and the
//! DEBUG refusals and pulse rows silently stop reaching the exporter.

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Layer};

use super::filters::OTEL_FILTER;

/// Records the target and level of every event the filter lets through.
struct Seen(std::sync::Arc<std::sync::Mutex<Vec<(String, tracing::Level)>>>);

impl<S: tracing::Subscriber> Layer<S> for Seen {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let m = event.metadata();
        self.0
            .lock()
            .unwrap()
            .push((m.target().to_string(), *m.level()));
    }
}

#[test]
fn otel_filter_exports_deployables_debug_rows() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry()
        .with(Seen(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: "deployables.lifecycle", event = "deploy_refused", "r");
        tracing::info!(target: "deployables.lifecycle", event = "spawned", "s");
        tracing::debug!(target: "deployables.pulse", event = "pulse", "p");
    });
    assert_eq!(
        *seen.lock().unwrap(),
        [
            ("deployables.lifecycle".to_string(), tracing::Level::DEBUG),
            ("deployables.lifecycle".to_string(), tracing::Level::INFO),
            ("deployables.pulse".to_string(), tracing::Level::DEBUG),
        ],
        "OTEL_FILTER must export the deployables targets at DEBUG (the `deployables=debug` row)"
    );
}
