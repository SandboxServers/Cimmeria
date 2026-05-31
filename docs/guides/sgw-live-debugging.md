---
title: "Live debugging SGW.exe with x32dbg + Ghidra MCP"
type: how-to
audience: contributors doing RE
last_updated: 2026-05-27
---

# Live debugging SGW.exe with x32dbg + Ghidra MCP

> **Type**: How-to guide
> **Audience**: Cimmeria reverse-engineers and server developers needing to confirm a client-side hypothesis against the live process.
> **Prerequisites**: Ghidra with SGW.exe loaded and the annotation scripts run (at minimum 01_rtti and 04_event_signal). x32dbg (use `x32dbg.exe` — SGW is 32-bit). Optional: Ghidra MCP plugin for HTTP-driven static lookups.
> **Setup**: If you haven't wired Ghidra MCP or x64dbg-automate MCP yet, do that first via [`re-toolchain-setup.md`](re-toolchain-setup.md). For the overall RE workflow (when to invoke the `game-archaeology-specialist` agent, what to verify yourself, what to hand off), see [`reverse-engineering-with-claude.md`](reverse-engineering-with-claude.md).

This guide captures what we learned hunting the [right-click routing on corpse bug](../reverse-engineering/findings/right-click-routing-on-corpse.md) — specifically, the techniques that worked, the dead ends, and the gotchas. Follow this when static analysis isn't enough and you need to confirm runtime state.

## When live debugging is worth it

