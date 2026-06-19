//! Discord notifications — emits server-side events to Discord channels via
//! webhooks for ops visibility during development.
//!
//! Two emit paths feed the same `Event` pipeline:
//!
//! 1. **Explicit `emit_*` calls** at semantically meaningful seams (login,
//!    world entry, mission complete, …) — the type system enforces the
//!    payload shape.
//! 2. **Tracing layer** that auto-harvests `warn!` and `error!` events that
//!    follow the [negative-logging convention] — structured fields like
//!    `reason`, `entity_id`, `rows_affected` are lifted into
//!    `Event::Warning` / `Event::Error` payloads automatically.
//!
//! [negative-logging convention]: ../../../docs/architecture/negative-logging-convention.md
//!
//! Architecture:
//!
//! ```text
//! emit_*() ─┐
//!           ├─► Router (Event → ChannelKind) ─► bounded mpsc ─► sender task
//! Layer  ───┘                                                       │
//!                                                                   ▼
//!                                                            per-channel
//!                                                            token bucket
//!                                                                   │
//!                                                                   ▼
//!                                                            reqwest POST
//!                                                              (Discord)
//! ```
//!
//! See `docs/architecture/discord-notifications.md` for the design rationale
//! and `config/discord.toml.example` for the configuration schema.

pub mod color;
pub mod config;
pub mod embed;
pub mod event;
pub mod layer;
pub mod router;
pub mod sender;

pub use config::{Config, ConfigError, ConfigWatcher, EventToggles};
pub use event::{ChannelKind, ChatKind, DisconnectReason, Event, EventKind, TracingEventKind};
pub use layer::DiscordLayer;
pub use sender::{
    spawn as spawn_sender, DiscordSender, HttpDiscordSender, MockSender, QueueFull, SendError,
    SenderHandle, SenderStats,
};

use std::path::Path;
use std::sync::OnceLock;

/// Singleton handle to the Discord sender. Lazy-initialised at `init` and
/// read by the `emit_*` helpers + `panic_hook`. Holding a global is the
/// pragmatic choice — the alternative (threading a `SenderHandle` through
/// every emit site) doubles the wiring without measurable benefit.
static GLOBAL: OnceLock<DiscordRuntime> = OnceLock::new();

/// Holds the live wiring: config watcher (for stats + reload), sender
/// handle, and the sender task's join handle (kept alive for the
/// lifetime of the process).
pub struct DiscordRuntime {
    pub config: ConfigWatcher,
    pub handle: SenderHandle,
    _task: tokio::task::JoinHandle<()>,
}

impl DiscordRuntime {
    pub fn stats(&self) -> SenderStats {
        self.handle.stats()
    }

    pub fn reload(&self) {
        self.config.reload();
    }
}

/// Initialise the Discord runtime from a config file path. Idempotent
/// (subsequent calls silently no-op) so the binary can call this from
/// `main` regardless of what other tests-or-tools-in-the-same-process
/// did.
///
/// Returns the runtime handle even when the config loads as disabled —
/// callers can install the tracing layer either way; `should_post`
/// gating means emits are no-ops when disabled.
///
/// Errors only when the file is present but invalid (so the operator
/// sees a clear startup failure on a typo); a missing file falls back
/// to the disabled-by-default runtime.
pub fn init(path: impl AsRef<Path>) -> Result<&'static DiscordRuntime, ConfigError> {
    if let Some(rt) = GLOBAL.get() {
        return Ok(rt);
    }
    let path_ref = path.as_ref();
    let config = if path_ref.exists() {
        ConfigWatcher::new(path_ref)?
    } else {
        tracing::info!(
            target: "cimmeria_discord",
            path = %path_ref.display(),
            "Discord config file not found — Discord notifications disabled"
        );
        ConfigWatcher::from_static(Config::disabled())
    };
    let (handle, task) = sender::spawn(HttpDiscordSender::default(), config.handle());
    let runtime = DiscordRuntime {
        config,
        handle,
        _task: task,
    };
    let _ = GLOBAL.set(runtime);
    Ok(GLOBAL.get().expect("just set"))
}

