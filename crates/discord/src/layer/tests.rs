//! `DiscordLayer` harvest, filter and field-visitor tests.

use super::*;
use crate::config::{ChannelConfig, EventToggles};
use crate::event::{ChannelKind, EventKind};
use crate::sender::{spawn, MockSender};
use std::collections::HashMap;
use std::time::Duration;

fn errors_only_config(warning: bool, error: bool) -> Arc<ArcSwap<Config>> {
    let mut channels = HashMap::new();
    channels.insert(
        ChannelKind::Errors,
        ChannelConfig {
            url: "https://discord.com/api/webhooks/1/errors".into(),
            rate_limit_per_min: 60,
        },
    );
    let events = EventToggles {
        warning,
        error,
        ..EventToggles::default()
    };
    let cfg = Config {
        enabled: true,
        username: None,
        avatar_url: None,
        channels,
        events,
        muted_accounts: Vec::new(),
    };
    Arc::new(ArcSwap::new(Arc::new(cfg)))
}

/// Bug shape: a `warn!` event from non-Discord code lands in the
/// errors channel. Pin both directions: warn enabled → fires, warn
/// disabled → doesn't fire.
#[tokio::test(flavor = "current_thread")]
async fn warn_event_routed_when_enabled() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(/* warning */ true, /* error */ true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::warn!(reason = "test", entity_id = 42, "something happened");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recorded = calls.lock().await;
    assert_eq!(recorded.len(), 1, "warn must produce one Discord post");
    let (url, body) = &recorded[0];
    assert!(url.ends_with("/errors"), "must route to errors channel");
    let serialized = body.to_string();
    assert!(serialized.contains("something happened"), "message body");
    assert!(serialized.contains("reason"), "field key in fields list");
}

/// Same emit but with `warning = false` → no post.
#[tokio::test(flavor = "current_thread")]
async fn warn_event_filtered_when_disabled() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(/* warning */ false, /* error */ true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::warn!("nope");
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(
        calls.lock().await.len(),
        0,
        "warn must not post when disabled"
    );
}

/// Recursion guard — events from `cimmeria_discord` must NOT round-
/// trip back through the layer. A regression here is an infinite
/// loop on the first send failure.
#[tokio::test(flavor = "current_thread")]
async fn discord_self_target_filtered() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::warn!(target: "cimmeria_discord", "should not post");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recorded: Vec<_> = calls.lock().await.clone();
    assert!(
        recorded.is_empty(),
        "events tagged `target: cimmeria_discord` must be filtered to prevent recursion, got {:?}",
        recorded
    );
}

/// `movement.validation` warns (speed_warning / validation_reject) are
/// calibration telemetry for SigNoz and must NOT reach Discord — they
/// fire on normal play and would flood the errors channel. Reverting the
/// target filter trips this.
#[tokio::test(flavor = "current_thread")]
async fn movement_validation_target_filtered() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(/* warning */ true, /* error */ true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::warn!(
        target: "movement.validation",
        entity_id = 2,
        reason = "speed",
        "movement.speed_warning: client move exceeded speed tolerance"
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(
        calls.lock().await.is_empty(),
        "movement.validation warns must never post to Discord (calibration noise)"
    );
}

/// Client telemetry replayed by the ingest never posts, at any level;
/// the server's own ingest records still do. Reverting the target
/// filter trips this.
#[tokio::test(flavor = "current_thread")]
async fn client_telemetry_replays_are_filtered() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::error!(target: "client.native", client_target = "client.mercury.error", "client.mercury.error");
    tracing::warn!(target: "client.native", client_target = "client.ui.cegui_log", "client.ui.cegui_log");
    tracing::error!(target: "launcher.client_log", "client log line");
    tracing::error!(target: "launcher.debug_log", "debug log line");
    tracing::warn!(target: "launcher.session_meta", "session meta");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        calls.lock().await.is_empty(),
        "client telemetry replays must never post to Discord"
    );

    tracing::error!(target: "launcher.ingest", reason = "session_over_budget", "ingest refused");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        calls.lock().await.len(),
        1,
        "the server's own ingest errors still post"
    );
}

/// Content-quality events never post; other events on the same target
/// still do. Reverting the `SIGNOZ_ONLY_EVENTS` check (or dropping a
/// row) trips this.
#[tokio::test(flavor = "current_thread")]
async fn signoz_only_events_are_filtered() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    // The owner's example row (colo, 2026-09-29), with its real fields.
    tracing::warn!(
        target: "spawner.npc_behaviour",
        event = "spawn_off_mesh",
        npc_id = 100233u32,
        tag = "DebugHub_Vendor",
        template_id = 300,
        world = "Castle_CellBlock",
        space_id = 65552u32,
        spawn_id = 400,
        on_navmesh = false,
        gate = "horizontal",
        "spawner: NPC spawned outside find_path's +-0.5 start box -- every path              it requests will fail with no_start_poly"
    );
    tracing::warn!(
        target: "abilities",
        event = "effect_script_unregistered",
        reason = "no_registered_script",
        count = 2u64,
        "effect rows name a script no registered script answers"
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(
        calls.lock().await.is_empty(),
        "SIGNOZ_ONLY_EVENTS rows must never post to Discord"
    );

    // Same targets, other events: still ops alerts.
    tracing::warn!(
        target: "spawner.npc_behaviour",
        event = "spawn_failed",
        "spawner: NPC could not be spawned"
    );
    tracing::warn!(target: "abilities", "an abilities warn with no event field");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        calls.lock().await.len(),
        2,
        "the filter is per (target, event), not per target"
    );
}

