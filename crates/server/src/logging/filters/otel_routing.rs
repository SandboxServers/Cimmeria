//! Which OTLP log index each record goes to, and the TRACE directives
//! derived from the file layers. Split from `filters.rs` (FZ-16), which
//! keeps the directive tables these functions read.

use cimmeria_services::firehose;
use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::filter::{filter_fn, FilterExt};
use tracing_subscriber::layer::Filter;
use tracing_subscriber::EnvFilter;

use super::{FileLayer, FILE_LAYERS, OTEL_FILTER};
use crate::otel;

/// Targets exported at TRACE that no file names: the firehose samples.
pub(crate) const TRACE_ONLY_TARGETS: &[&str] = &[firehose::SAMPLED_TARGET_PREFIX];

/// `(target, level)` for every `target=level` directive; bare levels such as
/// the leading `off` are skipped.
pub(crate) fn directive_pairs(directives: &str) -> impl Iterator<Item = (&str, &str)> {
    directives
        .split(',')
        .map(str::trim)
        .filter_map(|d| d.split_once('='))
}

/// A hand-named target (`npc_ai`, `wire.out`, `movement.navmesh`) rather than
/// a Rust module path. For module paths the file layers are the authority on
/// what is kept at TRACE, so [`OTEL_FILTER`]'s blanket `cimmeria_services=debug`
/// does not become `cimmeria_services=trace`.
///
/// Every workspace crate linked into the server is named `cimmeria_*`, so a
/// target this returns `false` for is a module path or an external crate's
/// path; `stale_target_tests` checks the former exist.
pub(crate) fn is_custom_target(target: &str) -> bool {
    !target.contains("::") && !target.starts_with("cimmeria_")
}

/// Directives for the `cimmeria-trace` log index: every target a file layer
/// keeps at TRACE, every custom target [`OTEL_FILTER`] names, and the firehose
/// samples; with `wire.firehose` off.
///
/// Every directive is `=trace` (or `off`), so the longest-match rule reduces
/// to "any of them matches", which is the OR over the file layers this
/// stands for. The firehose rows are skipped rather than listed and
/// overridden, because `wire.firehose.decrypt=trace` is longer than, and
/// would beat, `wire.firehose=off`.
pub(crate) fn otel_trace_directives() -> String {
    otel_trace_directives_for(FILE_LAYERS)
}

/// [`otel_trace_directives`] over an arbitrary file-layer table, so the
/// parity test can prove the guard catches a row the real table lacks.
pub(crate) fn otel_trace_directives_for(file_layers: &[FileLayer]) -> String {
    let file_targets = file_layers
        .iter()
        .flat_map(|l| directive_pairs(l.directives))
        .filter(|(_, level)| level.eq_ignore_ascii_case("trace"))
        .map(|(target, _)| target);
    let custom_targets = directive_pairs(OTEL_FILTER)
        .filter(|(target, level)| !level.eq_ignore_ascii_case("off") && is_custom_target(target))
        .map(|(target, _)| target);

    let mut targets: Vec<&str> = file_targets
        .chain(custom_targets)
        .chain(TRACE_ONLY_TARGETS.iter().copied())
        .filter(|t| !t.starts_with(firehose::FIREHOSE_TARGET_PREFIX))
        .collect();
    targets.sort_unstable();
    targets.dedup();

    let mut out = String::from("off");
    for t in targets {
        out.push(',');
        out.push_str(t);
        out.push_str("=trace");
    }
    out.push(',');
    out.push_str(firehose::FIREHOSE_TARGET_PREFIX);
    out.push_str("=off");
    // A child `OTEL_FILTER` turns off (`launcher.key_dump`) stays off here
    // too, although its parent (`launcher`) was raised to TRACE above.
    for (target, _) in
        directive_pairs(OTEL_FILTER).filter(|(_, level)| level.eq_ignore_ascii_case("off"))
    {
        out.push(',');
        out.push_str(target);
        out.push_str("=off");
    }
    out
}

/// `cimmeria-server`: DEBUG and above, except DEBUG/INFO from network-noise
/// scopes. WARN+ from those scopes still lands here so a real wire problem
/// shows in the primary view.
pub(crate) fn routes_to_server(meta: &Metadata<'_>) -> bool {
    let level = *meta.level();
    // `Level` orders by verbosity: `<= DEBUG` is DEBUG or more severe.
    level <= Level::DEBUG
        && !otel::is_client_target(meta.target())
        && (!otel::is_network_noise_target(meta.target()) || level <= Level::WARN)
}

/// `cimmeria-network`: INFO and DEBUG from network-noise scopes.
pub(crate) fn routes_to_network(meta: &Metadata<'_>) -> bool {
    let level = *meta.level();
    level <= Level::DEBUG && level > Level::WARN && otel::is_network_noise_target(meta.target())
}

/// `cimmeria-trace`: TRACE only, from every server scope. The other two
/// server indexes reject TRACE, so no record is indexed twice.
pub(crate) fn routes_to_trace(meta: &Metadata<'_>) -> bool {
    *meta.level() == Level::TRACE && !otel::is_client_target(meta.target())
}

/// `cimmeria-client`: the ingest's client-side replays, at every level the
/// client chose to send. The client already throttled and paid the upload
/// for a TRACE row, so dropping it here would only waste it; and a client
/// WARN is not a server problem, so it does not go to `cimmeria-server`.
pub(crate) fn routes_to_client(meta: &Metadata<'_>) -> bool {
    otel::is_client_target(meta.target())
}

/// Filter for the `cimmeria-server` log layer.
pub(crate) fn otel_server_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(OTEL_FILTER).and(filter_fn(routes_to_server))
}

/// Filter for the `cimmeria-network` log layer.
pub(crate) fn otel_network_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(OTEL_FILTER).and(filter_fn(routes_to_network))
}

/// Filter for the `cimmeria-trace` log layer.
pub(crate) fn otel_trace_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(otel_trace_directives()).and(filter_fn(routes_to_trace))
}

/// Filter for the `cimmeria-client` log layer. No `EnvFilter`: the target
/// list in [`otel::CLIENT_TARGETS`] is the whole rule, at every level.
pub(crate) fn otel_client_log_filter<S: Subscriber>() -> impl Filter<S> + Send + Sync + 'static {
    filter_fn(routes_to_client)
}

/// [`otel_trace_log_filter`] over an arbitrary file-layer table.
#[cfg(test)]
pub(crate) fn otel_trace_log_filter_for<S: Subscriber>(
    file_layers: &[FileLayer],
) -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(otel_trace_directives_for(file_layers)).and(filter_fn(routes_to_trace))
}
