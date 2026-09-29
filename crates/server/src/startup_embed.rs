//! What the Discord "Server up" embed reports: the build and the ports a
//! client or operator actually connects to.
//!
//! `auth_port` (13001) is deliberately absent: nothing binds it. Client login
//! is the SOAP listener on `logon_port`.

use cimmeria_common::ServerConfig;

/// `0.1.0+<12-char commit>`, or the bare crate version when the build could
/// not resolve a commit.
pub fn version(pkg_version: &str, build_sha: &str) -> String {
    match build_sha {
        "" | "unknown" => pkg_version.to_string(),
        sha => format!("{pkg_version}+{}", &sha[..sha.len().min(12)]),
    }
}

/// One `name :port` line per listener, in the order a player meets them.
pub fn bind_lines(config: &ServerConfig) -> Vec<String> {
    vec![
        format!("login :{}", config.logon_port),
        format!("base :{}", config.base_port),
        format!("cell :{}", config.cell_port),
        format!("minigame :{}", config.minigame_port),
        format!("admin :{}", config.admin_port),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_appends_a_short_commit() {
        assert_eq!(
            version("0.1.0", "08d63233f0123456789abcdef0123456789abcde"),
            "0.1.0+08d63233f012"
        );
    }

    #[test]
    fn version_without_a_commit_is_the_crate_version() {
        assert_eq!(version("0.1.0", "unknown"), "0.1.0");
        assert_eq!(version("0.1.0", ""), "0.1.0");
    }

    /// Regression guard: the embed used to list `auth :13001`, a port no
    /// listener binds, and omit the login and minigame ports.
    #[test]
    fn bind_lines_list_the_bound_listeners_not_auth_port() {
        let config = ServerConfig::default();
        let lines = bind_lines(&config);
        assert_eq!(
            lines,
            vec![
                format!("login :{}", config.logon_port),
                format!("base :{}", config.base_port),
                format!("cell :{}", config.cell_port),
                format!("minigame :{}", config.minigame_port),
                format!("admin :{}", config.admin_port),
            ]
        );
        assert!(!lines.iter().any(|l| l.starts_with("auth ")));
    }
}
