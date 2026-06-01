# CAT-O — World / Space / Gate / Ring

The CAT-O surface covers cross-world travel (stargate dial, ring transport
FSM, gate-region triggers), space-instance lifecycle controls
(`onWorldInstanceReset`, `onSpaceQueue*` responses, `onStrikeTeamResponse`),
cinematic suppression (`cancelMovie`), and the client-options sync
(`updateSystemOptions`). The bulk of the *position-spoofing* attack
surface — DHD-without-walk, ring-without-pad, region-without-position —
is already filed under CAT-B (see [[CAT-B-02]], [[CAT-B-03]], [[CAT-B-04]])
because the proximate trust violation is "client position discarded".
CAT-O's contribution to those messages is the *adjacent* trust violations
that survive even after the position check is added: destination
authorization (player has the unlocked address?), chain-trigger spam
(rapid-fire `triggerClientHintedGenericRegion` over a sequence of regions),
the dial-cancel race, and the systemic exposure of
`onWorldInstanceReset` on the **player** entity rather than the GM entity.
The system-options handler is well-scoped (only two booleans accepted) but
sits on a `WSTRING` wire that will silently broaden the moment anyone adds
a PvP-flag or similar; that future trap is captured under "Not Filed". The
space-queue / strike-team / movie-cancel paths are currently stubs — the
exploit surface lands at implementation time, and those gaps are listed in
"Not Filed" with the specific invariants any implementer must satisfy.

---

### CAT-O-01 — `onDialGate` ignores `knownStargateAddresses` — any character dials any gate the world catalog knows

**Severity**: High
**Class**: Missing destination authorization / content-gate bypass
**Wire surface**: Cell method 35 (`onDialGate`) — `Event_NetOut_onDialGate` / `Event_NetOut_DHD`
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The handler at `crates/services/src/cell/gate_travel.rs:35-108` validates
`target_address_id` only against `space_mgr.stargates` — the *global*
catalog of every stargate that exists in the world, loaded from
`resources.stargates` at startup. It does **not** check the player's
`knownStargateAddresses` (the CELL_PRIVATE address-book property defined
in `entities/defs/interfaces/GateTravel.def:4-8` and persisted in
`sgw_player.known_stargates`). Per-player gate unlocks are tracked in
`PlayerLoadData::known_stargates` and shipped to the client via the
`setupStargateInfo(worldStargateIds, knownStargateIds, hiddenStargateIds)`
method at world entry (`crates/services/src/mercury/world_data/map_loaded.rs:175-181`)
— the server knows exactly which gates the player has unlocked but never
consults that list when authorising a dial. Any character can therefore
dial any stargate that exists *anywhere* in the resource catalog, including
end-game-world gates the player has never seen in-game. This is distinct
from [[CAT-B-02]]: that finding fixes the *source* position; this finding
fixes *destination authorization*. Both have to be fixed independently —
adding the source-position check alone still lets a GM-spawned-near-a-
gate character dial to any unlocked-by-anyone gate.

**Evidence**
- Ghidra: `00d93060` `register_NetOut_onDialGate` + `019be588`/`019ca724`
  string anchors for the `Event_NetOut_onDialGate` class. Wire shape pinned
  by `entities/defs/interfaces/GateTravel.def:70-74` —
  `(INT32 TargetAddressId, INT32 SourceAddressId)`. The DHD slash-command
  emit also reaches the same cell method 35 (no separate `dhd` cell
  method exists in the dispatch surface).
- Client behavioral log: n/a (UI-driven dialer + slash-command path).
- Cross-ref to Rust handler:
  `crates/services/src/cell/gate_travel.rs:49-59` — the only check is
  `space_mgr.stargates.get(&target_address_id)`. The DB column
  `sgw_player.known_stargates` is read into `PlayerLoadData::known_stargates`
  at world entry (`crates/services/src/base/world_entry/methods/player_load/core.rs:60,217`)
  but the cell entity never carries it, so the gate handler can't consult
  it without an additional plumb.

**Attack scenario**
1. Player creates a fresh character; the server initialises
   `known_stargates` to `[]` (or to the starter-world gate).
