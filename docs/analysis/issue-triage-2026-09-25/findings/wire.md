# Triage findings — batch `wire`

Code of record: `main-ro` @ 059d6038 (origin/main, 2026-09-25). All file:line refs are on main-ro unless noted.
Ghidra reads in this batch were decompile-only (no renames, no writes) against the loaded SGW.exe.

## #735 — docs(protocol): settle BASEMSG_REPLY_MESSAGE (0xFF) length type — WORD vs DWORD

- Verdict: KEEP
- Priority: P3
- Labels: remove `needs-triage`; add `documentation`; `ready-for-human` (needs RE)
- Summary: The contradiction is real and unchanged on main. `docs/protocol/mercury-wire-format.md:359` says `DWORD_LENGTH`; `docs/protocol/message-dispatch-table.md:27` and the draft chapter (`docs/drafts/spec/mercury-wire-format.md:552,600,739,1308,1405`) say `WORD_LENGTH`, sourced from `findings/space-viewport-wire-formats.md:609-617,654` which has no Ghidra address. The server emits DWORD (`crates/services/src/mercury/protocol/session.rs` `build_connect_reply`, `25u32.to_le_bytes()`), and #732 (merged 2026-09-25 for #274) re-affirmed that table row without resolving the question. One claim in the ticket is wrong: `crates/wireclient/src/lib.rs:15` (`WORD_LENGTH = 25`) is describing **`baseAppLogin` (client→server msg 0x00)**, not the reply — it agrees with `crates/wireclient/src/handshake.rs:7,68` and is not stale. Acceptance criterion 3 should be dropped.
- Evidence:
  - `docs/protocol/mercury-wire-format.md:253,359` (DWORD), `docs/protocol/message-dispatch-table.md:27` (WORD), `docs/drafts/spec/mercury-wire-format.md:739,1308`
  - `crates/wireclient/src/handshake.rs:7` ("baseAppLogin msg_id=0x00 WORD_LENGTH=25"), `:72-74` (`REPLY_BODY_LENGTH: u32` "advertised in its DWORD_LENGTH")
  - Client-side decompile of `BaseAppLoginHandler` ctor (`02_bigworld_network.c:11343`) shows it is a `Mercury::ReplyMessageHandler`; the reply's read path (which descriptor width the client uses for 0xFF) was not established here.
- Related/duplicates: #274 (closed by #732), #733

### Action text

Status comment:

> Still open on main after #732. The docs contradiction stands exactly as described. One correction: `crates/wireclient/src/lib.rs:15` (`WORD_LENGTH = 25`) documents `baseAppLogin` (client→server msg `0x00`), matching `handshake.rs:7,68`; it is not a stale comment about the reply, so acceptance criterion 3 no longer applies. Settling this needs a decompile of the client's reply-message read path (the `ReplyMessageHandler` / `BaseAppLoginHandler` receive side) or a pcap read at both widths — labelling `ready-for-human` for the RE pass.

## #733 — mercury: Bundle::encode clamps >64KiB payloads to the 0xFFFF escape sentinel

- Verdict: KEEP
- Priority: P3
- Labels: remove `needs-triage`; add `enhancement` (or `bug`), `ready-for-agent`
- Summary: Accurate. `Bundle::encode` still clamps with `.min(u16::MAX)` and writes the truncated body, and `Bundle::decode` still reads `0xFFFF` as a literal length. #732 documented the escape sentinel in `docs/protocol/mercury-wire-format.md:211-253` and explicitly lists `Bundle::encode` as "2 only / escape not implemented", so the docs now back the ticket. Unreachable today (no >64 KiB caller), so a latent-trap hardening task.
- Evidence:
  - `crates/mercury/src/bundle.rs:90-99` (clamp), `:103-126` (decode reads `get_u16_le` literally)
  - `docs/protocol/mercury-wire-format.md:211-253`; `docs/reverse-engineering/findings/mercury-protocol-internals.md:140-160`
  - Ghidra: `0x0158b2d0` decompiles as `InterfaceElement_expandLength_1` (the escape reader: `for iVar2 in 0,8,16,24: len |= byte << iVar2`), confirming the ticket's reader citation.
- Related/duplicates: #735, #297, #274

### Action text

Status comment:

> Confirmed on main (059d6038): `crates/mercury/src/bundle.rs:90-99` still clamps and `decode` still treats `0xFFFF` literally. #732 now documents the escape sentinel in `docs/protocol/mercury-wire-format.md` ("The over-length escape" / "What Cimmeria implements"), so the evidence is in-tree. Suggested fix stands: make `encode` fallible (or `debug_assert!` + error) on `payload.len() >= 0xFFFF`, and have `decode` reject a `0xFFFF` length rather than read it literally. Wire-format test (TESTING.md type 2). Ready for an agent.

## #359 — mercury: two latent wire-format gaps from C++ Nub deep-dive (REPLY piggyback XOR, ACK batching)

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: Gap 2 (ACK batching) is already effectively implemented: inbound reliable seqs are pushed onto a per-client `pending_acks` accumulator and drained onto the next outbound packet or the 100 ms tickSync, not emitted inline per message — the ticket's premise ("tick_sync.rs emits ACKs inline during message handling") is wrong. Gap 1 (REPLY_EXPECTED XOR-inverted length) is still unimplemented but only matters if Cimmeria ever does server-initiated RPC, which nothing plans; the evidence is preserved in `mercury-nub-anatomy.md` §9/§12 D/§13, which is where a future implementer will look.
- Evidence:
  - `crates/services/src/base/connect_loop/encrypted/mod.rs:55,108` (push to `pending_acks`)
  - `crates/services/src/base/tick_sync.rs:91` (100 ms loop), `:121-126` (drain + "Piggybacking ACKs on tick_sync")
  - `crates/services/src/base/helpers/mod.rs:308,541,632,751`, `login/mod.rs:115` (drain on every outbound path)
  - `docs/reverse-engineering/findings/mercury-nub-anatomy.md:19,260,357-359,393`
- Related/duplicates: none

### Action text

Closing comment (not planned):

> Closing. Gap 2 is already covered: inbound reliable seqs accumulate in `ConnectedClientState::pending_acks` (`connect_loop/encrypted/mod.rs:108`) and are drained onto the next outbound packet or the 100 ms tickSync (`base/tick_sync.rs:121-126`, `base/helpers/mod.rs`), so the server already batches rather than emitting one ACK packet per inbound message. Gap 1 (REPLY_EXPECTED with the `~length` field) only matters for server-initiated RPC, which nothing on the roadmap needs; the Ghidra evidence stays in `docs/reverse-engineering/findings/mercury-nub-anatomy.md` §9 / §12 D / §13 for whoever builds it. Reopen if a feature needs server→client request/reply.

## #353 — Widen Mercury TX window from 32 → 64 via SGW.exe binary patch

