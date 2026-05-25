//! SOAP authentication driver — Phase 1 + Phase 2 of the SGW login.
//!
//! ## The exchange
//!
//! ```text
//! ┌──────────┐  POST /SGWLogin/UserAuth          ┌──────────────┐
//! │          │ ────────────────────────────────▶ │              │
//! │          │   SGWLoginRequest (creds + SKU)   │              │
//! │ wireclnt │                                   │  AuthService │
//! │          │ ◀──────────────────────────────── │              │
//! │          │   SGWLoginResponse + SID cookie   │              │
//! │          │                                   │              │
//! │          │  POST /SGWLogin/ServerSelection   │              │
//! │          │ ────────────────────────────────▶ │              │
//! │          │   SGWSelectServerRequest + SID    │              │
//! │          │ ◀──────────────────────────────── │              │
//! │          │   ServerLocationResponse          │              │
//! │          │   (SessionKey + Ticket + IP/Port) │              │
//! └──────────┘                                   └──────────────┘
//! ```
//!
//! Phase 1 hands back a single-use `SID` session cookie. Phase 2 returns the
//! load-bearing four-tuple the Mercury handshake needs:
//!
//! - `SessionKey` (64 hex chars / 32 bytes) — AES-256 key for the Mercury channel.
//! - `Ticket` (20 ASCII chars) — opaque value the server matches against its
//!   pending-logins map on receipt of `baseAppLogin`.
//! - `IP` / `Port` — BaseApp UDP endpoint to dial.
//!
//! This driver makes the same requests `cimmeria_services::auth::login_smoke`
//! exercises server-side, so a regression on either end surfaces in both
//! suites.
//!
//! ## Parser stance
//!
//! Phase 1 and Phase 2 responses are well-defined fixed-shape XML. Rather
//! than pull in a SOAP envelope parser, we extract the four attributes by
//! string scan — identical strategy to `login_smoke`'s `extract_attr`. The
//! response shapes are pinned by server-side handler tests; a drift there
//! would be caught before it reached us.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use cimmeria_mercury::encryption::MercuryEncryption;
use reqwest::header::{HeaderValue, SET_COOKIE};

use crate::error::{Error, Result};

/// Credentials for SOAP Phase 1.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub username: String,
    /// 40-character uppercase hex SHA-1 of the password. The auth server
    /// compares case-insensitively, but the C++ client always uppercases —
    /// we follow suit to keep wire dumps identical.
    pub password_sha1_hex: String,
    /// Stable across all Cimmeria clients. Reverse-engineered from SGW.exe;
    /// see `docs/protocol/` for derivation.
    pub protocol_digest: String,
    /// SKU tag — production used `"SGW_BETA"`; the auth server does not
    /// validate it, but recording it makes captures diffable.
    pub sku: String,
}

impl Credentials {
    /// Convenience constructor for the canonical test account
    /// (`account_name = "test"`, `password = "test"`).
    pub fn test_account() -> Self {
        Self {
            username: "test".into(),
            // SHA-1("test") uppercased.
            password_sha1_hex: "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3".into(),
            protocol_digest: "58AFA196AD3AC4F65CADD99BFF23B799".into(),
            sku: "SGW_BETA".into(),
        }
    }
}

/// What auth Phases 1 + 2 returned. Carries everything the Mercury
/// handshake needs in one record, including the account_id parsed from
/// the Phase 1 `<AccountInfo AccountId="…" />` element so downstream
/// callers don't have to thread it out of band.
#[derive(Debug, Clone)]
pub struct AuthSession {
    /// BaseApp UDP endpoint to dial.
    pub base_addr: SocketAddr,
    /// 20-character ASCII ticket — sent verbatim in `baseAppLogin`.
    pub ticket: String,
    /// 32-byte AES-256 key for Mercury encryption.
    pub session_key: [u8; 32],
    /// Account ID from Phase 1's `<AccountInfo AccountId="…">` — required
    /// by `build_baseapp_login`. The server cross-checks the value
    /// against its pending-logins map on receipt of `baseAppLogin`;
    /// mismatch = silent reject.
    pub account_id: u32,
}

impl AuthSession {
    /// Build a `MercuryEncryption` context from the session key.
    pub fn encryption(&self) -> MercuryEncryption {
        MercuryEncryption::from_session_key(self.session_key)
    }
}

