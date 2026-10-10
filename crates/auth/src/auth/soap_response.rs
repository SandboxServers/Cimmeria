//! Response XML for the Phase 1 and Phase 2 SOAP endpoints: the login
//! success and error envelopes, and the server-location reply that carries
//! the session key and ticket. Bare XML with no SOAP envelope, matching the
//! C++ `LogonConnection` output.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use super::{LOGIN_NS, SELECT_NS, XML_DECL};

pub(super) fn login_error(_code: u32, msg: &str) -> Response {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    // The client uses ErrorStr for display and ignores ErrorNum.
    let xml = format!(
        "{XML_DECL}\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginError ns3:ErrorStr=\"{msg}\" ns3:ErrorNum=\"1\" />\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

pub(super) fn login_success_xml(account_id: u32, shards: &[super::ShardInfo]) -> String {
    let entries: String = shards
        .iter()
        .map(|s| {
            format!(
                "<Shard ServerName=\"{}\" Fullness=\"LOW\" Busy=\"LOW\" />",
                s.name
            )
        })
        .collect();

    format!(
        "{XML_DECL}\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginSuccess>\
         <AccountInfo ExpireDate=\"0000-00-00T00:00:00.000Z\" AccountId=\"{account_id}\" />\
         <SGWShardListResp>{entries}</SGWShardListResp>\
         </SGWLoginSuccess>\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
    )
}

pub(super) fn select_error(_code: u32, msg: &str) -> Response {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    let xml = format!(
        "{XML_DECL}\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerSelectionError ns1:ErrorStr=\"{msg}\" ns1:ErrorNum=\"1\" />\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        xml,
    )
        .into_response()
}

pub(super) fn server_location_xml(
    shard: &super::ShardInfo,
    session_key: &str,
    ticket: &str,
) -> String {
    format!(
        "{XML_DECL}\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerLocation SessionKey=\"{session_key}\" Port=\"{port}\" IP=\"{ip}\" BWMailBox=\"1\">\
         <TICKET Ticket=\"{ticket}\" />\
         </ServerLocation>\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
        port = shard.port,
        ip = shard.host,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_success_xml_contains_shard() {
        let shards = vec![super::super::ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }];
        let xml = login_success_xml(42, &shards);
        assert!(xml.contains(r#"AccountId="42""#));
        assert!(xml.contains(r#"ServerName="Shard""#));
        // Bare XML — no SOAP envelope (matches C++ LogonConnection output).
        assert!(!xml.contains("SOAP-ENV:Envelope"));
        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));
        assert!(xml.contains("ns2:SGWLoginResponse"));
    }

    #[test]
    fn server_location_xml_contains_key_and_ticket() {
        let shard = super::super::ShardInfo {
            name: "Shard".into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        };
        let xml = server_location_xml(&shard, "AAAA", "BBBB");
        assert!(xml.contains(r#"SessionKey="AAAA""#));
        assert!(xml.contains(r#"Ticket="BBBB""#));
        assert!(xml.contains(r#"Port="32832""#));
    }
}
