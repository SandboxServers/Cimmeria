//! Filter directives for every log sink, and the routing of OTLP log records
//! between the three SigNoz services.
//!
//! # The parity guarantee (NA25)
//!
//! Every event that reaches an on-disk log file also reaches SigNoz, in
//! exactly one log index:
//!
//! | Level | `service.name` | Filter |
//! |---|---|---|
//! | ERROR, WARN | `cimmeria-server` | [`OTEL_FILTER`] |
//! | INFO, DEBUG | `cimmeria-server`, or `cimmeria-network` for [`otel::is_network_noise_target`] scopes | [`OTEL_FILTER`] |
//! | TRACE | `cimmeria-trace` | [`otel_trace_directives`] |
//!
//! Deliberate exceptions, each pinned by `parity_tests`:
//!
//! - **The `off` targets** in `OTLP_EXCLUDED_TARGETS`, each with its
//!   reason: the exporter's own crates (`hyper`, `h2`, `tonic`, `tower`,
//!   `reqwest`, `opentelemetry`, `tungstenite`), whose export would feed
//!   every batch into the next, and `launcher.key_dump`, which carries a
//!   client session key.
//! - **The per-packet firehoses** (`wire.firehose.*`) reach the files in
//!   full and SigNoz as a counted 1-in-N sample on another target. See
//!   `cimmeria_services::firehose`.
//! - Nothing else. A new file layer whose DEBUG rows [`OTEL_FILTER`] does not
//!   cover fails `every_file_directive_reaches_an_otlp_index`.
//!
//! Hand-named `target: "…"` rows match no file layer, so the file guard
//! cannot see them. `target_scan_tests` reads the source of every in-process
//! crate instead and requires each literal target to reach one index at the
//! level it is emitted (round 2 of NA25).
//!
//! The TRACE filter is *derived* from [`FILE_LAYERS`] rather than written out,
//! so a new file layer's TRACE rows are exported without a second edit. DEBUG
//! and above are not derived: [`OTEL_FILTER`] is also the span filter, and
//! whether a new scope belongs in `cimmeria-server` at DEBUG is a decision.

use cimmeria_services::firehose;
use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::filter::{filter_fn, FilterExt};
use tracing_subscriber::layer::Filter;
use tracing_subscriber::EnvFilter;

use crate::otel;

/// `EnvFilter` directives shared by the OTLP trace layer and the two
/// DEBUG-and-above log layers.
///
/// A custom `target:` that is not named here inherits the leading `info`, so
/// its DEBUG events never reach SigNoz however useful they are.
/// `aoi.create_emit` was built to localize the invisible-static-NPC drop and
/// was absent from the 2026-09-19 repro for exactly that reason.
///
/// A directive's target matches by **string prefix** (`tracing-subscriber`
/// compares `meta.target().starts_with(directive_target)`), and the longest
/// matching directive wins. So `npc_ai=debug` already exports
/// `npc_ai.transition`, `npc_ai.aggro`, `npc_ai.leash`, `npc_ai.tick` and
/// `npc_ai.path_fail`, and `cover=debug` covers every `cover.*` target; but
/// `wire.out=info`
/// needs the more specific `wire.out.avatar_update=debug` beside it to let
/// that one DEBUG sample through. `otel_filter_prefix_matching_exports_npc_ai_children`
/// pins this behaviour, not just the string.
///
/// `mercury.backpressure` is `info`, not `warn` (NA25). Its one emitter is a
/// WARN today, but `server.log` keeps the target from INFO, and the parity
/// rule is that nothing a file keeps is missing from SigNoz; `warn` here
/// would silently drop the first INFO row anyone adds.
///
/// `wire.firehose=debug` is for the same reason: the firehose rows are
/// TRACE, and the TRACE filter turns them off, but a DEBUG row ever emitted
/// on one of those targets reaches its file and so must reach SigNoz.
///
/// `mercury.lossy_transport` (NA37 round 2) is the network-chaos test
/// apparatus's own drop/latency/jitter log, behind `cimmeria-mercury`'s
/// `test-support` feature — never compiled into a release build, but
/// `target_scan_tests` reads source text regardless of feature gating, so it
/// still needs an explicit level here.
pub(crate) const OTEL_FILTER: &str = "info,\
                cimmeria_services=debug,\
                cimmeria_mercury=debug,\
                mercury.packet=info,\
                mercury.retransmit=info,\
                mercury.backpressure=info,\
                mercury.lossy_transport=debug,\
                wire.in=info,wire.out=info,\
                wire.out.avatar_update=debug,\
                wire.out.forced_position=debug,\
                wire.firehose=debug,\
                aoi.entity_enter=debug,aoi.entity_leave=debug,\
                aoi.create_emit=debug,\
                aoi.introduce=debug,\
                movement.npc=debug,movement.player=debug,\
                movement.navmesh=debug,\
                npc_ai=debug,\
                cover=debug,\
                spawner=debug,\
                content=info,\
                threat=info,\
                auth=info,\
                world_entry=info,\
                vendor=info,mail=info,progression=info,inventory=info,mission=info,\
                abilities=debug,\
                content.resolve=debug,\
                dialog.display=debug,\
                mission.step_context=debug,\
                movement.movement_type=debug,\
                movement.position_sample=debug,\
                movement.validation=debug,\
                player.journal=debug,\
                trade.atomic_swap=debug,\
                console.feedback=debug,\
                client.native=debug,\
                launcher=debug,\
                launcher.key_dump=off,\
                cimmeria_discord=debug,\
                sqlx::query=debug,\
                tungstenite=off,tokio_tungstenite=off,hyper=off,\
                h2=off,tower=off,tonic=off,reqwest=off,opentelemetry=off";

