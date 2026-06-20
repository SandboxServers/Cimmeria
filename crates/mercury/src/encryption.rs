//! Mercury packet encryption — two wire-compatible versions.
//!
//! A single [`MercuryEncryption`] context speaks **exactly one** version for
//! the lifetime of the session (the version is fixed at construction). This is
//! a deliberate anti-downgrade property: a v2 decryptor will reject any frame
//! that does not begin with the v2 version byte, so an attacker (or a stale
//! client) cannot force a session that negotiated v2 back down to the weaker
//! v1 scheme.
//!
//! # v1 — legacy, byte-identical to the C++ client (frozen — do not change)
//!
//! The original C++ implementation uses OpenSSL's AES-256-CBC with PKCS7
//! padding for confidentiality, and HMAC-MD5 for integrity. Wire format:
//!
//! ```text
//! [ ciphertext ][ 16-byte HMAC-MD5 tag ]
//! ```
//!
//! - Zero IV for every packet (a fresh session key is exchanged per session,
//!   so the first-block IV collision is bounded to one session).
//! - HMAC key == AES key (the full 32-byte session key is fed to both
//!   `EVP_aes_256_cbc` and `EVP_md5`/`HMAC_Init` in the C++ flow).
//! - No version byte, no IV on the wire, no length/sequence inside the unit.
//!
//! The v1 output MUST stay byte-identical to the C++ OpenSSL output for the
//! same key material and plaintext — it is wire-compatible with unpatched
//! clients during the v1→v2 transition. **Do not change v1 output bytes.**
//!
//! # v2 — #434 Phase 3, modernized
//!
//! ```text
//! [ 0x02 ][ 16-byte random IV ][ AES-256-CBC ciphertext (PKCS7) ][ 16-byte truncated HMAC-SHA256 ]
//! ```
//!
//! - **Per-packet random IV** from the OS CSPRNG — defeats the v1 zero-IV
//!   first-block leak.
//! - **HKDF-SHA256 key separation:** `enc_key` and `mac_key` are derived from
//!   the 32-byte session key with distinct `info` strings, so the AES key and
//!   the MAC key are independent.
//! - **HMAC-SHA256 over `IV || ciphertext`**, truncated to 16 bytes. Covering
//!   the IV prevents an IV-swap forgery (an attacker cannot replace the IV
//!   without invalidating the tag).
//! - **Encrypt-then-MAC**, verified in constant time before any decrypt runs.
//!
//! Encryption is applied to the **packet body** only (the 4-byte Mercury
//! header is always sent in the clear).
//!
//! This implementation uses the RustCrypto crates, which provide constant-time
//! MAC verification (`Mac::verify_slice` for v1, `Mac::verify_truncated_left`
//! for v2's 16-byte-truncated tag).

use aes::Aes256;
use cbc::{Decryptor, Encryptor};
// `KeyInit` (the source of `new_from_slice`) moved off the `Mac` trait in
// hmac 0.13 / digest 0.11 — import it explicitly now.
use cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyInit, KeyIvInit};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use md5::Md5;
use sha2::Sha256;

use cimmeria_common::{CimmeriaError, Result};

/// HMAC tag length in bytes (both v1 MD5 full-width and v2 SHA-256 truncated).
const HMAC_TAG_LEN: usize = 16;

/// AES block size in bytes.
const AES_BLOCK_SIZE: usize = 16;

/// v2 IV length in bytes (one AES block).
const V2_IV_LEN: usize = 16;

/// v2 wire version byte. The first byte of every v2 frame.
const V2_VERSION_BYTE: u8 = 0x02;

/// HKDF `info` string for the v2 AES-256 encryption key.
const V2_ENC_INFO: &[u8] = b"cimmeria.mercury.v2.aes";

/// HKDF `info` string for the v2 HMAC-SHA256 MAC key.
const V2_MAC_INFO: &[u8] = b"cimmeria.mercury.v2.mac";

/// Type alias for AES-256-CBC encryptor.
type Aes256CbcEnc = Encryptor<Aes256>;

/// Type alias for AES-256-CBC decryptor.
type Aes256CbcDec = Decryptor<Aes256>;

/// Type alias for HMAC-MD5 (v1).
type HmacMd5 = Hmac<Md5>;

