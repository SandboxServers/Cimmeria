# CAT-A — Auth / Session / Character lifecycle

**Overall trust posture**: The auth-handshake path applies several correct
server-authority disciplines: the inbound baseAppLogin's `account_id` field is
parsed-and-discarded (the trusted account_id comes from the ticket map);
deleteCharacter and requestCharacterVisuals both filter by `account_id` so
players cannot mutate or enumerate other accounts' characters; PlayCharacter's
spawn position is read server-side from `sgw_player` (not from the wire);
CreateCharacter's archetype/alignment/gender/bodyset/starting-position are all
derived server-side from a static `chardef_lookup` table keyed by the
client-supplied `CharDefId`; starting abilities/items come from
`resources.char_creation_*` tables, not from the client; and the GM-relevant
`access_level` for in-world dispatch lives on the server-side
`ConnectedClientState` row sourced from the auth ticket (which sources from
the `account.accesslevel` column). The login itself, however, is wide open in
several places that matter outside a closed LAN deployment:

1. The HTTP SOAP auth (Phases 1 + 2) runs **plaintext HTTP** — credentials
   (as SHA-1 hex, which the server accepts as the password), the session_key,
   and the ticket all traverse the wire unencrypted. Anyone on-path can replay
   either the SOAP request or the Phase 3 UDP packet.
2. The Phase 3 ticket has no source-IP binding, no nonce, and no per-attempt
   challenge — possession of the ticket = full session.
3. AES-256-CBC encryption uses a hard-coded all-zero IV reused across every
   packet for the lifetime of the channel. Identical plaintext blocks produce
   identical ciphertext blocks, leaking message structure to a passive
   observer.
4. There is no per-sequence inbound dedup on encrypted game packets — the
   per-channel `Channel::receive_packet` window is wired only on the outbound
   ACK path; the inbound dispatch in `connect_loop::encrypted` runs decrypt →
   parse → dispatch with no replay check, so any captured ciphertext can be
   re-injected and re-dispatched.
5. The Phase 3 handler does not consult `TICKET_TTL` — only a 10s reaper
   thread enforces it, so a ticket can be consumed up to ~10s past its 30s
   nominal expiry.
6. No rate limiting / lockout on Phase 1 — brute-force-by-SHA1 is unimpeded.
7. Password equality uses `String::!=` (variable time) — a millisecond-scale
   timing oracle, exploitable across the unencrypted SOAP channel.
8. CreateCharacter has no profanity / reserved-word filter; names like
   `Admin`, `GM_*`, etc. parse through unchanged.

The CharDef → starting-state derivation, account-id ownership filters, and
ConnectedClientState-rooted access_level are the load-bearing correctness
properties — none of those are wire-trust violations. Everything below is
either a wire-trust gap or an encryption/replay/credential-handling gap, not
a CharDef/visuals/ownership gap.

---

### CAT-A-01 — Phase 1/2 auth SOAP runs plaintext HTTP; credentials, session_key, and ticket all sniffable

