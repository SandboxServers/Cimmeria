//! Game telemetry over https only (LI-09, D-LI11): the mint, refresh and upload
//! addresses an http login server gets, and the refusals that remain.
use super::endpoint_sanity::is_loopback_url;
use super::session::{auth_base_url, mint_url, upload_for};
use crate::telemetry_endpoint::{EndpointError, EndpointPolicy};

/// Revert test for the derivation (D-LI15): an http login server on a DNS name is
/// asked over https on 443, under the same name.
#[test]
fn a_plain_http_login_server_on_a_dns_name_is_asked_over_https() {
    assert_eq!(
        mint_url("http://play.example:8081"),
        "https://play.example/api/auth/dev-session"
    );
    assert_eq!(
        auth_base_url("http://play.example:8081"),
        "https://play.example/api"
    );
}

#[test]
fn other_login_servers_keep_their_address() {
    assert_eq!(
        auth_base_url("https://play.example:9443"),
        "https://play.example:9443/api"
    );
    assert_eq!(
        auth_base_url("http://localhost:8081"),
        "http://localhost:8081/api"
    );
    assert_eq!(
        auth_base_url("http://192.168.1.10:8081"),
        "http://192.168.1.10:8081/api"
    );
}

/// An IP or LAN login server cannot be upgraded, so its mint is refused rather than
/// sent in plaintext (D-LI16).
#[test]
fn an_ip_or_lan_login_server_is_refused_not_sent_in_plaintext() {
    for server in ["http://192.168.1.10:8081", "http://sgwserver:8081"] {
        let policy = EndpointPolicy::from_login_servers([server]);
        assert!(
            matches!(
                policy.check(&mint_url(server)),
                Err(EndpointError::InsecureRemote(_))
            ),
            "{server}"
        );
    }
}

/// Revert test for the upload path (D-LI16): a plain-http upload URL the server hands
/// back is refused, never rewritten.
#[test]
fn a_plain_http_upload_url_from_the_server_is_refused() {
    let server = "http://play.example:8081";
    let policy = EndpointPolicy::from_login_servers([server]);
    assert_eq!(
        upload_for(
            &policy,
            server,
            "http://play.example:8081/api/telemetry",
            "game"
        ),
        Err("endpoint_refused")
    );
}

#[test]
fn an_ip_login_server_with_a_loopback_upload_has_no_usable_address() {
    let server = "http://192.168.1.10:8081";
    let policy = EndpointPolicy::from_login_servers([server]);
    assert_eq!(
        upload_for(
            &policy,
            server,
            "http://localhost:8443/api/telemetry",
            "game"
        ),
        Err("no_https_endpoint")
    );
}

/// Every mint URL built from a login server is https, loopback, or refused by the
/// policy. Covers the refused list of `https_upgrade_origin` and the two upgrades.
#[test]
fn no_derived_mint_url_is_plain_http_off_this_machine() {
    let login_servers = [
        "http://127.0.0.1:8081",
        "http://192.168.1.10:8081",
        "http://[2001:db8::1]:8081",
        "http://localhost:8081",
        "http://game.localhost:8081",
        "http://sgwserver:8081",
        "http://play.example.:8081",
        "http://user@play.example:8081",
        "http://u:p@play.example",
        "https://play.example",
        "ftp://play.example",
        "http://play.example:8081",
    ];
    for server in login_servers {
        let mint = mint_url(server);
        let policy = EndpointPolicy::from_login_servers([server]);
        assert!(
            mint.starts_with("https://") || is_loopback_url(&mint) || policy.check(&mint).is_err(),
            "{server} gives {mint}"
        );
    }
}
