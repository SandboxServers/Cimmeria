//! Unit tests for `otel.rs`: the opt-in `init()`, the identity attributes,
//! the baked build SHA and the network-noise routing predicate. A file of its
//! own because the predicate's pins grew with every wave of the services
//! crate split (each moved module path is pinned as noise or not).

use super::*;
use std::sync::Mutex;

// OTEL env-var reads contend on a single process-global state, so
// serialise the test cases that touch them. `unwrap_or_else` on
// PoisonError keeps a panicking test from cascading into the next.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Without `OTEL_EXPORTER_OTLP_ENDPOINT`, `init()` must return
/// `None` rather than failing — telemetry is opt-in.
///
/// Note: we intentionally do NOT have a paired "with endpoint set,
/// init returns Some" test. The OTLP exporter builder (tonic-based)
/// needs a live tokio runtime at construction time; in a sync test
/// without `#[tokio::test]` the builder panics inside hyper-util.
/// The realistic init path is exercised by booting cimmeria-server
/// with `OTEL_EXPORTER_OTLP_ENDPOINT` set against a live SigNoz
/// (smoke test, not unit).
#[test]
fn init_returns_none_when_endpoint_unset() {
    let _lock = ENV_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
    assert!(init().is_none(), "no endpoint → no layer");
}

/// Network-noise routing predicate — pinned for the routing logic
/// in `main.rs`. Add to [`is_network_noise_target`] when a new
/// high-volume scope appears, then add it here.
///
/// A regression that broadens this predicate (e.g., starts matching
/// `cimmeria_services::base::*`) would silently route auth/world-
/// entry/content events into the network index, hiding them from
/// the operator's primary triage view. Pin every accepted prefix
/// AND a few high-signal scopes that MUST remain in cimmeria-server.
#[test]
fn is_network_noise_target_matches_explicit_wire_scopes() {
    // Accepted (route to cimmeria-network):
    assert!(is_network_noise_target("mercury.packet"));
    assert!(is_network_noise_target("mercury.retransmit"));
    assert!(is_network_noise_target("mercury.backpressure"));
    assert!(is_network_noise_target(
        "cimmeria_base::base::connect_loop::encrypted"
    ));
    assert!(is_network_noise_target(
        "cimmeria_base::base::connect_loop::cell_arms"
    ));
    assert!(is_network_noise_target(
        "cimmeria_base_session::base::tick_sync"
    ));
    assert!(is_network_noise_target("cimmeria_mercury::session"));
}

/// The identity attributes the SigNoz runbook filters on. Fails if one
/// is dropped or renamed (`cimmeria.deploy_env='colo'` is the first
/// clause of every NPC-AI query in telemetry.md §3).
#[test]
fn identity_attributes_carry_env_host_and_version() {
    let attrs = identity_attributes("colo", "box-7", "0123abcd");
    let get = |k: &str| {
        attrs
            .iter()
            .find(|kv| kv.key.as_str() == k)
            .map(|kv| kv.value.to_string())
    };
    assert_eq!(get("deployment.environment").as_deref(), Some("colo"));
    assert_eq!(get("cimmeria.deploy_env").as_deref(), Some("colo"));
    assert_eq!(get("host.name").as_deref(), Some("box-7"));
    assert_eq!(get("service.version").as_deref(), Some("0123abcd"));
    assert_eq!(attrs.len(), 4);
}

/// The baked SHA is never empty: a hex commit or the literal "unknown".
#[test]
fn build_sha_is_a_commit_or_unknown() {
    assert!(
        BUILD_SHA == "unknown"
            || (BUILD_SHA.len() >= 7 && BUILD_SHA.chars().all(|c| c.is_ascii_hexdigit())),
        "unexpected CIMMERIA_BUILD_SHA {BUILD_SHA:?}"
    );
    assert!(!host_name().is_empty());
}

