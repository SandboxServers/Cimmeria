//! `LoginInternal.lua`: the login-server list the client offers.
//!
//! `Login.lua` calls `LoginMod.loadServerSystems()` when the file defines
//! it, and falls back to the dead CME shards otherwise. The stock client
//! ships a `LoginInternal.lua` pointing at CME's QA and production login
//! servers; the launcher replaces it with the configured list. Each entry
//! is a name shown in the login screen's dropdown and the auth server's
//! base URL (`http://<host>:8081` for a Cimmeria server).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::install_layout;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginServer {
    pub name: String,
    pub url: String,
}

/// The public Cimmeria server.
pub fn default_servers() -> Vec<LoginServer> {
    vec![LoginServer {
        name: "Cimmeria".into(),
        url: "http://play.cimmeria.app:8081".into(),
    }]
}

/// `<install>\Working\SGWGame\Content\UI\Startup\Login\LoginInternal.lua`.
pub fn path(install_dir: &Path) -> PathBuf {
    install_layout::sgwgame_dir(install_dir)
        .join("Content")
        .join("UI")
        .join("Startup")
        .join("Login")
        .join("LoginInternal.lua")
}

/// A name or URL goes into a Lua string literal, so it must not be able to
/// end the literal or the line. The client's UI Lua is ASCII.
fn valid_field(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && s.chars().all(|c| c.is_ascii_graphic() || c == ' ')
        && !s.contains(['"', '\\'])
}

/// Parse the settings text, one `Name = URL` per line. Blank lines and
/// lines starting with `#` are skipped.
pub fn parse(text: &str) -> Result<Vec<LoginServer>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, url) = line
            .split_once('=')
            .ok_or(format!("line {}: expected `Name = URL`", n + 1))?;
        let (name, url) = (name.trim(), url.trim());
        if !valid_field(name) || !valid_field(url) {
            return Err(format!(
                "line {}: names and URLs must be plain ASCII without quotes or backslashes",
                n + 1
            ));
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(format!(
                "line {}: the URL must start with http:// or https://",
                n + 1
            ));
        }
        out.push(LoginServer {
            name: name.into(),
            url: url.into(),
        });
    }
    if out.is_empty() {
        return Err("list at least one login server".into());
    }
    Ok(out)
}

/// The settings text for `servers`, the inverse of [`parse`].
pub fn to_text(servers: &[LoginServer]) -> String {
    servers
        .iter()
        .map(|s| format!("{} = {}", s.name, s.url))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The file's content, CRLF like the rest of the client's UI Lua.
pub fn render(servers: &[LoginServer]) -> String {
    let mut out = String::from(
        "-- LoginInternal.lua, written by the Cimmeria launcher. Edit the login\r\n\
         -- servers in the launcher; this file is rewritten before every launch.\r\n\
         \r\n\
         LoginMod = {}\r\n\
         \r\n\
         function LoginMod.loadServerSystems()\r\n\
         \x20   LoginMod.servers = {}\r\n",
    );
    for s in servers
        .iter()
        .filter(|s| valid_field(&s.name) && valid_field(&s.url))
    {
        out.push_str(&format!(
            "    LoginMod.servers[\"{}\"] = \"{}\"\r\n",
            s.name, s.url
        ));
    }
    out.push_str("end\r\n");
    out
}

/// Write the file when its content differs. Returns true when it wrote.
pub fn write(install_dir: &Path, servers: &[LoginServer]) -> std::io::Result<bool> {
    let path = path(install_dir);
    let content = render(servers);
    if std::fs::read(&path).is_ok_and(|current| current == content.as_bytes()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_server_as_a_crlf_lua_entry() {
        let lua = render(&[
            LoginServer {
                name: "Desktop".into(),
                url: "http://127.0.0.1:8081".into(),
            },
            LoginServer {
                name: "Cimmeria".into(),
                url: "http://play.cimmeria.app:8081".into(),
            },
        ]);
        assert!(lua.contains("function LoginMod.loadServerSystems()\r\n"));
        assert!(lua.contains("    LoginMod.servers[\"Desktop\"] = \"http://127.0.0.1:8081\"\r\n"));
        assert!(lua.contains("LoginMod.servers[\"Cimmeria\"]"));
        assert!(!lua.replace("\r\n", "").contains('\n'), "LF-only line");
        assert!(lua.is_ascii());
    }

    #[test]
    fn parse_round_trips_the_settings_text() {
        let servers =
            parse("# mine\nDesktop = http://127.0.0.1:8081\n\n Colo=http://example.org:8081 ")
                .unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[1].name, "Colo");
        assert_eq!(parse(&to_text(&servers)).unwrap(), servers);
    }

    // Bug shape: a quote or backslash would end the Lua string and let the
    // settings inject Lua into the client.
    #[test]
    fn parse_rejects_what_would_break_out_of_the_lua_string() {
        for bad in [
            "Evil\" .. os.exit() .. \" = http://x",
            "Name = http://x\\\"",
            "NoEquals",
            "Name = ftp://x",
            "",
            "# only a comment",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn write_puts_the_file_where_the_client_reads_it_and_only_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("Working").join("Binaries");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("SGW.exe"), b"MZ").unwrap();
        let expected = dir
            .path()
            .join("Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua");
        assert_eq!(path(dir.path()), expected);
        assert!(write(dir.path(), &default_servers()).unwrap());
        assert!(std::fs::read_to_string(&expected)
            .unwrap()
            .contains("play.cimmeria.app"));
        assert!(!write(dir.path(), &default_servers()).unwrap());
    }
}
