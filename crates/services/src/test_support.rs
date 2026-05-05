//! Shared helpers for live-DB tests.
//!
//! Tests that need a real PostgreSQL connection call [`test_pool`] and
//! self-skip when `DATABASE_URL` is unset. The unit-test suite stays
//! green on a fresh checkout; only `DATABASE_URL=postgres://… cargo
//! test` exercises the integration path.
//!
//! See `docs/architecture/integration-test-infra.md` for the rationale,
//! local-setup steps, and per-test data-isolation patterns.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Why a live-DB test couldn't run.
///
/// Distinguishes "no DATABASE_URL configured" (expected on a fresh
/// checkout — silent skip) from "DATABASE_URL set but unreachable"
/// (likely misconfiguration — surface the connection error so the
/// developer can fix it).
pub(crate) enum SkipReason {
    /// `DATABASE_URL` env var was unset or empty.
    NotConfigured,
    /// `DATABASE_URL` was set but `connect()` failed. The string
    /// captures sqlx's underlying error for operator triage.
    ConnectFailed(String),
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkipReason::NotConfigured => write!(f, "DATABASE_URL not set"),
            SkipReason::ConnectFailed(e) => write!(f, "DATABASE_URL set but connect failed: {e}"),
        }
    }
}

/// Open a `PgPool` against the developer-supplied `DATABASE_URL`, or
/// return a [`SkipReason`] explaining why no pool was produced.
///
/// Bounded to 4 connections — high enough for tests that exercise
/// concurrent paths (drainer + caller in parallel), low enough that
/// a careless test loop can't exhaust a hand-tuned local Postgres.
pub(crate) async fn test_pool() -> Result<PgPool, SkipReason> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return Err(SkipReason::NotConfigured),
    };
    PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await
        .map_err(|e| SkipReason::ConnectFailed(e.to_string()))
}

/// Convenience macro: skip a test with a reason-specific message if
/// no DB pool is available. Pairs with [`test_pool`] — same gate,
/// less ceremony at each call site.
///
/// ```ignore
/// #[tokio::test]
/// async fn my_db_test() {
///     let pool = require_db_or_skip!();
///     // ... test body uses pool ...
/// }
/// ```
macro_rules! require_db_or_skip {
    () => {{
        match $crate::test_support::test_pool().await {
            Ok(p) => p,
            Err(reason) => {
                eprintln!("{}: skipping live-DB test ({reason})", module_path!(),);
                return;
            }
        }
    }};
}

pub(crate) use require_db_or_skip;

// ── SpaceManager test fixtures (issue #149) ───────────────────────────

use crate::cell::space_manager::SpaceManager;

/// Standard test space setup: SpaceManager with a single Agnos space.
///
/// The same ~5-line block was duplicated across dispatch, interaction,
/// and vendor test modules. Extracted here so every cell test shares
/// the same default world geometry.
pub(crate) fn make_space_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(spaces_xml).unwrap();
    mgr.create_startup_spaces(cell_spaces_xml).unwrap();
    mgr
}

/// Same as [`make_space_manager`], but also creates a player entity at
/// the origin so tests that need an avatar don't repeat the entity
/// creation boilerplate.
#[allow(dead_code)]
pub(crate) fn make_space_manager_with_player(entity_id: u32) -> SpaceManager {
    let mut mgr = make_space_manager();
    mgr.create_entity(entity_id, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr
}
