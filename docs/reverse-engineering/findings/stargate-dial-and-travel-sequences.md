# Stargate Dial Timing and Gate-Travel Cinematic Sequences

> **Date**: 2026-09-25
> **Type**: Reference
> **Audience**: Engineers working on `cell::gate_travel`, the cinematic system, or Castle/Harset gate content
> **Phase**: NPC-AI restoration packet NA35
> **Confidence**: HIGH for the wire-protocol structural claims (DHD dial is single-shot, chevron progress is never sent to the server, `onStargatePassage` is unsent); MEDIUM for the CrossGate per-instance Kismet matching mechanism; LOW/OPEN for exact Matinee/animation durations, which were not extracted this pass
> **Sources**: Ghidra decompilation of `SGW.exe` (see per-claim addresses below); `docs/reverse-engineering/findings/stargate-dhd-state-machine.md` (2026-05-13, session 5 — supplies several addresses corroborated independently here); `entities/defs/interfaces/GateTravel.def`; `crates/services/src/cell/gate_travel/`; `crates/services/src/cell/space_manager/gate_dial_state.rs`

## Evidence policy correction (read this first)

Every earlier gate-travel decision in this repo (`D-CA10`, `docs/gameplay/gate-travel.md`, `docs/gameplay/cinematic-system.md`) cites `deprecated/python/cell/SGWPlayer.py` — the never-finished legacy reference server — as the reason Cimmeria emits exactly two Stargate sequence events (`Stargate_MakeGate` 6100, `Stargate_CrossGate` 6113) and none of the other twelve. **The owner confirmed on 2026-09-25 that the legacy server never had working gate travel end to end.** "The 2009 server did/didn't emit X" is therefore not evidence about the 2009 *client's* expectations — only about what one abandoned server implementation happened to do. This finding treats the **client binary** (`SGW.exe`) as the sole source of truth, per the owner's explicit correction, and calls out below exactly where that changes the story the existing docs tell.

This finding does **not** retract D-CA10's underlying wire mechanics (the `onSequence` event ids, the `.def` methods) — those are still correct, client-confirmed facts. It retracts the *justification* built on the deprecated Python, and adds evidence the earlier pass did not have.

## 1. Dialing: what actually drives the chevron/kawoosh sequence

### 1.1 The DHD dial is client-collected and sent once, complete

`FUN_005682d0` (`ghidra://SGW.exe@0x005682d0`) is the CEGUI/Flash external-interface callback dispatcher for the DHD dialling window (and several unrelated UI callbacks — `closewindow`, `openwindow`, `playsound`, etc., share this one function, dispatched on the first character of the callback name). The `'d'` branch for `"dialStargateAddress"`:

- Copies the incoming parameter string and repeatedly calls `strtok_s` against it, `atoi`-ing each token into a fixed 7-slot array (`local_260[7]`).
- Requires **more than 6 tokens** (i.e. at least 7) before doing anything else.
- Only then allocates an event payload and fires it once, through `thunk_FUN_0054c900()` / `FUN_00569f10` (the RTTI string cluster confirms this constructs `Event_World_DialStargateAddress`, the world-scoped CME event `class_GateTravel` subscribes to at `0x00e30110` per `stargate-dhd-state-machine.md`).

