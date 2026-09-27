//! Pets (#570) log targets reach SigNoz at the level they emit.
//!
//! The pets targets (`pets.lifecycle`, `pets.command`, `pets.ai`,
//! `pets.credit`, `pets.buff`) ride the one `pets=debug` prefix row in [`OTEL_FILTER`]
//! instead of a directive each. This pins that choice behaviourally: remove
//! the row, or let `tracing-subscriber` stop prefix-matching, and a pet's
//! DEBUG rows silently stop reaching the exporter.

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

/// PT-08: `pets.buff` exports its DEBUG `buff_applied` /
/// `owner_ability_refused` rows, its INFO `doom_fired` and its WARN
/// `owner_ability_refused reason=owner_identity_mismatch`.
#[test]
fn otel_filter_exports_pets_buff_debug_info_and_warn() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry()
        .with(Seen(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: "pets.buff", event = "buff_applied", "a");
        tracing::info!(target: "pets.buff", event = "doom_fired", "d");
        tracing::warn!(target: "pets.buff", reason = "owner_identity_mismatch", "w");
    });
    assert_eq!(
        *seen.lock().unwrap(),
        [
            ("pets.buff".to_string(), tracing::Level::DEBUG),
            ("pets.buff".to_string(), tracing::Level::INFO),
            ("pets.buff".to_string(), tracing::Level::WARN),
        ],
        "OTEL_FILTER must export `pets.buff` at DEBUG (the `pets=debug` row)"
    );
}

/// PT-06: `pets.credit` exports its DEBUG `pet_kill_credited` /
/// `kill_xp_not_granted` rows and its WARN `transfer_xp_invalid` row.
#[test]
fn otel_filter_exports_pets_credit_debug_and_warn() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry()
        .with(Seen(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
    tracing::subscriber::with_default(subscriber, || {
        tracing::debug!(target: "pets.credit", event = "pet_kill_credited", "c");
        tracing::warn!(target: "pets.credit", reason = "transfer_xp_invalid", "w");
    });
    assert_eq!(
        *seen.lock().unwrap(),
        [
            ("pets.credit".to_string(), tracing::Level::DEBUG),
            ("pets.credit".to_string(), tracing::Level::WARN),
        ],
        "OTEL_FILTER must export `pets.credit` at DEBUG (the `pets=debug` row)"
    );
}
