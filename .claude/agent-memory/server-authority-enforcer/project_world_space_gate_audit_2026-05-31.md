---
name: project-world-space-gate-audit-2026-05-31
description: CAT-O audit findings; gate/ring/region/world trust posture as of 2026-05-31
metadata:
  type: project
---

CAT-O (World/Space/Gate/Ring) audit completed 2026-05-31. Six findings filed.

**Fact**: The gate-travel handler (`crates/services/src/cell/gate_travel.rs`)
validates the destination against the *global* `space_mgr.stargates` catalog
only — it ignores the player's `knownStargateAddresses` (CELL_PRIVATE on
GateTravel.def, persisted to `sgw_player.known_stargates`, hydrated into
`PlayerLoadData::known_stargates` at world entry but never plumbed onto the
CellEntity). This is **distinct** from [[CAT-B-02]]'s source-position gap;
both must be fixed independently. CAT-O-01.

**Fact**: `onWorldInstanceReset` is declared on the **player** entity
(`SGWPlayer.def:868-870`, `<Exposed/>`) — NOT on `SGWGmPlayer.def`. The
naming implies GM but the wire surface is open to every client. Currently
UNIMPLEMENTED in Rust as a no-op stub; the trap fires when a future
contributor implements it without an access-level check. CAT-O-04, links to
[[reference_gm_auth_plumbing_gap]] for the systemic dispatch-layer fix.

**Fact**: `triggerClientHintedGenericRegion` has *no rate-limit and no
per-region debounce* in addition to the CAT-B-04 position-discard issue.
A client can chain-fire enter/exit packets across multiple region IDs at
packet rate, walking the content-chain engine through arbitrary mission
sequences in seconds. CAT-O-03.

**Fact**: `cancelMovie` (`Event_NetOut_CancelMovie`, wire shape
`WSTRING MovieName`) is parsed by Cimmeria but the `MovieName` argument is
**discarded** — the handler flips the global `cinematic_spam_cancel` flag
regardless of which movie the client claims to cancel. Today only the
first-login intro uses `send_cinematic`, so the bug is latent; the in-code
comment at `world_entry_appearance.rs:533-536` explicitly anticipates
"future cinematics (mission cutscenes, gate transitions, dialog overlays)"
through the same path. CAT-O-05.

**Fact**: `updateSystemOptions` is **structurally safe today** — apply
allowlist is closed at `crates/entity/src/cell_entity/system_options.rs:59-71`
to just `autoReload` and `reloadOnActivate`, both advisory booleans, and
the wire parse is count-capped at 256. The trap is forward-looking:
SystemOptions.xml has ~140 options and the wire `(WSTRING name, WSTRING
value)` makes adding a server-authoritative option (PvP-flag, accept-duel-
auto, etc.) to the apply match a one-line PR with privilege-escalation
consequences. CAT-O-06.

**Fact**: `onSpaceQueuedResponse`/`onSpaceQueueReadyResponse`/
`onSpaceQueueStatus`/`onStrikeTeamResponse` are all **completely unwired**
on the server — no cell-method arm, no dispatch entry. Server→client emits
exist (`ON_SPACE_QUEUED` 146, `ON_SPACE_QUEUE_READY` 147) but the inbound
responses go through the "unhandled cell method" warn path. No exploit
until implementation; flagged in CAT-O "Not Filed".

**Fact**: `Event_NetOut_DHD` and `Event_NetOut_onDialGate` map to the
*same* server-side wire — cell method 35, `handle_dial_gate`. The C++
client's `Event_SlashCmd_DHD` and the in-game dialer UI both emit to that
single cell method. There is no separate `dhd` cell method in the SGWPlayer
or GateTravel def.

**Why this matters**: CAT-O's residual exploit shape is "destination
authorization" — every position-anchored fix from CAT-B leaves the
destination-authorization gap untouched. Any future review of gate/ring
travel must verify both *source* and *destination* authority separately;
the source-position fix alone does not stop CAT-O-01.

**How to apply**: When reviewing future gate/ring/region handlers, run
this three-check pass:
1. Source authority (CAT-B-02/03/04 angle): is the player actually at the
   place they're claiming to depart from?
2. Destination authority (CAT-O-01 angle): does the player have the
   destination unlocked?
3. Transaction window (CAT-O-02 angle): is there a cancel/cost gate
   between accept-press and irreversible-mutation?

Links: [[reference_world_space_gate_wire_spec]],
[[reference_gm_auth_plumbing_gap]], [[reference_cell_method_entity_id_authority]].
