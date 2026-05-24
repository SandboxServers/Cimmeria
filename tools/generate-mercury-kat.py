"""Generate Mercury known-answer test (KAT) reference vectors.

Computes the expected ciphertext for the Mercury AES-256-CBC + HMAC-MD5
pipeline using a reference implementation independent of both
CryptoPP (the SGW.exe C++ stack) and RustCrypto (the Cimmeria
Rust stack). Output is the byte-exact reference the Rust test
asserts against.

Algorithm parameters pinned by this script (all from the
`MercuryEncryption` doc-comments in
`crates/mercury/src/encryption.rs`, themselves derived from the
C++ `EncryptionFilter::setKey` flow):

- AES-256 key = 32 bytes; HMAC-MD5 key = same 32 bytes
  (`EncryptionFilter::setKey` passes the full session key both
  ways).
- CBC IV = all zeros (every packet uses the same IV — this is
  acceptable because the auth handshake derives a fresh per-
  session key, so two packets never share both IV and key).
- Padding = PKCS7 (the C++ stack uses Crypto++'s default for
  `CBC_Encryption<Rijndael>`).
- Order = encrypt-then-MAC. HMAC is computed over the ciphertext
  AFTER encryption.
- Output layout = `[ciphertext || hmac_tag]` (16-byte tag appended).

Run with:
    python tools/generate-mercury-kat.py

Output is a Rust snippet ready to paste into
`crates/mercury/src/test_harness/tests/encryption.rs`.
"""

import hashlib
import hmac
from cryptography.hazmat.primitives import padding
from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes

AES_BLOCK_SIZE = 16
HMAC_TAG_LEN = 16


def mercury_encrypt(key: bytes, plaintext: bytes) -> bytes:
    """Reference Mercury encryption — independent of CryptoPP and RustCrypto."""
    assert len(key) == 32, "AES-256 key must be 32 bytes"

    # PKCS7 pad to AES block size.
    padder = padding.PKCS7(AES_BLOCK_SIZE * 8).padder()
    padded = padder.update(plaintext) + padder.finalize()

    # AES-256-CBC with zero IV.
    iv = b"\x00" * AES_BLOCK_SIZE
    cipher = Cipher(algorithms.AES(key), modes.CBC(iv))
    encryptor = cipher.encryptor()
    ciphertext = encryptor.update(padded) + encryptor.finalize()

    # HMAC-MD5 over the ciphertext, HMAC key = AES key.
    tag = hmac.new(key, ciphertext, hashlib.md5).digest()
    assert len(tag) == HMAC_TAG_LEN

    return ciphertext + tag


def emit_rust(name: str, plaintext: bytes, key_byte: int) -> str:
    """Emit a Rust `const` declaration for the KAT pair."""
    key = bytes([key_byte] * 32)
    expected = mercury_encrypt(key, plaintext)
    hex_pt = ", ".join(f"0x{b:02X}" for b in plaintext)
    hex_ct = ", ".join(f"0x{b:02X}" for b in expected)
    return (
        f"// {name}: plaintext={len(plaintext)} bytes, "
        f"ciphertext+tag={len(expected)} bytes, key=[0x{key_byte:02X}; 32]\n"
        f"const {name}_PLAINTEXT: &[u8] = &[{hex_pt}];\n"
        f"const {name}_KEY_BYTE: u8 = 0x{key_byte:02X};\n"
        f"const {name}_EXPECTED: &[u8] = &[{hex_ct}];\n"
    )


def main() -> None:
    # Three samples chosen to exercise:
    # SHORT — single 16-byte plaintext = 1 AES block before padding;
    #         padding adds a full block → 32-byte ciphertext + 16-byte tag.
    short_plaintext = b"hello mercury!!!"  # 16 bytes exactly
    assert len(short_plaintext) == 16

    # MID — 257-byte plaintext = crosses 16 AES block boundaries plus a
    #       partial block. PKCS7 pads to 272 bytes → 17 blocks. Catches
    #       any off-by-one in block-boundary handling.
    mid_plaintext = bytes((i * 7 + 13) & 0xFF for i in range(257))
    assert len(mid_plaintext) == 257

    # LONG — 1024-byte plaintext = exactly 64 AES blocks before padding;
    #         padding adds a full block → 65 blocks ciphertext + 16-byte
    #         tag. Mirrors a fragment-sized payload (one fragment is
    #         FRAGMENT_BODY_SIZE = 1300 bytes max; 1024 fits comfortably
    #         in a single fragment and exercises the bulk-encrypt path).
    long_plaintext = bytes((i ^ 0x5A) & 0xFF for i in range(1024))
    assert len(long_plaintext) == 1024

    print(emit_rust("KAT_SHORT", short_plaintext, 0x42))
    print(emit_rust("KAT_MID", mid_plaintext, 0xA5))
    print(emit_rust("KAT_LONG", long_plaintext, 0xC3))


if __name__ == "__main__":
    main()
