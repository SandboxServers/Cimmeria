---
name: client-mercury-receive-path
description: Client Mercury receive path RE (2026-09-29): ACK-before-decision, one fragment group per channel keyed by lastFrag, header-must-not-straddle rule, log fn 0x0081c2e0 is a ret stub; five telemetry hooks in PR #1088
metadata:
  type: project
---

Full write-up: `docs/reverse-engineering/findings/client-mercury-receive-path.md`. Hooks/events: PR #1088, `crates/client-telemetry/src/hooks/mercury_recv/`.

- Flag bits: 0x20 = fragment, 0x40 = seq footer (the older table in mercury-protocol-internals.md is wrong).
- `queueAckForPacket` (0x0158cba0) ACKs before the in-order test, so "all ACKed" proves nothing about reassembly. It has `ret 0x10` (4 stack words); the real arg order was not resolved, so the hook forwards blindly and reads only the channel.
- Fragment group: one per channel at channel+0x124, matched by lastFrag only; reliable fragment of another bundle is dropped for good.
- Message loop (0x0157c820, game thread): header must lie inside one packet, only bodies may straddle; a straddling header aborts the rest of the bundle silently.
- The Mercury log function 0x0081c2e0 is a one-byte `ret`; none of the `[Mercury]` strings ever print.
- Ghidra tooling gotcha: `search_instructions` operand filter needs the tail form (`0x60]`), not `[ESP + 0x60]`; a worktree-isolated agent's bash refuses `cd` chains and `for` loops, so run one plain command per call.

**Why:** the leading hypothesis for the partial 15-fragment bundle is a WORD_LENGTH header split by a fragment boundary; unconfirmed until a live `client.mercury.bundle` end event shows `fault = header_does_not_fit_packet`.