**Conclusion (HIGH confidence): the Flash DHD UI collects the entire 7-glyph address client-side, in one UI session, before it ever calls into the native game layer.** There is no per-glyph network round trip. This is corroborated independently by the existing `EmitNetOut_onDialGate` finding (`0x00e2e120`, `stargate-dhd-state-machine.md`): the wire message the client actually sends to the server carries **already-resolved** `TargetAddressId`/`SourceAddressId` integers, not raw glyphs — the resolution (6-glyph comparison against the player's known-address vector) happens entirely client-side, before the network call.

**Implication:** the server has no wire-level visibility into in-progress dialling. There is no "chevron N was just entered" message, and there cannot be one without a client patch — the client's own dial UI never sends one.

### 1.2 `runStargateEvent`: confirms the 6100–6113 enum base and how the client's own chevron/vortex animations play

The same dispatcher's `'r'` branch for `"runStargateEvent"`:

```text
iVar13 = atoi(param_3);
*local_2b4 = iVar13 + 0x17d4;   // 0x17d4 = 6100 decimal
... fires Event_World_StargateEvent(eventId) via FUN_0056a010
```

`0x17d4` is exactly **6100**, the `Stargate_MakeGate` base of `ESequenceEventType`'s Stargate range. This independently confirms the enum layout already documented in `docs/gameplay/cinematic-system.md` (6100–6113, 14 sequences) and shows the mechanism: **the DHD's own Flash timeline can command any of the 14 Stargate sequence events (index 0–13 → 6100–6113) directly and immediately, client-side, with no server involvement.** `FUN_0056a010` (`ghidra://SGW.exe@0x0056a010`) constructs a `CME::EventSignal::NoSubject`-typed event carrying the raw event id and emits it through the CME signal bus — the same `Event_World_StargateEvent` that `class_GateTravel` subscribes to at `0x00e30090` (confirmed by decompiling that address: it is the `MemberCallback` vfunc_3 RTTI accessor, returning `&MemberCallback<NoSubject, GateTravel, void(GateTravel::*)(Event_World_StargateEvent const*, void*), Event_World_StargateEvent>::RTTI_Type_Descriptor` — matches `stargate-dhd-state-machine.md`'s row exactly).

**Conclusion (HIGH confidence): the individual chevron-lock animations (6106–6112) and the gate-open/vortex variants (6100–6102) are things the client's own DHD UI is *capable* of triggering entirely locally**, as part of its own dialling presentation, independent of any server round trip.

### 1.3 `Stargate_CrossGate` (6113) is matched per-gate-instance, not globally

`FUN_00e2c810` (`ghidra://SGW.exe@0x00e2c810`) is gated on `*param_1 == 0xd` (13 — the zero-based index of `Stargate_CrossGate` within the 14-event Stargate family, i.e. `6113 - 6100`). It walks a list of level-resident `USeqEvent_Stargate` Kismet event nodes (`ghidra://SGW.exe@0x0069fba0` per `stargate-dhd-state-machine.md`), reads each node's `SourceAddressId` / `TargetAddressId` name-value pairs, and compares them (via `FUN_00d2d8a0`) against the event's own resolved address ids. On the first matching node it calls `FUN_00d2de90` (`ghidra://SGW.exe@0x00d2de90`), which — as part of what is otherwise a reference-counted release/cleanup routine guarded by a flag at `this+0x11` — unconditionally re-fires `0x17e1` (**6113** decimal) as the actual, final dispatch.

**Conclusion (MEDIUM-HIGH confidence): `Stargate_CrossGate` is a real, world-space Kismet trigger bound to a *specific* gate prop instance in the currently loaded map**, selected by matching the origin/destination address pair — not a single global "the gate" object and not UI chrome. This is consistent with (and now independently confirms) Cimmeria's existing per-gate `event_set_id` resolution in `cell::gate_travel::sequences::origin_gate_event_set`.

### 1.4 Chevron events cannot be meaningfully driven by the server — this is now a wire-protocol fact, not a legacy-server inference

Combining 1.1 and 1.2: the wire protocol gives the server **no observable moment** corresponding to "the player just selected chevron N." The entire 7-glyph selection happens in the DHD's Flash UI, which is free to play its own chevron-lock and vortex-formation Kismet triggers (via the exact `runStargateEvent` mechanism in 1.2) at whatever pace the player types, and only reports the *finished* address to the server, once, via `onDialGate`.

**This directly answers the "should the server emit 6106–6112" question independent of what the legacy Python server did or didn't do:** there is no server-observable per-chevron event to key a broadcast on. The best the server could do is guess a cadence and replay it — which would desync from what the dialer's own client has *already played* by the time `onDialGate` even arrives, and would only ever benefit bystanders (see §3).

## 2. Gate-travel cinematic: what fires on crossing, and what's missing

### 2.1 `onStargatePassage` (client method 68) is a real, distinct, currently-unsent RPC

`entities/defs/interfaces/GateTravel.def` declares `onStargatePassage(INT32 addressId)` as its own `ClientMethod`, separate from `onSequence`. The client dispatch table (`docs/protocol/client-method-dispatch-table.md:180`) assigns it index 68. `stargate-dhd-state-machine.md` (2026-05-13) already established, from RTTI, that `class_GateTravel` has its own distinct `MemberCallback` subscriber for `Event_NetIn_onStargatePassage` at `ghidra://SGW.exe@0x00e30010` — **separate from** the `onSequence` handling path. I independently re-decompiled `0x00e30010` this pass and confirmed it is the vfunc_3 RTTI accessor for exactly that `MemberCallback<NoSubject, GateTravel, void(GateTravel::*)(Event_NetIn_onStargatePassage const*, void*), Event_NetIn_onStargatePassage>` type — matching the prior finding verbatim.

Cimmeria declares the wire constant (`crates/services/src/cell/client_methods/gate_travel.rs:10`, `pub const ON_STARGATE_PASSAGE: u16 = 68;`) but **a repo-wide search finds zero call sites that send it.** Cimmeria's crossing path (`cell::gate_travel::on_stargate_passage` in `crates/services/src/cell/gate_travel/mod.rs`) sends only `onSequence(Stargate_CrossGate)` today, never the dedicated method-68 RPC the client's `GateTravel` component is separately listening for.

**Confidence: HIGH that this is a real gap** (declared constant, confirmed real subscriber, confirmed zero call sites). **Confidence: LOW/OPEN on what the client actually *does* upon receiving it** — see §4, open question 1. I was not able to locate the bound member-function pointer (the "vfunc_5 invoke" in this codebase's established CME RTTI-anatomy naming) within this session's budget; only the vfunc_3 RTTI-accessor address is confirmed. Whether `onStargatePassage` is what paces the client's own loading-screen trigger, or is purely a UI/HUD notification, is unresolved.

### 2.2 The current crossing path has no scheduled gap before the world-transition teardown

Reading `crates/services/src/cell/gate_travel/mod.rs` (`handle_stargate_region_entered` → `on_stargate_passage` → `perform_gate_travel`): the `onSequence(Stargate_CrossGate)` send and the `CellToBaseMsg::GateTravel` dispatch (which triggers `RESET_ENTITIES` on the base side) happen in the same async call chain, with **no delay of any kind** between them. Given §1.3's finding that CrossGate is a real world-space Kismet cinematic trigger, this is a structural race: nothing in the current code gives the client a scheduled window to render even one frame of whatever sequence `Stargate_CrossGate` resolves to before its view is torn down.

**Confidence: HIGH that the race exists** (this is a direct code-reading fact, not an inference about the client). **Confidence: LOW on how long the gap needs to be** — I did not extract the `Stargate_CrossGate` Kismet rig's Matinee track length from the cooked map packages this pass (would need either a live-client capture correlating packet timestamps with rendered frames, or a `.upk`/`.umap` Kismet/Matinee parse via `crates/upk-objects`, neither of which was completed).

### 2.3 `Event_NetIn_StargateTriggerFailed` — a confirmed client subscriber Cimmeria neither sends nor models

`stargate-dhd-state-machine.md` records a `class_GateTravel` `MemberCallback` subscriber for `Event_NetIn_StargateTriggerFailed` at `ghidra://SGW.exe@0x00e2ff90`, annotated there as "New — not in gate-travel-wire-formats.md." `docs/protocol/mercury-wire-format.md:678` lists `StargateTriggerFailed` among the Stargate-family client methods. Cimmeria's `client_methods::gate_travel` module only defines constants for `SETUP_STARGATE_INFO` (65), `UPDATE_STARGATE_ADDRESS` (66), `STARGATE_ROTATION_OVERRIDE` (67), and `ON_STARGATE_PASSAGE` (68) — no constant exists for this method at all, and Cimmeria currently signals dial refusal exclusively through `onErrorCode` (`ERRORCODE_SYSTEM_Ability` / 180) rather than this dedicated RPC.

**Not investigated further this pass** (out of scope for the timing questions asked; flagged as an open question in §4).

## 3. Answering the three questions directly

**Q1 (dialing/chevron timing).** The chevron-lock and gate-open visuals the client shows during dialling are driven by the DHD's own Flash UI, entirely client-side, using the same mechanism `runStargateEvent` exposes (§1.2). The server never sees per-chevron progress (§1.1, §1.4) — there is no wire message to key a server-driven chevron broadcast on, and inventing one would desync from what the dialer's own client already played by the time the server even learns a dial happened. `GATE_DIAL_DURATION` (`crates/services/src/cell/space_manager/gate_dial_state.rs:25`, currently `Duration::from_secs(4)`) has **zero client-binary support** — its only source is the disavowed `deprecated/python/cell/SGWPlayer.py`. Structural evidence (§1.1: the DHD window closes on the client's own timeline as soon as the address is fully entered, independent of the server) directly explains tester Lomiada's report that dialling "is quite fast/done when I leave the DHD" — the player is looking at an already-closed DHD and an inert gate for up to four more seconds after their own client considers dialling finished.

**Q2 (gate-travel cinematic before the loading screen).** `Stargate_CrossGate` (6113) is a real, per-gate-instance Kismet trigger (§1.3), and Cimmeria already sends it via `onSequence` before the `RESET_ENTITIES` teardown — the ordering is correct. What's missing is (a) any scheduled gap for it to actually render (§2.2) and (b) the dedicated `onStargatePassage` RPC the client's `GateTravel` component separately listens for (§2.1), whose behavioural role on receipt is unconfirmed.

**Q3 (server-authoritative, no client patch).** Every finding above is client-observation only; nothing here requires a client patch. The chevron-broadcast idea specifically is not just "unsupported by evidence" but **structurally impossible without a client patch**, because the client never reports per-chevron progress over the wire (§1.4).

## 4. Recommendations

| # | Change | Confidence | Why |
|---|--------|------------|-----|
| R1 | Retime `GATE_DIAL_DURATION` down from 4s to the minimum the tick-drain architecture can express (effectively "next 100ms cell tick"), not a new invented multi-second number. | HIGH | The 4s figure's only source is the disavowed legacy Python; the client shows no server-side hold in its own UI flow (§1.1, §3-Q1). |
| R2 | Do **not** implement server-driven broadcast of the chevron events (6106–6112). | HIGH | The wire protocol structurally cannot observe in-progress dialling (§1.1, §1.4) — this is now a binary fact, not an inference from what the legacy server did. |
| R3 | Send `onStargatePassage(addressId)` (client method 68) to the crossing player when `Stargate_CrossGate` fires. | MEDIUM-HIGH | Confirmed real, distinct, currently-unsent RPC with a confirmed client subscriber (§2.1). Behavioural payoff on the client side is unconfirmed, but sending a declared, subscribed RPC that matches the `.def` is a safe, additive, server-only change. |
| R4 | Introduce a short, explicitly provisional delay (with the traveller's movement locked, mirroring `ring_transport`'s `BSF_MovementLock` pattern) between the crossing notifications and the `GateTravel`/`RESET_ENTITIES` teardown. | MEDIUM | The race is confirmed (§2.2); the correct duration is not. A provisional constant, clearly labelled, is defensible; inventing a "final" number is not. |
| R5 | Investigate and, if warranted, model `StargateTriggerFailed` as a real client method rather than relying solely on `onErrorCode`. | LOW/OPEN | Confirmed client subscriber exists (§2.3); not investigated deeply enough this pass to recommend an implementation. |

**None of R1–R5 were implemented in this pass.** The owner's usage budget was exhausted mid-session before R1/R3/R4 could be written *and tested* to the standard this repo requires (byte-exact wire tests, revert-proof timer tests with the existing `gate_dial_state` test harness, and the movement-lock ref-counting edge cases R4 introduces — see §5). Shipping an untested timer/wire change here risks exactly the "happy-path test, not a guard" failure mode `TESTING.md` warns against, so this finding stops at the recommendation.

## 5. Implementation notes for whoever picks this up

- **R1** is a one-line constant change plus doc-comment correction in `crates/services/src/cell/space_manager/gate_dial_state.rs`. The existing tests (`a_fresh_dial_is_not_passable_and_does_not_open_early`, `dial_opens_once_the_deadline_passes_and_only_once`, etc.) reference `GATE_DIAL_DURATION` symbolically, not a hardcoded `4`, so they adapt automatically — but their **prose** ("a 4s dial must not open immediately") needs updating, and a new regression test should assert the dial opens well under a second (not just "eventually"), so a revert back toward multi-second territory is caught.
- **R3** is a straightforward `CellToBaseMsg::EntityMethodCall` send from `cell::gate_travel::on_stargate_passage`, sent to `entity_id` only (not fanned to witnesses — the argument is the traveller's own destination, not something a bystander's client renders). Needs a wire-format byte test (4-byte LE `addressId`, matching the existing `gate-travel-wire-formats.md` table).
- **R4** needs a new `PendingCrossing`-style deferred-tick state (mirroring `PendingGateDial` in the same file) rather than an inline `tokio::time::sleep` — the cell message loop owns `&mut SpaceManager` for the full duration of any handler that holds it, so a direct `sleep().await` inside `handle_stargate_region_entered` would stall the entire cell tick for every other entity in the space for the delay's duration. The movement lock must be explicitly cleared if the deferred `perform_gate_travel` call fails (arrival unusable, base channel closed) — otherwise a traveller whose crossing is refused after the hold starts would be left permanently unable to move. This needs its own regression test alongside the delay-elapsed test.
- All of the above should land together with a doc-comment/timer-test story consistent with the corrected policy in §0 — do not reintroduce a citation to `deprecated/python/cell/SGWPlayer.py` as behavioural justification for a timing constant.

## Open questions

1. **What does the client actually do on receiving `onStargatePassage`?** Only the vfunc_3 RTTI accessor (`0x00e30010`) is confirmed; the bound handler body (vfunc_5 invoke, in this repo's established CME-anatomy terms) was not located this pass. This is the single highest-value follow-up: it would confirm or refute whether this RPC is what paces the client's loading-screen trigger.
2. **What is `Stargate_CrossGate`'s (and `Stargate_MakeGate`'s) actual Matinee/Kismet track duration?** Needs either a live-client capture (packet timestamp correlated with recorded video) or a `.upk`/`.umap` Kismet sequence parse via `crates/upk-objects` against the cooked stargate prefab packages (`GA-Stargate_Prefab_Seq` per `docs/gameplay/gate-travel.md`). Neither was attempted this pass.
3. **`Event_NetIn_StargateTriggerFailed`'s wire payload and intended trigger conditions** — confirmed to exist client-side (§2.3), not otherwise investigated.
4. **Should the no-gate-region immediate-travel fallback** (`cell::gate_travel::handle_dial_gate`'s `!world_has_stargate_region` branch, covering roughly 18 of the ~30 seeded stargates) **also send `onStargatePassage`?** Left out of scope for R3/R4 to keep this pass's recommendations narrow; worth revisiting once R1–R4 land.
5. **Should player input, not just movement, be locked during the R4 hold?** `EKismetViewType`'s `KISMET_VIEW_Witness`/`KISMET_VIEW_EventInvoker` camera-lock semantics (`docs/gameplay/cinematic-system.md`) were not traced specifically to whether `Stargate_CrossGate` locks the camera; if it does, movement-lock alone may be sufficient, but this was not confirmed.

## Cross-reference targets

- [`docs/gameplay/gate-travel.md`](../../gameplay/gate-travel.md) — corrected inline to note the legacy-server-authority retraction; the "Stargate open animation" / "Stargate crossing animation" / "DHD chevron lock animations" status rows should eventually cite this finding instead of (or alongside) D-CA10.
- [`docs/gameplay/cinematic-system.md`](../../gameplay/cinematic-system.md) — the "Of the fourteen, the server emits exactly two" paragraph cites 2009-server behaviour as the reason; needs the same retraction note.
- [`docs/analysis/castle-rebuild/README.md`](../../analysis/castle-rebuild/README.md) — new decision row **D-CA20** added (see below); D-CA10 itself is left unedited per instruction, but is now superseded in its framing.
- [`docs/analysis/npc-ai-restoration/work-packets.md`](../../analysis/npc-ai-restoration/work-packets.md) — NA35 packet entry added.
- [`docs/reverse-engineering/findings/stargate-dhd-state-machine.md`](stargate-dhd-state-machine.md) — this finding corroborates and extends its subscriber table; no correction needed to that document.
- [`docs/reverse-engineering/findings/README.md`](README.md) and [`docs/reverse-engineering/address-map.md`](../address-map.md) — index rows added.
