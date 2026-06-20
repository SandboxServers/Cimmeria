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

### Phase 0 — shared hook foundation (**delivered by #504**)

Provide the **inline / IAT / vtable hooking** layer all client patching needs.

**Status update (2026-06-20):** [#504](https://github.com/SandboxServers/Cimmeria/pull/504)
(`cimmeria-client-telemetry` Phases 2–5) **built and shipped this layer** — it is
no longer from-scratch work. #504 provides the injection vehicle
(`crates/launcher/src/inject.rs::inject_dll`, LoadLibraryW-in-target) plus three
hook techniques in `crates/client-telemetry/src/hooks/`: `inline_hooks.rs`
(trampoline detours), `iat_hooks.rs` (IAT-slot replace), and `vtable_hooks.rs`
(vtable swap), all i686-CI-tested. They map 1:1 to the client patches below:
inline → `curl_easy_setopt @ 0x013A96E0` (Phase 1) and `LoginReplyHandler ctor @
0x00DDED60` (Phase 2); vtable → `Mercury::Channel::send @ 0x01576F90` (Phase 3).

So Phase 0 is re-scoped to **extracting #504's hook primitives into a reusable
API** (a behaviour-preserving refactor exposing `place_trampoline` /
`replace_iat_slot` / `swap_vtable_slot`) that the client patch crates call — not
a new from-scratch crate. This resolves the original "new crate vs. extend
client-telemetry" open decision in favour of **extending client-telemetry**.

### Phase 1 — auth TLS

- **Server side:** terminate TLS with **tokio-rustls**, with **arc-swap**-based
  hot certificate reload (rotate the cert without dropping the listener). **Cert
  hot-reload is implemented:** a background mtime watcher
  (`crates/services/src/auth/cert_watcher.rs`) polls the cert/key files and
  atomically swaps the live `rustls::ServerConfig` via `TlsCertStore::reload`
  when either file changes, so an operator's cert rotation (e.g. a Let's Encrypt
  renewal) is picked up without a server restart. The poll interval is
  `auth_tls_reload_interval_secs` (env `AUTH_TLS_RELOAD_INTERVAL_SECS`, default
  30s; `0` disables the watcher). A mid-write/corrupt PEM on a tick is logged and
  retried on the next tick — the swap only happens after a successful rebuild, so
  a botched rotation never takes the listener down.
- **Client side:** hook `curl_easy_setopt` (`0x013A96E0`), intercept
  `CURLOPT_URL` (`0x2712`), and rewrite the URL to a **localhost loopback**. A
  **local rustls proxy inside the injected shim** then re-originates the request
  as modern TLS upstream.

*Rationale (the key non-obvious decision):* login is **libcurl**, and curl's
bundled **OpenSSL 0.9.8g** cannot complete a modern TLS handshake. So we do
**not** try to make curl speak TLS. curl talks plain HTTP to `127.0.0.1`; the
shim's rustls proxy does the real TLS. This sidesteps the dead OpenSSL entirely.

### Phase 2 — argon2id password hashing

**Status (server half): Implemented.** The server-side storage, verification,
on-login migration, and the plaintext-over-TLS gate are in
`crates/services/src/auth/credentials.rs`. The client-side plaintext patch
(below) is still pending.

- **Schema (implemented):** a **dual-column** scheme — the legacy SHA-1
  `account.password` column (now NULLable) retained alongside a new
  `account.password_hash_v2` (argon2id PHC string) and a `account.password_algo`
  selector (`1`=sha1_legacy, `2`=argon2id). Edited directly in the `db/sgw/`
  schema + seed — **not** in `db/scripts/` (per project convention: never
  hand-write migration scripts; edit the seed). Seed accounts start at algo 1.
- **Migration (implemented): opportunistic, on-login.** When an account
  authenticates with a **plaintext** password and only the legacy hash exists,
  the server recomputes the client-side SHA-1, verifies it against the stored
  hash, then computes and stores the argon2id hash, flips `password_algo` to 2,
  and NULLs the legacy column — all in the same login. No mass re-hash, no forced
  reset. Migration failure is logged but never fails an already-verified login.
- **Plaintext is TLS-gated (implemented):** a request is classified as legacy
  hash (40-char hex, allowed on either listener) or plaintext (anything else).
  Plaintext is **only** accepted on the TLS listener — a marker inserted by a
  middleware layered solely onto the HTTPS Router clone. Plaintext over plain
  HTTP is rejected. argon2id params are explicit OWASP: 64 MiB / 3 iterations /
  1 lane, random per-hash salt.
