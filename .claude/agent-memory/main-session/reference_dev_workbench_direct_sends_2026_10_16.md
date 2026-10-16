---
name: Direct Base send-origin registration
description: DW-03S-D crate boundary and after-send reliable-window guard
type: reference
---

Dated 2026-10-16. `shadow_register_reliable_send_for_session` is private to `cimmeria-base-session`, so world-entry direct registration guards the original sequence Arc and boot/generation/peer/admission identity before touching the public Channel after send. It retains send site/kind and `mercury.reliable_send` context. Review extended D to `base-session/helpers/{mod.rs,reliable_fit.rs,reliable_fit_tests.rs}`: `send_reliable_to_addr_for_owner` checks the original Arc before normal or fragment ACK/sequence reservation. Unproven reliable entries carry an `_unproven` kind, and the retransmit collector keeps their cached bytes untagged. Inbound handlers have no immutable ingress-generation token before first lookup; Cell reanchor/gate travel have only numeric entity IDs. Their sends stay untagged, so `coverage_complete` remains false until separate owner-token work and DW-03S-F's census. See `crates/base-world-entry/src/base/world_entry/mod.rs`, `direct_send_origin_tests.rs` and `docs/analysis/dev-workbench/worknotes/DW-03S-D.md`.