2. Through any means — Ghidra string-search, capture from another
   character, or simple bruteforce of small i32s — discover the
   `target_address_id` for an end-game world (e.g., the Asuras hub).
3. Send `onDialGate(target=<asuras_id>, source=<whatever>)`.
4. Server finds the gate in `space_mgr.stargates`, calls `handle_gate_travel`,
   destroys the entity, persists the new world, and ships the
   `RESET_ENTITIES + new-world entry` bundle. Attacker is now on a world
   they never unlocked, bypassing every mission-driven gate-unlock chain.

**Suggested remediation (one line)**
Plumb `known_stargates` onto the cell entity at world entry and reject
`onDialGate` when `target_address_id` is not in the player's
`known_stargates` (and not flagged as a free/public address). Stack on
top of the source-position fix from [[CAT-B-02]].

**Would benefit from x64dbg trace?**
No — the gate handler's call to `space_mgr.stargates` is the proof; the
absence of any read of `known_stargates` in the gate path is a code-level
gap.

---

### CAT-O-02 — `onDialGate` skips the 4-second dial timer — no cancellation window, no cost

**Severity**: Medium
**Class**: Missing transaction window / no-cost teleport
**Wire surface**: Cell method 35 (`onDialGate`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The Python reference (`python/cell/SGWPlayer.py:onDialGate`) ran a
4-second dial timer before invoking the world transition — that window
was where the dial could be cancelled (`target_address_id == -1`),
where energy/consumable cost would be checked, and where mission
abort/interrupt could fire. The Rust handler at
`crates/services/src/cell/gate_travel.rs:28` explicitly documents
"we skip the timer and travel immediately for simplicity". Aside from
making the cancel-dial path (`target_address_id == -1` at line 43)
effectively dead code, this collapses the gate-travel transaction into a
single atomic mutation with no opportunity to apply any cost or
condition (the existing 2009 game charged the player nothing per dial,
but the surface is now permanently denied to *any* future cost
mechanic). Combined with [[CAT-O-01]], a player can chain-dial through
every world in the catalog in seconds with no rate-limit, no UI
animation gating, and no client-state check.

**Evidence**
- Ghidra: `onDialGate` emit path (see CAT-O-01) — the client's 4-second
  spin-up of the dial UI is purely visual; the Mercury packet ships
  immediately on the user's confirm click.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/cell/gate_travel.rs:25-29` documents the
  intentional skip, `:43-46` shows `target_address_id == -1` returning
  a no-op (would only matter if there was actually a pending dial to
  cancel).

**Attack scenario**
1. Send `onDialGate(target=W1, source=0)` — server immediately tears
   the entity down and starts the world-transition bundle.
2. Halfway through the bundle's RESET_ENTITIES round-trip, send
   `onDialGate(target=W2, source=0)`. The first transition's mid-flight
   state on `pending_world_entry` interleaves unpredictably with the
   second's RESET. Even when the two complete cleanly, there's no
   rate-limit between them — a single client can chain world transitions
   at packet-loop speed, exercising every world-load codepath in
   sequence.
3. (Future-cost angle.) If the project later adds a dial-cost mechanic
   (energy/naquadah/consumable), the only place to charge it is inside
   `handle_dial_gate` — but the transaction is atomic, so a failed
   charge mid-transition leaves no rollback target.

**Suggested remediation (one line)**
Reinstate the 4-second dial timer (or a shorter equivalent), store the
pending dial on `CellEntity` so a second dial cancels-and-replaces, and
move the destination world-write to the timer-elapsed callback so any
future cost/condition gate has a place to run before commit. Route the
redesign to `movement-teleport-advisor`.

**Would benefit from x64dbg trace?**
Yes — observing the 2009 server's actual dial timer (Python comment
"4 seconds" is the only reference) would tighten the duration; not
required for the structural fix.

---

### CAT-O-03 — `triggerClientHintedGenericRegion` allows unbounded chain-event injection across regions

