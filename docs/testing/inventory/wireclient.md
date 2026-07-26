# `wireclient` test inventory

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25  
> **Total tests**: 30 across 5 files  
> **Index**: [README](README.md) · **Playbook**: [TESTING.md](../../../TESTING.md) §11 · **ADR**: [wireclient](../../architecture/wireclient.md)

`cimmeria-wireclient` is the Phase 1 foundation of the planned wire-level
replay tier. It ships **byte builders, parsers, and a trace loader** — not a
replay engine. The crate contains no `UdpSocket`, `send_to`, or `recv_from`;
`Client::connect()` is unimplemented and `Client::from_handshake` is a
test-only constructor. Read [TESTING.md](../../../TESTING.md) §11 before
assuming any of these tests exercise a live server.

Counted with `grep -rnE "^[[:space:]]*#\[(tokio::)?test(\(.*\))?\]" crates/wireclient --include=*.rs`.

## `src/auth.rs` — SOAP auth helpers (6 unit tests)

| Test | Kind | What it tests |
|---|---|---|
| [`extract_attr_finds_session_key`](../../../crates/wireclient/src/auth.rs#L312) | unit | Pulls the session-key attribute out of a SOAP response body. |
| [`extract_attr_missing_returns_none`](../../../crates/wireclient/src/auth.rs#L319) | unit | Absent attribute yields `None` rather than a panic or empty string. |
| [`parse_sid_extracts_value_and_ignores_other_attrs`](../../../crates/wireclient/src/auth.rs#L325) | unit | `sid` cookie parse is not confused by neighbouring attributes. |
| [`parse_sid_returns_none_when_no_sid`](../../../crates/wireclient/src/auth.rs#L334) | unit | Missing `sid` yields `None`. |
| [`xml_attr_escape_handles_metacharacters`](../../../crates/wireclient/src/auth.rs#L340) | unit | XML attribute escaping covers the metacharacter set. |
| [`credentials_test_account_uses_sha1_of_test`](../../../crates/wireclient/src/auth.rs#L348) | unit | Test-account credentials hash to the expected SHA-1. |

## `src/handshake.rs` — Mercury phase-3 byte layer (10 wire-format tests)

| Test | Kind | What it tests |
|---|---|---|
| [`build_baseapp_login_byte_shape`](../../../crates/wireclient/src/handshake.rs#L318) | wire-format | Byte-exact layout of the unencrypted `baseAppLogin` datagram. |
| [`build_baseapp_login_matches_recorded_capture`](../../../crates/wireclient/src/handshake.rs#L356) | wire-format | Builder output equals bytes from a real recorded capture. |
| [`build_baseapp_login_rejects_wrong_ticket_length`](../../../crates/wireclient/src/handshake.rs#L382) | wire-format | Ticket-length validation fails loudly instead of emitting a bad frame. |
| [`build_baseapp_login_round_trips_through_parse_incoming`](../../../crates/wireclient/src/handshake.rs#L388) | wire-format | Built frame is accepted by `cimmeria-mercury`'s `parse_incoming`. |
| [`parse_baseapp_reply_round_trips_against_known_plaintext`](../../../crates/wireclient/src/handshake.rs#L401) | wire-format | Reply parser round-trips a known plaintext reply. |
| [`parse_baseapp_reply_detects_request_id_mismatch`](../../../crates/wireclient/src/handshake.rs#L429) | wire-format | Echoed `request_id` mismatch is caught (desync detection). |
| [`parse_baseapp_reply_rejects_trailing_bytes`](../../../crates/wireclient/src/handshake.rs#L454) | wire-format | Trailing bytes after the reply body are an error, not ignored. |
| [`parse_time_sync_decodes_all_three_subfields`](../../../crates/wireclient/src/handshake.rs#L474) | wire-format | `time_sync` decodes all three subfields. |
| [`parse_time_sync_rejects_trailing_bytes`](../../../crates/wireclient/src/handshake.rs#L501) | wire-format | Trailing bytes after `time_sync` are rejected. |
| [`parse_time_sync_rejects_wrong_flags`](../../../crates/wireclient/src/handshake.rs#L525) | wire-format | Wrong flag byte is rejected. |

## `src/session_trace.rs` — JSONL trace format + diff policy (10 unit tests)

| Test | Kind | What it tests |
|---|---|---|
| [`round_trip_minimal_trace`](../../../crates/wireclient/src/session_trace.rs#L336) | unit | Header + event serialise and parse back unchanged. |
| [`default_policy_exact_match_is_exact`](../../../crates/wireclient/src/session_trace.rs#L373) | unit | Identical messages classify as `Diff::Exact`. |
| [`default_policy_static_byte_drift_is_regression`](../../../crates/wireclient/src/session_trace.rs#L384) | unit | Any body drift on a static msg_id (`0x00`–`0x7F`) is a regression. |
| [`default_policy_entity_method_same_length_is_drift_not_regression`](../../../crates/wireclient/src/session_trace.rs#L403) | unit | Pins the length-only comparison for `0x80`–`0xFE` — same length, different bytes is `Diff::Drift`. |
| [`default_policy_entity_method_length_change_is_regression`](../../../crates/wireclient/src/session_trace.rs#L420) | unit | Length change on an entity-method body is a regression. |
| [`default_policy_msg_0xff_byte_drift_is_regression`](../../../crates/wireclient/src/session_trace.rs#L443) | unit | `0xFF` (`BASEMSG_REPLY_MESSAGE`) stays outside the drift band. |
| [`default_policy_case_insensitive_hex_match`](../../../crates/wireclient/src/session_trace.rs#L464) | unit | Upper/lowercase hex bodies compare equal. |
| [`trace_load_rejects_packet_count_mismatch`](../../../crates/wireclient/src/session_trace.rs#L482) | unit | Header `packet_count` disagreeing with the event count fails the load. |
| [`trace_load_rejects_unknown_schema_version`](../../../crates/wireclient/src/session_trace.rs#L515) | unit | Unknown `schema_version` fails the load. |
| [`trace_load_rejects_unknown_event_field`](../../../crates/wireclient/src/session_trace.rs#L537) | unit | `deny_unknown_fields` rejects an unrecognised event key. |

## `tests/auth_smoke.rs` — in-process `AuthService` round trip (3 integration tests)

| Test | Kind | What it tests |
|---|---|---|
| [`wireclient_drives_phase1_phase2_against_inprocess_auth`](../../../crates/wireclient/tests/auth_smoke.rs#L54) | integration | Full SOAP Phase 1 + Phase 2 against an in-process `AuthService`. |
| [`wireclient_phase1_returns_sid_cookie`](../../../crates/wireclient/tests/auth_smoke.rs#L112) | integration | Phase 1 hands back the `sid` cookie Phase 2 needs. |
| [`wireclient_phase2_replay_with_same_sid_errors`](../../../crates/wireclient/tests/auth_smoke.rs#L133) | integration | Replaying Phase 2 with an already-used `sid` is rejected. |

## `tests/trace_load.rs` — fixture load (1 integration test)

| Test | Kind | What it tests |
|---|---|---|
| [`loads_castle_cellblock_head_fixture`](../../../crates/wireclient/tests/trace_load.rs#L20) | integration | Loads `tests/fixtures/castle_cellblock_head.jsonl` (1 header line + 5 events) — the only trace corpus checked into the repo. |

## Gaps

- **No test drives a UDP socket**, because the crate has none. Phase 1.5 in
  the [ADR](../../architecture/wireclient.md) adds the socket loop.
- **No replay test.** `Trace::c2s()` / `Trace::s2c()` have no consumer.
- **No `castle-cellblock-full-run` corpus.** The 125,770-event capture named
  in the ADR is not committed; only the 5-event head fixture is.
