---
name: Direct Base send-origin registration
description: DW-03S-D crate boundary and after-send reliable-window guard
type: reference
---

Dated 2026-10-16. `shadow_register_reliable_send_for_session` is private to `cimmeria-base-session`, so `cimmeria-base-world-entry` direct sends cannot call it. The DW-03S-D world-entry helper copies the session origin before socket awaits, then uses the original sequence-counter Arc and boot/generation/peer/admission identity to guard public `Channel::register_sent_packet` under the connected lock. It retains send site/kind and `mercury.reliable_send` context. A same-peer relog cannot receive the old encrypted packet in its reliable window. The `enable_entities` create-player and character-visuals routes fence their later helper-origin choice against the earlier content owner. Reanchor's Cell event has no immutable generation, so that send remains untagged and coverage incomplete. See `crates/base-world-entry/src/base/world_entry/mod.rs` and `direct_send_origin_tests.rs`; complete outbound census is still DW-03S-F.
