---
name: reference_dw03s_h_2026_10_16
description: DW-03S-H source note on channel-coupled resend provenance
metadata:
  type: reference
---

# DW-03S-H source note — 2026-10-16

Source: `crates/base-session/src/base/helpers/reliable_fit.rs`, `helpers/mod.rs`, `tick_sync.rs` and `cooked_sync/task.rs` on DW-03S-H.

The `Channel::check_timeouts()` API yields cached encrypted `Bytes` without a per-entry origin. Capturing `ConnectedClientState::wire_send_origin()` while the same connected-state and channel locks are held is the available provenance seam; a scan after address reuse must also check the original task's cancellation-token identity before it touches the replacement channel. `tokio::spawn(run_tick_loop(...))` constructs the future before scheduling, so a synchronous `run_tick_loop` wrapper can copy the original ticket without changing the login caller's signature. A missing Phase 3 seed appears in test fixtures and must use the legacy untagged send path, leaving proof incomplete.

Merge-group CI's `lab_chaos_callers_stay_on_the_allowlist` caught `BaseService::start` reading the global chaos registry after the transport shim had already read it. The reviewed shim is the registry seam. Return the wrapped transport and its active flag from that one read, then feed the flag to `ProofTransport`; a separate Base read violates the allowlist and could diverge from the chosen transport.

INVALIDATED WHEN: Channel::check_timeouts gains a per-entry origin, or wire_send_origin / ProofTransport change
checked: 2026-11-01
