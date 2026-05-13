# V5 Documentation Campaign — Status Aggregator

Live log of worker results as they report back. Consolidator (Task #10) reads this + each worker's `worker-N.checkpoint.json` to produce the final report.

## Session 1 — 2026-05-12

### Discovered scope-predicate mismatch

The coordinator's partition plan assumed Ghidra contains functions named `CME_EventSignal_VEvent_NetOut_*` / `CME_EventSignal_VEvent_NetIn_*` (1,469 + 334 = 1,803 per its survey).

**Reality (from W1):** No such names exist. Annotation script `04_event_signal_annotator.py` applied `register_NetOut_<Event>` / `register_NetIn_<Event>` prefixes — total 479 NetOut + 496 NetIn = 975 per STATUS.md, of which 419 were renamed.

Workers W0/W2/W3/W4 will independently rediscover this via the same path W1 used: enumerate via `search_functions_enhanced` with the documented prefix, hit zero matches, read `02_..._annotator.py` and `04_event_signal_annotator.py` for the real naming convention, and adjust their `total_in_scope`.

**Revised total campaign scope estimate:** ~1,200–1,400 functions (was 2,300).

### Worker Reports

#### W1 — Complete (session 1)

| Metric | Value |
|---|---|
| Functions processed | 58 / 58 in scope |
| Tool calls used | 112 |
| Status | scope-complete this session |
| Checkpoint | `worker-1.checkpoint.json` |

**Findings:**

1. **`0x00c79120` is the only non-stub in W1's scope.** Originally named `register_NetOut_debugMinigameInstance` by script 04 — but the function is a 154-line CME event emitter, not a 3-line accessor. Renamed: `EmitNetOut_DebugMinigameInstance`. The other 57 functions in W1's scope are pure stubs returning a static name string (trimmed V5 applies).

2. **Canonical CME emit pipeline discovered.** `EmitNetOut_DebugMinigameInstance` (W1's one full V5 function) reveals the CME EventSignal dispatch path at 4 callee addresses. Add to `address-map.md`:

   | Address | Proposed name | Role |
   |---|---|---|
   | `0x005783b0` | `CmeEventData_GetField` | Extract named field from event data object |
   | `0x0155f790` | `CmeEventSignal_GetSystem` | Singleton accessor for CME EventSignal system |
   | `0x00a5c0f0` | `CmeEventSignal_LookupByName` | Resolve signal handle by name string |
   | `0x0043b850` | `CmeEventSignal_SetField` | Set key/value field on a signal object |
   | `0x00c79120` | `EmitNetOut_DebugMinigameInstance` | Emitter (only non-stub in W1's scope) |

   Pattern: `GetSystem → LookupByName → SetField × N → vtable dispatch`. This is the receipt for how every CME emitter is structured.

3. **Pending struct for W0:** `StdStringMSVC` — MSVC SSO layout, `+0x00 undefined4[4]` (16-byte SBO buffer), `+0x10 uint` capacity, `+0x14 uint` length. Evidence from `local_24[4]` and `local_40[4]` in `EmitNetOut_DebugMinigameInstance`. Used pervasively in CME emitter code.

4. **Pending globals for W0:**
   - `0x019ACEF4` → `g_szMinigameInstanceField2Name`
   - `0x019ACEE8` → `g_szMinigameInstanceField1AltName`
   - `0x019ACF38` → `g_szMinigameInstanceField3Name`

5. **No contradictions with existing findings docs.** W1's scope didn't overlap any documented wire format closely enough to conflict.

#### W0 — Partial (session 1) — scope-overlap incident

| Metric | Value |
|---|---|
| Functions processed | 243 / 580 enumerated |
| Tool calls used | ~1,050 |
| Status | partial — Mercury/UBWNet/NetOut-combat scope NOT yet touched |
| Checkpoint | `worker-0.checkpoint.json` |

**Coordination failure:** W0 rediscovered the `register_NetOut_*` predicate mismatch, but instead of filtering to its assigned `combat/ability/stats/effects/Mercury/UBWNet/BW_client` subset, enumerated ALL `register_NetOut_*` matches and processed in address order. This put W0 inside W1's and W2's scopes (Minigame, Organization, Mail, Trade, BM, contactList, Craft). Several functions had "plate already from prior workers" — confirming the overlap. Per-call atomic transactions in the Ghidra plugin prevented corruption, but the work was redundant.

**Primary work NOT done by W0 this session:**
- Mercury_* protocol functions (~68)
- UBWNet*, UBWConn*, BW_client* (~16)
- NetOut combat/ability/stats/effects handlers (~250)

These are W0's session-2 priority. **Total W0 session-2 remaining scope: ~334 functions.**

**Findings:**

1. **ContactList cyclic name misassignment — CORRECTED:**

   | Address | Prior label | Actual RTTI name |
   |---|---|---|
   | `0x00e5f990` | ContactListAddMembers | **contactListRename** |
   | `0x00e5f9b0` | ContactListRemoveMembers | **contactListFlagsUpdate** |
   | `0x00e5f9d0` | ContactListRename | **contactListAddMembers** |
   | `0x00e5f9f0` | ContactListSetFlag | **contactListRemoveMembers** |

   Cyclic shift — each label off by one slot. Prior annotation script picked up the wrong string xref for adjacent functions. **Pattern of concern:** similar bugs may exist in other adjacent-function clusters. Recommend a session-2 sweep that compares Ghidra RTTI names to annotation-script-assigned labels in every contiguous TypedEmitInfo block.

2. **Two undocumented client→server telemetry pushes:**
   - `0x00d9cc40` `SystemOptions` — sends hardware/performance info.
   - `0x00d9cee0` `PerfStats` — sends FPS/latency metrics.

   Neither is in `docs/protocol/`. Cimmeria server should handle gracefully (likely no-op or log).

3. **`contactListFlagsUpdate` is a contact-list method not in any current protocol doc** — wire-format candidate for `docs/reverse-engineering/findings/contact-list-wire-formats.md`. The cyclic-shift fix surfaces it as a real method, distinct from add/remove members.

4. **Pending globals NOT processed (still W0-owned, deferred to session 2):**
   - `0x01ee2bb8` → `g_pUBWNetDriverClass`
   - `0x01ee2bbc` → `g_pUBWConnectionClass`

5. **175 CallbackImpl__vfunc_2 stubs at `0x00d43df0`–`0x00d44ce0`** were checked and found `skipped_already_done` (effective score 75+, prior workers applied plates). Verified, not enumerated in checkpoint's functions array.

#### W2 — Complete (session 1)

| Metric | Value |
|---|---|
| Functions processed | 176 / 176 in scope |
| Tool calls used | 620 |
| Status | scope-complete this session |
| Checkpoint | `worker-2.checkpoint.json` |

**Findings:**

1. **WORKER_BRIEF correction (critical for W3):** `TypedEmitInfo__vfunc_0` is NOT a name-string accessor — it's the **MSVC virtual destructor** (`~TypedEmitInfo()`). Body calls a per-event cleanup function, then conditionally `scalable_free(pThis)` if `bDeallocate & 1`. Full V5 applies (score 37 → 73; structural ECX void* deduction unfixable). The trimmed-V5 guidance in WORKER_BRIEF for this family is wrong.

   *Implication:* W1's 57 "stub returning name string" functions in the mission/crafting/gate/minigame/combat/stats cluster were probably also destructors that W1 misread, OR they're a distinct family. Worth verifying when W3 reports — W3's scope includes the broader EventSignal infrastructure.

2. **New address-map clusters:**

   | Address range | Cluster |
   |---|---|
   | `0x00d43e30–0x00d44c80` | NetOut CallbackImpl__vfunc_2 RTTI type descriptor accessors (uniform 0x10-spacing) |
   | `0x00e11cb0–0x00e11cd0` | NetIn store CallbackImpl cluster (`onStoreOpen` / `Update` / `Close`) |
   | `0x00e219b0–0x00e21a10` | NetIn inventory CallbackImpl cluster (`onContainerInfo` through `CashChanged`) |
   | `0x00e24810` | LootDisplay CallbackImpl — isolated from inventory cluster by ~0x2E00, suggests different compile unit |

3. **Architectural anomalies (warrant follow-up):**
   - **Black Market** (`BMCreateAuction`, `BMCancelAuction`, `BMPlaceBid`, `BMSearch`) has TypedEmitInfo entries but NO `CallbackImpl__vfunc_2` pair. Different callback registration mechanism — possibly direct function pointers instead of the typed CME signal pattern.
   - **GiveInventory** has the same anomaly (TypedEmitInfo but no CallbackImpl pair).
   - **Trade has duplicate TypedEmitInfo instances** at `0x00d2ad10`/`0x00d2aa70` and `0x00e266c0`/`0x00e26700` for the same event names (`TradeRequestCancel`, `TradeLockState`). Legitimately separate signal objects for different subsystems handling the same wire event — both documented.

4. **`contactList*` naming inconsistency:** Pre-existing annotation artifact — script used camelCase for contactList events (`contactListCreate`, `contactListDelete`, `contactListRename`, `contactListFlagsUpdate`, `contactListAddMembers`, `contactListRemoveMembers`) but PascalCase everywhere else. W2 did not override; consolidator should decide whether to normalize.

5. **No new structs or globals needed.** Everything decompiled in W2's scope was already named by prior annotation scripts.

#### W3 — Complete (session 1)

| Metric | Value |
|---|---|
| Functions processed | 204 complete + 4 already-compliant / 208 in scope |
| Tool calls used | ~1,950 |
| Status | scope-complete this session |
| Checkpoint | `worker-3.checkpoint.json` |

**Findings:**

1. **Confirmed W2's destructor finding** across the entire NetIn TypedEmitInfo family (187 functions) plus 17 NetOut TypedEmitInfo and 17 CallbackImpl. Every `TypedEmitInfo__vfunc_0` is the MSVC scalar destructor (`~TypedEmitInfo()`); every `CallbackImpl__vfunc_2` is the RTTI type-name accessor (returns compile-time `TypeDescriptor` pointer, NOT a name string).

2. **Structural score ceiling for TypedEmitInfo__vfunc_0 is ~78.** Unfixable deduction: `void* this` in `__thiscall` cannot be retyped via the MCP API. Accepted as a known gap, not a worker error.

3. **Prior annotation scripts left 3-line stub plate comments on Cluster B CallbackImpl__vfunc_2 functions** — they scored 75 on first check. W3 brought all 17 to ~80 with full V5 plates. Not a contradiction with findings docs; a quality gap in the annotation-script output for that family.

4. **Cluster B address inventory** (all pre-named; included for consolidator cross-ref):

   - TypedEmitInfo__vfunc_0: `0x00c95430` Kill, `0x00c95450` SetGodMode, `0x00c95630` Spawn, `0x00c95650` Despawn, `0x00c95750` SetMovementType, `0x00c958b0–0x00c959b0` Load* family (9), `0x00c95b50` Unstuck, `0x00cb5650` Respawn, `0x00d34560` CreateCharacter
   - CallbackImpl__vfunc_2: `0x00d43eb0` CreateCharacter, `0x00d43f50` SetMovementType, `0x00d44340` Respawn, `0x00d44680` SetGodMode, `0x00d44a60` Spawn, `0x00d44a70` Despawn, `0x00d44ab0` Kill, `0x00d44b20–0x00d44ba0` Load* family (9), `0x00d44c50` Unstuck

5. **No new address-map entries, no new structs, no new global renames.** All Cluster B addresses were already named by prior annotation scripts.

#### W4 — Partial (session 1) — hit turn boundary at 234/~830

| Metric | Value |
|---|---|
| Functions processed | 234 / ~802 enumerated |
| Tool calls used | ~1,975 |
| Status | partial — `resume_from_address: 0x00de1c00` |
| Checkpoint | `worker-4.checkpoint.json` |

**Session-2 split proposed by W4 itself** (to fit within harness turn boundaries):

- **W4-A** — resume the current sweep `0x00de1c00 → 0x00de5fff`. ~60–70 functions. Inherits W4's decompile patterns and BigWorld property-type-constant context.
- **W4-B** — EntityDescription parse chain `0x01590000 → 0x01598fff`. Separate call graph from the 00de* range; no overlap with W4-A. Anchors: `0x01593cd0`, `0x015924a0`, `0x01594f60`, `0x015974a0`, `0x015652d0`.
- **W4-C** — name-clusters: `GameEntity*` (73), `EntityManager*` (34), `ABigWorldEntity*` (4). Already non-`FUN_` prefixed; many will skip-check on first call.

Total W4 session-2 scope: ~568 functions.

**Coordination note:** each W4-x agent should write to a separate shard file (`worker-4a.checkpoint.json`, `worker-4b.checkpoint.json`, `worker-4c.checkpoint.json`) to avoid checkpoint-write races against the existing `worker-4.checkpoint.json`. Consolidator merges them at end.

---

## Session 2 — W1-rescore replacement (2026-05-13)

47 rescored / 57 in scope / 78 tool calls. Checkpoint `worker-1.checkpoint.json` with `rescore_session_2` block.

**Important campaign-wide finding — V5 plate-format quality bug:**

The `analyze_function_completeness` `plate_issues` check is a **structural parse** check, not a semantic-richness check. Session-1 plates on the 47 NetOut stubs scored 85 (not 100) because of two parse-level defects:

1. `Algorithm:` / `Parameters:` / `Returns:` section keys must be **unindented top-level headers** — indented keys under a wrapper block are penalized.
2. Spurious `[IMPLICIT ECX: VEvent_* instance pointer]` claims on functions whose Ghidra signature is `char * name(void)` are penalized — void functions have no implicit ECX in this context.

The 10 NetIn stubs scored 100 in session 1 because they used the correct flat format.

**Implication:** any plate scoring <100 on a stub should be checked for (a) indented section headers and (b) spurious parameter claims on void functions. This applies to all 421 `register_Net*` stubs (254 NetOut + 167 NetIn) and likely to many of the TypedEmitInfo/CallbackImpl stubs touched in session 1.

**Action for consolidator:** amend `WORKER_BRIEF.md` to mandate flat unindented section headers in plates and to drop `[IMPLICIT ECX]` lines on void functions.

## Session 2 — incident log (2026-05-13)

**Coordination misjudgment.** First-wave session-2 workers were told to flag-wait via "Read another file to pass time" — orchestrator (me) assumed this returned the agent early. It did NOT; agents looped through their retry budget long enough to outlast W0's struct flush, saw the flag, and started doing real V5 work. Orchestrator then launched 10 fresh relaunches AND TaskStop'd the 9 originals (excluding W4-A which had genuinely returned early and W0 which never waited). Kill notifications confirmed the 9 were actively processing.

**Net impact:**
- Ghidra writes from killed agents are durable (per-call atomic).
- Some functions are now in mid-V5 state (rename done, no plate; or plate done, no local typing).
- 10 fresh relaunches absorb via `analyze_function_completeness` skip-check.
- Wasted tool calls on skip-rechecks of already-touched functions.

**Lost-in-transcript leads (worth re-discovering in session 2 or 3):**

- **`0x00e04570` `vfunc_5` is the actual CME EventSignal invoke dispatch** (per W5-E pre-kill transcript). Called when an emit fires — invokes the stored member function pointer at `this+8` with the event args. Shared across 12 vtables (`xref_count=12`). W5-E flagged this as "the pattern W5-C should have been targeting."
- **`0x0157b0d0` RTTI vs annotation-name contradiction** (per W0 pre-kill transcript): RTTI says class `TimerExpiryHandler` but session-1 annotation script labeled this `Mercury_Nub_ReplyHandlerElement`. Another cyclic-shift-class bug. Worth a Mercury-specific sweep in session 3.
- **`0x01590fc0` is a large function** in the EntityDescription parse chain low half — body spans `0x01590fc0 → 0x01591213` (~600 bytes). Probably the master parse entry point or schema bootstrap. W4-B1 was mapping when killed.
- **EntityDescription stores `name_ → DataDescription` in a `std::map`** (red-black tree) — W4-B2 identified `_Rotate_left` / `_Erase_helper` patterns in the high-half parse chain. Useful for typing local variables in adjacent functions.
- **Entity dispatch helpers in W4-C's scope** use a common buffer pointer pattern named `puVar1` → properly `pEntityBuf` (a stack-allocated entity-data scratch buffer). At least 6 functions share this pattern; renaming them improves cross-function readability.
- **W5-B got 70 functions done** before kill, was on batch 15 (debug-command cluster: DebugBehaviorsOnMob, DebugPathsOnMob, DebugEvents, DebugPerformance, AbilityDebug). Those 70 plates are durable in Ghidra.

## Session 2 — W0 resume run (2026-05-13)

**W0_COMPLETE.** 290 functions / 1,237 tool calls / ~23 min wall-clock. Checkpoint at `worker-0.checkpoint.json`.

**Phase A — Struct/global verify:** all 5 structs (`StdStringMSVC`, `EntityDescription`, `DataDescription`, `MethodDescription`, `CmeEventSignalSubject`) and all 5 globals confirmed already present from prior killed-agent writes. Confirms per-call atomic writes survived the disconnect. `structs-ready-v2.flag` timestamp refreshed.

**Phase B — Mercury cyclic-shift sweep:** 6 more naming contradictions found and corrected (added to `findings/annotation-script-shift-bugs.md`):

| Address | Pre-W0 label | Corrected (RTTI-canonical) |
|---|---|---|
| `0x01579a50` | Mercury_Bundle_3 | `Mercury_Nub_dispatchExceptions` |
| (several Nub_NN) | Mercury_Nub_11/12/13/14/15 | decoded from debug strings |
| `0x01584920` | Mercury_Endpoint | `Mercury_Endpoint_findIndicatedInterface` |
| `0x015898c0` | Mercury_MachineGuard | `Mercury_MachineGuard_sendAndRecv` |
| `0x0157b0d0` | Mercury_Nub_ReplyHandlerElement | `Mercury_TimerExpiryHandler` (already in session-2 incident log) |

**Phase C1 — Mercury_* full V5:** 31 functions documented (29 new + 2 from prior session boundary). Architectural finds:
- **Mercury::Nub constructor fully mapped** — 24 initialization steps documented
- **InterfaceElement compress/expandLength encoding pair** documented — 1/2/3/4-byte width switch
- **Channel destructor SEH filter pair** identified and named
- **MachineGuard UDP port confirmed as `0x4e36` (19510)** — concrete wire-format fact, should propagate to `mercury-protocol-internals.md`

**Phase C2 — UBWNet/UBWConn:** all 7 functions processed. `0x006f0069` logged `skipped_decode_artefact` per plan.

**Phase C3 — register_NetOut_* combat/ability/stats/effects trimmed V5:** 27 stubs documented. All names already correct from prior annotation script — V5 plates applied with the corrected flat format (no spurious `[IMPLICIT ECX]`).

**Tally:** 256 full V5 + 33 trimmed V5 + 1 skipped = 290 entries. 4 contradictions logged.

## Session 2 — kill #2 incident (2026-05-13, MCP disconnect)

Ghidra MCP server disconnected mid-session. All 205 `mcp__ghidra__*` tools became unavailable. User requested all in-flight workers stopped to avoid burning tokens on failed MCP calls. 10 agents TaskStop'd (W0 + 9 fresh relaunches; W1-rescue replacement had already completed successfully). All pre-disconnect Ghidra writes are durable; checkpoints partial.

**State at pause:**

| Worker | What landed |
|---|---|
| W0 | Cyclic-shift sweep partial — found `0x0157b0d0` contradiction; struct flush + Phase A/B status unknown (no checkpoint flushed) |
| W1-rescue replacement (a8a5ceb0b319094e1) | **Complete — 47 rescored 85→100, full V5 plate-format issue documented above** |
| W4-A | Read ~40 functions in scope; no V5 writes started |
| W4-B1 | Mapping in progress on `0x01590fc0+` cluster; no V5 writes started |
| W4-B2 | Enumeration complete (97 in-scope); identification in progress; no V5 writes started |
| W4-C | Rename batch in progress (`puVar1 → pEntityBuf` across 6 helpers); some writes may have landed |
| W5-A | Mid-SGWTextCommandMgr SlashCmd batch; some writes landed |
| W5-B | ~70 functions complete (full V5 plates + renames durable); batch 15 in progress at kill |
| W5-C | VCommunicator cluster renamed (renames durable); plate writes had not yet started at kill |
| W5-D | Mid-batch on GiveXp/GiveItem cluster; some writes landed |
| W5-E | ~31 functions processed (likely durable); was continuing into Trainer/Inventory/Lootables/Trade clusters at kill |

**Lesson for session-3+ brief:**
- Orchestrator owns flag synchronization, not workers. Either:
  - (a) Launch workers ONLY after the flag is written by W0 (gated launch), OR
  - (b) Have workers do flag-wait via a single explicit `Bash` poll command (one-shot, exits when flag exists) — not a Read-loop, since Read-loops sometimes work, sometimes don't.

---

## Session 1 Totals

| Worker | Status | Processed | Tool calls | Notes |
|---|---|---|---|---|
| W0 | Partial | 243 / 580 enum | ~1,050 | Heavy scope-overlap with W1/W2. Mercury/UBWNet/combat still untouched. ContactList cyclic-shift fix. |
| W1 | Complete (mis-classified) | 58 / 58 | 112 | Trimmed V5 likely wrong; needs full V5 rescore. EmitNetOut + CME emit pipeline (5 new addrs). |
| W2 | Complete | 176 / 176 | 620 | TypedEmitInfo destructor finding. 4 new address-map clusters. BM + GiveInventory anomalies. |
| W3 | Complete | 204 + 4 / 208 | ~1,950 | Confirmed destructor pattern, ~78 score ceiling. No new addrs / structs / globals. |
| W4 | Partial | 234 / ~802 | ~1,975 | Resume `0x00de1c00`. Session-2 needs 3-way split (W4-A, W4-B, W4-C). |
| **Total** | | **915 entries processed** (~770 unique after overlap dedup) | **~5,707** | |

**Session 2 outstanding scope: ~960 functions**
- W0 primary: ~334 (Mercury_*, UBWNet*, BW_client*, NetOut combat/ability/stats/effects)
- W1 rescore: ~57 (full V5 destructor plates on the trimmed batch)
- W4-A: ~70 (continue 00de* sweep)
- W4-B: ~400 (EntityDescription parse chain)
- W4-C: ~110 (GameEntity / EntityManager / ABigWorldEntity)

---

## Cross-worker reconciliation note

W2 + W3 both confirm: every `TypedEmitInfo__vfunc_0` is an MSVC scalar destructor, every `CallbackImpl__vfunc_2` is an RTTI accessor. **W1's report of "57 stubs returning a name string" is almost certainly mis-classified** — W1 likely applied trimmed V5 to destructors. Those 57 functions probably have inflated completeness scores (~70s) but lack proper destructor-pattern plate comments.

**Consolidator action:** Re-process W1's 57 "trimmed" functions in session 2 with full V5 and the correct destructor plate. Add a `pending_w1_rescore` array to the consolidator's working notes. W0's session-2 first action (struct/global flush) is a natural time to also fix this.

---

---

## Session 3 — W-cleanup close-out (2026-05-13)

Five close-out tasks completed. All Ghidra writes durable.

### Task 1 — CmeMemberCallback struct

`CmeMemberCallback` struct created in Ghidra (12 bytes, 3 fields):
- `+0x00 void* pVtable`
- `+0x04 void* pSubscriber`
- `+0x08 void* pMethodPtr`

`set_local_variable_type` on `this` at `0x00e04570` failed (structural: ECX auto-param
not typeable via API — known Ghidra MCP limitation). Struct exists in the type database.
Completeness score on `CmeEventSignal_InvokeMemberCallback`: **83.87** (above 80 target).

### Task 2 — Pattern B emit sweep

`NetworkEvent_Ctor` (`0x004412e0`) has 200+ callers (xref_count confirmed via live MCP).
All callers in `0x00573d70–0x005b0e30` are typed ctors (one per event type), not emitters.
The emitters call those ctors. Exhaustive check of all ctor callers:

| Address | Name | Source |
|---------|------|--------|
| `0x00aea880` | `EmitNetOut_callForAid` | W-emit-A (already named) |
| `0x00aeab70` | `EmitNetOut_SetRingTransporterDestination` | W-emit-A (already named) |
| `0x00c8a830` | `SlashCmd_EmitSetRingTransporterDestination` | **NEW** — SGWTextCommandManager.cpp:0xC35 |

`0x00da4720` and `0x00da7eb0` are not recognized Ghidra functions (inline or data-aliased
call sites). Pattern B emitter sweep is exhaustive for the callers found.

`NetworkEvent_Ctor` (`0x004412e0`) renamed and plated.

### Task 3 — RegisterBulkNetOutSignals

`0x00db3390` renamed `RegisterBulkNetOutSignals` (was `register_NetOut_onStrikeTeamResponse`).
V5 plate applied documenting 38+ registered signals. Decompile timed out (3000-byte body);
signal list sourced from W-emit-B contradiction entry. Score: 45 (effective) — decompile
unavailability limits what the completeness analyzer can assess.

### Task 4 — Pipeline helper renames

| Address | Old name | New name | Plate |
|---------|----------|----------|-------|
| `0x00cb1f00` | `FUN_00cb1f00` | `CmeEventSignal_SetFieldHelper` | Applied |
| `0x00a5c150` | `FUN_00a5c150` | `CmeEventSignal_Subscribe` | Applied |

`CmeEventSignal_Subscribe` verified as subscription (not lookup): inserts callback into
subscriber set, returns bool (true=newly inserted). Body: `FUN_0158ea90` + count check.
Distinct from `CmeEventSignal_LookupByName` (0x00a5c0f0) confirmed.

### Task 5 — Globals flush

24 globals applied from all Session 1-2-3 worker checkpoints. See `globals-applied.json`
for the full list. One entry (`0x01f126b8`) used `rename_or_label` (no defined data).

### Doc updates

- `docs/reverse-engineering/findings/cme-event-signal.md` — added pipeline table rows
  for SetFieldHelper, Subscribe, InvokeMemberCallback; added Pattern B structural
  comparison table; added `CmeMemberCallback` struct layout section.
- `docs/reverse-engineering/address-map.md` — added 5 new addresses to CME EventSignal
  pipeline table (SetFieldHelper, Subscribe, InvokeMemberCallback, NetworkEvent_Ctor,
  RegisterBulkNetOutSignals).
- `docs/reverse-engineering/v5-campaign/globals-applied.json` — created.

---

## Session 5 — 2026-05-13 — W0-flush (Struct + Global Flush + Re-score)

### Scope

W0-flush read all named-worker checkpoints (worker-mission-state, worker-character-creation, worker-anim, worker-state, worker-abilities, worker-cover) for `pending_structs` and `pending_globals` arrays, then applied them to the Ghidra database and ran a re-score pass on all in-progress functions.

### Structs Created (18 total)

| Struct | Size | Source Worker | Notes |
|---|---|---|---|
| `SGWAnimSequenceListEntry` | 12 B | worker-anim | flags byte + FName idx + FName group; stride 0x0C |
| `SGWAnimTransitionEntry` | 20 B | worker-anim | char[3] weaponCode + char[3] postureCode + void* seqList + int maxCount; stride 0x14 |
| `SGWEntityCombatState_StanceCode` | 3 B | worker-anim | 3-byte overlay at entity+0x3D0: stance char[3] / substate byte / posture enum byte |
| `VisualChoiceEntry` | 36 B (0x24) | worker-character-creation | ChoiceId uint + 32-byte pad; stride 0x24 confirmed |
| `VisualGroupEntry` | 52 B (0x34) | worker-character-creation | VisGroupId + selectedIndex + choicesBegin/End ptrs; stride 0xD uint32s |
| `CoverNodePrefabData` | 24 B | worker-cover | pos(x,y,z) + orientation floats + coverHeight/quality/width bytes |
| `CoverInfo` | 20 B | worker-cover | vftable + pEventSubscriptionList + pSpatialTreeHandle |
| `AbilitySlot` | 160 B | worker-abilities | Partial: targetType@+0x48, TCM@+0x94, AERadius@+0xA0 |
| `CooldownEntry` | 12 B | worker-abilities | abilityId + startTime + endTime floats |
| `ClientEffectResult` | 10 B | worker-abilities | StatID byte + Delta float + DamageCode byte + StatResultCode byte |
| `CharacterInfo` | 192 B (0xC0) | worker-character-creation | 16 fields: playerId, name, extraName, worldLocation, alignment/level/gender/archetype bytes, playerType, playable, bodySet, components, primaryTint/secondaryTint/skinTint float[4] |
| `MissionSet` | 172 B | worker-mission-state | stepList/missionList/taskList/timerState ptrs + missionLoadState[4] |
| `MissionEntry` | 356 B | worker-mission-state | missionGiverName@+0x60, status@+0x64, status2@+0x100 (OQ-2 open) |
| `StepEntry` | 61 B | worker-mission-state | status byte@+0x3C |
| `ObjectiveEntry` | 53 B | worker-mission-state | status byte@+0x30, optionalFlag@+0x34 (OQ-3 open: hidden flag offset unknown) |
| `TaskEntry` | 8 B | worker-mission-state | status byte@+0x2, count int@+0x4 |
| `GameBeingAnimController` | 1100 B | worker-state | pPrimaryActor@+0x34C, pAnimSetTable@+0x3C0, keyBytes@+0x3D0..3D2, pAuxActors@+0x444, auxCount@+0x448 |
| `SGWPawnActor` | 952 B | worker-state | pos@+0xDC/E0/E4, locomotionFlags@+0x1E4, MaxSpeed@+0x25C, pAnimSetSlot@+0x2F8, actorFlags@+0x380, targets@+0x3AC/3B0, deadFlag@+0x3B4 |

**Skipped:** `StdStringMSVC` — already existed (created in a prior W0 session).

### Globals Applied (6 new)

| Address | Name | Source | Method |
|---|---|---|---|
| `0x01816aa0` | `g_ZeroDouble` | worker-anim | rename_or_label |
| `0x01e0c458` | `g_AbilityCooldown_UISubscriber` | worker-abilities | rename_or_label |
| `0x00c71790` | `GameBeing_GetMovementSpeedTable` | worker-state | rename_function_by_address |
| `0x019ACEF4` | `g_szMinigameInstanceField2Name` | W1 session 1 | rename_or_label (idempotent re-apply) |
| `0x019ACEE8` | `g_szMinigameInstanceField1AltName` | W1 session 1 | rename_or_label (idempotent re-apply) |
| `0x019ACF38` | `g_szMinigameInstanceField3Name` | W1 session 1 | rename_or_label (idempotent re-apply) |

### Re-score Pass Results

| Function | Before | After | Delta | Crossed 60? |
|---|---|---|---|---|
| `MissionSet_FireUiEvent` (00d163e0) | 54.9 | 61.7 | +6.8 | **YES** |
| `MissionSet_FindMissionById` (00d16800) | 44.4 | 51.2 | +6.8 | no |
| `MissionSet_PropagateMissionUpdate` (00d16dd0) | 41.1 | 47.9 | +6.8 | no |
| `MissionSet_HandleOnStepUpdate` (00d18cf0) | 23.0 | 27.4 | +4.4 | no |
| `MissionSet_HandleOnObjectiveUpdate` (00d18fd0) | 23.0 | 27.3 | +4.3 | no |
| `MissionSet_HandleOnTaskUpdate` (00d194b0) | 33.1 | 37.4 | +4.3 | no |
| `MissionSet_HandleOnMissionUpdate` (00d1a270) | 33.1 | 37.4 | +4.3 | no |
| `MissionSet_HandleMissionRewards` (00d1a500) | 33.1 | 37.4 | +4.3 | no |
| `GameBeing_UpdateCombatStanceWeaponSet` (00e7b4c0) | 42.0 | 47.2 | +5.2 | no |
| `GameBeing_UpdateMovementSpeed` (00dfff70) | 22.0 | 22.4 | +0.4 | no |
| `GameAccount_HandleNetIn_CharacterList` (00e74060) | 32.0 | 31.9 | −0.1 | no |
| `GameBeing_OnStateFieldUpdate` (00e01c90) | 34.0 | 32.2 | −1.8 | no |

**Analysis of residual gap:** The dominant deduction across all functions that did not cross 60 is `undefined_count` (40–87 undefined SEH stack frame locals). Creating the structs resolved the `local_type_quality` deduction (~3.9 pts per function) but the SEH locals require a full per-function V5 decompile+audit pass to address. All structs listed above are now in the Ghidra database and ready for a deeper typing session.

**Checkpoint:** `worker-0-flush.checkpoint.json`

---

## Consolidator notes (for Task #10)

When the campaign winds down:
- Update `address-map.md` with the 5 CME emit pipeline addresses (W1) plus whatever W0/W2/W3/W4 add.
- Update `STATUS.md` with the V5 docs phase result and the corrected scope numbers.
- Consider authoring `docs/reverse-engineering/findings/cme-event-signal.md` if it doesn't already exist — the 4 callee addresses above form a coherent pattern worth a dedicated findings doc.
- The brief at `WORKER_BRIEF.md` should be amended to reference `register_NetOut_` / `register_NetIn_` prefixes for any session 2+.
- **Pattern B emitter count confirmed: 3 emitters (2 primary + 1 slash-cmd wrapper). Not 5-10 as the brief estimated — the 200+ xrefs to NetworkEvent_Ctor are typed ctors, not emitters.**
- **`CmeMemberCallback` struct is in root category (not `/CmeEventSignal`) — `create_data_type_category` returned "Transaction not started" error. Move to `/CmeEventSignal` category in a future Ghidra UI session.**

---

## Session 5 — W-cooked (Cooked Data Pipeline, 2026-05-13)

### Scope

Verified and documented the full cooked data pipeline client-side architecture. Traced PAK open path,
ZipStorage, version negotiation, and CME event subscriptions per category.

### Functions Documented (25 total)

| Address | Name | V5 Level |
|---------|------|----------|
| `0x00420074` | `CookedData_RegisterAllLibCategories` | Renamed + Plated |
| `0x004786c0` | `LibCategoryBase_Ctor` | Renamed + Plated + Prototype |
| `0x004786f0` | `CacheLibrary_GetSingleton` | Renamed + Plated + Prototype |
| `0x00478840` | `CacheLibrary_Ctor` | Renamed + Plated |
| `0x00437650` | `CacheLibrary_RegisterCategory` | Renamed + Plated |
| `0x0044c800` | `LibCategory_ServerSource_Ctor_cat1_KismetSeqEvent` | Renamed + Plated |
| `0x004267f0` | `CME_MemberCallback_Ctor_ServerSource_NetConnected` | Renamed + Plated |
| `0x004268f0` | `CME_MemberCallback_Ctor_ServerSource_NetDisconnected` | Renamed |
| `0x00426970` | `CME_MemberCallback_Ctor_ServerSource_onVersionInfo` | Renamed |
| `0x004269f0` | `CME_MemberCallback_Ctor_ServerSource_NetProxyData` | Renamed |
| `0x00426a70` | `CME_MemberCallback_Ctor_ServerSource_onCookedDataError` | Renamed |
| `0x0042a7b0` | `CME_Subscribe_ServerSource_NetConnected` | Renamed |
| `0x0042a840` | `CME_Subscribe_ServerSource_NetDisconnected` | Renamed |
| `0x0042a8d0` | `CME_Subscribe_ServerSource_onVersionInfo` | Renamed |
| `0x0042a960` | `CME_Subscribe_ServerSource_NetProxyData` | Renamed |
| `0x0042a9f0` | `CME_Subscribe_ServerSource_onCookedDataError` | Renamed |
| `0x00a37790` | `CME_EventSignal_Subscribe` | Renamed + Plated |
| `0x00441630` | `ServerSource_onVersionInfo_Handler_cat6` | Renamed + Plated |
| `0x00441aa0` | `ServerSource_onCookedDataError_Handler_cat6` | Renamed + Plated |
| `0x00479340` | `ZipStorageBase_OpenArchive` | Renamed + Plated |
| `0x00479930` | `ZipStorageBase_WriteStreamToFile` | Renamed |
| `0x00479e10` | `ZipStorageBase_WriteMetaDataVersion` | Renamed + Plated |
| `0x00479e90` | `ServerSource_SetVersion` | Renamed + Plated |
| `0x0043bdb0` | `ServerSource_RequestElement` | Renamed + Plated |
| `0x013a1620` | `CZipStorage_Dtor` | Renamed + Plated |

### Key Findings

1. **Category count correction (HIGH confidence):** The client registers exactly 21 ServerSource
   categories (IDs 1–21). The existing `docs/engine/cooked-data-pipeline.md` table lists 22 (0–22).
   Category 0 does NOT exist on the client. All category integers confirmed from binary template
   parameters in `CookedData_RegisterAllLibCategories` @ `0x00420074`.

2. **Category 21 name mismatch:** Binary shows category 21 = `BehaviorEventData` /
   `CookedBehaviorEvents.pak`. Server `resource.cpp` lists category 21 = `pet_command`,
   category 22 = `behavior_event`. This numbering drift may cause the server to send category 22
   data the client will silently ignore.

3. **Five CME events per category:** Each `LibCategory<LibCategoryKey<N,...>>` ctor wires
   `Event_Net_Connected`, `Event_Net_Disconnected`, `Event_NetIn_onVersionInfo`,
   `Event_Net_ProxyData`, `Event_NetIn_onCookedDataError` subscriptions.

4. **Version stamp persisted to PAK MetaData:** After receiving `onVersionInfo`, the client writes
   the server's version uint32 to the PAK's `MetaData` ZIP entry via `ZipStorageBase_WriteMetaDataVersion`.
   This enables session-to-session cache reuse — client skips requesting elements if PAK MetaData
   matches current server version.

5. **`covernodes_local.pak` is a `ClientSource`:** Not in the 1–21 ServerSource list, no category ID,
   loaded locally.

### Docs Updated

- `docs/reverse-engineering/findings/cooked-data-pipeline.md` — CREATED (new findings doc)
- `docs/reverse-engineering/findings/README.md` — new row added
- `docs/reverse-engineering/address-map.md` — cooked-data section appended (26 addresses)
- `docs/reverse-engineering/v5-campaign/worker-cooked.checkpoint.json` — CREATED

### Docs NOT Updated (scope restriction)

- `docs/engine/cooked-data-pipeline.md` — out of W-cooked scope per WORKER_BRIEF. Requires separate
  correction session. The category ID table must be changed from 0–22 to 1–21 with corrected high-end names.
- `docs/engine/cooked-data-pak-format.md` — out of scope. Integer category IDs should be added.

### Checkpoint

`worker-cooked.checkpoint.json`

### Open for Follow-up

- `LAB_0043dad0` — `Event_Net_ProxyData` fragment reassembly handler (address is a label, not a
  function symbol; may need `create_function` before decompile)
- `FUN_0047a690` — `InvalidateAll` element cache flush
- `FUN_0043a9d0` — cache-miss predicate in `ServerSource_RequestElement`
- Server-side audit: does `resource.cpp` send category IDs 1–21 or 0–21?