#[test]
fn is_network_noise_target_does_not_match_high_signal_scopes() {
    // Rejected (stay in cimmeria-server):
    assert!(!is_network_noise_target("cimmeria_auth::auth::handlers"));
    // Combat and the NPC AI's behaviour, in cimmeria-server since before
    // wave C2 moved them from `cimmeria_services::cell` to
    // `cimmeria-cell-combat`.
    assert!(!is_network_noise_target(
        "cimmeria_cell_combat::cell::abilities::use_ability"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_combat::cell::service::npc_ai::dispatch"
    ));
    // The content layer, `cimmeria_services::cell::{content, missions,
    // ring_transport}` in cimmeria-server until wave C3 moved it to
    // cimmeria-cell-content.
    assert!(!is_network_noise_target(
        "cimmeria_cell_content::cell::content::executor::dialog"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_content::cell::missions::progression"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_content::cell::ring_transport::dispatch"
    ));
    // The GM surfaces, `cimmeria_services::cell::console` (with chat and
    // the GM handlers) in cimmeria-server until wave C5b moved them to
    // cimmeria-cell-console.
    assert!(!is_network_noise_target(
        "cimmeria_cell_console::cell::console::dispatch"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_console::cell::console::chat"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_console::cell::console::gm::world"
    ));
    // The player interactions, `cimmeria_services::cell::{interactions,
    // gate_travel, respawn, ...}` in cimmeria-server until wave C4 moved
    // them to cimmeria-cell-interactions.
    assert!(!is_network_noise_target(
        "cimmeria_cell_interactions::cell::interactions::dispatch::interact"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_interactions::cell::gate_travel"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_interactions::cell::respawn::resync"
    ));
    // The client-callable cell methods, `cimmeria_services::cell::cell_methods`
    // in cimmeria-server until wave C5a moved them to cimmeria-cell-methods.
    assert!(!is_network_noise_target(
        "cimmeria_cell_methods::cell::cell_methods::player::combat"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell_methods::cell::cell_methods::inventory::item_ops"
    ));
    // The cell service and the cell-method router,
    // `cimmeria_services::cell::{service, dispatch}` in cimmeria-server
    // until wave C6 moved them to cimmeria-cell. The loop logs per message
    // and per tick, the router per cell-method call, not per datagram.
    assert!(!is_network_noise_target(
        "cimmeria_cell::cell::service::message_loop"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell::cell::service::ticks::npc_movement"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_cell::cell::dispatch::router"
    ));
    // The feature handlers, `cimmeria_services::base::world_entry::methods`
    // in cimmeria-server until wave B2 moved them to cimmeria-base-methods.
    assert!(!is_network_noise_target(
        "cimmeria_base_methods::base::world_entry::methods::inventory::grant"
    ));
    // World entry, the CellToBase dispatch and the character list were
    // `cimmeria_services::base::{world_entry, world_entry_appearance,
    // character}` in cimmeria-server until wave B3 moved them to
    // cimmeria-base-world-entry. The AoI dispatch runs once per AoI event,
    // not per datagram, and has always been a cimmeria-server row.
    assert!(!is_network_noise_target(
        "cimmeria_base_world_entry::base::world_entry::cell_dispatch::aoi_dispatch"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_base_world_entry::base::world_entry_appearance::cinematic_aoi_hold"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_base_world_entry::base::character"
    ));
    // The rest of the connect loop, login, the base-method dispatch and the
    // service were `cimmeria_services::base::…` in cimmeria-server until
    // wave B4 moved them to cimmeria-base; only the two per-packet arms
    // above are noise.
    assert!(!is_network_noise_target("cimmeria_base::base::dispatch"));
    assert!(!is_network_noise_target(
        "cimmeria_base::base::connect_loop"
    ));
    assert!(!is_network_noise_target("cimmeria_base::base::login"));
    assert!(!is_network_noise_target("cimmeria_base::base::service"));
    // Only tick sync of the session crate is noise: its send helpers, outbox
    // and contact list stay in cimmeria-server, as they did before wave B1.
    assert!(!is_network_noise_target(
        "cimmeria_base_session::base::helpers"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_base_session::base::outbox"
    ));
    // The services-side Mercury glue is not the `cimmeria-mercury`
    // transport crate: it was `cimmeria_services::mercury` in
    // cimmeria-server until wave W3a moved it to `cimmeria-wire`, and a
    // prefix grown to catch every `::mercury` module must not take it.
    assert!(!is_network_noise_target("cimmeria_wire::mercury"));
    assert!(!is_network_noise_target(
        "cimmeria_wire::mercury::world_data::map_loaded"
    ));
    // The decoded wire-message stream (one row per message, not per
    // datagram) has always been `cimmeria-server`; wave W3b moved its
    // code from `cimmeria_services::wire_log` to `cimmeria-wire-log`.
    assert!(!is_network_noise_target("wire.in"));
    assert!(!is_network_noise_target("wire.out"));
    assert!(!is_network_noise_target("cimmeria_wire_log::wire_log::tap"));
    // The SmartFoxServer host logs per minigame session, not per packet,
    // and stayed in cimmeria-server when wave W3c moved it from
    // `cimmeria_services::minigame` to `cimmeria-minigame`.
    assert!(!is_network_noise_target(
        "cimmeria_minigame::minigame::server"
    ));
    assert!(!is_network_noise_target(
        "cimmeria_minigame::minigame::server::framing"
    ));
    // Empty / arbitrary string — defaults to "not noise" (server).
    assert!(!is_network_noise_target(""));
    assert!(!is_network_noise_target("unknown"));
}