- Verdict: REWRITE
- Priority: P2
- Labels: add `enhancement`, `needs-triage` → after rewrite `ready-for-human` (one confirming RE check), then `ready-for-agent`
- Summary: Two things changed. (1) The failure mode that motivated it — reliable packets silently downgraded to best-effort on TX-window-full — was fixed by the deferred-send queue (#357, `MAX_UNSENT_PACKETS = 1024`) plus the bundling work (#350/#356/#361/#363/#365). (2) **The premise that the client's capacity is 32 appears to be wrong, and the patch would shrink it.** Decompiling in this triage: `Channel__ctor` (`0x01576bf0`) writes `*(Channel+0x2c) = 0x200`; `Nub::registerChannel` (`0x0157e920`) constructs `ChannelInternal(internal, &channel)`; `ChannelInternal__ctor` (`0x0158c7b0`) copies `channel+0x2c` to `this+0x30` (the "way out of window" bound used by `queueAckForPacket`) and passes it to `FUN_0158c170` as the slot-store capacity. So the unpatched client's receive window and slot store are **512**, not 32. Patching `8B 47 2C` → `PUSH 0x40; POP EAX` would cut it to 64. Further, the "32-bit outstanding-ack bitmap" citation `[^ack-bitmap]` → `0x0158b2d0` in the draft chapter is actually `InterfaceElement_expandLength_1` — its `0,8,16,24 < 0x20` loop assembles a u32 escape length, not a bitmap. The 32 cap has no binary evidence left. The work becomes: confirm nothing else rewrites `Channel+0x2c` after construction, then raise the **server** `TX_WINDOW_SIZE` (≤ 512, power of two) with no client patch, and correct the docs.
- Evidence:
  - `crates/mercury/src/lib.rs:73-90` (`TX_WINDOW_SIZE = 32`, rationale "defaults to 32 in the unmodified binary"); `:92-105` (`MAX_UNSENT_PACKETS`)
  - `crates/mercury/src/channel/tests/reassembly.rs:225-241` (`tx_window_size_pinned_until_client_patch_widens_slot_store`)
  - `docs/drafts/spec/mercury-wire-format.md:168,237,418,424-426,452-454,503,509` (bitmap claims + 32-cap), `:1496` (`[^ack-bitmap]` → `0x0158b2d0`), `:1524` (`[^channel-ctor]` already says Channel__ctor hardcodes 0x200)
  - `docs/drafts/spec/figures/sources/mercury-14a-outstanding-ack-bitmap.edn` + SVG (Figure 16) still present
  - `docs/reverse-engineering/findings/mercury-protocol-internals.md:390` ("builds ACK bundle from 32-bit ack mask" — same misidentified address)
  - Ghidra (this triage): `0x01576bf0` `*(this+0x2c)=0x200`; `0x0157e920` → `Mercury__unknown_0158c7b0(internal, channel)`; `0x0158c7b0` `this+0x30 = ch+0x2c; FUN_0158c170(this+0x40, ch+0x2c)`; `0x0158cba0` compares `delta > this+0x30` for "way out of window"
  - PRs #357 (deferred-send), #337 (tickSync → unreliable), #350 (#345 cold-cache pressure), #716 (#713 tx-window chaos test)
- Related/duplicates: #298 (same structure, mislabelled as a dedup hash), #318

### Action text

Comment:

> Rewriting. Since this was filed, #357 replaced the silent best-effort downgrade with a deferred-send queue, so TX-window-full no longer loses packets. More importantly, a re-read of the binary during triage says the unpatched client is not capped at 32: `Channel__ctor` (`0x01576bf0`) sets `Channel+0x2c = 0x200`, and `ChannelInternal__ctor` (`0x0158c7b0`, called from `Nub::registerChannel` `0x0157e920` with the channel as `param_1`) uses that value both as the out-of-window bound (`+0x30`) and as the slot-store capacity passed to `FUN_0158c170`. That makes the client window 512, and the proposed `PUSH 0x40; POP EAX` patch would *shrink* it. The "32-bit ack bitmap" citation in the draft chapter (`0x0158b2d0`) is actually `InterfaceElement::expandLength`'s escape reader. New body below: no client patch, raise the server constant after one confirming check, fix the docs.

#### New body

## Problem

The server caps in-flight reliable packets per channel at `TX_WINDOW_SIZE = 32` (`crates/mercury/src/lib.rs:90`) on the belief that the SGW client's receive slot store holds 32 entries. The binary says the default is 512. The cap forces world-entry and defeat bursts through the deferred-send queue (#357) and adds latency with no client-side reason. The draft Mercury chapter and one RE finding still describe a "32-bit outstanding-ack bitmap" that does not exist.

## Evidence

- `Channel__ctor` `ghidra://SGW.exe@0x01576bf0`: `*(Channel+0x2c) = 0x200`.
- `Nub::registerChannel` `ghidra://SGW.exe@0x0157e920`: allocates 0x180 bytes and calls `ChannelInternal__ctor(internal, &channel)`.
- `ChannelInternal__ctor` `ghidra://SGW.exe@0x0158c7b0`: `this+0x30 = channel+0x2c` (window bound); `FUN_0158c170(this+0x40, channel+0x2c)` (slot store, mask `cap-1` at `+0x44`).
- `UnAckedHandler::queueAckForPacket` `ghidra://SGW.exe@0x0158cba0`: out-of-window test `((seq - inSeqAt) & 0xFFFFFFF) > this+0x30`; slot index `seq & this+0x44`.
- The draft's `[^ack-bitmap]` (`docs/drafts/spec/mercury-wire-format.md:1496`) cites `0x0158b2d0`, which decompiles as `InterfaceElement_expandLength_1`; its `0,8,16,24 < 0x20` loop assembles a u32 length. The draft's own `[^channel-ctor]` (`:1524`) already records the 0x200.
- Not yet checked: whether anything writes `Channel+0x2c` after construction (a setter or config read). Check xrefs to that offset before changing the constant.
- Earlier failure mode (silent downgrade) already fixed by #357; #716 made the tx-window chaos test deterministic.

## Acceptance criteria

- RE note in `docs/reverse-engineering/findings/mercury-protocol-internals.md` records the 512 default, the call chain above, and the result of the `Channel+0x2c` xref check. Fix the `:390` "32-bit ack mask" row.
- If confirmed: `TX_WINDOW_SIZE` is raised to a power of two ≤ 512 (suggest 256 for headroom under the 512 out-of-window bound). The docstring is rewritten, and `tx_window_size_pinned_until_client_patch_widens_slot_store` becomes a guard that the value is a power of two and ≤ 512.
- Draft chapter §1.7 / §1.7.1 / §1.16 Q5 (`docs/drafts/spec/mercury-wire-format.md:168,237,418-454,503,509`) no longer claim a 32-bit bitmap or a 32 cap. Figure 16 (`mercury-14a-outstanding-ack-bitmap.edn` + SVG) is removed or redrawn and the figure numbering stays sequential.
- There is no client patch.

## Test type

Unit (constant guard). Mercury session / loopback (TESTING.md type 9): a 100-reliable-packet burst with no ACKs stays in the TX window and does not enter the deferred queue. The test must fail at 32.

## Docs to update

Mercury protocol-layer row (`docs/architecture/mercury-loopback-harness.md` if harness assumptions change), `docs/protocol/`, `docs/reverse-engineering/findings/`, the draft spec plus figure (figure-sources and figure-style lints are blocking).

## Client impact

Free. This is a server constant and needs no client patch.

## Domain advisor

bigworld-engine-advisor; game-archaeology-specialist for the xref check.

## Needs a human for

One RE confirmation, the `Channel+0x2c` xref sweep. After the change, an in-game world-entry and defeat smoke on the colo with `TX window` warnings watched in SigNoz.

## #318 — docs(spec): document two-counter sequencing model in mercury-wire-format §1.7 (follow-up to #317)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: The requested content landed. §1.7 of the draft chapter now has the packet-class/counter table, the "Two-counter separation is canon" rule, and a Cimmeria note that names `next_seq` / `next_seq_unreliable` and states the `inSeqAt` +0x50 vs +0x128 independence and the permanent-gap failure mode. It landed via the PR #337 review follow-up (commit d269e399) and #357. The optional §1.9.4 bandwidth note is moot because #337 moved tickSync back to the **unreliable** counter.
- Evidence:
  - `docs/drafts/spec/mercury-wire-format.md:490-505` (table + two-counter canon + server-side note), `:1853-1857` (receiver states)
  - `crates/services/src/base/helpers/mod.rs:14,540`; test `helpers/tests.rs:43` `reliable_and_unreliable_seq_counters_are_independent`
  - `git log -S next_seq_unreliable -- docs/drafts/spec/mercury-wire-format.md` → d269e399 (PR #337 review items)
- Related/duplicates: #353 (same section still carries the bogus 32-bit bitmap text)

### Action text

Closing comment (completed):

> Done. `docs/drafts/spec/mercury-wire-format.md` §1.7 (lines ~490-505) now has the reliable/unreliable counter table, the "two-counter separation is canon" rule, and the Cimmeria note naming `next_seq` / `next_seq_unreliable` with the `inSeqAt` (+0x50) vs unreliable-dedup (+0x128) independence and the permanent-gap failure mode. It landed with the #337 review follow-up (d269e399). The optional tickSync bandwidth note no longer applies, because #337 moved tickSync to the unreliable counter. The same section's "32-bit ack bitmap" wording is being handled under #353.

## #298 — [Enhancement] Add 512-entry received-sequence dedup hash (spec §1.7)

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: The premise mislabels the structure. The client's `ChannelInternal+0x40/+0x44` table is its **receive reorder buffer**, indexed by `seq & (cap-1)`. It is not a separate dedup hash. Its capacity equals its out-of-window bound (both come from `Channel+0x2c = 0x200`), and packets below `inSeqAt` are discarded by comparison, not by hash lookup. The server's `rx_window` (`RX_WINDOW_SIZE = 64`) is the same structure, only smaller. Its below-head duplicate drop matches the client's "Discarding already-seen packet below inSeqAt" behaviour. The proposed `[Option<u32>; 512]` sidecar adds nothing. The only real delta is 64 vs 512 for out-of-order packets arriving far ahead. The client sends about 6 pkt/s when idle and retransmits anything the server drops, so that delta has no observed impact.
- Evidence:
  - Ghidra `0x0158cba0` (`queueAckForPacket`): below-`inSeqAt` → discard path; `delta > this+0x30` → "way out of window"; otherwise store at `this+0x40[seq & this+0x44]` or "already-buffered"
  - Ghidra `0x0158c7b0` / `0x01576bf0`: capacity and window are both `Channel+0x2c = 0x200`
  - `crates/mercury/src/lib.rs:70` (`RX_WINDOW_SIZE = 64`); `crates/mercury/src/channel/channel_core.rs:383-387` (drop if behind or beyond window)
- Related/duplicates: #353 (same structure; the 512 also invalidates the 32 cap there)

### Action text

Closing comment (not planned):

> Closing as not planned. Re-reading `queueAckForPacket` (`0x0158cba0`) and the channel constructors (`0x01576bf0`, `0x0158c7b0`): the `+0x40/+0x44` table is the client's receive reorder buffer (slot = `seq & (cap-1)`), and its capacity equals its out-of-window bound, both `Channel+0x2c = 0x200`. It is not a separate dedup hash. Below-`inSeqAt` duplicates are dropped by comparison, which our `rx_window` already does (`channel_core.rs:383-387`). The only difference is window size, 64 vs 512, for packets arriving far ahead of the head. At the client's send rate those are rare, and the client retransmits anything we drop, so a 512-entry sidecar would add structure without changing behaviour. If we ever want strict parity, it is a one-constant change to `RX_WINDOW_SIZE`. The 512 finding matters more for #353's TX cap.

## #316 — Schema MD5 digest (PROTOCOL_DIGEST) is hardcoded — should be computed from entity definitions at startup

- Verdict: KEEP
- Priority: P3
- Labels: add `ready-for-human` (Phase 1 is RE)
- Summary: Still accurate in substance, but line numbers have moved. The constant is now at `auth/mod.rs:57` and the comparison at `auth/handlers.rs:129`. There is also an adjacent defect the ticket does not mention. `ServerConfig::protocol_digest` exists (`common/src/config.rs:95,159`), is documented as an env override (`server/src/main.rs:23,404-405` `PROTOCOL_DIGEST`), and is **never read**: the auth handler compares against the module `const`, so the env var and config key are dead. Phase 1 (OQ-4, re-verify the 17 `GetTypeName_WriteStream` encodings) is still open. `entity-property-sync.md:590` still marks the encodings MEDIUM confidence. The client `.def` tree never changes, so the frozen constant is harmless today.
- Evidence:
  - `crates/services/src/auth/mod.rs:57`; `crates/services/src/auth/handlers.rs:129-130`
  - `crates/common/src/config.rs:94-95,159`; `crates/server/src/main.rs:23,404-405` (dead override)
  - `docs/drafts/spec/entity-property-sync.md:542,572-607` (OQ-4 still MEDIUM)
- Related/duplicates: none

### Action text

Status comment:

> Still open. Updated anchors: the constant is at `crates/services/src/auth/mod.rs:57` and the comparison at `auth/handlers.rs:129`. There is an adjacent problem worth fixing on its own before the full roadmap: `ServerConfig::protocol_digest` (`common/src/config.rs:95`) and the documented `PROTOCOL_DIGEST` env override (`server/src/main.rs:404`) are never read, because the handler compares against the `const`. The cheap first step is to thread `cfg.protocol_digest` into `HandlerState` and compare against that (unit test: a non-default digest is accepted, the default is rejected), and it needs no RE. Phases 1-4 remain as written. Phase 1 (OQ-4, `entity-property-sync.md` §1.13) is still MEDIUM confidence and needs a human RE pass.

## #302 — [Enhancement] Implement restoreClient (msg 0x34) + restoreClientAck round-trip for fault recovery

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: The blocker is fixed. #389 (closing #291) parses `restoreClientAck` (0x0B) as CONSTANT_LENGTH=4 with a framing test. The rest is speculative: Cimmeria runs one process with an in-process base/cell, so there is no BaseApp restart or fault-recovery flow that could emit `restoreClient`. The ticket itself says "defer until a concrete fault-recovery feature needs it", and none is planned.
- Evidence:
  - `crates/services/src/base/connect_loop/encrypted/mod.rs:481,513`; `encrypted/tests.rs:3,33,113`
  - No `0x34` / `restoreClient` emitter in `crates/services/src`; PR #389
- Related/duplicates: #291 (closed)

### Action text

Closing comment (not planned):

> Closing as not planned. The prerequisite landed in #389: `restoreClientAck` (0x0B) is parsed as CONSTANT_LENGTH=4 and pinned by `connect_loop/encrypted/tests.rs`. Emitting `restoreClient` (0x34) only makes sense with a BaseApp-restart or fault-recovery flow, and Cimmeria's single-process base/cell has none planned. The wire layout stays documented in the Mercury chapter §1.9.5 and `space-viewport-wire-formats.md`. Reopen alongside whichever feature first needs client state restoration.

## #301 — [Enhancement] Add detailedPosition (msg 0x30) full-precision non-controlled-entity emit

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: Still no `0x30` emitter, but no need for one has come up. The NPC-AI campaign (NA00-NA30, finished 2026-09-25) worked through NPC facing and positioning (saturating `pack_angle` cast, grounding) and chose to keep UPDATE_AVATAR `0x10` (D-NA07). It did not need `detailedPosition`. The ticket is explicitly "defer until a concrete need surfaces".
- Evidence:
  - No `DETAILED_POSITION` in `crates/`; `crates/services/src/mercury/aoi/update.rs:13-22` (only `0x10`)
  - `docs/analysis/npc-ai-restoration/README.md:44` (D-NA07: keep 0x10); `audit.md:52` (M6)
  - PR #677 (NPC wire facing), #779 (stopped NPCs zero velocity)
- Related/duplicates: #300

### Action text

Closing comment (not planned):

> Closing as not planned. The NPC-AI campaign (NA00-NA30) covered NPC facing and position precision and settled on keeping UPDATE_AVATAR `0x10` with server-side grounding (D-NA07, `docs/analysis/npc-ai-restoration/README.md`). Nothing needs `detailedPosition` (0x30). The 41-byte layout stays documented in `position-movement-wire-formats.md` / Mercury chapter §1.11.2. Reopen if a concrete path (for example an admin-placement tool) shows `0x10` quantization is visible.

## #300 — [Enhancement] Add the 31 missing UPDATE_AVATAR variants (msg_ids 0x11..0x2F)

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: Still true that only `0x10` is emitted. The premise that the other variants are harmless bandwidth optimizations is now partly wrong. Audit finding M6 showed the **OnGround** family (e.g. `0x18`) makes the client substitute the entity's *current client* height for a sentinel Y (-13000.0) with no ray-cast. Sending it would pin NPCs at creation height. So a quarter of the proposed variants would be actively harmful, and the rest are a bandwidth optimization with no measured need. Per the ticket's own "defer until a gameplay path needs a particular variant", close.
- Evidence:
  - `crates/services/src/mercury/aoi/update.rs:1-22`
  - `docs/analysis/npc-ai-restoration/audit.md:52` (M6, `FUN_00ddb830`, `0x00dd1859`), `README.md:44` (D-NA07 "Do not use 0x18")
  - `docs/protocol/client-verified-wire-formats.md:55-56,327` (variant table and a known 0x21/0x22 naming swap)
- Related/duplicates: #301

### Action text

Closing comment (not planned):

> Closing as not planned. Only `0x10` is emitted, and that is deliberate now. The NPC-AI audit (M6, `docs/analysis/npc-ai-restoration/audit.md`) found that the OnGround variants make the client substitute its own current height for a sentinel Y with no ray-cast, so emitting them would pin NPCs at creation height (D-NA07: "Do not use 0x18"). The remaining variants would only save bandwidth, and nothing has shown that matters. If a specific path later needs one (for example rotation-only NoPos updates in a dense scene), file it for that variant, with a wire-format test and the M6 caveat.

## #299 — [Enhancement] AUTHENTICATE (msg 0x00) DWORD_LENGTH emit path

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: The only stated trigger was a Mercury-layer rekey, and #575 (Mercury v2 session-key rotation, `crates/mercury/src/encryption/rotation.rs`) delivered rotation without server→client msg `0x00`. The server has never needed to emit AUTHENTICATE, and `docs/protocol/message-dispatch-table.md:63` records it as "never observed on wire". Its "blocked on M1" dependency (#297) is also being closed.
- Evidence:
  - `crates/mercury/src/encryption/rotation.rs`, `test_harness/tests/rotation.rs` (#575)
  - `docs/protocol/message-dispatch-table.md:63`; `space-viewport-wire-formats.md` table row 0x00 "Never observed"
  - Inbound client AUTHENTICATE is handled and ignored: `connect_loop/encrypted/mod.rs:156`
- Related/duplicates: #297, #735

### Action text

Closing comment (not planned):

> Closing as not planned. The ticket's only trigger was a Mercury-layer rekey, and #575 shipped v2 session-key rotation without it. Server→client `AUTHENTICATE` (0x00) is recorded as never observed on the wire (`docs/protocol/message-dispatch-table.md:63`), and nothing in the handshake needs it. The one real `DWORD_LENGTH` question left on this wire is `BASEMSG_REPLY_MESSAGE`, tracked in #735.

## #297 — [Enhancement] Add InterfaceElement length-encoding table (CONSTANT / WORD / DWORD / compressed widths)

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: #732 (for #274) did the verification this ticket needed. It enumerated which widths SGW actually declares: none use width 1 or 3, no client→server message uses DWORD, and only one hand-written DWORD emitter exists (`build_connect_reply`). It concluded "width alone cannot mis-frame anything on this wire today". The ticket's own claim that "compressed widths are fixed with no threshold" was also superseded by #732's escape-sentinel finding. The one real exposure is the >64 KiB escape, which #733 tracks. A general `LengthEncoding` table has no consumer (its motivating consumer, AUTHENTICATE #299, is being closed).
- Evidence:
  - `docs/protocol/mercury-wire-format.md:196-253` ("What Cimmeria implements" table); `crates/services/src/mercury/protocol/length_framing_tests.rs`
  - `docs/reverse-engineering/findings/mercury-protocol-internals.md:140-160`
- Related/duplicates: #733 (successor for the real hazard), #299, #274

### Action text

Closing comment (not planned):

> Closing. #732 (for #274) audited every length-framing site. SGW declares no width-1/3 messages and no client→server `DWORD_LENGTH`, and the one outbound DWORD (`build_connect_reply`) is written by hand and pinned by `length_framing_tests.rs`, so the width switch cannot mis-frame anything today (`docs/protocol/mercury-wire-format.md` "What Cimmeria implements"). The remaining real exposure, the over-length `0xFF…` escape, is tracked in #733. A generic `LengthEncoding` table can come back if a message ever needs a width we don't already handle.

## #295 — [Docs] Mercury chapter wire-format corrections from V3-V5 audit (flag bits, sub-slot threshold, createEntity size)

- Verdict: REWRITE
- Priority: P3
- Labels: keep `documentation`; swap `ready-for-human` → `ready-for-agent` after the rewrite (the evidence is now in-tree)
- Summary: All four corrections are still needed in `docs/drafts/spec/mercury-wire-format.md`. The unattended-agent comment of 2026-09-19 calling #1 and #2 "contradicted" is wrong: the chapter is the losing source on both, and the **working Rust code, which talks to the real client**, proves it. (1) Flag bits: Rust uses `0x01 HAS_REQUESTS`, `0x20 FRAGMENTED`, `0x40 HAS_SEQUENCE`, `0x80 INDEXED` (`crates/mercury/src/packet/mod.rs:56-100`, tidied by #708/#296), and the live `baseAppLogin` goes out as `0x41` (`wireclient/src/handshake.rs:15`). The chapter table at §1.2 (`:150-166`) still has the stock-BW assignment. (2) Sub-slot threshold: #392 (closing #315) established a per-entity `idBase = 0x3E - (nExposed + 0xC0)/0xFF`, which is 61 for SGWPlayer (`docs/protocol/mercury-wire-format.md:325-333`, `crates/services/src/mercury/mod.rs:185-189,297-317`). The chapter still says a flat 62 with `117-62 = 55` and an "override" note calling 56 off-by-one (`:561-594,1306,1390,1648,1662`). The correct fix is "per-entity idBase; 61 for SGWPlayer; 117 → 56". A flat 61 would also be wrong. (3) createEntity: the chapter says 5 bytes (`:1083`) while its own field list and Rust (`aoi/create.rs:87` `wordLength = 8`) say 8. (4) The §2.3 baseline note is still absent. New since filing: the chapter's §1.5 claim that compressed-length has "no runtime-selected threshold sentinel" and Q1 "closed: no thresholds" (`:371,381,383,1364,1466`) is contradicted by #732's escape-sentinel finding.
- Evidence: as above; PRs #708, #392, #732; `docs/protocol/mercury-wire-format.md:211-253,325-333`
- Related/duplicates: #353 (bitmap corrections to the same chapter §1.7, kept there because they drive a code change), #315 (closed), #296 (closed)

### Action text

Comment:

> Re-scoping after re-checking against main. The 2026-09-19 note that #1 and #2 are "contradicted" by the chapter has it backwards. The chapter is the stale side, and the code that talks to the real client is the evidence. Rust's flag constants (`packet/mod.rs:56-100`) and the live `0x41` baseAppLogin flags match the audit, not §1.2. For the sub-slot split, #392 established a *per-entity* idBase (61 for SGWPlayer), so the chapter's flat 62 / `117-62=55` and its "override" note are wrong. A flat 61 would be wrong too. createEntity is 8 bytes in Rust. #732 has since added a fifth correction: §1.5 / Q1's "no threshold" claim is superseded by the over-length escape. Body rewritten with exact line anchors. The 32-bit-bitmap text in §1.7 is handled in #353.

#### New body

## Problem

`docs/drafts/spec/mercury-wire-format.md` disagrees with the code that works against the real client, and with `docs/protocol/`, in five places. Reimplementers following the chapter will build a broken wire.

## Evidence

1. **§1.2 flag-bit table** (`:150-166`) maps bit0=`HAS_FIRST_REQUEST_OFFSET`, bit5=`HAS_SEQUENCE_NUMBER`, bit6=`HAS_REQUESTS`, bit7=`IS_FRAGMENT`. The working server uses bit0=`HAS_REQUESTS`, bit5=`FRAGMENTED`, bit6=`HAS_SEQUENCE`, bit7=`INDEXED`/reserved (`crates/mercury/src/packet/mod.rs:56-100`; comments corrected in #708). The client's first packet is flags `0x41` (`crates/wireclient/src/handshake.rs:15`). Same decoder cited: `processFilteredPacket_inner` `ghidra://SGW.exe@0x01580840`.
2. **Sub-slot split** (`:561-594`, `:1306`, `:1390`, `:1648`, `:1662`): the chapter says a flat threshold of 62 and `117 → 55`. The code and protocol doc say it is per-entity, `idBase = 0x3E - (nExposedCount + 0xC0) / 0xFF` (`EntityDescription_AssignClientMethodIds` `ghidra://SGW.exe@0x01590df0`), which is 61 for SGWPlayer, so `onClientMapLoad` (117) → sub_index 56 (`docs/protocol/mercury-wire-format.md:325-333`; `crates/services/src/mercury/mod.rs:297-317`; #392 / #315).
3. **§1.10.5 createEntity** (`:1083`): "5 bytes (`word_len = 5`)". The field list below it sums to 8, and Rust sends `wordLength = 8` (`crates/services/src/mercury/aoi/create.rs:87`).
4. **§2.3**: no note that Rust's `PACKET_MAX_SIZE = 1472` is MTU-inclusive and `FRAGMENT_BODY_SIZE = 1300` is a conservative margin, distinct from the client's 1453-byte send cap (`:109,1726,1752`).
5. **§1.5 / §1.14 / Q1** (`:371,381,383,1364,1466`): "no runtime-selected threshold sentinel" and "Q1 closed: no thresholds". `docs/protocol/mercury-wire-format.md:211-240` (#732) shows `compressLength` (`0x0158b120`) does test width 1/2/3 overflow and emits the `0xFF…` + u32 escape (`0x0158acc0`, reader `0x0158b2d0`).

## Acceptance criteria

- §1.2 table matches `packet/mod.rs`, with a source-doc-override callout citing the Rust constants and the `0x41` handshake flags.
- §1.5/§1.8 state the per-entity idBase formula, give SGWPlayer = 61 and the 117 → 56 example, and delete the "off-by-one override" note. The §1.14 row and footnotes `[^subslot-threshold]` / `[^stockbw-method-desc]` are updated.
- §1.10.5 says 8 bytes / `word_len = 8`.
- §2.3 carries the baseline note.
- §1.5, §1.14 row, and Q1 describe the over-length escape and point to `docs/protocol/mercury-wire-format.md`.
- The markdown and figure-style lints pass. Docs are CRLF (keep line endings).

## Test type

None. This is a docs-only change and the Rust behaviour is already correct. Cross-check the worked examples against the existing sub-slot tests in `crates/services/src/mercury/`.

## Docs to update

The wire-format row: `docs/drafts/spec/mercury-wire-format.md`. `world-entry-pipeline.md` is the *winning* side on 117 → 56, so leave it alone.

## Client impact

Free. Docs only.

## Domain advisor

bigworld-engine-advisor; documentation-writer.

## Needs a human for

Nothing. The evidence is in-tree.

## #293 — [Major] Reconcile three inactivity-timeout constants (15s spec vs 60s tick-sync vs 300s Mercury)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: #711 (merged 2026-09-19) delivered what survived the owner's 2026-09-19 correction. `INACTIVITY_TIMEOUT_MS` was renamed to `MERCURY_PEER_DEAD_MS` (300 s, unchanged), `UE3_INACTIVITY_TIMEOUT_MS = 15_000` was added and documented as the *client's* tolerance for server silence, and tick_sync's 60 s server-side reap was kept with a clarifying comment and a regression guard. The ticket's core premise was wrong in direction: the 15 s value is client-side tolerance, which the server honours by sending tickSync every 100 ms, and the original server tolerated 300 s of client silence. The optional "reap dead clients faster than 60 s" idea has no evidence behind it: idle clients send about 6 pkt/s, a CellBlock load takes about 2.4 s, and there were no 60 s timeouts in 30 days of colo data. It would only hurt debugger-paused RE sessions.
- Evidence:
  - PR #711 (a18d4886); `crates/mercury/src/channel/channel_core.rs:595,626`; `channel/tests/channel_lifecycle.rs:195-221`
  - `crates/services/src/base/tick_sync.rs:84` (60 s kept)
  - Owner comment on the issue 2026-09-19; `docs/protocol/login-handshake.md` (`client_inactivity_timeout = 300000`)
- Related/duplicates: none

### Action text

Closing comment (completed):

> Done in #711: `INACTIVITY_TIMEOUT_MS` → `MERCURY_PEER_DEAD_MS` (300 s), a new `UE3_INACTIVITY_TIMEOUT_MS = 15_000` documented as the client-side R10 tolerance (which the server meets with its 100 ms tickSync, not by matching the number), and tick_sync's 60 s server-side reap kept, with a guard test that the two are not conflated (`channel_lifecycle.rs`). Faster client reaping is not being pursued. The original server used 300 s, measured idle clients send about 6 pkt/s, and no 60 s timeouts fired in 30 days, so a shorter reap would only drop frozen or debugger-paused clients. Reopen with a concrete failure if one shows up.

## #171 — Two parallel method-index constant tables: consolidate mercury::method_idx and cell::client_methods

- Verdict: REWRITE
- Priority: P3
- Labels: add `ready-for-agent` after rewrite
- Summary: Still valid, and it has grown. `mercury::method_idx` (`crates/services/src/mercury/mod.rs:195+`) now has about 320 call-site references and `cell::client_methods::*` (14 per-interface files) about 66. Shared constants are still duplicated (`ON_STATE_FIELD_UPDATE` 19, `ON_ENTITY_PROPERTY` 7), and a **third** copy has appeared: `cell/console/net.rs:27-29` defines local `ON_TIMER_UPDATE`/`ON_MAP_INFO`/`ON_CLIENT_CHALLENGE` "for the handful not in method_idx" (duplicated again in `console/tests/p38.rs:25`). The recommended Option 2 (delete `method_idx` and migrate to `client_methods`) now conflicts with CLAUDE.md, which names "the `method_idx` constants module in `crates/services/src/mercury/mod.rs`" as the canonical place to update for wire-index changes. At roughly 320 references it is also the bigger migration. No test asserts the two tables agree.
- Evidence:
  - `crates/services/src/mercury/mod.rs:195-290`; `crates/services/src/cell/client_methods/mod.rs` (14 submodules); `being.rs:6,20`, `spawnable_entity.rs:18`
  - `crates/services/src/cell/console/net.rs:23-29`; `console/tests/p38.rs:25`
  - Ten files import from both tables (e.g. `cell/service/ticks/reload_completion.rs`, `cell/effects/pulsing/tick.rs`)
  - CLAUDE.md doc-update map, "Wire format, method indices" row
- Related/duplicates: none

### Action text

Comment:

> Still valid and larger than when filed. `method_idx` now has about 320 references and `client_methods` about 66, and `cell/console/net.rs:27-29` added a third, local copy (`ON_TIMER_UPDATE`, `ON_MAP_INFO`, `ON_CLIENT_CHALLENGE`). The original Option 2 conflicts with CLAUDE.md, which names `mercury::method_idx` as the canonical constants module. The body is rewritten to make `method_idx` the single source, with the per-interface modules re-exporting from it plus a parity guard.

#### New body

## Problem

Server→client (ClientMethod) wire indices are defined in three places. Adding or renumbering a method in one place silently diverges from the others.

## Evidence

- `crates/services/src/mercury/mod.rs:195` `pub mod method_idx`: about 69 consts, about 320 call-site references. CLAUDE.md names it canonical for method-index changes.
- `crates/services/src/cell/client_methods/{being,spawnable_entity,player,...}.rs` (14 files): about 66 references. Duplicates include `ON_STATE_FIELD_UPDATE = 19` (`being.rs:20` vs `mercury/mod.rs:211`), `ON_ENTITY_PROPERTY = 7` (`spawnable_entity.rs:18` vs `mercury/mod.rs:200`), and `ON_TIMER_UPDATE = 12` (`being.rs:6`).
- `crates/services/src/cell/console/net.rs:27-29` and `console/tests/p38.rs:25`: local literal copies.
- Source of truth for values: `docs/protocol/client-method-dispatch-table.md` (157 SGWPlayer client methods).

## Acceptance criteria

- Every server→client index constant is defined once, in `mercury::method_idx`. `cell::client_methods::<iface>` keeps its per-interface grouping by `pub use`-ing from `method_idx`, so the 66 call sites don't change.
- The local consts in `cell/console/net.rs` and `console/tests/p38.rs` are removed in favour of `method_idx`.
- A unit test pins a sample of indices against `client-method-dispatch-table.md` values (at minimum 7, 12, 19, 100, 120, 123, 142).
- No behaviour change. A byte-identical wire is expected.

## Test type

Unit (constant pin). The existing wire-format tests must stay green unchanged.

## Docs to update

None beyond a doc comment in `cell/client_methods/mod.rs` stating it re-exports from `method_idx`.

## Client impact

Free. This is a refactor.

## Domain advisor

rust-gameserver-dev.

## Needs a human for

Nothing.

## #276 — Verify: onDHDReply emit targets VCommunicator (not VGateTravel)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change (closing)
- Summary: The verification is done. PR #729 (merged 2026-09-25) audited the emit path and found **no production `onDHDReply` emitter**. The only references are `ON_DHD_REPLY = 100` (`cell/client_methods/player.rs:8`), a comment in `mercury/mod.rs:263`, and the wire-log decoder. So there is no wrong-target routing bug to find. #729 also recorded in `stargate-dhd-state-machine.md` §"onDHDReply Declaration and Rust Audit" that the VCommunicator subscriber at `0x00cf5440` is the recorded handler and that an entity ID does not select the subscriber. It corrected `gate-travel-wire-formats.md` too. The live-test criterion is moot without an emitter, and the missing-emitter work was split out to #727.
- Evidence:
  - PR #729; `docs/reverse-engineering/findings/stargate-dhd-state-machine.md:4,8,48,135-147`
  - `crates/services/src/cell/client_methods/player.rs:8`; no send call in `crates/services/src`
- Related/duplicates: #727 (follow-up)

### Action text

Closing comment (completed):

> Verified in #729. Cimmeria has no production `onDHDReply` (method 100) emitter, so it cannot be mis-targeted. `stargate-dhd-state-machine.md` §"onDHDReply Declaration and Rust Audit" records the declaration (`WSTRING aMessage`), the VCommunicator subscriber (`0x00cf5440`), and that an entity ID does not select the subscriber. `gate-travel-wire-formats.md` was corrected in the same PR. Adding an emitter, and live-verifying where the text appears, continues in #727.

## #727 — DHD text feedback: method 100 has no production emitter

- Verdict: REWRITE
- Priority: P2
- Labels: remove `needs-triage`; add `bug`, `ready-for-human` (one UAT step), then `ready-for-agent`
- Summary: Accurate, but it undersells the problem. Every rejected dial is currently **silent** to the player. The 2009 server told the player on each reject branch via `self.onError("Failed to dial: …")`, which is `onPlayerCommunication('', 0, CHAN_server, msg)` (`deprecated/python/cell/SGWPlayer.py:879-884,2050-2067`). Rust's `handle_dial_gate` cancels and logs only (`cell/gate_travel/mod.rs:127-160`), and the dispatcher comment at `cell/cell_methods/gate_travel.rs:29-32` says "the ordinary client has no dial-failure wire surface". That comment overlooks `onDHDReply(WSTRING)`, which `SGWPlayer.def` describes as "feedback on attempted DHD use". This breaks the project rule that every button press gets visible feedback on the first press. Two free fixes exist: `onDHDReply` (method 100, most likely intended) or the legacy server-channel chat (`ON_PLAYER_COMMUNICATION`, already used by `cell/console/net.rs`). Neither needs a client patch. Which surface actually renders text in the DHD UI has not been verified.
- Evidence:
  - `crates/services/src/cell/cell_methods/gate_travel.rs:29-32` (wrong "no surface" comment)
  - `crates/services/src/cell/gate_travel/mod.rs:127-160` (silent reject branches: unknown address, invalid address, entity not found, same world)
  - `deprecated/python/cell/SGWPlayer.py:879-884` (`onError` → CHAN_server), `:2050-2067` (three reject messages)
  - `entities/defs/SGWPlayer.def` `onDHDReply(WSTRING aMessage)`; `docs/reverse-engineering/findings/stargate-dhd-state-machine.md:48,135-147`
- Related/duplicates: #276 (closing, parent audit)

### Action text

Comment:

> Rewriting with the gameplay consequence. Every rejected dial (unknown address, invalid address, dialing your own world) is currently silent to the player. The 2009 Python answered each one with `onError("Failed to dial: …")`, i.e. a server-channel `onPlayerCommunication`. The comment at `cell_methods/gate_travel.rs:29-32` that the client "has no dial-failure wire surface" misses method 100, which the `.def` defines for exactly this. Both candidate surfaces are free. The only open question is which one the DHD UI visibly renders, and that needs one live check.

#### New body

## Problem

When a player's DHD dial is rejected, the server cancels the dial and logs, but it tells the player nothing. The player presses a glyph sequence and sees no reaction, which breaks the first-press feedback rule (CLAUDE.md project rules). `onDHDReply` (SGWPlayer client method 100, `WSTRING aMessage`, "Give the client feedback on attempted DHD use") has no production emitter.

## Evidence

- Silent reject branches: `crates/services/src/cell/gate_travel/mod.rs:127-160` (address not in address book, address not in `stargates`, entity not found, already in destination world).
- Incorrect justification: `crates/services/src/cell/cell_methods/gate_travel.rs:29-32` says there is "no dial-failure wire surface".
- 2009 behaviour: `deprecated/python/cell/SGWPlayer.py:2050-2067` calls `self.onError("Failed to dial: invalid stargate address" | "…not a known stargate address" | "…world has no stargates!")`, and `onError` (`:879-884`) sends `onPlayerCommunication('', 0, CHAN_server, msg)`.
- Method 100 declaration and the VCommunicator subscriber at `0x00cf5440`: `docs/reverse-engineering/findings/stargate-dhd-state-machine.md` §"onDHDReply Declaration and Rust Audit" (#729). The binary payload decode and on-screen placement are unverified.
- The address-book check deliberately makes unknown and nonexistent addresses indistinguishable (`gate_travel/mod.rs:109-114`, existence-oracle concern). The feedback text must preserve that: use one message for both.

## Acceptance criteria

- Each reject branch in `handle_dial_gate` sends one visible message to the dialling player on the first press. Unknown and nonexistent addresses share identical text.
- The surface is chosen after a live check: send `onDHDReply("…")` with the DHD open and confirm where it renders. If it does not render visibly, fall back to server-channel `onPlayerCommunication` as the 2009 server did.
- The wrong comment at `cell_methods/gate_travel.rs:29-32` is corrected.
- `stargate-dhd-state-machine.md` records the observed rendering.

## Test type

Wire-format (TESTING.md type 2): byte-exact method-100 (or chat) payload to the dialling player's entity. Chain-replay or unit: each reject branch emits exactly one message, and the unknown-vs-nonexistent texts are identical. The guard must fail if the emit is removed.

## Docs to update

`docs/reverse-engineering/findings/stargate-dhd-state-machine.md`; the `docs/gameplay/` stargate section if it describes dialing.

## Client impact

Free. Both candidate surfaces are messages the client already handles.

## Domain advisor

movement-teleport-advisor (gate travel); game-archaeology-specialist for the VCommunicator render check.

## Needs a human for

One in-game UAT to see where `onDHDReply` text renders.

## #720 — verify: weapon reload onTimerUpdate uses Type=2 but the binary has a dedicated reload handler for types 12/13

- Verdict: KEEP
- Priority: P3
- Labels: remove `needs-triage`; add `documentation`, `ready-for-human` (needs RE or a capture)
- Summary: Accurate and correctly hedged. `reload.rs:216-228` sends `onTimerUpdate(ABILITY_RELOAD_WEAPON, TIMER_ABILITY_COOLDOWN=2, …, total_time, 0.0)`. The client has `GameBeing_HandleOnTimerUpdate_Reload` (`0x00e02380`) for types 12/13, which fires `Event_UI_EntityReload[Deployment]` (`ability-resolution-pipeline.md:233-234,278-281`; `address-map.md:1315`). `crates/entity/src/abilities/defs.rs:66-69` has no 12/13 constants. The 2009 Python never sent 12/13 either, so nothing yet proves the UI depends on them. Type 2 on the reload *ability* id is a legitimate ability-cooldown use in its own right. The open question is whether the reload bar or animation also needs a type-12 timer. Open PR #718 (for #271) touches `reload.rs` to fill `BigWorldTimeComplete` but keeps type 2, so the two should be sequenced.
- Evidence:
  - `crates/services/src/cell/cell_methods/player/world/reload.rs:216-228`; `crates/entity/src/abilities/defs.rs:64-69`
  - `docs/reverse-engineering/findings/ability-resolution-pipeline.md:233-234,278-281`; `docs/reverse-engineering/address-map.md:1315`
  - `deprecated/python/Atrea/enums.py:491-492` (`ReloadTimer = 12`, no sender)
  - Open PR #718 file list includes `reload.rs` and `tests/reload.rs`
- Related/duplicates: #271 (same emit site, open PR #718)

### Action text

Status comment:

> Triage: the claim checks out on main. `reload.rs:216-228` emits type 2, and the client's type-12/13 handler (`0x00e02380`) fires `Event_UI_EntityReload`. Nothing in-tree shows whether the reload UI needs that event: no 2009 sender exists, and the owner has reported bandolier and reload working. Next step is RE: find the `Event_UI_EntityReload` subscribers, or capture a reload on the live client with a type-12 timer injected via the `.net timer` console, and see what changes. Note that PR #718 (#271) edits the same emit to fill `BigWorldTimeComplete`. Land that first, then adding a type-12 emit is a small follow-up.

## #266 — MissionTaskStatus.count wire format: alias.xml says INT8, binary reads INT32

- Verdict: CLOSE (not planned)
- Priority: P3
- Labels: no change (closing)
- Summary: The premise is disproved, and the 2026-09-19 agent comment on the issue is correct on every point, re-checked on main. `onTaskUpdate` (client method 83) is declared `INT32 TaskID, INT8 Status, INT32 Count` in `Missionary.def`, which matches the binary's INT32 reader. The alias.xml `Mission*Status` FIXED_DICTs are referenced by no `.def`, so they never reach the wire. Cimmeria has no `onTaskUpdate` emitter (only `ON_TASK_UPDATE = 83` in `cell/client_methods/missionary.rs:10` and the wire-log decoder, which already reads `i32`), so no serializer exists to be wrong. The "missing emitter" follow-up the agent suggested is weak: the 2009 Python never sent `onTaskUpdate` either (`grep deprecated/python` finds nothing).
- Evidence:
  - `entities/defs/interfaces/Missionary.def:87-91`; `docs/protocol/client-method-dispatch-table.md:210`
  - `crates/services/src/cell/client_methods/missionary.rs:10`; `wire_log/decoders/generated.rs:895`
  - `grep -rn "Mission[A-Za-z]*Status" entities/` → alias.xml only; `grep -rn onTaskUpdate deprecated/python` → no hits
- Related/duplicates: none

### Action text

Closing comment (not planned):

> Closing: the premise does not hold (see the 2026-09-19 analysis above, re-verified on main). `onTaskUpdate`'s `Count` is `INT32` in `Missionary.def`, which matches the binary. The INT8 `count` belongs to an alias.xml `MissionTaskStatus` FIXED_DICT that no `.def` references, so it never reaches the wire. Cimmeria has no `onTaskUpdate` serializer that could get it wrong. The 2009 server never emitted `onTaskUpdate` either, so a missing emitter isn't a regression. If per-task counters (for example "kill 3/5") are wanted in the tracker, file that as a feature against `docs/content/`.

## #271 — Ability cooldown bigWorldTimeComplete: emit absolute game-time of expiry

- Verdict: KEEP
- Priority: P3
- Labels: add `ready-for-human` (the PR needs owner review of a clock change on the tickSync wire)
- Summary: Still valid, and an implementation is in flight. The emit site moved from `cell/abilities.rs:185` to `cell/abilities/use_ability/handle.rs`, and `BigWorldTimeComplete` is still `0.0` on main. Open PR #718 (updated 2026-09-25, owner has commented) adds a boot-stamped `base/game_time.rs` clock, carries it on tickSync `gameTime` in place of the per-session counter, and emits `now + cooldown`. It also touches `reload.rs` and `console/net.rs`. The clock-domain trap (SET_GAME_TIME / tickSync must share the clock the absolute value is measured in) is recorded in the combat advisor's agent memory (`ontimerupdate-wire-and-clock.md`, commit 192d4216). The same note flags that there is no type-1/type-8 (warmup/category cooldown) sender and that `serialize_timer_update` hardcodes `SecondaryId = 0`. Both are out of scope here.
- Evidence:
  - `crates/services/src/cell/abilities/use_ability/handle.rs` (TODO `bigWorldTimeComplete = gameTime + cooldown`)
  - Open PR #718 `feat/271-cooldown-absolute-time` (21 files incl. `base/game_time.rs`, `base/tick_sync.rs`, `mercury/protocol/session.rs`)
  - `deprecated/python/cell/AbilityManager.py:605-617` (`cooldown = now + cooldown + warmup`)
- Related/duplicates: #720 (same `reload.rs` emit)

### Action text

Status comment:

> Status: implementation is in PR #718, awaiting owner review. Updated anchor: the emit now lives in `crates/services/src/cell/abilities/use_ability/handle.rs`, still `0.0` on main. #718 changes what tickSync carries (a shared boot-stamped game clock instead of a per-session counter), so it needs the in-game reconnect-mid-cooldown UAT from the acceptance criteria before merge. Close this issue when #718 merges.

## #534 — RE: settle requestAmmoChange.ItemId semantics (instance vs design vs slot token) + consider adding a slot id to the message

- Verdict: REWRITE
- Priority: P1
- Labels: keep `bug`; add `ready-for-agent` after the rewrite (the RE question is settled below); remove the protocol-revision ask
- Summary: **Settled during this triage by decompile: the client sends the item's instance id (`InvItem.id`).** The server treats it as a design id, so `requestAmmoChange` probably never matches a bandolier slot. Chain: `FUN_00e1f4e0(bagId, _, ammoType)` resolves the bag's active-slot record (`FUN_00e1c4c0`), fetches the item in that slot (`FUN_00d22580`), and calls `FUN_00e1ee10(item, ammoType)`. That function checks `ammoType` is in the item's `ammoTypes` vector and sends `ItemId = item+0x0C`. The item ctor `FUN_00d21750(this, id, dbid, stackSize, slotID, containerID, isBound, durability, ammoTypes, curAmmoType)` stores **`+0x0C = id`** and `+0x00 = dbid`, and the `ItemUpdates` parser `0x00e1fd30` passes the `"id"` field first. Rust serializes `InvItem.id = sgw_inventory.item_id` (the instance id; `player_load/core/inventory_items.rs:41`), exactly as the 2009 Python did (`Item.py:231-232`: `'id': self.id, 'dbid': self.typeId`). So the RE doc's "Weapon instance ID" (`inventory-wire-formats.md:242`) is **correct**. The "slot token echoed from onActiveSlotUpdate" reading is **refuted**: `FUN_00e1fb20` writes `SlotId` into a separate 0x1c-byte active-slot record, which only *selects* the item. The server handler `ammo_change.rs:84-120` looks up `item_defs.get(&item_id)` (design-keyed) and filters `item.item_id == item_id` (design id), so for a real instance id it logs "item not in bandolier" and does nothing. Worse, if an instance id happens to equal some design id, it validates against the wrong weapon's whitelist. No protocol change is needed: `BandolierItem` already carries `instance_id` (`cell_entity/mod.rs:107-119`, from #520). Matching on it is unique by PK, which also removes the "ambiguous duplicate weapons" rejection and its TODO. Open PR #602 ("reject requestAmmoChange when weapon def is not cached") would turn today's silent miss into an explicit reject but keep the wrong key, so it should be rebased onto this fix or folded into it.
- Evidence:
  - Ghidra (decompile only): `0x00e1f4e0` → `0x00e1ee10` (sends `item[3]` = `+0x0C` as `"ItemId"`); `0x00d21750` item ctor (`+0x0C = param_1 = id`, `+0x00 = dbid`); `0x00e1fd30` `ItemUpdates` parser reads `"id"`, `"dbid"`, … and calls the ctor in that order; `0x00e1fb20` (onActiveSlotUpdate) writes `SlotId` to a 0x1c-byte record's `+0x0C`, a different object
  - `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs:84,94-120`
  - `crates/entity/src/inventory.rs:65-69,90-92` (`InvItem.id` = instance, serialized first); `base/world_entry/methods/player_load/core/inventory_items.rs:40-42`
  - `crates/entity/src/cell_entity/mod.rs:107-119` (`BandolierItem.instance_id`)
  - `deprecated/python/cell/Item.py:231-232`; `SGWPlayer.py:2199-2200` (legacy handler was `pass`)
  - Open PR #602
- Related/duplicates: #520 / #445 (instance-id persistence, merged), #602 (open, same handler), #533 (closed)

### Action text

Comment:

> Settled by decompile during triage: `ItemId` is the item **instance id** (`InvItem.id`). The client sends `item+0x0C` from `FUN_00e1ee10`. The item ctor `FUN_00d21750` stores `id` at `+0x0C` (and `dbid` at `+0x00`), and the item comes from the bag's active slot, so the slot number only picks the item and is not what's sent. `inventory-wire-formats.md:242` is right. The server is wrong: `ammo_change.rs` matches the value against the bandolier item's **design** id, so a genuine request most likely falls through to "item not in bandolier" and is silently ignored. No protocol change is needed, because `BandolierItem.instance_id` already exists (#520) and is unique. Body rewritten as a server fix. #602 touches the same handler and should be reconciled with it. Please confirm with one log check (`requestAmmoChange: item not in bandolier` on an ammo swap in SigNoz) or an in-game ammo-type swap.

#### New body

## Problem

`requestAmmoChange(ItemId, AmmoType)` (cell method 42) carries the weapon's **instance id**, but `handle_request_ammo_change` treats it as a **design id**. Ammo-type swaps from the client almost certainly never match a bandolier slot and are silently ignored. That breaks the first-press feedback rule and makes ammo-type switching non-functional.

## Evidence

- Client send path (Ghidra, SGW.exe): `FUN_00e1f4e0` → active-slot record (`FUN_00e1c4c0`) → item in slot (`FUN_00d22580`) → `FUN_00e1ee10(item, ammoType)`. It checks `ammoType ∈ item.ammoTypes` (`+0x28..+0x2C`) and sends `"ItemId" = item+0x0C`, `"AmmoType"`.
- Item layout: ctor `FUN_00d21750(this, id, dbid, stackSize, slotID, containerID, isBound, durability, ammoTypes, curAmmoType)` stores `+0x00 = dbid`, `+0x0C = id`. It is fed by the `ItemUpdates` parser `0x00e1fd30` in `InvItem` field order.
- Server sends `InvItem.id = sgw_inventory.item_id` (instance): `crates/entity/src/inventory.rs:65-69,90-92`; `base/world_entry/methods/player_load/core/inventory_items.rs:40-42`. 2009 Python did the same (`deprecated/python/cell/Item.py:231-232`).
- Handler: `crates/services/src/cell/cell_methods/inventory/bandolier/ammo_change.rs:84` (`item_defs.get(&item_id)`, design-keyed) and `:94-120` (`item.item_id == item_id`, design id; "ambiguous" rejection plus TODO for a slot id).
- `BandolierItem.instance_id` exists and is the `sgw_inventory` PK (`crates/entity/src/cell_entity/mod.rs:107-119`, #520).
- The "slot token" hypothesis is refuted: `FUN_00e1fb20` (onActiveSlotUpdate) writes `SlotId` into a separate 0x1c-byte active-slot record.

## Acceptance criteria

- The handler resolves the slot by `bandolier_items[..].instance_id == ItemId` (at most one match). The ambiguous-duplicate rejection and the "add slot id to the message" TODO are removed.
- The whitelist lookup uses the matched item's design id (`item.item_id`) for `item_defs.get`, never the wire value.
- Unknown instance id → reject with a `warn!` (negative-log convention). Valid swap → persist + `onEntityProperty(AmmoTypeId)` as today.
- `docs/reverse-engineering/findings/inventory-wire-formats.md:242` gets a citation for the ctor/sender chain above. `BandolierItem.item_id` / handler comments are updated.
- Open PR #602 is rebased onto this or closed as superseded.

## Test type

Unit/cell: two bandolier slots holding the same design with different instance ids; a request with instance id A swaps only slot A. The guard must fail if matching reverts to the design id. Negative log (`LogCapture`) for an unknown instance id. The existing live-DB bandolier persistence guards stay green.

## Docs to update

`docs/reverse-engineering/findings/inventory-wire-formats.md`, `weapon-ammo-pipeline.md` (the `requestAmmoChange` rows), and `docs/gameplay/weapon-ammo-reload.md` if it describes the lookup.

## Client impact

Free. The server interprets a value the client already sends.

## Domain advisor

items-systems-advisor; server-authority-enforcer (whitelist source).

## Needs a human for

An in-game ammo-type swap before and after, to confirm the symptom and the fix.

## #314 — createBasePlayer omits SGWPlayer's BASE properties (account mailbox + perfStatsByChannel)

- Verdict: REWRITE
- Priority: P3
- Labels: remove `bug`; add `documentation`; keep `ready-for-human` → `ready-for-agent` after the rewrite
- Summary: The premise rests on a mislabelled byte. The client's `ServerConnection_createBasePlayer` (`0x00dddca0`, decompiled this triage) reads `EntityID` (4 bytes) and then a **2-byte `TypeID`**, and passes the remaining stream to the entity-create callback at `this+0x168`. There is no property-count byte. `docs/reverse-engineering/findings/entity-property-sync.md:229-247` documents the same (`EntityID u32 | TypeID u16 | property stream`). Rust's `body.push(0x00); // propertyCount = 0` (`mercury/world_data/phases.rs:50-51`) is really the high byte of the u16 TypeID (`class_id` 0x02/0x03), so it is correct on the wire and only the comment is wrong. On content: `account` and `perfStatsByChannel` are `<Flags>BASE</Flags>` (`SGWPlayer.def:41-44,269-273`), i.e. `DATA_BASE = 0x08` only, and the only binary anchor the chapter cites for the stream filter (`EntityDescription_WriteClientData` `0x01590fc0`) gates on `flags & 0x06`, which excludes BASE-only properties. The chapter contradicts itself (R5 / §1.11 say `0x0E` while quoting an `& 6` decompile: `entity-property-sync.md:504-521,727`). In stock BigWorld, BASE-only data (as opposed to BASE_AND_CLIENT) is server-private, and a MAILBOX is never client data. The `todo!()` stubs in `crates/entity/src/mailbox.rs:43,78,113` are server-internal routing and do not depend on a client-side mailbox. The current empty stream is therefore almost certainly correct. What remains is fixing a misleading comment and the chapter contradiction.
- Evidence:
  - Ghidra `0x00dddca0` (decompile): `read(4)` → id; `read(2)` → type; callback `(*this+0x168)(id, type, stream)`
  - `docs/reverse-engineering/findings/entity-property-sync.md:229-247`
  - `crates/services/src/mercury/world_data/phases.rs:45-51`
  - `entities/defs/SGWPlayer.def:41-44,269-273`; `docs/drafts/spec/entity-property-sync.md:217-221,504-521,727`
- Related/duplicates: #316 (same chapter / OQ-4 area)

### Action text

Comment:

> Re-scoping. The client's `createBasePlayer` handler (`0x00dddca0`) reads a 4-byte id and then a **u16 TypeID**, with no property count. So the byte `phases.rs:51` calls `propertyCount = 0` is the TypeID's high byte, and today's wire is right (`entity-property-sync.md:229-247` agrees). `account` / `perfStatsByChannel` are BASE-only (`0x08`), and the filter the chapter actually cites from the binary is `flags & 0x06`, so they are not client data. That also fits stock BigWorld, where a MAILBOX is never sent to the client. The chapter's `0x0E` / R5 wording contradicts its own decompile quote. Rewriting as a comment and doc fix. No OQ-4 RE is needed for this.

#### New body

## Problem

Two sources misdescribe `createBasePlayer` (msg `0x05`) and invite a harmful change (appending BASE properties to the client stream):

1. `crates/services/src/mercury/world_data/phases.rs:50-51` labels the high byte of the u16 `TypeID` as `propertyCount = 0`.
2. `docs/drafts/spec/entity-property-sync.md` says the stream filter is `CLIENT_DATA | BASE_DATA = 0x0E` (§1.11 `:516-521`, R5 `:727`) while citing a binary gate of `flags & 0x06` (`:504,519`).

## Evidence

- `ServerConnection_createBasePlayer` `ghidra://SGW.exe@0x00dddca0`: reads u32 id, then u16 type, and hands the stream to the `this+0x168` callback.
- `docs/reverse-engineering/findings/entity-property-sync.md:229-247`: `EntityID u32 | TypeID u16 | PropertyStream`.
- `EntityDescription_WriteClientData` `ghidra://SGW.exe@0x01590fc0`: `(flags & 6) != 0`.
- `SGWPlayer.def:41-44,269-273`: `account` (MAILBOX) and `perfStatsByChannel` are `BASE` only (`0x08`).

## Acceptance criteria

- `phases.rs` writes `class_id` as a `u16` LE `TypeID` (or keeps the two bytes with a correct comment). The wire bytes are unchanged, pinned by a byte-exact test.
- The chapter's §1.11 / R5 / Figure 6 caption state the filter the binary uses (`& 0x06`) and that the SGW stream is empty because no `.def` sets client bits. The 0x0E claim is removed or marked as the stock-BW reference. If the figure source changes, re-render its SVG.
- No property bytes are added to `createBasePlayer`.

## Test type

Wire-format (type 2): the `createBasePlayer` body for SGWPlayer and SGWGmPlayer is byte-exact (`05 06 00 <id u32> 02 00` / `03 00`).

## Docs to update

`docs/drafts/spec/entity-property-sync.md` (plus figure sync if the caption source changes).

## Client impact

Free. No wire change.

## Domain advisor

bigworld-engine-advisor.

## Needs a human for

Nothing.

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 735 | KEEP | P3 | The 0xFF WORD vs DWORD contradiction is still unresolved after #732. Needs client RE. The ticket's "stale wireclient comment" is actually about baseAppLogin. |
| 733 | KEEP | P3 | `Bundle::encode` still clamps to the 0xFFFF escape sentinel. #732 documented the escape. Latent hardening. |
| 359 | CLOSE (not planned) | P3 | ACK batching already exists (`pending_acks` drained on the next send or the 100 ms tickSync). The REPLY XOR gap only matters for server RPC, which isn't planned. |
| 353 | REWRITE | P2 | Downgrade loss was fixed by #357. Binary shows the client window/slot store is **512** (`Channel+0x2c=0x200`), so the "64 patch" would shrink it. Raise the server TX cap instead (free) and fix the bitmap docs. |
| 318 | CLOSE (completed) | P3 | Two-counter model is documented in draft §1.7 (d269e399 / #337). tickSync is back on the unreliable counter. |
| 298 | CLOSE (not planned) | P3 | The "512 dedup hash" is the client's receive reorder buffer. The server RX window is the same structure (64 vs 512), with no observed impact. |
| 316 | KEEP | P3 | Digest still a hardcoded const. Also `cfg.protocol_digest` / the `PROTOCOL_DIGEST` env override are dead code. |
| 302 | CLOSE (not planned) | P3 | restoreClientAck framing fixed (#389). No BaseApp fault-recovery flow exists in single-process Cimmeria. |
| 301 | CLOSE (not planned) | P3 | No need arose. The NA campaign settled on 0x10 (D-NA07). |
| 300 | CLOSE (not planned) | P3 | Bandwidth-only, and the OnGround variants are actively harmful (M6: pins NPCs at creation height). |
| 299 | CLOSE (not planned) | P3 | Rekey shipped via #575 without msg 0x00. Server AUTHENTICATE never observed. |
| 297 | CLOSE (not planned) | P3 | #732 showed no width can mis-frame today. The real hazard is tracked in #733. |
| 295 | REWRITE | P3 | All corrections still needed. Working code proves the chapter wrong on flags and sub-slot (per-entity idBase, 61 for SGWPlayer). Adds the #732 escape correction. |
| 293 | CLOSE (completed) | P3 | #711 split and renamed the constants. The 15 s is client-side tolerance and the original server used 300 s. |
| 171 | REWRITE | P3 | Still duplicated, and a third copy now exists in `console/net.rs`. CLAUDE.md names `method_idx` canonical, so reverse the recommended direction. |
| 727 | REWRITE | P2 | Every rejected DHD dial is silent. The 2009 server sent "Failed to dial" text. A free fix exists via method 100 or server chat. |
| 276 | CLOSE (completed) | P3 | #729 verified there is no emitter, so nothing can be mis-targeted. Follow-up is #727. |
| 720 | KEEP | P3 | Type-12/13 reload handler exists in the binary, but nothing proves the UI needs it. Needs RE. Sequence after #718. |
| 266 | CLOSE (not planned) | P3 | Premise disproved: the `.def` says INT32 Count, the alias type is unreferenced, and there is no serializer. |
| 271 | KEEP | P3 | Still 0.0 on main. PR #718 implements it and awaits owner review/UAT. |
| 534 | REWRITE | P1 | Decompile settles it: ItemId = **instance id** (item+0x0C = InvItem.id). The server matches on design id, so ammo swaps are likely silently ignored. Fix is free via `BandolierItem.instance_id`. |
| 314 | REWRITE | P3 | The "propertyCount" byte is the u16 TypeID high byte, and BASE-only props aren't client data (`& 0x06` gate). Reduce to a comment and doc fix. Do not add properties. |
