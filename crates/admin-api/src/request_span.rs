//! The per-request tracing span, shared by the admin router
//! ([`crate::build_router`]) and the login-port telemetry router
//! ([`crate::login_port_telemetry_router`]).

use axum::body::Body;
use axum::http::Request;
use tracing::Span;

/// The span around one request: tower-http's `DefaultMakeSpan` at INFO
/// (name `request`; fields `method`, `uri`, `version`), except that `uri`
/// is the path alone.
///
/// The default records the whole request target. Every row a handler
/// writes sits inside this span, and the console and `server.log` layers
/// print span fields on every event line, so the default would put two
/// pieces of caller-chosen text beside each of the 161 rows one
/// launcher-summary request can write:
///
/// - a query string, up to 64 KiB of it;
/// - the scheme and authority of an absolute-form target
///   (`POST http://host/api/… HTTP/1.1`), which is served like the bare
///   path.
///
/// The login port is public, and the admin port is reachable by anyone who
/// can route TCP to it (#439), so both routers use this span. No route on
/// the login port reads a query string. Some admin routes do (the login
/// audit list, for one); their filters are no longer in the span, and a
/// handler that wants one in its logs records it itself.
///
/// The method and the path are still the caller's. A request whose path or
/// method matches no route gets a 404 or 405 and writes no rows, so that
/// text appears once per request, not once per row.
///
/// The target is spelled out as tower-http's own. The OTLP filter turns
/// `tower` off (`OTEL_FILTER` in `crates/server/src/logging/filters.rs`), so
/// tower-http's span is not exported; under this module's path the span
/// would start being exported, and the console and file layers would file
/// it under a different target than before.
pub(crate) fn request_span(request: &Request<Body>) -> Span {
    tracing::info_span!(
        target: "tower_http::trace::make_span",
        "request",
        method = %request.method(),
        uri = %request.uri().path(),
        version = ?request.version(),
    )
}

/// The recorder the two routers' request-span tests share.
#[cfg(test)]
pub(crate) mod recorder {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};

    /// Everything a subscriber is shown, as `name=value` lines: each span's
    /// name, target and fields (at creation and when recorded later) and
    /// each event's fields. The capture layers elsewhere in this crate record
    /// event fields only, which is why they could not see what a request
    /// span holds.
    #[derive(Clone, Default)]
    pub(crate) struct Shown(Arc<Mutex<Vec<String>>>);

    struct Lines<'a>(&'a mut Vec<String>);

    impl Visit for Lines<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.push(format!("{}={value:?}", field.name()));
        }
        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push(format!("{}={value}", field.name()));
        }
    }

    impl<S: Subscriber> Layer<S> for Shown {
        fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
            let mut lines = self.0.lock().unwrap();
            let metadata = attrs.metadata();
            lines.push(format!(
                "span={} target={}",
                metadata.name(),
                metadata.target()
            ));
            attrs.record(&mut Lines(&mut lines));
        }
        fn on_record(&self, _: &Id, values: &Record<'_>, _: Context<'_, S>) {
            values.record(&mut Lines(&mut self.0.lock().unwrap()));
        }
        fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
            event.record(&mut Lines(&mut self.0.lock().unwrap()));
        }
    }

    impl Shown {
        pub(crate) fn lines(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }
}
