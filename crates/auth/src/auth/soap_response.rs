//! Response XML for the Phase 1 and Phase 2 SOAP endpoints: the login
//! success and error envelopes, and the server-location reply that carries
//! the session key and ticket. Bare XML with no SOAP envelope, matching the
//! C++ `LogonConnection` output.

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

use super::{LOGIN_NS, SELECT_NS, XML_DECL};

/// Escape an interpolated value for a double-quoted XML attribute, so a value
/// cannot end the attribute or open an element. Inside a double-quoted
/// attribute at least `"` and `&` need escaping; `quick_xml::escape` also
/// escapes `<`, `>` and `'`.
fn attr(s: &str) -> std::borrow::Cow<'_, str> {
    quick_xml::escape::escape(s)
}

pub(super) fn login_error(_code: u32, msg: &str) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        login_error_xml(msg),
    )
        .into_response()
}

/// The Phase 1 error envelope. `msg` is escaped.
pub(super) fn login_error_xml(msg: &str) -> String {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    // The client uses ErrorStr for display and ignores ErrorNum.
    format!(
        "{XML_DECL}\
         <ns2:SGWLoginResponse {ns}>\
         <SGWLoginError ns3:ErrorStr=\"{msg}\" ns3:ErrorNum=\"1\" />\
         </ns2:SGWLoginResponse>",
        ns = LOGIN_NS,
        msg = attr(msg),
    )
}

pub(super) fn login_success_xml(account_id: u32, shards: &[super::ShardInfo]) -> String {
    let entries: String = shards
        .iter()
        .map(|s| {
            format!(
                "<Shard ServerName=\"{}\" Fullness=\"LOW\" Busy=\"LOW\" />",
                attr(&s.name)
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
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml".to_string())],
        select_error_xml(msg),
    )
        .into_response()
}

/// The Phase 2 error envelope. `msg` is escaped.
pub(super) fn select_error_xml(msg: &str) -> String {
    // C++ always sends ErrorNum="1" regardless of the actual FailureCode.
    format!(
        "{XML_DECL}\
         <ns3:SGWServerLocationResponse {ns}>\
         <ServerSelectionError ns1:ErrorStr=\"{msg}\" ns1:ErrorNum=\"1\" />\
         </ns3:SGWServerLocationResponse>",
        ns = SELECT_NS,
        msg = attr(msg),
    )
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
        ip = attr(&shard.host),
        session_key = attr(session_key),
        ticket = attr(ticket),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::{Reader, XmlVersion};

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

    /// Hostile value: unescaped, the first `"` ends the attribute and the rest
    /// opens an `Evil` element.
    const EVIL: &str = r#"x" Injected="1"/><Evil a="&"#;

    /// One start or empty element: its local name and its attributes, keyed by
    /// qualified name and holding decoded values.
    struct Elem {
        name: String,
        attrs: Vec<(String, String)>,
    }

    /// Parse `xml` in a `read_event()` loop until `Eof`, collecting every start
    /// and empty element. Panics on any parse error.
    fn elements(xml: &str) -> Vec<Elem> {
        let mut reader = Reader::from_str(xml);
        let mut out = Vec::new();
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let mut attrs = Vec::new();
                    for a in e.attributes() {
                        let a = a.unwrap_or_else(|err| panic!("attribute must parse: {err}"));
                        let key = a.key.as_ref().to_owned();
                        let value = a
                            .normalized_value(XmlVersion::Implicit1_0)
                            .unwrap_or_else(|err| panic!("value must decode: {err}"))
                            .into_owned();
                        attrs.push((key, value));
                    }
                    out.push(Elem {
                        name: e.local_name().as_ref().to_owned(),
                        attrs,
                    });
                }
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(err) => panic!("xml must parse: {err}\n{xml}"),
            }
        }
        out
    }

    /// The first element named `name`.
    fn find<'a>(elems: &'a [Elem], name: &str) -> &'a Elem {
        elems
            .iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no {name} element"))
    }

    /// The decoded value of the attribute with qualified name `key`.
    fn value_of(elem: &Elem, key: &str) -> String {
        elem.attrs
            .iter()
            .find(|(k, _)| k.as_str() == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("{} has no {key} attribute", elem.name))
    }

    #[test]
    fn login_success_xml_escapes_the_shard_name() {
        let shards = vec![super::super::ShardInfo {
            name: EVIL.into(),
            host: "127.0.0.1".into(),
            port: 32832,
            protected: false,
        }];
        let elems = elements(&login_success_xml(42, &shards));
        assert!(
            elems.iter().all(|e| e.name != "Evil"),
            "no injected element"
        );
        assert_eq!(value_of(find(&elems, "Shard"), "ServerName"), EVIL);
    }

    #[test]
    fn server_location_xml_escapes_host_key_and_ticket() {
        let shard = super::super::ShardInfo {
            name: "Shard".into(),
            host: EVIL.into(),
            port: 32832,
            protected: false,
        };
        let elems = elements(&server_location_xml(&shard, EVIL, EVIL));
        assert!(
            elems.iter().all(|e| e.name != "Evil"),
            "no injected element"
        );
        let location = find(&elems, "ServerLocation");
        assert_eq!(value_of(location, "SessionKey"), EVIL);
        assert_eq!(value_of(location, "IP"), EVIL);
        assert_eq!(value_of(find(&elems, "TICKET"), "Ticket"), EVIL);
    }

    #[test]
    fn error_xml_escapes_the_message() {
        let elems = elements(&login_error_xml(EVIL));
        assert!(
            elems.iter().all(|e| e.name != "Evil"),
            "no injected element"
        );
        assert_eq!(
            value_of(find(&elems, "SGWLoginError"), "ns3:ErrorStr"),
            EVIL
        );

        let elems = elements(&select_error_xml(EVIL));
        assert!(
            elems.iter().all(|e| e.name != "Evil"),
            "no injected element"
        );
        assert_eq!(
            value_of(find(&elems, "ServerSelectionError"), "ns1:ErrorStr"),
            EVIL
        );
    }
}