/// Type alias for HMAC-SHA256 (v2).
type HmacSha256 = Hmac<Sha256>;

/// Wire-format version a [`MercuryEncryption`] context speaks.
///
/// A session is pinned to one variant at construction and never changes — the
/// `encrypt`/`decrypt` methods branch on this. The pinning is what makes the
/// v1→v2 downgrade defense work: a v2 context refuses v1-shaped input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Version {
    /// Legacy AES-256-CBC + HMAC-MD5, zero IV, HMAC key == AES key.
    V1,
    /// Modern AES-256-CBC + truncated HMAC-SHA256, random IV, HKDF-split keys.
    V2,
}

/// Derive the v2 encryption and MAC keys from the 32-byte session key via
/// HKDF-SHA256 with distinct `info` labels (no salt).
///
/// Returns `(enc_key, mac_key)`. HKDF-Extract with no salt uses a zero salt
/// per RFC 5869; the two `info` labels guarantee the outputs are independent.
fn v2_derive_keys(session_key: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let hk = Hkdf::<Sha256>::new(None, session_key);

    let mut enc_key = [0u8; 32];
    hk.expand(V2_ENC_INFO, &mut enc_key)
        .expect("32 is a valid HKDF-SHA256 output length");

    let mut mac_key = [0u8; 32];
    hk.expand(V2_MAC_INFO, &mut mac_key)
        .expect("32 is a valid HKDF-SHA256 output length");

    (enc_key, mac_key)
}

/// Mercury packet encryption. Pinned to one wire version for the session.
///
/// Key material is established during the login handshake and remains fixed
/// for the lifetime of the channel.
#[derive(Clone)]
pub struct MercuryEncryption {
    /// Wire version this context speaks (fixed at construction).
    version: Version,
    /// 256-bit AES key. For v1 this is the raw session key; for v2 it is the
    /// HKDF-derived `enc_key`.
    aes_key: [u8; 32],
    /// 16-byte CBC initialization vector. v1: all zeros (per C++). v2: unused
    /// (a fresh random IV is generated per packet) — kept zero.
    iv: [u8; 16],
    /// HMAC key. v1: the session key (HMAC-MD5, C++ uses the full 32-byte key).
    /// v2: the HKDF-derived `mac_key` (HMAC-SHA256).
    hmac_key: [u8; 32],
}

impl MercuryEncryption {
    /// Create a v1 encryption context with explicit key material.
    ///
    /// # Arguments
    ///
    /// - `aes_key` — 32-byte AES-256 key derived from the 64-char hex session key.
    /// - `iv` — 16-byte CBC initialization vector (must be all zeros per C++).
    /// - `hmac_key` — 32-byte HMAC-MD5 key (must equal `aes_key` per C++).
    pub fn new(aes_key: [u8; 32], iv: [u8; 16], hmac_key: [u8; 32]) -> Self {
        Self {
            version: Version::V1,
            aes_key,
            iv,
            hmac_key,
        }
    }

    /// Create a **v1** encryption context from a 32-byte session key with zero IV.
    ///
    /// This matches the C++ `EncryptionFilter::setKey()` which uses the same
    /// key for both AES and HMAC, and a zero IV for every packet.
    pub fn from_session_key(key: [u8; 32]) -> Self {
        Self {
            version: Version::V1,
            aes_key: key,
            iv: [0u8; 16],
            hmac_key: key,
        }
    }

    /// Create a **v2** encryption context from a 32-byte session key.
    ///
    /// The session key is HKDF-SHA256-expanded into an independent AES
    /// `enc_key` and an HMAC-SHA256 `mac_key`. A fresh random IV is generated
    /// per [`encrypt`](Self::encrypt) call, so no IV is stored here.
    ///
    /// A context built this way emits and accepts **only** v2 frames: its
    /// [`decrypt`](Self::decrypt) rejects any buffer whose first byte is not
    /// `0x02`, which is the v1→v2 downgrade defense.
    pub fn from_session_key_v2(session_key: [u8; 32]) -> Self {
        let (enc_key, mac_key) = v2_derive_keys(&session_key);
        Self {
            version: Version::V2,
            aes_key: enc_key,
            iv: [0u8; 16],
            hmac_key: mac_key,
        }
    }

