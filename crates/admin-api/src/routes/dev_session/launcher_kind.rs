//! The `launcher` session kind (C12, AU-11): a launcher session is minted
//! with no game and no credential, lives 15 minutes and is never refreshed.
//! Minting stays unauthenticated (D-AU26), so the kind is a label backed by
//! its own scope and tighter caps, never a privilege. See README D-AU19.

use std::net::IpAddr;
use std::time::Instant;

use super::handlers::{
    env_u32, DevSessionRequest, QuotaPolicy, Tables, DEFAULT_CHUNK_MAX_BYTES, TOKEN_TTL_SECONDS,
};
use super::quota::{install_key, ip_key, validate_metadata};
use super::token::{
    AuthError, TokenClaims, SCOPE_LAUNCHER_WRITE, SCOPE_TELEMETRY_WRITE, SESSION_KIND_LAUNCHER,
};

/// A launcher session lives 15 minutes and is never refreshed (R3, R4).
pub const LAUNCHER_TOKEN_TTL_SECONDS: i64 = 15 * 60;
/// The `chunk_max_bytes` a launcher mint replies with (NDJSON before gzip).
pub const LAUNCHER_CHUNK_MAX_BYTES: u64 = 65_536;
/// Launcher mints per address per quota window (1 h), on top of the shared per-IP table (R6).
pub const DEFAULT_LAUNCHER_MINT_PER_IP: u32 = 30;
pub const LAUNCHER_MINT_IP_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_IP";
/// Launcher mints per install_id per quota window, on their own table.
pub const DEFAULT_LAUNCHER_MINT_PER_INSTALL: u32 = 12;
pub const LAUNCHER_MINT_INSTALL_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_INSTALL";
/// "1" or "true" turns off the launcher kind only (mint and upload answer 503). Rec4.
pub const LAUNCHER_DISABLED_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_DISABLED";

/// True when `CIMMERIA_TELEMETRY_LAUNCHER_DISABLED` is `1` or `true`.
pub fn launcher_disabled_from_env() -> bool {
    std::env::var(LAUNCHER_DISABLED_ENV)
        .map(|v| matches!(v.trim(), "1" | "true"))
        .unwrap_or(false)
}

/// True when the raw request asks for the launcher kind.
pub(super) fn is_launcher_request(req: &DevSessionRequest) -> bool {
    req.session_kind.as_deref().map(str::trim) == Some(SESSION_KIND_LAUNCHER)
}

/// Launcher mint metadata (branch, git_sha, launcher_version, tags) must be
/// printable ASCII (' '..='~'); anything else is BadField 400. Rec8.
pub(super) fn validate_launcher_metadata(req: &DevSessionRequest) -> Result<(), AuthError> {
    let fields = [
        ("branch", req.branch.as_str()),
        ("git_sha", req.git_sha.as_str()),
        ("launcher_version", req.launcher_version.as_str()),
    ];
    let tags = req.tags.iter().map(|t| ("tags", t.as_str()));
    for (field, value) in fields.into_iter().chain(tags) {
        if !value.bytes().all(|b| (b' '..=b'~').contains(&b)) {
            return Err(AuthError::BadField {
                field,
                reason: "must be printable ASCII",
            });
        }
    }
    Ok(())
}

/// The launcher flag for a request, after the launcher-only kill switch,
/// checked before any quota is charged (C12 order step 2). A launcher's
/// `machine_id` is ignored, not validated.
pub(super) fn admit(policy: &QuotaPolicy, req: &DevSessionRequest) -> Result<bool, AuthError> {
    let launcher = is_launcher_request(req);
    if launcher && policy.launcher_disabled {
        return Err(AuthError::KillSwitchActive);
    }
    Ok(launcher)
}

/// The quota reads for the launcher kind, from the C12 env names:
/// `(per IP, per install, disabled)`.
pub(super) fn quota_from_env() -> (u32, u32, bool) {
    (
        env_u32(LAUNCHER_MINT_IP_ENV, DEFAULT_LAUNCHER_MINT_PER_IP),
        env_u32(LAUNCHER_MINT_INSTALL_ENV, DEFAULT_LAUNCHER_MINT_PER_INSTALL),
        launcher_disabled_from_env(),
    )
}

