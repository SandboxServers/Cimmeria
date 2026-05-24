# Mercury Bundle abstraction

> **Last updated**: 2026-05-25
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
- **#360 follow-up — progression + teleport migrations**:
  - `handle_grant_xp` post-grant burst (1..2N+3 packets where N = levels
    gained) now rides one `ChannelBundle`. Player's own entity, no CREATE
    in-handler. Pinned by `grant_xp_max_level_burst_bundles_to_single_packet`
    + `grant_xp_single_level_burst_bundles_to_single_packet`. ✅
  - `handle_teleport_player` `FORCED_POSITION + onPlayerTeleport` handshake
    (2 → 1 packet). Pinned by
    `teleport_bundles_forced_position_and_player_teleport_to_single_packet`
    + a new `compose_forced_position_body_matches_build_forced_position_body`
    byte-equivalence guard for the raw-message bundle path. ✅
- **#360 follow-up — closed unmigrated (rationale, no code change)**: every
  remaining checkbox on issue #360 falls into one of three "no benefit"
  buckets:
  - **Already a single bundle** — `mercury/world_data/map_loaded.rs` is
    already two deliberate bundles via manual `build_fragmented_bundle`.
    Output is byte-identical to `ChannelBundle::finalize`. The caller in
    `base/world_entry/map_loaded.rs` can't use `send_bundle_to_witness_reliable`
    because `entity_to_addr` isn't populated until after the send (the
    write-ordering is deliberate — see the inline comment at line 170).
  - **No send sites** — `mercury/world_data/phases.rs` contains body
    builders (`build_create_player`, `build_enter_world_body`, etc.), not
    senders. The `append_entity_method` occurrences are inside body
    composition, not wire emits. Issue text counted body ops as sends.
  - **Single send per handler call** — these handlers each emit exactly 1
    packet per call: `mail/mod.rs` (1 per match arm), `vendor/store.rs`,
    `vendor/helpers.rs`, `inventory/core/mod.rs`, `inventory/grant/mod.rs`,
    `inventory/appearance.rs`, `cell_dispatch/minigame.rs`. Bundling a
    single send emits exactly 1 packet — no wire saving, no TX-window
    relief. The "grant + appearance recomposite" or "vendor open + price
    update" caller-level bursts WOULD benefit, but bundling them needs a
    helper-return-type refactor (compose into shared bundle rather than
    self-send), which is a separate concern.
  - **Utility builder** — `mercury/aoi/method.rs::build_entity_method_packet`
    is a single-packet wire builder; bundling is the caller's job, not the
    builder's. The byte-equivalence guard added in #363
    (`channel_bundle_append_entity_method_matches_build_entity_method_packet_body`)
    already pins that the bundle path and standalone path produce identical
    bytes, so caller-side migrations can swap freely.
  - **Needs batching layer** — `cell_dispatch/aoi.rs` `EntityMethodCall`
    fanout (per [Cadacious's scope-clarification comment on #360]) needs
    cross-message batching at the cell→base channel drain pass, not
    per-handler. Tracked separately; not a per-handler migration. The
    "EnteredAoI(X) followed by EntityMethodCall(X) for same X in the same
    drain pass" interleave requires the audit-and-split logic the
    `flush_deferred_aoi` path already implements; lifting that pattern to
    every drain-pass send is a separate, larger refactor.

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

Plus the `resend_appearance_after_cinematic` 2-packet pair (BeingAppearance
+ onEntityTint), called both from `handle_cancel_movie` (single-shot) and
from the cinematic-guard spam loop in `send_cinematic` (every 100 ms for up
to 20 s — 200 iterations × 2 packets = 400 reliable packets worst-case,
now 200 packets). For a first-login cinematic that runs the full spam
window (e.g. SGWLogo intro at 13.10 s natural end + 7 s safety buffer),
that's a ~50% reduction in cinematic-guard TX-window pressure.

Progression (`handle_grant_xp`) collapses every grant into one packet:

| Grant shape | Pre-bundle | Post-bundle |
|---|---:|---:|
| No-level grant (steady-state XP) | 1 | 1 |
| Single-level grant | 5 | 1 |
| Max-level catch-up (19 levels) | 41 | 1 |

Teleport (`handle_teleport_player`) collapses the engine-snap + load-hint
handshake:

| Pre-bundle | Post-bundle |
|---:|---:|
| FORCED_POSITION + onPlayerTeleport (2 packets) | 1 packet |

Safe per the transaction-state rule because the player entity was created
in `handle_map_loaded`'s prior bundle and its transaction released at the
prior bundle's end-of-frame; this bundle is exclusively post-transaction
property/method updates. Regression-guarded by
`on_client_ready_burst_bundles_to_single_packet` (pins `num_messages =
2 + DEFAULT_CHAT_CHANNELS.len() + 1` and `estimated_packet_count() == 1`)
and `appearance_resend_bundle_collapses_to_single_packet` (pins
`num_messages == 2` and `estimated_packet_count() == 1`) — both in
[base/world_entry_appearance.rs](../../crates/services/src/base/world_entry_appearance.rs).
Plus the entity-method byte-equivalence guard at
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