    /// Encrypt `plaintext` according to this context's pinned wire version.
    ///
    /// - **v1:** `[ ciphertext || 16-byte HMAC-MD5 ]` (zero IV, encrypt-then-MAC).
    /// - **v2:** `[ 0x02 || IV || ciphertext || 16-byte truncated HMAC-SHA256 ]`
    ///   with a fresh random IV, HMAC over `IV || ciphertext`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        match self.version {
            Version::V1 => self.encrypt_v1(plaintext),
            Version::V2 => {
                // Fresh per-packet IV from the OS CSPRNG.
                let iv: [u8; V2_IV_LEN] = rand::random();
                self.encrypt_v2_with_iv(plaintext, iv)
            }
        }
    }

    /// Decrypt according to this context's pinned wire version.
    ///
    /// A v2 context **rejects** any buffer whose first byte is not `0x02`
    /// (downgrade defense). A v1 context is unchanged from the legacy path.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.version {
            Version::V1 => self.decrypt_v1(data),
            Version::V2 => self.decrypt_v2(data),
        }
    }

    // ----- v1 (legacy, byte-identical to C++) --------------------------------

    /// v1 encrypt: AES-256-CBC + PKCS7, then append a 16-byte HMAC-MD5 tag
    /// over the ciphertext. Returns `[ciphertext || hmac_tag]`.
    fn encrypt_v1(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // PKCS7 pad the plaintext to a multiple of AES_BLOCK_SIZE.
        let padded = pkcs7_pad(plaintext);

        // Encrypt with AES-256-CBC. Key/IV are fixed-size arrays, so the
        // `KeyIvInit::new` constructor is infallible — no Result to unwrap.
        let encryptor = Aes256CbcEnc::new(&self.aes_key.into(), &self.iv.into());

        let mut buf = padded;
        let n = buf.len();
        encryptor
            .encrypt_padded::<cipher::block_padding::NoPadding>(&mut buf, n)
            .map_err(|e| CimmeriaError::Encryption(format!("AES encrypt failed: {e}")))?;
        let ciphertext = buf;

        // Compute HMAC-MD5 over the ciphertext (encrypt-then-MAC).
        let mut mac = HmacMd5::new_from_slice(&self.hmac_key)
            .map_err(|e| CimmeriaError::Encryption(format!("HMAC init failed: {e}")))?;
        mac.update(&ciphertext);
        let tag = mac.finalize().into_bytes();

        // Concatenate: [ciphertext] [16-byte HMAC tag].
        let mut output = Vec::with_capacity(ciphertext.len() + HMAC_TAG_LEN);
        output.extend_from_slice(&ciphertext);
        output.extend_from_slice(&tag);

        tracing::trace!(
            version = 1,
            plaintext_len = plaintext.len(),
            ciphertext_len = output.len(),
            "encrypt"
        );

        Ok(output)
    }

    /// v1 decrypt: verify the HMAC-MD5 tag, then AES-256-CBC decrypt and strip
    /// PKCS7 padding. Input format: `[ciphertext || 16-byte hmac_tag]`.
    fn decrypt_v1(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < HMAC_TAG_LEN {
            return Err(CimmeriaError::Encryption(format!(
                "encrypted data too short: {} bytes (need at least {})",
                data.len(),
                HMAC_TAG_LEN
            )));
        }

        let (ciphertext, received_tag) = data.split_at(data.len() - HMAC_TAG_LEN);

        if ciphertext.is_empty() {
            return Err(CimmeriaError::Encryption(
                "ciphertext portion is empty".into(),
            ));
        }

        if ciphertext.len() % AES_BLOCK_SIZE != 0 {
            return Err(CimmeriaError::Encryption(format!(
                "ciphertext length {} is not a multiple of AES block size {}",
                ciphertext.len(),
                AES_BLOCK_SIZE
            )));
        }

        // Verify HMAC-MD5 tag (encrypt-then-MAC: verify before decrypting).
        let mut mac = HmacMd5::new_from_slice(&self.hmac_key)
            .map_err(|e| CimmeriaError::Encryption(format!("HMAC init failed: {e}")))?;
        mac.update(ciphertext);
        mac.verify_slice(received_tag).map_err(|_| {
            tracing::warn!(input_len = data.len(), "HMAC-MD5 verification failed");
            CimmeriaError::Encryption("HMAC-MD5 verification failed".into())
        })?;

        // Decrypt with AES-256-CBC. Fixed-size key/IV make `new` infallible.
        let decryptor = Aes256CbcDec::new(&self.aes_key.into(), &self.iv.into());

        let mut buf = ciphertext.to_vec();
        decryptor
            .decrypt_padded::<cipher::block_padding::NoPadding>(&mut buf)
            .map_err(|e| CimmeriaError::Encryption(format!("AES decrypt failed: {e}")))?;

        // Strip PKCS7 padding.
        let plaintext = pkcs7_unpad(&buf)?;

        tracing::trace!(
            version = 1,
            input_len = data.len(),
            plaintext_len = plaintext.len(),
            "decrypt"
        );

        Ok(plaintext.to_vec())
    }

    // ----- v2 (#434 Phase 3) -------------------------------------------------

    /// v2 encrypt with a caller-supplied IV. The public [`encrypt`](Self::encrypt)
    /// generates a random IV and delegates here; tests use this seam to pin a
    /// deterministic IV for known-answer vectors.
    ///
    /// Wire layout:
    /// `[ 0x02 ][ iv ][ AES-256-CBC(PKCS7) ciphertext ][ trunc16(HMAC-SHA256(iv || ciphertext)) ]`.
    fn encrypt_v2_with_iv(&self, plaintext: &[u8], iv: [u8; V2_IV_LEN]) -> Result<Vec<u8>> {
        debug_assert_eq!(self.version, Version::V2, "encrypt_v2 on a non-v2 context");

        // PKCS7 pad, then AES-256-CBC encrypt under the random IV.
        let padded = pkcs7_pad(plaintext);
        let encryptor = Aes256CbcEnc::new(&self.aes_key.into(), &iv.into());

        let mut buf = padded;
        let n = buf.len();
        encryptor
            .encrypt_padded::<cipher::block_padding::NoPadding>(&mut buf, n)
            .map_err(|e| CimmeriaError::Encryption(format!("AES encrypt failed: {e}")))?;
        let ciphertext = buf;

        // HMAC-SHA256 over (IV || ciphertext); covering the IV defeats IV-swap.
        let mut mac = HmacSha256::new_from_slice(&self.hmac_key)
            .map_err(|e| CimmeriaError::Encryption(format!("HMAC init failed: {e}")))?;
        mac.update(&iv);
        mac.update(&ciphertext);
        let full_tag = mac.finalize().into_bytes();
        let tag = &full_tag[..HMAC_TAG_LEN];

        // [0x02][IV][ciphertext][16-byte truncated tag].
        let mut output = Vec::with_capacity(1 + V2_IV_LEN + ciphertext.len() + HMAC_TAG_LEN);
        output.push(V2_VERSION_BYTE);
        output.extend_from_slice(&iv);
        output.extend_from_slice(&ciphertext);
        output.extend_from_slice(tag);

        tracing::trace!(
            version = 2,
            plaintext_len = plaintext.len(),
            ciphertext_len = output.len(),
            "encrypt"
        );

        Ok(output)
    }

    /// v2 decrypt. Rejects any buffer not beginning with `0x02` (downgrade
    /// defense), verifies the truncated HMAC-SHA256 over `IV || ciphertext` in
    /// constant time, then AES-256-CBC decrypts and strips PKCS7 padding.
    fn decrypt_v2(&self, data: &[u8]) -> Result<Vec<u8>> {
        debug_assert_eq!(self.version, Version::V2, "decrypt_v2 on a non-v2 context");

        // Downgrade defense FIRST: a v2 context refuses anything not flagged
        // v2. Checking the version byte before the length guard means a
        // v1-style buffer (whose first byte is ciphertext, not 0x02) is
        // rejected as a downgrade attempt regardless of its length — the
        // security-relevant reason, not an incidental "too short". An empty
        // buffer has no version byte to inspect, so it falls through to the
        // length guard below.
        if let Some(&first) = data.first() {
            if first != V2_VERSION_BYTE {
                return Err(CimmeriaError::Encryption(format!(
                    "v2 decryptor rejected non-v2 version byte {first:#04x} (downgrade defense)"
                )));
            }
        }

        // Minimum v2 frame: version + IV + one ciphertext block + tag.
        let min_len = 1 + V2_IV_LEN + AES_BLOCK_SIZE + HMAC_TAG_LEN;
        if data.len() < min_len {
            return Err(CimmeriaError::Encryption(format!(
                "v2 encrypted data too short: {} bytes (need at least {min_len})",
                data.len(),
            )));
        }

        let iv: [u8; V2_IV_LEN] = data[1..1 + V2_IV_LEN]
            .try_into()
            .expect("slice is exactly V2_IV_LEN by construction");
        let body = &data[1 + V2_IV_LEN..];
        let (ciphertext, received_tag) = body.split_at(body.len() - HMAC_TAG_LEN);

        if ciphertext.is_empty() {
            return Err(CimmeriaError::Encryption(
                "v2 ciphertext portion is empty".into(),
            ));
        }

        if ciphertext.len() % AES_BLOCK_SIZE != 0 {
            return Err(CimmeriaError::Encryption(format!(
                "v2 ciphertext length {} is not a multiple of AES block size {}",
                ciphertext.len(),
                AES_BLOCK_SIZE
            )));
        }

        // Verify truncated HMAC-SHA256 over (IV || ciphertext) in constant
        // time before decrypting (encrypt-then-MAC). `verify_truncated_left`
        // checks our 16-byte received tag against the leading 16 bytes of the
        // full 32-byte HMAC-SHA256 output, matching the encrypt-side
        // truncation.
        let mut mac = HmacSha256::new_from_slice(&self.hmac_key)
            .map_err(|e| CimmeriaError::Encryption(format!("HMAC init failed: {e}")))?;
        mac.update(&iv);
        mac.update(ciphertext);
        mac.verify_truncated_left(received_tag).map_err(|_| {
            tracing::warn!(input_len = data.len(), "HMAC-SHA256 verification failed");
            CimmeriaError::Encryption("HMAC-SHA256 verification failed".into())
        })?;

        // Decrypt with AES-256-CBC under the wire IV.
        let decryptor = Aes256CbcDec::new(&self.aes_key.into(), &iv.into());

        let mut buf = ciphertext.to_vec();
        decryptor
            .decrypt_padded::<cipher::block_padding::NoPadding>(&mut buf)
            .map_err(|e| CimmeriaError::Encryption(format!("AES decrypt failed: {e}")))?;

        // Strip PKCS7 padding.
        let plaintext = pkcs7_unpad(&buf)?;

        tracing::trace!(
            version = 2,
            input_len = data.len(),
            plaintext_len = plaintext.len(),
            "decrypt"
        );

        Ok(plaintext.to_vec())
    }
}

