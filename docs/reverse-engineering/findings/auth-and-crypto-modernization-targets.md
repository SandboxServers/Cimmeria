# Auth & Crypto Modernization Targets — Client Binary Analysis

> **Last updated**: 2026-06-19
> **Source**: SGW.exe Ghidra decompilation (image base `0x00400000`, ASLR disabled)
> **Confidence**: HIGH for binary anatomy

---

This finding de-risks issue **#434** (encryption modernization). It maps the four
client-side surfaces the rewrite must touch — login transport, the client-side
SHA-1 password site, the anti-debug/anti-tamper posture, and the Mercury packet
crypto — to exact addresses in `SGW.exe`, and records where a patch hook should
land for each. The design that consumes these targets is the ADR at
[../../architecture/encryption-modernization.md](../../architecture/encryption-modernization.md).

All addresses are static (`SGW.exe` loads at its preferred base `0x00400000`;
ASLR is disabled), so they double as live breakpoint addresses in x64dbg.

---

## 1. Login transport — libcurl, not gSOAP

The SOAP auth login uses statically-linked **libcurl 7.17.0** for HTTP transport.
gSOAP 2.7 is present, but **only as an XML serialization layer** — it never owns
the socket. The gSOAP `tcp_connect` / `fsend` / `frecv` symbols appear in the
binary solely as dead diagnostic strings, not as called functions.

> **This corrects the prior assumption** (and any earlier RE note implying gSOAP
> owns the TCP login socket) that login could be intercepted at the gSOAP layer.
> It cannot — gSOAP hands a serialized XML body to libcurl, and libcurl does all
> socket I/O.

### Call chain

```text
logOnBegin                          0x00DDF580
  → LoginReplyHandler ctor          0x00DDED60
      curl_easy_init      @ thunk   0x013A96A0
      curl_multi_init     @ thunk   0x013A7970
      curl_easy_setopt    @ thunk   0x013A96E0   (×9 calls)
      curl_multi_add_handle @ thunk 0x013A9040
  → BW_servconn pump                0x00DE10B0
      curl_multi_perform  @ thunk   0x013A9210   (actual socket I/O, do/while loop)
```

`curl_multi_perform` at `0x013A9210` is where the socket actually drains, inside
a `do { … } while` pump loop in `BW_servconn`. libcurl is **statically linked** —
there is no DLL import (no curl IAT entries); the `0x013A9xxx` addresses are
internal call thunks, not import stubs.

### CURLOPT codes observed at the `curl_easy_setopt` site

| CURLOPT | Code | Meaning |
|---|---|---|
| `CURLOPT_URL` | `0x2712` | request URL |
| `CURLOPT_POSTFIELDS` | `0x271F` | POST body (the serialized SOAP XML) |
| `CURLOPT_HTTPHEADER` | `0x2751` | header list |
| `CURLOPT_POST` | `0x4E` | POST method flag |
| `CURLOPT_WRITEFUNCTION` | `0x4E2B` | response-body callback |
| `CURLOPT_TIMEOUT` | `0x0D` | request timeout |

### Endpoint URL

A hardcoded fallback URL string `"http://www.stargateworlds.com/xml/sgwlogin"`
lives at `0x019CEB8C`. Note the scheme: **plain HTTP**. The live endpoint URL is
*not* this constant in practice — it arrives at runtime via `logOnBegin`'s
server-address parameter; the constant is the baked-in fallback.

---

## 2. TLS injection target (issue #434 Phase 1)

**Recommended hook:** an inline detour at `curl_easy_setopt` (`0x013A96E0`).
Intercept the `CURLOPT_URL` set (option code `0x2712`) and rewrite the URL's
scheme/host before curl ever uses it.

**The wrinkle that drives the whole Phase 1 design:** the client's bundled
OpenSSL is **0.9.8g (2007)**. It predates TLS 1.2 and SNI and cannot complete a
handshake against any modern HTTPS endpoint. Reusing curl's own SSL stack is
therefore a dead end.

So the recommended approach is **not** to make curl speak TLS. Instead:

