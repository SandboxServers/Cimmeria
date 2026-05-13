# V5 Documentation Campaign — Session 2 Plan

**Status:** drafted 2026-05-12, awaiting launch approval. Do not launch workers without orchestrator sign-off.

## What's new since session 1

Live enumeration of `SGW.exe` via the Ghidra MCP surfaced naming-family counts the session-1 coordinator did not (it hallucinated some prefixes). Real counts:

| Name family | Real count | What they are |
|---|---:|---|
| `Mercury_*` | 54 | BigWorld Mercury protocol functions |
| `UBWNet*` | 4 | UE3 BigWorld network driver shim |
| `UBWConn*` | 3 | UE3 BigWorld connection shim |
| `BW_client*` | **0** | does not exist (coordinator hallucinated this prefix) |
| `GameEntity*` | 76 (~35 after deduping CME-mangled noise) | GameEntity / GameEntityManager / GameEntityBase methods |
| `EntityManager*` | 50 | overlaps GameEntity; needs dedup |
| `ABigWorldEntity*` | 4 | UE3 BigWorld entity shim |
| `register_NetOut_*` | 254 | event-name registration stubs (return static name string). Trimmed V5. |
| `register_NetIn_*` | 167 | same pattern as register_NetOut. Trimmed V5. |
| `EmitNetOut_*` | 1 (so far) | the emit functions — only `EmitNetOut_DebugMinigameInstance` is named. The rest hide as `FUN_*`. |
| **`_MemberCallback__vfunc_3`** | **1,176** | **per-(event, subscriber-class) handler bodies. The actual game logic. Untouched by session 1.** |

### Architectural picture now visible

For every CME event (e.g. `NetIn_onTimerUpdate`):
- **1** `register_NetIn_TimerUpdate` stub (returns the event name)
- **1** emit function (currently un-named `FUN_*`, except for the debug one)
- **N** `TypedEmitInfo` instances — one per subscriber class, each with destructor `__vfunc_0`
- **N** `CallbackImpl` instances — one per subscriber class, each with RTTI accessor `__vfunc_2`
- **N** `_MemberCallback__vfunc_3` instances — **one per subscriber class, the actual handler body**

Session 1 processed the destructors + RTTI accessors + registration stubs. The handler bodies are the third leg and untouched. **Recommend that's a session-3 campaign** scoped at ~1,176 functions (~15,000 tool calls). Not in session 2.

## Session 2 scope (~1,900 functions)

| Worker | Scope | Count | Address source |
|---|---|---:|---|
| **W0 warmup** | Cyclic-shift sweep on adjacent TypedEmitInfo / CallbackImpl blocks | discovery task | reads existing annotations + RTTI |
| **W0 primary A** | `Mercury_*` | 54 | enumerated below |
| **W0 primary B** | `UBWNet*` + `UBWConn*` (with `0x006f0069` skip-with-note for the decode artefact) | 7 (1 skipped) | enumerated below |
| **W0 primary C** | `register_NetOut_*` filtered by combat/ability/stats/effects suffixes | ~40–60 | filter from the 254 total at session start |
| **W0 primary D** | Apply 5 pending globals + create `StdStringMSVC` struct | 5 globals + 1 struct | itemised below |
| **W1 rescore** | Re-V5 the 57 trimmed-V5 functions from worker-1.checkpoint.json with full destructor plates | 57 | explicit list below |
| **W4-A** | Continue address sweep `0x00de1c00 → 0x00de5fff` | ~70 | already explicit |
| **W4-B1** | EntityDescription parse chain (low half) `0x01590000 → 0x01593fff` | ~200 | already explicit |
| **W4-B2** | EntityDescription parse chain (high half) `0x01594000 → 0x01598fff` | ~200 | already explicit |
| **W4-C** | Real `GameEntity*` + `EntityManager*` + `ABigWorldEntity*` after deduping CME-mangled noise | ~80 | filter rule below |
| **W5-A** | `_MemberCallback__vfunc_3` shard 1 (address `0x00400000 → 0x00c98e1f`) | ~235 | quintile boundary |
| **W5-B** | `_MemberCallback__vfunc_3` shard 2 (address `0x00c98e20 → 0x00ccae9f`) | ~235 | quintile boundary |
| **W5-C** | `_MemberCallback__vfunc_3` shard 3 (address `0x00ccaea0 → 0x00d46cdf`) | ~235 | quintile boundary |
| **W5-D** | `_MemberCallback__vfunc_3` shard 4 (address `0x00d46ce0 → 0x00dfa81f`) | ~235 | quintile boundary |
| **W5-E** | `_MemberCallback__vfunc_3` shard 5 (address `0x00dfa820 → 0x01570a10`) | ~236 | quintile boundary |

