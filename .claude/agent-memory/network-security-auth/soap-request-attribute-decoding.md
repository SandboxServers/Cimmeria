---
name: soap-request-attribute-decoding
description: Phase 1/2 SOAP request attributes are XML-decoded once via quick-xml normalized_value in auth/soap_request.rs (#1289); error type never carries the value; LoginReq has no Debug on purpose
metadata:
  type: project
---

Since #1289 (2026-10-10) the SOAP request parsers live in
`crates/auth/src/auth/soap_request.rs`, not `handlers.rs`. Every recognised
attribute (SKU, AccountName, Password, ProtocolDigest, ServerSelection) is
decoded once with `Attribute::normalized_value(XmlVersion::Implicit1_0)`:
predefined entities and char refs decoded, raw tab/CR/LF become a space.
`unescape_value()` is deprecated in quick-xml 0.41 and itself normalizes.
Malformed entities and attribute-syntax errors (incl. duplicated attributes)
reject the request.

**Why:** a TLS plaintext password `a&b` arrives as `a&amp;b`; comparing the
wire spelling failed the login. quick-xml's own `EscapeError` Display quotes
the unrecognized entity name, which can be a password fragment, so
`SoapRequestError` carries only a static reason and the attribute name.

**How to apply:** never log a quick-xml error from attribute decoding
directly; map it to a reason code. `LoginReq` deliberately has no `Debug`
(it holds the password) — tests use a `login_err` helper instead of
`unwrap_err`. Guards: parser tests in `soap_request.rs`, the dev-mode TLS
malformed-entity test and the live-DB escaped-password TLS smoke in
`tls_smoke.rs` (sentinel account `0x7000_1B20`).

Follow-up on the same PR: `handle_user_auth` checks the account-name format right after
the SKU check, BEFORE recording `account_name` into the span or any audit
row (guard: `account_name_guard.rs`). Response XML builders live in
`soap_response.rs`; argon2id hash/verify/migrate in `password_hash.rs`.

Related: [[password-storage-argon2id]]