/// Default per-request timeout for SOAP calls. The auth server replies
/// in milliseconds in the happy path; 10 seconds is generous enough to
/// absorb a one-off GC pause but short enough that a hung endpoint fails
/// the test instead of wedging the suite.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// SOAP auth client.
///
/// Holds a shared `reqwest::Client` so connection reuse and TLS handshake
/// state cache across Phase 1 and Phase 2.
pub struct AuthClient {
    http: reqwest::Client,
    base_url: String,
}

impl AuthClient {
    /// Construct a client targeting `base_url` (e.g. `http://127.0.0.1:8081`).
    /// No trailing slash; the driver appends `/SGWLogin/...` itself.
    ///
    /// The shared HTTP client carries a 10-second per-request timeout
    /// ([`DEFAULT_REQUEST_TIMEOUT`]) — a stalled auth endpoint fails
    /// fast rather than wedging `phase1()`/`phase2()` indefinitely.
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_REQUEST_TIMEOUT)
            .build()
            .expect("default reqwest::Client builder must succeed");
        Self {
            http,
            base_url: base_url.into(),
        }
    }

    /// Run Phase 1 + Phase 2 against the configured auth server and the
    /// named shard. Returns the [`AuthSession`] needed by the Mercury
    /// handshake.
    pub async fn login(&self, creds: &Credentials, shard: &str) -> Result<AuthSession> {
        let (sid, account_id) = self.phase1(creds).await?;
        self.phase2(&sid, account_id, shard).await
    }

    /// Phase 1 — POST `/SGWLogin/UserAuth`. Returns the SID cookie and
    /// the server-assigned `account_id` parsed from
    /// `<AccountInfo AccountId="…" />`. The `account_id` is required by
    /// the Mercury phase-3 handshake; surfacing it here keeps the
    /// auth → handshake flow self-contained.
    pub async fn phase1(&self, creds: &Credentials) -> Result<(String, u32)> {
        const ENDPOINT: &str = "/SGWLogin/UserAuth";
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="{sku}" AccountName="{user}" Password="{pw}" ProtocolDigest="{digest}" />"#,
            sku = xml_attr_escape(&creds.sku),
            user = xml_attr_escape(&creds.username),
            pw = xml_attr_escape(&creds.password_sha1_hex),
            digest = xml_attr_escape(&creds.protocol_digest),
        );
        let resp = self
            .http
            .post(format!("{}{ENDPOINT}", self.base_url))
            .header("Content-Type", "text/xml")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Error::SoapStatus {
                status: resp.status().as_u16(),
                endpoint: ENDPOINT,
            });
        }

        let sid = resp
            .headers()
            .get(SET_COOKIE)
            .and_then(parse_sid_from_set_cookie)
            .ok_or(Error::NoSidCookie)?;

        // Parse account_id out of the response body — `<AccountInfo
        // AccountId="42" />`. The auth server emits this on every
        // successful login; absence implies an error envelope we should
        // surface rather than silently masking.
        let xml = resp.text().await?;
        let account_id_s =
            extract_attr(&xml, "AccountId").ok_or(Error::MissingAttribute("AccountId"))?;
        let account_id: u32 = account_id_s
            .parse()
            .map_err(|_| Error::MissingAttribute("AccountId"))?;
        Ok((sid, account_id))
    }

    /// Phase 2 — POST `/SGWLogin/ServerSelection` with the SID cookie.
    /// Takes the `account_id` from Phase 1 and threads it onto the
    /// returned [`AuthSession`].
    pub async fn phase2(&self, sid: &str, account_id: u32, shard: &str) -> Result<AuthSession> {
        const ENDPOINT: &str = "/SGWLogin/ServerSelection";
        let body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="{shard}" />"#,
            shard = xml_attr_escape(shard),
        );
        let resp = self
            .http
            .post(format!("{}{ENDPOINT}", self.base_url))
            .header("Content-Type", "text/xml")
            .header("Cookie", format!("SID={sid}"))
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Error::SoapStatus {
                status: resp.status().as_u16(),
                endpoint: ENDPOINT,
            });
        }

        let xml = resp.text().await?;
        // Both success and error responses wrap in `<…SGWServerLocationResponse>`,
        // so substring-matching "ServerLocation" alone reports false success
        // on `<ServerSelectionError>`. Detect the error envelope explicitly,
        // and require the success element `<ServerLocation ` with its
        // trailing space (the wrapping `SGWServerLocationResponse` lacks one).
        if xml.contains("ServerSelectionError") || !xml.contains("<ServerLocation ") {
            return Err(Error::NoServerLocation);
        }

        let session_key_hex =
            extract_attr(&xml, "SessionKey").ok_or(Error::MissingAttribute("SessionKey"))?;
        let ticket = extract_attr(&xml, "Ticket").ok_or(Error::MissingAttribute("Ticket"))?;
        let ip = extract_attr(&xml, "IP").ok_or(Error::MissingAttribute("IP"))?;
        let port_s = extract_attr(&xml, "Port").ok_or(Error::MissingAttribute("Port"))?;

        // Length checks before hex decode so the error variant is specific.
        if session_key_hex.len() != 64 {
            return Err(Error::InvalidSessionKeyLength(session_key_hex.len()));
        }
        if ticket.len() != 20 {
            return Err(Error::InvalidTicketLength(ticket.len()));
        }

        let key_vec = hex::decode(&session_key_hex)?;
        let mut session_key = [0u8; 32];
        session_key.copy_from_slice(&key_vec);

        let port: u16 = port_s
            .parse()
            .map_err(|_| Error::InvalidPort(port_s.clone()))?;
        let ip_addr: IpAddr = ip.parse().map_err(|_| Error::InvalidIp(ip.clone()))?;

        Ok(AuthSession {
            base_addr: SocketAddr::new(ip_addr, port),
            ticket,
            session_key,
            account_id,
        })
    }
}