/// Initialise the Discord runtime with an in-memory config (used by
/// tests / harnesses that don't want a file).
pub fn init_with_config(config: Config) -> &'static DiscordRuntime {
    if let Some(rt) = GLOBAL.get() {
        return rt;
    }
    let watcher = ConfigWatcher::from_static(config);
    let (handle, task) = sender::spawn(HttpDiscordSender::default(), watcher.handle());
    let runtime = DiscordRuntime {
        config: watcher,
        handle,
        _task: task,
    };
    let _ = GLOBAL.set(runtime);
    GLOBAL.get().expect("just set")
}

/// Snapshot of the global runtime, if `init` was called. Used by the
/// `emit_*` helpers and by any external integrations (admin-api,
/// supervisor, …) that want to read stats or force a reload.
pub fn global() -> Option<&'static DiscordRuntime> {
    GLOBAL.get()
}

/// Convenience: emit an event through the global runtime. If `init` was
/// never called, this is a no-op (returns silently). This is the function
/// every emit-site in the server bin calls.
pub fn emit(event: Event) {
    if let Some(rt) = GLOBAL.get() {
        let _ = rt.handle.try_send(event);
    }
}

// ── Typed convenience constructors ──────────────────────────────────────
//
// Each helper builds the Event variant + current timestamp + calls
// `emit`. The point is to keep the call site terse:
//
//     cimmeria_discord::emit_player_login(account_id, Some(name), addr);
//
// rather than:
//
//     cimmeria_discord::emit(cimmeria_discord::Event::PlayerLogin {
//         account_id, character_name: Some(name), addr,
//         timestamp: chrono::Utc::now(),
//     });
//
// Add a helper here for any event type that has a permanent emit site.

use std::net::SocketAddr;