1. The injected shim rewrites the URL to a **localhost loopback** (curl talks
   plain HTTP to `127.0.0.1`).
2. A **modern rustls proxy inside the injected shim** terminates that plain HTTP
   and re-originates the request as proper TLS 1.3 upstream.

curl never sees TLS; its ancient OpenSSL is bypassed entirely.

**Debugger confirmation:** `bp 0x013A96E0`, watch `arg2 == 0x2712` → confirm the
URL argument is `http://…` (proving the scheme is rewritable here and the
transport is unencrypted at this seam).

---

## 3. Client SHA-1 password site (Phase 2)

The password is hashed **inline in `logOnBegin`** before it goes on the wire:

- SHA-1 constructor `FUN_0040D270` @ `0x0040D270`.
- CryptoPP `HexEncoder`, **uppercase**, `GroupSize = 0` → a 40-char hex string.
- Result is stored at `ServerConnection + 0x130`.
- **The plaintext password is stored at `ServerConnection + 0x3C`** (a
  `std::string`) *before* hashing. Input is ASCII.

**Cleanest Phase 2 patch:** hook the `LoginReplyHandler` ctor @ `0x00DDED60` and
swap `c_str(this + 0x3C)` (the plaintext) in for the hashed-password argument, so
the plaintext travels over the now-TLS-protected channel and the server does the
modern hashing. (This is only safe *because* Phase 1 has wrapped the transport in
TLS — never send plaintext over the legacy plain-HTTP path.)

> **Loose end to verify in x64dbg:** a *second* `logOnBegin` variant references
> the same debug string near `0x019CF248`. Confirm it has **no separate SHA-1
> path** before declaring the swap complete — otherwise one login route would
> still emit a hashed password.

---

## 4. Anti-debug / anti-tamper — CLEAN

There is **no anti-debug and no anti-tamper** in this binary. Every API that
*could* be used for detection is present in the IAT but has **zero code xrefs**
(dead imports):

| Import | IAT address | xrefs |
|---|---|---|
| `IsDebuggerPresent` | `0x01D6B052` | 0 |
| `OutputDebugStringA` / `OutputDebugStringW` | — | 0 |
| `GetTickCount` | — | 0 |
| `QueryPerformanceCounter` | — | 0 |
| `ContinueDebugEvent` | — | 0 |

Additionally:

- **No PEB `fs:[30h]` `BeingDebugged` probes** (0 hits).
- **No `NtGlobalFlag` checks, no DR-register (hardware breakpoint) checks.**
- **No INT3 / `0xCC` scanning loop.** The `0xCC` clusters in `.text` are MSVC
  inter-function padding, not a self-scan.
- **No TLS callbacks.** The `"TLS Directory"` string at `0x01B1C39C` belongs to
  UE3's own PE inspector, not to a TLS callback table.
- **No `.text` CRC / `MapFileAndCheckSum` self-integrity check.**
- Binary is **not packed**; ASLR is disabled (loads at `0x00400000`).
- The `Debugger.ini` / `UnDebuggerCore` strings are UnrealScript's *script*
  debugger, not a native anti-debug.

> **Verdict:** DLL injection (`CreateRemoteThread` or search-order hijack) plus
> inline / IAT / vtable hooking are **all safe**. There is nothing to neutralize
> before patching.

---

## 5. Mercury crypto internals (Phase 3) — v1 == our server, byte-exact

The client's Mercury packet crypto is implemented by a CryptoPP `PacketEncrypter`:

| Component | Address |
|---|---|
| `PacketEncrypter` ctor | `0x01603A70` |
| `PacketEncrypter` vtable | `0x01B27374` |
| `filterOut` / encrypt | `0x01603B80` |
| `filterIn` / decrypt | `0x01603FA0` |

### Confirmed v1 scheme

- **Cipher:** AES-256-CBC (Rijndael Enc vtable `0x0040E030` / Dec vtable
  `0x01604EA0`), CBC mode (`0x0040D000` / `0x0040D0B0`).
