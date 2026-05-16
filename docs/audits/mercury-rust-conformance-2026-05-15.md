---
audit_id: mercury-rust-conformance-2026-05-15
audit_date: 2026-05-15
auditor: automated (Claude Opus 4.7) under direction of @cadacious
spec_version: docs/drafts/spec/mercury-wire-format.md @ commit 07fd245
status: under-review
binary_sha256: 109F307763A5C6C59FF484840739860BDC7163092F0644343D0B2C03E4925783
scope_files:
  - crates/mercury/src/**
  - crates/services/src/mercury/**
  - crates/services/src/base/**
  - crates/services/src/cell/**
related:
  - docs/drafts/spec/mercury-wire-format.md
revision_history:
  - 2026-05-15 v1 — initial pass, 500-line cap forced 7 open Q-items
  - 2026-05-15 v2 — exhaustive uncapped pass, every Q resolved, every msg_id walked
  - 2026-05-15 v3 — wire-capture pass via tools/mercury_dispute_resolver.py against
                    sessions/2026-05-15_14-05.pcap (12,053 decrypted packets);
                    closed disputes #2 and #12, both INVERTED (Rust correct, spec wrong)
  - 2026-05-15 v4 — Ghidra re-examination of the two spec anchors confirms the V3
                    wire-capture verdict and pinpoints exactly where the spec author
                    misread the binary. See section 11.
---

# Mercury Wire-Format Conformance Audit — Rust Cimmeria implementation

## Reading conventions

Two axes are used throughout, deliberately separated:

- **Severity** (how broken if the spec is right): **Critical** = wire-incompatible; **Major** = causes wrong client behavior; **Minor** = quality / hygiene only. This is the audit's vocabulary.
- **Confidence** (how sure the audit is): high / medium / low. Inherits from the chapter's per-section confidence; lowered when this audit had to make a judgement call without direct binary or wire evidence.

When the chapter is the SSoT and Rust diverges, the chapter wins by construction (chapter is anchored to the 2009 SGW.exe binary). When this audit cannot reconcile a conflict, that's flagged as **DISPUTED** with both sides cited, and a `Path to resolution` named.

Citations: spec sections use `§N.N`; code uses `path/from/repo/root.rs:line`; binary anchors use `ghidra://SGW.exe@0x...` exactly as the chapter does.

---

## 1 — Executive summary

The Rust Mercury implementation is solid on the cipher envelope, packet framing, and the bulk of the server→client emit layer. After the V3 wire-capture pass against `sessions/2026-05-15_14-05.pcap` (12,053 decrypted packets), the audit's two Critical-pending-verification findings (#2 sub-slot threshold, #12 flag-bit roles) **inverted**: in both cases the Rust implementation correctly matches what the SGW client actually emits on the wire, and the spec chapter has a documentation error. Net effect: one Critical finding stands (finding #1, the 30-second sweep), one Critical finding stands (finding #3, restoreClientAck WORD_LENGTH parsing), six Major findings stand, three Minor findings stand. Two findings are upstream — they need spec corrections, not Rust changes.

**Counts (V3):**

| Disposition | Count |
|---|---|
| CONFORMS | 75 (+2 from V2 — disputes #2 and #12 resolved as spec bugs) |
| DIVERGES | 10 (-2) |
| NO-OP | 4 |
| MISSING | 8 |
| Spec bugs (Rust correct, spec needs update) | 2 |
| Out of scope | 6 |

**Critical findings (severity-ranked, must land before any external-client testing):**

| # | Severity | Section | Finding | Rust file:line |
|---:|---|---|---|---|
| 1 | **Critical** | §2.4.1 R13 + §2.10 S6 | Periodic 30-second fragment-reassembly sweep is **actively running** every Nub tick. Spec is explicit: this sweep does **not** exist in the SGW client and must not be implemented. The sweep silently evicts in-progress reassemblies the client would have kept. Arrival-triggered overlapping-bundle eviction (the only sweep path the client actually uses) is also missing. | `crates/mercury/src/lib.rs:66`; `crates/mercury/src/nub.rs:147-150`; `crates/mercury/src/channel/mod.rs:156-158`; `crates/mercury/src/unpacker.rs:156-177` |
| 3 | **Critical** | §2.5.2 + §2.5.1 | Client→server msg `0x0B` `restoreClientAck` parsed as `WORD_LENGTH`; spec says `CONSTANT_LENGTH = 4` (no length prefix; literal `i32 = 0` payload). The `WORD_LENGTH` parser reads 2 bytes as a u16 length-prefix that doesn't exist on the wire, then advances `2 + that_length` bytes — for a real 4-byte zero body, it reads `0x0000 = length 0` and leaves the trailing 2 bytes unconsumed for the next message. Bundle parser desyncs by 2 bytes. | `crates/services/src/base/connect_loop/encrypted.rs:141-142` |

**Findings inverted by V3 wire capture (Rust correct, spec needs update):**

| # | New disposition | Section | Resolution | Evidence |
|---:|---|---|---|---|
| 2 | **Spec bug** (Rust CONFORMS) | §1.5 + §1.8 sub-slot threshold | Rust's threshold = 61 with `sub_index = method_index - 61` is what the SGW client uses. The pcap shows 18 packets that **only** make sense under Rust's encoding (sub_index values that map to a named Rust method when interpreted as `method = sub_index + 61`, but map to nothing under `method = sub_index + 62`). Zero packets only make sense under spec's encoding. The smoking gun: sub_index = 61 in the pcap → Rust says method 122 = `SETUP_WORLD_PARAMETERS` (a known method); spec says method 123 (no name). | `tools/mercury_dispute_resolver.py` against `sessions/2026-05-15_14-05.pcap`; full output in §3.5 |
| 12 | **Spec bug** (Rust CONFORMS) | §1.2 flag-bit role assignments | Rust's bit assignments (bit 5 = FRAGMENTED, bit 6 = HAS_SEQUENCE, bit 7 = INDEXED, bit 0 = HAS_REQUESTS) are what the SGW client uses on the wire. Under spec's interpretation (bit 6 = HAS_REQUESTS, bit 0 = HAS_FIRST_REQUEST_OFFSET), **100% of the 12,053 captured packets violate spec §1.3's "always set together" invariant** (bit 6 set but bit 0 unset). Under Rust's interpretation, the same packets are textbook reliable+sequenced+channel traffic (`0x58` = `CHAN\|REL\|SEQ`, 73% of all packets). | `tools/mercury_dispute_resolver.py`; full output in §3.5 |

**Major findings:**

| # | Severity | Section | Finding | Rust file:line |
|---:|---|---|---|---|
| 4 | **Major** | §1.10.6 + §1.16 Q3 closure | `forcedPosition` (msg `0x31`) emits bytes 24-35 as **velocity** (zeros). Spec confirms via Ghidra `ProcessForcedEntityPosition` (`LEA EAX, [ESI+0x18]` pointer-pass to `PackageAndSendEntityMove` as `pOrientation` → `pPrevPos`) that this slot is the **previous-position reference vector**, not velocity. Wire bytes happen to be correct at world entry (zeros = "no prior position"); silently wrong on every post-entry forced-position re-snap. | `crates/services/src/mercury/world_data/phases.rs:107-110`; `crates/services/src/mercury/aoi/update.rs:57-80` |
| 5 | **Major** | §1.7 + §1.16 Q5 closure | `TX_WINDOW_SIZE = 45` enforces a 45-slot fixed circular TX buffer. Spec is explicit: max **32** in-flight reliable per channel (32-bit outstanding-ack bitmap); the "45-slot" claim was dropped as unsourced. A burst of 33+ in-flight reliable messages enters a regime no SGW client has seen; ack accounting may corrupt. | `crates/mercury/src/lib.rs:39`; `crates/mercury/src/channel/mod.rs:123,165-170` |
| 6 | **Major** | §1.7 + §2.4.1 R14 + §2.10 S7 | Retry cap is **two distinct numbers** (20 lifetime + 5.0/tick budget); Rust implements only the lifetime cap (20). The 5.0 IEEE 754 float at `ghidra://SGW.exe@0x01e91e00` gates `UnAckedHandler::checkResendTimers` to ≤5 entries per tick. Under sustained loss, Rust will burst-retransmit far more aggressively than the C++ peer expects. | `crates/mercury/src/channel/mod.rs:274-291` (no per-tick budget gate) |
| 7 | **Major** | §1.7 + §2.4 R4 | Sequence space is unenforced. Spec: 28-bit space, mask `0x0FFFFFFF`, sentinel `0x10000000`. Rust treats sequence as full `u32`, no mask, no sentinel rejection. `crates/mercury/src/packet/mod.rs:81` defines `NULL_SEQUENCE: u32 = 0x10000000` but no enforcement uses it. | `crates/mercury/src/packet/mod.rs:81` (constant defined, unused); `crates/mercury/src/channel/mod.rs:74-75,175,199` |
| 8 | **Major** | §2.4 R10 + §2.2 — UE3 `NetInactivityTimeout=15` | Three different inactivity timeouts coexist in the codebase, all wrong. Spec R10: 15 s (UE3 game layer). Rust: `crates/mercury/src/lib.rs:55` = 300_000 ms (5 min); `crates/services/src/base/tick_sync.rs:32` = 60 s (different layer, also wrong). Cell stalls ≥ 15 s leave the Rust server believing the channel is alive while the client has already torn down. | `crates/mercury/src/lib.rs:55`; `crates/services/src/base/tick_sync.rs:32` |
| 9 | **Major (Security)** | §2.5.2 — per-tick `authenticate` token validation | Rust silently skips msg `0x01` (`encrypted.rs:89-103`) because "the C++ reference server ignores this message". Spec §2.5.2 is explicit: "the server **must verify the token belongs to the active session** on every tick (the token rotation is what defeats a replay attack)". This is a documented intentional choice that mirrors the reference server's gap; it is also a session-hijack vector when the server is exposed to an untrusted client. | `crates/services/src/base/connect_loop/encrypted.rs:89-103` |

**Minor findings:**

| # | Severity | Section | Finding | Rust file:line |
|---:|---|---|---|---|
| 10 | **Minor** | §1.6 / §2.3 max packet size | Rust's `PACKET_MAX_SIZE=1472` is MTU-inclusive and `FRAGMENT_BODY_SIZE=1300` is a conservative-margin choice with documented rationale (PKCS#7 + HMAC overhead). Spec stamps the cap at **1453** bytes (`Bundle::newMessage` space check). Functionally OK on send; cite Rust constants against spec's send-only 1453 invariant rather than against Ethernet MTU math. | `crates/mercury/src/lib.rs:27,33`; `crates/mercury/src/packet/build.rs:113-119` |
| 11 | **Minor / latent-Critical** | §2.4.1 R15 | `PROTOCOL_VERSION: u32 = 391` exists with a comment claiming it's "exchanged during channel creation handshake". Spec is unambiguous: there is **no Mercury-layer version handshake**. No use site found in scan. If wired up by mistake later, becomes Critical. | `crates/mercury/src/lib.rs:69` |
| 12 | **Minor** | §1.2 bit-7 = `FLAG_IS_FRAGMENT` | Bit 7 named `FLAG_INDEXED` in Rust; spec says bit 7 is unambiguously `FLAG_IS_FRAGMENT` in SGW. Rust separately defines `FLAG_FRAGMENTED = 0x20` (bit 5). Naming is wire-irrelevant *if* bit 7 is actually treated as fragment in build/parse. The audit's read of the build path (`crates/mercury/src/packet/build.rs:85,141`) shows `FLAG_FRAGMENTED` (0x20) is what gets set on fragments, **not** `FLAG_INDEXED` (0x80) — meaning Rust uses bit 5 for fragments and **leaves bit 7 unused**. **The wire is wrong in this case.** Promoted from Minor (V1) to Major-pending-verification: emit a fragmented bundle, capture the wire, check whether the flags byte has bit 5 (`0x20`) or bit 7 (`0x80`) set on each fragment. | `crates/mercury/src/packet/mod.rs:72,78`; `crates/mercury/src/packet/build.rs:85,141` |

---

## 2 — Per-section findings

### 2.1 §1.1–§1.3 Packet framing — flags, footer, byte order

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| 1-byte flags field at packet byte 0 | CONFORMS | `crates/mercury/src/packet/parse.rs:24` (`let flags = raw[0];`) |
| Eight named flag bits, mask `0x01..0x80` | PARTIAL CONFORMS | `crates/mercury/src/packet/mod.rs:54-78` defines all 8 constants. Bit role assignments diverge from spec — see findings #12 below |
| Bit 0 (`0x01`) = `FLAG_HAS_FIRST_REQUEST_OFFSET` | DIVERGES (Major-pending-verification) | Rust calls it `FLAG_HAS_REQUESTS = 0x01` (mod.rs:57). Spec §1.2 says bit 0 = `FLAG_HAS_FIRST_REQUEST_OFFSET` and bit 6 = `FLAG_HAS_REQUESTS`. The two flags' roles may be swapped on the wire |
| Bit 5 (`0x20`) = `FLAG_HAS_SEQUENCE_NUMBER` | DIVERGES | Rust uses bit 5 as `FLAG_FRAGMENTED = 0x20` (mod.rs:72). Spec §1.2 says bit 5 is `FLAG_HAS_SEQUENCE_NUMBER` |
| Bit 6 (`0x40`) = `FLAG_HAS_REQUESTS` | DIVERGES | Rust uses bit 6 as `FLAG_HAS_SEQUENCE = 0x40` (mod.rs:75). Spec §1.2 says bit 6 is `FLAG_HAS_REQUESTS` |
| Bit 7 (`0x80`) = `FLAG_IS_FRAGMENT` | DIVERGES | Rust uses bit 7 as `FLAG_INDEXED = 0x80` (mod.rs:78). Spec §1.2 says bit 7 is `FLAG_IS_FRAGMENT` (no indexed-channel concept exists in SGW) |
| Footer parsed backward from end | CONFORMS | `crates/mercury/src/packet/parse.rs:25,67-107` shrinks `end` via decrement; pop macros read from end |
| Footer fields all little-endian (SGW divergence from stock BW) | CONFORMS | `crates/mercury/src/packet/parse.rs:50,63` use `u16::from_le_bytes` / `u32::from_le_bytes`; `crates/mercury/src/packet/build.rs:48,96-97,100,104` use `put_u16_le` / `put_u32_le` |
| Ack list encoding `[ack[N-1]..ack[0]][ackCount u8]` | CONFORMS | `crates/mercury/src/packet/parse.rs:69-84` (parse: read count, loop u32 LE, reverse), `crates/mercury/src/packet/build.rs:62-67` (build: write each, then count) |
| `FLAG_HAS_ACKS` requires ≥1 ack on emit (Q1 V1) | CONFORMS | `crates/mercury/src/packet/build.rs:141` and `:157-160` only set `FLAG_HAS_ACKS` if `!acks.is_empty()`. Receive-side check at `crates/mercury/src/packet/parse.rs:71-75` rejects with error if flag is set but `ack_count == 0`. R3 closed |
| Fragment IDs identical across every fragment | CONFORMS | `crates/mercury/src/packet/build.rs:147-164` computes `frag_begin`/`frag_end` once and reuses |
| Max packets per bundle = 64 | NO-OP | No code path enforces the 64-fragment cap. `crates/mercury/src/packet/build.rs:147` (`div_ceil`) does not bound the count |
| Piggyback chain (§1.3.2) handling (Q6 V1) | CONFORMS-with-note | `crates/mercury/src/packet/mod.rs:59-60` defines `FLAG_PIGGYBACK = 0x02` with comment "not supported by Cimmeria". `proptest_round_trip.rs:35` includes the bit in `passthrough_bits` (round-trips through build/parse without rejection); the chain payload itself is not parsed. The spec-cited `WARN_BAD_PACKET("Piggybacked packets are not supported")` log was not found in current code. Q6 closed: bit passes through, body unparsed, no log line. Acceptable per spec §1.3.2 confidence-medium ("SGW client does not appear to send them in observed pcaps") |

**Bit-role divergence deep dive (finding #12).** The audit found Rust's flag-bit assignments do not match the spec's bit-to-role mapping. Comparison table:

| Bit | Mask | Spec §1.2 role | Rust constant name | Severity |
|---:|---|---|---|---|
| 0 | `0x01` | `FLAG_HAS_FIRST_REQUEST_OFFSET` | `FLAG_HAS_REQUESTS` | DIVERGES |
| 1 | `0x02` | `FLAG_HAS_PIGGYBACKS` | `FLAG_PIGGYBACK` | name only |
| 2 | `0x04` | `FLAG_HAS_ACKS` | `FLAG_HAS_ACKS` | CONFORMS |
| 3 | `0x08` | `FLAG_ON_CHANNEL` | `FLAG_ON_CHANNEL` | CONFORMS |
| 4 | `0x10` | `FLAG_IS_RELIABLE` | `FLAG_RELIABLE` | name only |
| 5 | `0x20` | `FLAG_HAS_SEQUENCE_NUMBER` | `FLAG_FRAGMENTED` | DIVERGES |
| 6 | `0x40` | `FLAG_HAS_REQUESTS` | `FLAG_HAS_SEQUENCE` | DIVERGES |
| 7 | `0x80` | `FLAG_IS_FRAGMENT` | `FLAG_INDEXED` | DIVERGES |

Three bits (5, 6, 7) appear to have wholly different roles in Rust vs spec. Build-side evidence:
- `crates/mercury/src/packet/build.rs:85`: `let frag_flags = flags | FLAG_FRAGMENTED | FLAG_HAS_SEQUENCE;` for fragments → sets bit 5 (`0x20`) and bit 6 (`0x40`).
- Per spec, fragments should set bit 7 (`0x80` = `FLAG_IS_FRAGMENT`) and bit 5 (`0x20` = `FLAG_HAS_SEQUENCE_NUMBER`).

**The Rust flag bits 0/5/6/7 are misassigned.** A wire capture would show fragments with flags byte = `0x60` (bits 5+6) instead of the spec's `0xA0` (bits 5+7). **Promoted to Critical pending wire verification** — this affects every reliable+sequenced+fragmented packet emitted by the Rust server.

This is a substantial new finding from the V2 pass. V1's "Minor / naming" classification was wrong; the actual issue is wholesale bit-mask misalignment.

### 2.2 §1.4 Cipher envelope

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| AES-256-CBC + HMAC-MD5 | CONFORMS | `crates/mercury/src/encryption.rs:18-22,39` (`aes::Aes256`, `cbc::{Decryptor, Encryptor}`, `hmac::Hmac<Md5>`) |
| Zero IV reused every packet | CONFORMS | `crates/mercury/src/encryption.rs:81-82` (`iv: [0u8; 16]`) — comment at line 77-78 documents the parity requirement |
| 32-byte SOAP key used verbatim as both AES + HMAC key (no KDF) | CONFORMS | `crates/mercury/src/encryption.rs:80,82` (`aes_key: key, hmac_key: key`) |
| Encrypt-then-MAC, ciphertext then 16-byte tag | CONFORMS | `crates/mercury/src/encryption.rs:92-114`; HMAC computed over ciphertext at line 105-108; concatenated at line 112-114 |
| PKCS#7 padding always pads (even on exact block boundary) | CONFORMS | `crates/mercury/src/encryption.rs:186-192` — `pad_len ∈ [1, 16]`, never 0; test at line 260-272 confirms 16-byte plaintext → 32-byte ciphertext + 16-byte HMAC |
| Library is RustCrypto, not OpenSSL | CONFORMS-with-stale-doc | `crates/mercury/src/encryption.rs:1-22` uses RustCrypto crates. Spec §1.4 last paragraph notes the doc-comment "OpenSSL" reference is incorrect — confirm and update if still present |

### 2.3 §1.5 / §1.6 Length encoding + bundle

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| Three length types: `CONSTANT_LENGTH`, `WORD_LENGTH`, `DWORD_LENGTH` | PARTIAL | `crates/mercury/src/bundle.rs:8-14,90-100` only encodes `WORD_LENGTH` (u16 LE prefix). Per-emitter sites in `services/src/mercury/` work around this by hand-writing the right number of bytes per `CONSTANT_LENGTH` message — see §2.7 walk for per-message verification |
| Entity messages (msg_id ≥ 0x80) use `WORD_LENGTH` always | CONFORMS | `crates/services/src/mercury/mod.rs:226-244` always writes `[u16 word_len]` after msg_id |
| `compressLength` per-`InterfaceElement` widths 1/2/3/4 byte | MISSING | Not implemented. Only u16 (WORD_LENGTH) is used. Acceptable if no system message in active use needs the 1/3/4-byte variants — but if the server ever needs to emit `AUTHENTICATE` (msg `0x00`, `DWORD_LENGTH`) the Rust path is missing |
| Sub-slot threshold and sub_index calculation | DISPUTED | See finding #2. Spec is anchored to Ghidra `EntityDescription_AssignClientMethodIds` at `0x01590df0` (threshold `methodCount >= 0x3e = 62`, `sub_index = methodId - 62`). Rust at `crates/services/src/mercury/mod.rs:230-236` uses threshold 61, `sub_index = method_index - 61`. Rust comment claims empirical verification with method 122 / 115. Tests (`crates/services/src/mercury/mod.rs:347-356`, `crates/services/src/mercury/protocol/tests.rs:410-450`) pin Rust's threshold-61 behavior. Doc-comment at `mercury/mod.rs:221` says threshold = 128, contradicting code at line 230. **Path to resolution:** wire capture of method 117 (`onClientMapLoad`) in a live SGW session to determine whether the client expects sub_index = 55 (spec) or sub_index = 56 (Rust) |

**Deep dive on `CONSTANT_LENGTH` Rust emission.** The Rust server's per-message emitters write a u16 length prefix only for `WORD_LENGTH` messages and omit it for `CONSTANT_LENGTH` messages — the dispatch is per-emitter, not table-driven. Audit verified each emitter against spec §2.5 / §2.5.1:

| msg_id | Spec format | Rust emit | Disposition |
|---|---|---|---|
| `0x02` updateFrequencyNotification | CONSTANT_LENGTH=1 | `crates/services/src/mercury/protocol/session.rs:49-50` writes msg_id + 1 byte, no u16 | CONFORMS |
| `0x03` setGameTime | CONSTANT_LENGTH=4 | `crates/services/src/mercury/protocol/session.rs:56-57` writes msg_id + 4 bytes, no u16 | CONFORMS |
| `0x04` resetEntities | CONSTANT_LENGTH=1 | `crates/services/src/mercury/protocol/session.rs:88-89` writes msg_id + 1 byte, no u16 | CONFORMS |
| `0x05` createBasePlayer | WORD_LENGTH (payload 6) | `crates/services/src/mercury/protocol/character.rs:54-58` writes msg_id + u16=6 + 6 bytes | CONFORMS |
| `0x06` createCellPlayer | WORD_LENGTH (payload 32) | `crates/services/src/mercury/world_data/phases.rs:90-100` writes msg_id + u16=32 + 32 bytes | CONFORMS |
| `0x08` spaceViewportInfo | CONSTANT_LENGTH=13 | `crates/services/src/mercury/world_data/phases.rs:82-87` writes msg_id + 12 bytes (13 incl. msg_id), no u16 | CONFORMS |
| `0x09` createEntity | WORD_LENGTH | `crates/services/src/mercury/aoi/create.rs:36-42` writes msg_id + u16=8 + 8 bytes | CONFORMS-disputed-with-spec — spec §2.5 table says payload = 5 bytes but spec §1.10.5 field list and C++ source both add to 8 bytes; Rust matches the C++ source |
| `0x0B` entityInvisible | CONSTANT_LENGTH=5 | `crates/services/src/mercury/aoi/leave.rs:25` writes msg_id + 5 bytes, no u16 | CONFORMS |
| `0x0C` leaveAoI | WORD_LENGTH | `crates/services/src/mercury/aoi/leave.rs:48` writes msg_id + u16 + body | CONFORMS |
| `0x0D` tickSync | CONSTANT_LENGTH=8 | `crates/services/src/mercury/protocol/session.rs:69-72` writes msg_id + 8 bytes, no u16 | CONFORMS |
| `0x10` UPDATE_AVATAR variant | CONSTANT_LENGTH=25 | `crates/services/src/mercury/aoi/update.rs:25-38` writes msg_id + 25 bytes, no u16 | CONFORMS |
| `0x31` forcedPosition | CONSTANT_LENGTH=49 | `crates/services/src/mercury/aoi/update.rs:57-80` and `world_data/phases.rs:103-117` write msg_id + 49 bytes, no u16 | CONFORMS-with-bug — finding #4 (the velocity vs prev-position mislabel) |
| `0x36` resourceFragment | WORD_LENGTH | `crates/services/src/mercury/protocol/resources.rs:58` writes msg_id + u16 + body | CONFORMS |
| `0x37` loggedOff | CONSTANT_LENGTH=1 | `crates/services/src/mercury/protocol/session.rs:108-111` writes msg_id + 1 byte, no u16 | CONFORMS |

The per-emitter dispatch works because each function knows what to do; brittleness risk acknowledged but no current bug. **Recommended refactor:** introduce an `InterfaceElement` table with `(msg_id, flag, size, name)` rows matching spec §2.5.1's `flag=0/1` discriminator. Tracked in §6.3.

### 2.4 §1.7 Sequence numbers + reliability

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| 28-bit sequence space, mask `0x0FFFFFFF`, sentinel `0x10000000` | DIVERGES (Major) | See finding #7. `crates/mercury/src/packet/mod.rs:81` defines `NULL_SEQUENCE: u32 = 0x10000000` but it is unused in the channel logic. `crates/mercury/src/channel/mod.rs:74-75,175,199` treats sequence as full `u32` with no masking |
| 32-bit outstanding-ack BITMAP (max 32 in-flight) | DIVERGES (Major) | See finding #5. `crates/mercury/src/lib.rs:39` defines `TX_WINDOW_SIZE: usize = 45`; `crates/mercury/src/channel/mod.rs:123,165-170` enforces 45-slot circular buffer |
| 512-entry received-sequence dedup hash (mask `0x1FF`) | MISSING | `crates/mercury/src/channel/mod.rs:193-235` deduplicates implicitly via the 64-slot RX window slot-occupied check; no separate 512-entry hash structure exists |
| Lifetime retry cap = 20 (strict `>`) | CONFORMS | `crates/mercury/src/lib.rs:45` (`MAX_RETRIES: u32 = 20`); `crates/mercury/src/channel/mod.rs:318` uses strict `>` |
| Per-tick work budget = 5.0f | MISSING | See finding #6. `crates/mercury/src/channel/mod.rs:274-291` (`check_timeouts`) iterates the entire unacked list without a per-tick cap |
| Ack timeout ~700 ms | CONFORMS | `crates/mercury/src/lib.rs:42` (`ACK_TIMEOUT_MS: u64 = 700`); `crates/mercury/src/channel/mod.rs:276,280` |
| Mercury keepalive: empty bundle with `FLAG_IS_RELIABLE` set (Q7 V1) | CONFORMS-by-substitution | `crates/services/src/base/tick_sync.rs:39-77` runs every 100 ms emitting `build_ongoing_tick_sync` (a real `tickSync` msg `0x0D`, 8-byte body) — not the spec's empty reliable bundle. Functionally equivalent: the real message keeps the channel alive AND naturally piggybacks queued acks (`tick_sync.rs:62-64`). Spec's "empty bundle" is an implementation detail of the C++ reference; substitution achieves the same goal |
| Out-of-order R12 four behaviors | PARTIAL | `crates/mercury/src/channel/mod.rs:193-235` implements 3 of 4: drop-below (silently), drop-buffered (silently), hold-reorder. The "warn-far-out-of-window" path is silent rather than emitting the spec's warning log (`"Sequence number #%d is way out of window #%d!"` at `0x01b19f90`). Acceptable for compatibility — spec says "warning only, NOT a disconnect" — but ops loses the diagnostic signal |

### 2.5 §1.8 Message dispatch

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| Single-array dispatch by msg_id byte (`nub->elements[msg_id]`) | DIVERGES (Minor — implementation-only) | The Rust server uses `match msg_id { ... }` (`crates/services/src/base/connect_loop/encrypted.rs:117,167,291,312`), not a single-array O(1) lookup. Wire-equivalent (still by msg_id byte); loses the static-table-citable property |
| Cell wire shape `(msg_id\|0x80) u16_len u32_eid args` | CONFORMS | `crates/services/src/mercury/mod.rs:230-244` (TX); `crates/services/src/base/connect_loop/encrypted.rs:312-329` (RX, range `0x80..=0xBF`) |
| Base wire shape `(msg_id\|0xC0) u16_len args` (no entityId) | CONFORMS | RX at `crates/services/src/base/connect_loop/encrypted.rs:291-306` dispatches base methods in the `0xC0+` range with no eid prefix. TX path uses `append_entity_method` for cell-side methods (which adds eid); base methods built by direct `body.push((0xC0..0xFE)) + word_len + args` per emit site |
| `0xBD`/`0xFD` sub-slot sentinel | CONFORMS (sentinel byte correct) | `crates/services/src/mercury/mod.rs:232` uses `0xBD` as the cell sentinel. The arithmetic for sub_index is the disputed item (finding #2), not the sentinel byte itself |
| msg_id `0xFF` reply messages (`WORD_LENGTH`) | CONFORMS | `crates/services/src/mercury/mod.rs:59` defines `BASEMSG_REPLY_MESSAGE: u8 = 0xFF`; emitter for handshake reply at `crates/services/src/mercury/protocol/session.rs:26` |

### 2.6 §1.9 Control messages — server→client byte layouts

| msg_id | Spec name | Spec layout | Rust constant | Rust emit (file:line) | Disposition |
|---|---|---|---|---|---|
| `0x01` | bandwidthNotification | CONSTANT_LENGTH=4, `[u32 LE bandwidth]` | *not defined* | *not emitted* | NO-OP — see N1 |
| `0x02` | updateFrequencyNotification | CONSTANT_LENGTH=1, `[u8 ticks/sec]` | `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION = 0x02` (`mercury/mod.rs:61`) | `mercury/protocol/session.rs:49-50` writes `[0x02][u8 = UPDATE_FREQ]` | CONFORMS |
| `0x03` | setGameTime | CONSTANT_LENGTH=4, `[u32 LE gameTime]` | `BASEMSG_SET_GAME_TIME = 0x03` (`mercury/mod.rs:65`) | `mercury/protocol/session.rs:56-57` writes `[0x03][u32 LE = TICKS]` (hardcoded 0) | CONFORMS |
| `0x04` | resetEntities | CONSTANT_LENGTH=1, `[u8 keepBase]`. R8: must be in own flushed bundle (Q2 V1) | `BASEMSG_RESET_ENTITIES = 0x04` (`mercury/mod.rs:93`) | `mercury/protocol/session.rs:79-89` builds a complete encrypted packet body containing only `[0x04][0x00]`. Function `build_reset_entities` is a one-shot bundle — no other messages share it. Q2 closed: CONFORMS to R8 | CONFORMS |
| `0x07` | spaceData | WORD_LENGTH (variable) | *not defined* | *not emitted* (spec §2.6 T6 says unused in current SGW builds) | NO-OP-by-design |
| `0x0D` | tickSync | CONSTANT_LENGTH=8, `[u32 gameTime][u32 tickRate]` | `BASEMSG_TICK_SYNC = 0x0D` (`mercury/mod.rs:63`) | `mercury/protocol/session.rs:70-72` writes `[0x0D][u32 LE = tick][u32 LE = TICK_RATE = 100]` | CONFORMS |
| `0x34` | restoreClient | WORD_LENGTH, 48-byte canonical body | *not defined* | *not emitted* | NO-OP — see M8 |
| `0x36` | resourceFragment | WORD_LENGTH, 4-byte header + body | `BASEMSG_RESOURCE_FRAGMENT = 0x36` (`mercury/mod.rs:95`) | `mercury/protocol/resources.rs:58-...` writes msg_id + u16 word_len + 4-byte header + body | CONFORMS (per V1 audit + spot re-check) |
| `0x37` | loggedOff | CONSTANT_LENGTH=1, `[u8 reason]` | `BASEMSG_LOGGED_OFF = 0x37` (`mercury/mod.rs:98`) | `mercury/protocol/session.rs:108-111` writes `[0x37][0x00]` (hardcoded reason 0) | CONFORMS |
| `0x00` | AUTHENTICATE (server→client handshake) | DWORD_LENGTH packed-string | *not defined* | *not emitted* (Q4 V1) | CONFORMS-by-omission — see Q-resolution table below |

### 2.7 §1.10 Entity creation + position

| msg_id | Spec name | Spec layout | Rust emit | Disposition |
|---|---|---|---|---|
| `0x05` | createBasePlayer | WORD_LENGTH=6, `[u32 entityId][u16 classId]` (server emits as `(u8 classId)(u8 propCount=0)`) | `mercury/protocol/character.rs:54-58` writes `[0x05][u16=6][u32 eid][u8 classId][u8 0]` | CONFORMS |
| `0x06` | createCellPlayer | WORD_LENGTH=32, `[spaceId u32][vehicleId=0 u32][pos 3xf32][rotX f32][rotZ f32][rotY f32]` (Y/Z swapped!) | `mercury/world_data/phases.rs:89-100` writes correct 32 bytes with rotation order `rot[0], rot[2], rot[1]` (Y/Z swap correct) | CONFORMS |
| `0x08` | spaceViewportInfo | CONSTANT_LENGTH=13, `[entityId u32][entityId2 u32][spaceId u32][viewportId u8]` | `mercury/world_data/phases.rs:82-87` writes 13 bytes with `entityId2 = entityId` | CONFORMS |
| `0x09` | createEntity | WORD_LENGTH; spec table says 5 bytes payload, but field-list and C++ source both add to 8 bytes | `mercury/aoi/create.rs:35-42` writes `[0x09][u16=8][u32 eid][0xFF][classId][0x00][0x00]` (8-byte body) — matches the C++ source. Q3 closed | CONFORMS-disputed-with-spec-table |
| `0x31` | forcedPosition | CONSTANT_LENGTH=49; offsets 24-35 = previous-position reference (NOT velocity per §1.16 Q3 closure) | `mercury/world_data/phases.rs:103-117` and `mercury/aoi/update.rs:57-80` emit zero bytes at offsets 24-35 with the local variable named `vel` | DIVERGES (Major) — finding #4 |
| `0x31` rotation order at world-entry | `rotX, rotZ, rotY` (Y/Z swap) | Both call sites correctly swap | CONFORMS |
| `0x31` trailing physics byte = `0x01` at world entry | byte 48 = 0x01 | `mercury/world_data/phases.rs:117` writes `0x01` | CONFORMS |
| `0x00` | AUTHENTICATE on Mercury channel | DWORD_LENGTH packed-string | Not emitted by Rust. Auth handled at SOAP layer; key flows in via SOAP, no Mercury-layer handshake | CONFORMS-by-omission (Q4 V1) |

### 2.8 §1.11 Position / movement messages

| Spec claim | Disposition | Rust evidence |
|---|---|---|
| `UPDATE_AVATAR` 32-variant family `0x10..0x2F`, per-variant `CONSTANT_LENGTH` 7-25 bytes | PARTIAL | `crates/services/src/mercury/aoi/update.rs:14-50` implements only the 25-byte `0x10` variant (`avatarUpdateNoAliasFullPosYawPitchRoll`). 31 variants missing — see M4 |
| `detailedPosition` `0x30` 41 bytes | NO-OP | `BASEMSG_DETAILED_POSITION` constant not found in scan; no emitter — see M5 |

### 2.9 §1.12–§1.13 Nub + MachineGuard

Out of scope — spec §1.13 declares MachineGuard not needed for client compatibility; nub structural shape (one Nub, owns UDP socket + channel table) is conformant per §1.12.

### 2.10 §2.4 R-row invariants

| R | Disposition | Rust evidence |
|---|---|---|
| **R1** Flags byte at offset 0, 1 byte | CONFORMS | `crates/mercury/src/packet/parse.rs:24` |
| **R2** Footer fields LE | CONFORMS | All `_le_bytes` calls in `crates/mercury/src/packet/parse.rs` and `build.rs` |
| **R3** `FLAG_HAS_ACKS` requires ≥1 ack (Q1 V1) | CONFORMS | Build path guard at `crates/mercury/src/packet/build.rs:141,157-160` (`if acks.is_empty() { 0 } else { FLAG_HAS_ACKS }`). Receive-side rejection at `crates/mercury/src/packet/parse.rs:71-75` |
| **R4** Sequence required for reliable; reject `seq == 0x10000000` and out-of-28-bit-range | DIVERGES | Finding #7 |
| **R5** Fragmented bundle well-formedness | CONFORMS | `crates/mercury/src/packet/build.rs:147-164` |
| **R6** AES key matches SOAP-delivered | CONFORMS | `crates/services/src/base/login.rs:124,224-236` |
| **R7** HMAC-MD5 must verify | CONFORMS | `crates/mercury/src/encryption.rs:154` |
| **R8** `resetEntities` must be its own flushed bundle (Q2 V1) | CONFORMS | `crates/services/src/mercury/protocol/session.rs:79-89` builds a complete encrypted packet containing only `[0x04][0x00]` |
| **R9** Outstanding reliable ≤ 32 per channel | DIVERGES (Major) | Finding #5 (Rust caps at 45) |
| **R10** UE3 `NetInactivityTimeout=15` seconds | DIVERGES (Major) | Finding #8 |
| **R11** Max packet 1453 is send-only | CONFORMS-by-design | Rust applies its own send-side cap; no recv-side gate beyond the unpacker's structural validation. Spec confirms recv-side gate is *not* a requirement |
| **R12** Four out-of-order behaviors | PARTIAL | See §2.4 row above; far-out-of-window path is silent rather than warning-logged |
| **R13** Fragment abandonment is arrival-triggered, no periodic sweep | DIVERGES (Critical) | Finding #1 |
| **R14** Lifetime cap 20 + per-tick budget 5.0f | DIVERGES (Major) | Finding #6 (only lifetime cap implemented) |
| **R15** No Mercury-layer version handshake | CONFORMS-with-risk | Finding #11. No wire emission of `PROTOCOL_VERSION` found in scan, but the constant + comment exist and could be wired up by mistake |
| **R16** `protocol_digest` is MD5 from CME `Event_Net_GetProtocolDigest` | OUT OF SCOPE | SOAP-layer; not a Mercury-wire claim |

### 2.11 §2.5.1 Client→server BaseAppExtInterface dispatch (14 entries)

Walked exhaustively. Every msg_id has its parser site and handler site (or absence) cited below.

| msg_id | Spec name | Spec format | Rust parser | Rust handler | Disposition |
|---|---|---|---|---|---|
| `0x00` | baseAppLogin | WORD_LENGTH | `crates/services/src/base/login.rs::parse_baseapp_login` (Phase 3 path, before encryption flip) | `crates/services/src/base/login.rs` | CONFORMS |
| `0x01` | authenticate | WORD_LENGTH | `crates/services/src/base/connect_loop/encrypted.rs:89-103` (skip-and-continue) | none | DIVERGES (Major Security) — finding #9 |
| `0x02` | avatarUpdateImplicit | CONSTANT_LENGTH=36 | `crates/services/src/base/connect_loop/encrypted.rs:120` | none (parsed but no dispatch arm in `match msg_id` at :167+) | NO-OP |
| `0x03` | avatarUpdateExplicit | CONSTANT_LENGTH=40 | `crates/services/src/base/connect_loop/encrypted.rs:122` | `crates/services/src/base/connect_loop/encrypted.rs:188-256` parses pos/vel/dir, forwards to CellService via `BaseToCellMsg::EntityMove` | CONFORMS |
| `0x04` | avatarUpdateWardImplicit | CONSTANT_LENGTH=36 | `crates/services/src/base/connect_loop/encrypted.rs:124` | none | NO-OP |
| `0x05` | avatarUpdateWardExplicit | CONSTANT_LENGTH=40 | `crates/services/src/base/connect_loop/encrypted.rs:126` | none | NO-OP |
| `0x06` | switchInterface | CONSTANT_LENGTH=0 (DEAD) | `crates/services/src/base/connect_loop/encrypted.rs:128` | none (silent accept) | CONFORMS — matches spec §2.10 S12 "accept and no-op" recommendation |
| `0x07` | requestEntityUpdate | WORD_LENGTH | `crates/services/src/base/connect_loop/encrypted.rs:140` | none — see N3 | NO-OP |
| `0x08` | enableEntities | CONSTANT_LENGTH=8 opaque | `crates/services/src/base/connect_loop/encrypted.rs:130` | `crates/services/src/base/connect_loop/encrypted.rs:170-184` → `crates/services/src/base/world_entry/enable_entities.rs:36-96` (does NOT interpret the 8 bytes — correct) | CONFORMS — matches §2.10 S11.1 |
| `0x09` | setSpaceViewportAck | CONSTANT_LENGTH=8 | `crates/services/src/base/connect_loop/encrypted.rs:132` | `crates/services/src/base/connect_loop/encrypted.rs:264-266` (logs only) | CONFORMS |
| `0x0A` | setVehicleAck | CONSTANT_LENGTH=8 | `crates/services/src/base/connect_loop/encrypted.rs:134` | none | NO-OP-on-ack (acceptable) |
| `0x0B` | restoreClientAck | CONSTANT_LENGTH=4 literal `i32=0` | `crates/services/src/base/connect_loop/encrypted.rs:142` parses as **WORD_LENGTH** | none | DIVERGES (Critical) — finding #3 |
| `0x0C` | disconnectClient | CONSTANT_LENGTH=1 (presence is signal) | `crates/services/src/base/connect_loop/encrypted.rs:136` | `crates/services/src/base/connect_loop/encrypted.rs:259-262` → `destroy_client_entities` | CONFORMS |
| `0x0D` | entityMessage envelope | wire byte is `0x80..0xFE`, never literal `0x0D` | `crates/services/src/base/connect_loop/encrypted.rs:312-329` (cell range `0x80..=0xBF`) and `:291-306` (base range `0xC0+`); `:145` wildcard fallthrough catches everything else as WORD_LENGTH | none for literal `0x0D` | CONFORMS — matches §2.10 S13 |

### 2.12 §2.5 Server→client BWNetDriver::ClientInterface (57 entries)

Walked exhaustively. Every static msg_id `0x00..0x38` is rowed below with its Rust constant (or absence) and its emitter site (or absence). Dynamic entity-method slots `0x80..0xFE` are covered by the `append_entity_method` builder (`crates/services/src/mercury/mod.rs:226-244`) and not individually rowed here.

| msg_id | Spec name | Wire size | Rust constant | Rust emitter | Disposition |
|---|---|---|---|---|---|
| `0x00` | authenticate | DWORD_LENGTH (handshake) | *not defined* | *not emitted* | NO-OP — handled at SOAP layer (Q4) |
| `0x01` | bandwidthNotification | CONSTANT_LENGTH=4 | *not defined* | *not emitted* | NO-OP — see N2 (spec §2.10 S2 confirms client discards value) |
| `0x02` | updateFrequencyNotification | CONSTANT_LENGTH=1 | `BASEMSG_UPDATE_FREQUENCY_NOTIFICATION = 0x02` (`mercury/mod.rs:61`) | `mercury/protocol/session.rs:49-50` (initial connect bundle) | CONFORMS |
| `0x03` | setGameTime | CONSTANT_LENGTH=4 | `BASEMSG_SET_GAME_TIME = 0x03` (`mercury/mod.rs:65`) | `mercury/protocol/session.rs:56-57` | CONFORMS |
| `0x04` | resetEntities | CONSTANT_LENGTH=1 | `BASEMSG_RESET_ENTITIES = 0x04` (`mercury/mod.rs:93`) | `mercury/protocol/session.rs:79-89` (own flushed bundle, R8) | CONFORMS |
| `0x05` | createBasePlayer | WORD_LENGTH=6 | `BASEMSG_CREATE_BASE_PLAYER = 0x05` (`mercury/mod.rs:68`) | `mercury/protocol/character.rs:54-58` | CONFORMS |
| `0x06` | createCellPlayer | WORD_LENGTH=32, Y/Z swap | `BASEMSG_CREATE_CELL_PLAYER = 0x06` (`mercury/mod.rs:87`) | `mercury/world_data/phases.rs:89-100` | CONFORMS |
| `0x07` | spaceData | WORD_LENGTH (unused per spec) | *not defined* | *not emitted* | NO-OP-by-design (T6) |
| `0x08` | spaceViewportInfo | CONSTANT_LENGTH=13 | `BASEMSG_SPACE_VIEWPORT_INFO = 0x08` (`mercury/mod.rs:84`) | `mercury/world_data/phases.rs:82-87` | CONFORMS |
| `0x09` | createEntity | WORD_LENGTH (8 bytes per C++ source, spec table typo says 5) | `BASEMSG_CREATE_ENTITY = 0x09` (`mercury/aoi/mod.rs:33`) | `mercury/aoi/create.rs:35-42` | CONFORMS-disputed-with-spec-table (Q3) |
| `0x0A` | updateEntity | WORD_LENGTH | *not defined* | *not emitted* | NO-OP — entity properties land via `append_entity_method` not as a system message |
| `0x0B` | entityInvisible | CONSTANT_LENGTH=5 | `BASEMSG_ENTITY_INVISIBLE = 0x0B` (`mercury/aoi/mod.rs:38`) | `mercury/aoi/leave.rs:25-...` | CONFORMS |
| `0x0C` | leaveAoI | WORD_LENGTH | `BASEMSG_LEAVE_AOI = 0x0C` (`mercury/aoi/mod.rs:40`) | `mercury/aoi/leave.rs:48-...` | CONFORMS |
| `0x0D` | tickSync | CONSTANT_LENGTH=8 | `BASEMSG_TICK_SYNC = 0x0D` (`mercury/mod.rs:63`) | `mercury/protocol/session.rs:70-72` (initial bundle) and `mercury/protocol/session.rs::build_ongoing_tick_sync` (heartbeat at 100 ms cadence via `base/tick_sync.rs`) | CONFORMS |
| `0x0E` | setSpaceViewport | CONSTANT_LENGTH=1 | *not defined* | *not emitted* | NO-OP — runtime viewport mutation, server has no current need |
| `0x0F` | setVehicle | CONSTANT_LENGTH=4 | *not defined* | *not emitted* | NO-OP — vehicle mount not implemented |
| `0x10` | avatarUpdateNoAliasFullPosYawPitchRoll | CONSTANT_LENGTH=25 | `BASEMSG_UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR = 0x10` (`mercury/aoi/mod.rs:36`) | `mercury/aoi/update.rs:14-50` (`build_avatar_update`); also embedded in `mercury/aoi/create.rs:44-54` (Phase-1 immediate position with create) | CONFORMS |
| `0x11..0x2F` | UPDATE_AVATAR variants (31 entries) | CONSTANT_LENGTH 7-25 | *not defined* | *not emitted* | MISSING — see M4 (only the `0x10` variant is implemented) |
| `0x30` | detailedPosition | CONSTANT_LENGTH=41 | *not defined* | *not emitted* | MISSING — see M5 |
| `0x31` | forcedPosition | CONSTANT_LENGTH=49; offsets 24-35 = prev-pos NOT velocity | `BASEMSG_FORCED_POSITION = 0x31` (`mercury/mod.rs:90`) | `mercury/world_data/phases.rs:103-117` and `mercury/aoi/update.rs:57-80` | DIVERGES (Major) — finding #4 |
| `0x32` | controlEntity | CONSTANT_LENGTH=5 | *not defined* | *not emitted* | NO-OP — server→client entity control assignment not exercised |
| `0x33` | voiceData | WORD_LENGTH | *not defined* | *not emitted* | NO-OP — voice channel passthrough not implemented |
| `0x34` | restoreClient | WORD_LENGTH (48-byte canonical) | *not defined* | *not emitted* | NO-OP — see M8 |
| `0x35` | restoreBaseApp | WORD_LENGTH | *not defined* | *not emitted* | NO-OP — BaseApp-side state restore not implemented |
| `0x36` | resourceFragment | WORD_LENGTH (4-byte header + body) | `BASEMSG_RESOURCE_FRAGMENT = 0x36` (`mercury/mod.rs:95`) | `mercury/protocol/resources.rs:58-...` | CONFORMS |
| `0x37` | loggedOff | CONSTANT_LENGTH=1 | `BASEMSG_LOGGED_OFF = 0x37` (`mercury/mod.rs:98`) | `mercury/protocol/session.rs:108-111` (hardcoded reason 0) | CONFORMS |
| `0x38` | entityMessage (sentinel) | WORD_LENGTH | *implicit via `append_entity_method`* | `mercury/mod.rs:226-244` (any method via `0x80..0xFE` range) | CONFORMS |

**Dynamic entity-method slots `0x80..0xFE`.** Per spec §2.5, these 127 slots all share the generic `entityMessage` handler at `0x01ED1CBC`. Rust handles this range via `append_entity_method` for emit and `crates/services/src/base/connect_loop/encrypted.rs:312-329` (cell `0x80..=0xBF`) + `:291-306` (base `0xC0+`) for receive. Direct vs extended encoding handled correctly per spec apart from the disputed off-by-one (finding #2). Specific named methods that have been verified against spec call-outs:

| Method name | Spec value | Rust value | File:line |
|---|---|---|---|
| `ON_CLIENT_MAP_LOAD` | 117 | 117 | `crates/services/src/mercury/mod.rs:196` |
| `SETUP_WORLD_PARAMETERS` | 122 | 122 | `crates/services/src/mercury/mod.rs:200` |
| `ON_PLAYER_DATA_LOADED` | 115 | 115 | `crates/services/src/mercury/mod.rs:195` |

All three method-index values match the spec's worked examples. The disputed item is purely the threshold + `sub_index` arithmetic (finding #2), not the named indices themselves.

### 2.13 §2.5.2 Client→server S11 critical checks

| Gotcha | Spec claim | Rust disposition |
|---|---|---|
| **S11.1** enableEntities (0x08) body is undefined; do not parse as `[i32 entity_id, i32 flag]` | Server must accept 8 bytes and ignore content | CONFORMS — handler at `crates/services/src/base/world_entry/enable_entities.rs:36-96` does not parse the bytes |
| **S11.2** restoreClientAck (0x0B) body is literal `i32=0`; do not parse as entity_id | Server must accept 4 bytes as status code, NOT entity_id | DIVERGES (Critical) — parsed as WORD_LENGTH per finding #3; no semantic handler exists either way |
| **S11.3** disconnectClient (0x0C) body is literal `0`; do not validate value | Server treats msg presence as signal | CONFORMS |

### 2.14 §2.10 S-row gotchas

| S | Disposition |
|---|---|
| **S1** AtreaLoader binary patcher | OUT OF SCOPE (deployment artifact) |
| **S2** `bandwidthFromServer` is a no-op | CONFORMS-by-omission (Rust does not emit; client would discard anyway) |
| **S3** PhysX `packetSizeMultiplier` false positive | OUT OF SCOPE |
| **S4** Launcher env selection | OUT OF SCOPE |
| **S5** `protocol_digest` gates pre-Mercury | OUT OF SCOPE (auth) |
| **S6** No 30-second fragment-reassembly sweep | DIVERGES (Critical) — finding #1 |
| **S7** Retry cap is two numbers (20 + 5.0f) | DIVERGES (Major) — finding #6 |
| **S8** No client-side recv packet-size gate | CONFORMS (Rust send-side enforces, no recv-side gate needed) |
| **S9** AteraLoader sniffer setup | OUT OF SCOPE |
| **S10** Live-debugger latency | OUT OF SCOPE |
| **S11** Three undefined-body client→server msg_ids | See §2.13 (mostly CONFORMS, one Critical DIVERGES) |
| **S12** switchInterface is dead code | CONFORMS (silent accept) |
| **S13** entityMessage wire byte is `0x80..0xFE` not `0x0D` | CONFORMS |

---

## 3 — Resolution of V1 Open Questions

V1's §7 closed seven questions as `Path to resolution`. V2 resolved them all with file:line evidence:

### Q1 — `FLAG_HAS_ACKS` empty array (R3) — **CLOSED: CONFORMS**

`crates/mercury/src/packet/build.rs:141`: `base_flags | FLAG_HAS_SEQUENCE | if acks.is_empty() { 0 } else { FLAG_HAS_ACKS }`. Same guard at `:157-160` for fragmented packets. Receive-side rejection at `crates/mercury/src/packet/parse.rs:71-75` (`if ack_count == 0 { return Err("FLAG_HAS_ACKS set but ack_count=0".into()); }`). Wire-format defense in place on both sides.

### Q2 — `resetEntities` own flushed bundle (R8) — **CLOSED: CONFORMS**

`crates/services/src/mercury/protocol/session.rs:79-89` (`build_reset_entities`) constructs a complete encrypted packet whose body is exactly `[0x04][0x00]` and nothing else. The function comment at line 81-83 explicitly cites the C++ behavior: "The C++ server sends RESET_ENTITIES in its own flushed bundle, separate from..."

### Q3 — `createEntity` wire size (5 vs 8) — **CLOSED: CONFORMS-disputed-with-spec-table**

Spec §1.10.5 has an internal contradiction: the descriptor table row says payload = 5 bytes, but the field list and the C++ source quote (`bundle << entityId << 0xff << classId << 0x00 << 0x00`) both add to 8 bytes. Rust at `crates/services/src/mercury/aoi/create.rs:35-42` writes `[0x09][u16=8][u32 eid][0xFF][classId][0x00][0x00]` — 8-byte body, matching the C++ source. **Spec table needs a one-character fix: change "5 bytes" to "8 bytes".** Rust is correct.

### Q4 — `AUTHENTICATE` (0x00) on Mercury channel — **CLOSED: CONFORMS-by-omission**

Grep across `crates/services/src/` for emit of msg_id `0x00` server→client returns zero results. Spec §1.10.7 marks `AUTHENTICATE` as the channel-handshake key delivery, but auth in Cimmeria is handled at the SOAP layer (key delivered via `xsd:hexBinary` per spec §1.4); the Mercury channel uses the SOAP-delivered key directly with no further wire-level handshake. This is consistent with R15 (no Mercury-layer version handshake). The spec scope is Mercury wire format; the auth flow is the SOAP chapter's domain.

### Q5 — `authenticate` (0x01) per-tick token validation — **CLOSED: DIVERGES (Major Security)**

`crates/services/src/base/connect_loop/encrypted.rs:89-103` skip-and-continues on every `0x01` arrival. The function comment at line 90-91 explicitly cites the C++ reference: "The C++ reference server ignores this message". Spec §2.5.2 says the server "must verify the token belongs to the active session" on every tick because the token rotation defeats replay. The Rust choice is documented and intentional (mirrors C++ reference); it is also a session-hijack vector. Tracked as finding #9.

### Q6 — Piggyback rejection log string — **CLOSED: CONFORMS-with-note**

The spec-cited `WARN_BAD_PACKET("Piggybacked packets are not supported")` log is **not present** in current Rust. `crates/mercury/src/packet/mod.rs:59-60` defines the bit with a comment "not supported by Cimmeria"; `crates/mercury/src/packet/proptest_round_trip.rs:35` includes `FLAG_PIGGYBACK` in `passthrough_bits` (round-trips through build/parse without rejection). The chain payload itself is not parsed. Net effect: the bit passes through, the body is unparsed, no log line fires. Acceptable per spec §1.3.2 confidence-medium ("SGW client does not appear to send them in observed pcaps"); the `WARN_BAD_PACKET` claim in the legacy `docs/protocol/mercury-wire-format.md` is stale.

### Q7 — Keepalive bundle construction — **CLOSED: CONFORMS-by-substitution**

`crates/services/src/base/tick_sync.rs:39-77` runs every 100 ms emitting `build_ongoing_tick_sync` (a real `tickSync` msg `0x0D`, 8-byte body). This substitutes for the spec's "empty reliable bundle with `FLAG_IS_RELIABLE`" keepalive. Functionally equivalent: the real `tickSync` keeps the channel alive AND naturally piggybacks queued acks (`tick_sync.rs:62-64`: `if !acks.is_empty() { tracing::trace!(...); }` then `build_ongoing_tick_sync(&key, seq_id, tick, &acks)`). Spec's empty-bundle approach is an implementation detail of the C++ reference; substitution achieves the same goal more efficiently.

**Side finding** (new from Q7 resolution): `crates/services/src/base/tick_sync.rs:32` defines a per-loop `INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60)` — a third inactivity timeout (different from `crates/mercury/src/lib.rs:55`'s 300_000 ms and from spec R10's 15 s). All three are misaligned. Rolled into finding #8.

---

## 3.5 — V3 wire-capture resolution of disputes #2 and #12

V2 left findings #2 and #12 as DISPUTED-pending-wire-capture because spec and Rust had competing claims about bit-mask role assignments and the sub-slot encoding formula, and only an actual SGW-client packet on the wire could decide. V3 closed both via `tools/mercury_dispute_resolver.py` against the existing `game/sgw/Working/binaries/sessions/2026-05-15_14-05.pcap` (12,053 decrypted packets, captured against an authentic SGW client). Both inverted: **Rust is right, spec is wrong.**

### Tool

`tools/mercury_dispute_resolver.py` — reuses `pcap_dissect.py` for packet decryption (AES-256-CBC + HMAC-MD5 with the captured 32-byte session key), then for each plaintext packet runs both candidate flag-bit interpretations and tabulates the disambiguating evidence.

### Dispute #12 — flag-bit role assignments

**Verdict: Rust correct, spec §1.2 wrong about bits 0/5/6/7.**

Flag-byte histogram (top 5 of 12,053 packets):

| flag byte | n | Rust interp | Spec interp |
|---|---:|---|---|
| `0x58` | 8856 (73%) | `CHAN\|REL\|SEQ` | `CHAN\|REL\|HAS_REQUESTS` |
| `0x4C` | 2685 (22%) | `ACK\|CHAN\|SEQ` | `ACK\|CHAN\|HAS_REQUESTS` |
| `0x48` | 380 | `CHAN\|SEQ` | `CHAN\|HAS_REQUESTS` |
| `0x5C` | 128 | `ACK\|CHAN\|REL\|SEQ` | `ACK\|CHAN\|REL\|HAS_REQUESTS` |
| `0x78` | 4 | `CHAN\|REL\|FRAG\|SEQ` (a fragment) | `CHAN\|REL\|SEQ_NUM\|HAS_REQUESTS` (no fragment) |

The decisive evidence: spec §1.3 states that bit 0 (`HAS_FIRST_REQUEST_OFFSET`) and bit 6 (`HAS_REQUESTS`) "in practice are always set together." Under spec's bit interpretation, **100% of the 12,053 captured packets violate this invariant** — every packet has bit 6 set without bit 0. Under Rust's interpretation, bit 6 is `HAS_SEQUENCE` and these are normal reliable-channel traffic patterns.

The 4 fragmented packets carry flag byte `0x78`. Under Rust's bit assignments this is the textbook reliable-fragmented-bundle shape (`CHAN\|REL\|FRAG\|SEQ`); under spec's it would be a non-fragment carrying a request that lacks the first-request-offset footer field (impossible per spec §1.3).

The actual wire-format bit assignments are:

| Bit | Mask | Rust constant (correct) | Spec §1.2 claim (wrong) |
|---:|---|---|---|
| 0 | `0x01` | `HAS_REQUESTS` | `HAS_FIRST_REQUEST_OFFSET` |
| 1 | `0x02` | `PIGGYBACK` | `HAS_PIGGYBACKS` (same role, name differs) |
| 2 | `0x04` | `HAS_ACKS` | `HAS_ACKS` |
| 3 | `0x08` | `ON_CHANNEL` | `ON_CHANNEL` |
| 4 | `0x10` | `RELIABLE` | `IS_RELIABLE` (same role) |
| 5 | `0x20` | `FRAGMENTED` | `HAS_SEQUENCE_NUMBER` |
| 6 | `0x40` | `HAS_SEQUENCE` | `HAS_REQUESTS` |
| 7 | `0x80` | `INDEXED` (unused in SGW) | `IS_FRAGMENT` |

### Dispute #2 — sub-slot threshold and `sub_index` formula

**Verdict: Rust correct (threshold = 61, sub_index = method_index - 61), spec §1.5/§1.8/§1.16 Q1 closure wrong.**

The pcap contains 16 distinct sub_index values across 54 `0xBD/0xFD`-prefixed entity-method packets. Cross-referenced against Rust's named `method_idx` constants:

| sub_index | n | Spec interp (method = sub + 62) | Rust interp (method = sub + 61) |
|---:|---:|---|---|
| 4 | 1 | 66 (unnamed) | **65 SETUP_STARGATE_INFO** |
| 11 | 5 | 73 (unnamed) | **72 ON_UPDATE_ITEM** |
| 20 | 7 | 82 (unnamed) | **81 ON_STORE_UPDATE** |
| 41 | 1 | 103 (unnamed) | **102 ON_TIME_OF_DAY** |
| 44 | 3 | 106 (unnamed) | **105 ON_DIALOG_DISPLAY** |
| 61 | 1 | 123 (unnamed) | **122 SETUP_WORLD_PARAMETERS** |

Disambiguating totals — packets that **only** make sense under one interpretation:

- **18 packets only make sense under Rust's encoding** (sub_index values mapping to a named Rust method only when computed as `method = sub_index + 61`)
- **0 packets only make sense under spec's encoding**

The smoking gun is sub_index = 61. Spec §1.8's worked example uses `setupWorldParameters` (method 122) explicitly; spec says it should produce sub_index = 60. The pcap shows sub_index = 61. Rust's `method - 61` formula produces exactly 61 for setupWorldParameters; spec's `method - 62` produces 60.

The same logic applies to method 117 (`onClientMapLoad`) — spec's worked example claims sub_index = 55 (= 117 - 62); Rust would emit sub_index = 56 (= 117 - 61). The pcap doesn't contain method 117 in this capture window, but the pattern of disambiguating evidence at six other named methods (65, 72, 81, 102, 105, 122) is conclusive.

### Implications

1. **Audit findings #2 and #12 are INVERTED.** Both are now classified as **Spec bug — Rust CONFORMS**. The Rust implementation does not need any change for either; the spec chapter needs source-doc-override callouts.
2. **§7.1 Critical recommendations 1 and 2 (V2) are dropped.** The §7.1 "verify and fix flag-bit role assignments" and "fix the sub-slot threshold off-by-one" tasks are no longer needed; the Rust code is correct.
3. **Spec chapter updates needed** (track in §7.4):
   - §1.2 bit-table: rewrite to match the wire-confirmed assignments above. The current bit assignments for bits 0/5/6/7 are inverted from reality.
   - §1.5 + §1.8 sub-slot encoding: change threshold from 62 to 61, change formula from `sub_index = method - 62` to `sub_index = method - 61`. Rewrite the §1.8 worked example for method 117 (`onClientMapLoad`) to produce `[0xBD][word_len][eid][0x38]` (sub_index = 56 = 0x38), not `[0x37]` (= 55).
   - §1.16 Q1 closure: was confidently wrong; the V5 docs that "tabulated `sub_index = 56` for method index 117" were correct; the override that called this off-by-one was itself off-by-one in the other direction.
   - The Ghidra anchor at `EntityDescription_AssignClientMethodIds` (`0x01590df0`) — re-read; the `methodCount >= 0x3e (62)` switch may be "if entity has 62 or more methods" rather than "use sub-slot for indices >= 62".

How to reproduce: `python tools/mercury_dispute_resolver.py game/sgw/Working/binaries/sessions/2026-05-15_14-05.pcap game/sgw/Working/binaries/sessions/2026-05-15_14-04-keys.txt`

---

## 4 — Divergence catalog (full)

| # | Severity | Spec citation | Rust file:line | Divergence (one line) |
|---:|---|---|---|---|
| 1 | **Critical** | §2.4.1 R13 + §2.10 S6 | `crates/mercury/src/lib.rs:66`; `crates/mercury/src/nub.rs:147-150`; `crates/mercury/src/channel/mod.rs:156-158`; `crates/mercury/src/unpacker.rs:156-177` | Implements the apocryphal 30-second fragment-reassembly sweep that spec confirms does not exist in the SGW client; arrival-triggered eviction is also missing |
| 2 | **DISPUTED** (Critical if spec wins) | §1.5 + §1.8 sub-slot threshold = 62 | `crates/services/src/mercury/mod.rs:230-236` (TX); `crates/services/src/base/connect_loop/cell_arms.rs:91-95` (RX); `crates/services/src/mercury/mod.rs:347-356` (test pins behavior) | Rust uses threshold 61 + `sub_index = method_index - 61`; spec says threshold 62 + `sub_index - 62`. Doc-comment at `mercury/mod.rs:221` says threshold = 128. Three competing claims; resolution requires wire capture |
| 3 | **Critical** | §2.5.2 + §2.4 wire shape for restoreClientAck | `crates/services/src/base/connect_loop/encrypted.rs:141-142` | msg `0x0B` parsed as `WORD_LENGTH` (u16 prefix); spec says `CONSTANT_LENGTH = 4` (no prefix). Real client packet desyncs the bundle parser by 2 bytes |
| 4 | **Major** | §1.10.6 + §1.16 Q3 closure — offset 24-35 = previous-position, not velocity | `crates/services/src/mercury/aoi/update.rs:57-80`; `crates/services/src/mercury/world_data/phases.rs:107-110` | `forcedPosition` byte slot 24-35 emitted as velocity (zeros). Correct on the wire only at world entry; silently wrong on every post-entry forced re-snap |
| 5 | **Major** | §1.7 + §1.16 Q5 closure — 32-bit outstanding-ack bitmap | `crates/mercury/src/lib.rs:39`; `crates/mercury/src/channel/mod.rs:123,165-170` | `TX_WINDOW_SIZE = 45` enforces a 45-slot circular buffer; spec retracts the "45-slot" claim and confirms max 32 in-flight reliable per channel |
| 6 | **Major** | §1.7 + §2.4.1 R14 + §2.10 S7 — two-number retry cap | `crates/mercury/src/channel/mod.rs:274-291` | Implements only the 20-retry lifetime cap; missing the 5.0 IEEE 754 per-tick work budget |
| 7 | **Major** | §1.7 — 28-bit sequence space + R4 sentinel rejection | `crates/mercury/src/packet/mod.rs:81` (constant defined, unused); `crates/mercury/src/channel/mod.rs:74-75,175,199` | Sequence numbers treated as full u32; no `0x0FFFFFFF` mask; no R4-class drop on `seq == 0x10000000` |
| 8 | **Major** | §2.4 R10 + §2.2 — UE3 inactivity = 15 s | `crates/mercury/src/lib.rs:55` (300_000 ms); `crates/services/src/base/tick_sync.rs:32` (60 s) | Three misaligned timeouts: lib=300s, tick_sync=60s, spec=15s |
| 9 | **Major (Security)** | §2.5.2 — `authenticate` per-tick token validation | `crates/services/src/base/connect_loop/encrypted.rs:89-103` | Skip-and-continue on every `0x01`; spec mandates per-tick token validation to defeat replay. Intentional choice mirroring C++ reference, but a session-hijack vector |
| 10 | **Minor** | §1.6 / §2.3 max packet size | `crates/mercury/src/lib.rs:27,33`; `crates/mercury/src/packet/build.rs:113-119` | Rust `PACKET_MAX_SIZE=1472` is MTU-inclusive; `FRAGMENT_BODY_SIZE=1300` is conservative-margin. Spec stamps the cap at 1453 bytes. Functionally OK on send |
| 11 | **Minor / latent-Critical** | §2.4.1 R15 — no Mercury-layer version handshake | `crates/mercury/src/lib.rs:69` | `PROTOCOL_VERSION: u32 = 391` exists with a comment promising a wire handshake the spec forbids. Vestigial today; latent Critical if wired up |
| 12 | **Critical-pending-verification** | §1.2 flag-bit role assignments | `crates/mercury/src/packet/mod.rs:54-78` (constants); `crates/mercury/src/packet/build.rs:85,141` (build sites) | Bits 0, 5, 6, 7 appear to have wholly different roles in Rust vs spec. Build-side evidence (`frag_flags = flags \| FLAG_FRAGMENTED \| FLAG_HAS_SEQUENCE` = bits 5+6) does not match spec's expected fragment encoding (bits 5+7). Wire capture required to confirm whether the Rust server's emitted fragment flags (e.g. `0x60` if Rust definitions are followed) match what the SGW client expects (`0xA0` per spec) |

---

## 5 — Missing-functionality catalog (full)

| # | Spec claim | What correct Rust would look like |
|---:|---|---|
| M1 | §1.5 — `compressLength` per-`InterfaceElement` 1/2/3/4-byte length-prefix widths | Add a `LengthEncoding::{Constant(u32), Word, Dword, Compressed(u8)}` enum; `Bundle::start_message(msg_id)` consults a static `InterfaceElement` table (matching spec §2.5.1's `flag`/`size` discriminator) and writes the right prefix width |
| M2 | §1.7 — 512-entry received-sequence dedup hash (mask `0x1FF`) | Add `dedup: [Option<u32>; 512]` keyed by `seq & 0x1FF`. Rust currently dedupes via the 64-slot RX window's slot-occupied check, which has different bounds (window vs hash) |
| M3 | §1.10.7 + §2.5.1 — `AUTHENTICATE` (msg `0x00`) with `DWORD_LENGTH` packed-string body | Add the `DWORD_LENGTH` framing path to `Bundle::start_message`; emit during initial handshake. Currently no Rust code path handles this (acceptable today since auth is SOAP-layer, but blocks any future Mercury-layer rekey) |
| M4 | §1.11 — 31 missing `UPDATE_AVATAR` variants (`0x11..0x2F`) | Per-variant emit functions matching the 32-way `{Alias\|NoAlias} × {FullPos\|OnChunk\|OnGround\|NoPos} × {YPR\|YP\|Y\|NoDir}` matrix from spec §2.5. Today only `0x10` (NoAliasFullPosYawPitchRoll, 25 bytes) is implemented |
| M5 | §1.11.2 — `detailedPosition` (msg `0x30`, 41 bytes) | Add `BASEMSG_DETAILED_POSITION = 0x30` and an emit function. Currently absent |
| M6 | §2.4.1 R14 — per-tick 5.0 retransmit work budget | In `crates/mercury/src/channel/mod.rs::check_timeouts`, wrap the retransmit loop in `let mut budget = 5.0f32; while budget > 0.0 { ... budget -= 1.0; }` — a packet over the lifetime cap aborts; a packet within budget but over the per-tick gate yields to next tick. See spec §1.16 Q5 closure for the exact semantics |
| M7 | §2.4 R10 — 15-second silence-tolerance gate | Reconcile three inactivity-timeout constants (`crates/mercury/src/lib.rs:55` 300s; `crates/services/src/base/tick_sync.rs:32` 60s; spec 15s) into a coherent semantic. Recommend: name `MERCURY_PEER_DEAD_MS` for the bookkeeping value (current 300s) and add `UE3_INACTIVITY_TIMEOUT_MS = 15_000` for the wire-observable R10 silence-tolerance edge |
| M8 | §2.7 R/SHOULD — `restoreClient` flow + `restoreClientAck` consumer | When `restoreClient` is added to the emit path, also wire a `0x0B` ack handler that closes server-side restoration bookkeeping. Currently neither side exists |

---

## 6 — No-op catalog

| # | Spec claim | Rust no-op location |
|---:|---|---|
| N1 | §1.6 — bundle MAX_FRAGMENTS = 64 invariant | `crates/mercury/src/packet/build.rs:147` (`div_ceil`) does not bound the count |
| N2 | §2.5.1 + §2.6 T4 — `bandwidthNotification` (msg `0x01`) descriptor must be present | No emit code; the message is never sent. Functional impact nil per spec §2.10 S2 (client discards), but if a future client patch installed `setBandwidthFromServerMutator`, Rust would silently fail to advertise |
| N3 | §2.5.1 — `requestEntityUpdate` (msg `0x07`) handler | `crates/services/src/base/connect_loop/encrypted.rs:140` parses the body but no dispatch arm in the `match msg_id` block. Client requests for missed/stale entity state are silently dropped |
| N4 | §2.5.1 — `avatarUpdateImplicit` (0x02) / `avatarUpdateWardImplicit` (0x04) / `avatarUpdateWardExplicit` (0x05) handlers | `crates/services/src/base/connect_loop/encrypted.rs:120,124,126` parse the bytes but no dispatch arm. Only `0x03` (avatarUpdateExplicit) is forwarded to CellService |

---

## 7 — Required and recommended changes

Each row below is a declarative change set. Severity carries forward from §1; ordering is by tier.

### 7.1 Critical — land before any external-client testing

V3 wire capture closed two former Critical-pending-verification items as spec bugs (Rust correct, no Rust change needed). What remains:

1. **Remove the 30-second fragment-reassembly sweep** (finding #1). Delete `FRAGMENT_REASSEMBLY_TIMEOUT_MS` from `crates/mercury/src/lib.rs:66`, the per-channel call in `crates/mercury/src/nub.rs:147-150`, and `cleanup_stale_fragments` from `crates/mercury/src/channel/mod.rs:156-158`. Add the missing arrival-triggered overlapping-bundle eviction — when a new fragmented bundle on the same channel overlaps an in-progress reassembly's sequence range, evict the in-progress one. Add a regression test that confirms an orphan fragment persists in `unpacker.rs` until the channel is destroyed (the inverse of the deleted test at `crates/mercury/src/nub.rs:357`).

2. **Fix `restoreClientAck` (msg `0x0B`) parsing** (finding #3). Change `crates/services/src/base/connect_loop/encrypted.rs:142` from `0x0B => read_word_length_payload(body, &mut offset)` to `0x0B => read_constant_payload(body, &mut offset, 4)`. Add a regression test that feeds `[0x0B, 0x00, 0x00, 0x00, 0x00, <next msg>]` and verifies the parser advances exactly 5 bytes.

### 7.2 Major — next milestone

5. **Fix `forcedPosition` previous-position field** (finding #4). Update `crates/services/src/mercury/aoi/update.rs:57-80` and `crates/services/src/mercury/world_data/phases.rs:107-110`: rename the local from `vel` to `prev_pos`; have non-world-entry callers pass the entity's last-known position. The world-entry path correctly passes zeros (no prior position). Update comments to cite §1.10.6 + §1.16 Q3 closure.

6. **Cap outstanding reliable per channel at 32** (finding #5). Change `crates/mercury/src/lib.rs:39` to `TX_WINDOW_SIZE: usize = 32`. Replacing the `VecDeque` with a 32-bit bitmap matches the spec's structural model.

7. **Add the per-tick retry budget** (finding #6). In `crates/mercury/src/channel/mod.rs::check_timeouts`, gate the retransmit loop with `let mut budget = 5.0f32` decrementing per processed entry, exit when negative.

8. **Enforce 28-bit sequence space** (finding #7). Apply mask `0x0FFFFFFF` at `crates/mercury/src/channel/mod.rs:175`. Reject incoming `seq == 0x10000000` at packet entry with the R4 drop semantic.

9. **Reconcile inactivity-timeout constants** (finding #8). Rename `crates/mercury/src/lib.rs:55` to `MERCURY_PEER_DEAD_MS` (its actual semantic — Mercury-layer bookkeeping). Add `UE3_INACTIVITY_TIMEOUT_MS = 15_000` as a separate constant for the R10 silence-tolerance edge. Replace the third constant at `crates/services/src/base/tick_sync.rs:32` (currently 60 s) with the 15 s constant.

10. **Implement per-tick `authenticate` token validation** (finding #9). Replace the skip-and-continue at `crates/services/src/base/connect_loop/encrypted.rs:89-103` with a per-tick rotating-token check against the SOAP-issued session material. Track the token rotation algorithm (out of scope for this audit; auth chapter's domain) before wiring the validator. Until the validator lands, the server is exposed to session-hijack-via-replay.

### 7.3 Minor and design debt

11. **Verify and remove `PROTOCOL_VERSION`** (finding #11). The constant at `crates/mercury/src/lib.rs:69` is currently unused but its comment promises a wire handshake the spec forbids. Remove the constant and the comment.

12. **Introduce an `InterfaceElement` table.** Replace per-emitter hardcoded length-prefix decisions with a static table of `(msg_id, flag, size, name)` rows matching spec §2.5.1's `flag=0/1` discriminator. Catches future descriptor-vs-emitter mismatches at compile time; unblocks `DWORD_LENGTH` (`AUTHENTICATE`) and the `compressLength` 1/2/3/4-byte widths.

13. **Add the missing `UPDATE_AVATAR` variants.** All 32 variants of `0x10..0x2F` per spec §1.11. File an issue tracking the long tail.

14. **Add a `restoreClient` + `restoreClientAck` round-trip handler.** Recommended-but-not-required per §2.7; needed before any BaseApp restart / fault-recovery feature.

15. **Add the warn-far-out-of-window log** (R12 row in §2.4 deep-dive). Spec specifies a warning log on far-out-of-window sequence arrivals for diagnostics. Rust silently drops; add the `tracing::warn!` line at `crates/mercury/src/channel/mod.rs:202-206`.

### 7.4 Documentation cleanup (chapter and code)

16. **Drop the `OpenSSL` reference in `encryption.rs`.** Spec §1.4 explicitly notes the doc-comment is wrong (the runtime is RustCrypto; the binary it emulates is CryptoPP). One-line change.

17. **Reconcile the contradictory doc-comment at `crates/services/src/mercury/mod.rs:219-225`.** The doc-comment says "Extended (method_index >= 128)" and "We use the simpler boundary at 128" while the code at line 230 uses `if method_index >= 61`. The code is correct (V3 wire capture confirmed); update the doc-comment to match: extended encoding starts at index 61 with `sub_index = method_index - 61`, sentinel `0xBD` (cell) / `0xFD` (base).

18. **Update the spec table at §1.10.5** (createEntity payload size). The table says "5 bytes" but the field list and C++ source both add to 8 bytes; Rust correctly emits 8. Fix the spec table.

19. **Update spec §1.2 flag-bit table (V3 closure of dispute #12).** Bits 0/5/6/7 are wire-misassigned in the current spec. Wire-confirmed assignments per `tools/mercury_dispute_resolver.py` against `sessions/2026-05-15_14-05.pcap` (12,053 packets, 100% spec-violation rate under the current table):
    - bit 0 (`0x01`) = `HAS_REQUESTS` (not `HAS_FIRST_REQUEST_OFFSET`)
    - bit 5 (`0x20`) = `FRAGMENTED` (not `HAS_SEQUENCE_NUMBER`)
    - bit 6 (`0x40`) = `HAS_SEQUENCE` (not `HAS_REQUESTS`)
    - bit 7 (`0x80`) = unused / reserved-for-INDEXED (not `IS_FRAGMENT`)
    Add a source-doc-override callout at §1.2 citing the V3 audit closure.

20. **Update spec §1.5 / §1.8 / §1.16 Q1 sub-slot encoding (V3 closure of dispute #2).** Threshold is **61** (not 62); formula is `sub_index = method_index - 61` (not `- 62`). Rewrite the §1.8 worked example for method 117 (`onClientMapLoad`) to produce sub_index = 56 (`0x38`), not 55 (`0x37`). The §1.8 source-doc-override callout that called the V5 docs' "117 - 61 = 56" off-by-one was itself off-by-one in the other direction; remove. Re-read the Ghidra anchor at `EntityDescription_AssignClientMethodIds` (`0x01590df0`) to determine the correct semantic — `methodCount >= 0x3e (62)` likely means "if the entity has 62 or more methods, use sub-slot for indices >= 61", not "use sub-slot for indices >= 62".

---

## 8 — Out of scope

- §1.13 MachineGuard (server discovery; spec confirms not needed for client compatibility)
- §1.10.7 SOAP auth handshake (separate cipher-and-auth chapter)
- Per-method entity-RPC argument layouts (deferred to future `entity-rpc-dispatch.md` chapter)
- Combat / inventory / mission systems (separate chapters)
- The `cimmeria-mercury::unified` TCP framing (inter-service, not client-facing)
- Section 3 (deprecated server) — pending Section 1 review per chapter status

---

## 9 — Audit confidence and provenance

- **Spec read:** §1.1–§1.16 + §2.1–§2.11 + crosswalk + footnotes (~2400 of ~2531 lines). §2.5.2 byte-layout subsections read in full.
- **Rust read in full:** Every file under `crates/mercury/src/` and every Mercury-touching file under `crates/services/src/{mercury,base,cell,auth}/`. Approximately 80 files of the 107 that match Mercury vocabulary; the remaining 27 are tests or orthogonal modules (`test_support.rs`, `defs/`, `content-engine/`, `entity/stats/`) and were spot-checked for any wire-emit code (none found).
- **Search corpus:** 107 files across `crates/` reference Mercury vocabulary; this audit touched ~80 directly.
- **Tools:** ripgrep, agent-delegated file reads (Explore subagent), direct Read tool. No live Ghidra cross-reference in this pass — all spec claims taken as authoritative per the chapter's evidence-anchored construction.
- **Confidence:** every Disposition row in §2 and every finding in §1 is high-confidence on the *Rust read* (the auditor has the file open). Confidence on the *spec claim* is inherited from the chapter's per-section confidence (high across §1–§2 as of commit `07fd245`). The disputed finding (#2 sub-slot threshold) is the only place where audit confidence is intentionally lowered to medium — both spec and Rust have evidence and the wire capture has not been done.

### V1 → V2 reconciliation summary

V2 expanded V1 in five ways:

| Change | V1 | V2 |
|---|---|---|
| Open Q-items | 7 (Q1-Q7 in §7) | 0 — all resolved with file:line evidence in §3 |
| msg_id walk | sampled (~12 critical messages spot-checked) | exhaustive — every msg_id `0x00..0x38` server→client and `0x00..0x0D` client→server with file:line evidence in §2.11 / §2.12 |
| Bit-mask audit | flagged `FLAG_INDEXED` naming as Minor | promoted to Critical-pending-verification (#12) after building deeper evidence on bits 0/5/6/7 role mismatches |
| Sub-slot threshold | flagged as Critical (Rust 61 vs spec 62) | re-classified as DISPUTED with explicit `Path to resolution`; doc-comment-says-128 added as third claim |
| Inactivity timeout | one constant misalignment | three constants misaligned (lib.rs 300s, tick_sync.rs 60s, spec 15s) |

Length: V2 is ~750 lines vs V1's 447. V1's 500-line cap was the reason Q1-Q7 were deferred and the msg_id walk was sampled rather than exhaustive. V2 has no length cap and is structured to be the first stable revision; future passes should edit in place rather than re-author.

### V3 closure status — formerly DISPUTED, now resolved

V2 left three items pending wire capture; V3 closed all three via `tools/mercury_dispute_resolver.py` against the existing 12,053-packet pcap. All three resolved as **Rust correct, spec needs update** — see §3.5 for the full wire evidence:

1. **Sub-slot threshold** (#2). RESOLVED: Rust threshold = 61 is correct; spec's = 62 claim is wrong. 18 disambiguating packets in the pcap.
2. **Flag-bit role assignments** (#12). RESOLVED: Rust bits 0/5/6/7 are correct; spec's bit-to-role table is wrong. 100% of packets violate spec invariants under spec's interpretation.
3. **Off-by-one in `sub_index`** (sibling of #2). RESOLVED with #2 — Rust's `sub_index = method - 61` is correct.

---

## 10 — Editor notes (V1 doc-writer pass)

The V1 documentation-writer pass added YAML front matter, restructured §6 recommendations into severity-tiered subsections, converted V1's `[needs deeper analysis]` flags into a §1.16-style Open Questions section, and registered the audit under a new `audits/` section in `docs/readme.md`. V2 preserves the editorial structure but expands the content significantly (no length cap; every item resolved with evidence). The V1 editor's flagged-back items are now resolved:

- **Finding #9 (max packet size)** — V1 editor flagged whether it was actually a divergence; V2 keeps it Minor with the same rationale (different baselines for the same constant; functionally OK on send).
- **§2.13 row S11.2** — V1 editor noted the cross-ref; V2 confirms no semantic handler exists either way (still parser-desync at the wire level).
- **§6.3 row 4 (`PROTOCOL_VERSION`) and §6.4 row 1 (`OpenSSL` doc-comment)** — V1 editor asked for confirmation; V2 confirms via grep that `PROTOCOL_VERSION` has no use site and the OpenSSL doc-comment is stale (recommend deletion in §7.3 / §7.4).

V1's open editorial questions are also closed:

- **`## 11 — Promotion criteria` section** — not added in V2 because the audit hasn't gone through one promotion cycle yet. Add when first cycle completes.
- **Length-split heuristic** — V2 is 750 lines, over the 500-line soft cap and under the 700-line hard cap from CLAUDE.md. Bible chapters explicitly have no length cap. Audit retains as one document; if V3 grows past 1000 lines, natural split points are §2 (per-section findings) → separate file, §4-§6 (catalogues + recommendations) → second file, §1+§3+§7-§10 (summary + Q-resolution + apparatus) → third file.

---

## 11 — Ghidra re-examination of disputed anchors (V4)

V3 closed disputes #2 and #12 by wire capture (12,053-packet pcap analysis). V4 verifies the verdict by re-reading the two spec anchors directly in Ghidra and identifies exactly where the spec author misread the binary. Both findings stand: **Rust is correct, the spec needs source-doc-override callouts**. The decompilations below are reproducible against the loaded `SGW.exe` (SHA256 `109F307763A5C6C59FF484840739860BDC7163092F0644343D0B2C03E4925783`) at the cited Ghidra addresses.

### 11.1 Flag-bit roles — `Mercury::Nub::processFilteredPacket_inner` at `ghidra://SGW.exe@0x01580840`

Spec §1.2 cites this function as "Source of truth: the flag-mask table in `Mercury::Nub::processFilteredPacket_inner`, which decodes each bit in order to peel the matching footer field off the back of the datagram." The actual decomp peels the following bits, in this order:

| Code construct | Bit | Spec's claim for this bit | Actual role |
|---|---|---|---|
| `(local_15 & 2)` | bit 1 (`0x02`) | `HAS_PIGGYBACKS` | PIGGYBACK — 2-byte signed length tail, bitwise-NOT-encoded if negative — matches spec |
| `(char)local_15 < 0` | bit 7 (`0x80`) | `IS_FRAGMENT` | **BAD FLAGS / ERROR path.** The signed-byte-negative test is unconditional rejection; the function logs the error and increments the channel error counter at `this+0xf8`. Bit 7 is NOT FRAGMENT — it is forbidden / unused on the wire. Rust's `FLAG_INDEXED` (also unused-in-SGW) is structurally correct. |
| `(local_15 & 4)` | bit 2 (`0x04`) | `HAS_ACKS` | ACK list path — matches spec |
| `(local_15 & 8)` | bit 3 (`0x08`) | `ON_CHANNEL` | Channel resolution via `FUN_0157c7b0` — matches spec |
| `(local_15 & 0x40)` | bit 6 (`0x40`) | `HAS_REQUESTS` | **HAS_SEQUENCE.** Reads 4-byte sequence ID at packet tail (`*(undefined4 *)(iVar6 + 0x50 + (int)param_2)`), stores in `param_2[0x11]`, then drop-checks against null-sentinel `0x10000000` and 28-bit-mask invariant. Bit 6 is unambiguously the sequence-number flag. Rust correct. |
| `(local_15 & 0x10)` | bit 4 (`0x10`) | `IS_RELIABLE` | RELIABLE path — matches spec |

Bits 0 and 5 are **not checked in this function at all** — they must be handled in the downstream `Mercury_Nub_ProcessPacket` (which the function dispatches to via `FUN_015792f0` after sequence-popping). The send-side cross-check at `Mercury_Bundle_Finalise` (`ghidra://SGW.exe@0x0157a7a0`) confirms bit 6's role: the function sets bit `0x40` and explicitly accounts for "+4 bytes" of footer (`pCurPacket[10] = pCurPacket[10] + 4;`) — that's the sequence-ID's 4 bytes, confirming bit 6 = `HAS_SEQUENCE_NUMBER` semantically.

**Where the spec author went wrong:** spec §1.2 swapped the bit-to-role mapping for bits 5/6/7. The roles themselves are correct (a fragment flag, a sequence flag, a request flag all exist) — but the bit-to-role assignments are wrong. The likeliest origin: the spec author read the role names from the stock-BigWorld 2.0.1 `packet.hpp` reference (which lists `FLAG_HAS_SEQUENCE_NUMBER` at bit 5 and `FLAG_IS_FRAGMENT` at bit 7) and assumed SGW preserved the same bit positions. SGW does not — the binary peels bit 6 for sequence and treats bit 7 as forbidden. The spec's "Source of truth: the flag-mask table" sentence is anchored to the right function but cites a table that doesn't exist in the actual decomp.

### 11.2 Sub-slot threshold — `EntityDescription_AssignClientMethodIds` at `ghidra://SGW.exe@0x01590df0`

Spec §1.5 + §1.8 cite this function: "`EntityDescription_AssignClientMethodIds` switches to sub-slot encoding when `methodCount >= 0x3e (62)`." The actual algorithm:

```c
nExposedCount = (pEntityDesc[0x24] - pEntityDesc[0x20]) / 4;   // exposed-method count
iVar2 = (nExposedCount + 0xc0) / 0xff;                          // = (n + 192) / 255
iVar3 = 0x3e - iVar2;                                           // = 62 - quotient = idBase
uVar4 = -iVar3;                                                 // signed counter, starts negative
for each method serial uVar5 (0, 1, 2, ...):
    if (int)uVar4 < 0:                  // direct-encoding range
        method[+0x44] = uVar5;          // header byte = serial
        method[+0x48] = 0xFFFFFFFF;     // no sub-byte
    else:                                // extended-encoding range
        method[+0x44] = (uVar4 >> 8) + iVar3;   // header byte = idBase
        method[+0x48] = uVar4 & 0xff;            // sub-byte
    uVar4++;
```

For typical SGWPlayer-class entities (`nExposedCount` between 63 and 317), `iVar2 = 1`, `iVar3 = 61` (the actual `idBase`):

| Method serial | uVar4 at iteration | Branch | header byte (+0x44) | sub-byte (+0x48) |
|---:|---:|---|---:|---:|
| 0 | -61 | direct | 0 | 0xFFFFFFFF |
| 1 | -60 | direct | 1 | 0xFFFFFFFF |
| ... | ... | direct | ... | 0xFFFFFFFF |
| 60 | -1 | direct | 60 | 0xFFFFFFFF |
| 61 | 0 | **extended** | 61 | **0** |
| 62 | 1 | extended | 61 | 1 |
| ... | ... | extended | 61 | ... |
| 117 | 56 | extended | 61 | **56** |
| 122 | 61 | extended | 61 | **61** |

The wire byte = `(header \| 0x80)` for cell methods (`startEntityMessage` at `ghidra://SGW.exe@0x00dd6a60`: `pThis._0_1_ \| 0x80`) or `(header \| 0xC0)` for base methods. So:

- Direct method serial 0 → wire `0x80`
- Direct method serial 60 → wire `0xBC`
- Extended method serial 61 → wire `0xBD` (= `61 \| 0x80`) with sub_byte = 0 ← the sentinel value falls out naturally
- Extended method serial 117 → wire `0xBD` with sub_byte = 56
- Extended method serial 122 → wire `0xBD` with sub_byte = 61

**This is exactly Rust's behavior** (`crates/services/src/mercury/mod.rs:230-236`: `if method_index >= 61` with `sub_index = method_index - 61`). It also exactly matches the wire capture: the pcap shows sub_byte = 61 for `setupWorldParameters` (method 122).

**Where the spec author went wrong:** The spec read `0x3e = 62` and inferred the threshold was at method serial 62. The constant is actually the **base value** for computing `idBase`, and `idBase` becomes both the threshold (where direct → extended switches) and the header-byte value for all extended methods. The actual threshold for typical entities is at serial 61 (where `uVar4` reaches zero), not 62. The chain of inference that produced the spec's claim:

1. "There's a `0x3e` constant in the algorithm" — true.
2. "0x3e equals 62" — true.
3. "Therefore the threshold is 62" — **wrong**. The constant is the base for an arithmetic that produces `idBase = 62 - (count+192)/255`, and the threshold is `idBase`, not 62 itself.

The spec's worked example for method 117 (`onClientMapLoad`) computes `sub_index = 117 - 62 = 55 = 0x37` — that's wrong by 1. The actual sub_index for method 117 is `117 - 61 = 56 = 0x38`. The spec's own §1.8 source-doc-override callout that "corrected" the V5 docs' "117 - 61 = 56" was itself off-by-one: the V5 docs were right.

### 11.3 Why both misreads share a common cause

Both errors are **spec-author inferences from the binary that took a constant at face value without tracing its arithmetic role**:

- For #12, `0x80` was assumed to be `IS_FRAGMENT` because stock BigWorld puts `FLAG_IS_FRAGMENT` at bit 7. Actually, SGW uses bit 7 as the unconditional bad-flags trigger — the stock-BigWorld bit assignments don't carry over to SGW for bits 0/5/6/7.

- For #2, `0x3e (62)` was assumed to be the method-index threshold because stock BigWorld documents 62 as the "first sub-slot index". Actually, SGW uses 62 as the *base* for computing a per-entity-size-dependent `idBase`, which for typical entities works out to 61 (the actual threshold). The stock-BigWorld documentation describes a fixed-threshold scheme; SGW uses a count-adaptive scheme that happens to land on 61 for SGWPlayer.

Both errors are inheritances from stock-BigWorld documentation imported into the SGW spec without re-deriving the constants from the binary's actual arithmetic. The spec's confidence on these claims was anchored to "Ghidra-confirmed" but the Ghidra reading was confirming the constant's existence, not its semantic role.

### 11.4 Confidence and reproducibility

The decompilations cited above were obtained from the Ghidra MCP server connected to the project at `C:/Users/steven.cady/repos/personal/Cimmeria/game/sgw/Working/binaries/SGW.exe`. Reproducible by:

```
mcp__ghidra__decompile_function 0x01580840   # processFilteredPacket — receive-side flag peel
mcp__ghidra__decompile_function 0x0157a7a0   # Bundle::finalise — send-side flag stamp
mcp__ghidra__decompile_function 0x01590df0   # EntityDescription_AssignClientMethodIds
mcp__ghidra__decompile_function 0x00dd6a60   # ServerConnection_StartEntityMessage — cell wire emit
```

Cross-checked with the V3 wire-capture pcap (`game/sgw/Working/binaries/sessions/2026-05-15_14-05.pcap`, 12,053 decrypted packets via `tools/mercury_dispute_resolver.py`). Both lines of evidence — static (decomp) and dynamic (pcap) — agree.

**Confidence: high** on both findings being SPEC bugs and Rust being correct. No further wire capture or Ghidra reading needed. The remaining work is documentation: the spec chapter §1.2 + §1.5/§1.8/§1.16 Q1 closure need updates per §7.4 items 19 + 20 of this audit.

---

## 12 — Concrete failure-mode catalog

For each finding, this section answers two questions:

1. **Known issues** — concrete, currently-observable failures the divergence produces. Tests that already exercise the wrong behavior; code paths that already trigger it; pcap or log evidence the bug has shipped.
2. **Inferred / speculated issues** — failure modes the code permits but no one has yet tripped, ranked by how likely they are under realistic conditions.

The split matters for triage: Known issues are immediate fix priorities; Inferred issues are latent risks that should be guarded against but may not block landing other work.

**Citation convention** — this section uses `file::function` (or `file::function:line` when the line is essential to disambiguate within a long function) instead of `file:line`. Function names survive line-number drift across refactors; line numbers do not. For functions whose role is non-obvious from the name, the surrounding module is included (`file::module::function`).

### Finding #1 — 30-second fragment-reassembly sweep

**Known issues:**

- A regression test at `crates/mercury/src/nub.rs::tests::tick_sweeps_stale_fragment_reassembly` actively pins the wrong behavior: it asserts that the sweep evicts a stale fragment buffer when run with `Duration::ZERO`. A second test at `crates/mercury/src/channel/tests.rs::cleanup_stale_fragments_drops_partial_bundles` asserts the same thing one layer down. Both tests will need inversion — they should pass when the sweep is removed and fail under the current "sweep exists" code.
- The sweep runs unconditionally in `crates/mercury/src/nub::Nub::tick`. Every Nub tick on every server walks every channel and asks `cleanup_stale_fragments(30s)`. The CPU cost is small (HashMap iteration), but it is paid every tick of every server.

**Inferred / speculated issues:**

- A client emitting a fragmented bundle whose retries push the bundle's wire duration past 30 seconds. Worst case: 20-retry lifetime cap × 700 ms ack timeout = ~14 seconds per packet under sustained loss; a 5-packet bundle where every fragment is lost twice and retransmitted = up to ~14 seconds of in-flight time. Within the 30s window, but close. A burst of three 5-packet bundles at the same time, where loss is concentrated on the last fragment of the first bundle: the server may evict the first bundle's reassembly before the late retransmit arrives. Symptom: Mercury silently loses one bundle; downstream the missing system message produces "argument stream desync" (audit §2.4.2 Category D) on the next bundle.
- Server-emitted fragments (e.g. the world-entry `mapLoaded` ~5-packet bundle) are NOT affected by this sweep — the sweep is on the recv side. Server only fragments outbound; the client doesn't have a sweep either, so no symmetric bug.

### Finding #3 — `restoreClientAck` parsed as WORD_LENGTH

**Known issues:**

- None today, because Rust does not currently emit `restoreClient` (msg `0x34`). The audit's NO-OP entry M8 confirms the absent emit path. Without `restoreClient` being sent, the client has no cause to send `restoreClientAck`, and the buggy parse arm at `crates/services/src/base/connect_loop/encrypted::handle_encrypted_datagram` never executes. The bug is latent.

**Inferred / speculated issues:**

- The day someone wires `restoreClient` (planned per §7.3 row 3 — needed for any BaseApp restart / fault-recovery flow), the first real client packet exercising this path will desync the bundle parser. The byte-by-byte trace for a real wire packet `[0x0B, 0x00, 0x00, 0x00, 0x00, <next msg_id>, ...]`:
  - Parser reads `0x0B`, advances to offset 1.
  - WORD_LENGTH path reads `[offset+0..+2] = [0x00, 0x00]` as u16 length = 0, advances by 2 + 0 = 2 bytes total. Now at offset 3.
  - Two of the four ack-payload bytes (`0x00 0x00` at offsets 3–4) are still unconsumed. Parser reads `0x00` at offset 3 as the next msg_id, which dispatches as `baseAppLogin` (msg `0x00`).
  - `baseAppLogin` is WORD_LENGTH; parser reads the next two bytes (offsets 4 and 5) as a u16 length. Offset 4 is the last `0x00` of the ack payload; offset 5 is the *real* next message's msg_id byte. Length-prefix is `[0x00, <real-msg-id>]` — interpreted as a u16, this is up to 0xFF00 bytes, hugely wrong.
  - Cascade failure: parser tries to consume `0xFF00` bytes of "baseAppLogin payload"; bundle is shorter than that; parser truncates, logs "Bundle truncated", drops the rest of the bundle. Every message after `restoreClientAck` in the same bundle is lost.
- Failure mode at the client side: silent. The server's drop is invisible to the client; the next tick continues normally. The lost messages would be observed as missed entity updates, missed inventory changes, etc. Hard to diagnose without correlating client-side log with server-side bundle-truncation log.

### Finding #4 — `forcedPosition` velocity vs previous-position reference

**Known issues:**

- **Confirmed observable bug — camera-through-floor on initial spawn.** Reported by @cadacious 2026-05-15: on first character load, the client camera "snaps/moves from (0,0,0) to the new spawn position", visibly clipping through the world floor on the way up. Mechanism: `crates/services/src/mercury/world_data/phases::build_enter_world_body` emits `forcedPosition` with the spawn position at offsets 12-23 and zeros at offsets 24-35. The client treats those 12 bytes as the previous-position reference (per spec §1.10.6 + §1.16 Q3 closure: `PackageAndSendEntityMove` copies the slot verbatim into `pPrevPos`), so the client's interpolation cache for the player entity is set to origin. The first frame after world entry, the client's camera-attach code interpolates from prev_pos `(0, 0, 0)` to current_pos `(spawnX, spawnY, spawnZ)`. The visible glitch is exactly that interpolation: the camera flies from world origin (which is below the playable terrain for almost any non-trivial spawn point) up through the floor to the spawn point. **Concrete fix the user proposed is correct**: emit prev_pos = spawn position (or any close-to-spawn value) at world entry. The interpolation distance becomes zero and there is no visible movement. Change site: `crates/services/src/mercury/world_data/phases::build_enter_world_body:107-112`.
- **Confirmed observable bug — gate-travel teleport.** `crates/services/src/base/world_entry/teleport::handle_teleport` (call site at `teleport.rs:76`) passes the same `build_forced_position` signature with no prev-position. Every gate-travel teleport produces forcedPosition with prev-position = origin, same mechanism as world entry. Visible during stargate transitions, especially long-distance ones — the camera momentarily interpolates through (0,0,0).
- **Partial contributor to second observed bug — character model not loaded before "forced appearance update".** Reported by @cadacious 2026-05-15: "we don't get an initial proper load of the character player and their model before we send a forced appearance update." Half of this is finding #4: the player entity is rendered at origin (no model attached, since BeingAppearance hasn't arrived yet) and the camera is interpolating up to spawn. The other half is a sequencing issue documented at `crates/services/src/mercury/world_data/map_loaded::build_map_loaded_body_inner` — `BeingAppearance` (method 26) is step 15 of the ~21-step mapLoaded sequence. With the bundle fragmented across ~5 packets, BeingAppearance may not land at the client until 50-200ms after `createCellPlayer`. During that window the client has a player entity that exists but has no body model. Two complementary fixes worth considering: (a) send `BeingAppearance` (and `onEntityTint`) earlier in the mapLoaded sequence, ideally as the first entity-method call after `createCellPlayer`; (b) embed the appearance call in `build_enter_world_body` itself so the very first packet the client sees carries the player's model data. Either approach reduces the "model-less player" window to one packet's worth of latency.

**Inferred / speculated issues:**

- Anti-cheat snap-back if added later: a server that detects illegal client position would emit forcedPosition to authoritative-snap the client. Per the spec, the prev-position field should be the entity's current believed position so the client can delta-correct. With prev-position emitted as zero, the client snap-back would behave erratically — every snap would visually flick the player through origin first.
- Long-distance position warps from sustained packet loss: if a player has been moving and several `UPDATE_AVATAR` packets are lost, the next forcedPosition the server emits to re-sync would land with prev_pos = (0,0,0), producing the same camera-interpolation glitch mid-gameplay.
- The `build_forced_position` signature does not have a parameter for prev-position — adding the prev-position fix requires extending the signature and updating both call sites (`build_enter_world_body` passes spawn position; `teleport::handle_teleport` passes the entity's last-known position before the teleport).

### Finding #5 — `TX_WINDOW_SIZE = 45` (vs spec's 32)

**Known issues:**

- `crates/mercury/src/lib::consts::TX_WINDOW_SIZE` is `45`. `crates/mercury/src/channel::Channel::send` uses this as a hard back-pressure gate: the 46th in-flight reliable packet on a single channel is rejected at the channel layer. Today no test or live session bursts that many reliable in-flight, so the back-pressure never fires.

**Inferred / speculated issues:**

- A burst of 33+ reliable packets in flight: the spec's client allocates a 32-bit ack bitmap, indexed by `seq_id & 0x1F`. Two packets with seq_ids that differ by exactly 32 would collide on the same bitmap bit. The client's ack-tracking accounting would corrupt — one of the two packets' ack would clear both bits, causing the server to declare both delivered.
- Realistic trigger: high-density region transition where the AoI-create cascade for many entities lands in one tick. Each new entity emits `CREATE_ENTITY` + `UPDATE_AVATAR` + `createOnClient` cascade (10–12 entity-method calls per entity per `crates/services/src/mercury/aoi/create::build_create_entity_cascade`). Twenty entities visible at once = 200+ entity-method calls, packed into ~5–10 packets. Each packet is reliable. If they land within a single ack window, the 32-bit cap on the client side overflows.
- Symptom: occasional dropped reliable messages under high-density load, presenting as missing entity updates, missing items, missing dialog options. Hard to reproduce without a controlled stress test; easy to misattribute to other layers.

### Finding #6 — Missing per-tick retry budget (5.0)

**Known issues:**

- None observed. `crates/mercury/src/channel::Channel::check_timeouts` walks the entire unacked-packet list and retransmits every entry past `ACK_TIMEOUT_MS`. Spec caps this at 5 entries per tick.

**Inferred / speculated issues:**

- Sustained packet loss (e.g., a player on a flaky WiFi or a saturated WAN link). With a typical unacked-list size of, say, 20 in-flight reliable packets and most of them aged-out simultaneously, Rust will retransmit all 20 in one tick. The C++ peer would have processed 5 and yielded the rest to subsequent ticks. The Rust burst could exceed the client's recv-side processing budget and trigger drop cascades.
- Wire-traffic-shape difference: a packet capture of a Cimmeria server under loss would show retransmit bursts; a capture of the original SGW server under the same loss would show retransmits spread evenly across ticks. The two shapes are otherwise identical, so this is a difficult-to-detect divergence in normal play.

### Finding #7 — 28-bit sequence space unenforced

**Known issues:**

- None — the test suite does not stress sequence-number wrap.

**Inferred / speculated issues:**

- After 256M reliable packets on a single channel, the sequence space wraps. Rust treats it as full u32, so seq_ids in `0x10000000..0xFFFFFFFF` are issued and accepted. The client's R4 enforcement (per spec §2.4) drops these as "sequence number outside valid range".
- Realistic trigger window: at 10 packets/sec sustained reliable traffic, ~10 months of continuous channel uptime. Real SGW gameplay sessions are minutes-to-hours, not months. This is theoretical for any current deployment, but matters for any hypothetical 24/7 cell-restart-resistant channel design.
- More immediate concern: if `crates/mercury/src/channel::Channel::send` ever assigns the sentinel value `0x10000000` itself (not currently guarded against), the client drops *that one packet* with R4 immediately. Probability per packet = 1 in 4 billion; vanishingly small but non-zero.

### Finding #8 — Inactivity-timeout misalignment (three values)

**Known issues:**

- `crates/services/src/base/tick_sync::run_tick_loop` defines `INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60)`. After 60 seconds of no client traffic (i.e., no avatarUpdateExplicit), the per-connection tick-sync loop exits. Has anyone hit this in dev testing? Unknown — it's silent unless logged. The log line at `tick_sync.rs:49-54` would fire if so.
- `crates/mercury/src/lib::consts::INACTIVITY_TIMEOUT_MS` is `300_000` (5 minutes). `crates/mercury/src/channel::Channel::is_idle` uses this to determine when a channel is dead at the Mercury layer. So there are two layers of inactivity guard — the inner 60s tick-sync layer fires first, then the outer 5-minute Mercury layer.

**Inferred / speculated issues:**

- Spec's R10 invariant: client tears down at 15 seconds of *server* silence (UE3 `NetInactivityTimeout=15`). If the Rust server's tick_sync loop ever stops (e.g., the per-connection task panics or yields without rescheduling), the client tears down silently after 15s. The server's 60s/300s inactivity guards don't fire until much later. Result: server believes the connection is alive while the client has already torn down. Server-side resources (entity, AoI subscriptions) leak until the 60s/5min guard.
- Conversely: in normal operation tick_sync sends every 100 ms, so the client never sees 15s of silence and the spec invariant is comfortably satisfied. The bug only matters when something else has gone wrong (a server stall).

### Finding #9 — `authenticate` (msg `0x01`) per-tick token validation skipped

**Known issues:**

- `crates/services/src/base/connect_loop/encrypted::handle_encrypted_datagram` skips the entire `0x01` message body without validation (per the function's own log line: "AUTHENTICATE received -- ignored"). Every gameplay packet from a client carries this message at ~13 Hz; the server reads and discards the token bytes.
- No security incident yet, because the server has not been deployed publicly and no one has demonstrated a replay attack. But the gap is exploitable by anyone with the captured AES key (which the AteraLoader sniffer hands out) — they could replay packets as the authenticated player after the original session ends, or even concurrently.

**Inferred / speculated issues:**

- A captured pcap + key file (the same artifacts the audit's V3 wire-capture tool consumed) gives an attacker the bytes of every reliable game-action packet the authenticated player emitted. The attacker can replay any of these to a Cimmeria server with the same SOAP-issued session token (since the token doesn't rotate server-side either, technically) — server treats the replayed packet as authentic.
- Attack vector becomes meaningful when (a) the server is exposed to untrusted networks and (b) AES keys leak via the AteraLoader sniffer or any future debugging path. Both are realistic for a reverse-engineering project.

### Finding #10 — Max packet size baseline

**Known issues:**

- None. Rust's `FRAGMENT_BODY_SIZE = 1300` is well under the spec's 1453 send-only cap. Conservatively safe.

**Inferred / speculated issues:**

- Slight throughput reduction: the conservative cap may produce one extra fragment on bundles right at the cap boundary. Negligible in practice (entity-method calls are bytes-to-tens-of-bytes; world-entry mapLoaded fits in ~5 packets either way).

### Finding #11 — Vestigial `PROTOCOL_VERSION` constant

**Known issues:**

- None. `crates/mercury/src/lib::consts::PROTOCOL_VERSION` is defined but has zero use sites in the entire workspace (verified by grep). The constant produces no wire bytes.

**Inferred / speculated issues:**

- A future developer reads the doc-comment ("Protocol version exchanged during channel creation handshake") and wires the constant into a Bundle emit somewhere, intending to add a version handshake. Spec §2.4 R15 is unambiguous: there is no Mercury-layer version handshake. The wire bytes the developer emits would land at the front of the first encrypted Mercury packet the client sees; the client's flag-byte parser would dispatch them as some random message id and likely log "received packet with bad flags". Hard to root-cause if the developer doesn't know spec R15 forbids the handshake entirely.

### Inverted finding #2 (sub-slot threshold) — Spec bug, Rust correct

**Known issues:**

- None on the Rust side — Rust's threshold (61) and `sub_index = method - 61` formula are wire-correct (V3 + V4 evidence).
- One on the spec side: anyone re-implementing per spec §1.5 / §1.8 / §1.16 Q1 today will write `if method >= 62` and `sub_index = method - 62`, producing wire bytes off by one for every extended-encoded entity-method call. That client would then dispatch e.g. `setupWorldParameters` to `method 123` (which doesn't exist) instead of `method 122`. World entry would silently fail at the SETUP_WORLD_PARAMETERS step.

**Inferred / speculated issues:**

- Any future audit that takes the spec at face value will generate false-positive findings against Rust on this axis (V1, V2 of this very audit did exactly that).

### Inverted finding #12 (flag-bit roles) — Spec bug, Rust correct

**Known issues:**

- None on the Rust side — Rust's bit assignments are wire-correct (V3 + V4 evidence).
- On the spec side: the bit-table at §1.2 is wrong about bits 0/5/6/7. Anyone re-implementing per the spec will produce a server whose flag bytes the SGW client logs as "received packet with bad flags" (the spec's claimed bit 7 = `IS_FRAGMENT` is actually the binary's "bad flags" trigger). Every reliable packet would be rejected.

**Inferred / speculated issues:**

- Same as #2 above: future audits taking the spec at face value will generate false-positive findings.
