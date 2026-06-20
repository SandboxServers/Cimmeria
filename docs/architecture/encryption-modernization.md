# Encryption modernization — auth TLS, password hashing, Mercury v2

**Status:** Proposed
**Confidence:** RE targets High (binary-validated); client-patch engineering Medium (per-hook runtime confirmation pending)
**Issue:** [#434](https://github.com/SandboxServers/Cimmeria/issues/434) (encryption modernization, as rewritten)

This ADR records the design that consumes the binary-validated targets in
[../reverse-engineering/findings/auth-and-crypto-modernization-targets.md](../reverse-engineering/findings/auth-and-crypto-modernization-targets.md).
Read that finding first — every address and hook point cited here is sourced
there.

## Context

Cimmeria's current cryptographic posture inherits three weaknesses from the 2009
Stargate Worlds stack, each of which a modern emulator should not ship with:

1. **Plain-HTTP SOAP login.** The auth login is a SOAP POST over **plain HTTP**
   via statically-linked libcurl 7.17.0. Credentials cross the network
   unencrypted. (gSOAP is XML-serialization only — it does not own the socket;
   any prior note implying a gSOAP TCP hook is wrong. See finding §1.)
2. **Unsalted SHA-1 passwords.** The client hashes the password inline with
   SHA-1 (uppercase hex, no salt) before sending it. SHA-1 of an unsalted
   password is trivially rainbow-table-able and offers no server-side defense.
   (Finding §3.)
3. **Zero-IV AES-256-CBC + HMAC-MD5, shared-key Mercury.** Packet encryption
   reuses a single 16-byte **zero IV** for every packet, derives no keys (the
   session key is the AES key *and* the HMAC key verbatim), MACs with **HMAC-MD5**,
   and verifies in non-constant time. (Finding §5.) The zero-IV reuse is the
   most serious flaw: identical plaintext blocks produce identical ciphertext,
   leaking structure.

**The hard constraint:** the 2009 client is the only client. There is no source,
and its bundled OpenSSL is **0.9.8g (2007)** — too old to speak modern TLS. Every
client-side change must therefore be a **binary patch via injection + hooking**.

- **Non-goal:** vanilla-client compatibility. We are explicitly patching the
  client; an unpatched client is not a supported configuration for the modernized
  crypto paths.
- **Enabling fact:** the binary has **no anti-debug and no anti-tamper** (finding
  §4) — injection and inline/IAT/vtable hooking are all safe with no
  neutralization step.

## Decision

A four-phase rollout, plus one deliberate non-change.

### Phase 0 — shared hook foundation

Build a **shared inline / IAT / vtable hooking crate** as the foundation for all
client patching.

*Rationale:* the existing `cimmeria-client-telemetry` cdylib already has the
injection vehicle (it loads into the client) but **no general hook layer** —
nothing to place an inline detour, swap an IAT slot, or replace a vtable entry.
Phases 1–3 all need that layer, so it is built once, first.

### Phase 1 — auth TLS

- **Server side:** terminate TLS with **tokio-rustls**, with **arc-swap**-based
  hot certificate reload (rotate the cert without dropping the listener).
- **Client side:** hook `curl_easy_setopt` (`0x013A96E0`), intercept
  `CURLOPT_URL` (`0x2712`), and rewrite the URL to a **localhost loopback**. A
  **local rustls proxy inside the injected shim** then re-originates the request
  as modern TLS upstream.

*Rationale (the key non-obvious decision):* login is **libcurl**, and curl's
bundled **OpenSSL 0.9.8g** cannot complete a modern TLS handshake. So we do
**not** try to make curl speak TLS. curl talks plain HTTP to `127.0.0.1`; the
shim's rustls proxy does the real TLS. This sidesteps the dead OpenSSL entirely.

### Phase 2 — argon2id password hashing

- **Schema:** a **dual-column** scheme (legacy SHA-1 column retained alongside a
  new argon2id column), edited directly in the `db/sgw/` schema and the
  `db/resources/` seed — **not** in `db/scripts/` (per project convention: never
  hand-write migration scripts; edit the seed).
- **Migration:** **opportunistic, on-login.** When a user authenticates and only
  the legacy hash exists, verify against it, then compute and store the argon2id
  hash transparently. No mass re-hash, no forced reset.
- **Client side:** patch the `LoginReplyHandler` ctor (`0x00DDED60`) to send the
  **plaintext** password (read from `ServerConnection + 0x3C`) over the
  now-TLS-protected channel, in place of the client SHA-1 hash. The server does
  the modern hashing.

*Rationale:* sending plaintext is only acceptable **because Phase 1 wrapped the
transport in TLS**. Server-side argon2id is the actual defense; the client can no
longer dictate the hash. Dual-column + opportunistic migration avoids a flag day.

### Phase 3 — Mercury v2 packet crypto

- **Wire format:** version-byte-gated
  `[ version ][ IV ][ ciphertext ][ HMAC ]`, with a **random per-packet IV**,
  **HKDF-SHA256** splitting separate encryption and MAC keys from the session
  key, and **HMAC-SHA256** (truncated to 16 bytes) over `IV || ciphertext`.
- **Client side:** patch at **`Mercury::Channel::send` (`0x01576F90`)** via a
  vtable hook on the heap channel object — **not** the CryptoPP `filterOut` /
  `filterIn` functions.

*Rationale:* the CryptoPP filter objects are stack-allocated and rebuilt per call
behind an SEH frame, which makes a stable inline patch on the filters fragile.
Hooking one layer up at `Channel::send` gives a clean, heap-resident vtable
target. The version byte lets v1 and v2 coexist during transition.

### Non-change — leave the session-key handshake as-is

The Mercury session key continues to be exchanged as it is today.

*Rationale:* once Phase 1 is in place, the handshake rides inside **HTTPS**, so
the key is not exposed in transit. Combined with the Phase 3 **per-packet random
IV** and periodic **key rotation** (which gives forward secrecy across rotation
boundaries), a bespoke key-agreement step adds complexity without closing a real
gap.

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| **gSOAP TCP hook for login interception** | Login transport is **libcurl, not gSOAP**. gSOAP only serializes XML; it never owns the socket (finding §1). The hook would never fire on the real I/O path. |
| **Reuse curl's bundled OpenSSL 0.9.8g for TLS** | OpenSSL 0.9.8g (2007) predates TLS 1.2 and SNI and cannot handshake with a modern endpoint. This is exactly why Phase 1 uses a loopback + rustls proxy instead. |
| **Inline-patch the `PacketEncrypter` filters directly** | The CryptoPP filters are stack-allocated, rebuilt per call, and wrapped in an SEH frame — an inline detour there is fragile. The `Channel::send` vtable hook is stable. |
| **Diffie-Hellman key agreement on `baseAppLogin`** | Redundant. HTTPS already protects the session-key exchange, the per-packet random IV removes the zero-IV reuse problem, and rotation provides forward secrecy. DH adds a handshake round-trip and code surface for no additional protection given the rest of the design. |

## Consequences

- **Transition windows.** Two overlapping dual-stack windows exist:
  - **Auth:** the server accepts **both plain HTTP and HTTPS** until all patched
    clients are confirmed on TLS, then HTTP is retired.
  - **Mercury:** the server speaks **both v1 and v2** (selected by the version
    byte) until v1 is retired.
- **v1 → v2 downgrade-gating risk.** While both Mercury versions are live, an
  attacker (or a stale client) could force a downgrade to v1's weaker scheme. The
  version gate must be **monotonic per session** — once a session negotiates v2
  it must refuse v1 frames — and v1 acceptance must be **removed on a schedule**,
  not left open indefinitely.
- **AV / injection considerations.** DLL injection into a game client is a
  pattern antivirus heuristics flag. Expect to document the shim, possibly sign
  it, and account for EDR false positives in the launcher/runbook. (The client
  itself has no anti-tamper to fight — the concern is host AV, not the binary.)
- **Cert-pin rotation.** If the client shim pins the server certificate, rotation
  must be coordinated: the **arc-swap** server reload and the shim's pin set have
  to move together, or a rotated cert locks out patched clients. Plan a pin
  rollover window (accept old + new pin) mirroring the transport dual-stack
  windows above.
- **Schema.** Dual-column auth lands in `db/sgw/` + the `db/resources/` seed; the
  legacy SHA-1 column is retained until opportunistic migration has covered the
  active player base, then it can be dropped.

## Confidence Level

- **RE targets: HIGH.** Every address, call chain, CURLOPT code, struct offset,
  and crypto-scheme detail is binary-validated against `SGW.exe` (image base
  `0x00400000`, ASLR disabled). The v1 Mercury scheme is a byte-exact match with
  `crates/mercury/src/encryption.rs`.
- **Client-patch engineering: MEDIUM.** Each hook (curl URL rewrite, password
  swap, `Channel::send` vtable replacement) is *located* with high confidence but
  not yet *built and runtime-confirmed*. Confidence promotes to HIGH per hook as
  the x64dbg confirmations in finding §6 are executed and the shim is exercised
  against a live client. The second `logOnBegin` variant (no separate SHA-1 path)
  is an explicit open verification item for Phase 2.

## See also

- [../reverse-engineering/findings/auth-and-crypto-modernization-targets.md](../reverse-engineering/findings/auth-and-crypto-modernization-targets.md)
  — the binary-validated targets and exact addresses this design consumes.
- [../reverse-engineering/findings/mercury-protocol-internals.md](../reverse-engineering/findings/mercury-protocol-internals.md)
  — `Mercury::Channel::send` context (the Phase 3 hook point).
- [`crates/mercury/src/encryption.rs`](../../crates/mercury/src/encryption.rs)
  — the current (v1-equivalent) server encryption implementation.
- Issue **#434**.