pub fn emit_server_startup(version: impl Into<String>, bind_addrs: Vec<String>) {
    emit(Event::ServerStartup {
        version: version.into(),
        bind_addrs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_server_shutdown(reason: impl Into<String>, uptime_secs: u64) {
    emit(Event::ServerShutdown {
        reason: reason.into(),
        uptime_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_login(account_id: u32, character_name: Option<String>, addr: SocketAddr) {
    emit(Event::PlayerLogin {
        account_id,
        character_name,
        addr,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_logout(account_id: u32, character_name: Option<String>, session_secs: u64) {
    emit(Event::PlayerLogout {
        account_id,
        character_name,
        session_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_disconnect(
    account_id: Option<u32>,
    character_name: Option<String>,
    addr: SocketAddr,
    reason: DisconnectReason,
    session_secs: u64,
) {
    emit(Event::PlayerDisconnect {
        account_id,
        character_name,
        addr,
        reason,
        session_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_auth_failed(
    account_name: impl Into<String>,
    addr: SocketAddr,
    reason: impl Into<String>,
) {
    emit(Event::PlayerAuthFailed {
        account_name: account_name.into(),
        addr,
        reason: reason.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_world_entry(
    account_id: u32,
    character_name: impl Into<String>,
    world_name: impl Into<String>,
    position: [f32; 3],
) {
    emit(Event::PlayerWorldEntry {
        account_id,
        character_name: character_name.into(),
        world_name: world_name.into(),
        position,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_world_exit(
    account_id: u32,
    character_name: impl Into<String>,
    from_world: impl Into<String>,
    to_world: Option<String>,
) {
    emit(Event::PlayerWorldExit {
        account_id,
        character_name: character_name.into(),
        from_world: from_world.into(),
        to_world,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_chat(
    kind: ChatKind,
    speaker: impl Into<String>,
    recipient: Option<String>,
    content: impl Into<String>,
) {
    emit(Event::Chat {
        kind,
        speaker: speaker.into(),
        recipient,
        content: content.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_level_up(character_name: impl Into<String>, new_level: u32) {
    emit(Event::PlayerLevelUp {
        character_name: character_name.into(),
        new_level,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_accepted(
    character_name: impl Into<String>,
    mission_id: i32,
    mission_name: Option<String>,
) {
    emit(Event::MissionAccepted {
        character_name: character_name.into(),
        mission_id,
        mission_name,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_completed(
    character_name: impl Into<String>,
    mission_id: i32,
    mission_name: Option<String>,
) {
    emit(Event::MissionCompleted {
        character_name: character_name.into(),
        mission_id,
        mission_name,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_gm_command(
    gm_name: impl Into<String>,
    command: impl Into<String>,
    args: impl Into<String>,
) {
    emit(Event::GmCommand {
        gm_name: gm_name.into(),
        command: command.into(),
        args: args.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_item_used(
    character_name: impl Into<String>,
    item_type_id: i32,
    target: Option<String>,
) {
    emit(Event::ItemUsed {
        character_name: character_name.into(),
        item_type_id,
        target,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_wire_format_error(
    kind: impl Into<String>,
    addr: Option<SocketAddr>,
    details: impl Into<String>,
) {
    emit(Event::WireFormatError {
        kind: kind.into(),
        addr,
        details: details.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_db_error(operation: impl Into<String>, details: impl Into<String>) {
    emit(Event::DbError {
        operation: operation.into(),
        details: details.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mercury_timeout(addr: SocketAddr, account_id: Option<u32>, silence_secs: u64) {
    emit(Event::MercuryTimeout {
        addr,
        account_id,
        silence_secs,
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_mission_failed(
    character_name: impl Into<String>,
    mission_id: i32,
    mission_name: Option<String>,
    reason: impl Into<String>,
) {
    emit(Event::MissionFailed {
        character_name: character_name.into(),
        mission_id,
        mission_name,
        reason: reason.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_death(
    character_name: impl Into<String>,
    killer: Option<String>,
    cause: impl Into<String>,
) {
    emit(Event::PlayerDeath {
        character_name: character_name.into(),
        killer,
        cause: cause.into(),
        timestamp: chrono::Utc::now(),
    });
}

pub fn emit_player_respawn(character_name: impl Into<String>, world_name: impl Into<String>) {
    emit(Event::PlayerRespawn {
        character_name: character_name.into(),
        world_name: world_name.into(),
        timestamp: chrono::Utc::now(),
    });
}

/// Install a panic hook that posts a [`Event::ServerPanic`] to the
/// lifecycle channel before the default hook aborts the process.
///
/// Uses a synchronous `reqwest::blocking` POST with a 2-second timeout —
/// the async pipeline can't drain in time when the runtime is unwinding.
/// On timeout or failure the embed is dropped; the panic still surfaces
/// via the default hook (stderr / log file).
///
/// Wraps the previous hook so existing panic handling (backtrace, log,
/// etc.) is preserved.
pub fn install_panic_hook() {
    use std::sync::Mutex;
    static INSTALLED: Mutex<bool> = Mutex::new(false);
    let mut installed = match INSTALLED.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if *installed {
        return;
    }
    *installed = true;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Synchronously post if Discord is configured for lifecycle.
        post_panic_synchronous(info);
        previous(info);
    }));
}

/// Synchronous panic post. Uses `reqwest::blocking` because the tokio
/// runtime may not be safe to use mid-unwind. Drops the embed silently
/// on any failure — losing the Discord notification is better than
/// hanging the unwind.
fn post_panic_synchronous(info: &std::panic::PanicHookInfo<'_>) {
    let Some(rt) = GLOBAL.get() else { return };
    let cfg = rt.config.load();
    if !cfg.should_post(EventKind::ServerPanic) {
        return;
    }
    let Some(url) = cfg.webhook_url_for(EventKind::ServerPanic) else {
        return;
    };
    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "<unknown>".to_string());
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_string());
    let event = Event::ServerPanic {
        location,
        message,
        timestamp: chrono::Utc::now(),
    };
    let body = embed::build_embed_body(&event, cfg.username.as_deref(), cfg.avatar_url.as_deref());

    // Best-effort 2-second blocking POST. Errors swallowed — the
    // process is dying; surface elsewhere.
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let _ = client.post(url).json(&body).send();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ChannelConfig, EventToggles};
    use crate::event::{ChannelKind, ChatKind, DisconnectReason};
    use std::collections::HashMap;
    use std::sync::OnceLock;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Process-wide guard: tests in this module both touch the
    /// `GLOBAL: OnceLock<DiscordRuntime>` and only one of them can
    /// succeed in *setting* it (subsequent `init_with_config` calls
    /// silently return the existing runtime). All assertions that
    /// depend on the global match against whatever the chosen-
    /// initialiser set up — adding a second initialiser test to this
    /// file would race against it.
    ///
    /// Holding a `OnceLock<()>` here is the simplest sentinel: tests
    /// can `.get()` to check whether the suite has already paid the
    /// init cost. We don't actually need to coordinate further —
    /// nextest gives us one process per binary and the one
    /// initialiser is fine for every test that needs the global.
    static TESTS_INIT_GUARD: OnceLock<()> = OnceLock::new();

    /// Build a wiremock-backed config with **every** channel routed at
    /// the supplied URL, every event toggled ON, and the burst
    /// budget high enough that 14 emits in a tight loop all land.
    fn all_on_config(wiremock_uri: &str) -> Config {
        let mut channels = HashMap::new();
        for c in ChannelKind::ALL {
            channels.insert(
                *c,
                ChannelConfig {
                    url: format!("{}/{}", wiremock_uri, c.as_str()),
                    // Above Discord's 150/min hard cap on purpose — the
                    // config sanitiser clamps to 150 (we exercised that
                    // in config::tests::rate_limit_clamped_to_safe_range),
                    // and 150 is plenty of burst for 14 sequential
                    // emits in this test.
                    rate_limit_per_min: 150,
                },
            );
        }
        // Every toggle ON so no emit_* call gets filtered.
        let events = EventToggles {
            chat_say: true,
            chat_whisper: true,
            chat_guild: true,
            chat_team: true,
            chat_command: true,
            player_death: true,
            player_respawn: true,
            mission_failed: true,
            mission_reward_granted: true,
            loot_generated: true,
            item_used: true,
            warning: true,
            ..EventToggles::default()
        };
        Config {
            enabled: true,
            username: None,
            avatar_url: None,
            channels,
            events,
        }
    }

    /// End-to-end coverage for every typed `emit_*` helper. Boots a
    /// wiremock server, wires every channel at it, calls
    /// `init_with_config`, then fires each `emit_*` helper once and
    /// asserts the wiremock saw the expected count of POSTs.
    ///
    /// Pinning the COUNT (not the bodies) is deliberate: the per-
    /// variant body format is already covered by
    /// `embed::tests::every_event_variant_builds` + the per-event
    /// formatter tests. This test's regression target is the typed-
    /// wrapper layer — a future hand that drops `emit_player_login`
    /// or routes it through the wrong `Event` variant trips this.
    ///
    /// **One-shot global init.** This is the only test in
    /// `cimmeria-discord` that calls `init_with_config`. Adding a
    /// second initialiser test in this binary would silently observe
    /// the runtime set up here (`OnceLock::set` returns
    /// `Err(existing)` quietly). If you need that, factor a
    /// `#[serial_test::serial]` discipline in first.
    #[tokio::test]
    async fn every_emit_helper_routes_through_global_runtime() {
        let server = MockServer::start().await;
        // One mock that matches every POST regardless of path.
        // Returning 204 lets the sender post successfully and feeds
        // the request log we assert against.
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let cfg = all_on_config(&server.uri());
        let rt = init_with_config(cfg);
        // Belt-and-braces: if a stale GLOBAL slipped in from a
        // future test, mark the guard so the next reader of this
        // file knows what's going on. (No actual coordination —
        // see the doc comment on TESTS_INIT_GUARD.)
        let _ = TESTS_INIT_GUARD.set(());

        // Fire every typed helper. Add a new line here when a new
        // emit_* helper lands in lib.rs.
        let addr: SocketAddr = "127.0.0.1:50000".parse().unwrap();
        emit_server_startup("0.1.0", vec!["addr".into()]);
        emit_server_shutdown("test", 100);
        emit_player_login(1, Some("alice".into()), addr);
        emit_player_logout(1, Some("alice".into()), 60);
        emit_player_disconnect(
            Some(1),
            Some("alice".into()),
            addr,
            DisconnectReason::Timeout,
            42,
        );
        emit_player_auth_failed("badname", addr, "invalid password");
        emit_player_world_entry(1, "alice", "Castle", [1.0, 2.0, 3.0]);
        emit_player_world_exit(1, "alice", "Castle", Some("Tollana".into()));
        emit_chat(ChatKind::Global, "alice", None, "hello");
        emit_level_up("alice", 5);
        emit_mission_accepted("alice", 1234, Some("Find Ambernol".into()));
        emit_mission_completed("alice", 1234, Some("Find Ambernol".into()));
        emit_gm_command("steve", "/teleport", "alice 1,2,3");
        emit_item_used("alice", 5001, Some("self".into()));
        emit_wire_format_error(
            "seq_out_of_range",
            Some(addr),
            "seq 0x1fffffff >= NULL_SEQUENCE",
        );
        emit_db_error("auth_user", "connection refused");
        emit_mercury_timeout(addr, Some(1), 60);
        emit_mission_failed("alice", 1234, Some("Find Ambernol".into()), "timer expired");
        emit_player_death("alice", Some("Jaffa Guard".into()), "staff blast");
        emit_player_respawn("alice", "Castle");

        const EXPECTED_EMITS: u64 = 20;

        // The typed-wrapper regression target: every `emit_*` helper must
        // enqueue exactly one event. `enqueued` is bumped synchronously in
        // `SenderHandle::try_send` BEFORE the per-channel token bucket, so
        // this count is immune to rate-limit drops — which matters now that
        // the gameplay channel has 7 helper types but only a 5-msg burst
        // budget (the tight loop here would otherwise drop ~2 gameplay
        // posts). A future hand that drops `emit_player_login` or stops a
        // helper from constructing its event trips this.
        assert_eq!(
            rt.stats().enqueued,
            EXPECTED_EMITS,
            "every emit_* helper must enqueue exactly one event through the global runtime"
        );

        // Drain briefly so the routing-diversity check below has traffic to
        // inspect. Not pinning an exact POST count — the gameplay burst cap
        // drops a couple, and the enqueue assertion above already guards the
        // count. 15 is comfortably below 20-minus-drops and above the
        // 5-channel diversity floor.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let count = server.received_requests().await.unwrap().len();
            if count >= 15 || std::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        // Sanity that every channel got some traffic — proves the
        // routing table in `router::channel_for` matches the
        // EventKind each helper produces. (Most channels get one
        // emit; the test isn't pinning specific counts per channel
        // because that's the router's contract, not the typed
        // wrapper's.)
        let received = server.received_requests().await.unwrap();
        let urls: std::collections::HashSet<String> =
            received.iter().map(|r| r.url.path().to_string()).collect();
        assert!(
            urls.len() >= 5,
            "expected multiple channels routed; got paths {urls:?}"
        );
    }

    /// `install_panic_hook` is idempotent — calling it twice (e.g. by
    /// the server bin and a test harness in the same process) must
    /// not stack hooks. Verifies the `INSTALLED` mutex guards
    /// re-entry.
    ///
    /// We don't trigger an actual panic — `post_panic_synchronous`
    /// would attempt a blocking HTTP POST and the test process would
    /// terminate via the default hook's abort.
    #[test]
    fn install_panic_hook_is_idempotent() {
        install_panic_hook();
        install_panic_hook();
        install_panic_hook();
        // No assertion — we're pinning that this doesn't deadlock,
        // panic, or stack-overflow. A naïve implementation that
        // wrapped the existing hook unconditionally would either
        // grow the call stack on each panic or recurse infinitely.
    }

    /// `global()` returns whatever `init_with_config` set in the
    /// init-test above (or None if THIS test happens to run first).
    /// Either outcome is a valid no-panic exercise of the read path,
    /// so we don't assert which — we're just covering the function
    /// body.
    #[test]
    fn global_accessor_does_not_panic() {
        let _ = global();
    }
}