/// Every target [`OTEL_FILTER`] turns `off`, with the reason. These are the
/// only rows the server logs that SigNoz never receives.
///
/// Test-only: nothing routes on it. `parity_tests` checks it equals the `off`
/// set in [`OTEL_FILTER`], and exempts exactly these from the file-parity
/// and source-target guards.
#[cfg(test)]
pub(crate) const OTLP_EXCLUDED_TARGETS: &[(&str, &str)] = &[
    ("tungstenite", EXPORTER_TRANSPORT),
    ("tokio_tungstenite", EXPORTER_TRANSPORT),
    ("hyper", EXPORTER_TRANSPORT),
    ("h2", EXPORTER_TRANSPORT),
    ("tower", EXPORTER_TRANSPORT),
    ("tonic", EXPORTER_TRANSPORT),
    ("reqwest", EXPORTER_TRANSPORT),
    ("opentelemetry", EXPORTER_TRANSPORT),
    (
        "launcher.key_dump",
        "carries the client's session key (`key_b64`); logged at DEBUG only so \
         the default sinks keep it off disk, and it must not leave the host \
         through the exporter either",
    ),
];

/// Reason shared by the exporter's own crates in [`OTLP_EXCLUDED_TARGETS`].
#[cfg(test)]
const EXPORTER_TRANSPORT: &str = "the OTLP exporter's own transport (or the admin \
     WebSocket's): exporting it loops every batch's HTTP/gRPC chatter into the next batch";

/// The per-packet wire stream belongs in protocol.log + OTLP at full
/// fidelity, not in human-facing sinks (console, server.log, admin WS). Mute
/// it in each.
pub(crate) const WIRE_FIREHOSE_MUTED: &str =
    "mercury.packet=warn,wire.in=warn,wire.out=warn,mercury.retransmit=warn";

/// `server.log` (JSON, every module, INFO and above).
pub(crate) fn server_log_directives() -> String {
    format!("info,{WIRE_FIREHOSE_MUTED}")
}

/// One per-system `logs/<file>` layer.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FileLayer {
    pub(crate) file: &'static str,
    pub(crate) directives: &'static str,
}