**Total: ~1,900 V5 actions + 57 W1 rescores + 1 discovery sweep**. Roughly 2.5× session 1. Budget at ~13 calls/function average: ~25K tool calls.

## Explicit address lists

### W0 primary A — Mercury_* (54)

```
00de1670  Mercury_InputMessageHandler__vfunc_0
00de16a0  Mercury_ReplyMessageHandler__vfunc_0
00de16d0  Mercury_TimerExpiryHandler__vfunc_0
00de1700  Mercury_BundlePrimer__vfunc_0
01576cf0  Mercury_Channel__vfunc_0
01577210  Mercury_BaseNub_ProcessMessageHandler__vfunc_0
01577310  Mercury_Nub_3
01577b80  Mercury_Nub_9
01577e10  Mercury_BaseNub__vfunc_0
01579830  Mercury_Bundle
01579a50  Mercury_Bundle_3
0157a7a0  Mercury_Bundle_2
0157aaf0  Mercury_Bundle__vfunc_0
0157ac90  Mercury_Bundle_newMessage
0157af00  CME_5_VWin32ThreadEx_..._Mercury_Nub_UNetworkTask___Thread__vfunc_0
0157b090  Mercury_NubException__vfunc_0
0157b0d0  Mercury_Nub_ReplyHandlerElement__vfunc_0
0157bd30  Mercury_Nub_handleMessage
0157c820  Mercury_Nub_6
0157d9a0  Mercury_Channel_14
0157db80  Mercury_Nub_10
0157e480  Mercury_Channel_13
0157e920  Mercury_Nub
0157eb00  Mercury_Channel_15
0157ec70  Mercury_Nub_2
0157f4a0  tbb_..._Mercury_VClientMessage_..._concurrent_queue__vfunc_0
0157fd20  Mercury_Nub_14
01580840  Mercury_Nub_12
01580a9e  Mercury_Nub_13
01580ad4  Mercury_Nub_5
01581ab0  Mercury_Nub_11
01582160  Mercury_Nub_4
01583440  Mercury_Nub_addListeningSocket
01583820  Mercury_Nub_15
01583a00  Mercury_Nub_Connection__vfunc_0
01583a20  Mercury_Nub__vfunc_0
01583a90  Mercury_Nub_writeConnection
015841d0  Mercury_Nub_Nub
01584920  Mercury_Endpoint
015898c0  Mercury_MachineGuard
0158a8c0  Mercury_Packet__vfunc_0
0158acc0  Mercury_InterfaceElement_compressLength
0158b120  Mercury_InterfaceElement_compressLength_2
0158b770  Mercury_InterfaceElement_expandLength
0158d050  Mercury_Channel_18
0158d207  Mercury_Channel_16
0158d2b0  Mercury_Channel_17
0158d3b0  Mercury_ChannelInternal__vfunc_0
0158d4b0  Mercury_ClientMessage__vfunc_0
0158d680  Mercury_ClientExceptionMessage__vfunc_0
0158d730  Mercury_ClientNetMessage__vfunc_0
0158d850  Mercury_ClientIncomingMessage__vfunc_0
0158daf0  Mercury_ClientChannelRegMessage__vfunc_0
01604330  Mercury_PacketFilter__vfunc_0
```

Address range is non-contiguous: 4 in `0x00de1670–0x00de1700`, 49 in `0x01576cf0–0x01604330`. The cluster in `0x01577000–0x0158daf0` is dense (~50 of them) and represents the Mercury implementation.

### W0 primary B — UBWNet* + UBWConn* (7)

```
004804c0  UBWNetDriver_IsSupported
004804d0  UBWNetDriver_IsConnected
00480830  UBWNetDriver_Initialize
006f0069  UBWNetDriver__vfunc_83        ← odd address, suspect alignment/decoding artefact
004809b0  UBWConnection_IsSupported
004809c0  UBWConnection_IsConnected
00480b90  UBWConnection_Initialize
```

