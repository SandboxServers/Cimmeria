# Tests — `mercury`

> **Type**: reference  
> **Audience**: engineers  
> **Last updated**: 2026-07-25 *(links repaired; catalogue rows still the 2026-05-21 snapshot)*  
> **Total tests**: 93  
> **CI-gated**: yes  
> **Index**: [README](README.md) | **Playbook**: [TESTING.md](../../../TESTING.md)

Mercury reliable UDP protocol plus AES-256-CBC / HMAC-MD5 encryption. Includes packet framing, sequencing, and the byte-level codec that the BigWorld client must accept verbatim.

## All tests (93)

| Test | Kind | System / Feature | Added | What it tests | Notes |
|---|---|---|---|---|---|
| [round_trip_bundle](../../../crates/mercury/src/bundle.rs#L147) | unit | Bundle | 2026-03-03 | Asserts equality on `decoded.messages.len()` |  |
| [empty_bundle](../../../crates/mercury/src/bundle.rs#L164) | unit | Bundle | 2026-03-03 | Asserts on `buf.is_empty()` |  |
| [decode_empty_buf_returns_empty_bundle](../../../crates/mercury/src/bundle.rs#L175) | unit | Bundle | 2026-03-05 | Asserts equality on `decoded.messages.len()` |  |
| [finalize_no_count_prefix](../../../crates/mercury/src/bundle.rs#L182) | unit | Bundle | 2026-03-05 | Asserts equality on `encoded[0]` |  |
| [new_channel_starts_connecting](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L9) | unit | Channel | 2026-05-03 | Asserts equality on `ch.state` |  |
| [fresh_channel_not_timed_out](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L21) | unit | Channel | 2026-05-03 | Asserts on `!ch.is_timed_out()` |  |
| [mark_reliable_stores_packet](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L44) | unit | Channel | 2026-05-03 | Asserts equality on `ch.tx_window.len()` |  |
| [on_ack_removes_packet](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L55) | unit | Channel | 2026-05-03 | Asserts equality on `ch.tx_window.len()` |  |
| [check_retransmits_returns_expired](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L66) | unit | Channel | 2026-05-03 | Asserts equality on `retransmits.len()` |  |
| [check_retransmits_empty_when_fresh](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L88) | unit | Channel | 2026-05-03 | Asserts on `retransmits.is_empty()` |  |
| [send_packet_updates_only_last_sent](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L101) | unit | Channel | 2026-05-03 | Asserts on `ch.last_sent > baseline` |  |
| [receive_packet_updates_only_last_received](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L119) | unit | Channel | 2026-05-03 | Receive packet updates only last received |  |
| [process_acks_updates_only_last_received](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L143) | unit | Channel | 2026-05-03 | Asserts on `ch.last_received > baseline` |  |
| [keepalive_due_when_we_havent_sent_in_a_while](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L166) | unit | Channel | 2026-05-03 | Asserts on `ch.keepalive_due()` |  |
| [keepalive_not_due_right_after_sending](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L183) | unit | Channel | 2026-05-03 | Asserts on `!ch.keepalive_due()` |  |
| [is_timed_out_fires_on_silent_peer_even_if_we_keep_sending](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L191) | unit | Channel | 2026-05-03 | Asserts on `ch.is_timed_out()` |  |
| [is_timed_out_does_not_fire_when_peer_is_chatty](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L210) | unit | Channel | 2026-05-03 | Asserts on `!ch.is_timed_out()` |  |
| [touch_sent_only_moves_send_clock](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L219) | unit | Channel | 2026-05-03 | Asserts on `ch.last_sent > baseline` |  |
| [touch_received_only_moves_receive_clock](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L235) | unit | Channel | 2026-05-03 | Asserts on `ch.last_received > baseline` |  |
| [check_timeouts_bumps_last_sent_when_retransmitting](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L254) | unit | Channel | 2026-05-03 | Check timeouts bumps last sent when retransmitting |  |
| [check_timeouts_does_not_bump_last_sent_when_no_retransmits](../../../crates/mercury/src/channel/tests/channel_lifecycle.rs#L286) | unit | Channel | 2026-05-03 | Asserts on `retransmits.is_empty()` |  |
| [reassemble_parsed_passes_through_non_fragmented](../../../crates/mercury/src/channel/tests/reassembly.rs#L32) | unit | Channel | 2026-05-03 | Asserts equality on `body.as_ref()` |  |
| [reassemble_parsed_completes_3_fragment_bundle](../../../crates/mercury/src/channel/tests/reassembly.rs#L46) | unit | Channel | 2026-05-03 | Asserts on `ch.reassemble_parsed(&f0).unwrap().is_none()` |  |
| [reassemble_parsed_bumps_last_received](../../../crates/mercury/src/channel/tests/reassembly.rs#L62) | unit | Channel | 2026-05-03 | Reassemble parsed bumps last received |  |
| [reassemble_parsed_isolates_per_channel_state](../../../crates/mercury/src/channel/tests/reassembly.rs#L86) | unit | Channel | 2026-05-03 | Reassemble parsed isolates per channel state |  |
| [channel_keeps_orphan_partial_reassembly_indefinitely](../../../crates/mercury/src/channel/tests/reassembly.rs#L166) | unit | Channel | 2026-05-17 | Inverse contract of the deleted `cleanup_stale_fragments_drops_partial_bundles` — SGW client has no periodic reassembly sweep (spec §2.4.1 R13); orphan partials must persist until channel teardown |  |
| [sliding_window_rejects_overflow](../../../crates/mercury/src/channel/tests/reassembly.rs#L197) | unit | Channel | 2026-05-03 | Asserts equality on `ch.tx_window.len()` |  |
| [round_trip_plaintext](../../../crates/mercury/src/codec.rs#L121) | unit | Codec | 2026-03-03 | Round trip plaintext |  |
| [round_trip_encrypted](../../../crates/mercury/src/codec.rs#L142) | unit | Codec | 2026-03-03 | Round trip encrypted |  |
| [empty_body_round_trip](../../../crates/mercury/src/codec.rs#L164) | unit | Codec | 2026-03-03 | Asserts on `decoded.body.is_empty()` |  |
| [empty_buffer_returns_none](../../../crates/mercury/src/codec.rs#L178) | unit | Codec | 2026-03-03 | Asserts on `codec.decode(&mut buf).unwrap().is_none()` |  |
| [round_trip_encrypt_decrypt](../../../crates/mercury/src/encryption/tests.rs#L11) | unit | Encryption | 2026-03-03 | Asserts equality on `ciphertext.len()` |  |
| [round_trip_block_aligned](../../../crates/mercury/src/encryption/tests.rs#L24) | unit | Encryption | 2026-03-03 | Asserts equality on `plaintext.len()` |  |
| [round_trip_empty](../../../crates/mercury/src/encryption/tests.rs#L39) | unit | Encryption | 2026-03-03 | Asserts equality on `ciphertext.len()` |  |
| [tampered_ciphertext_fails_hmac](../../../crates/mercury/src/encryption/tests.rs#L52) | unit | Encryption | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::Encryption(_))` |  |
| [tampered_hmac_fails_verification](../../../crates/mercury/src/encryption/tests.rs#L65) | unit | Encryption | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::Encryption(_))` |  |
| [too_short_data_fails](../../../crates/mercury/src/encryption/tests.rs#L79) | unit | Encryption | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::Encryption(_))` |  |
| [deterministic_output](../../../crates/mercury/src/encryption/tests.rs#L86) | unit | Encryption | 2026-03-03 | Asserts equality on `ct1` |  |
| [pkcs7_padding_correctness](../../../crates/mercury/src/encryption/tests.rs#L100) | unit | Encryption | 2026-03-03 | Asserts equality on `padded.len()` |  |
| [debug_redacts_keys](../../../crates/mercury/src/encryption/tests.rs#L118) | unit | Encryption | 2026-03-03 | Asserts on `debug.contains("REDACTED")` |  |
| [decrypt_with_wrong_key_fails_hmac_verification](../../../crates/mercury/src/encryption/tests.rs#L138) | unit | Encryption | 2026-05-04 | Decrypting with a wrong key fails at HMAC verification (the HMAC key is the same as the AES key, so a wrong AES key always produces a wrong HMAC tag) |  |
| [decrypt_buffer_exactly_hmac_tag_len_rejects_empty_ciphertext](../../../crates/mercury/src/encryption/tests.rs#L163) | unit | Encryption | 2026-05-04 | Buffer exactly `HMAC_TAG_LEN` bytes (16) has an empty ciphertext portion |  |
| [decrypt_non_block_aligned_ciphertext_rejects_before_aes](../../../crates/mercury/src/encryption/tests.rs#L181) | unit | Encryption | 2026-05-04 | Buffer with a ciphertext portion that's not a multiple of the AES block size must reject with the block-size error — never fall through to the AES decrypt call (which would produce a garbage / partial-block error harder to interpret) |  |
| [pkcs7_unpad_rejects_pad_byte_above_block_size](../../../crates/mercury/src/encryption/tests.rs#L206) | unit | Encryption | 2026-05-04 | PKCS7 unpadding must reject any pad byte > AES_BLOCK_SIZE |  |
| [pkcs7_unpad_rejects_zero_pad_byte](../../../crates/mercury/src/encryption/tests.rs#L219) | unit | Encryption | 2026-05-04 | PKCS7 unpadding must reject a pad byte of 0 — the spec requires every padding byte to equal the pad length, and 0 indicates no padding was applied (which can't happen because the encoder always pads to a full block, including a full block of pad when the plaintext length is already block-aligned) |  |
| [round_trip_all_ids](../../../crates/mercury/src/messages.rs#L108) | unit | Messages | 2026-03-03 | Round trip all ids |  |
| [unknown_id_returns_none](../../../crates/mercury/src/messages.rs#L129) | unit | Messages | 2026-03-03 | Asserts on `MsgId::from_u8(0).is_none()` |  |
| [try_from_u8](../../../crates/mercury/src/messages.rs#L135) | unit | Messages | 2026-03-03 | Asserts equality on `id` |  |
| [display_formatting](../../../crates/mercury/src/messages.rs#L144) | unit | Messages | 2026-03-03 | Asserts equality on `MsgId::LoginRequest.to_string()` |  |
| [parse_baseapp_login_packet](../../../crates/mercury/src/packet/tests.rs#L15) | unit | Packet | 2026-03-03 | Parse baseapp login packet |  |
| [build_and_parse_reply_packet](../../../crates/mercury/src/packet/tests.rs#L46) | unit | Packet | 2026-03-03 | Asserts equality on `pkt.flags` |  |
| [parse_empty_body_with_seq](../../../crates/mercury/src/packet/tests.rs#L60) | unit | Packet | 2026-03-03 | Asserts equality on `pkt.seq_id` |  |
| [parse_flags_only_no_footers](../../../crates/mercury/src/packet/tests.rs#L68) | unit | Packet | 2026-03-03 | Asserts equality on `pkt.flags` |  |
| [parse_too_short_fails](../../../crates/mercury/src/packet/tests.rs#L78) | unit | Packet | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::BufferUnderflow { .. })` |  |
| [parse_seq_truncated_fails](../../../crates/mercury/src/packet/tests.rs#L84) | unit | Packet | 2026-03-03 | Asserts on `parse_incoming(&raw).is_err()` |  |
| [packet_flags_operations](../../../crates/mercury/src/packet/tests.rs#L92) | unit | Packet | 2026-03-03 | Asserts on `f.is_reliable()` |  |
| [round_trip_reliable_with_seq_and_acks](../../../crates/mercury/src/packet/tests.rs#L115) | unit | Packet | 2026-05-04 | All-flags-on packet: FLAG_RELIABLE \| FLAG_HAS_ACKS \| FLAG_HAS_SEQUENCE with a non-empty acks vector |  |
| [round_trip_fragmented_with_reliable_and_acks](../../../crates/mercury/src/packet/tests.rs#L157) | unit | Packet | 2026-05-04 | Fragmented + reliable + acks together: a real bundle for a large reliable-channel message that piggybacks acks |  |
| [parse_empty_body_with_acks_only](../../../crates/mercury/src/packet/tests.rs#L201) | unit | Packet | 2026-05-04 | Empty body with acks-only footer |  |
| [round_trip_with_piggyback_in_full_matrix](../../../crates/mercury/src/packet/tests.rs#L225) | unit | Packet | 2026-05-04 | Full matrix: FLAG_PIGGYBACK \| FLAG_HAS_ACKS \| FLAG_FRAGMENTED \| FLAG_RELIABLE \| FLAG_HAS_SEQUENCE all set together |  |
| [parse_incoming_never_panics_on_arbitrary_bytes](../../../crates/mercury/src/packet/parse_proptest.rs#L39) | proptest | Packet / Parse Proptest | 2026-05-04 | Arbitrary byte input must never crash the parser | smell: no_assert_or_question_mark |
| [parse_incoming_handles_truncated_footer_input_without_panic](../../../crates/mercury/src/packet/parse_proptest.rs#L59) | proptest | Packet / Parse Proptest | 2026-05-04 | Specifically exercise the FLAG_HAS_REQUESTS / FLAG_HAS_ACKS / FLAG_HAS_SEQUENCE / FLAG_FRAGMENTED bits in the leading flags byte | smell: no_assert_or_question_mark |
| [build_outgoing_round_trips_through_parse_incoming](../../../crates/mercury/src/packet/proptest_round_trip.rs#L73) | proptest | Packet / Proptest Round Trip | 2026-05-04 | Round-trip property: every valid non-fragmented packet that `build_outgoing` produces is parsed by `parse_incoming` back to the same flags / body / seq_id / acks / first_req_offset | smell: no_assert_or_question_mark |
| [fragmented_packet_round_trips_through_parse_incoming](../../../crates/mercury/src/packet/proptest_round_trip.rs#L120) | proptest | Packet / Proptest Round Trip | 2026-05-04 | Companion property for the fragmented builder | smell: no_assert_or_question_mark |
| [replay_packet_stream_decodes_every_shape_without_drift](../../../crates/mercury/src/packet/replay_smoke.rs#L143) | smoke | Packet / Replay Smoke | 2026-05-04 | Replay the full synthesized stream through `parse_incoming` and assert every frame round-trips its body, flags, and footers |  |
| [replay_stream_twice_in_a_row_produces_identical_decodes](../../../crates/mercury/src/packet/replay_smoke.rs#L196) | smoke | Packet / Replay Smoke | 2026-05-04 | Concatenated-stream replay: feed every frame through `parse_incoming` in sequence (each as its own datagram) |  |
| [fragmented_packet_round_trips_frag_footers](../../../crates/mercury/src/packet/replay_smoke.rs#L262) | smoke | Packet / Replay Smoke | 2026-05-04 | Fragmented packets travel through `build_outgoing_fragmented` rather than `build_outgoing` and carry an extra `frag_begin` / `frag_end` footer pair (innermost) |  |
| [parser_decodes_hand_coded_byte_fixtures_per_wire_spec](../../../crates/mercury/src/packet/replay_smoke.rs#L308) | smoke | Packet / Replay Smoke | 2026-05-04 | Independent oracle for the parser |  |
| [round_trip_unified_frame](../../../crates/mercury/src/unified.rs#L159) | unit | Unified | 2026-03-03 | Asserts equality on `decoded.message_id` |  |
| [partial_frame_returns_none](../../../crates/mercury/src/unified.rs#L275) | unit | Unified | 2026-03-03 | Asserts on `codec.decode(&mut buf).unwrap().is_none()` |  |
| [empty_payload](../../../crates/mercury/src/unified.rs#L288) | unit | Unified | 2026-03-03 | Asserts equality on `decoded.message_id` |  |
| [zero_length_frame_errors](../../../crates/mercury/src/unified.rs#L301) | unit | Unified | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::Protocol(_))` |  |
| [oversized_frame_errors](../../../crates/mercury/src/unified.rs#L311) | unit | Unified | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::Protocol(_))` |  |
| [single_fragment_completes_immediately](../../../crates/mercury/src/unpacker/tests.rs#L4) | unit | Unpacker | 2026-03-03 | Asserts equality on `result.unwrap().as_ref()` |  |
| [multi_fragment_assembly](../../../crates/mercury/src/unpacker/tests.rs#L14) | unit | Unpacker | 2026-03-03 | Asserts on `r.is_none()` |  |
| [invalid_frag_index](../../../crates/mercury/src/unpacker/tests.rs#L38) | unit | Unpacker | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::FragmentReassembly(_))` |  |
| [zero_total_frags](../../../crates/mercury/src/unpacker/tests.rs#L47) | unit | Unpacker | 2026-03-03 | Asserts on `matches!(err, CimmeriaError::FragmentReassembly(_))` |  |
| [process_parsed_passes_through_non_fragmented](../../../crates/mercury/src/unpacker/tests.rs#L72) | unit | Unpacker | 2026-05-02 | Asserts equality on `body.as_ref()` |  |
| [process_parsed_reassembles_in_order_3_fragment_bundle](../../../crates/mercury/src/unpacker/tests.rs#L91) | unit | Unpacker | 2026-05-02 | Asserts on `asm.process_parsed(&f0).unwrap().is_none()` |  |
| [process_parsed_reassembles_out_of_order](../../../crates/mercury/src/unpacker/tests.rs#L110) | unit | Unpacker | 2026-05-02 | Asserts on `asm.process_parsed(&f2).unwrap().is_none()` |  |
| [process_parsed_handles_duplicate_fragments](../../../crates/mercury/src/unpacker/tests.rs#L129) | unit | Unpacker | 2026-05-02 | Asserts on `asm.process_parsed(&f0).unwrap().is_none()` |  |
| [process_parsed_rejects_fragment_count_above_max](../../../crates/mercury/src/unpacker/tests.rs#L148) | unit | Unpacker | 2026-05-02 | Asserts on `matches!(err, CimmeriaError::FragmentReassembly(_))` |  |
| [process_parsed_rejects_seq_outside_range](../../../crates/mercury/src/unpacker/tests.rs#L166) | unit | Unpacker | 2026-05-02 | Asserts on `matches!(err, CimmeriaError::FragmentReassembly(_))` |  |
| [process_parsed_handles_u32_max_range_without_overflow](../../../crates/mercury/src/unpacker/tests.rs#L179) | unit | Unpacker | 2026-05-02 | Pathological begin=0, end=u32::MAX would overflow `(end - begin + 1)` in u32 — modular cap rejects it |  |
| [process_parsed_rejects_bogus_range_via_max_fragments_cap](../../../crates/mercury/src/unpacker/tests.rs#L207) | unit | Unpacker | 2026-05-21 | Non-wrap garbage range (e.g. begin=10, end=4) implies a ~268M-fragment wrap under modular arithmetic — rejected via the MAX_FRAGMENTS cap that `add_fragment` already enforces |  |
| [process_parsed_accepts_28_bit_wrapped_range](../../../crates/mercury/src/unpacker/tests.rs#L230) | unit | Unpacker | 2026-05-21 | Regression guard: a wire-arriving bundle whose range straddles the 28-bit sequence-space wrap MUST be accepted (pre-fix the `frag_end < frag_begin` gate dropped every wrapped bundle before reaching the modular helpers) |  |
| [add_fragment_rejects_conflicting_total_fragments](../../../crates/mercury/src/unpacker/tests.rs#L260) | unit | Unpacker | 2026-05-04 | Two fragments arriving for the same `first_seq` must agree on `total_frags` |  |
| [arrival_of_overlapping_bundle_evicts_in_progress_reassembly](../../../crates/mercury/src/unpacker/tests.rs#L285) | unit | Unpacker | 2026-05-17 | Arrival-triggered eviction (spec §2.4.1 R13 / §2.10 S6): a new fragmented bundle whose sequence range overlaps an in-progress reassembly with an older `first_seq` evicts the in-progress one |  |
| [arrival_of_non_overlapping_bundle_leaves_in_progress_alone](../../../crates/mercury/src/unpacker/tests.rs#L324) | unit | Unpacker | 2026-05-17 | Non-overlapping bundles coexist — eviction is "older overlapping abandoned" signal only, not "any new fragment resets everything" |  |
| [orphan_partial_reassembly_persists_indefinitely](../../../crates/mercury/src/unpacker/tests.rs#L341) | unit | Unpacker | 2026-05-17 | Inverse of the deleted `process_parsed_times_out_incomplete_set` — an in-progress reassembly that never sees its remaining fragments must persist (no periodic sweep) |  |
| [late_fragment_from_evicted_older_bundle_does_not_displace_newer](../../../crates/mercury/src/unpacker/tests.rs#L364) | unit | Unpacker | 2026-05-17 | Eviction is asymmetric: a late straggler from an already-evicted older bundle must NOT displace the newer bundle that took over |  |
| [incoming_overlapping_multiple_with_any_newer_existing_drops_stale](../../../crates/mercury/src/unpacker/tests.rs#L418) | unit | Unpacker | 2026-05-17 | Incoming bundle whose range straddles multiple existing bundles must be dropped as stale if ANY existing is strictly newer |  |
| [overlap_detection_handles_28_bit_sequence_wraparound](../../../crates/mercury/src/unpacker/tests.rs#L453) | unit | Unpacker | 2026-05-17 | Wraparound case for the modular overlap test — ranges that straddle the 28-bit sequence-space boundary must still detect overlap correctly |  |
