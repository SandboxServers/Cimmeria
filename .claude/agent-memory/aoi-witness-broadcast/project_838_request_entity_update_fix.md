---
name: project-838-request-entity-update-fix
description: requestEntityUpdate (0x07) parser fix + cell behavior redesign (issue #838, RE in #1000); PR fixing crates/base connect_loop/encrypted parser and crates/cell request_entity_update.rs
metadata:
  type: project
---

2026-09-28: fixed `requestEntityUpdate` (msg 0x07) end to end. The parser at
`crates/base/src/base/connect_loop/encrypted/mod.rs::parse_request_entity_update`
had assumed `[u32 header][N × u32 entity_id]`; the real wire is `[u32
entityId][N × u32 cacheStamp]` with N always 0 on the live client, so every
real payload (a bare 4-byte entity id) decoded to an empty `Vec` and the
whole handler had never run on production traffic (colo: 0/2,275 decoded).

**RE evidence** (headless Ghidra, no GUI/MCP needed — the project already
existed at `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\binaries\SGW.gpr`):
full writeup in `docs/reverse-engineering/findings/request-entity-update-cache-stamp.md`.
Key facts for future AoI work:

- `EntityManager::onEntityEnter` (`0x00dd24f0`, vtable slot 3 of the
  corrected `GameEntityManager` vtable) fires this once per non-player
  entity per AoI entry, deduped against a last-id/last-object pair. It does
  **not** wait for a reply before treating the entity as usable — the
  entity is already visible (via `EntityManager_enterWorld`, called
  synchronously in the same function) regardless of what the server does
  with this message.
- The `GameEntityManager` vtable base is `0x019aaec4`, not `0x019aaeb8` —
  proven by the RTTI Complete Object Locator pointer at `0x019aaec0`
  (`vtable - 4`, a data pointer, not code). A 2026-05-16 audit appendix
  (`entity-property-sync-section2-audit-2026-05-16.md` Appendix E.4) had
  used the wrong base and called two correct Ghidra plate comments "doubly
  wrong" — that correction is now retracted in the same doc.
- `EntityManager_LeaveAoI` (`0x00dd29d0`)'s Path A/B was inverted in
  `docs/drafts/spec/entity-property-sync.md` §1.10: **found** in the
  primary map → immediate dispatch; **not found** → deferred into the
  `GameEntityManager+0x3C` buffer. Fixed in the same PR.
- `0x0B`'s shared trampoline (`0x00de1c90`) is now fully decompiled (thin
  forwarder to a per-instance `this+4` callback) but the callback's
  identity is still open — needs a live x64dbg trace, not static RE.

**Design decision on cell behavior (my judgment call, not literally spelled
out in the issue — flagged in the PR for the owner to veto):** since 0x07
fires identically on every routine AoI entry with no field that
distinguishes "just entered" from "genuinely missing state," the cell
handler (`crates/cell/src/cell/service/base_messages/request_entity_update.rs`)
now does:

- id **in** the witness's current AoI → **answer nothing** (no
  `EnteredAoI`, no cascade). This is the overwhelming normal case.
- id **not in** the witness's AoI → **refuse** (unchanged anti-probe
  posture from PR #390 — a witness must not pull another entity's state by
  asserting an arbitrary id). Now logs a `not_in_witness_aoi` WARN instead
  of being silent.

This **retires PR #390's re-create-on-request recovery path** (it
unconditionally re-emitted `EnteredAoI` + cascade, including a pet-list
replay for A-23 and a duel PvP-flag replay for SS-D2, whenever the
requested id was in AoI). That path had never actually run in production
(the parser dropped everything before this fix), and the normal AoI tick
already does its own pet-list/duel-flag replay on first entry
(`crates/cell-world/src/cell/space_manager/aoi.rs`), so removing the
duplicate from this handler doesn't touch normal delivery — only the
untested recovery safety net. If a real create-delivery-confirmation
recovery mechanism is needed later (e.g. for [[project_invisible_cellblock_guard]]),
it needs a different signal than 0x07 — Mercury delivery confirmation or a
periodic AoI-consistency resync — since 0x07 provably can't distinguish the
two cases.

**Invisible Cellblock guard (#838/#849) assessment:** plausibly *not* fixed
by this change alone. The RE shows the client renders the entity from its
own create-time state regardless of 0x07's answer (no wait-for-reply
gate), so a client that already fails to render from `CREATE_ENTITY` alone
won't be healed by anything this handler could send. See
[[project_invisible_cellblock_guard]] for the open investigation.

Tests: byte-exact parser pins in `crates/base/.../encrypted/tests.rs`
(`parses_bare_entity_id_with_no_cache_stamps` is the regression guard —
confirmed it fails against the reverted header-skipping parser, 8 of the
new/adjacent tests failed on revert), a `payload_too_short` negative-log
guard in `rx_order_tests.rs`, and known/unknown-path + negative-log guards
in `crates/cell/.../tests/request_entity_update.rs`. The old
`request_entity_update_replays_pet_lists_to_the_owner_only` test was
removed (its code path no longer exists); `pet_create_on_client_events`
still has independent coverage in
`crates/cell-world/src/cell/pets/tests/create_on_client.rs` for the real
(AoI-tick) delivery path.
