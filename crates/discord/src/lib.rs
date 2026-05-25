//! Discord notifications — emits server-side events to Discord channels via
//! webhooks for ops visibility during development.
//!
//! Two emit paths feed the same `Event` pipeline:
//!
//! 1. **Explicit `emit_*` calls** at semantically meaningful seams (login,
//!    world entry, mission complete, …) — the type system enforces the
//!    payload shape.
//! 2. **Tracing layer** that auto-harvests `warn!` and `error!` events from
//!    the existing #304 negative-logging convention, lifting `reason`,
//!    `entity_id`, etc. into `Event::Warning` / `Event::Error` payloads.
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
/// `emit_*` helpers and the admin-api endpoint.
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