**Severity**: Critical
**Class**: missing transport security
**Wire surface**: `POST /SGWLogin/UserAuth`, `POST /SGWLogin/ServerSelection`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The auth service binds an axum `TcpListener` and serves the SOAP login
endpoints over plain HTTP (no `rustls`, `TlsAcceptor`, or any TLS termination
in front). The client's `password` field is a 40-char hex SHA-1 of the
plaintext password and is accepted directly by `validate_credentials` —
possession of the hash is functionally equivalent to possession of the
password (it's the only thing the wire protocol needs). Phase 2's response
includes the `SessionKey` (64-char hex AES-256 key) and `Ticket` (20-char
hex) in plaintext XML. Anyone with passive network capture between client
and server captures: (a) a reusable credential, (b) the AES key for the
client's entire in-world session, and (c) the ticket needed to claim the
session via Phase 3.

**Evidence**
- Ghidra: `0x019d0030` `ServerConnection::authenticate: Unexpected key! (%s, wanted %s)` — the client emits `authenticate` carrying the SessionKey it got from Phase 2, and the server's encrypted-channel logic relies on that exact key on both sides. SessionKey is XML-encoded in the Phase 2 SOAP response (`0x01b26028` `SessionKey`).
- Client behavioral log: n/a — the protocol is HTTP-by-design per `crates/services/src/auth/handlers.rs` (no TLS in scope).
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/auth/service.rs:135` (`TcpListener::bind`), `crates/services/src/auth/handlers.rs:172-181` (XML response with cleartext SessionKey/Ticket).

**Attack scenario**
1. Attacker is on-path between the client and the auth server (open Wi-Fi,
   ISP, compromised router).
2. Captures the Phase 1 POST body — extracts `Password=<40 hex chars>` and
   `AccountName=<user>`.
3. Captures the Phase 2 response — extracts `SessionKey=<64 hex chars>` and
   `Ticket=<20 hex chars>`.
4. Either: (a) reuses the captured `Password` hash anytime to log in as the
   user (the wire protocol accepts the hash; no challenge-response), or (b)
   races the legitimate client to the Phase 3 UDP endpoint, claims the
   ticket, and runs the in-world session under the captured session_key.
5. Observable effect on the server: the legitimate client either fails Phase
   3 (ticket already consumed) or is duplicate-evicted by the attacker's
   later login.

**Suggested remediation (one line)**
Terminate Phases 1/2 behind TLS (axum + rustls or a reverse proxy); migrate the password column off raw SHA-1 to a salted PBKDF2/argon2 with a challenge-response wire protocol so the hash is not itself a reusable credential.

**Would benefit from x64dbg trace?**
No — the protocol is unambiguous from Ghidra strings and the Rust handler. A
network capture in a real session would confirm but adds nothing beyond what
the code states.

---

### CAT-A-02 — AES-CBC encryption uses a hard-coded all-zero IV reused for every packet

**Severity**: High
**Class**: cryptographic misuse — IV reuse
**Wire surface**: every encrypted Mercury packet (post-Phase 3)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`MercuryEncryption::from_session_key` constructs the per-session encryption
context with `iv: [0u8; 16]` and never advances it. Every packet sent on the
channel encrypts under `AES-256-CBC(key, iv=0, padded_plaintext)`. With a
fixed IV, two packets whose first plaintext block is identical produce
identical first ciphertext blocks — and because the SGW wire protocol begins
many message families with stable preambles (msg_id + WORD_LENGTH +
entity_id, repeating heartbeat bundles, fixed AVATAR_UPDATE_EXPLICIT layout,
etc.), a passive observer can recover substantial plaintext structure
without breaking AES at all. Combined with CAT-A-01 (key is sniffable
anyway), this is a defense-in-depth gap rather than the primary break, but
the design is wrong: AES-CBC requires a unique IV per encryption.

**Evidence**
- Ghidra: n/a — encryption is server-side; the matching C++ behaviour is referenced in the Rust source comment ("matches the C++ `EncryptionFilter::setKey()`").
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/mercury/src/encryption.rs:78-84` (`from_session_key` hard-codes the zero IV).

**Attack scenario**
1. Attacker passively captures the encrypted bundle stream from any client.
2. Identifies recurring 16-byte ciphertext prefixes — these correspond to
   message-family preambles whose plaintext is dictionary-known.
3. Cross-references against the (also-sniffable) AES key from CAT-A-01 to
   fully decrypt; or, even without the key, performs pattern-matching to
   classify message families and time them against game events.
4. Observable effect on the server: no direct effect, but information
   disclosure compounds with any other gap (replay, ticket capture, etc.).

**Suggested remediation (one line)**
Generate a fresh random 16-byte IV per packet and prepend it to the
ciphertext (then HMAC over IV‖ciphertext); the per-packet IV cost is
trivial vs. the kept-forever key-reuse leak.

**Would benefit from x64dbg trace?**
No — the misuse is on the server side; behaviour is unambiguous from the Rust source.

---

### CAT-A-03 — No inbound packet replay / dedup on the encrypted game-packet path

**Severity**: High
**Class**: missing replay protection
**Wire surface**: every encrypted Mercury packet from a connected client
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The per-channel `Channel::receive_packet` implementation in
`crates/mercury/src/channel/mod.rs:412` does maintain an RX window with
duplicate-drop semantics (`if self.rx_window[offset].is_none() { … }`), but
**that method is never called for inbound encrypted game packets** — a
workspace-wide grep for `receive_packet` returns zero hits in
`crates/services`. The actual game-packet path in
`connect_loop/encrypted/mod.rs::handle_encrypted_datagram` only (i) decrypts,
(ii) parses, (iii) queues an ACK for the client's reliable sequence number,
and (iv) dispatches the bundle. There is no check that the client's
`pkt.seq_id` has not already been processed; a captured ciphertext blob can
be re-injected verbatim by any on-path attacker and the server will
re-dispatch every message in the bundle.

**Evidence**
- Ghidra: n/a — the Mercury wire format and 28-bit sequence space are described in `docs/drafts/spec/mercury-wire-format.md` and confirmed by `crates/mercury/src/packet`. The server's omission is on the receive side.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/connect_loop/encrypted/mod.rs:99-105` (queues ACK without dedup), `crates/mercury/src/channel/mod.rs:412-468` (the unwired `receive_packet` dedup window).

**Attack scenario**
1. Attacker captures one ciphertext datagram from any client (e.g. a UseItem
   bundle that consumes a stack item, or a PurchaseItems, or a GM-shout from
   a moderator).
2. Re-injects the same datagram against the same UDP endpoint, optionally
   multiple times.
3. Server decrypts (succeeds — HMAC valid), parses, dispatches every
   contained message a second time.
4. Observable effect: depending on the captured message family, double
   item-consume (CAT-D class), double purchase, repeated chat broadcast,
   repeated GM-action, repeated character delete, etc.

**Suggested remediation (one line)**
Plumb every inbound packet through `Channel::receive_packet` (or an
equivalent per-session received-sequence set) and drop datagrams whose
`seq_id` has already been processed; the dedup window is already
implemented, it just isn't called on the receive path.

**Would benefit from x64dbg trace?**
No — the omission is observable purely in the server source. A
proof-of-concept replay using the existing `cimmeria-wireclient` chaos
harness would confirm but the code-level evidence is already sufficient.

---

### CAT-A-04 — Phase 3 baseAppLogin does not enforce TICKET_TTL — ~10s reaper lag is the only gate

**Severity**: Medium
**Class**: stale-credential acceptance
**Wire surface**: Phase 3 UDP `baseAppLogin` (msg_id 0x00, flags 0x41)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`PendingLogin` carries a `created: Instant` and the auth service defines
`TICKET_TTL = 30s`, but the actual enforcement only happens in the 10-second
background reaper (`crates/services/src/auth/service.rs:160`). The Phase 3
consumer `handle_login` performs `pending_logins.lock().remove(ticket)`
without checking `login.created.elapsed() < TICKET_TTL`. Consequence: a
ticket older than 30s but younger than the reaper's next sweep is still
consumable — up to ~40s old in the worst case. This widens the attacker's
race window after a CAT-A-01-style ticket capture: the attacker has not 30s
but as much as 40s to win the race to Phase 3.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/login/mod.rs:44-57` (`pending_logins.remove(ticket)` with no elapsed check), `crates/services/src/auth/service.rs:157-162` (reaper is the only TTL enforcer).

**Attack scenario**
1. Attacker captures a ticket from a CAT-A-01 SOAP sniff at T=0.
2. Legitimate client is slow to Phase 3 (e.g. firewall, DNS, retry) — at
   T=31s no Phase 3 has happened.
3. Attacker's Phase 3 packet at T=35s arrives. Reaper last ran at T=30s and
   next runs at T=40s. The ticket is still in the map.
4. `handle_login` succeeds; attacker now owns the session for the
   legitimate user.

**Suggested remediation (one line)**
Add `if login.created.elapsed() >= TICKET_TTL { return Ok(()); }` in
`handle_login` right after the `remove()`, before any session setup.

**Would benefit from x64dbg trace?**
No — purely a server-side check omission.

---

### CAT-A-05 — Phase 3 ticket has no source-IP binding; capture-from-anywhere = full session

**Severity**: High
**Class**: missing session binding
**Wire surface**: Phase 3 UDP `baseAppLogin`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (needs network capture to confirm)

**Trust violation**
`PendingLogin` stores only `(account_id, access_level, ticket, session_key,
created)`. The client_ip observed at Phase 2 (`addr.ip()` in
`handle_server_selection`) is logged but discarded — it is not stored in the
PendingLogin record, and `handle_login` does not compare the inbound UDP
source address against an expected IP. Combined with CAT-A-01 (plaintext
ticket on the wire), this means any attacker who observes a ticket from any
network location can replay it from any other source address.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/auth/mod.rs:99-110` (PendingLogin struct, no source_ip field), `crates/services/src/auth/handlers.rs:251-262` (Phase 2 stores PendingLogin without binding `addr`).

**Attack scenario**
1. Attacker captures Phase 2 response from an off-path vantage (e.g. mirror
   port at an ISP, BGP hijack of a tiny prefix, or a captured PCAP shared
   among adversaries).
2. Sends Phase 3 from a completely different network (different country,
   different ASN).
3. Server accepts — no IP comparison — and creates the session bound to the
   attacker's UDP address. The legitimate client's later Phase 3 either
   fails ("ticket already consumed") or duplicate-evicts the attacker
   AFTER the attacker has had time to perform an action.

**Suggested remediation (one line)**
Store `peer_ip` in `PendingLogin` at Phase 2 and reject `handle_login` when
the inbound UDP source IP does not match (with a strict-vs-permissive knob
for NAT'd clients if needed).

**Would benefit from x64dbg trace?**
No — server-side check omission. A two-host PoC is the natural test.

---

### CAT-A-06 — No rate-limiting / lockout on Phase 1 credential check — credential stuffing is unimpeded

**Severity**: Medium
**Class**: missing brute-force defense
**Wire surface**: `POST /SGWLogin/UserAuth`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_user_auth` runs through `validate_credentials` synchronously on each
request and emits a `LoginEvent` audit record on failure, but there is no
per-IP, per-account, or global throttle. An attacker can hammer the SOAP
endpoint at full server CPU and never trigger a slowdown or account lock.
Combined with the wire protocol's stable SHA-1 hash format (CAT-A-01), an
offline rainbow table / leaked-credential list is fully effective.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/auth/handlers.rs:117-144` (credential check with no throttle); workspace-wide grep for `rate_limit|lockout` in `crates/services` returns no matches.

**Attack scenario**
1. Attacker scripts POST requests to `/SGWLogin/UserAuth` with a known
   account_name + a wordlist of SHA-1 hashes.
2. Server processes each request at full speed; no failure threshold pauses
   or blocks the attacker.
3. On hit, the success returns a valid Phase 1 SID — attacker proceeds to
   Phase 2 + Phase 3 normally.

**Suggested remediation (one line)**
Add a per-(IP, account_name) sliding-window failure counter that backs off
exponentially and triggers an account lock after N failures within a window;
emit `LoginEvent` with a `rate_limited` outcome.

**Would benefit from x64dbg trace?**
No.

---

### CAT-A-07 — Password comparison uses variable-time `String::!=` — timing oracle

**Severity**: Low
**Class**: side-channel — timing attack
**Wire surface**: `POST /SGWLogin/UserAuth`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (timing leak; would require careful measurement, especially over a plaintext HTTP path)

**Trust violation**
`validate_credentials` compares the stored password hash against the
client-supplied hash using `row.password.to_uppercase() != client_password_hash.to_uppercase()` —
this is `std::String`'s `PartialEq`, which short-circuits on the first
mismatching byte. The comparison happens AFTER the DB round-trip (which
dominates the wall-clock), but the difference between "first byte mismatch"
and "37th byte mismatch" is measurable over many samples. For a 40-char
SHA-1 hex string, this leaks the password hash one nibble at a time.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/auth/handlers.rs:481`.

**Attack scenario**
1. Attacker knows the target account_name.
2. Performs many calibrated Phase 1 requests with crafted password hashes,
   binary-search-style measuring server response time per prefix.
3. Over ~40 × 16 ≈ 640 measurements (in practice many more, to denoise),
   recovers the stored hash one nibble at a time.
4. Recovered hash is itself a reusable credential per CAT-A-01.

**Suggested remediation (one line)**
Use `subtle::ConstantTimeEq` (or the `constant_time_eq` crate) for the hash
comparison, and convert both sides to bytes before the compare.

**Would benefit from x64dbg trace?**
No — pure timing channel on the server.

---

### CAT-A-08 — Developer-mode bypass: any-credentials login with access_level=99 if DB is absent

**Severity**: High (if reachable in deployed env) / Low (if dev-only)
**Class**: configuration footgun — admin bypass
**Wire surface**: `POST /SGWLogin/UserAuth`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
When `state.developer_mode = true` AND `state.db = None`,
`handle_user_auth` short-circuits the credential check and returns
`(account_id=1, access_level=99)` for any submitted account_name/password
combination (subject only to format-validation: 40-char hex password, valid
account-name characters). The same flag also bypasses the protocol-digest
client-version check. Access_level 99 is the highest privilege the server
recognises — that account is full GM. The default for `developer_mode` is
`false`, but the dual condition (developer_mode AND no DB) is an easy
operator mistake: any deployment that forgets to wire the Postgres pool
turns into a wide-open admin server.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/auth/handlers.rs:107-145` (developer_mode protocol-digest bypass + no-DB credential bypass at access_level 99).

**Attack scenario**
1. Operator deploys with `developer_mode=true` (left over from local dev)
   and a misconfigured DATABASE_URL that fails the connection.
2. AuthService starts with `state.db = None`.
3. Any attacker hits `/SGWLogin/UserAuth` with any account_name + a valid
   40-hex password string and gets `(account_id=1, access_level=99)`.
4. Phase 2 + 3 proceed normally; the attacker is in-world as a full GM.

**Suggested remediation (one line)**
Refuse to start the AuthService when `developer_mode=true` and the DB pool
is `None` simultaneously — both conditions in production should be a fatal
startup error, not a silent open door. Failing that, log an
operator-visible WARN-level alert on every developer-mode credential
acceptance.

**Would benefit from x64dbg trace?**
No.

---

### CAT-A-09 — `Channel` per-session inbound dedup state exists but is never wired to the live game-packet path

**Severity**: Medium
**Class**: defense-in-depth missing, related to CAT-A-03
**Wire surface**: every encrypted Mercury packet
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`ConnectedClientState` carries `channel: Mutex<cimmeria_mercury::channel::Channel>`
(initialised at Phase 3 in `crates/services/src/base/login/mod.rs:186`),
which contains the full `rx_window` + `expected_rx_seq` machinery
described in spec §1.7. Today only the **outbound ACK side** is wired
(`channel.process_acks(...)` in `encrypted/mod.rs:117`). The inbound
`channel.receive_packet(...)` is never called — duplicates aren't dropped,
out-of-window packets aren't logged, and the carefully-built RX window
sits dead. This is the same root cause as CAT-A-03 but worth filing
separately because the fix is "wire the existing implementation up",
not "design replay protection from scratch".

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/mercury/src/channel/mod.rs:412-468` (the unused `receive_packet` dedup path), `crates/services/src/base/login/mod.rs:186` (Channel is created but never sees inbound packets).

**Attack scenario**
Same as CAT-A-03 — the per-session `Channel` already has the data
structure to detect a replayed `seq_id`; the bug is that the receive
loop bypasses it.

**Suggested remediation (one line)**
In `handle_encrypted_datagram`, before dispatching the bundle, call
`state.channel.lock().receive_packet(pkt.clone())?` and drop the bundle
if the call returns `Ok(None)` and the packet is reliable (i.e. it was
a duplicate that the RX window already saw, not a buffered-ahead one).

**Would benefit from x64dbg trace?**
No.

---

### CAT-A-10 — CreateCharacter has no profanity / reserved-word filter; "Admin", "Moderator", "GM_*" are creatable

**Severity**: Low
**Class**: missing impersonation defense
**Wire surface**: `Event_NetOut_CreateCharacter` (Account base method 0xC3)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`validate_character_name` enforces length (3–20), ASCII letters/digits/space/
hyphen/apostrophe only, no leading/trailing/consecutive whitespace. There is
no profanity or reserved-word filter. A normal user can create characters
named `Admin`, `Moderator`, `GM Sam`, `Customer Service`, etc. — these
appear in chat and on the wire as legitimate-looking entities and are an
impersonation / social-engineering vector even though the actual
`access_level` is still 0.

**Evidence**
- Ghidra: `0x019bbdb0` `Event_NetOut_CreateCharacter` — the client emits CreateCharacter carrying the Name WSTRING; client-side validation does not filter reserved names (the client passes whatever the UI text field contains).
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/character_create.rs:495-515` (validate_character_name).

**Attack scenario**
1. Attacker creates a character named `Admin` or `Moderator <ServerName>`.
2. The character appears in chat as if it were staff; the attacker uses it
   to phish other players ("I'm a GM, send me your password to verify").
3. Observable effect on the server: no direct game-state corruption, but a
   real-world impersonation channel.

**Suggested remediation (one line)**
Add a reserved-prefix/reserved-name list (`["admin", "moderator", "gm", "cs", "support", "system", "cimmeria", ...]`) and reject case-insensitively in `validate_character_name`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-A-11 — onClientVersion / versionInfoRequest does not enforce a server-side minimum client version

**Severity**: Low
**Class**: defense-in-depth — modded/old client tolerance
**Wire surface**: `Event_NetOut_versionInfoRequest`, `Event_NetOut_onClientVersion` (Account base method 0xC7)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`onClientVersion` (0xC7) is dispatched in `account_arms.rs:150-152` as
nothing more than `tracing::debug!(...)`. `versionInfoRequest` is handled by
`handle_version_info_request` which serves invalid-keys deltas but does NOT
compare a client-reported version to a minimum. The only version gate is
the Phase 1 SOAP `ProtocolDigest` MD5, which a modded client can hard-code.
After Phase 1, the server accepts any client version — including a modded
client that disables client-side bounds checks, disables UI lockouts on GM
buttons, etc.

**Evidence**
- Ghidra: client emits `Event_NetOut_onClientVersion` at the standard NetOut emit site; the payload carries a version blob that the server reads but does not validate against a floor.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/connect_loop/account_arms.rs:150-152` (no-op accept).

**Attack scenario**
1. Attacker modifies SGW.exe (or AtreaRL.dll wrapper) to disable client-side
   UI lockouts on debug/CHEAT-class messages (Goto, Spawn, SetGodMode, etc.).
2. Logs in normally — the modded client passes Phase 1 because the
   ProtocolDigest matches; passes onClientVersion because the server doesn't
   check.
3. Sends GM-class messages from a non-GM account — those are gated
   server-side per CAT-N, but defense-in-depth was supposed to start here.

**Suggested remediation (one line)**
Parse and enforce a minimum client version in `onClientVersion`; reject
clients below the floor with `LOGGED_OFF`.

**Would benefit from x64dbg trace?**
No — the server-side omission is the issue, not the client behaviour.

---

### CAT-A-12 — `restoreClientAck` (0x0B) is consumed silently; no validation that a restore was actually pending

**Severity**: Low
**Class**: protocol-state ambiguity
**Wire surface**: System message 0x0B (`restoreClientAck`, CONSTANT_LENGTH=4)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The wire framing code in `encrypted/mod.rs:500-501` correctly reads
`restoreClientAck` as CONSTANT_LENGTH=4 and advances the offset, but there
is no match arm for `0x0B` in the dispatcher — it falls through to the
`_` wildcard at `mod.rs:444` and is logged at trace. The 4 bytes of payload
are never inspected, and the server doesn't verify that the client is
acknowledging an actual outstanding `RESTORE_CLIENT` — a stale or
maliciously-injected `restoreClientAck` would be silently accepted. In the
current server this is dead code on both sides (no `RESTORE_CLIENT` is ever
sent), but the silent-accept means any future addition of a restore
sequence would have to remember to bolt validation on; today's behavior
silently drops the evidence.

**Evidence**
- Ghidra: the spec annotation in the Rust source cites `ghidra://SGW.exe@0x00dd8bc9` as the sole emitter (writes literal `i32 = 0`). The client always sends a zero-int payload.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/connect_loop/encrypted/mod.rs:444-446` (the wildcard arm consumes 0x0B without any state check).

**Attack scenario**
1. Attacker sends a `restoreClientAck` (4 bytes, all-zero) on any encrypted
   channel that they own.
2. Server silently consumes it.
3. Today's observable effect: none — no server logic is gated on
   restoreClientAck. **The finding is filed as a noise-class warning**: any
   future work that wires `RESTORE_CLIENT` (e.g. for snapshot-resume) must
   validate that an ack arrives only when one was solicited.

**Suggested remediation (one line)**
Either explicitly drop `0x0B` with a `warn!` if no restore was solicited,
or add a guarded state machine in `ConnectedClientState` (`pending_restore: Option<...>`) and validate against it.

**Would benefit from x64dbg trace?**
No.

---

### CAT-A-13 — Phase 3 ticket consumption is not atomic with eviction of an existing session for the same account

**Severity**: Low
**Class**: TOCTOU on session swap
**Wire surface**: Phase 3 UDP `baseAppLogin`
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (race-window narrow but extant)

**Trust violation**
`handle_login` performs three separate lock acquisitions on the `connected`
map: (1) scan-for-existing-session-by-account_id, (2) send LOGGED_OFF to the
old session, (3) insert the new session. Between steps 1 and 3 another
thread (e.g. a concurrent `handle_log_off` or the tick loop's
`destroy_client_entities`) can mutate the connected map. The eviction logic
also assumes the old session's `account_id` is still on the entry — it is,
but the lock is dropped twice. In normal operation the duplicate-eviction
flow works, but a hostile client can race: (a) initiate Phase 3 from one
network endpoint, (b) before the LOGGED_OFF send completes, fire another
Phase 3 from a different endpoint. The two new sessions can both register
because the second one's scan in step (1) doesn't see the first one (it's
still being set up). One of the sessions is then orphaned in the connected
map but with cancelled-tick-loop status.

**Evidence**
- Ghidra: n/a.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/login/mod.rs:67-113` (multi-step eviction with intervening lock drops).

**Attack scenario**
1. Attacker holds two valid (captured) tickets for the same account.
2. Fires Phase 3 from two source ports nearly simultaneously.
3. Both pass the "no existing session" check; both insert a connected
   state. One wins the entity_to_addr race; the other holds a zombie
   connected entry that the tick loop may or may not clean up.
4. Observable effect: account_id appears in `clients` twice;
   `destroy_client_entities` and account_id-keyed lookups elsewhere
   (chat, friends) get an ambiguous answer.

**Suggested remediation (one line)**
Hold the `connected` lock across the entire scan-evict-insert sequence
(refactor `handle_login` to acquire once and structure the eviction
work as data extraction inside the lock + side effects outside).

**Would benefit from x64dbg trace?**
No — repro is a two-socket Rust test.

---

### CAT-A-14 — Authentication (0x01) message is parsed-and-discarded; the server does not verify the client knows the session key

**Severity**: Low
**Class**: defense-in-depth — handshake completeness
**Wire surface**: System message 0x01 (`authenticate`, WORD_LENGTH)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The first message in every post-Phase-3 bundle is the client's
`authenticate` (0x01). The C++ reference server (per the in-code comment)
"ignores this message — entity creation happens on ENABLE_ENTITIES (0x08)
so the client's entity system is ready." The current Rust implementation
mirrors that: `encrypted/mod.rs:145-156` skips the message and advances the
offset. This is correct vs. the C++ behaviour, but it does mean the
**possession of the AES key** is the only thing proving the client knows
the session_key — there is no challenge-response that the client must
satisfy with the session_key as input. Combined with the all-zero IV
(CAT-A-02) and the absence of inbound replay protection (CAT-A-03), there
is no mechanism that distinguishes a legitimate client from a
captured-key replayer.

The Ghidra string `ServerConnection::authenticate: Unexpected key! (%s, wanted %s)`
at `0x019d0030` confirms the client SIDE does compare keys, but the
server-side comparison is absent.

**Evidence**
- Ghidra: `0x019d0030` `ServerConnection::authenticate: Unexpected key! (%s, wanted %s)` — invoked from `ServerConnection_authenticate` at `0x00dd85b6`. The client validates that the SessionKey it sees in the inbound stream matches the one Phase 2 returned. The server has no symmetric check.
- Client behavioral log: n/a.
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/base/connect_loop/encrypted/mod.rs:142-156` (authenticate is consumed without inspection).

**Attack scenario**
1. Attacker captures Phase 2 ticket + session_key (CAT-A-01).
2. Wins the Phase 3 race (CAT-A-05).
3. Sends a valid encrypted bundle. Server decrypts (HMAC valid since the
   attacker has the key); the `authenticate` field is consumed without
   inspection; subsequent messages dispatch normally.
4. Observable effect: full session takeover with no proof-of-key beyond
   the HMAC verification, which is implicit in successful decryption.

**Suggested remediation (one line)**
Parse the `authenticate` payload and verify it carries the same session
fingerprint Phase 2 produced (e.g. an HMAC over a server-issued nonce);
treat a mismatch as `disconnect_reason = "auth_mismatch"`. The HMAC over
the AES key already exists — the gap is the explicit fingerprint check.

**Would benefit from x64dbg trace?**
Yes — capturing the exact bytes the client puts in the authenticate
WSTRING would lock the spec for the fix author.

---

## Not Filed

- **AVATAR_UPDATE_EXPLICIT (0x03) accepted on the encrypted channel before world entry** — checked; the handler in `encrypted/mod.rs:214` requires `player_entity_id` to be `Some`, which is only set in `play_character.rs:131-133`. Pre-world-entry packets are dropped, not dispatched. Properly gated.
- **deleteCharacter has no confirmation prompt server-side** — the spec/protocol does not require one (it's a UX decision in the client). The account-id ownership check at the SQL layer (`WHERE player_id = $1 AND account_id = $2`) is the load-bearing defense and is present.
- **requestCharacterVisuals as enumeration vector** — the SQL filters by `account_id` (handler line 184), so it cannot leak other accounts' characters even with a guessed player_id. Not filed.
- **Phase 3 baseAppLogin embeds an `account_id` field in the wire payload** — parsed-and-discarded at `login/mod.rs:242` (`_account_id`). The trusted account_id comes from the ticket map. Correct shape; no finding.
- **CharDefId trust** — chardef_lookup is a static server-side table; an out-of-range CharDefId returns None and the handler rejects the create. Properly server-derived. Not filed.
- **Starting inventory / abilities** — pulled from `resources.char_creation_abilities` and `resources.items.container_sets` server-side, not from the client payload. Properly server-derived.
- **`access_level` desync between `account.accesslevel` and `sgw_player.access_level`** — the only **authorization-load-bearing** consumer is `ConnectedClientState.access_level` (sourced from auth ticket = account row). The `PlayerLoadData.access_level` (sourced from sgw_player row) flows out to the client as a property (informational) but is never used for an auth decision. Two sources of truth, but the authorization-affecting one is the correct one. Borderline — filed as a documentation concern, not a security finding.
- **LogOff/Disconnect from another session** — both arrive on the encrypted channel keyed by the inbound UDP `addr`; an attacker cannot inject these on behalf of a different session without decrypting that session's channel (which is the same problem as session takeover, already covered by CAT-A-01/03/05/14).
- **Python console on port 8989** — referenced in the audit prompt as something `network-security-auth` knows about. Workspace grep for `8989`, `python_console`, `Python.*Console` returns no hits in `crates/`. Either retired or never implemented in the Rust port; nothing to file. If the surface is added later, treat it as a separate audit pass.
- **0x0D channelSetup** — per the comment in `encrypted/mod.rs:511-517`, 0x0D as a literal msg_id never appears on the wire (it's reserved for the high-bit entity-message namespace). The audit-prompt's "channelSetup" name appears in `wire_log/client_names.rs:206` as a stale label. No live surface to audit.
- **Mercury HMAC-MD5 strength** — MD5 is broken for collision resistance but HMAC-MD5 remains acceptable for integrity in 2026 (no practical key recovery). Not filed as a primary finding; lumped into the CAT-A-01/02 "modernize the crypto stack" remediation.