Live debugging is expensive — every breakpoint hit costs a possible server disconnect (the heartbeat thread can't run while the process is paused). Reach for it only when:

- Static analysis has narrowed the question to a few alternatives, and any of them is testable in one or two breakpoints.
- You're trying to disprove a hypothesis as fast as confirming one.
- The same data could not be obtained from server-side logging.

## Toolchain choice

We tried two debuggers; here's what worked.

### x32dbg — recommended

Use `x32dbg.exe` (the 32-bit build). SGW.exe is 32-bit, the regular `x64dbg` won't attach to it. x32dbg reliably attaches, sets breakpoints, supports log breakpoints, resumes cleanly, and detaches without crashing the game.

### pybag (the `ghidra-mcp` debugger plugin) — avoid for SGW

pybag's HTTP-driven debugger is convenient on paper but has two problems specifically with SGW:

1. **Attach freezes the game even with no breakpoints set.** Some interaction with SGW's main thread state prevents the resume from actually progressing. Status shows `running` but the game window is stuck.
2. **`go` after `interrupt` fails with `E_ACCESSDENIED` (HRESULT 0x80070005).** Once you've interrupted to inspect state, the only way back to a running process is to fully detach. There's no way to step or resume.

We spent significant time on pybag before concluding it's incompatible. If you must script-drive a debugger, prefer Ghidra's built-in debugger over pybag.

## Address mapping (Ghidra static address → runtime)

SGW.exe's preferred image base in the PE header is `0x00400000`, but Windows ASLR will load it elsewhere. To translate any Ghidra address to a runtime address:

1. Attach x32dbg to SGW.
2. Click the **Memory Map** tab.
3. Find the row whose `Info` column says simply `sgw.exe` (this is the PE header; subsequent rows are `.text`, `.rdata`, etc.).
4. Note the `Address` column — this is the runtime base. In our experience it has consistently been `0x00960000`, giving an ASLR slide of `+0x00560000`.
5. **Runtime VA** = `Ghidra_VA - 0x00400000 + runtime_base`.

Example: `FUN_00e84b20` (Ghidra) at base `0x00960000` resolves to runtime `0x013e4b20`.

## Setting breakpoints

In the bottom command bar:

```
bp 013E4B20
```

Verify in the **Breakpoints** tab — the disassembly column should show a sensible function prologue (`push ebp` / `cmp dword ptr` / similar). If you see something like `inc ebx`, you're at the wrong address — most often because you typed the Ghidra address as if it were an RVA. Recompute and retry.

## Avoiding heartbeat disconnect: log breakpoints

Halting on every right-click click hangs the game long enough to lose the server heartbeat. The fix is **log breakpoints** that capture state on each hit and resume immediately without pausing the UI thread.

Right-click the breakpoint row in the Breakpoints tab → `Edit`:

| Field | Value | Effect |
|---|---|---|
| **Log Text** | `[my-marker] target=0x{x:esi} pawn_field=0x{x:[eax+1B4]} eax=0x{x:eax} edi=0x{x:edi}` | Format string written to the Log tab on each hit |
| **Break Condition** | `0` | Critical — makes the breakpoint log-only, never halts the process |
| **Log Condition** | (blank or `1`) | Always log |

The format placeholders are:
- `{x:reg}` — register value as hex (e.g. `{x:esi}`)
- `{x:[expr]}` — 4-byte memory dereference as hex (e.g. `{x:[eax+1B4]}` reads `*(uint32_t*)(eax + 0x1B4)`)
- Nested derefs work: `{x:[[edi+1B0]+1B4]}` is `*(uint32_t*)(*(uint32_t*)(edi + 0x1B0) + 0x1B4)`.

Read the log via the **Log** tab. To filter by your prefix, right-click → Find.

### Pitfalls with log expressions

- **Hex offsets must be unambiguous.** `[eax+C]` is parsed correctly as `eax + 0xC` in our experience, but if you see suspicious `???` values, switch to `[eax+0xC]` explicitly.
- **`???`** in a log line means the address was unreadable. Most often you're dereferencing a register that doesn't hold the pointer you think — re-read the disassembly to confirm what each register holds at that exact instruction.
- **Get registers right for the BP location**. The same logical value (e.g. "the target pointer") may live in different registers at different addresses inside a function. Pick the BP address based on what registers are alive there.

## Hot functions = freezes; pick BP locations carefully

Some functions are called every frame (cursor target tracking, animation ticks). Setting a breakpoint there freezes the game even briefly, which is enough to disconnect.

What we learned the hard way:

| Function | Hot? | Why |
|---|---|---|
| `FUN_00e85860` (click router / MouseLook handler) | **Yes** | Fires on every RMB press AND release event |
| `FUN_00e68570` (gate predicate) | **Yes** | Cursor target tracking calls it on every hover |
| `FUN_00e84860` (resolver) | **Yes** | Cursor logic re-resolves frequently |
| `FUN_00e84b20` (interact firer) | **No** | Only called from the click router on actual RMB release with valid pick. Safe to halt-break here. |

Rule of thumb: **prefer breakpoints on functions that have a single specific user trigger**. If a function gets called by the cursor system or any per-frame tick, it's hot.

## A working investigation flow

For the corpse-loot bug investigation specifically:

1. **Static phase**: walk callers from `Event_NetOut_Interact` constructor (`FUN_00d97990`) backward to find the firer `FUN_00def4b0`, then to its only caller `FUN_00e84b20`, then up to `FUN_00e85860` (the click router). This gives you a labeled call graph without touching the live process.
2. **Bridge phase** (Ghidra annotation scripts): run `01_rtti_annotator.py` and `04_event_signal_annotator.py` first. These two alone get you 8000+ vtables labeled and ~422 `register_NetOut_*` functions named. `07_vtable_annotator.py` is comprehensive but takes 30+ minutes — only run it if you need vfunc names beyond vfunc_0.
3. **Targeted annotation**: if 07 is too slow, write a trimmed version that only annotates the classes you care about (see `docs/reverse-engineering/annotation-scripts/07b_targeted_vtable_annotator.py`).
4. **Live phase**: log breakpoint at the entry of the function whose runtime behavior you're trying to confirm. Format the log to capture the registers + memory you need. Right-click in-game a few times. Read the log.

## Recovering from a stuck debugger session

If pybag (or anything else) leaves the game frozen:

1. Detach: `POST /debugger/detach` (or click Detach in x32dbg). Game resumes.
2. If Detach hangs too: kill the debugger process. The OS detaches, game survives.
3. Worst case: kill SGW.exe and re-launch. You lose your in-game position, but the server is fine.

## See also

- [right-click-routing-on-corpse.md](../reverse-engineering/findings/right-click-routing-on-corpse.md) — the investigation this guide came out of, including the resolution that this technique uncovered.
- [docs/reverse-engineering/annotation-scripts/](../reverse-engineering/annotation-scripts/) — the Jython scripts that produce the labels you'll see in x32dbg's disassembly.
- [docs/reverse-engineering/findings/entity-property-sync.md](../reverse-engineering/findings/entity-property-sync.md) — wire format for property changes if you're tracing those.
