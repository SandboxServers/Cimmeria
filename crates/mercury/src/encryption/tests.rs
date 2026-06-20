use super::*;

fn test_keys() -> MercuryEncryption {
    let aes_key = [0x42u8; 32];
    let iv = [0x13u8; 16];
    let hmac_key = [0x37u8; 32]; // C++ uses 32-byte key for HMAC
    MercuryEncryption::new(aes_key, iv, hmac_key)
}

#[test]
fn round_trip_encrypt_decrypt() {
    let enc = test_keys();
    let plaintext = b"Hello, Stargate Worlds!";

    let ciphertext = enc.encrypt(plaintext).unwrap();
    // Ciphertext should be: padded plaintext (32 bytes) + 16 byte HMAC = 48 bytes.
    assert_eq!(ciphertext.len(), 32 + HMAC_TAG_LEN);

    let decrypted = enc.decrypt(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_block_aligned() {
    let enc = test_keys();
    // Exactly 16 bytes — PKCS7 adds a full block of padding.
    let plaintext = b"exactly16bytes!!";
    assert_eq!(plaintext.len(), 16);

    let ciphertext = enc.encrypt(plaintext).unwrap();
    // 16 bytes + 16 padding + 16 HMAC = 48 bytes.
    assert_eq!(ciphertext.len(), 32 + HMAC_TAG_LEN);

    let decrypted = enc.decrypt(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn round_trip_empty() {
    let enc = test_keys();
    let plaintext = b"";

    let ciphertext = enc.encrypt(plaintext).unwrap();
    // Empty -> 16 bytes padding + 16 HMAC = 32 bytes.
    assert_eq!(ciphertext.len(), 16 + HMAC_TAG_LEN);

    let decrypted = enc.decrypt(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext.as_slice());
}

#[test]
fn tampered_ciphertext_fails_hmac() {
    let enc = test_keys();
    let plaintext = b"tamper test";

    let mut ciphertext = enc.encrypt(plaintext).unwrap();
    // Flip a bit in the ciphertext (before the HMAC tag).
    ciphertext[0] ^= 0xFF;

    let err = enc.decrypt(&ciphertext).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

#[test]
fn tampered_hmac_fails_verification() {
    let enc = test_keys();
    let plaintext = b"hmac test";

    let mut ciphertext = enc.encrypt(plaintext).unwrap();
    // Flip a bit in the HMAC tag.
    let tag_start = ciphertext.len() - HMAC_TAG_LEN;
    ciphertext[tag_start] ^= 0xFF;

    let err = enc.decrypt(&ciphertext).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

#[test]
fn too_short_data_fails() {
    let enc = test_keys();
    let err = enc.decrypt(&[0u8; 10]).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

#[test]
fn deterministic_output() {
    let enc = test_keys();
    let plaintext = b"determinism check";

    let ct1 = enc.encrypt(plaintext).unwrap();
    let ct2 = enc.encrypt(plaintext).unwrap();

    // Same key + same IV + same plaintext = same ciphertext (CBC is
    // deterministic given identical IV). This is critical for byte-identical
    // output matching the C++ implementation.
    assert_eq!(ct1, ct2);
}

#[test]
fn pkcs7_padding_correctness() {
    // 1 byte input -> 15 bytes padding.
    let padded = pkcs7_pad(&[0x41]);
    assert_eq!(padded.len(), 16);
    assert_eq!(padded[1..], [0x0F; 15]);

    // 15 byte input -> 1 byte padding.
    let padded = pkcs7_pad(&[0x41; 15]);
    assert_eq!(padded.len(), 16);
    assert_eq!(padded[15], 0x01);

    // 16 byte input -> 16 bytes padding (full extra block).
    let padded = pkcs7_pad(&[0x41; 16]);
    assert_eq!(padded.len(), 32);
    assert_eq!(padded[16..], [0x10; 16]);
}

#[test]
fn debug_redacts_keys() {
    let enc = test_keys();
    let debug = format!("{:?}", enc);
    assert!(debug.contains("REDACTED"));
    assert!(!debug.contains("42")); // Should not leak key bytes.
}

/// Decrypting with a wrong key fails at HMAC verification (the
/// HMAC key is the same as the AES key, so a wrong AES key always
/// produces a wrong HMAC tag). Surface this as an Encryption error
/// rather than letting it slip through to a garbage plaintext —
/// the encrypt-then-MAC ordering guarantees we reject before the
/// AES decrypt step runs.
///
/// Pin via `from_session_key` on both sides so the only thing that
/// differs is the 32-byte key (zero IV, HMAC key == AES key on both).
/// `test_keys()`-derived ciphertext would also have a different IV
/// and HMAC key, which would mask whether HMAC truly catches the
/// AES-key mismatch alone.
#[test]
fn decrypt_with_wrong_key_fails_hmac_verification() {
    let plaintext = b"secret payload";
    let ct = MercuryEncryption::from_session_key([0x55u8; 32])
        .encrypt(plaintext)
        .unwrap();

    // Same `from_session_key` shape so IV and HMAC-key derivation
    // are identical to the encrypt side — only the AES/HMAC key
    // bytes differ.
    let wrong = MercuryEncryption::from_session_key([0xAAu8; 32]);
    let err = wrong.decrypt(&ct).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    assert!(
        msg.contains("HMAC-MD5 verification failed"),
        "wrong-key decrypt must be caught by HMAC verify branch (encrypt-then-MAC), got: {msg}"
    );
}

/// Buffer exactly `HMAC_TAG_LEN` bytes (16) has an empty ciphertext
/// portion. The "ciphertext too short" branch must reject before
/// the AES init runs — otherwise a mock plaintext could be coaxed
/// out of a degenerate empty-block decrypt.
#[test]
fn decrypt_buffer_exactly_hmac_tag_len_rejects_empty_ciphertext() {
    let enc = test_keys();
    // 16 bytes = exactly the HMAC tag; ciphertext portion is empty.
    let err = enc.decrypt(&[0u8; 16]).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    assert!(
        msg.contains("ciphertext portion is empty"),
        "empty-ciphertext input must hit the dedicated guard, not slip into HMAC verify; got: {msg}"
    );
}

/// Buffer with a ciphertext portion that's not a multiple of the
/// AES block size must reject with the block-size error — never
/// fall through to the AES decrypt call (which would produce a
/// garbage / partial-block error harder to interpret).
#[test]
fn decrypt_non_block_aligned_ciphertext_rejects_before_aes() {
    let enc = test_keys();
    // 17 ciphertext bytes (not divisible by 16) + 16 HMAC = 33 bytes.
    let buf = vec![0u8; 17 + HMAC_TAG_LEN];
    let err = enc.decrypt(&buf).unwrap_err();
    match err {
        CimmeriaError::Encryption(msg) => assert!(
            msg.contains("multiple of AES block size"),
            "expected block-size error, got: {msg}"
        ),
        other => panic!("expected Encryption error, got {other:?}"),
    }
}

/// PKCS7 unpadding must reject any pad byte > AES_BLOCK_SIZE.
/// Use a buffer LARGER than the would-be pad_len so the
/// `pad_len > data.len()` guard does NOT also fire — that way
/// the test isolates the > AES_BLOCK_SIZE branch specifically.
/// With pad_len = AES_BLOCK_SIZE + 1 = 17 in an 18-byte buffer:
/// `pad_len > AES_BLOCK_SIZE` (17 > 16) rejects (the branch
/// under test); `pad_len > data.len()` (17 > 18) is false, so
/// that guard would NOT catch it. If a regression drops the
/// `> AES_BLOCK_SIZE` check, the function would happily strip 17
/// bytes from the 18-byte buffer and produce a 1-byte plaintext.
#[test]
fn pkcs7_unpad_rejects_pad_byte_above_block_size() {
    let mut buf = vec![0u8; 18];
    buf[17] = (AES_BLOCK_SIZE + 1) as u8; // 17, just above the cap
    let err = pkcs7_unpad(&buf).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

/// PKCS7 unpadding must reject a pad byte of 0 — the spec requires
/// every padding byte to equal the pad length, and 0 indicates
/// no padding was applied (which can't happen because the encoder
/// always pads to a full block, including a full block of pad when
/// the plaintext length is already block-aligned).
#[test]
fn pkcs7_unpad_rejects_zero_pad_byte() {
    let buf = vec![0u8; 16]; // last byte is 0x00
    let err = pkcs7_unpad(&buf).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

// ===== v2 =================================================================

/// A fixed 32-byte session key for v2 tests.
const V2_SESSION_KEY: [u8; 32] = [0x5Au8; 32];

/// v2 round-trip across the size classes the v1 round-trip tests cover:
/// short (sub-block), block-aligned, multi-block, and the empty edge.
/// Each must `decrypt(encrypt(x)) == x`.
#[test]
fn v2_round_trip_all_sizes() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    for plaintext in [
        b"".as_slice(),                                                 // empty edge
        b"short".as_slice(),                                            // sub-block
        b"exactly16bytes!!".as_slice(),                                 // exactly one block
        b"this payload spans several AES blocks of content".as_slice(), // multi-block
    ] {
        let ct = enc.encrypt(plaintext).expect("v2 encrypt");
        // Frame must start with the version byte and carry a 16-byte IV.
        assert_eq!(ct[0], V2_VERSION_BYTE, "v2 frame must start with 0x02");
        assert!(
            ct.len() >= 1 + V2_IV_LEN + AES_BLOCK_SIZE + HMAC_TAG_LEN,
            "v2 frame shorter than the minimum for {:?}",
            plaintext
        );
        let pt = enc.decrypt(&ct).expect("v2 decrypt");
        assert_eq!(pt, plaintext, "v2 round-trip must recover plaintext");
    }
}

/// Two encryptions of the same plaintext under the same key must differ
/// (random per-packet IV), but both must still decrypt back to the same
/// plaintext. This is the explicit contrast with v1's deterministic output.
#[test]
fn v2_random_iv_makes_output_nondeterministic() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    let plaintext = b"non-determinism via random IV";

    let ct1 = enc.encrypt(plaintext).unwrap();
    let ct2 = enc.encrypt(plaintext).unwrap();

    assert_ne!(
        ct1, ct2,
        "v2 must use a fresh random IV per packet, so two encryptions differ"
    );
    // The IV is the differing part; both still decrypt correctly.
    assert_eq!(enc.decrypt(&ct1).unwrap(), plaintext);
    assert_eq!(enc.decrypt(&ct2).unwrap(), plaintext);
}

/// Known-answer test pinning the exact v2 wire bytes for a fixed
/// (session_key, plaintext, IV). Uses the private deterministic-IV seam so
/// the format cannot silently drift (IV placement, HKDF info strings,
/// HMAC-over-IV||ciphertext, truncation length, byte order). The expected
/// vector is self-generated by this implementation and frozen here; if any
/// v2 parameter changes, this assertion fails and forces a conscious
/// re-pin.
#[test]
fn v2_kat_fixed_iv_pins_wire_format() {
    let enc = MercuryEncryption::from_session_key_v2([0x01u8; 32]);
    let iv = [0x02u8; V2_IV_LEN];
    let plaintext = b"mercury v2 kat!!"; // 16 bytes = one block

    let got = enc.encrypt_v2_with_iv(plaintext, iv).expect("v2 encrypt");

    // Structural invariants: [0x02][16 IV][32 ciphertext][16 tag] = 65 bytes.
    // (16-byte plaintext + full PKCS7 block = 32 ciphertext bytes.)
    assert_eq!(got.len(), 1 + V2_IV_LEN + 32 + HMAC_TAG_LEN);
    assert_eq!(got[0], V2_VERSION_BYTE);
    assert_eq!(&got[1..1 + V2_IV_LEN], &iv);

    // Frozen full-frame vector. Regenerate intentionally only on a
    // deliberate v2 format change (print `got` and re-pin). Layout:
    // [0x02][16-byte IV][32-byte ciphertext][16-byte truncated HMAC-SHA256].
    assert_eq!(
        got.as_slice(),
        KAT_V2_EXPECTED,
        "v2 wire format drifted from the pinned KAT vector"
    );

    // And it must round-trip back to the plaintext.
    assert_eq!(enc.decrypt(&got).unwrap(), plaintext);
}

/// Pinned v2 known-answer frame for
/// `from_session_key_v2([0x01; 32])`, IV = `[0x02; 16]`,
/// plaintext = `b"mercury v2 kat!!"`. Generated by this implementation and
/// frozen; see `v2_kat_fixed_iv_pins_wire_format`. Layout:
/// `[0x02][16-byte IV][32-byte ciphertext][16-byte truncated HMAC-SHA256]`.
#[rustfmt::skip]
const KAT_V2_EXPECTED: &[u8] = &[
    0x02,                                                             // version
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,                   // IV[0..8]
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02,                   // IV[8..16]
    0xBC, 0x8A, 0xE8, 0x83, 0x1B, 0x07, 0x7B, 0x53,                   // ciphertext[0..8]
    0xA3, 0xF0, 0xFC, 0x27, 0x9F, 0x5B, 0xA5, 0xD6,                   // ciphertext[8..16]
    0xD7, 0xC1, 0xCA, 0x2C, 0x74, 0x68, 0x04, 0xBF,                   // ciphertext[16..24]
    0x6B, 0x3B, 0xC2, 0xE4, 0x19, 0x3D, 0x64, 0x43,                   // ciphertext[24..32]
    0xAA, 0xA2, 0x14, 0x0F, 0xCB, 0xC3, 0x73, 0x84,                   // tag[0..8]
    0x9D, 0x02, 0x83, 0xDF, 0x03, 0xFB, 0x7C, 0x88,                   // tag[8..16]
];

/// Tampering with the IV must be caught by the HMAC (the tag covers
/// `IV || ciphertext`, so an IV swap invalidates it) — verify-then-decrypt.
#[test]
fn v2_tampered_iv_rejected() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    let mut ct = enc.encrypt(b"iv tamper test").unwrap();
    ct[1] ^= 0xFF; // flip a bit in the IV (offset 1, just past version byte)
    let err = enc.decrypt(&ct).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    assert!(
        msg.contains("HMAC-SHA256 verification failed"),
        "IV tamper must fail HMAC (tag covers IV||ciphertext); got: {msg}"
    );
}

/// Tampering with the HMAC tag must be rejected.
#[test]
fn v2_tampered_hmac_rejected() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    let mut ct = enc.encrypt(b"tag tamper test").unwrap();
    let last = ct.len() - 1;
    ct[last] ^= 0xFF; // flip a bit in the truncated tag
    let err = enc.decrypt(&ct).unwrap_err();
    assert!(matches!(err, CimmeriaError::Encryption(_)));
}

/// Tampering with the ciphertext must be rejected at HMAC verify (before
/// the AES decrypt runs).
#[test]
fn v2_tampered_ciphertext_rejected() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    let mut ct = enc.encrypt(b"ciphertext tamper").unwrap();
    // Offset 1 + IV = first ciphertext byte.
    ct[1 + V2_IV_LEN] ^= 0xFF;
    let err = enc.decrypt(&ct).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    assert!(
        msg.contains("HMAC-SHA256 verification failed"),
        "ciphertext tamper must be caught by HMAC verify; got: {msg}"
    );
}

/// Downgrade defense: a v2 decryptor handed a v1-style buffer (first byte
/// is whatever the v1 ciphertext happens to be, not 0x02) must reject. We
/// build a real v1 frame and feed it to a v2 decryptor — it must not be
/// accepted, and must not be silently treated as v1.
#[test]
fn v2_decryptor_rejects_v1_buffer_downgrade_defense() {
    // A genuine v1 frame from the same session key.
    let v1 = MercuryEncryption::from_session_key(V2_SESSION_KEY);
    let v1_frame = v1.encrypt(b"downgrade me").unwrap();

    let v2 = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    // Only reject if the v1 frame's first byte isn't coincidentally 0x02;
    // if it were, the HMAC check would still reject. Assert the error
    // either way, and assert the dedicated downgrade message when the
    // version byte differs (the common case).
    let err = v2.decrypt(&v1_frame).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    if v1_frame[0] != V2_VERSION_BYTE {
        assert!(
            msg.contains("downgrade defense"),
            "v1 buffer with non-0x02 first byte must hit the downgrade guard; got: {msg}"
        );
    }
}

/// An explicit non-0x02 version byte (e.g. a future 0x03) must be rejected
/// by the downgrade guard before any crypto runs.
#[test]
fn v2_decryptor_rejects_foreign_version_byte() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    let mut ct = enc.encrypt(b"version gate").unwrap();
    ct[0] = 0x03; // pretend to be a different version
    let err = enc.decrypt(&ct).unwrap_err();
    let CimmeriaError::Encryption(msg) = err else {
        panic!("expected Encryption error");
    };
    assert!(
        msg.contains("downgrade defense"),
        "foreign version byte must hit the downgrade guard; got: {msg}"
    );
}

/// Truncated / too-short v2 buffers must be rejected before indexing past
/// the end (length guard runs first).
#[test]
fn v2_too_short_rejected() {
    let enc = MercuryEncryption::from_session_key_v2(V2_SESSION_KEY);
    for len in [
        0usize,
        1,
        16,
        32,
        1 + V2_IV_LEN + AES_BLOCK_SIZE + HMAC_TAG_LEN - 1,
    ] {
        let buf = vec![V2_VERSION_BYTE; len];
        let err = enc.decrypt(&buf).unwrap_err();
        assert!(
            matches!(err, CimmeriaError::Encryption(_)),
            "v2 decrypt of {len}-byte buffer must error"
        );
    }
}

/// HKDF key separation: a v2 context does NOT reuse the raw session key for
/// AES, and `enc_key != mac_key`. Proves the keys are actually derived (with
/// distinct `info` labels), not passed through. Also confirms a real v2
/// frame round-trips through the derived keys.
#[test]
fn v2_keys_are_hkdf_derived_not_raw_session_key() {
    let session = [0x77u8; 32];
    let v2 = MercuryEncryption::from_session_key_v2(session);
    // Sanity: the derived context still round-trips a real frame.
    let frame = v2.encrypt(b"derived keys").unwrap();
    assert_eq!(v2.decrypt(&frame).unwrap(), b"derived keys");

    // The v2 enc_key/mac_key are HKDF-derived, not the raw session key.
    let (enc_key, mac_key) = v2_derive_keys(&session);
    assert_ne!(
        enc_key, session,
        "HKDF enc_key must differ from session key"
    );
    assert_ne!(
        mac_key, session,
        "HKDF mac_key must differ from session key"
    );
    assert_ne!(
        enc_key, mac_key,
        "HKDF enc_key and mac_key must be independent"
    );
}

/// v1 byte-exact regression: `from_session_key` is unchanged and still
/// produces a deterministic v1 frame (no version byte, no IV on the wire).
/// This guards against the v2 work accidentally altering the v1 path.
#[test]
fn v1_path_unchanged_by_v2_addition() {
    let enc = MercuryEncryption::from_session_key([0x42u8; 32]);
    let pt = b"v1 stays v1";
    let ct = enc.encrypt(pt).unwrap();

    // v1 frame is [ciphertext || 16-byte tag]; ciphertext is block-aligned
    // and the frame does NOT start with a version byte / 16-byte IV prefix.
    assert_eq!((ct.len() - HMAC_TAG_LEN) % AES_BLOCK_SIZE, 0);
    // Deterministic (zero IV): same input → same output.
    assert_eq!(ct, enc.encrypt(pt).unwrap());
    // Round-trips on the v1 path.
    assert_eq!(enc.decrypt(&ct).unwrap(), pt);
}

/// `from_config_u8` maps `1 → V1`, `2 → V2`, and anything else falls back to
/// the always-compatible v1 default. The fallback must never silently produce
/// v2 frames on a typo'd config.
#[test]
fn config_u8_maps_to_version() {
    assert_eq!(EncryptionVersion::from_config_u8(1), EncryptionVersion::V1);
    assert_eq!(EncryptionVersion::from_config_u8(2), EncryptionVersion::V2);
    assert_eq!(EncryptionVersion::from_config_u8(0), EncryptionVersion::V1);
    assert_eq!(EncryptionVersion::from_config_u8(3), EncryptionVersion::V1);
    assert_eq!(
        EncryptionVersion::from_config_u8(255),
        EncryptionVersion::V1
    );
}

/// The default `EncryptionVersion` is v1 — the only version unpatched clients
/// understand. A regression that flips the default would break every stock
/// client, so pin it.
#[test]
fn default_version_is_v1() {
    assert_eq!(EncryptionVersion::default(), EncryptionVersion::V1);
}

/// `from_session_key_versioned(key, V1)` is byte-identical to
/// `from_session_key(key)` — the versioned constructor must dispatch to the
/// exact legacy path, not a re-derived one, or the default session would stop
/// being wire-compatible with the stock client.
#[test]
fn versioned_v1_byte_identical_to_legacy() {
    let key = [0x42u8; 32];
    let pt = b"versioned v1 == legacy v1";
    let legacy = MercuryEncryption::from_session_key(key)
        .encrypt(pt)
        .unwrap();
    let versioned = MercuryEncryption::from_session_key_versioned(key, EncryptionVersion::V1)
        .encrypt(pt)
        .unwrap();
    assert_eq!(versioned, legacy);
}

/// `from_session_key_versioned(key, V2)` produces a v2 frame (leading `0x02`)
/// that round-trips through a v2 context built the same way, and is byte-identical
/// to the dedicated `from_session_key_v2` path for a pinned IV is not testable
/// here (random IV), so we assert frame shape + round-trip instead.
#[test]
fn versioned_v2_produces_v2_frame_and_round_trips() {
    let key = [0x99u8; 32];
    let enc = MercuryEncryption::from_session_key_versioned(key, EncryptionVersion::V2);
    let pt = b"versioned v2 frame";
    let ct = enc.encrypt(pt).unwrap();
    assert_eq!(ct[0], 0x02, "v2 frame must begin with the version byte");
    assert_eq!(enc.decrypt(&ct).unwrap(), pt);
}

/// Cross-version incompatibility through the versioned constructor: a v2 frame
/// fed to a v1 context fails, and a v1 frame fed to a v2 context fails (the
/// downgrade defense). This is the session-level assertion that the version
/// selection actually isolates the two wire formats.
#[test]
fn versioned_cross_version_frames_are_rejected() {
    let key = [0x77u8; 32];
    let v1 = MercuryEncryption::from_session_key_versioned(key, EncryptionVersion::V1);
    let v2 = MercuryEncryption::from_session_key_versioned(key, EncryptionVersion::V2);
    let pt = b"cross version";

    let v1_frame = v1.encrypt(pt).unwrap();
    let v2_frame = v2.encrypt(pt).unwrap();

    // v2 context rejects the v1 frame (no 0x02 version byte → downgrade defense).
    assert!(
        v2.decrypt(&v1_frame).is_err(),
        "v2 context must reject a v1 frame"
    );
    // v1 context cannot make sense of a v2 frame (HMAC-MD5 over the wrong
    // bytes / bad padding) → it must error rather than silently mis-decrypt.
    assert!(
        v1.decrypt(&v2_frame).is_err(),
        "v1 context must reject a v2 frame"
    );
}
