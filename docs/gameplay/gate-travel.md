---
title: "Gate Travel System"
type: reference
audience: engineers
last_updated: 2026-10-04
---

# Gate Travel System

> **Last updated**: 2026-10-04 (Debug Area dial-out, DA-07)
> **Status**: Zone transition and ring transport both work. Two of the fourteen `Stargate_*` sequence events (6100, 6113) now fire and fan to witnesses; `onStargatePassage` (client method 68) is now sent to the crossing player; the world transition after a walk-through crossing is deferred behind a movement-locked, timeout-bounded hold so a Kismet cinematic has a scheduled window to play. DHD chevrons and squad travel are still missing. **Evidence-policy correction (NA35, 2026-09-25):** the deprecated legacy server never had working gate travel end to end, so "the 2009 server did/didn't emit event X" is not evidence about the 2009 client's expectations. `docs/reverse-engineering/findings/stargate-dial-and-travel-sequences.md` re-grounds the dial-timing and gate-crossing-cinematic questions in the client binary alone; D-CA20 in `docs/analysis/castle-rebuild/README.md` records the correction and supersedes D-CA10's framing without editing it.

## Overview

Gate travel enables zone transitions via stargates and ring transporters. Stargates provide long-distance travel between worlds, while ring transporters provide local teleportation within or between nearby areas. Both systems involve multi-step sequences with animations, player visibility toggling, and movement locking.

Stargate zone transition is implemented in [`base/world_entry/gate_travel/`](../../crates/base-world-entry/src/base/world_entry/gate_travel/): on `CellToBaseMsg::GateTravel` the base sends RESET_ENTITIES to tear down the client's view of the old space, persists the destination world and position, and seeds `pending_world_entry` so the client's next ENABLE_ENTITIES drives a fresh create-player + enter-world cycle. Ring transport lives in [`cell/ring_transport/`](../../crates/cell-content/src/cell/ring_transport/) with an 8-state finite state machine.

> **Where the placement is chosen.** Castle CA10 split the dial from the crossing: `onDialGate` arms a 4-second dial and the player then walks into the `REGION_FLAG_Stargate` (bit 2) volume to cross. Both that crossing and the no-gate-volume immediate fallback funnel through one function, `cell::gate_travel::perform_gate_travel`, which holds the single `validate_gate_arrival` call. There is deliberately exactly one — a second call in either caller would validate, and warn, twice per crossing.

## Arrival placement

A `resources.stargates` row's `x_pos` / `y_pos` / `z_pos` / `yaw` is the **stargate prop's own transform** — the `GLB-Stargate_Prefab_Seq` origin lifted out of the cooked map. The 2009 server arrived travellers on exactly that point and never validated it (`deprecated/python/cell/SGWPlayer.py:2129`). On a navmesh-backed world the prefab origin is usually inside the prefab's own footprint carve-out: at Harset it sits ~1.5 units above the floor with the nearest walkable vertex ~5 units away in XZ, well outside the ±3.0 search extents. An arriving player therefore lands off-mesh, every position update they send is suppressed, and witnesses see a frozen avatar while the only log is a `CorrectionSuppressed` with no obvious cause.

`resources.stargates` now carries four nullable columns — `arrival_x`, `arrival_y`, `arrival_z`, `arrival_yaw` — holding an absolute "stand here on arrival" point pinned in-game. All four are set or all four are `NULL`, enforced by the `stargates_arrival_all_or_nothing` CHECK; when `NULL` the arrival falls back to the gate row, `yaw` included. No gate is pinned today. Harset's gate 3 carried a pin placed from map data (PL-A-01) from 2026-09-19 until 2026-09-25, when NPC-AI NA29 dropped it: the NA26 `harset.nav` has the gate dais, so the gate row is standable and travellers arrive on it, the way the 2009 server did (see [the ledger](../analysis/harset-rebuild/placements/A-arrival-and-travel.md#pl-a-01--the-pin-was-dropped-na29)).

