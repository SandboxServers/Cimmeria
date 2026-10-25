//! `LoginInternal.lua` in ticket mode: the file a launch writes when the
//! player is signed in through the launcher (contract C8).
//!
//! The client's `Login.lua` (patch 017) reads `CimmeriaLogin` from this file
//! and signs in with the ticket instead of a typed password. Each string that
//! goes into the Lua literal is checked first: a quote or a backslash would
//! end the literal. A required field that fails its rule is an error; an
//! optional one (the display name, a link, an error text) is left out.

use std::path::Path;

use super::login_servers::{self, LoginServer};

/// `CimmeriaLogin.auto`: whether the login screen submits the ticket without
/// a press. Off until spike V2 passes (CT-57 flips it).
pub const TICKET_AUTO_SUBMIT: bool = false;

/// The `CimmeriaLogin.links` entries the server's `/providers` returned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links {
    pub website: Option<String>,
    pub account: Option<String>,
    pub forums: Option<String>,
}

/// Everything one ticket-mode `LoginInternal.lua` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketLogin {
    pub server_name: String,
    pub relay_url: String,
    pub account: String,
    pub ticket: String,
    pub display: String,
    pub links: Links,
    /// Error number and the text the login screen shows for it.
    pub errors: Vec<(u32, String)>,
}

/// The texts for the two failures a ticket login can show (C8): 4 is a
/// ticket the server no longer accepts, 5 is a suspended account.
pub fn default_errors() -> Vec<(u32, String)> {
    vec![
        (
            4,
            "Your sign-in for this launch is no longer valid. Close the game and press Play in the launcher."
                .into(),
        ),
        (5, "This account is suspended. Contact the server staff.".into()),
    ]
}

/// `s` trimmed, when it is a non-empty string of at most `max` characters
/// made of printable ASCII (0x20-0x7E) without `"` or `\`. `None` otherwise.
pub fn sanitize(s: &str, max: usize) -> Option<String> {
    let s = s.trim();
    let safe = !s.is_empty()
        && s.len() <= max
        && s.bytes()
            .all(|b| (0x20..=0x7e).contains(&b) && b != b'"' && b != b'\\');
    safe.then(|| s.to_string())
}

