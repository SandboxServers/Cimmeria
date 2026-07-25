---
name: feedback-x64dbg-session-liveness-check
description: Always verify SGW.exe process is alive and running before any cave writes in x64dbg — second-chance AV crashes pre-exist silently
metadata:
  type: feedback
---

# x64dbg Session Liveness Protocol (2026-06-22)

**Rule:** Before writing ANY cave code or patches in a new x64dbg session, run a mandatory two-step liveness check. Do NOT assume the process is healthy just because `list_sessions` shows a PID.

**Why:** On 2026-06-22, the target process (SGW.exe PID 2014556) was already in a second-chance AV state (EIP=`0x0FF51000`, unrecoverable) when the agent connected. The prior session had crashed (EIP jumped past the end of a 4096-byte `VirtualAlloc` allocation at `0x0FF50000`). The agent wrote all cave code into the dead process, verified disassembly (which worked — pages were still mapped), then tried to run and immediately hit the pre-existing exception. All work was wasted.

**The two-step check:**
1. `get_debugger_status` → verify `Running: True`. If `Running: False`, call `run` once, re-check. If still False, the process is likely stopped at an exception.
2. `get_latest_event` → verify result is NOT `EVENT_EXCEPTION` with `dwFirstChance: False` (second-chance = unrecoverable). If it IS a second-chance AV, STOP immediately.

**If either check fails:** Terminate the dead session, have the user restart SGW.exe fresh, wait for re-attach confirmation before proceeding.

**How to apply:** Add this as the first two tool calls of every new x64dbg session, before any memory reads, writes, or BP installs.

**Related:** [[feedback_x64dbg_nonfreezing_breakpoints]] — also about x64dbg session hygiene.

## Session-specific crash analysis (2026-06-22)

The crash at `0x0FF51000` (one page past `VirtualAlloc` base `0x0FF50000`) was caused by stale cave code from the PRIOR session. The prior session's `VirtualAlloc` returned `0x0FF50000`; the prior cave had a displacement error that computed a jump target of `0x0FF51000` (unmapped). The process crashed, was not fully terminated, and PID 2014556 survived in x32dbg in a dead state. Next session's agent connected to the same PID without checking liveness.

**Note on VirtualAlloc address reuse:** When SGW.exe restarts, the heap allocator may return the SAME base address as the prior session. This means stale cave bytes from a partially-applied prior session can survive and cause the next session's fresh VirtualAlloc to return a page that already contains corrupted code. The liveness check catches this; additionally, zero-fill the first 256 bytes of any new allocation before writing cave code to clear stale bytes.