/// Every per-system log file and what it receives. `init_logging` builds one
/// layer per row and the parity test walks the same rows, so a file cannot be
/// added without the test seeing it.
///
/// Each row starts `off` and names module paths, which is why the
/// `wire.firehose.*` targets are listed explicitly: moving a row off its
/// module-path target would otherwise drop it from its file.
pub(crate) const FILE_LAYERS: &[FileLayer] = &[
    FileLayer {
        file: "auth.log",
        directives: "off,cimmeria_services::auth=trace",
    },
    FileLayer {
        file: "base.log",
        directives: "off,\
             cimmeria_services::base::service=trace,\
             cimmeria_services::base::connect_loop=trace,\
             cimmeria_services::base::login=trace,\
             cimmeria_services::base::tick_sync=trace,\
             cimmeria_services::base::helpers=trace,\
             wire.firehose.decrypt=trace,\
             wire.firehose.udp_in=trace",
    },
    FileLayer {
        file: "world_entry.log",
        directives: "off,\
             cimmeria_services::base::world_entry=trace,\
             cimmeria_services::base::world_entry_player=trace,\
             cimmeria_services::base::world_entry_appearance=trace,\
             wire.firehose.aoi_position=trace",
    },
    FileLayer {
        file: "character.log",
        directives: "off,\
             cimmeria_services::base::character=trace,\
             cimmeria_services::base::character_create=trace,\
             cimmeria_services::base::chardef=trace,\
             cimmeria_services::base::cooked_data=trace,\
             cimmeria_services::base::resources=trace",
    },
    FileLayer {
        file: "protocol.log",
        directives: "off,\
             cimmeria_services::mercury=trace,\
             cimmeria_mercury=trace,\
             mercury.packet=info,\
             wire.in=info,wire.out=info,\
             mercury.retransmit=info",
    },
    FileLayer {
        file: "aoi.log",
        directives: "off,\
             cimmeria_services::cell::service=trace,\
             cimmeria_services::cell::space_manager=trace",
    },
    FileLayer {
        file: "combat.log",
        directives: "off,\
             cimmeria_services::cell::combat=trace,\
             cimmeria_services::cell::abilities=trace",
    },
    FileLayer {
        file: "content.log",
        directives: "off,cimmeria_services::cell::content=trace",
    },
    FileLayer {
        file: "missions.log",
        directives: "off,cimmeria_services::cell::missions=trace",
    },
    FileLayer {
        file: "interactions.log",
        directives: "off,\
             cimmeria_services::cell::interactions=trace,\
             cimmeria_services::cell::chat=trace,\
             cimmeria_services::cell::mail=trace",
    },
    FileLayer {
        file: "spawner.log",
        directives: "off,\
             cimmeria_services::cell::spawner=trace,\
             cimmeria_services::cell::gate_travel=trace,\
             cimmeria_services::cell::ring_transport=trace",
    },
    FileLayer {
        file: "dispatch.log",
        directives: "off,\
             cimmeria_services::cell::dispatch=trace,\
             cimmeria_services::base::dispatch=trace",
    },
];

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
fn is_custom_target(target: &str) -> bool {
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
    level <= Level::DEBUG && (!otel::is_network_noise_target(meta.target()) || level <= Level::WARN)
}

/// `cimmeria-network`: INFO and DEBUG from network-noise scopes.
pub(crate) fn routes_to_network(meta: &Metadata<'_>) -> bool {
    let level = *meta.level();
    level <= Level::DEBUG && level > Level::WARN && otel::is_network_noise_target(meta.target())
}

/// `cimmeria-trace`: TRACE only, from every scope. The other two indexes
/// reject TRACE, so no record is indexed twice.
pub(crate) fn routes_to_trace(meta: &Metadata<'_>) -> bool {
    *meta.level() == Level::TRACE
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

/// [`otel_trace_log_filter`] over an arbitrary file-layer table.
#[cfg(test)]
pub(crate) fn otel_trace_log_filter_for<S: Subscriber>(
    file_layers: &[FileLayer],
) -> impl Filter<S> + Send + Sync + 'static {
    EnvFilter::new(otel_trace_directives_for(file_layers)).and(filter_fn(routes_to_trace))
}