**Severity**: High
**Class**: Content-chain-engine injection / mission-skip via region spam
**Wire surface**: Cell method 85 (`triggerClientHintedGenericRegion`)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
Building on [[CAT-B-04]] (which is anchored on the discarded `_x/_y/_z`):
the handler at
`crates/services/src/cell/cell_methods/player/world/mod.rs:128-191`
imposes **no rate limit, no per-region debounce, and no chain-event
ordering check** beyond "the region id resolves to a known
`region.tag`". A malicious client can rapid-fire (region=R1,
entering=true) → (region=R1, entering=false) → (region=R2, entering=true)
→ ... walking the chain engine through every `enter_region::<tag>` /
`exit_region::<tag>` arm in arbitrary order at packet rate. Content
chains are how mission-step gating is wired
(`fire_enter_region`/`fire_exit_region` at lines 163-171 dispatch into
`crate::cell::content`), so this is a one-handler primitive for
advancing *any* region-triggered mission step the chain engine knows
about — without ever having been near the region geometrically (the
CAT-B-04 piece) and without having walked the intervening regions in
order (the CAT-O angle here). The ring-transport FSM forwarding at
lines 176-181 inherits the same property: a client can drive
`region_triggered` for every loaded ring pad in sequence to scout
destination IDs (`onRingTransporterList` is sent on each interact) or
to leave the ring FSM in a corrupted state across multiple pads.

**Evidence**
- Ghidra: `Event_NetOut_TriggerClientHintedGenericRegion` (standard
  `Event_NetOut_*` shape per
  `entities/defs/SGWPlayer.def:766-771` — `INT32 id, UINT8 bEntering,
  VECTOR3 position`).
- Client behavioral log: n/a (continuous as the player walks; under
  attack, packet-rate).
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/world/mod.rs:128-191`.
  No per-entity `last_region_trigger_at` field exists. No content-chain
  step-order check exists in `crate::cell::content`.

**Attack scenario**
1. Enumerate mission-relevant region IDs (via observing one legitimate
   playthrough, or via the `region_tag` resolution log on a captured
   server, or via brute-force of small i32s).
2. Build a packet sequence:
   `[trigger(R1,true), trigger(R1,false), trigger(R2,true),
   trigger(R2,false), ..., trigger(Rn,true)]` — one packet per step
   needed by a multi-region mission chain.
3. Server fires `fire_enter_region` / `fire_exit_region` for each tag
   in order. Content chains advance the mission as if the player had
   physically walked the entire mission path in proper order.
4. Mission completes; rewards granted; mission-gated zones unlocked.
   Repeat against any region-driven chain in the content engine.

**Suggested remediation (one line)**
Add a per-entity per-region cooldown (e.g., 250ms minimum between
`enter`/`exit` on the same region; reject `enter`/`enter` or
`exit`/`exit` without the toggle), plus a `last_region_change_at`
field on `CellEntity` to rate-limit cross-region transitions to a sane
walking-pace cap (e.g., max 8 region transitions per second). Layer on
top of the position validation from [[CAT-B-04]].

**Would benefit from x64dbg trace?**
No — the absence of any cooldown / debounce / chain-order check is a
code-level gap; the chain engine's lack of step-order enforcement is
visible in `crate::cell::content`.

---

### CAT-O-04 — `onWorldInstanceReset` is exposed on the **player** entity (not GM), currently UNIMPLEMENTED

**Severity**: High (future trap — currently latent)
**Class**: GM-command exposure / instance DoS surface
**Wire surface**: Cell method 92 (`onWorldInstanceReset`)
**Demonstrable / Likely-theoretical**: Likely-theoretical (latent)

**Trust violation**
The entity-def at `entities/defs/SGWPlayer.def:868-870` declares
`<onWorldInstanceReset><Exposed/></onWorldInstanceReset>` *on the player
entity* — not the GM player entity (`SGWGmPlayer.def` has none).
`<Exposed/>` means the client can emit this directly. The Rust handler
at
`crates/services/src/cell/cell_methods/player/world/mod.rs:230-233`
is a stub: `tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset"); true`.
Today this is harmless (it does nothing), but the failure mode is the
*future* trap: a future contributor implementing it naturally puts the
implementation *here*, in the player-side handler, and unless they
*also* add an access-level check, the resulting instance-reset is
callable by any player (instance DoS — kick every player on the
instance, force re-entry, lose in-flight state). The
[[reference_gm_auth_plumbing_gap]] memory documents the systemic gap:
the cell-method dispatch path
(`crates/services/src/base/connect_loop/cell_arms.rs`) has no
access-level field on the call — any GM-shaped command lacking an
explicit `is_gm_session(player_id)` check defaults to "open to all
players". The current `Event_NetOut_WorldInstanceReset` C++ client
emits this from a slash-command (`Event_SlashCmd_WorldInstanceReset`
at `00cbe5a0`), but slash-commands aren't client-side-gated either —
the gate has to land on the server.