/// Rows are distinct, and the matcher keys on target and event
/// together: the same event on another target, or no event, posts.
#[test]
fn signoz_only_rows_are_distinct_and_well_formed() {
    let mut seen = std::collections::HashSet::new();
    for &(target, event) in SIGNOZ_ONLY_EVENTS {
        assert!(!target.is_empty() && !event.is_empty());
        assert!(
            seen.insert((target, event)),
            "duplicate row {target}/{event}"
        );
    }
    assert!(is_signoz_only(
        "spawner.npc_behaviour",
        &[("event".into(), "spawn_off_mesh".into())]
    ));
    assert!(!is_signoz_only(
        "npc_ai",
        &[("event".into(), "spawn_off_mesh".into())]
    ));
    assert!(!is_signoz_only("spawner.npc_behaviour", &[]));
}

/// info/debug/trace are deliberately ignored — Discord is for warn+
/// only.
#[tokio::test(flavor = "current_thread")]
async fn info_debug_trace_ignored() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::info!("ignored");
    tracing::debug!("ignored");
    tracing::trace!("ignored");
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(calls.lock().await.len(), 0);
}

/// Error events carry the right EventKind and route to the same
/// channel as warns (both go to `errors`).
#[tokio::test(flavor = "current_thread")]
async fn error_event_routed_to_errors_channel() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::error!(player_id = 100, "DB failure");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recorded = calls.lock().await;
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].0.ends_with("/errors"));
    // The EventKind discriminator is implicit via routing; pin
    // that we're using the right variant by spot-checking the body
    // carries the player, folded into the Who field (NT-11).
    let body = recorded[0].1.to_string();
    assert!(body.contains(r#""name":"Who""#), "{body}");
    assert!(body.contains("#100"), "{body}");
}

/// Fields are recorded in stable order — `reason` and `entity_id`
/// from the negative-logging convention need to show up somewhere
/// predictable in the embed.
#[tokio::test(flavor = "current_thread")]
async fn structured_fields_preserved_in_embed() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::error!(
        reason = "entity_to_addr_miss",
        entity_id = 42u32,
        entity_count_in_map = 17u64,
        "AoI: no client addr -- skipping"
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recorded = calls.lock().await;
    let body = &recorded[0].1;
    let s = body.to_string();
    assert!(s.contains("entity_to_addr_miss"));
    // `entity_id` has no `entity_name` here, so it folds to `#id` under
    // the `entity` label (NT-11).
    assert!(s.contains(r#""name":"entity""#), "{s}");
    assert!(s.contains("#42"), "{s}");
    assert!(s.contains("entity_count_in_map"));
}

/// Sanity: EventKind for the tracing event matches expectation.
#[test]
fn tracing_event_kind_warn_maps_to_warning() {
    assert_eq!(TracingEventKind::Warn.event_kind(), EventKind::Warning);
    assert_eq!(TracingEventKind::Error.event_kind(), EventKind::Error);
}

/// Field-visitor coverage for every primitive type the
/// `FieldCollector` implements. tracing emits structured fields
/// by calling one of `record_str`, `record_i64`, `record_u64`,
/// `record_bool`, `record_f64`, or `record_debug` depending on
/// the value's type. The existing tests only cover string +
/// debug; this pins the numeric and boolean arms so future
/// refactors don't quietly drop a primitive type from the
/// embed's fields list.
///
/// Reverting any `record_*` arm to `unimplemented!()` (or to a
/// `panic!`) trips this immediately because tracing macros
/// dispatch to the visitor by value type at compile time.
#[tokio::test(flavor = "current_thread")]
async fn field_collector_records_all_primitive_types() {
    let mock = MockSender::new();
    let calls = mock.calls_handle();
    let cfg = errors_only_config(true, true);
    let (handle, _task) = spawn(mock, cfg.clone());

    let subscriber = tracing_subscriber::registry().with(DiscordLayer::new(handle, cfg));
    let _guard = tracing::subscriber::set_default(subscriber);

    tracing::error!(
        // i64 (record_i64)
        signed_field = -42_i64,
        // u64 (record_u64)
        unsigned_field = 99_u64,
        // bool (record_bool)
        bool_field = true,
        // f64 (record_f64). Not an approx-pi value — clippy
        // would flag that as `approx_constant`.
        float_field = 2.5_f64,
        // &str (record_str)
        string_field = "hello",
        "primitive harvest"
    );
    tokio::time::sleep(Duration::from_millis(50)).await;

    let recorded = calls.lock().await;
    assert_eq!(recorded.len(), 1, "exactly one post recorded");
    let body = recorded[0].1.to_string();

    // Every field name + every formatted value must appear in
    // the body. Each pair pins a distinct `record_*` arm.
    for (key, val) in [
        ("signed_field", "-42"),
        ("unsigned_field", "99"),
        ("bool_field", "true"),
        ("float_field", "2.5"),
        ("string_field", "hello"),
    ] {
        assert!(body.contains(key), "key `{key}` missing in body: {body}");
        assert!(
            body.contains(val),
            "value `{val}` for key `{key}` missing in body: {body}"
        );
    }
}
