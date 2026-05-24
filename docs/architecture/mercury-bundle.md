# Mercury Bundle abstraction

> **Last updated**: 2026-05-24
> **Audience**: Engineers touching Mercury send paths, AoI fanout, world-entry
> bursts, or anything that calls `send_to_witness_reliable`
> **Type**: Architecture decision + reference for callers
> **Owner**: Mercury / network
> **Related issues**: #356 (this work), #354 (umbrella TX-window relief),
> #357 (deferred-send queue, sibling), #353 (long-term TX-window widening)

## TL;DR

`ChannelBundle` ([crates/mercury/src/channel_bundle.rs](../../crates/mercury/src/channel_bundle.rs))
is an **accumulator** that lets a caller compose N application-level messages
into one fragmented Mercury bundle so the client processes them as a single
frame. The wire savings come from collapsing the per-packet IP+UDP header
overhead (~28 bytes × N) and from consuming fewer slots in the per-channel
TX window.

**Critical rule**: one bundle == one client frame. `CREATE_ENTITY(X)` puts
entity X into a creation "transaction" that holds for the rest of the bundle,
and same-entity messages later in the same bundle hit the client's
HOLD-FOR-TRANSACTION path and are silently dropped. Cross-entity batching is
safe; same-entity-after-CREATE batching is not.

Caller-owned, not channel-owned: the existing deliberate two-bundle split in
[base/world_entry/map_loaded.rs](../../crates/services/src/base/world_entry/map_loaded.rs)
exists because the transaction-state hazard demands that the bundle-boundary
decision sits with the caller, not with a per-channel auto-accumulator.

## Status

- **Layer A**: `ChannelBundle` lives in `crates/mercury` with 11 wire-format
  tests. ✅
- **Layer A.5**: `send_bundle_to_witness_reliable` bridge in
  [base/helpers.rs](../../crates/services/src/base/helpers.rs) ties the
  bundle to the session UDP socket + Channel TX-window registration. ✅
