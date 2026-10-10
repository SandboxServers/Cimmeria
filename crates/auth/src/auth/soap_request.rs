//! Parsers for the two SOAP request bodies the client POSTs: Phase 1
//! `SGWLoginRequest` and Phase 2 `SGWSelectServerRequest`.
//!
//! Every attribute value the server reads is XML-decoded exactly once by
//! quick-xml's own attribute-value normalization (XML 1.0 §3.3.3): the five
//! predefined entities (`&amp;` `&lt;` `&gt;` `&quot;` `&apos;`) and character
//! references (`&#38;`, `&#x26;`) are replaced, and a raw tab, CR or LF becomes
//! a space. A client that escapes a plaintext password `a&b` as `a&amp;b` is
//! therefore verified against `a&b` (#1289). Decoding is single-pass, so
//! `&amp;amp;` yields the literal text `&amp;`.
//!
//! Malformed syntax is rejected rather than passed through: an unknown entity
//! (`&bogus;`), a bare `&` with no `;`, a bad character reference, or broken
//! attribute syntax on the request element (including a duplicated
//! attribute) fails the parse. The [`SoapRequestError`] carries a static
//! reason and the attribute *name* only, never any part of the value: the
//! value may be a password, and quick-xml's own error text quotes the
//! offending entity name.

use std::fmt;

use quick_xml::escape::EscapeError;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::LoginReq;

/// Why a SOAP request body could not be parsed.
///
/// Safe to log in full: neither variant holds any byte of an attribute value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SoapRequestError {
    /// The expected request element never appeared (or the document broke
    /// before it did).
    ElementNotFound(&'static str),
    /// A required attribute was absent. Only `ServerSelection` is required at
    /// parse time; Phase 1 attributes are validated by the handler.
    MissingAttribute(&'static str),
    /// The request element's attribute list is syntactically broken
    /// (unquoted value, missing `=`, duplicated attribute, ...).
    MalformedAttributes,
    /// An attribute value holds malformed entity or character-reference
    /// syntax, or is not valid UTF-8.
    BadAttributeValue {
        attribute: &'static str,
        reason: &'static str,
    },
}

impl SoapRequestError {
    /// Stable, low-cardinality reason code for the `reason` log field.
    pub(super) fn reason(&self) -> &'static str {
        match self {
            Self::ElementNotFound(_) => "element_not_found",
            Self::MissingAttribute(_) => "missing_attribute",
            Self::MalformedAttributes => "malformed_attribute_syntax",
            Self::BadAttributeValue { reason, .. } => reason,
        }
    }

    /// The attribute (or element) the error concerns, for the `attribute`
    /// log field. Empty when the error is about the attribute list as a whole.
    pub(super) fn attribute(&self) -> &'static str {
        match self {
            Self::ElementNotFound(element) => element,
            Self::MissingAttribute(attribute) => attribute,
            Self::MalformedAttributes => "",
            Self::BadAttributeValue { attribute, .. } => attribute,
        }
    }
}

impl fmt::Display for SoapRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElementNotFound(element) => write!(f, "{element} element not found"),
            Self::MissingAttribute(attribute) => write!(f, "{attribute} attribute missing"),
            Self::MalformedAttributes => f.write_str("malformed attribute syntax"),
            Self::BadAttributeValue { attribute, reason } => {
                write!(f, "{attribute} attribute value rejected: {reason}")
            }
        }
    }
}

/// Map a quick-xml value error to a reason code that names the defect class
/// without quoting the value.
fn value_error_reason(err: &quick_xml::Error) -> &'static str {
    match err {
        quick_xml::Error::Escape(EscapeError::UnrecognizedEntity(..)) => "unrecognized_entity",
        quick_xml::Error::Escape(EscapeError::UnterminatedEntity(..)) => "unterminated_entity",
        quick_xml::Error::Escape(EscapeError::InvalidCharRef(..)) => "invalid_char_ref",
        quick_xml::Error::Escape(EscapeError::TooManyNestedEntities) => "too_many_nested_entities",
        quick_xml::Error::Encoding(..) => "invalid_utf8",
        _ => "malformed_value",
    }
}

