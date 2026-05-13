# V5 Documentation Campaign — Shared Worker Brief

**Read this first.** Every worker (W0–W4) must follow this brief. Per-worker prompts only add scope-specific overrides.

## Scope boundary — what workers MUST NOT modify

Workers in this campaign may modify:
- `docs/reverse-engineering/findings/` (new + existing findings docs)
- `docs/reverse-engineering/address-map.md`
- `docs/reverse-engineering/v5-campaign/` (checkpoints, this brief, campaign status)
- `docs/reverse-engineering/findings/README.md` index
- The Ghidra database (renames, plates, prototypes, structs/globals per ownership rules)
- Agent-memory files for the agent persona running the worker

Workers **MUST NOT** modify:
- `crates/**/*.rs` — any Rust source code in the project's crates. Bug fixes go in the findings doc as "Recommended Rust fix" sections with proposed diffs (citing module path + type/method names, no line numbers per bible convention). The orchestrator or a human applies fixes in a separate code-change session.
- `entities/defs/*.def` — entity contracts. These are frozen.
- `db/**` — schema or seed data.
- `game/sgw/**` — client tree.
- `deprecated/**` — legacy server tree.
- Any other source-of-truth file outside `docs/reverse-engineering/`.

If a worker discovers a bug it could fix in code, **document it; do not apply it.** The findings doc should include the proposed Rust diff at a granularity the human can verify in 5 minutes.

## Mission

Apply the **V5 Function Documentation Workflow** to ~2,300 server-relevant functions in `SGW.exe`, currently loaded in Ghidra and reachable via the Ghidra MCP at `http://127.0.0.1:8089`. All worker tools are prefixed `mcp__ghidra__*` and must be loaded with `ToolSearch select:<name>` before first use (they are deferred).

The end state is a Ghidra database where every in-scope function has:
- A V5-compliant name (PascalCase with verb, module prefix accepted; auto-validated by `NamingConventions.java`).
- An accurate function prototype.
- Typed local variables with Hungarian notation (auto-prefixed by the tools on struct fields).
- A plate comment with **Algorithm / Parameters / Returns** sections.
- Instruction-level comments where they aid understanding (loops, branches, magic numbers).
- A `analyze_function_completeness` score ≥ 60 (target 80+ where structurally possible).

## Inputs you must read before starting

