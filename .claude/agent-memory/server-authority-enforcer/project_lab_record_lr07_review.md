---
name: project-lab-record-lr07-review
description: LR-07 lab_record_* daemon tools trust review (2026-10-11): lease/takeover/camera-cache shape cleared; open-stop, specs_dir path and unbounded section residuals
metadata:
  type: project
---

LR-07 (lab-record, range 5d95addf1..db514e70f) reviewed 2026-10-11. Cleared shape:
- Recorder task runs inside `permit::scope` with its own RunLease; every client read (camera_read, ui_state, pump_events) goes through `Supervisor::bridge_call`, which calls `permit::ensure`, and nothing in the sampler path spawns a sub-task (a spawn would drop the task-local and `ensure` passes outside any scope — check this on any future change).
- `RunLease` drop releases only if the id is still current (`LeaseBook::release_at`), so a stop never frees a taker's lease.
- Character name gated by `check_character` both in start and in `Sampler::begin`; snapshot SQL keyed by integer player_id. Not bound to the lab account (any sgw_player name), but the server MCP already exposes taps/queries for any session.
- Camera cache survives client restarts but is re-checked by rebased vtable and DLL reads are VirtualQuery-guarded.
- rmcp 3.5 does not drop a handler future on cancel, so start cannot be orphaned between spawn and arm.

Residuals reported: stop is OPEN and `take_if`'s refusal echoes the live id; `specs_dir` accepts any absolute/UNC path (same as lab_golden_record/lab_uat_run precedent) and is only used at stop; `section` and `character_expr` unbounded/unvalidated.

**Why:** Lab daemon is loopback-only, single operator; the threat is a confused or prompt-injected local agent, not a remote attacker.
**How to apply:** For LR-08+ (CLI) and any later record change, re-check the no-spawn rule under permit::scope and whether stop gains a lease/operator gate. See [[reference-auth-handshake-layers]] for the general lease idiom.