**Confirmed:** `0x006f0069` is a decoding artefact (oddly aligned, isolated, no real callsites). W0 logs `status: "skipped_decode_artefact"` with `notes: "0x006f0069 is mid-instruction, not a real function entry; decoded by Ghidra during the original auto-analysis pass."` and does NOT attempt V5 on it.

### W0 primary C — register_NetOut_* combat/ability/stats/effects (~40–60, to be filtered at session start)

**Filter rule for W0:** enumerate `register_NetOut_*` (254 total), keep only suffixes matching any of these globs:
```
*Ability*, *UseAbility*, *Combat*, *Stat*, *Effect*, *Timer*, *Ammo*, *Reload*, *Crouch*, *LOS*, *Melee*, *Threat*, *AutoCycle*, *ConfirmEffect*, *RespecAbility*, *AlignmentUpdate*, *TestLOS*, *ToggleCombat*, *RequestReload*, *RequestAmmoChange*, *SetCrouched*, *KnownAbilities*, *AbilityTreeInfo*
```

Trimmed V5 applies — these are all 3-line `return <name_string>` stubs.

### W0 primary D — globals from session-1 W1 checkpoint (3, not 2)

```
0x01ee2bb8  → g_pUBWNetDriverClass      (proposed in W0's own session-1 checkpoint)
0x01ee2bbc  → g_pUBWConnectionClass     (proposed in W0's own session-1 checkpoint)
0x019ACEF4  → g_szMinigameInstanceField2Name   (from W1 pending_globals)
0x019ACEE8  → g_szMinigameInstanceField1AltName (from W1 pending_globals)
0x019ACF38  → g_szMinigameInstanceField3Name   (from W1 pending_globals)
```