- **Layer B (conservative slice for #356)**: the AoI EnteredAoI burst in
  [base/world_entry/cell_dispatch/aoi.rs](../../crates/services/src/base/world_entry/cell_dispatch/aoi.rs)
  now bundles into 2 cross-entity bundles (phase-1, phase-2) instead of
  2 packets per NPC. Pinned by a regression test at 28 NPCs ≤ 15 packets
  (was 56 pre-bundle). ✅
- **Layer C**: regression guards in place (compose↔build byte-equivalence,
  28-NPC burst budget). ✅
- **#360 follow-up — world_entry_appearance migration**: the
  `handle_on_client_ready` 11-packet burst (1 BeingAppearance + 1 onEntityTint
  + 8 onChatJoined + 1 onPlayerCommunication welcome) and the
  `resend_appearance_after_cinematic` 2-packet pair now ride a single
  `ChannelBundle` each. Safe per the rule below: every message in both
  bursts targets the player's own entity_id, which was created in
  `handle_map_loaded`'s prior bundle, so the CREATE_BASE_PLAYER transaction
  released between bundles. Pinned by
  `on_client_ready_burst_bundles_to_single_packet` (11 → 1 packet) and a
  new compose↔build byte-equivalence guard for the entity-method bundle
  path covering both direct and extended encodings. ✅
- **Deferred to follow-up [#360](https://github.com/SandboxServers/Cimmeria/issues/360)**:
  remaining per-handler migrations across world-data/phases.rs, inventory/
  vendor/mail, progression, teleport, and the remaining AoI handlers. Each
  has its own transaction-state surface to audit and warrants a separate,
  reviewable PR.

## The transaction-state rule

When the client processes a `CREATE_ENTITY(X)` (or `CELL_PLAYER` for the
player entity) inside a bundle, entity X enters a creation transaction. For
the remainder of that bundle, certain same-entity messages — `BeingAppearance`,
`onStatUpdate`, `onLevelUpdate`, and others that require X to be fully
created before they apply — hit the client's HOLD-FOR-TRANSACTION path and
are dropped. The transaction releases at the end of the bundle, and the same
messages in a subsequent bundle apply normally.

**Evidence**: the comment block at
[base/world_entry/map_loaded.rs lines 66-79](../../crates/services/src/base/world_entry/map_loaded.rs#L66)
documents this directly:

> Previously we combined everything into one fragmented bundle, which caused
> BeingAppearance to be silently dropped (HOLD FOR TRANSACTION) because the
> entity was still in its creation transaction during bundle processing.

That deliberate two-bundle split is the source of truth on the rule.

### Safe to combine in one bundle

- `CREATE_ENTITY(A)` + `CREATE_ENTITY(B)` + … — different entity ids; A's
  transaction doesn't affect any subsequent record targeting B.
- Cross-entity AoI updates: `EnteredAoI(A)` + `EnteredAoI(B)` + … (each
  EnteredAoI body is its own CREATE_ENTITY + UPDATE_AVATAR, and
  cross-entity batching is what the AoI burst migration exploits).
- Multiple property updates for an entity already created in a **prior**
  bundle (the transaction released between bundles).

### Unsafe (silently dropped by client)

- `CREATE_ENTITY(A)` + `BeingAppearance(A)` in the same bundle.
- `CREATE_ENTITY(A)` + `onStatUpdate(A)` in the same bundle.
- `CELL_PLAYER` + any same-player entity method in the same bundle.

When in doubt, split into two bundles. The cross-entity savings vastly
outweigh the intra-entity ones.

## Why caller-owned, not channel-owned

The original `#354` umbrella proposal sketched a per-channel auto-accumulator:
every `send_*` call appends to a `Channel::current_bundle()`, flushed on
tick boundary. That design is unsafe given the transaction-state rule —
every `CREATE_ENTITY` would collide with whatever same-entity message
happened to land in the same tick, silently dropping it.

Caller ownership keeps the bundle-boundary decision in the caller's hands,
right where the transaction-state semantics are visible. The AoI flush
explicitly splits into a phase-1 bundle and a phase-2 bundle for this
reason; an auto-accumulator could not know to split.

## Wire format

The bundle body is the same byte layout the services-layer
`append_entity_method` produces:

- Direct (method_index 0–60):
  `[(index | 0x80): u8][word_len: u16 LE][entity_id: u32 LE][args...]`
- Extended (method_index ≥ 61):
  `[0xBD: u8][word_len: u16 LE][entity_id: u32 LE][(index - 61): u8][args...]`

Multiple messages concatenate in append order. On `finalize`, the body
goes through `crate::packet::build_fragmented_bundle` which:

- emits one non-fragmented packet for bodies ≤ `FRAGMENT_BODY_SIZE`
  (1300 bytes — Mercury's `MAX_BODY_LENGTH` is 1411, minus per-packet
  footer overhead of flags(1) + seq(4) + frag_begin(4) + frag_end(4) +
  ack headroom, and AES-256-CBC encryption overhead of up to 16 bytes
  PKCS7 padding + 16-byte HMAC. The 111-byte slack on `MAX_BODY_LENGTH`
  is what keeps the encrypted datagram under the 1472-byte
  PACKET_MAX_SIZE = UDP MTU-safe size. See the constant doc in
  [crates/mercury/src/packet/build.rs](../../crates/mercury/src/packet/build.rs).);
- otherwise emits `ceil(body / 1300)` fragments, each carrying
  `FLAG_FRAGMENTED` + matching `frag_begin` / `frag_end` footers;
- piggybacks the bundle's ACKs **only on the first fragment**;
- masks every per-fragment seq, `frag_begin`, and `frag_end` against
  `SEQUENCE_MASK` so a `base_seq` near the 28-bit wrap point cannot
  silently emit `seq >= NULL_SEQUENCE` (which the peer parser drops as
  an R4 violation, killing reliable delivery for the whole bundle).

Byte-equivalence with the standalone-packet builders is pinned by tests in
[crates/services/src/mercury/aoi/tests.rs](../../crates/services/src/mercury/aoi/tests.rs)
(`compose_create_entity_base_body_matches_build_create_entity_base_body`
and the cascade variant).

## TX-window interaction

`ChannelBundle::finalize` returns N packets and `seqs_consumed = N`. The
caller (via `send_bundle_to_witness_reliable`) atomically reserves N
consecutive sequence numbers from the per-session counter, sends each
fragment, and registers each with the per-channel TX window via
`shadow_register_reliable_send`. If the TX window is full, the deferred-send
queue from issue #357 absorbs the overflow — `register_sent_packet` never
returns the silent best-effort path anymore.

`estimated_packet_count()` is a load-bearing contract: it must equal
`finalize().packets.len()` for any given bundle state. Violating that
contract makes the send helper either **over-reserve** seqs (creating
permanent gaps in the reliable stream that stall every subsequent
reliable packet behind them) or **under-reserve** seqs (causing
collision with later sends from concurrent threads).

The helper drains pending ACKs into the bundle **before** consulting
`estimated_packet_count()` under the same lock window as the seq
reservation, then `finalize()` runs without further mutation — so the
estimate reflects the exact post-drain state at reservation time and
no TOCTOU window opens between estimate and finalize. The contract is
guarded by a `debug_assert!` (the post-finalize check in
[base/helpers.rs](../../crates/services/src/base/helpers.rs)) and by
the boundary-case test
`estimated_packet_count_matches_finalize_at_fragment_boundary_with_acks`
in [crates/mercury/src/channel_bundle.rs](../../crates/mercury/src/channel_bundle.rs).

`estimated_packet_count()` itself depends only on `body.len()`
(fragmented) or the empty-body-with-acks special case (which always
emits exactly 1 packet); ACK count never affects the fragment count
because ACKs ride only the first fragment.

## Observed win

The conservative Layer B slice migrates only the AoI `EnteredAoI` burst.
For a Castle_CellBlock instance with 28 NPCs:

| Pre-bundle | Post-bundle |
|---:|---:|
| 28 × CREATE_ENTITY/UPDATE_AVATAR packet | 1 cross-entity phase-1 bundle, ~1 KB body, 1 packet |
| 28 × cascade packet | 1 cross-entity phase-2 bundle, ~12 KB body, ~10 fragments |
| **56 reliable packets** | **~11 reliable packets** |

Regression-guarded at "≤ 15 packets" in
[base/world_entry/cell_dispatch/tests.rs](../../crates/services/src/base/world_entry/cell_dispatch/tests.rs)
(`flush_deferred_aoi_bundles_28_npc_burst_under_packet_budget`) with
comfortable headroom for cascade-payload growth.

The #360 follow-up extends the same shape to the post-`onClientReady`
appearance / chat / welcome burst on every world entry:

| Pre-bundle | Post-bundle |
|---:|---:|
| 1 × BeingAppearance + 1 × onEntityTint + 8 × onChatJoined + 1 × welcome | 1 same-entity bundle, ~700 B body, 1 packet |
| **11 reliable packets** | **1 reliable packet** |

Safe per the transaction-state rule because the player entity was created
in `handle_map_loaded`'s prior bundle and its transaction released at the
prior bundle's end-of-frame; this bundle is exclusively post-transaction
property/method updates. Regression-guarded by
`on_client_ready_burst_bundles_to_single_packet` in
[base/world_entry_appearance.rs](../../crates/services/src/base/world_entry_appearance.rs)
(pins `num_messages = 2 + DEFAULT_CHAT_CHANNELS.len() + 1` and
`estimated_packet_count() == 1`) and by the new entity-method byte-
equivalence guard at
[mercury/aoi/tests.rs](../../crates/services/src/mercury/aoi/tests.rs)
(`channel_bundle_append_entity_method_matches_build_entity_method_packet_body` —
covers both direct and extended encodings via ON_PLAY_MOVIE = 155).

## Migration playbook for follow-up call families

When migrating another call family (the issue's deferred list):

1. **Audit the transaction-state surface.** List every entity the caller
   touches in this code path and whether any CREATE_ENTITY (or
   CELL_PLAYER) is mixed with same-entity messages. If yes, the migration
   needs a bundle split, not a single bundle.
2. **Extract body-only composer.** Refactor the existing
   `build_<thing>_packet` into a `compose_<thing>_body() -> Vec<u8>` that
   omits framing + encryption, then have `build_*` call it + add framing.
   Mirror the pattern from
   [mercury/aoi/create.rs](../../crates/services/src/mercury/aoi/create.rs)
   (`compose_create_entity_base_body` / `compose_create_entity_cascade_body`).
3. **Add a byte-equivalence regression guard** comparing `compose_*` output
   against the decrypted body portion of the standalone `build_*` packet.
   Mirror the pattern in
   [mercury/aoi/tests.rs](../../crates/services/src/mercury/aoi/tests.rs).
4. **Migrate the caller** to build a `ChannelBundle`, append composed
   bodies, and send via `send_bundle_to_witness_reliable`. Keep
   non-burst-shaped one-off sends on `send_to_witness_reliable`.
5. **Add a burst-shape regression guard** asserting the new packet count is
   below the pre-migration count. Mirror the pattern in
   [cell_dispatch/tests.rs](../../crates/services/src/base/world_entry/cell_dispatch/tests.rs).

## What did NOT change

- Wire format. The bundle body is byte-identical to a concatenation of
  per-message bodies that the existing `append_entity_method`
  produces. Pinned by tests.
- Reliability. Bundled packets ride `FLAG_RELIABLE | FLAG_ON_CHANNEL` and
  register with the TX window the same way as pre-migration packets.
- Sequence counter. Bundle reserves N consecutive seqs via
  `next_seq.fetch_add(N, Relaxed)` — same counter as the per-packet path,
  no reordering relative to interleaved per-packet sends.
- The deferred-send queue ([#357](https://github.com/SandboxServers/Cimmeria/pull/357))
  still backstops TX-window overflow. Bundles plus the queue stack: if a
  bundle finalizes into more packets than the remaining TX window, the
  bytes still go on the wire and bookkeeping queues for retransmit
  promotion as ACKs free slots.
