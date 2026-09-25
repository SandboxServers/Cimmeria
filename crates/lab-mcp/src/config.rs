//! Fail-closed startup configuration for the lab MCP endpoint.
//!
//! The endpoint is opt-in and starts **only** when BOTH `CIMMERIA_LAB_MCP_BIND`
//! and `CIMMERIA_LAB_MCP_TOKEN` are set, and the token is at least
//! [`MIN_TOKEN_LEN`] bytes. There is no default bind — an operator must
//! explicitly choose an address (and should keep it off the player-facing
//! interfaces). See `docs/architecture/live-research-lab.md` §3.5.

/// Bind-address environment variable. No default — must be set explicitly.
pub const ENV_BIND: &str = "CIMMERIA_LAB_MCP_BIND";
/// Shared-bearer-token environment variable.
pub const ENV_TOKEN: &str = "CIMMERIA_LAB_MCP_TOKEN";

/// Minimum accepted token length in bytes. A shorter token is refused rather
/// than started, so a weak/placeholder secret can never expose the endpoint.
pub const MIN_TOKEN_LEN: usize = 32;

/// Validated configuration for a lab MCP endpoint that is clear to start.
#[derive(Debug, Clone)]
pub struct LabMcpConfig {
    /// The socket address to bind (e.g. `127.0.0.1:8451`).
    pub bind: String,
    /// The shared bearer token clients must present.
    pub token: String,
}

/// Outcome of evaluating the environment. The endpoint starts only for
/// [`LabMcpStartup::Enabled`].
#[derive(Debug)]
pub enum LabMcpStartup {
    /// One or both env vars are unset. The endpoint is opt-in, so this is the
    /// normal "not running" state — no error, just off.
    Disabled,
    /// Both env vars are set but the token is shorter than [`MIN_TOKEN_LEN`].
    /// This is a *refusal*, not a silent skip: the operator asked for the
    /// endpoint but with a token too weak to run it safely.
    Refused(String),
    /// Both env vars are set and the token is long enough — start it.
    Enabled(LabMcpConfig),
}

/// Evaluate an already-read `(bind, token)` pair. Pure, so the fail-closed
/// rules can be unit-tested without mutating process env (which is racy across
/// the parallel test run). [`from_env`] is the thin wrapper that reads the env.
pub fn evaluate(bind: Option<String>, token: Option<String>) -> LabMcpStartup {
    // Treat empty strings as unset — an env var exported as "" is a
    // configuration slip, not a deliberate enable.
    let bind = bind.filter(|s| !s.is_empty());
    let token = token.filter(|s| !s.is_empty());

    let (Some(bind), Some(token)) = (bind, token) else {
        return LabMcpStartup::Disabled;
    };

    // Length is measured in bytes (`.len()`), matching the "32 bytes" rule.
    if token.len() < MIN_TOKEN_LEN {
        return LabMcpStartup::Refused(format!(
            "{ENV_TOKEN} is {} bytes; the lab MCP endpoint requires at least {MIN_TOKEN_LEN}",
            token.len()
        ));
    }

    LabMcpStartup::Enabled(LabMcpConfig { bind, token })
}

/// Read `CIMMERIA_LAB_MCP_BIND` / `CIMMERIA_LAB_MCP_TOKEN` and apply the
/// fail-closed rules via [`evaluate`].
pub fn from_env() -> LabMcpStartup {
    evaluate(std::env::var(ENV_BIND).ok(), std::env::var(ENV_TOKEN).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(n: usize) -> String {
        "a".repeat(n)
    }

    /// Missing bind → the endpoint must not start. Regression guard: a default
    /// bind would silently expose the endpoint on some interface.
    #[test]
    fn missing_bind_disables() {
        assert!(matches!(
            evaluate(None, Some(tok(64))),
            LabMcpStartup::Disabled
        ));
    }

    /// Missing token → the endpoint must not start.
    #[test]
    fn missing_token_disables() {
        assert!(matches!(
            evaluate(Some("127.0.0.1:8451".into()), None),
            LabMcpStartup::Disabled
        ));
    }

    /// An empty-string env var counts as unset (config slip, not an enable).
    #[test]
    fn empty_values_disable() {
        assert!(matches!(
            evaluate(Some(String::new()), Some(tok(64))),
            LabMcpStartup::Disabled
        ));
        assert!(matches!(
            evaluate(Some("127.0.0.1:8451".into()), Some(String::new())),
            LabMcpStartup::Disabled
        ));
    }

    /// A token shorter than 32 bytes → refused (not started), with a reason.
    /// Regression guard: dropping the length check would start the endpoint
    /// behind a weak secret.
    #[test]
    fn short_token_is_refused() {
        // 31 bytes: one short of the floor.
        match evaluate(Some("127.0.0.1:8451".into()), Some(tok(MIN_TOKEN_LEN - 1))) {
            LabMcpStartup::Refused(msg) => assert!(msg.contains("at least 32")),
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    /// Exactly 32 bytes is accepted (boundary), and both fields carry through.
    #[test]
    fn exact_min_token_enables() {
        match evaluate(Some("127.0.0.1:8451".into()), Some(tok(MIN_TOKEN_LEN))) {
            LabMcpStartup::Enabled(cfg) => {
                assert_eq!(cfg.bind, "127.0.0.1:8451");
                assert_eq!(cfg.token.len(), MIN_TOKEN_LEN);
            }
            other => panic!("expected Enabled, got {other:?}"),
        }
    }
}