**Evidence**
- Ghidra: `00cbe5a0` `register_NetOut_WorldInstanceReset`; the C++
  client wires `Event_SlashCmd_WorldInstanceReset` (`018429f4`) to
  `Event_NetOut_WorldInstanceReset` (`019b4340`) — a slash-command
  fires the network emit with no client-side access-level gate.
- Client behavioral log: n/a.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/world/mod.rs:230-233`
  (UNIMPLEMENTED stub),
  `crates/services/src/cell/cell_methods/player/constants.rs:30`
  (the index is in the player-method range, not a GM-method range).

**Attack scenario** (post-implementation)
1. Contributor implements `WORLD_INSTANCE_RESET` to do the obvious
   thing: walk `space_mgr.world_spaces[current_world].entities`,
   destroy each, send RESET_ENTITIES bundles, re-spawn at the world's
   start position.
2. They forget (or are unaware of) the access-level check — the dispatch
   layer doesn't surface it as a required parameter.
3. Any player sends `onWorldInstanceReset` (empty payload — no args
   per the def). Every player on the instance is force-kicked back to
   start, losing in-flight loot/state/combat. Repeat at packet rate
   for sustained denial-of-service against an instance.

**Suggested remediation (one line)**
Mark the stub `BLOCK` until two prerequisites land: (a) plumb
`access_level` from `ConnectedClientState` into `dispatch_cell_method`
so handlers can server-side-validate the session's GM bit, and
(b) the `onWorldInstanceReset` arm's first line must be
`if !is_gm_session(player_id) { return true; }`. Route to the GM-auth
plumbing track (see [[reference_gm_auth_plumbing_gap]]).

**Would benefit from x64dbg trace?**
No — the gap is in the def (exposed-to-player) and the dispatch surface
(no access-level plumb).

---

### CAT-O-05 — `cancelMovie` flips `cinematic_spam_cancel` from any sender; no movie-id binding

**Severity**: Low
**Class**: Cross-session state poisoning (cosmetic)
**Wire surface**: Cell method 108 (`cancelMovie`) — `Event_NetOut_CancelMovie`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The wire payload (per `entities/defs/SGWPlayer.def:1104-1107`) is
`WSTRING MovieName`. The base handler at
`crates/services/src/base/world_entry_appearance.rs:721-741` ignores
the supplied `MovieName` entirely — it just flips
`cinematic_spam_cancel` to `true` on the connected state and resends
BeingAppearance + onEntityTint. Today the only cinematic in flight is
the first-login intro, so there's no scenario where canceling movie
"A" is supposed to leave movie "B" running. But the design plants a
foot-gun: any future code that calls `send_cinematic` for a
mission-critical cutscene (per the in-code comment at
`world_entry_appearance.rs:533-536` —
"Future cinematics (mission cutscenes, gate transitions, dialog
overlays, etc.) should go through this function") will be cancellable
by a client `cancelMovie` packet regardless of which movie was sent.
The trust violation is the unread `MovieName` argument: the server
treats *all* cancellations as global.

**Evidence**
- Ghidra: `Event_NetOut_CancelMovie` standard shape — wire is just the
  movie name wstring (matches def line 1106).
- Client behavioral log: `SGWDebugLog.log` shows `cancelMovie` being
  sent on Esc / Lua-stop today.
- Cross-ref to Rust handler:
  `crates/services/src/base/world_entry_appearance.rs:721-741` —
  no `MovieName` parse, no movie-id binding to the currently-active
  cinematic's id.

**Attack scenario** (future)
1. Mission script triggers `send_cinematic("mission_X_intro", true)`,
   armed with the spam-cancel flag.
2. Player sends `cancelMovie("some_other_movie_name")` (or even
   the unused intro name "DHD.DHD").
3. Server flips `cinematic_spam_cancel = true`, the spam loop exits
   early. The cutscene continues to play on-screen, but the
   server's post-cutscene appearance-resend guard is now off; if the
   client `Esc`s out of the mission cutscene, the appearance-asset
   collection race that the spam guard exists to mitigate (issue #288)
   re-emerges as a dev-cube bug — visible to the player but not
   server-state-damaging.
4. Generalises to any future cinematic-gated mission-completion path
   (none today): the server would clear the cinematic-in-flight flag
   regardless of which movie the client claims to have cancelled.

**Suggested remediation (one line)**
Bind the in-flight cinematic id to `ConnectedClientState` when
`send_cinematic` arms the spam guard, and compare the `MovieName`
argument against it; reject (or no-op silently with a `debug!`) when
the names don't match.

**Would benefit from x64dbg trace?**
No — the `MovieName` parse simply isn't there in the Rust handler.

---

### CAT-O-06 — `updateSystemOptions` is structurally safe today but rides a value-WSTRING wire that accepts arbitrary option names

**Severity**: Low (current) / Medium (future trap)
**Class**: Future-trap / scope-creep risk on a server-authoritative-by-default surface
**Wire surface**: Cell method 93 (`updateSystemOptions`)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The wire-format parse at
`crates/services/src/cell/cell_methods/player/world/mod.rs:262-399`
is correct as far as it goes: count-capped at 256 entries, applies
only via the closed allowlist
`SystemOptions::apply` at `crates/entity/src/cell_entity/system_options.rs:59-71`
(only `autoReload` and `reloadOnActivate` are recognised), unknowns
land in a `debug!` log. So today the player can only toggle the two
documented booleans, both of which are advisory-display / behavior
hints with no security impact (auto-reload at clip empty;
reload-on-activate on weapon swap).
The trap is the SystemOptions.xml surface: it defines ~140 options,
many of which look advisory but a non-trivial subset are
*server-synched* in the original game's semantic
(per the doc comment at lines 1-13 of `system_options.rs`). The wire
shape is `(WSTRING name, WSTRING value)`, so any name added to the
`apply` match arm becomes a client-writable server-authoritative
field. A future contributor adding (say) `pvpFlag` or
`acceptDuelAuto` or `accessLevel` to the apply list without an
access-level check turns this handler into a privilege-escalation
vector immediately. The struct currently has `Default` returning
`auto_reload: true` from a hardcoded constant, not from the DB
column's default — so a fresh player who never sends
`updateSystemOptions` will use the in-code default even if the DB
default drifts. That's a different scope-creep risk.

**Evidence**
- Ghidra: `Event_NetOut_UpdateSystemOptions` (or whatever the wire
  shape resolves to — the def at `SGWPlayer.def:872-875` pins the
  `ARRAY <of> NameValuePair`).
- Client behavioral log: n/a — UI checkbox driven.
- Cross-ref to Rust handler:
  `crates/services/src/cell/cell_methods/player/world/mod.rs:262-399`,
  the allowlist at `crates/entity/src/cell_entity/system_options.rs:59-71`,
  and the persistence path at
  `crates/services/src/base/world_entry/cell_dispatch/system_options.rs`.

**Attack scenario** (future)
1. A contributor extends the system-options panel to expose a server-
   authoritative toggle (PvP-flag, "I am AFK", "ignore duels", etc.).
2. They add a match arm to `SystemOptions::apply` and a column to
   `sgw_player`. No access-level check, no value-range validation —
   the wire is "string", so the apply path looks like a string parse.
3. A client sends `updateSystemOptions([("pvpFlag", "false")])` from
   inside a PvP zone where the player is engaged in combat. Server
   flips the bit; the player becomes untargetable mid-fight.

**Suggested remediation (one line)**
Document in the system-options doc-comment that **adding any
server-authoritative option name to `apply` requires a server-side
condition check inside that arm** (e.g., "PvP flag only changes when
out of combat for 60s"); add a compile-time enforcement by typing
the allowlist as an enum or by gating the apply path behind a
`SystemOptions::apply_with_context(name, value, &CellEntity)`
signature that forces the implementer to read state before writing.

**Would benefit from x64dbg trace?**
No — the gap is forward-looking design, not current behavior.

---

## Not Filed

- **`onSpaceQueuedResponse` / `onSpaceQueueReadyResponse` / `onSpaceQueueStatus`** —
  none of these are wired up on the server. Grep confirms there's no
  cell-method arm or dispatch entry for any of the three (the only
  matches are server→client emits `ON_SPACE_QUEUED` / `ON_SPACE_QUEUE_READY`
  at `crates/services/src/cell/client_methods/player.rs:99-102`). No
  exploit until they're implemented; the implementer must validate that
  the inbound response came from a session that is actually in the
  pending-queue map for the named space (and that the named space is
  instanced). Save this for the queue-system implementation review.
- **`onStrikeTeamResponse`** —
  stub at `crates/services/src/cell/cell_methods/organization.rs:65-77`
  logs `UNIMPLEMENTED` and returns. No exploit until implemented; the
  implementer must verify the receiving player was the one offered the
  strike-team invite (server-side invite-state lookup, not client-id
  trust). Filed as a partial overlap with CAT-M's organisation review.
- **`onSpaceQueueStatus`** — overlaps the above three; same disposition.
- **`DHD` (the `Event_NetOut_DHD` class)** —
  Ghidra confirms this is the *same* server-side wire as `onDialGate`
  (no separate `dhd` cell method exists in the SGW.def or in the Rust
  dispatch; `Event_NetOut_DHD` is the C++ client's slash-command-shape
  emit that maps to cell method 35 / `onDialGate`). All exploitation
  is already captured under [[CAT-O-01]], [[CAT-O-02]], and [[CAT-B-02]].
- **Ring-transport destination outside registered pad set** —
  CAT-O question 6 asked whether the destination can be set to a coord
  NOT on a registered pad. Confirmed mitigated: `validate_destination`
  at `crates/services/src/cell/ring_transport/transporter/mod.rs:245-256`
  rejects any `destination_id` not in the source's `destination_ids`
  list, and the runtime additionally requires
  `space_mgr.ring_transporters.get(destination_region_id)` to be Some
  (`runtime.rs:289-299`). The coords are then driven from the
  *server's* `RingRegion::x/y/z`, not from a client field. Source-pad
  trust is the gap, but that's [[CAT-B-03]].
- **Gate-travel destination coords can be spoofed via target_address_id** —
  examined and dropped: `gate.x/y/z/yaw` is read from
  `space_mgr.stargates` (server DB / resources), not from a client
  field. The client only supplies the i32 id; the destination position
  is fully server-sourced.
- **`SystemOptions` PvP flag concern** — see CAT-O-06 ("future trap"). Today
  the apply allowlist only contains the two advisory booleans; no
  exploit path exists in the current code, the finding is forward-
  looking.
- **`onSquadMemberRingTransport` (and `Finished` companion)** — these
  are server→client only (per the def — they're under `<ClientMethods>`
  in SGWPlayer.def at lines 855-863). Not in scope for CAT-O's
  client→server audit.
- **`Disconnect` race during cross-world gate travel** — the
  cross-world transition destroys the cell entity, sends `GateTravel`
  to base, and waits for the client's next ENABLE_ENTITIES bundle. A
  disconnect mid-flight is structurally possible but the base-side
  bundle is keyed on a per-session token, and `entity_to_addr` is
  cleared on disconnect. Examined `crates/services/src/base/world_entry/gate_travel/mod.rs`
  — no obvious dupe / stuck-entity exploit. Defer to CAT-A's
  disconnect-race coverage if one surfaces; not specifically a CAT-O
  problem.
- **`callForAid` / `Respawn` / `Petition`** — out of CAT-O scope (they
  live in combat / chat respectively).
