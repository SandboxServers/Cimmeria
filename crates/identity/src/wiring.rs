//! Mounts the `/auth/v1` routes on the auth service at startup (contract C4,
//! C12) and starts the reaper that drops expired sign-in state.
//!
//! [`install`] reads the providers file named by `CIMMERIA_AUTH_PROVIDERS`
//! (default `config/auth-providers.toml`), builds the providers, and hands the
//! router to [`AuthService::set_identity_routes`]. A refused file is an error
//! the caller turns into a stop; a skipped provider is a warning, logged once.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use cimmeria_auth::auth::AuthService;
use sqlx::PgPool;

use crate::config::{self, ConfigError};
use crate::http;
use crate::limits::RateLimiter;
use crate::link_code::{self, LinkCodeStore};
use crate::providers;
use crate::repo::refresh::purge_expired;
use crate::routes;
use crate::routes::ctx::IdentityCtx;
use crate::stores::Stores;

/// The default providers file, relative to the server's working directory.
pub const DEFAULT_PROVIDERS_PATH: &str = "config/auth-providers.toml";

/// How often the reaper drops expired sign-in state.
pub const REAP_EVERY: Duration = Duration::from_secs(60);

/// How often the refresh-token purge deletes dead rows (C12).
pub const PURGE_EVERY: Duration = Duration::from_secs(60 * 60);

/// Installs the `/auth/v1` routes on `auth` and starts the reaper.
///
/// The path comes from `CIMMERIA_AUTH_PROVIDERS`, else
/// [`DEFAULT_PROVIDERS_PATH`]. Must run inside the tokio runtime, since the
/// reaper is spawned here.
pub fn install(auth: &mut AuthService, developer_mode: bool) -> Result<(), ConfigError> {
    install_with(auth, &providers_path(), developer_mode)
}

/// [`install`] with the providers file given by the caller. The live-DB smoke
/// test points it at a temporary file instead of writing the process env.
pub(crate) fn install_with(
    auth: &mut AuthService,
    path: &Path,
    developer_mode: bool,
) -> Result<(), ConfigError> {
    let (cfg, warnings) = config::load_file(path, developer_mode)?;
    for warning in &warnings {
        tracing::warn!(
            reason = warning.reason(),
            provider = warning.provider(),
            "auth provider skipped"
        );
    }
    let cfg = Arc::new(cfg);
    let providers = Arc::new(providers::build(&cfg, http::provider_client()));
    let stores = Arc::new(Stores::default());
    let limiter = Arc::new(RateLimiter::with_limits(&cfg.limits));
    let ctx = IdentityCtx {
        cfg: Arc::clone(&cfg),
        providers: Arc::clone(&providers),
        stores: Arc::clone(&stores),
        limiter,
        link_codes: link_code::shared(),
    };
    tracing::info!(
        providers = providers.list().len(),
        signup_open = cfg.signup_open,
        idp_disabled = cfg.kill.idp_disabled,
        signup_disabled = cfg.kill.signup_disabled,
        soap_relay_disabled = cfg.kill.soap_relay_disabled,
        "auth identity routes installed"
    );
    tokio::spawn(run_reaper(
        stores,
        link_code::shared(),
        Instant::now,
        REAP_EVERY,
    ));
    // The pool is only known once the auth service hands over its deps, so the
    // purge starts here, and only when a database is configured.
    auth.set_identity_routes(Box::new(move |deps| {
        if let Some(pool) = deps.db.clone() {
            tokio::spawn(run_purge(pool, PURGE_EVERY));
        }
        routes::router(deps, ctx)
    }));
    Ok(())
}

/// Calls [`purge_expired`] every `period` until the task is dropped. A failed
/// run logs its reason only (no row data) and waits for the next tick.
async fn run_purge(pool: Arc<PgPool>, period: Duration) {
    let mut ticks = tokio::time::interval(period);
    loop {
        ticks.tick().await;
        if purge_expired(&pool).await.is_err() {
            tracing::warn!(reason = "refresh_purge_failed", "auth refresh purge failed");
        }
    }
}

/// The providers file path: `CIMMERIA_AUTH_PROVIDERS` when set and non-empty.
fn providers_path() -> PathBuf {
    std::env::var_os("CIMMERIA_AUTH_PROVIDERS")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_PROVIDERS_PATH))
}

/// Calls [`reap_once`] every `period`, with `clock` supplying the time, until
/// the task is dropped.
pub async fn run_reaper(
    stores: Arc<Stores>,
    codes: &'static LinkCodeStore,
    clock: impl Fn() -> Instant + Send + 'static,
    period: Duration,
) {
    let mut ticks = tokio::time::interval(period);
    loop {
        ticks.tick().await;
        reap_once(&stores, codes, clock());
    }
}

/// Drops every expired record of the sign-in stores and the link-code store.
pub fn reap_once(stores: &Stores, codes: &LinkCodeStore, now: Instant) {
    stores.reap(now);
    codes.reap(now);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reaper runs once a minute (C12).
    #[test]
    fn reaper_period_is_sixty_seconds() {
        assert_eq!(REAP_EVERY, Duration::from_secs(60));
    }

    /// The reaper drops an expired access token on its own: with the reap
    /// call removed from `run_reaper`, the token is still there afterwards.
    #[tokio::test]
    async fn reaper_drops_expired_state_without_a_caller() {
        let stores = Arc::new(Stores::default());
        let codes: &'static LinkCodeStore = Box::leak(Box::default());
        let t0 = Instant::now();
        let token = stores.access.mint(7, "fam-reaper", false, t0);
        let later = t0 + Duration::from_secs(60 * 60 * 24);
        let reaper = tokio::spawn(run_reaper(
            Arc::clone(&stores),
            codes,
            move || later,
            Duration::from_millis(5),
        ));
        tokio::time::sleep(Duration::from_millis(60)).await;
        reaper.abort();
        assert!(stores.access.check(&token, t0).is_none());
    }

    /// A missing providers file is a valid empty configuration (C4), not an error.
    #[tokio::test]
    async fn missing_file_installs_no_providers() {
        let mut auth = AuthService::new(&Default::default());
        let missing = std::env::temp_dir().join("cimmeria-no-such-auth-providers.toml");
        assert!(install_with(&mut auth, &missing, false).is_ok());
    }
}