/// The request checks, run after the install id: the `machine_id` rule
/// (skipped for a launcher), the printable metadata every kind must pass,
/// then the launcher-only rules (C12): printable metadata and the refusal
/// of a non-blank account name.
pub(super) fn check_request(req: &DevSessionRequest, launcher: bool) -> Result<(), AuthError> {
    if !launcher {
        validate_metadata(&req.machine_id).map_err(|reason| AuthError::BadField {
            field: "machine_id",
            reason,
        })?;
    }
    for (field, value) in [
        ("branch", &req.branch),
        ("git_sha", &req.git_sha),
        ("launcher_version", &req.launcher_version),
    ] {
        validate_metadata(value).map_err(|reason| AuthError::BadField { field, reason })?;
    }
    if !launcher {
        return Ok(());
    }
    validate_launcher_metadata(req)?;
    if req
        .account_name
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty())
    {
        return Err(AuthError::BadField {
            field: "account_name",
            reason: "not allowed for a launcher session",
        });
    }
    Ok(())
}

/// Charges the launcher per-address quota, on its own table (C12 step 4).
pub(super) fn charge_ip(
    tables: &Tables,
    policy: &QuotaPolicy,
    peer_ip: IpAddr,
    launcher: bool,
    now: Instant,
) -> Result<(), AuthError> {
    if !launcher {
        return Ok(());
    }
    tables.mint_launcher_ip.check_and_record(
        ip_key(peer_ip),
        policy.mint_launcher_per_ip,
        policy.window,
        "mint/launcher_ip",
        now,
    )?;
    Ok(())
}

/// Charges the per-install quota. A launcher charges its own table; every
/// other request charges the player's `mint_install`, as before (C12 step 7).
pub(super) fn charge_install(
    tables: &Tables,
    policy: &QuotaPolicy,
    install_id: &str,
    launcher: bool,
    now: Instant,
) -> Result<(), AuthError> {
    if launcher {
        tables.mint_launcher_install.check_and_record(
            install_key(install_id),
            policy.mint_launcher_per_install,
            policy.window,
            "mint/launcher_install",
            now,
        )?;
    } else {
        tables.mint_install.check_and_record(
            install_key(install_id),
            policy.mint_per_install,
            policy.window,
            "mint/install_id",
            now,
        )?;
    }
    Ok(())
}

/// The `machine_id` claim: a launcher mint never signs one (C12).
pub(super) fn machine_claim(launcher: bool, machine_id: &str) -> Option<String> {
    if launcher {
        return None;
    }
    Some(machine_id.trim())
        .filter(|m| !m.is_empty())
        .map(str::to_string)
}

/// Records the stream of a player or lab mint. A launcher session records
/// none: it has no game to join (R4).
pub(super) fn record_unless_launcher(launcher: bool, claims: &TokenClaims, peer_ip: IpAddr) {
    if !launcher {
        super::handlers::record_stream(claims, peer_ip);
    }
}

/// The `machine_id` a debug line may print: never for a launcher mint.
pub(super) fn machine_id_for_log(launcher: bool, machine_id: &str) -> &str {
    if launcher {
        "(launcher: not logged)"
    } else {
        machine_id
    }
}

/// The scopes a mint grants: `telemetry.launcher.write` for a launcher, the
/// shared `telemetry.write` otherwise.
pub(super) fn scope_for(launcher: bool) -> Vec<String> {
    let scope = if launcher {
        SCOPE_LAUNCHER_WRITE
    } else {
        SCOPE_TELEMETRY_WRITE
    };
    vec![scope.into()]
}

/// The lifetime of a mint: 15 minutes for a launcher, 8 hours otherwise.
pub(super) fn token_ttl(launcher: bool) -> i64 {
    if launcher {
        LAUNCHER_TOKEN_TTL_SECONDS
    } else {
        TOKEN_TTL_SECONDS
    }
}

/// The `chunk_max_bytes` a mint replies with.
pub(super) fn chunk_max_bytes(launcher: bool) -> u64 {
    if launcher {
        LAUNCHER_CHUNK_MAX_BYTES
    } else {
        DEFAULT_CHUNK_MAX_BYTES
    }
}

/// Refuses a refresh of a launcher token before any quota is charged (R3).
pub(super) fn refuse_refresh(claims: &TokenClaims) -> Result<(), AuthError> {
    if claims.is_launcher() || claims.has_scope(SCOPE_LAUNCHER_WRITE) {
        return Err(AuthError::BadField {
            field: "session_kind",
            reason: "a launcher session is not refreshed; mint a new one",
        });
    }
    Ok(())
}