[`cell/arrival.rs`](../../crates/cell-world/src/cell/arrival.rs) resolves the final placement. `validate_gate_arrival` is the gate-specific wrapper; `resolve_arrival` is the gate-agnostic core. (Ring transport calls the *validate-only* half, `check_arrival`, and never the respawner substitution — a ring pad is a pad the client is animating at, not a pin on a prop transform. See [ring-transport-system.md](ring-transport-system.md#bounded-aborts-cimmeria-not-2009).) The order is:

1. The authored `arrival_*` pin, or the gate row when there is no pin.
2. If the destination world has a **resident** navmesh, the point must pass both `NavMesh::is_point_valid` and the space AABB derived from the mesh extents. Both layers, because a point that is on-mesh but outside the AABB is hard-rejected by the very next client packet — with the correction budget already cleared by the authorised teleport, which is how a "recovery" turns into a permanent freeze one position over.
3. On failure, the **nearest** authored respawner for that world that is not a placeholder `(0,0,0)` row and that passes the same two checks. Logged at `warn` with `reason = "arrival_off_navmesh"` naming the world and both coordinates, so the operator-actionable seam is "re-pin this gate".
4. If nothing qualifies, there is **no arrival** and the dial is refused. The failure is logged at `error` with `reason = "arrival_unrecoverable"`, and the fix is to seed a respawner for the world or re-pin the gate.

The outcome is reported as an `ArrivalSource`: `Validated`, `Respawner`, `UnrecoverableOffMesh`, or `Unvalidated`. The last one covers destinations with nothing to check against — a world with no `.nav` file, or an **instanced** destination, which has no space until one is created and so never appears in `world_spaces`. Those arrivals are accepted as-is and logged at `debug`.

**`UnrecoverableOffMesh` carries no usable position.** `ResolvedArrival::position` still echoes the *rejected* input so the caller's warn can name it, but `ResolvedArrival::is_usable()` is false and every caller that moves a player gates on it. `perform_gate_travel` refuses and enqueues no `GateTravel` — on the crossing and on the immediate-travel fallback alike: the traveller keeps the position they had, which they can at least walk out of. Handing the rejected point to the transfer is the same silent freeze one layer further along — the traveller is torn out of a world they *could* stand in and re-created off-mesh on one they can't. `gmDHD` reports the refusal to the GM rather than the unconditional "dialing gate address N" it used to print for every outcome.

The helper deliberately does **not** call `NavMesh::get_nearest_point`. That returns its input unchanged on a miss, so its output can never be trusted without re-validating it; and an arrival Detour *can* reproject is an authored pin that is a metre or two wrong and should be corrected at authoring time rather than papered over on every arrival.

The yaw is carried through unchanged even when the position falls back to a respawner — respawner rows have no yaw of their own, and the authored gate facing is the only non-arbitrary answer.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| DHD UI display | DONE | `setupStargateInfo` sends the gate list at world entry; right-clicking a DHD prop (`INT_DHD`, bit 16) emits `onDisplayDHD` (120) — see below |
| Gate arrival validation | DONE | Per-gate `stargates.arrival_*` pin, validated against the destination navmesh with a respawner fallback — see [Arrival placement](#arrival-placement) |
| Walking into the gate | DONE | `REGION_FLAG_Stargate` (bit 2) region routing plus the 4-second dial timer (CA10); the crossing shares the dial's arrival validation |
| Stargate address tracking | DONE | `knownStargateAddresses` property, give/remove |
| Known-address enforcement on dial | DONE | `handle_dial_gate` refuses an address not in `known_stargates` with `onErrorCode` 180 — see [Dial authorization](#dial-authorization) |
| Address unlock on arrival | DONE | A committed arrival learns both worlds' gates in the destination-persistence UPDATE. **New behaviour, not 2009** — see [Address unlock on arrival](#address-unlock-on-arrival) |
| Stargate zone transition | DONE | `base/world_entry/gate_travel/` — RESET_ENTITIES, persist destination, replay world entry |
| Stargate dial timer | DONE | `onDialGate` arms a 4 s timer (`SGWPlayer.beginDialing`) instead of travelling; drained by `cell::gate_travel::gate_dial_tick` on the 100 ms cell tick |
| Stargate walk-through crossing | DONE | Travel fires when the player enters the gate's `REGION_FLAG_Stargate` volume, not on the dial. Worlds with no such region fall back to travelling on the dial |
| Gate-travel contact event | DONE | Fires `ECONTACT_LIST_EVENT_GateTravel` to the traveller's contacts with the destination `world_id` |
| Ring transporter interaction | DONE | Sends the destination list |
| Ring transport FSM | DONE | 8-state machine: IDLE through COOLDOWN |
| Ring player teleportation | DONE | Position-based teleport with visibility toggle |
| Ring Kismet sequences | DONE | `Region_Teleport_Out` / `Region_Teleport_In` |
| Ring movement locking | DONE | `BSF_MovementLock` set/unset during transport |
| Ring cross-world transport | PARTIAL | Same-world works; cross-world path exists but untested |
| Ring multi-player sync | FIXME | Only the first player in the region gets the Matinee — the sequence drives a shared world prop |
| Stargate open animation | DONE | `Stargate_MakeGate` (6100) fires on the next 100ms cell tick after a successful dial (`GATE_DIAL_DURATION`, retimed NA35 2026-09-25 from a 4s hold with no client-binary support — D-CA20). No confirmed client-side duration exists to replace it with, so this is the tick-drain architecture's minimum, not a measured number. `Stargate_DestroyGate` (6103) stays unemitted; whether it should be is unresolved, not settled by D-CA10's now-superseded framing |
| Stargate crossing animation | DONE | `Stargate_CrossGate` (6113) and `onStargatePassage` (client method 68) fire on entering the gate volume; the `GateTravel` world transition is then deferred behind a movement-locked `CROSSING_CINEMATIC_HOLD` (1.5s, provisional — NA35) so the client has a scheduled window to render whatever Kismet cinematic the sequence resolves to before `RESET_ENTITIES` tears the view down. The hold's exact duration is unverified against the client's Matinee data; the mechanism (race closed, lock released on a failed deferred travel, hold cancelled on disconnect) is tested |
| `onStargatePassage` (client method 68) | DONE | Declared since before NA35 (`ON_STARGATE_PASSAGE` constant) but never sent; now sent to the crossing player only, immediately after `Stargate_CrossGate` |
| DHD chevron lock animations | NOT IMPL, and not implementable server-side | Events 6106–6112 exist in the DB for every gate. NA35 confirmed the DHD dial UI never reports in-progress glyph selection to the server (`onDialGate` carries only the finished address) — there is no wire-level signal to key a server-driven chevron broadcast on, so this is a client-patch-only feature, not merely an unimplemented one |
| Stargate witness visibility | DONE | Both gate sequences fan to every witness of the dialer plus the dialer, one `onSequence` each. The 2009 server sent to `self.client` only; this is a deliberate addition |
| Squad leader gate travel | NOT IMPL | `processSquadLeaderGateTravel` defined; blocked on the group system |
| Debug Area dial-out | DONE | Gate 29 on world 1300 is an outbound-only dial hub: a GM at its DHD can dial every gate on a world this server loads, and nobody can dial it. See [Debug Area dial-out](#debug-area-dial-out) |
| Gate address discovery | DONE | Two grant paths: a committed gate arrival, and the content action `grant_stargate_address` (Harset H55), which is the port of 2009's `Act_StargateAddress` node. The content grant is visible without a relog — it sends client method 66 `updateStargateAddress`. `giveStargateAddressStr` / `removeStargateAddressStr` are defined and unimplemented; there is still no revoke path |

## DHD interaction

Right-clicking a prop whose `interaction_type_flags` carry `INT_DHD` (bit 16) opens the dialling UI. [`cell/interactions/dhd.rs`](../../crates/cell-interactions/src/cell/interactions/dhd.rs) claims the interaction, looks up the stargate belonging to the **player's current world**, and emits `onDisplayDHD` (flat index 120) with a single `UINT8` — the gate's point-of-origin glyph.

Three things are easy to get wrong here:

- **`address_origin` is a glyph (1-38), not an identifier.** It repeats across rows (value 1 on both `SGC W2` and `SGC`, 13 on both Dakara E2 and E3), so it must never be used as a key into the `stargates` map, which is keyed by `stargate_id`. The emit validates against the **authored glyph range**, not just the wire's `UINT8` domain: `0`, `39`-`255` and anything that fails `u8::try_from` all refuse to emit. The column is `INT32` and the wire slot is `UINT8`, so neither type is the domain — a value outside 1-38 serialises perfectly cleanly and reaches the client as a DHD with no symbol to render, which reads as a client bug rather than the seed error it is. Refusing to emit is what makes it findable (`reason = "address_origin_out_of_range"`).
- **The known-address list is not sent here.** It rides `setupStargateInfo` at world entry; the client filters against what it already has.
- **Two gates on one world resolve deterministically** by lowest `stargate_id`, because `stargates` is a `HashMap` and an unordered pick would hand the client a different glyph across restarts.

Template 1 (`GLB-DHD_00`, every DHD prop but the Castle's) shipped with `interaction_type = 0`, so until DA-07 those props were not right-clickable and only the Castle's template 162 opened the dialling UI. Template 1 now carries `INT_DHD` too, which makes the DHDs at Harset, Tollana, Lucia, Omega Site, Beta Site E1, Dakara E1, both Ihpet Craters, Men'fa (Praxis) and the Debug Area open. A live-DB guard checks that every spawned DHD prop can be clicked and stands on a world with a gate row.

A DHD prop on a world with no `stargates` row logs `reason = "no_stargate_for_world"` and shows the player nothing. The 2009 server sent a free-text `onError` here; Cimmeria's `onErrorCode` is an enum-coded surface with no free-text arm, so there is nowhere for that string to go. Acceptable while every seeded DHD has a gate.

## Dial authorization

`onDialGate` carries `targetAddressId` as a raw client `INT32`, so the address book is the only thing standing between a crafted packet and a cross-world teleport into unearned content. [`cell/gate_travel/address_book.rs`](../../crates/cell-interactions/src/cell/gate_travel/address_book.rs) refuses any address not in `CellEntity::known_stargates`, which the base loads from `sgw_player.known_stargates` and hands to the cell on `InitPlayerState`.

The check is the first thing `handle_dial_gate` does after the `-1` cancel sentinel, which matters three times over:

- It runs **before** the 4-second dial is armed, so a refusal cannot leave a gate that opens on a timer.
- It runs **before** the `stargates` cache lookup, so "that address does not exist" and "that address is not yours" are the same observable. A client cannot probe the id space.
- Like 2009's three reject branches, it cancels any dial already in flight (`SGWPlayer.py:2061`). Without that, dialling a gate you hold and then one you do not would leave the first destination armed and crossable.

The refusal reaches the player as `onErrorCode` (121): `SystemID = 0` (`ERRORCODE_SYSTEM_Ability`, the only token the enum defines), `InstanceID = 0`, `ErrorCodeID = 180` (`CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`). `InstanceID` is deliberately zero rather than the stargate id — under system 0 the client reads that field as an ability id.

**A destination world with no space is refused before anything is torn down.** After the book check and the same-world check, `handle_dial_gate` refuses a gate whose world `SpaceManager::world_is_enterable` rejects (not in `cell_spaces.xml`, not instanced) with `reason = "destination_world_not_loaded"` and the unrecoverable-arrival line. Before this, a held address on one of the fourteen 2009 worlds with no map here (by `gmDHD`, a stale row or content) armed or travelled, `perform_gate_travel` destroyed the cell entity, and the base then failed to create a space: a player in no space. Only an address the player already holds reaches this check, so "unreachable" tells a client nothing new.

Since #727 every dial refusal also sends a chat line on `CHAN_feedback`: `Failed to dial: not a known stargate address` for an address the player does not hold *and* for one that does not exist (the two are byte-identical, so the answer is not an existence oracle), plus lines for dialling your own world, dialling before the cell entity exists, and a destination with no standable arrival. The texts are in [stargate-dhd-state-machine.md](../reverse-engineering/findings/stargate-dhd-state-machine.md#dial-refusal-feedback-727-2026-09-28). `onDHDReply` (SGWPlayer client method 100) is *not* used for this: #1024 traced its client-side subscriber to the same `Communicator` chat component that renders this line, not to the DHD window, so switching to it would not gain a DHD-window popup — see the [render-target resolution](../reverse-engineering/findings/stargate-dhd-state-machine.md#ondhdreply-render-target-resolution-1024-2026-09-28).

**Transit is not gated.** The check is on the dial and only the dial, matching 2009, which gates `onDialGate` and never `GateTravel.stargatePassed`. A player may walk through a wormhole somebody else opened.

**Outbound-only gates are refused here too.** A `stargates` row with `debug_dial_hub = true` (only the Debug Area's, gate 29) is never a destination. `player_knows_stargate` refuses it *before* reading the book, so the id cannot be dialled even if it got into a book somehow, and the refusal is byte-identical to an unknown address's. That keeps the outbound-only rule inside the one authorization function. See [Debug Area dial-out](#debug-area-dial-out).

**`gmDHD` is not exempted in the primitive.** An `access_level` branch would put a second authorization surface on a check whose whole value is having exactly one. Instead the GM arm — already authorized against the session's access level — tops the caller's *in-memory* address book up with a `reason = "gm_address_grant"` audit warn before dialling. Nothing is persisted; this mirrors 2009's `giveaddress` console command.

## Address unlock on arrival

Neither half of the unlock ever learns an outbound-only gate (`debug_dial_hub`): the statement filters them out of the origin and the destination union alike. Leaving the Debug Area puts its gate in the origin half, and `.gotolocation DebugArea` puts it in the destination half.

A committed gate arrival appends the addresses the trip taught the traveller, in the same `sgw_player` UPDATE that persists the destination world and position ([`base/world_entry/gate_travel/persist_arrival.rs`](../../crates/base-world-entry/src/base/world_entry/gate_travel/persist_arrival.rs)).

**This is new behaviour, not a restoration.** There is no unlock-on-visit anywhere in the 2009 Python: `SGWPlayer.addStargateAddress` has exactly two callers, the GM console command `giveaddress` and the Atrea authoring node `Act_StargateAddress`. Addresses were authored content. Cimmeria has no content-engine equivalent, so with the dial gate enforced this is the only grant path in the game.

**Both ends of the trip are learned, and the origin is the half that does any work.** On the dial route the destination is a no-op by construction — the dial gate refuses an address you do not already hold, so a dialled destination is always already known. What a traveller does not have is a way back, and `handle_gate_travel` is also the transport for GM `.gotolocation` cross-world, the respawn fork, content `cross_world_teleport` and cross-world rings, none of which consult the address book. The origin world's gates are resolved inside the UPDATE from the row's own pre-update `world_location`, which is the only source that stays correct across consecutive hops.

Two ordering constraints hold this together. The write runs after every mid-transfer abort branch, so nothing is persisted for a transfer that did not happen, and before `query_player_load_data`, which fills the `setupStargateInfo` list the client is about to receive. Get the second wrong and the client renders an address book one hop out of date while the cell enforces the current one.

**A newly created character still starts with an empty book.** `base::character_create` does not name `known_stargates`, so the column defaults to `'{}'`. That predates this work — the dial UI only ever offered known destinations — and Harset H55 decided to leave it that way: in 2009 the first address was always authored content, and content now has a verb that can author it. See *Address grants from content* below.

## Address grants from content

`grant_stargate_address` is the content-engine port of the 2009 Atrea authoring node `Act_StargateAddress` (`entities-editor/editor/Nodes.xml:2428`), which called `SGWPlayer.addStargateAddress`. That node and the GM `giveaddress` console command were its only two callers, so in 2009 stargate addresses were authored content and nothing else. Cimmeria had no equivalent until Harset H55, which is why a character who had never travelled could dial nowhere.

The seed verb takes `target_id` = `resources.stargates.stargate_id`. It is the address itself — not a world id, and not the repeating `address_origin` glyph.

A grant has to reach three places, and all three are emitted from [`cell/content/executor/stargate.rs`](../../crates/cell-content/src/cell/content/executor/stargate.rs):

1. `CellEntity::known_stargates`, which is what the dial gate above enforces against.
2. The client, via `updateStargateAddress` (client method 66: `INT32 addressId`, `UINT8 hasAddress = 1`, `UINT8 hidden = 0`). The client is handed its whole address book exactly once, by `setupStargateInfo` at map load, so without this the grant is invisible until a relog.
3. `sgw_player.known_stargates`, through `CellToBaseMsg::GrantStargateAddress` and an idempotent append in [`base/world_entry/gate_travel/address_grant.rs`](../../crates/base-world-entry/src/base/world_entry/gate_travel/address_grant.rs) — deliberately the same statement shape as the arrival append beside it, minus the origin-world union.

Legs 1 and 2 go out **before** leg 3 is confirmed. The cell's copy is the thing the dial gate reads, so making the client's copy wait on a database round trip would reopen the divergence the arrival path closes: the server accepting a dial the client's UI does not offer. A lost leg-3 write costs the address at next login and warns; a lost leg-2 send makes a granted address undialable in silence.

The grant is idempotent at both ends. A player who already holds the address gets no write, no client method and no base round trip, and the SQL append is a set difference rather than an `array_append` — `known_stargates` is a bare `integer[]` with no uniqueness constraint, so a duplicate would be silent, permanent, and visible in the player's DHD.

Refusals are never silent: an id with no `stargates` row, a non-player actor, and either failed send each warn with a `reason` field, and the grant is noted in the player journal so a `.bug` bookmark shows when the address was learned.

**Castle mission 708 is the first consumer.** Chain 1357 (the Livewire victory that repairs the DHD) grants `stargate_id = 3`, Harset. Step 4462 — "Use the DHD to dial the Stargate to Harset" — was unreachable before that row existed.

**There is no revoke verb.** 2009's node had a `Remove` port and no shipped content used it; `revoke_stargate_address` can be added when a chain needs one.

## Debug Area dial-out

The Debug Area (world 1300, [campaign plan](../analysis/debug-area/README.md), packet DA-07) keeps the Ihpet_Crater_Light map's own stargate. Standing at it, a GM can dial **every gate on a world this server can load**, and nobody can dial *into* the Debug Area. Getting back is `.gotolocation DebugArea` only.

### The gate

| Piece | Seed | Notes |
|---|---|---|
| Gate row | `stargates` 29 `Debug Area`, world 1300 | Same prefab sequence, transform, point-of-origin glyph (2) and event set (10005) as gate 20 `Ihpet Crater (SGU)`, because it is the same prop on the same client map. An event set binds a gate's Kismet sequences by event id and the client plays them on whatever map is loaded, so two rows can share 10005. `debug_dial_hub = true` |
| Address | 38-37-36-35-34-33 | Unique among every seeded and shipped gate, so a glyph sequence typed into the DHD never resolves to it. No player can hold it |
| Arrival pin | (251.0, 8.0, -962.0), yaw 0 | Z1, the same point as respawner 130, 28 m in front of the gate and facing away from it. Nobody arrives *through* gate 29, so the pin only places `.gotolocation DebugArea` with no coordinates (the stargate arrival is a world's entry point). It keeps that landing outside the gate's own volume |
| Gate volume | `point_sets` 13800 `DebugArea.Stargate` | A copy of set 1011 for world 1300. With it, a dial opens the gate and waits for the GM to walk through. Without it, the dial would travel at once with no gate animation |
| DHD | `spawnlist` 13800 `DebugArea_DHD`, template 1 | Where the world-73 DHD stands on the same map |
| Cooked entry | category 13 addition `_29` (`crates/resources/src/base/stargate_overrides.rs`) | `setupStargateInfo` and `updateStargateAddress` carry bare ids that the client resolves in `CookedDataStargates`. The shipped PAK holds ids 1-28, so the server adds 29 in memory and bumps the category version (the #840 handshake, like the Debug Area's world info entry). A live-DB test holds the seed row and the cooked entry together |

### Authorization: one surface, one grant

The dial check stays `address_book::player_knows_stargate`, the only authorization surface ([Dial authorization](#dial-authorization)). The hub only adds a **grant**, the way `gmDHD` does:

1. A player right-clicks the Debug Area DHD. `try_open_dhd` resolves the world's gate (29). Because it is a hub, `gate_travel::dial_hub::top_up_gm_dial_hub` runs **before** `onDisplayDHD` is sent.
2. If the caller's server-side `access_level` is GM or higher (`is_gm`, 2+), every gate that is not a hub, is not on the hub's world, is not on `HUB_EXCLUDED_GATES` (gate 22, below), and whose world `SpaceManager::world_is_enterable` (a `cell_spaces.xml` startup space, or a world `spaces.xml` marks instanced) is added to the GM's **in-memory** address book. Each new one is sent to the client as `updateStargateAddress(id, 1, 0)` (client method 66) on the same ordered channel, so the DHD opens with the full list. One `warn` with `reason = "gm_dial_hub_grant"` records the GM, the gates granted (`id:name@world`) and the gates left out. A non-GM gets nothing (`reason = "dial_hub_not_gm"`, `info`), and their DHD offers only their own book.
3. The dial is then judged like any other: the address is in the book, so `handle_dial_gate` arms it, and the dial logs name the destination gate (`target_address_name`, from the name book).

The grant happens on DHD open, not at world entry. The base sends the client its whole book in `setupStargateInfo` at map load, so a push from the cell during world entry could race it and be overwritten. Opening the DHD also re-checks the access level when it matters.

Nothing is persisted. The top-up dies with the cell entity on the next transfer. A gate the GM actually travels to is then learned by the arrival unlock, the same as after a `gmDHD` dial.

### Outbound only

No path can put gate 29 in a book or make it a destination:

| Path | Guard |
|---|---|
| A dial naming 29 (DHD or crafted `onDialGate`) | `player_knows_stargate` refuses a hub before reading the book, with the unknown-address bytes |
| `gmDHD 29` | Not granted (`reason = "gm_address_grant_dial_hub_skipped"`), then refused by the dial check |
| The hub top-up | Skips every hub |
| Content `grant_stargate_address` | Refused (`reason = "grant_dial_hub_outbound_only"`): no book entry, no client method, no base write |
| Arrival unlock, both halves | `persist_arrival` filters `debug_dial_hub` |
| Base address append | `append_known_stargate` filters `debug_dial_hub` |

"Never enters an address book" is exact: gate 29's cooked catalogue entry (name, world 1300, glyphs) is resynced to every client, because the client resolves stargate ids in that table. Knowing the catalogue does not help: the dial carries the id, and the address-book check refuses id 29 before it reads the book.

### What a GM can dial from the Debug Area

All 28 other gates are offered except the 14 on worlds this server cannot load. Those have a `resources.worlds` row but no space: CombatSim (1), Ihpet (9), Hebridan (11), Dakara E2 (12), Dakara E3 (13), Pen-Lai (14), Beta Site E2 (16), SGC W2 (17), Yotunheim (18), Vitrus (19), Meridian (21), Egypt (24), Pertho (26) and Asgard High Council (28). Dialling one would tear the GM out of the Debug Area and then fail to create a space for them, so they are left out and named in the grant log.

One more gate is left out although its world loads: **22 `Men'fa (SGU)`** (`HUB_EXCLUDED_GATES` in `dial_hub.rs`, named with its reason in the grant log). Its row, and the client's own cooked entry, put the gate at y -191.9, but `menfa_light.nav` has no polygon within 3 m of that point and its playable surface at the same XZ is near y 0, about 192 m higher. Men'fa (Praxis), gate 7, has the identical row and stands on `menfa_dark.nav`, so the Light map differs and only an in-client look can place its pad. World 78 has no respawner to fall back on. The same row is what any player who dials Men'fa (SGU) from elsewhere arrives on; an `arrival_*` pin from DA-06 fixes both, and then the entry comes off the list (a live-DB test fails when the arrival becomes usable, to say so).

The 13 offered gates are The Castle, Harset, Tollana, Omega Site, Beta Site E1, Men'fa (Praxis), Ihpet Crater (Praxis), Lucia, Agnos, Ihpet Crater (SGU), SGC, Dakara E1 and SGC W1. No dial from the Debug Area is refused for `arrival_unrecoverable`: every one of those worlds is navmesh `advisory`, so the arrival rules accept the authored point. Applied as if each world enforced its mesh, one would fail, and a live-DB test pins it:

| Gate | World | Measured (NA28 mesh) |
|---|---|---|
| 27 `SGC W1` | SGC_W1 | Gate row off-mesh, nearest walkable point 3.4 m away and 1.3 m lower. No respawner |

A GM dialling it lands on the gate row and the client settles them onto the floor beside it. The other twelve arrivals are on their world's mesh.

## Entity Definition (GateTravel.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `knownStargateAddresses` | ARRAY\<PYTHON\> | CELL_PRIVATE | Player's discovered gate addresses |
| `oldWorldID` | INT32 | CELL_PRIVATE | Previous world before travel |
| `gateCounter` | INT32 | CELL_PRIVATE | Gate usage counter |
| `destinationGate` | INT32 | CELL_PRIVATE | Target gate address ID |
| `destinationGateArrivalTime` | FLOAT | CELL_PRIVATE | Expected arrival timestamp |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `setupStargateInfo` | worldStargateList, knownStargateList, hiddenStargateList | Initialize DHD UI |
| `updateStargateAddress` | addressId, hasAddress, hidden | Update single address |
| `stargateRotationOverride` | yaw | Override gate rotation |
| `onStargatePassage` | addressId | Notify successful gate travel |

### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `onDialGate` | YES | TargetAddressId, SourceAddressId | Player dials a gate |
| `giveStargateAddressStr` | NO | AddressId, Hidden | Grant gate address |
| `removeStargateAddressStr` | NO | AddressId | Remove gate address |
| `closeGatesTo` | NO | AddressId | Close gates to address |
| `processGateTravel` | NO | userData | Execute gate travel |

### Base Methods

| Method | Args | Purpose |
|--------|------|---------|
| `processSquadLeaderGateTravel` | memberId, userData | Squad leader triggers group travel |
| `processGateTravel` | userData | Execute gate travel on base |

## Ring Transporter FSM

The ring transporter uses an 8-state finite state machine:

```
STATE_IDLE
  |-> selectDestination() --> STATE_SEND_WAIT
       |-> regionTriggered() / players present --> STATE_SEND_WARMUP
            |-> __beginTransport(): lock movement, play TeleportOut sequence
            |-> remoteRegion.remoteSend()
            |-> 3.5s timer: hide players (setVisible=false)
            |-> 4.0s timer --> STATE_REMOTE_LOAD_WAIT
                 |-> __doTransport(): teleportTo(destination)
                 |-> remoteTransport()

Remote side:
STATE_IDLE
  |-> remoteWait() --> STATE_RECV_WAIT
       |-> remoteSend() --> STATE_RECV_WARMUP
            |-> __beginTransport()
            |-> remoteTransport() --> STATE_REMOTE_LOAD_WAIT
                 |-> __doTransport()
                 |-> remoteCountUpdate()
                 |-> playerLoaded() x N --> STATE_REMOTE_WARMUP
                      |-> Play TeleportIn sequence
                      |-> 3.0s timer --> STATE_COOLDOWN
                           |-> setVisible(true)
                           |-> 2.5s timer --> STATE_IDLE
                                |-> unsetStateFlag(BSF_MovementLock)
                                |-> onTeleportIn()
```

## Ring Transport Timings

| Phase | Duration | Action |
|-------|----------|--------|
| Warmup (send) | 3.5s | Players hidden |
| Transport (send) | 4.0s | Teleport executed |
| Warmup (receive) | 3.0s | TeleportIn sequence |
| Cooldown | 2.5s | Players visible, movement unlocked |

## Data References

- **Stargate addresses**: 29 in `db/resources/Worlds/Seed/stargates.sql` (the 28 from 2009 plus the Debug Area's 29); the nullable `arrival_x/y/z/yaw` columns and the `stargates_arrival_all_or_nothing` CHECK are declared in `db/resources/Worlds/Tables/stargates.sql`
- **Respawners**: `db/resources/Worlds/Seed/respawners.sql` — the arrival fallback pool. A world with no row has no recovery from an off-mesh gate
- **Ring transporter regions**: `RingTransporterRegion` definitions
- **Kismet events**: `Region_Teleport_Out`, `Region_Teleport_In`

## RE Priorities

1. **Stargate travel** - Implement `processGateTravel` for zone transitions
2. **Gate animation** - Stargate dialing/kawoosh sequence from client
3. **Squad gate travel** - `processSquadLeaderGateTravel` group teleport protocol
4. **Hidden addresses** - How hidden gate addresses work in the DHD UI
5. **Cross-world rings** - Verify ring transport across world boundaries

## Related Docs

- [combat-system.md](combat-system.md) - Movement lock during transport
- [group-system.md](group-system.md) - Squad leader gate travel
- [ring-transport-system.md](ring-transport-system.md) - The ring FSM in full, including the bounded aborts and the shared arrival validation
- [../engine/space-management.md](../engine/space-management.md) - Space extents, navmesh loading, and which worlds are instanced