(That's 5 globals total, not 2 or 3 — I undercounted in the table above. W0 owns `rename_data` / `rename_or_label`.)

Plus the struct `StdStringMSVC` from W1's checkpoint:
- `+0x00 undefined4[4]` SBO buffer
- `+0x10 uint capacity`
- `+0x14 uint length`
- Evidence: MSVC std::string SSO pattern at `EmitNetOut_DebugMinigameInstance` (`0x00c79120`).

### W0 warmup — cyclic-shift sweep

**Goal:** find the next contactList-style bug. Method:

1. Enumerate every contiguous block of TypedEmitInfo / CallbackImpl / register_Net* functions in address order (use `list_functions` filtered by name pattern, then bin by address adjacency).
2. For each block of ≥4 adjacent functions, list the (address, Ghidra-symbol-name, RTTI-class-name) tuple.
3. Look for cyclic mismatches: the Ghidra symbol claims "FooEvent" but RTTI says "BarEvent" while the next slot has the inverse.
4. Document findings in a new `docs/reverse-engineering/findings/annotation-script-shift-bugs.md`. Apply corrections only when the RTTI name is unambiguous.

**Budget:** ~15-20 tool calls. **Output:** the new findings doc + any additional `pending_globals` for misassigned function names.

### W1 rescore — 57 addresses

The 57 functions in `worker-1.checkpoint.json` (all `register_NetOut_*` or `register_NetIn_*` stubs, trimmed in session 1):

Cluster A (NetOut, 48):
```
00ae9dd0 SetRingTransporterDestination
00aea3d0 RespecCraft
00cb29f0 ShareMission
00cb2ba0 ShareMissionResponse
00cb2d50 MissionAbandon
00cb3da0 debugStartMinigame
00cb3f50 debugSpectateMinigame
00cb41f0 debugJoinMinigame
00cb4490 MinigameComplete
00cb4640 GiveMinigameContact
00cb47f0 RemoveMinigameContact
00cb5810 MissionAssign
00cb59c0 MissionClear
00cb5b70 MissionClearActive
00cb5e10 MissionClearHistory
00cb60b0 MissionList
00cb6350 MissionListFull
00cb65f0 MissionDetails
00cb67a0 MissionAdvance
00cb6950 MissionReset
00cb6b00 MissionComplete
00cb6cb0 MissionSetAvailable
00cbc3e0 GiveStargateAddress
00cbc590 RemoveStargateAddress
00d1f920 AbandonMission
00d1fad0 ChosenRewards
00d8fe80 DHD
00d90e40 debugMinigameInstance (duplicate stub site)
00d910e0 StartMinigame
00d91380 EndMinigame
00d91620 RegisterToMinigameHelp
00d918c0 UpdateRegisterToMinigameHelp
00d91b60 RequestSpectateList
00d91e00 SpectateMinigame
00d920a0 MinigameStartCancel
00d92340 MinigameCallRequest
00d925e0 MinigameCallAbort
00d92880 MinigameCallAccept
00d92b20 MinigameCallDecline
00d92dc0 MinigameContactRequest
00d93060 onDialGate
00d96f70 SetTechSkill
00e4a910 Craft
00e4aac0 Alloy
00e4ac70 Research
00e4ae20 ReverseEngineer
00e4afd0 SpendAppliedSciencePoint
```

Wait — that's 46, not 48. Let me note: the checkpoint excerpt I read covered W1 entries 1–58; some are duplicates or have been re-categorized. The exact rescore list = every entry in `worker-1.checkpoint.json`'s `functions[]` array where `workflow == "trimmed"` AND `name_at_end` starts with `register_NetOut_` or `register_NetIn_`. **W1 worker reads its own checkpoint to build this list.** Not error-prone.

Cluster B (NetIn, 11):
```
00d771e0 onEffectResults
00d77480 KnownAbilitiesUpdate
00d7f520 TimerUpdate
00d80250 AbilityTreeInfo
00d821e0 onLOSResult
00d860e0 onMeleeRangeUpdate
00d86620 onStatUpdate
00d868c0 onStatBaseUpdate
00d86b60 onAlignmentUpdate
00d8d480 onThreatenedMobsUpdate
```

That's 10. Plus the one full V5 already done at `0x00c79120` (`EmitNetOut_DebugMinigameInstance`) which doesn't need rescore.

**W1 rescore action per function:** these are real stubs (3-line `return <name>`), NOT destructors. The W1 session-1 trimmed-V5 call was correct in process but the **plate quality is thin** — the brief now requires explanation of why they're stubs and how they relate to the emitters + member-callbacks. Action: full V5 plate explaining the role in the CME event signal chain, cross-link to `findings/cme-event-signal.md`. Score before is 75 (post-script-04 plate); target after is 100.

### W4-A — continue 0x00de1c00 → 0x00de5fff (~70)

Address range explicit. Workers resume from `resume_from_address: 0x00de1c00` in `worker-4.checkpoint.json`. Already-completed addresses in that checkpoint are skipped.

### W4-B1 — EntityDescription parse chain low half `0x01590000 → 0x01593fff` (~200)

Pre-split per orchestrator decision (session-1 evidence showed ~400 single-worker scope hits the turn boundary). Anchors in this half from session 1's `address-map.md`: `0x015924a0` (parseProperties), `0x01593cd0` (parse dispatch). Also `0x015652d0` (FNetworkPropertyChange__vfunc_0) is outside this range; not in W4-B1's scope.

### W4-B2 — EntityDescription parse chain high half `0x01594000 → 0x01598fff` (~200)

Anchors in this half: `0x01594f60` (MethodDescription_parse), `0x015974a0` (DataDescription_parse_2).

**W4-B1 and W4-B2 use separate checkpoint files** (`worker-4b1.checkpoint.json`, `worker-4b2.checkpoint.json`). No struct contention expected — both reference the existing `EntityDescription` / `DataDescription` / `MethodDescription` types that W0 should create from session-1 pending_structs requests.

### W5 — `_MemberCallback__vfunc_3` campaign (1,176 total, 5 address-disjoint shards)

**Background.** Session-1 enumeration revealed 1,176 functions matching `_MemberCallback__vfunc_3` — these are the actual handler bodies (one per `(event, subscriber-class)` pair), the real game logic. Session 1 missed all of them because no worker's scope included this name pattern.

Names are heavily mangled (CME EventSignal subscriber template instantiations). Each worker enumerates `_MemberCallback__vfunc_3` filtered by its address range, intersects with the explicit range bound, and processes in ascending address order. Many will be small (~5–30 lines) dispatch wrappers that forward to a non-callback method; full V5 still applies — the plate documents which event they handle and which subscriber method they invoke.

Quintile boundaries derived from `search_functions_enhanced` with `sort_by: address` and offsets `[0, 235, 470, 705, 940, 1175]`:

| Shard | Address range (inclusive low / exclusive high) | Anchor at offset 0 of shard | ~Count |
|---|---|---|---:|
| W5-A | `0x00400000 → 0x00c98e1f` | `0x00426860` (first MemberCallback by address) | ~235 |
| W5-B | `0x00c98e20 → 0x00ccae9f` | `0x00c98e20` (SGWTextCommandMgr SlashCmd_ShowLog) | ~235 |
| W5-C | `0x00ccaea0 → 0x00d46cdf` | `0x00ccaea0` (SGWScriptedWindow CreationVisualsUpdate) | ~235 |
| W5-D | `0x00d46ce0 → 0x00dfa81f` | `0x00d46ce0` (SGWNetworkManager NetOut_RepairItem handler) | ~235 |
| W5-E | `0x00dfa820 → 0x01570a10` | `0x00dfa820` (VGameProxyPlayer Entity_StatUpdate) | ~236 |

**Per-shard workflow:**

1. Enumerate `MemberCallback` with `name_pattern=MemberCallback`, no regex, paginated with `limit=50` until the result's first address exceeds the shard's high bound.
2. Filter to the shard's `[low, high)` address range.
3. For each in ascending address order: apply full V5. Plate template MUST include:
   - **Event:** which event class (extracted from the mangled name's `_PAX_AEXPB*Event_<X>_` segment).
   - **Subscriber:** which class registered the callback (the `P{nn}_V*` segment).
   - **Handler shape:** what the callback body does — typically `unpack args → call subscriber method → return`.
4. Checkpoint to `worker-5{a,b,c,d,e}.checkpoint.json` every 50 functions.
5. **Naming convention:** rename mangled `CME_EventSignal_...___MemberCallback__vfunc_3` to `OnEvent_<EventName>__<SubscriberClass>` (e.g. `OnEvent_NetIn_TimerUpdate__VGameEntityManager`). The `NamingConventions.java` validator will accept this; if it rejects, fall back to `Callback_<EventName>__<SubscriberClass>`.
6. **Pending structs/globals:** flush to checkpoint for W0. Common candidate: `CmeEventSignalSubject` with the SubscriberRefCount + EventDataPointer offsets — W0 creates once, all W5 shards reference.

**Failure modes:**
- **Many duplicate shapes.** Most MemberCallbacks follow the same 5–10 line pattern: `cast event data → call method → return`. Workers will be tempted to copy-paste plate text. Accept this — V5 doesn't require unique plates, only correct ones. Save tokens by templating.
- **Subscriber method may be obfuscated.** If the callback invokes a `FUN_*` that hasn't been named, log to `pending_callees` in checkpoint; don't recursively V5 the callee (that's session 3+ work, otherwise infinite recursion).
- **Tool-call budget.** Each W5 shard ≈ 235 × ~10 calls (mostly trimmed-ish due to repetitive shapes) = ~2,350 calls. Total W5 ≈ 11,750 calls. Plus all the other workers ≈ 25K total session-2 budget. Confirms the ~25K estimate.

### W4-C — GameEntity / EntityManager / ABigWorldEntity (~80 after dedup)

Filter rule: function name matches ONE of:
- Exact prefix `GameEntity` (excluding names containing `CME_EventSignal_` — those are CME-mangled member callbacks, not GameEntity methods proper)
- Exact prefix `EntityManager` (no CME_ in name)
- Exact prefix `ABigWorldEntity`

**Pre-deduped list size:** 76 + 50 + 4 = 130. After dedup (`GameEntity*` and `EntityManager*` overlap heavily on `GameEntityManager_*` names) and CME-mangled exclusion: ~80.

**Note:** W4-C will discover many already-named functions (e.g. `GameEntityManager_EmitEntityMethod` at `0x00dd0980`, `GameEntityManager_BuildEntityMethodPacket` at `0x00dd1510`). Apply `analyze_function_completeness` first; many will already score ≥80 with the session-1 W4 plates. Trimmed pass (rename-and-prototype only) for stub `__vfunc_N` entries with no body.

## Cross-worker coordination (session 2)

Same as session 1, with two amendments:

**Amendment 1 — Explicit address-range fencing.** Every worker's prompt explicitly lists its scope as `(address-set | name-pattern-set)`. Workers **must intersect** their discovered enumeration with the explicit set before processing any function. No address outside the explicit set is touched. This is the structural fix for the W0 overlap incident.

**Amendment 2 — Shared checkpoint directory.** All session-2 checkpoints go in `docs/reverse-engineering/v5-campaign/`. W4 splits use sharded checkpoint files (`worker-4a.checkpoint.json`, `worker-4b.checkpoint.json`, `worker-4c.checkpoint.json`) to avoid write races. W0 still owns `worker-0.checkpoint.json`. W1 rescore appends to the existing `worker-1.checkpoint.json` with a `rescore_session_2` block (don't overwrite session-1 entries).

## Recommended launch order

1. **W0 warmup (cyclic-shift sweep)** — fast, ~15-min, produces `findings/annotation-script-shift-bugs.md`.
2. **W0 struct/global flush** — apply `StdStringMSVC` + plan for `EntityDescription` / `DataDescription` / `MethodDescription` / `CmeEventSignalSubject`, plus the 5 pending globals. ~10 tool calls. Writes `structs-ready-v2.flag` to unblock the rest.
3. **W0 primary A+B+C** — Mercury, UBW*, register_NetOut combat. ~110 functions full V5. **W0 still serial — runs to completion before next batch.**
4. **In parallel after W0 writes `structs-ready-v2.flag`** (10 concurrent agents):
   - **W1 rescore** (~57 functions, trimmed→full V5 plate upgrade)
   - **W4-A** (~70 functions, resume from checkpoint)
   - **W4-B1** (~200 functions, EntityDescription parse chain low half)
   - **W4-B2** (~200 functions, EntityDescription parse chain high half)
   - **W4-C** (~80 functions, named clusters, many already-complete)
   - **W5-A** (~235 MemberCallbacks, `0x00400000–0x00c98e1f`)
   - **W5-B** (~235 MemberCallbacks, `0x00c98e20–0x00ccae9f`)
   - **W5-C** (~235 MemberCallbacks, `0x00ccaea0–0x00d46cdf`)
   - **W5-D** (~235 MemberCallbacks, `0x00d46ce0–0x00dfa81f`)
   - **W5-E** (~236 MemberCallbacks, `0x00dfa820–0x01570a10`)

W5-A is the longest expected runtime (densest CME-callback cluster in the early address space) — start it first in the parallel batch. W4-A is the shortest — start it last (or interleave).

**Concurrency reality check:** 10 parallel agents on one Ghidra HTTP endpoint will serialize at the plugin layer. Practical throughput stays ~4–6 calls/min regardless of agent count. The benefit of parallelism is **harness-turn resilience** (one stalling worker doesn't block the others) and **token budget distribution** (each agent's turn boundary frees independently). Wall-clock for session 2 with 25K calls at ~6 calls/min: roughly **70 hours of cumulative Ghidra work**, spread across whatever number of harness turns the agents collectively need.

## Per-worker prompt drafts

Each worker reads `docs/reverse-engineering/v5-campaign/WORKER_BRIEF.md` first. Drafts of the per-worker prompts live in the same directory as `prompt-w0.md`, `prompt-w1.md`, `prompt-w4a.md`, `prompt-w4b1.md`, `prompt-w4b2.md`, `prompt-w4c.md`, `prompt-w5a.md`, `prompt-w5b.md`, `prompt-w5c.md`, `prompt-w5d.md`, `prompt-w5e.md`.

**Drafts not yet written** — orchestrator drafts on launch day so they reference this plan by line/section.

## Decisions taken 2026-05-13

1. **Tackle all 1,176 `_MemberCallback__vfunc_3` functions in session 2.** Split into 5 address-disjoint shards W5-A through W5-E. ~25K total session-2 tool-call budget.
2. **`0x006f0069` UBWNetDriver__vfunc_83 is a decode artefact** — W0 skip with note.
3. **W4-B pre-split into W4-B1 (`0x01590000–0x01593fff`) and W4-B2 (`0x01594000–0x01598fff`)** — avoids the turn-boundary issue W4 hit in session 1.
