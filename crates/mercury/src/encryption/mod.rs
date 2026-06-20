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
//! # v2 — modernized
//!
//! ```text
//! [ 0x02 ][ 16-byte random IV ][ AES-256-CBC ciphertext (PKCS7) ][ 16-byte truncated HMAC-SHA256 ]
//! ```
//!
//! - **Per-packet random IV** from a CSPRNG (`rand`, seeded from the OS) —
//!   defeats the v1 zero-IV first-block leak.
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
                // Fresh per-packet IV from a CSPRNG (`rand`, seeded by the OS).
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

    // ----- v2 -----------------------------------------------------------------

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
mod tests;