/// `3` to `20` letters, digits, `_` or `-`.
fn is_account(s: &str) -> bool {
    (3..=20).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Exactly 32 ASCII letters or digits.
fn is_ticket(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn line(out: &mut String, text: &str) {
    out.push_str(text);
    out.push_str("\r\n");
}

/// The C8 text for `t`, CRLF. An error names the field that broke its rule,
/// never the value.
pub fn render_ticket(t: &TicketLogin) -> Result<String, String> {
    let server = LoginServer {
        name: t.server_name.clone(),
        url: t.relay_url.clone(),
    };
    login_servers::validate(std::slice::from_ref(&server))
        .map_err(|_| "server_name or relay_url: not a valid login server".to_string())?;
    if !t.relay_url.starts_with("http://127.0.0.1:") {
        return Err("relay_url must start with http://127.0.0.1:".into());
    }
    if !is_account(&t.account) {
        return Err("account must be 3 to 20 letters, digits, _ or -".into());
    }
    if !is_ticket(&t.ticket) {
        return Err("ticket must be 32 ASCII letters or digits".into());
    }
    let display = sanitize(&t.display, 32).unwrap_or_else(|| t.account.clone());

    let mut out = String::new();
    line(
        &mut out,
        "-- LoginInternal.lua, written by the Cimmeria launcher for one launch.",
    );
    line(&mut out, "-- It is rewritten when the game exits.");
    line(&mut out, "");
    line(&mut out, "LoginMod = {}");
    line(&mut out, "");
    line(&mut out, "function LoginMod.loadServerSystems()");
    line(&mut out, "    LoginMod.servers = {}");
    line(
        &mut out,
        &format!(
            "    LoginMod.servers[\"{}\"] = \"{}\"",
            t.server_name, t.relay_url
        ),
    );
    line(&mut out, "    CimmeriaLogin = {}");
    line(&mut out, "    CimmeriaLogin.version = 1");
    line(&mut out, "    CimmeriaLogin.mode = \"ticket\"");
    line(
        &mut out,
        &format!("    CimmeriaLogin.server = \"{}\"", t.server_name),
    );
    line(
        &mut out,
        &format!("    CimmeriaLogin.account = \"{}\"", t.account),
    );
    line(
        &mut out,
        &format!("    CimmeriaLogin.ticket = \"{}\"", t.ticket),
    );
    line(
        &mut out,
        &format!("    CimmeriaLogin.display = \"{display}\""),
    );
    line(
        &mut out,
        &format!("    CimmeriaLogin.auto = {TICKET_AUTO_SUBMIT}"),
    );
    line(&mut out, "    CimmeriaLogin.links = {}");
    let links = [
        ("website", t.links.website.as_deref()),
        ("account", t.links.account.as_deref()),
        ("forums", t.links.forums.as_deref()),
    ];
    for (name, url) in links {
        let safe = url
            .and_then(|u| sanitize(u, 160))
            .filter(|u| u.starts_with("https://"));
        if let Some(url) = safe {
            line(
                &mut out,
                &format!("    CimmeriaLogin.links.{name} = \"{url}\""),
            );
        }
    }
    line(&mut out, "    CimmeriaLogin.errors = {}");
    for (num, text) in &t.errors {
        if let Some(text) = sanitize(text, 160) {
            line(
                &mut out,
                &format!("    CimmeriaLogin.errors[{num}] = \"{text}\""),
            );
        }
    }
    line(
        &mut out,
        "    if LoginMod.cimmeriaOverlayVersion == nil then",
    );
    line(
        &mut out,
        "        Login_AccountEdit:setText(CimmeriaLogin.account)",
    );
    line(
        &mut out,
        "        Login_PasswordEdit:setText(CimmeriaLogin.ticket)",
    );
    line(&mut out, "    end");
    line(&mut out, "end");
    Ok(out)
}

/// Write `t` to the install's `LoginInternal.lua`, through a temp file in the
/// same folder and a rename, so the client never reads half a file.
pub fn write_ticket(install_dir: &Path, t: &TicketLogin) -> std::io::Result<()> {
    let content =
        render_ticket(t).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    let path = login_servers::path(install_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("lua.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, &path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN: &str = include_str!(
        "../../../../data/client-patches/017-launcher-sign-in/test/LoginInternal.ticket.lua"
    );

    fn fixture() -> TicketLogin {
        TicketLogin {
            server_name: "Cimmeria".into(),
            relay_url: "http://127.0.0.1:50123".into(),
            account: "Player_1234".into(),
            ticket: "AbCdEf0123456789AbCdEf0123456789".into(),
            display: "Player".into(),
            links: Links::default(),
            errors: default_errors(),
        }
    }

    #[test]
    fn renders_the_golden_ticket_file() {
        assert_eq!(render_ticket(&fixture()).unwrap(), GOLDEN);
        assert!(GOLDEN.is_ascii());
        assert!(!GOLDEN.replace("\r\n", "").contains('\n'), "LF-only line");
    }

    // Bug shape: a ticket or account with a quote, or a relay URL off the
    // loopback relay, would reach the Lua unchecked.
    #[test]
    fn refuses_quotes_backslashes_and_bad_tickets() {
        let bad = [
            TicketLogin {
                ticket: "A".repeat(31),
                ..fixture()
            },
            TicketLogin {
                ticket: "AbCdEf012345678\"AbCdEf0123456789".into(),
                ..fixture()
            },
            TicketLogin {
                account: "a\"b".into(),
                ..fixture()
            },
            TicketLogin {
                relay_url: "http://example.org:1".into(),
                ..fixture()
            },
        ];
        for t in bad {
            assert!(render_ticket(&t).is_err(), "{t:?}");
        }
    }

    #[test]
    fn display_falls_back_to_the_account() {
        let t = TicketLogin {
            display: "\u{1F600}".into(),
            ..fixture()
        };
        let lua = render_ticket(&t).unwrap();
        assert!(lua.contains("    CimmeriaLogin.display = \"Player_1234\"\r\n"));
    }

    #[test]
    fn drops_unsafe_links_and_errors() {
        let t = TicketLogin {
            links: Links {
                website: Some("http://example.org/x".into()),
                account: Some("https://example.org/a\\b".into()),
                forums: Some("https://example.org/forums".into()),
            },
            errors: vec![(4, "Bad \\ text".into()), (5, "Suspended.".into())],
            ..fixture()
        };
        let lua = render_ticket(&t).unwrap();
        assert!(!lua.contains("links.website"));
        assert!(!lua.contains("links.account"));
        assert!(lua.contains("    CimmeriaLogin.links.forums = \"https://example.org/forums\"\r\n"));
        assert!(!lua.contains("errors[4]"));
        assert!(lua.contains("    CimmeriaLogin.errors[5] = \"Suspended.\"\r\n"));
    }

    // CT-57 flips the flag after spike V2 passes; this test then fails on
    // purpose, so the flip is a reviewed change.
    #[test]
    fn auto_submit_stays_off_until_v2() {
        let auto = TICKET_AUTO_SUBMIT;
        assert!(!auto, "CT-57 flips TICKET_AUTO_SUBMIT after spike V2");
        let lua = render_ticket(&fixture()).unwrap();
        assert!(lua.contains("    CimmeriaLogin.auto = false\r\n"));
    }

    #[test]
    fn write_ticket_replaces_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        login_servers::write(&root, &login_servers::default_servers()).unwrap();
        write_ticket(&root, &fixture()).unwrap();
        let path = login_servers::path(&root);
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            render_ticket(&fixture()).unwrap()
        );
        assert!(!path.with_extension("lua.tmp").exists());
    }
}