- **Padding:** PKCS7 (`PKCS_PADDING = 4`).
- **MAC:** HMAC-MD5 (`CryptoPP::HMAC<Weak1::MD5>` vtable `0x01604D00`), computed
  **over ciphertext only** — encrypt-then-MAC (the `HashFilter` at `0x00414720`
  sits downstream of the `StreamTransformationFilter`). Tag is **16 bytes**.
- **Key reuse:** HMAC key **== AES key** (the same 32 bytes).
- **IV:** a **16-byte zero IV**, stored once in the ctor at `this + 0x18` and
  **reused** for every packet.
- **No KDF:** the session key is copied **verbatim** into `this + 0x8`.
- **Decrypt path:** `filterIn` uses a `HashVerificationFilter` (`0x00409440`),
  **verify-then-decrypt**, and the comparison is **NOT constant-time**
  (memcmp-style).

### Wire layout (v1)

```text
[ AES-256-CBC ciphertext, PKCS7 padded ][ 16-byte HMAC-MD5 tag ]
```

No IV, no version byte, no length, no sequence number inside the encrypted unit —
Mercury's outer layer frames all of that.

> This is a **byte-exact match** with our Rust server at
> [`crates/mercury/src/encryption/`](../../../crates/mercury/src/encryption/mod.rs).
> The Rust side is in fact *more* secure: it verifies the tag in constant time
> via the `subtle` crate, where the client uses a `memcmp`-style compare.

### Why the v2 hook lands one layer up

The CryptoPP filter objects are **stack-allocated and rebuilt per call** behind
an **SEH frame**, so inline-patching `filterOut` / `filterIn` directly is
fragile (the stack layout and SEH unwinding make a stable detour hard to place).

**Recommended v2 hook:** `Mercury::Channel::send` @ `0x01576F90` — a vtable hook
on the heap-allocated channel object, one layer above the per-call filter
construction. This is the same `Channel::send` documented in
[mercury-protocol-internals.md](mercury-protocol-internals.md).

### Proposed v2 layout

```text
[ 0x02 version ][ 16-byte random IV ][ ciphertext ][ 16-byte truncated HMAC-SHA256 over IV||ciphertext ]
```

with **HKDF-SHA256** splitting separate encryption and MAC keys from the session
key.

---

## 6. Suggested debugger confirmations (x64dbg)

| Breakpoint | Address | What it proves |
|---|---|---|
| curl URL is `http://` | `0x013A96E0` | login transport is unencrypted and rewritable at `curl_easy_setopt` (watch `arg2 == 0x2712`) |
| plaintext password present | `0x00DDED60` | dump `[ServerConnection + 0x3C]` → plaintext is available at the `LoginReplyHandler` ctor for the swap |
| no 2nd SHA-1 path | 2nd `logOnBegin` (string near `0x019CF248`) | the second login variant does not separately hash |
| right v2 layer | `0x01576F90` | `Channel::send` pre-encryption buffer is the correct hook point for the v2 framing |

---

## Implementation impact

- **Phase 1 (auth TLS):** hook `curl_easy_setopt` (`0x013A96E0`), rewrite
  `CURLOPT_URL` to loopback, run a rustls proxy in the shim. Do **not** reuse
  curl's OpenSSL 0.9.8g.
- **Phase 2 (password):** hook `LoginReplyHandler` ctor (`0x00DDED60`), swap the
  plaintext at `ServerConnection + 0x3C` for the hashed argument — only valid
  once Phase 1 TLS is in place. Verify the second `logOnBegin` variant.
- **Phase 3 (Mercury v2):** vtable-hook `Mercury::Channel::send` (`0x01576F90`),
  not the CryptoPP filters; emit the version-byte-gated `[version][IV][ciphertext][HMAC]`
  layout with HKDF-split keys and a per-packet random IV.
- **Anti-tamper:** none — no neutralization step required before any of the above.

See the ADR at
[../../architecture/encryption-modernization.md](../../architecture/encryption-modernization.md)
for the full design rationale, alternatives considered, and the transition plan.