- **Client side (pending):** patch the `LoginReplyHandler` ctor (`0x00DDED60`) to
  send the **plaintext** password (read from `ServerConnection + 0x3C`) over the
  now-TLS-protected channel, in place of the client SHA-1 hash. The server does
  the modern hashing.

*Rationale:* sending plaintext is only acceptable **because Phase 1 wrapped the
transport in TLS**. Server-side argon2id is the actual defense; the client can no
longer dictate the hash. Dual-column + opportunistic migration avoids a flag day.

### Phase 3 — Mercury v2 packet crypto

**Status (server v2 wiring): Implemented; default v1; per-client negotiation
pending the client patch.** The v2 crypto primitive (HKDF-split keys, per-packet
random IV, truncated HMAC-SHA256, version-byte downgrade defense) and the server
wiring that selects a version per session both exist. A session is pinned to one
wire version at login (`MercuryEncryption::from_session_key_versioned`), and that
version is applied consistently for both directions and every handshake/outbound
builder. Selection is **server-wide**, sourced from
`ServerConfig::mercury_encryption_version` (env `MERCURY_ENCRYPTION_VERSION`),
**defaulting to `1`**: the stock client only understands v1, so producing v2
frames by default would break every unpatched connection. A byte-exact
regression guard pins the default-v1 handshake output. There is **no per-client
negotiation yet** — every session uses the configured version; negotiation
arrives with the client patch that teaches `SGW.exe` to speak v2. Until then v2
is selectable only for a patched client or a test harness.

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

### Phase 3 — Mercury v2 session-key rotation

**Status (server side): Implemented, v2-gated; client rotation handling and the
production scheduler hookup pending the client patch.** Rotation is a
server-initiated, periodic refresh of the v2 session key that bounds how much
ciphertext is ever produced under one key and gives forward secrecy across each
rotation boundary.

- **Gated to v2 only.** The stock v1 client has no code path to receive a
  rotation control message, so rotation is hard-gated:
  `rotation_enabled(version, cadence_secs)` returns true **only** for v2 with a
  non-zero cadence. A v1 session never arms a rotation and never emits a
  `RotateSessionKey` message — today's clients are byte-identical to the
  pre-rotation behavior. With the default v1 server-wide version, the rotation
  cadence setting is inert.
- **Cadence:** `ServerConfig::mercury_key_rotation_secs`, default `3600`
  (one hour); `0` disables rotation.
- **Wire message:** a `RotateSessionKey` Mercury control message
  (`MsgId::RotateSessionKey = 9`) whose body is the new 32-byte key **encrypted
  under the current session context**. The raw key never appears in the clear;
  it rides inside the existing v2 envelope (random IV, HKDF-split keys,
  encrypt-then-MAC), so it is confidential and integrity-protected.
- **Switch semantics (server → peer):**
  1. **Arm.** Server mints a fresh CSPRNG key, builds the `RotateSessionKey`
     payload (encrypted under the *current* key), and stages the new context as
     pending — but keeps encrypting outbound under the **old** key so the
     rotation message is decryptable by the peer.
  2. **Commit outbound.** After the rotation message is on the wire, the server
     promotes pending → current; every subsequent outbound packet uses the new
     key.
  3. **Peer applies on receipt.** The peer decrypts the message under the key it
     still holds, recovers the new key, and rebuilds both directions from it.
  4. **Dual-key inbound window.** Between arming and observing the first inbound
     packet that decrypts under the new key, the server accepts **both** keys
     (new first, old as fallback) so an in-flight old-key packet that crosses
     the switch boundary still decrypts. The window closes on the first
     new-key success.
  A dropped `RotateSessionKey` message is retransmitted by the transport layer
  (byte-identically, under the old key) until acked, so the data plane is
  loss-tolerant across the switch.
- **Narrowed scope:** the rotation **primitives and switch state machine** live
  in `cimmeria-mercury` (`encryption::rotation`) and are proven end-to-end
  through the **loopback test harness** simulating a cooperating v2 peer
  (`test_harness::tests::rotation`), including a rotate-under-load test. **No
  real client is exercised** — the stock client speaks v1 and is never sent a
  rotation message. The production hookup that drives a per-session rotation
  scheduler against the live BaseApp recv/send loop, and the client-side
  handling of `RotateSessionKey`, both land with the client patch; deferring
  them avoids adding race surface to the live session path with no validation
  target.

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
