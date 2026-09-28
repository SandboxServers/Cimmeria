//! Launcher manifest signatures: Ed25519 over the exact `manifest.json`
//! bytes, written as 128 lowercase hex characters to `manifest.json.sig`.
//! The launcher verifies against the public key baked in at release time
//! (`LAUNCHER_MANIFEST_PUBKEY_HEX`, see
//! `docs/client/launcher-distribution-setup.md`).

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use crate::{PatchsetError, Result};

fn hex_decode<const N: usize>(s: &str, what: &str) -> Result<[u8; N]> {
    let s = s.trim();
    if s.len() != N * 2 {
        return Err(PatchsetError::Invalid(format!(
            "{what}: expected {} hex characters, got {}",
            N * 2,
            s.len()
        )));
    }
    let mut out = [0u8; N];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .map_err(|_| PatchsetError::Invalid(format!("{what}: not hex")))?;
    }
    Ok(out)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Sign `manifest` with the 64-hex-character private key. Returns the
/// signature as hex.
pub fn sign(private_key_hex: &str, manifest: &[u8]) -> Result<String> {
    let key = SigningKey::from_bytes(&hex_decode::<32>(private_key_hex, "private key")?);
    Ok(hex(&key.sign(manifest).to_bytes()))
}

/// The public key for a private key, as hex. Lets the operator check a key
/// file against the `LAUNCHER_MANIFEST_PUBKEY_HEX` secret.
pub fn public_key(private_key_hex: &str) -> Result<String> {
    let key = SigningKey::from_bytes(&hex_decode::<32>(private_key_hex, "private key")?);
    Ok(hex(key.verifying_key().as_bytes()))
}

/// Check `signature_hex` over `manifest` against `public_key_hex`.
pub fn verify(public_key_hex: &str, manifest: &[u8], signature_hex: &str) -> Result<()> {
    let key = VerifyingKey::from_bytes(&hex_decode::<32>(public_key_hex, "public key")?)
        .map_err(|e| PatchsetError::Invalid(format!("public key: {e}")))?;
    let sig = Signature::from_bytes(&hex_decode::<64>(signature_hex, "signature")?);
    key.verify(manifest, &sig)
        .map_err(|_| PatchsetError::Invalid("signature does not verify".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a";

    #[test]
    fn sign_then_verify_and_reject_tampering() {
        let body = br#"{"schema":1}"#;
        let sig = sign(KEY, body).unwrap();
        assert_eq!(sig.len(), 128);
        let pubkey = public_key(KEY).unwrap();
        verify(&pubkey, body, &sig).unwrap();
        assert!(verify(&pubkey, br#"{"schema":2}"#, &sig).is_err());
    }

    // The launcher's debug builds verify against the key made from 32 bytes
    // of 0x2a (manifest.rs DEV_MANIFEST_PRIVKEY); a signature from this tool
    // must verify there too, so pin the derived public key.
    #[test]
    fn dev_key_matches_the_launchers_dev_public_key() {
        let expected = VerifyingKey::from(&SigningKey::from_bytes(&[0x2a; 32]));
        assert_eq!(public_key(KEY).unwrap(), hex(expected.as_bytes()));
    }
}
