//! Mercury encryption: AES-256-CBC + HMAC-MD5.
//!
//! The original C++ implementation uses OpenSSL's AES-256-CBC with PKCS7
//! padding for confidentiality, and HMAC-MD5 for integrity. The encrypted
//! wire format is:
//!
//! ```text
//! [encrypted_payload] [16-byte HMAC-MD5 tag]
//! ```
//!
//! Encryption is applied to the **packet body** only (the 4-byte Mercury
//! header is always sent in the clear).
//!
//! This implementation uses the RustCrypto crates which provide constant-time
//! operations. The output MUST be byte-identical to the C++ OpenSSL output
//! for the same key material and plaintext.

use aes::Aes256;
use cbc::{Decryptor, Encryptor};
use cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use md5::Md5;

use cimmeria_common::{CimmeriaError, Result};

/// HMAC-MD5 tag length in bytes.
const HMAC_TAG_LEN: usize = 16;

/// AES block size in bytes.
const AES_BLOCK_SIZE: usize = 16;

/// Type alias for AES-256-CBC encryptor.
type Aes256CbcEnc = Encryptor<Aes256>;

/// Type alias for AES-256-CBC decryptor.
type Aes256CbcDec = Decryptor<Aes256>;

/// Type alias for HMAC-MD5.
type HmacMd5 = Hmac<Md5>;

/// Mercury packet encryption using AES-256-CBC + HMAC-MD5.
///
/// Key material is established during the login handshake and remains
/// fixed for the lifetime of the channel.
#[derive(Clone)]
pub struct MercuryEncryption {
    /// 256-bit AES key.
    aes_key: [u8; 32],
    /// 128-bit IV for CBC mode.
    iv: [u8; 16],
    /// 128-bit key for HMAC-MD5.
    hmac_key: [u8; 16],
}

impl MercuryEncryption {
    /// Create a new encryption context.
    ///
    /// # Arguments
    ///
    /// - `aes_key` — 32-byte AES-256 key.
    /// - `iv` — 16-byte CBC initialization vector.
    /// - `hmac_key` — 16-byte HMAC-MD5 key.
    pub fn new(aes_key: [u8; 32], iv: [u8; 16], hmac_key: [u8; 16]) -> Self {
        Self {
            aes_key,
            iv,
            hmac_key,
        }
    }

    /// Encrypt `plaintext` using AES-256-CBC with PKCS7 padding, then
    /// append a 16-byte HMAC-MD5 tag over the ciphertext.
    ///
    /// Returns `[ciphertext || hmac_tag]`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        // PKCS7 pad the plaintext to a multiple of AES_BLOCK_SIZE.
        let padded = pkcs7_pad(plaintext);

        // Encrypt with AES-256-CBC.
        let encryptor = Aes256CbcEnc::new_from_slices(&self.aes_key, &self.iv)
            .map_err(|e| CimmeriaError::Encryption(format!("AES init failed: {e}")))?;

        let mut buf = padded;
        let n = buf.len();
        encryptor
            .encrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buf, n)
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

        Ok(output)
    }

    /// Verify the HMAC-MD5 tag, then decrypt the AES-256-CBC ciphertext
    /// and strip PKCS7 padding.
    ///
    /// Input format: `[ciphertext || 16-byte hmac_tag]`.
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
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
        mac.verify_slice(received_tag)
            .map_err(|_| CimmeriaError::Encryption("HMAC-MD5 verification failed".into()))?;

        // Decrypt with AES-256-CBC.
        let decryptor = Aes256CbcDec::new_from_slices(&self.aes_key, &self.iv)
            .map_err(|e| CimmeriaError::Encryption(format!("AES init failed: {e}")))?;

        let mut buf = ciphertext.to_vec();
        decryptor
            .decrypt_padded_mut::<cipher::block_padding::NoPadding>(&mut buf)
            .map_err(|e| CimmeriaError::Encryption(format!("AES decrypt failed: {e}")))?;

        // Strip PKCS7 padding.
        let plaintext = pkcs7_unpad(&buf)?;

        Ok(plaintext.to_vec())
    }
}

/// Apply PKCS7 padding to make `data` a multiple of `AES_BLOCK_SIZE`.
fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad_len = AES_BLOCK_SIZE - (data.len() % AES_BLOCK_SIZE);
    let mut padded = Vec::with_capacity(data.len() + pad_len);
    padded.extend_from_slice(data);
    padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    padded
}

/// Validate and strip PKCS7 padding.
fn pkcs7_unpad(data: &[u8]) -> Result<&[u8]> {
    if data.is_empty() {
        return Err(CimmeriaError::Encryption(
            "cannot unpad empty data".into(),
        ));
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
        let hmac_key = [0x37u8; 16];
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
}
