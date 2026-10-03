//! The client's login-server list, read from `LoginInternal.lua`.
//!
//! The launcher writes `LoginMod.servers["<name>"] = "<url>"` lines into
//! `<install>/SGWGame/Content/UI/Startup/Login/LoginInternal.lua` before
//! every launch (`crates/launcher/src/client_setup/login_servers.rs`). The
//! URL is the server's login port, which also serves
//! `/api/auth/dev-session`, so the lab mints its telemetry token from the
//! same server the client logs into, with no extra config.

use std::path::{Path, PathBuf};

/// One `LoginMod.servers[...]` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginServer {
    pub name: String,
    pub url: String,
}

/// `<install>/SGWGame/Content/UI/Startup/Login/LoginInternal.lua`.
pub fn login_internal_path(install_dir: &Path) -> PathBuf {
    install_dir
        .join("SGWGame")
        .join("Content")
        .join("UI")
        .join("Startup")
        .join("Login")
        .join("LoginInternal.lua")
}

/// Every `LoginMod.servers["name"] = "url"` row in `lua`, in file order.
/// Comments and other lines are skipped.
pub fn parse(lua: &str) -> Vec<LoginServer> {
    lua.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("--") {
                return None;
            }
            let rest = line.strip_prefix("LoginMod.servers[\"")?;
            let (name, rest) = rest.split_once("\"]")?;
            let rest = rest.trim_start().strip_prefix('=')?.trim_start();
            let url = rest.strip_prefix('"')?.split_once('"')?.0;
            Some(LoginServer {
                name: name.to_string(),
                url: url.to_string(),
            })
        })
        .collect()
}

/// The login URL for the row named `name`, or the only row when there is
/// just one. Several rows and no name match is an error naming the rows,
/// so the lab never mints from a server the client is not logging into.
pub fn pick(servers: &[LoginServer], name: Option<&str>) -> Result<String, String> {
    let wanted = name.map(str::trim).filter(|n| !n.is_empty());
    if let Some(n) = wanted {
        if let Some(s) = servers.iter().find(|s| s.name.eq_ignore_ascii_case(n)) {
            return Ok(s.url.clone());
        }
    }
    match servers {
        [] => Err("LoginInternal.lua lists no login servers".into()),
        [only] => Ok(only.url.clone()),
        many => {
            let names: Vec<&str> = many.iter().map(|s| s.name.as_str()).collect();
            Err(format!(
                "LoginInternal.lua lists {} login servers ({}) and none is named {:?}; \
                 set `server` in lab-account.json or CIMMERIA_LAB_SERVER_URL",
                many.len(),
                names.join(", "),
                wanted.unwrap_or("")
            ))
        }
    }
}

/// The login URL the client at `install_dir` will use for server row
/// `name` ([`parse`] + [`pick`]).
pub fn login_url(install_dir: &Path, name: Option<&str>) -> Result<String, String> {
    let path = login_internal_path(install_dir);
    let lua =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    pick(&parse(&lua), name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact shape the launcher writes (CRLF, header comment).
    const LAUNCHER_LUA: &str =
        "-- LoginInternal.lua, written by the Cimmeria launcher. Edit the login\r\n\
        -- servers in the launcher; this file is rewritten before every launch.\r\n\
        \r\n\
        LoginMod = {}\r\n\
        \r\n\
        function LoginMod.loadServerSystems()\r\n\
        \x20   LoginMod.servers = {}\r\n\
        \x20   LoginMod.servers[\"Desktop\"] = \"http://127.0.0.1:8081\"\r\n\
        \x20   LoginMod.servers[\"Cimmeria\"] = \"http://play.example:8081\"\r\n\
        end\r\n";

    #[test]
    fn parses_the_launcher_written_rows_in_order() {
        assert_eq!(
            parse(LAUNCHER_LUA),
            vec![
                LoginServer {
                    name: "Desktop".into(),
                    url: "http://127.0.0.1:8081".into()
                },
                LoginServer {
                    name: "Cimmeria".into(),
                    url: "http://play.example:8081".into()
                },
            ]
        );
    }

    #[test]
    fn skips_commented_out_rows() {
        let lua = "-- LoginMod.servers[\"Old\"] = \"http://old:8081\"\n\
                   LoginMod.servers[\"New\"]=\"http://new:8081\"\n";
        let rows = parse(lua);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "http://new:8081");
    }

    #[test]
    fn picks_the_named_row_case_insensitively() {
        let rows = parse(LAUNCHER_LUA);
        assert_eq!(
            pick(&rows, Some("cimmeria")).unwrap(),
            "http://play.example:8081"
        );
        assert_eq!(
            pick(&rows, Some("Desktop")).unwrap(),
            "http://127.0.0.1:8081"
        );
    }

    #[test]
    fn a_single_row_is_used_whatever_the_name() {
        let rows = vec![LoginServer {
            name: "Cimmeria".into(),
            url: "http://play.example:8081".into(),
        }];
        assert_eq!(
            pick(&rows, Some("colo")).unwrap(),
            "http://play.example:8081"
        );
        assert_eq!(pick(&rows, None).unwrap(), "http://play.example:8081");
    }

    #[test]
    fn several_rows_and_no_match_is_an_error_naming_them() {
        let err = pick(&parse(LAUNCHER_LUA), Some("colo")).unwrap_err();
        assert!(err.contains("Desktop, Cimmeria"), "{err}");
        assert!(err.contains("CIMMERIA_LAB_SERVER_URL"), "{err}");
        assert!(pick(&[], None).is_err());
    }

    #[test]
    fn login_url_reads_the_install_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = login_internal_path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, LAUNCHER_LUA).unwrap();
        assert_eq!(
            login_url(dir.path(), Some("Cimmeria")).unwrap(),
            "http://play.example:8081"
        );
        let missing = tempfile::tempdir().unwrap();
        assert!(login_url(missing.path(), None)
            .unwrap_err()
            .contains("LoginInternal.lua"));
    }
}
