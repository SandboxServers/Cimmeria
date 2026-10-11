//! TG-MER-08: a Mercury encrypt and decrypt of one packet emits no per-packet
//! `encrypt` or `decrypt` TRACE rows, on either wire version. The rows were
//! about 63M lines a week in `cimmeria-trace`. Fails when the rows are restored.

use cimmeria_mercury::encryption::MercuryEncryption;

use crate::test_support::LogCapture;

#[test]
fn encrypt_decrypt_emit_no_per_packet_rows() {
    let capture = LogCapture::install();
    let v1 = MercuryEncryption::from_session_key([7u8; 32]);
    let v2 = MercuryEncryption::from_session_key_v2([7u8; 32]);

    for enc in [&v1, &v2] {
        let ciphertext = enc.encrypt(b"per-packet payload").expect("encrypt");
        let plaintext = enc.decrypt(&ciphertext).expect("decrypt");
        assert_eq!(plaintext, b"per-packet payload");
    }

    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("encrypt") || c.message_contains("decrypt"))
        .collect();
    assert!(
        rows.is_empty(),
        "per-packet encrypt/decrypt rows must not be emitted, got {rows:?}"
    );
}