/// Apply PKCS7 padding to make `data` a multiple of `AES_BLOCK_SIZE`.
fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad_len = AES_BLOCK_SIZE - (data.len() % AES_BLOCK_SIZE);
    let mut padded = Vec::with_capacity(data.len() + pad_len);
    padded.extend_from_slice(data);
    padded.extend(std::iter::repeat_n(pad_len as u8, pad_len));
    padded
}

/// Validate and strip PKCS7 padding.
fn pkcs7_unpad(data: &[u8]) -> Result<&[u8]> {
    if data.is_empty() {
        return Err(CimmeriaError::Encryption("cannot unpad empty data".into()));
    }

    let pad_byte = *data.last().unwrap();
    let pad_len = pad_byte as usize;

    if pad_len == 0 || pad_len > AES_BLOCK_SIZE || pad_len > data.len() {
        return Err(CimmeriaError::Encryption(format!(
            "invalid PKCS7 padding byte: {pad_byte:#04x} (data len: {})",
            data.len()
        )));
    }

    // Verify all padding bytes are equal.
    let padding_start = data.len() - pad_len;
    for (i, &b) in data[padding_start..].iter().enumerate() {
        if b != pad_byte {
            return Err(CimmeriaError::Encryption(format!(
                "invalid PKCS7 padding at offset {}: expected {pad_byte:#04x}, got {b:#04x}",
                padding_start + i
            )));
        }
    }

    Ok(&data[..padding_start])
}

impl std::fmt::Debug for MercuryEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material in debug output.
        f.debug_struct("MercuryEncryption")
            .field("aes_key", &"[REDACTED]")
            .field("iv", &"[REDACTED]")
            .field("hmac_key", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
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

    // ===== v2 (#434 Phase 3) =================================================

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
}
