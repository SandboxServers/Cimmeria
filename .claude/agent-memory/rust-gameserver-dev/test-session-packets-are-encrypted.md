---
name: test-session-packets-are-encrypted
description: TestTransport packets to a test_default_connected_client_state session are Mercury-encrypted (all-zero key), and feedback lines need player_entity_id set; grep text only after decrypting
metadata:
  type: reference
---

A base test that wants to see a chat/feedback line (`send_feedback_line`, `onPlayerCommunication`) in `TestTransport::filter_to(addr)` hits two traps:

- `test_default_connected_client_state()` (`crates/base-session/src/test_fixtures.rs`) has `player_entity_id: None`, so `send_feedback_line` drops the line (`reason=not_in_world` WARN) and the send count does not change. Set `state.player_entity_id = Some(entity_id)`.
- The state carries `MercuryEncryption::from_session_key([0u8; 32])`, so every recorded packet is ciphertext. Searching it for the UTF-16LE text finds nothing. Decrypt first: `MercuryEncryption::from_session_key([0u8; 32]).decrypt(p)`, then search the plaintext (text is UTF-16LE). `decode_feedback` in `base-session/src/base/feedback.rs` tests does the full parse.

Worked example: `Client::saw_text` in `crates/base-methods/.../inventory/move_/vault_move_tests.rs` (BV-03).

Related: [[vacuous-guard-and-sentinel-collision-review]].