/// Pull `SID=<value>` out of a `Set-Cookie` header.
///
/// Visible for testing — the smoke and unit tests both want to assert on the
/// raw cookie shape so a regression that introduces newlines or extra
/// attributes surfaces here, not in the encrypted layer downstream.
pub fn parse_sid_from_set_cookie(h: &HeaderValue) -> Option<String> {
    let s = h.to_str().ok()?;
    s.split(';').find_map(|p| {
        let p = p.trim();
        p.strip_prefix("SID=").map(str::to_string)
    })
}

/// Locate `key="value"` in the response XML. Same strategy as
/// `login_smoke::extract_attr` — fixed-shape responses don't warrant a full
/// XML reader.
pub fn extract_attr(xml: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = xml.find(&needle)? + needle.len();
    let end = start + xml[start..].find('"')?;
    Some(xml[start..end].to_string())
}

/// XML attribute escape for the five characters quick-xml requires escaped
/// inside double-quoted attribute values. The auth credentials we send are
/// all hex / SKU strings today, but escaping here is cheap insurance against
/// a future caller threading a username that contains `&` or `"`.
fn xml_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn extract_attr_finds_session_key() {
        let xml = r#"<ServerLocation SessionKey="DEADBEEF" Port="32832" />"#;
        assert_eq!(extract_attr(xml, "SessionKey").as_deref(), Some("DEADBEEF"));
        assert_eq!(extract_attr(xml, "Port").as_deref(), Some("32832"));
    }

    #[test]
    fn extract_attr_missing_returns_none() {
        let xml = r#"<ServerLocation Port="32832" />"#;
        assert!(extract_attr(xml, "SessionKey").is_none());
    }

    #[test]
    fn parse_sid_extracts_value_and_ignores_other_attrs() {
        let h = HeaderValue::from_static("SID=abc123def456; Path=/; HttpOnly");
        assert_eq!(
            parse_sid_from_set_cookie(&h).as_deref(),
            Some("abc123def456")
        );
    }

    #[test]
    fn parse_sid_returns_none_when_no_sid() {
        let h = HeaderValue::from_static("OtherCookie=value; Path=/");
        assert!(parse_sid_from_set_cookie(&h).is_none());
    }

    #[test]
    fn xml_attr_escape_handles_metacharacters() {
        assert_eq!(
            xml_attr_escape(r#"a & b < c > d " e ' f"#),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f"
        );
    }

    #[test]
    fn credentials_test_account_uses_sha1_of_test() {
        let c = Credentials::test_account();
        assert_eq!(c.username, "test");
        assert_eq!(c.password_sha1_hex.len(), 40);
        // SHA-1("test") = a94a8fe5ccb19ba61c4c0873d391e987982fbbd3 (canonical).
        assert!(c
            .password_sha1_hex
            .eq_ignore_ascii_case("a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"));
    }
}