/// Decode one attribute value exactly once.
fn decode_value(attr: &Attribute<'_>, name: &'static str) -> Result<String, SoapRequestError> {
    attr.normalized_value(XmlVersion::Implicit1_0)
        .map(|v| v.into_owned())
        .map_err(|e| SoapRequestError::BadAttributeValue {
            attribute: name,
            reason: value_error_reason(&e),
        })
}

/// Walk the request element's attributes, handing each recognised one (by
/// its static name) to `on_attr` after decoding. Unrecognised attributes are
/// skipped undecoded; a syntax error anywhere in the list fails the parse.
fn for_each_known_attribute(
    element: &BytesStart<'_>,
    known: &[&'static str],
    mut on_attr: impl FnMut(&'static str, String),
) -> Result<(), SoapRequestError> {
    for attr in element.attributes() {
        let attr = attr.map_err(|_| SoapRequestError::MalformedAttributes)?;
        let Some(&name) = known.iter().find(|k| k.as_bytes() == attr.key.as_ref()) else {
            continue;
        };
        on_attr(name, decode_value(&attr, name)?);
    }
    Ok(())
}

/// Find the first `local_name` element (start or empty) in `body` and run
/// `with_element` on it.
fn with_request_element<T>(
    body: &str,
    local_name: &'static str,
    with_element: impl FnOnce(&BytesStart<'_>) -> Result<T, SoapRequestError>,
) -> Result<T, SoapRequestError> {
    let mut reader = Reader::from_str(body);
    loop {
        match reader.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e))
                if e.local_name().as_ref() == local_name.as_bytes() =>
            {
                return with_element(&e);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    Err(SoapRequestError::ElementNotFound(local_name))
}

/// Parse the Phase 1 `SGWLoginRequest`. Missing attributes surface as empty
/// strings; the handler owns field validation.
pub(super) fn parse_login_request(body: &str) -> Result<LoginReq, SoapRequestError> {
    with_request_element(body, "SGWLoginRequest", |element| {
        let mut req = LoginReq::default();
        for_each_known_attribute(
            element,
            &["SKU", "AccountName", "Password", "ProtocolDigest"],
            |name, value| match name {
                "SKU" => req.sku = value,
                "AccountName" => req.account_name = value,
                "Password" => req.password = value,
                _ => req.protocol_digest = value,
            },
        )?;
        Ok(req)
    })
}

/// Parse the Phase 2 `SGWSelectServerRequest` and return its decoded
/// `ServerSelection` (the shard name).
pub(super) fn parse_server_selection(body: &str) -> Result<String, SoapRequestError> {
    with_request_element(body, "SGWSelectServerRequest", |element| {
        let mut selection = None;
        for_each_known_attribute(element, &["ServerSelection"], |_, value| {
            selection = Some(value);
        })?;
        selection.ok_or(SoapRequestError::MissingAttribute("ServerSelection"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN_PREFIX: &str = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="test" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" "#;

    /// A login request whose `Password` attribute is `raw_password`, spelled
    /// exactly as it appears between the quotes on the wire.
    fn login_body(raw_password: &str) -> String {
        format!(r#"{LOGIN_PREFIX}Password="{raw_password}" />"#)
    }

    /// The parse error for `body`. `LoginReq` deliberately has no `Debug`
    /// (it holds the password), so `unwrap_err` is unavailable.
    fn login_err(body: &str) -> SoapRequestError {
        match parse_login_request(body) {
            Ok(_) => panic!("expected the login request to be rejected"),
            Err(e) => e,
        }
    }

    #[test]
    fn parse_valid_login_request() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="test" Password="A94A8FE5CCB19BA61C4C0873D391E987982FBBD3" ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let req = parse_login_request(body).unwrap();
        assert_eq!(req.sku, "SGW_BETA");
        assert_eq!(req.account_name, "test");
        assert_eq!(req.password, "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3");
        assert_eq!(req.protocol_digest, "58AFA196AD3AC4F65CADD99BFF23B799");
    }

    #[test]
    fn parse_server_selection_request() {
        let body = r#"<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" ServerSelection="Shard" />"#;
        let sel = parse_server_selection(body).unwrap();
        assert_eq!(sel, "Shard");
    }

    /// Pin the parser's narrow contract: when the SGWLoginRequest element
    /// is present but required attributes (SKU/AccountName/Password) are
    /// missing, the parse succeeds with default (empty) fields. The
    /// handler validates above the parser; a refactor that pushed
    /// validation down here would silently break that layering.
    #[test]
    fn parse_login_request_does_not_validate_missing_attributes() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" />"#;
        let req = parse_login_request(body).expect("element present must parse Ok");
        assert_eq!(req.sku, "", "missing SKU must surface as empty, not error");
        assert_eq!(req.account_name, "");
        assert_eq!(req.password, "");
    }

    /// Same narrow-contract pin, but for password length: a password the
    /// handler will reject (not 40 hex chars) parses through unchanged.
    /// Validation lives in the handler — this guard keeps it there.
    #[test]
    fn parse_login_request_does_not_validate_password_length() {
        let req =
            parse_login_request(&login_body("short")).expect("well-formed element must parse Ok");
        assert_eq!(
            req.password, "short",
            "parser must hand the password through; length check is the handler's job",
        );
    }

    /// Unlike parse_login_request, parse_server_selection requires the
    /// ServerSelection attribute. A refactor that loosened this would let
    /// the auth flow accept a server-selection message with no shard id.
    #[test]
    fn parse_server_selection_missing_attribute_returns_error() {
        let body = r#"<sgwLogin:SGWSelectServerRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" />"#;
        assert_eq!(
            parse_server_selection(body),
            Err(SoapRequestError::MissingAttribute("ServerSelection"))
        );
    }

    #[test]
    fn missing_element_is_element_not_found() {
        assert_eq!(
            login_err("<Other />"),
            SoapRequestError::ElementNotFound("SGWLoginRequest")
        );
    }

    // ── #1289: XML-escaped attribute values ──────────────────────────────

    /// **#1289 regression guard.** Each predefined entity and both
    /// character-reference spellings decode to the character they stand
    /// for. Before the fix the parser handed the escaped wire spelling
    /// through, so `a&amp;b` was compared against a stored `a&b` and failed.
    #[test]
    fn password_entities_and_char_refs_decode() {
        for (wire, decoded) in [
            ("a&amp;b", "a&b"),
            ("a&lt;b", "a<b"),
            ("a&gt;b", "a>b"),
            ("a&quot;b", "a\"b"),
            ("a&apos;b", "a'b"),
            ("a&#38;b", "a&b"),
            ("a&#x26;b", "a&b"),
            ("&lt;&amp;&quot;&#x41;&#66;", "<&\"AB"),
        ] {
            let req = parse_login_request(&login_body(wire))
                .unwrap_or_else(|e| panic!("{wire:?} must parse, got {e}"));
            assert_eq!(req.password, decoded, "wire spelling {wire:?}");
        }
    }

    /// Decoding happens exactly once: `&amp;amp;` is the escaped spelling of
    /// the literal text `&amp;`, not of `&`.
    #[test]
    fn double_escaped_password_decodes_once() {
        let req = parse_login_request(&login_body("p&amp;amp;w")).unwrap();
        assert_eq!(req.password, "p&amp;w");
    }

    /// A character reference survives normalization (`&#9;` stays a tab),
    /// while a raw tab in the value is normalized to a space per XML 1.0
    /// §3.3.3. Pins that quick-xml's normalization, not a bare unescape, is
    /// what runs.
    #[test]
    fn char_ref_whitespace_survives_raw_whitespace_normalizes() {
        let req = parse_login_request(&login_body("a&#9;b\tc")).unwrap();
        assert_eq!(req.password, "a\tb c");
    }

    /// Malformed entity syntax is rejected with a reason code, never passed
    /// through as literal text.
    #[test]
    fn malformed_entities_are_rejected() {
        for (wire, reason) in [
            ("a&bogus;b", "unrecognized_entity"),
            ("a&b", "unterminated_entity"),
            ("trailing&", "unterminated_entity"),
            ("a&#xZZ;b", "invalid_char_ref"),
            ("a&#0;b", "invalid_char_ref"),
        ] {
            let err = login_err(&login_body(wire));
            assert_eq!(
                err,
                SoapRequestError::BadAttributeValue {
                    attribute: "Password",
                    reason
                },
                "wire spelling {wire:?}"
            );
        }
    }

    /// The error never carries the value: quick-xml's own message quotes the
    /// unrecognized entity name, which here is part of a password.
    #[test]
    fn rejection_text_never_contains_the_value() {
        let err = login_err(&login_body("pw&SecretFragment;x"));
        let text = format!("{err} {err:?} {} {}", err.reason(), err.attribute());
        assert!(
            !text.contains("SecretFragment"),
            "error text must not quote the password: {text}"
        );
        assert_eq!(err.attribute(), "Password");
        assert_eq!(err.reason(), "unrecognized_entity");
    }

    /// The other Phase 1 attributes decode the same way.
    #[test]
    fn other_login_attributes_decode() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW&#95;BETA" AccountName="te&#115;t" Password="x" ProtocolDigest="&#x35;8AFA196AD3AC4F65CADD99BFF23B799" />"#;
        let req = parse_login_request(body).unwrap();
        assert_eq!(req.sku, "SGW_BETA");
        assert_eq!(req.account_name, "test");
        assert_eq!(req.protocol_digest, "58AFA196AD3AC4F65CADD99BFF23B799");
    }

    /// A malformed entity in a non-password attribute is rejected too, and
    /// names that attribute.
    #[test]
    fn malformed_entity_in_account_name_names_the_attribute() {
        let body = r#"<sgwLogin:SGWLoginRequest xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin" SKU="SGW_BETA" AccountName="a&nope;" Password="x" />"#;
        let err = login_err(body);
        assert_eq!(err.attribute(), "AccountName");
        assert_eq!(err.reason(), "unrecognized_entity");
    }

    /// A duplicated Password is ambiguous; the old `.flatten()` silently kept
    /// the first. Fail closed instead.
    #[test]
    fn duplicated_password_attribute_is_rejected() {
        let body = r#"<sgwLogin:SGWLoginRequest SKU="SGW_BETA" AccountName="test" Password="one" Password="two" />"#;
        assert_eq!(login_err(body), SoapRequestError::MalformedAttributes);
    }

    /// An attribute the server ignores is not decoded, so junk in it cannot
    /// fail an otherwise valid login.
    #[test]
    fn unknown_attribute_is_skipped_undecoded() {
        let body = r#"<sgwLogin:SGWLoginRequest SKU="SGW_BETA" AccountName="test" Password="x" Extra="&junk;" />"#;
        assert_eq!(parse_login_request(body).unwrap().password, "x");
    }

    /// Phase 2's shard name decodes, and a malformed one is rejected.
    #[test]
    fn server_selection_decodes_and_rejects_malformed() {
        let ok = r#"<sgwLogin:SGWSelectServerRequest ServerSelection="A&amp;B" />"#;
        assert_eq!(parse_server_selection(ok).unwrap(), "A&B");

        let bad = r#"<sgwLogin:SGWSelectServerRequest ServerSelection="A&B" />"#;
        assert_eq!(
            parse_server_selection(bad),
            Err(SoapRequestError::BadAttributeValue {
                attribute: "ServerSelection",
                reason: "unterminated_entity"
            })
        );
    }

    /// The legacy client's 40-char uppercase-hex SHA-1 contains no `&`, so
    /// decoding is the identity on it.
    #[test]
    fn legacy_sha1_hex_password_is_unchanged() {
        let hash = "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3";
        assert_eq!(
            parse_login_request(&login_body(hash)).unwrap().password,
            hash
        );
    }
}
