//! Guard for the `minigame.connection` span (TG-MG-06): every row a
//! connection's task logs, down to a framing send error, sits under that
//! span, and the span names the player once the login has.
//!
//! `LogCapture` records spans and events side by side but not which span an
//! event was logged in, and that parenthood is the whole point here. So this
//! file carries its own small layer that snapshots, for each event, the
//! names and fields of every span in its scope.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Registry;

use super::listener_tests::{connect_to, send_handshake};
use super::tests::read_until_game_begin;
use super::*;

type Fields = HashMap<String, String>;

/// One captured event: its message and, root first, each span in its scope
/// as `(name, fields at the time of the event)`.
#[derive(Debug, Clone)]
struct Row {
    message: String,
    scope: Vec<(String, Fields)>,
}

impl Row {
    /// The fields of the span named `name` in this row's scope, if it is
    /// there.
    fn span(&self, name: &str) -> Option<&Fields> {
        self.scope.iter().find(|(n, _)| n == name).map(|(_, f)| f)
    }
}

#[derive(Default)]
struct FieldVisitor(Fields);

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

#[derive(Clone, Default)]
struct ScopeCapture {
    rows: Arc<Mutex<Vec<Row>>>,
    spans: Arc<Mutex<HashMap<u64, Fields>>>,
}

impl<S> Layer<S> for ScopeCapture
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);
        self.spans.lock().unwrap().insert(id.into_u64(), visitor.0);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);
        self.spans
            .lock()
            .unwrap()
            .entry(id.into_u64())
            .or_default()
            .extend(visitor.0);
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let spans = self.spans.lock().unwrap();
        let scope = ctx
            .event_scope(event)
            .map(|scope| {
                scope
                    .from_root()
                    .map(|span| {
                        let fields = spans.get(&span.id().into_u64()).cloned();
                        (span.name().to_string(), fields.unwrap_or_default())
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.rows.lock().unwrap().push(Row {
            message: visitor.0.remove("message").unwrap_or_default(),
            scope,
        });
    }
}

impl ScopeCapture {
    /// Install as this thread's default subscriber. Keep the guard alive
    /// for the test.
    fn install(&self) -> tracing::subscriber::DefaultGuard {
        // An always-interested global dispatcher, as `LogCapture` keeps, so a
        // callsite registered while no capture was alive (under plain
        // `cargo test`) is not cached as `never`. Fails harmlessly when one
        // is already set.
        let _ = tracing::subscriber::set_global_default(Registry::default());
        let guard = tracing::subscriber::set_default(Registry::default().with(self.clone()));
        tracing::callsite::rebuild_interest_cache();
        guard
    }

    fn find(&self, message: &str) -> Option<Row> {
        self.rows
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.message.contains(message))
            .cloned()
    }

    fn all(&self) -> Vec<Row> {
        self.rows.lock().unwrap().clone()
    }
}

/// Assert `row` was logged inside a `minigame.connection` span that names
/// entity 4330, player 71 ("Tess") and the `Hack` game.
fn assert_in_connection_span(row: &Row) {
    let span = row.span("minigame.connection").unwrap_or_else(|| {
        panic!(
            "{:?} has no minigame.connection parent: {row:#?}",
            row.message
        )
    });
    for (key, value) in [
        ("entity_id", "4330"),
        ("entity_name", "Tess"),
        ("player_id", "71"),
        ("player_name", "Tess"),
        ("game", "Hack"),
    ] {
        assert_eq!(
            span.get(key).map(String::as_str),
            Some(value),
            "the connection span must carry {key} after login: {row:#?}",
        );
    }
    assert!(
        span.contains_key("peer"),
        "the span opens with the peer address: {row:#?}"
    );
}

/// A player logs in, reaches the game loop, then the socket is reset. The
/// teardown's next write fails, and the framing layer's `Minigame send
/// error` row must sit under the connection span with the player filled in;
/// so must the closing `minigame.session_end` row.
///
/// Fails without the `.instrument(span)` on the connection (no parent
/// span) and without the post-login `record` calls (empty fields).
#[tokio::test]
async fn a_send_error_row_sits_under_the_connection_span() {
    let capture = ScopeCapture::default();
    let _guard = capture.install();

    let registry = SessionRegistry::new();
    let ticket = registry
        .register(
            4330,
            71,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            Some("Tess".into()),
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .expect("fresh registry must accept the session");

    let (mut client, _rx, handle) = connect_to(registry, ListenerLimits::default()).await;
    send_handshake(&mut client, 4330, &ticket).await;
    read_until_game_begin(&mut client).await;

    // SO_LINGER=0 turns the close into an RST, so the server's read fails
    // and every later write errors: the teardown's first send is the
    // framing send error this test needs (a FIN would let it succeed).
    socket2::SockRef::from(&client)
        .set_linger(Some(Duration::ZERO))
        .expect("SO_LINGER must be settable on a loopback TCP socket");
    drop(client);

    tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("a reset connection must end")
        .expect("connection task must not panic");

    let send_error = capture
        .find("Minigame send error")
        .unwrap_or_else(|| panic!("no framing send-error row: {:#?}", capture.all()));
    assert_in_connection_span(&send_error);

    let end = capture
        .find("Minigame session ended")
        .unwrap_or_else(|| panic!("no session_end row: {:#?}", capture.all()));
    assert_in_connection_span(&end);
}
