---
name: mercury-cipher-chain
description: PacketEncrypter vtable, AES-256-CBC+HMAC-MD5 via CryptoPP, zero IV per packet, key derivation confirmed no-KDF (W-auth session 2026-05-13)
metadata:
  type: project
---

> [!NOTE] PROMOTION TARGET: spec.protocol.cipher-and-auth §"PacketEncrypter object layout" + spec.protocol.mercury-wire-format §"cipher envelope"
>
> Triaged 2026-05-13 (Phase −0.5 step 4). Highest-confidence promotion target in this agent's memory. V5-confirmed against `findings/mercury-protocol-internals.md`. The "no KDF, raw 32-byte SOAP key → AES key" + "AES key = HMAC key" + "zero IV every packet" facts are canonical and supersede the prior "OpenSSL" doc comment in `encryption.rs`.

# Mercury Cipher Chain — W-auth Session Findings (2026-05-13)

**Why:** Re-verification after V5 campaign; prior doc-comments said "OpenSSL" but binary uses CryptoPP.

**How to apply:** When working on Mercury encryption, cipher filter construction, or session key
handling — these addresses and the no-KDF fact are load-bearing.

## Key Addresses

| Address | Role |
|---------|------|
| `0x01603a70` | `PacketEncrypter` constructor |
| `0x01b27374` | `PacketEncrypter` vtable |
| `0x01603b80` | `PacketEncrypter::send` (encrypt, vfunc_1) |
| `0x01603fa0` | `PacketEncrypter::recv` (decrypt, vfunc_2) |
| `0x0040e030` | AES Rijndael-256 encryptor init (CryptoPP) |
| `0x0040d000` | CBC-Encryption mode init (CryptoPP) |
| `0x0040d0b0` | CBC-Decryption mode init (CryptoPP) |
| `0x01604d00` | HMAC-MD5 init (CryptoPP) |
| `0x004089b0` | StreamTransformationFilter ctor (PKCS7 padding mode 4) |
| `0x00414720` | HashFilter ctor (HMAC output target) |
| `0x00ddfd00` | `register_NetIn_ServerSelectSuccess` — where PacketEncrypter is allocated |
| `0x015eb940` | gSOAP SessionKeyType deserializer (xsd:hexBinary → 32 bytes) |

## Confirmed Facts

- **No KDF**: 64-char hex SessionKey in SOAP → gSOAP hex decode → 32 raw bytes → AES key. No PBKDF, no hashing, no salting.
- **AES key = HMAC key**: Both read same 32-byte buffer at `PacketEncrypter+0x8`.
- **IV**: 16 zero bytes stored at `PacketEncrypter+0x18`, re-read but never mutated — same zero IV every packet.
- **Wire order**: Encrypt-then-MAC `[ciphertext][16-byte HMAC-MD5 tag]`
- **Padding**: PKCS#7 via CryptoPP StreamTransformationFilter
- **Library**: CryptoPP, NOT OpenSSL (previous doc-comment in encryption.rs was wrong)
- **Cimmeria `MercuryEncryption`**: Matches protocol exactly. `from_session_key` constructor is correct.

## Object Layout (PacketEncrypter)

```
+0x00  vtable ptr (0x01b27374)
+0x04  ref_count (SafeReferenceCount base)
+0x08  key_buf (vector-like: 32-byte AES+HMAC key)
+0x18  iv_buf (vector-like: 16 zero bytes)
```

## Activation Path

```
ProcessLoginAppResponse (0x00de0e8b)
  → register_NetIn_ServerSelectSuccess (0x00ddfd00)
    → FUN_01603a70 PacketEncrypter_ctor (key stored at this+0x7c)
      → FUN_016043a0 (PacketFilter base init)
      → FUN_01604bb0 (key buffer copy)
      → FUN_00a587f0 (IV buffer init: 16 zero bytes)
    → PacketEncrypter* stored at ServerConnection+0x310
```