1. `C:\Users\steven.cady\repos\personal\ghidra-mcp\docs\prompts\FUNCTION_DOC_WORKFLOW_V5.md` — **canonical V5 spec, 7 steps. The rest of this brief defers to it.**
2. `C:\Users\steven.cady\repos\personal\ghidra-mcp\docs\prompts\TOOL_USAGE_GUIDE.md` — MCP tool selection patterns.
3. `C:\Users\steven.cady\repos\personal\ghidra-mcp\docs\prompts\STRING_LABELING_CONVENTION.md` and `DATA_TYPE_INVESTIGATION_WORKFLOW.md` — convention enforcement.
4. `C:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\STATUS.md` — what's already been done.
5. `C:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\address-map.md` — key addresses and their roles. Treat as authoritative.
6. The findings doc(s) relevant to your scope under `C:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\findings\`. Per-worker prompt lists yours.
7. BigWorld engine reference: `C:\Users\steven.cady\repos\personal\Cimmeria\external\BigWorld-1.9.1\` and `BigWorld-2.0.1\`. Use when you need a struct layout or function semantics that aren't in our findings docs.

## Per-function workflow (full V5)

For each function in your scope, **in ascending address order**:

1. **Skip check.** Call `mcp__ghidra__analyze_function_completeness` (or `analyze_for_documentation` if you also want xref context). If `effective_score >= 80`, log `status: skipped_already_done` in your checkpoint and move on.

2. **Skip check 2.** If `code_line_count <= 8` AND function has no xrefs in: log `status: skipped_too_small`, move on. (Inline stubs / padding don't justify full V5.)

3. **Decompile + read.** Use `decompile_function`. Cross-reference the decompiled output against your scope's findings doc(s). Note any contradiction with existing findings — log it; don't silently override.

4. **Rename + prototype (Step 2 of V5).** Apply `rename_function_by_address` then `set_function_prototype`. Order matters: prototype after rename. Prototype goes BEFORE plate comment (V5 rule — prototype changes wipe plate comments otherwise).

5. **Type locals (Step 3 of V5).** `get_function_variables`, then up to 3 targeted `set_local_variable_type` for the highest-value variables (typically the `this` pointer's pseudo-struct, return-value-receiving locals, and frequently-dereferenced pointer params). Then `rename_variables` for Hungarian-prefix renames in one batch.

6. **Plate comment + instruction comments (Step 4).** Use `batch_set_comments` with the plate comment + any PRE/EOL inline comments in one call. Plate structure:
   ```
   Algorithm:
     <prose description of what the function does, in order>
   
   Parameters:
     <param 1>: <type> — <semantic role>
     <param 2>: <type> — <semantic role>
     [IMPLICIT ECX: <ClassName>*]   ← for __thiscall when not retypable
   
   Returns:
     <type> — <semantic meaning>
   
   Special Cases (optional):
     <edge cases, error paths, magic numbers>
   ```
   Always use literal multi-line strings; `\n` in the plate text becomes literal `\n` in Ghidra, not a newline.

7. **Verify (Step 5).** Call `analyze_function_completeness` again. If `effective_score < 60` AND `fixable_deductions > 10`, do one targeted correction pass. If after that score is still <60, accept and log `notes: "residual fixable deductions: N — accepted after 2 passes"`. **Do not loop indefinitely.**

8. **Checkpoint.** Every 50 functions processed (any status), flush your checkpoint to disk (see schema below). On error or termination, the checkpoint is your resume point.

## Trimmed workflow (stubs and accessors)

**WARNING — pre-session-1 guidance was wrong.** `TypedEmitInfo__vfunc_0` is the **MSVC scalar destructor** (`~TypedEmitInfo()`), NOT a name-string accessor. **Full V5 applies.** Confirmed by W2 + W3 across 363 functions in V5 Documentation Campaign session 1 (2026-05-12). The body shape is: call per-event cleanup, then conditionally `scalable_free(pThis)` if `bDeallocate & 1`. The structural score ceiling is ~78 because `void* this` in MSVC `__thiscall` cannot be retyped via the MCP API; accept that, don't fight it.

**Same warning for `CallbackImpl__vfunc_2`** — it is the **RTTI type-name accessor**, returning a compile-time `TypeDescriptor*` pointer, NOT a name string. **Full V5 applies.** Confirmed across 17 CallbackImpl functions by W3.

See [`../findings/cme-event-signal.md`](../findings/cme-event-signal.md) for the full pipeline + class anatomy that drives this correction. W1's 57 trimmed-V5 functions from session 1 need rescoring with full V5 destructor plates in session 2.

**The actual stub/accessor pattern qualifying for trimmed V5 is narrower than the pre-session-1 brief described.** Decompile and inspect before applying trimmed V5. **If you see `scalable_free(pThis)` or a vtable dispatch in the body, it's full V5.** Only true single-`return <constant>` bodies (≤5 lines, no branches, no destructor pattern, no vtable dispatch) qualify for the trimmed path.

For functions that genuinely qualify after decompilation (single `return <constant>`, ≤5 lines, no branches, no `scalable_free`, no vtable dispatch), apply only:

- Step 1 (skip check)
- Step 2 (rename + prototype)
- Step 5 (verify)

Skip plate comment and local typing. Budget ~4 tool calls instead of ~13.

## Naming conventions

### `MemberCallbackRtti_` prefix (Session 3 correction)

Functions of the form `MemberCallbackRtti_<Event>__<Subscriber>` are **RTTI type-name accessors** for
`MemberCallback<EventType, SubscriberType>` template instantiations — they implement vtable slot 2
(`_MemberCallback__vfunc_2`) and return a compile-time `TypeDescriptor*` pointer. They do **not** handle
events. Prior sessions named these `OnEvent_<Event>__<Subscriber>`, which incorrectly implied they were
event handlers. Session 3 corrected all 489 functions in scope.

Key distinctions:
- **`MemberCallbackRtti_*`** (slot 2) — returns `TypeDescriptor*` for the bound subscriber class. Pure RTTI.
- **`CmeEventSignal_InvokeMemberCallback` (`0x00e04570`)** (slot 5) — the actual dispatch body. Loads the
  bound method pointer from `MemberCallback+0x8` and the subscriber object from `MemberCallback+0x4`, then
  calls into the handler. This one function is shared across all 10 vtable instantiations.
- **The handler itself** — the concrete per-event method on the subscriber class (e.g., a method on
  `SGWNetworkManager`), stored at `MemberCallback+0x8` and called by slot 5.

NamingConventions.java issues advisory warnings for this prefix (not a recognized verb, contains underscores)
but does **not** reject the rename. All 489 renames succeeded. Do not treat these warnings as failures.

Reference: [`../findings/cme-event-signal.md`](../findings/cme-event-signal.md) — full MemberCallback
object layout, vtable slot table, and pipeline diagram.

## Shared-state invariants

- **Struct creation (`create_struct`, `add_struct_field`, `modify_struct_field`):** Only W0 calls these. Other workers, when they discover a struct need, log it to their checkpoint's `pending_structs` array (schema below) and **continue without creating the struct**. They can still reference fields by offset in plate comments.
- **Global renames (`rename_data`, `rename_or_label` for `DAT_*`):** Only W0. Others log to `pending_globals`.
- **All per-function writes** (`rename_function_by_address`, `set_function_prototype`, `set_local_variable_type`, `rename_variables`, `batch_set_comments`): **partitioned by function address — your scope only.** Never touch a function outside your scope.

## Coordination flag

- W0's **very first action** is to write `C:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\v5-campaign\structs-ready.flag` containing a single ISO-8601 timestamp. This signals to W1–W4 that struct/global ownership is established and they can begin.
- W1–W4 should **Read** the flag file as their first action. If it does not exist, they wait/retry (re-Read every 30 seconds, max 10 retries). If it never appears, they may begin their own work but **must NOT call `create_struct` / `add_struct_field` / `modify_struct_field` / `rename_data`** — those stay W0's exclusive ownership.
- W0 returns to flush pending structs/globals at the start of each subsequent session (Read all other checkpoints' `pending_structs` / `pending_globals` arrays, create what's needed, then proceed).

## Checkpoint schema

Write to `C:\Users\steven.cady\repos\personal\Cimmeria\docs\reverse-engineering\v5-campaign\worker-{N}.checkpoint.json` every 50 functions.

```json
{
  "worker_id": 0,
  "campaign_version": "v5",
  "created_at": "2026-05-12T17:20:00Z",
  "last_updated_at": "2026-05-12T18:14:00Z",
  "scope_predicate": "<your scope as a sentence>",
  "total_in_scope": 350,
  "functions": [
    {
      "address": "00c6fc40",
      "name_at_start": "FUN_00c6fc40",
      "name_at_end": "DispatchEntityMethodRpc",
      "status": "complete | skipped_already_done | skipped_too_small | in_progress | error",
      "score_before": 0,
      "score_after": 71,
      "workflow": "full | trimmed",
      "skipped_reason": null,
      "tool_calls_used": 14,
      "notes": "free text — contradictions with findings docs, residual deductions, etc."
    }
  ],
  "pending_structs": [
    {
      "requested_by": "00dd6a60",
      "struct_name": "ServerConnectionState",
      "observed_offsets": ["+0x10 size 4", "+0x14 size 4"],
      "evidence": "ServerConnection_startEntityMessage accesses pConn+0x10 (pBundle) and pConn+0x14 (nSeqNum)"
    }
  ],
  "pending_globals": [
    {
      "requested_by": "00dd6a60",
      "address": "01b8a0e8",
      "proposed_name": "g_pServerConn",
      "evidence": "Globally read by ServerConnection_startEntityMessage; sole writer is g_ClientApp_init at 0x00dd5000"
    }
  ],
  "stats": {
    "complete": 47,
    "skipped_already_done": 3,
    "skipped_too_small": 8,
    "in_progress": 1,
    "error": 0,
    "not_started": 291,
    "total_tool_calls": 612
  },
  "resume_from_address": "00c6fc41"
}
```

`resume_from_address` is the **next** address to process on resume (one past the last completed address in ascending order). On resume, Read the checkpoint, build a set of completed/skipped addresses, and process the next unprocessed address ≥ `resume_from_address`.

## Constraints you must respect

- **Read-only escape valves only.** If something feels wrong (the binary doesn't match what the findings doc says, or `analyze_function_completeness` returns inconsistent scores between two calls), STOP writes for that function, log to `notes`, and continue with the next address. Do not improvise corrections.
- **Never run a Ghidra script.** Do not call `mcp__ghidra__run_ghidra_script` or `mcp__ghidra__run_script_inline`. The user reports those freeze the machine.
- **Never re-run the 10 annotation scripts** under `docs/reverse-engineering/annotation-scripts/`. They have already been run; the current naming is the result. Read them only if you need to understand a name's provenance.
- **Don't try to update `STATUS.md`, `function-naming-progress.md`, or `address-map.md` mid-run.** Those are the consolidator's job (a follow-up step after the campaign).
- **Token discipline.** Prefer batch tools (`batch_decompile`, `batch_set_comments`, `get_bulk_function_hashes`, `get_bulk_xrefs`) over per-function loops. Cap `batch_decompile` at 50 functions/call.
- **No re-kb pushes.** `RE_KB_ARCHIVE_URL` is set empty in `.mcp.json`; the plugin will skip the push, but if you see `archive_ingest_function` in the tool list, **don't call it**.

## Output

Your final message back to the orchestrator must include:
1. Headline (functions completed / in scope / total tool calls used).
2. Checkpoint file path (so the orchestrator can read it).
3. Any **contradictions** with existing findings docs — list them. These are the most valuable artifact from this campaign.
4. Any **new addresses worth adding to `address-map.md`** — list with one-line justification.
5. Suggested follow-ups for the consolidator (struct names not yet created, globals not yet renamed, etc.).
